//! The verification engine (docs/03): structural pass, verifier orchestration
//! (determinism, sampling, voting, depth), K3 folds, budgets.

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use futures::future::{join_all, BoxFuture};
use indexmap::IndexMap;
use serde_json::{json, Value};

use crate::cache::{self, Cache};
use crate::host::{
    Determinism, Envelope, ExtensionDecl, ExtensionHost, HostCall, PassFail, WireDelta,
};
use crate::intake::Mode;
use crate::meta::structure_only_ok;
use crate::monitor::{CallOutcome, Monitor};
use crate::path::Path;
use crate::registry::Registry;
use crate::report::{CheckResult, Children, NodeResult, Usage};
use crate::schema::{LeafSpec, Node, Type, VerifierSpec};
use crate::settings::{ResolvedSettings, SettingsResolver};
use crate::verdict::{CheckError, Delta, DeltaKind, Notice, Verdict, VoteTally};
use crate::Options;

/// Maximum number of nondeterministic host calls materialized and polled as
/// one sampling batch. The request-wide sample budget still governs the total
/// number of attempts; this ceiling bounds only per-job in-flight allocation.
pub const MAX_SAMPLE_IN_FLIGHT_PER_JOB: u32 = 64;

pub(crate) struct Shared {
    pub samples_left: AtomicU32,
    pub usage: Mutex<Usage>,
    pub deadline: Option<Instant>,
}

#[derive(Default)]
pub(crate) struct Sinks {
    pub notices: Mutex<Vec<Notice>>,
    pub errors: Mutex<Vec<CheckError>>,
    pub fingerprints: Mutex<IndexMap<String, String>>,
}

#[derive(Clone, Copy)]
pub(crate) struct Engine<'a> {
    pub reg: &'a Registry,
    pub cache: &'a dyn Cache,
    pub settings: &'a dyn SettingsResolver,
    pub monitor: Option<&'a dyn Monitor>,
    pub opts: &'a Options,
    pub env: &'a Value,
    pub root: &'a Value,
    pub shared: &'a Shared,
    pub sinks: &'a Sinks,
    pub stop: &'a AtomicBool,
}

enum SampleVote {
    Pass,
    Fail(WireDelta),
}

impl<'e> Engine<'e> {
    pub fn verify_node<'a>(
        self,
        node: &'a Node,
        value: &'a Value,
        path: Path,
        depth: u32,
    ) -> BoxFuture<'a, NodeResult>
    where
        'e: 'a,
    {
        Box::pin(async move {
            let mut coerced: Option<Value> = None;
            if self.opts.mode == Mode::Lenient {
                if let Some(number) = coerce_numeric_string(&node.ty, value) {
                    self.push_notice(&path, format!("coerced string {value} to number {number}"));
                    coerced = Some(number);
                }
            }
            let value: &Value = coerced.as_ref().unwrap_or(value);

            if let Type::Union { variants } = &node.ty {
                return self.verify_union(node, variants, value, path, depth).await;
            }

            let (proceed, struct_deltas) = self.structural_check(node, value, &path);
            let struct_verdict = if struct_deltas.is_empty() {
                Verdict::Pass
            } else {
                Verdict::Fail
            };
            let mut checks = vec![if struct_deltas.is_empty() {
                CheckResult::passing("structure")
            } else {
                CheckResult::failing("structure", struct_deltas)
            }];

            let mut spec_checks = Vec::new();
            if proceed {
                for spec in &node.verify {
                    spec_checks.push(self.run_spec(spec, value, &path, depth).await);
                }
            }
            let spec_verdict = fold_spec_checks(&spec_checks);

            let children = if proceed {
                self.verify_children(node, value, &path, depth).await
            } else {
                None
            };
            let child_verdict = children_verdict(&children);

            let verdict = struct_verdict.and(spec_verdict).and(child_verdict);
            if verdict == Verdict::Fail && self.opts.fail_fast {
                self.stop.store(true, Ordering::Relaxed);
            }
            checks.extend(spec_checks);
            NodeResult {
                path: path.to_string(),
                verdict,
                checks,
                children,
            }
        })
    }

    /// Types, required keys, closed objects, array bounds (docs/02). Returns
    /// whether verifiers and children should still run.
    fn structural_check(self, node: &Node, value: &Value, path: &Path) -> (bool, Vec<Delta>) {
        let mut deltas = Vec::new();
        let mut proceed = true;
        let mismatch = |expected: &str, deltas: &mut Vec<Delta>| {
            deltas.push(Delta {
                expected: Some(expected.to_string()),
                actual: Some(value_kind(value).to_string()),
                ..Delta::structure(
                    path.to_string(),
                    format!("expected {expected}, got {}", value_kind(value)),
                )
            });
        };
        match &node.ty {
            Type::Any => {}
            Type::Null => {
                if !value.is_null() {
                    mismatch("null", &mut deltas);
                    proceed = false;
                }
            }
            Type::Bool => {
                if !value.is_boolean() {
                    mismatch("bool", &mut deltas);
                    proceed = false;
                }
            }
            Type::Int => {
                let integral = value.as_i64().is_some() || value.as_u64().is_some();
                if !integral {
                    if let Some(f) = value.as_f64() {
                        if f.fract() == 0.0 && self.opts.mode == Mode::Lenient {
                            self.push_notice(
                                path,
                                format!("float {f} with integral value accepted as int"),
                            );
                        } else {
                            mismatch("int", &mut deltas);
                            proceed = false;
                        }
                    } else {
                        mismatch("int", &mut deltas);
                        proceed = false;
                    }
                }
            }
            Type::Float => {
                if !value.is_number() {
                    mismatch("float", &mut deltas);
                    proceed = false;
                }
            }
            Type::Str => {
                if !value.is_string() {
                    mismatch("str", &mut deltas);
                    proceed = false;
                }
            }
            Type::Object { fields, open } => match value {
                Value::Object(map) => {
                    for (name, field) in fields {
                        if field.required && !map.contains_key(name) {
                            deltas.push(Delta {
                                expected: Some(field.node.ty.name().to_string()),
                                ..Delta::structure(
                                    path.child_key(name).to_string(),
                                    format!("missing required key '{name}'"),
                                )
                            });
                        }
                    }
                    if !open {
                        for key in map.keys() {
                            if !fields.contains_key(key) {
                                deltas.push(Delta::structure(
                                    path.child_key(key).to_string(),
                                    format!("unexpected key '{key}' (object is closed)"),
                                ));
                            }
                        }
                    }
                }
                _ => {
                    mismatch("object", &mut deltas);
                    proceed = false;
                }
            },
            Type::Array { len, .. } => match value {
                Value::Array(items) => {
                    if let Some(bounds) = len {
                        let n = items.len();
                        let low = bounds.min.map(|lo| n < lo).unwrap_or(false);
                        let high = bounds.max.map(|hi| n > hi).unwrap_or(false);
                        if low || high {
                            deltas.push(Delta {
                                actual: Some(n.to_string()),
                                ..Delta::structure(
                                    path.to_string(),
                                    format!(
                                        "array length {n} outside bounds {:?}..{:?}",
                                        bounds.min, bounds.max
                                    ),
                                )
                            });
                        }
                    }
                }
                _ => {
                    mismatch("array", &mut deltas);
                    proceed = false;
                }
            },
            Type::Union { .. } => unreachable!("unions are handled in verify_node"),
        }
        (proceed, deltas)
    }

    async fn verify_children(
        self,
        node: &Node,
        value: &Value,
        path: &Path,
        depth: u32,
    ) -> Option<Children> {
        match (&node.ty, value) {
            (Type::Object { fields, .. }, Value::Object(map)) => {
                let mut out = IndexMap::new();
                for (name, field) in fields {
                    if self.should_stop() {
                        break;
                    }
                    if let Some(child_value) = map.get(name) {
                        let result = self
                            .verify_node(&field.node, child_value, path.child_key(name), depth)
                            .await;
                        out.insert(name.clone(), result);
                    }
                }
                Some(Children::Fields(out))
            }
            (Type::Array { item, .. }, Value::Array(items)) => {
                let mut out = Vec::new();
                for (index, child_value) in items.iter().enumerate() {
                    if self.should_stop() {
                        break;
                    }
                    out.push(
                        self.verify_node(item, child_value, path.child_index(index), depth)
                            .await,
                    );
                }
                Some(Children::Items(out))
            }
            _ => None,
        }
    }

    /// Best-match union (docs/02): the verdict is the K3 join over variants;
    /// the reported tree is the passing variant, or the one with fewest deltas.
    async fn verify_union(
        self,
        node: &Node,
        variants: &[Node],
        value: &Value,
        path: Path,
        depth: u32,
    ) -> NodeResult {
        if variants.is_empty() {
            let delta = Delta::structure(
                path.to_string(),
                "union must contain at least one variant".to_string(),
            );
            let result = NodeResult {
                path: path.to_string(),
                verdict: Verdict::Fail,
                checks: vec![CheckResult::failing("structure", vec![delta])],
                children: None,
            };
            if self.opts.fail_fast {
                self.stop.store(true, Ordering::Relaxed);
            }
            return result;
        }
        let mut attempts: Vec<(usize, NodeResult)> = Vec::new();
        for (index, variant) in variants.iter().enumerate() {
            // A union branch is speculative. Its fail-fast state must never
            // suppress checks in a later branch or escape before the union
            // itself has selected a result.
            let variant_stop = AtomicBool::new(false);
            let result = self
                .with_stop(&variant_stop)
                .verify_node(variant, value, path.clone(), depth)
                .await;
            let passed = result.verdict == Verdict::Pass;
            attempts.push((index, result));
            if passed {
                break;
            }
        }
        let union_verdict = attempts
            .iter()
            .fold(Verdict::Fail, |acc, (_, r)| acc.or(r.verdict));
        let chosen_pos = attempts
            .iter()
            .position(|(_, r)| r.verdict == Verdict::Pass)
            .unwrap_or_else(|| {
                let mut best = 0;
                let mut best_count = usize::MAX;
                for (pos, (_, r)) in attempts.iter().enumerate() {
                    let count = r.count_deltas();
                    if count < best_count {
                        best = pos;
                        best_count = count;
                    }
                }
                best
            });
        let (variant_index, mut chosen) = attempts
            .into_iter()
            .nth(chosen_pos)
            .expect("chosen union variant exists");

        match chosen.checks.first_mut() {
            Some(first) if first.source == "structure" && first.variant.is_none() => {
                first.variant = Some(variant_index);
            }
            _ => {
                let mut marker = CheckResult::passing("structure");
                marker.variant = Some(variant_index);
                chosen.checks.insert(0, marker);
            }
        }

        let mut spec_checks = Vec::new();
        for spec in &node.verify {
            spec_checks.push(self.run_spec(spec, value, &path, depth).await);
        }
        let spec_verdict = fold_spec_checks(&spec_checks);
        chosen.checks.extend(spec_checks);
        chosen.verdict = union_verdict.and(spec_verdict);
        chosen.path = path.to_string();
        if chosen.verdict == Verdict::Fail && self.opts.fail_fast {
            self.stop.store(true, Ordering::Relaxed);
        }
        chosen
    }

    fn run_spec<'a>(
        self,
        spec: &'a VerifierSpec,
        value: &'a Value,
        path: &'a Path,
        depth: u32,
    ) -> BoxFuture<'a, CheckResult>
    where
        'e: 'a,
    {
        Box::pin(async move {
            match spec {
                VerifierSpec::AllOf { all_of } => {
                    let mut results = Vec::new();
                    for child in all_of {
                        results.push(self.run_spec(child, value, path, depth).await);
                    }
                    all_of_result(results)
                }
                VerifierSpec::AnyOf { any_of } => {
                    let mut results = Vec::new();
                    for child in any_of {
                        let result = self.run_spec(child, value, path, depth).await;
                        let passed = !result.skipped && result.verdict == Verdict::Pass;
                        results.push(result);
                        if passed {
                            // any_of stops at the first pass (docs/03).
                            return CheckResult::passing("any_of");
                        }
                    }
                    let executed: Vec<&CheckResult> =
                        results.iter().filter(|c| !c.skipped).collect();
                    if executed.is_empty() {
                        return CheckResult::skipped("any_of");
                    }
                    if executed.iter().any(|c| c.verdict == Verdict::Inconclusive) {
                        return CheckResult::erroring(
                            "any_of",
                            "no branch passed and at least one was inconclusive".to_string(),
                        );
                    }
                    let best = executed
                        .iter()
                        .min_by_key(|c| c.deltas.len())
                        .expect("non-empty executed set");
                    CheckResult::failing("any_of", best.deltas.clone())
                }
                VerifierSpec::Not { not, message } => {
                    let inner = self.run_spec(not, value, path, depth).await;
                    if inner.skipped {
                        return CheckResult::skipped("not");
                    }
                    match inner.verdict {
                        Verdict::Pass => CheckResult::failing(
                            "not",
                            vec![Delta {
                                path: path.to_string(),
                                kind: DeltaKind::Constraint,
                                message: message.clone(),
                                expected: None,
                                actual: None,
                                source: "not".to_string(),
                                data: None,
                                votes: None,
                            }],
                        ),
                        Verdict::Fail => CheckResult::passing("not"),
                        Verdict::Inconclusive => CheckResult::erroring(
                            "not",
                            inner
                                .error
                                .unwrap_or_else(|| "inner check was inconclusive".to_string()),
                        ),
                    }
                }
                VerifierSpec::Leaf(leaf) => self.run_leaf(leaf, value, path, depth).await,
            }
        })
    }

    async fn run_leaf(
        self,
        leaf: &LeafSpec,
        value: &Value,
        path: &Path,
        depth: u32,
    ) -> CheckResult {
        let config = match resolve_config(&leaf.config, self.env) {
            Ok(config) => config,
            Err(message) => return self.error_check(&leaf.ext, path, message, 0),
        };
        let Some((decl, host)) = self.reg.get(&leaf.ext) else {
            return self.error_check(
                &leaf.ext,
                path,
                format!("unknown extension '{}'", leaf.ext),
                0,
            );
        };
        if let Some(config_schema) = &decl.config_schema {
            if !structure_only_ok(config_schema, &config) {
                return self.error_check(
                    &leaf.ext,
                    path,
                    format!(
                        "resolved config does not satisfy the config_schema of '{}'",
                        leaf.ext
                    ),
                    0,
                );
            }
        }
        if let Err(message) = decl.preflight_config(&config) {
            return self.error_check(
                &leaf.ext,
                path,
                format!(
                    "resolved config failed the semantic preflight of '{}': {message}",
                    leaf.ext
                ),
                0,
            );
        }
        let resolved = match self.settings.resolve(&leaf.ext) {
            Ok(resolved) => resolved,
            Err(message) => {
                return self.error_check(&leaf.ext, path, format!("settings: {message}"), 0)
            }
        };
        if let Some(settings_schema) = &decl.settings_schema {
            if !structure_only_ok(settings_schema, &resolved.value) {
                return self.error_check(
                    &leaf.ext,
                    path,
                    format!(
                        "resolved settings do not satisfy the settings_schema of '{}'",
                        leaf.ext
                    ),
                    0,
                );
            }
        }
        // Attribution (docs/04): record the fingerprint for every extension
        // that is actually configurable — declared schema or non-empty values.
        let configurable = decl.settings_schema.is_some()
            || resolved.value.as_object().is_none_or(|map| !map.is_empty());
        if configurable {
            self.sinks
                .fingerprints
                .lock()
                .unwrap()
                .entry(leaf.ext.clone())
                .or_insert_with(|| resolved.fingerprint.clone());
        }
        match decl.determinism {
            Determinism::Deterministic => {
                self.run_deterministic(leaf, &decl, &*host, &config, &resolved, value, path, depth)
                    .await
            }
            Determinism::Nondeterministic => {
                self.run_sampling(leaf, &decl, &*host, &config, &resolved, value, path, depth)
                    .await
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_deterministic(
        self,
        leaf: &LeafSpec,
        decl: &ExtensionDecl,
        host: &dyn ExtensionHost,
        config: &Value,
        resolved: &ResolvedSettings,
        value: &Value,
        path: &Path,
        depth: u32,
    ) -> CheckResult {
        let root_ctx = decl.needs.root.then_some(self.root);
        let env_ctx = decl.needs.env.then_some(self.env);
        let envelope = if decl.cacheable {
            let key = cache::key(cache::KeyMaterial {
                ext: &leaf.ext,
                semantic_revision: &decl.semantic_revision,
                config,
                settings_fingerprint: &resolved.fingerprint,
                value,
                path: &path.to_string(),
                depth,
                root: root_ctx,
                env: env_ctx,
            });
            match self.cache.get(key).await {
                Some(hit) => Ok(hit),
                None => {
                    let result = self
                        .call_host(host, decl, config, &resolved.value, value, path, depth)
                        .await;
                    if let Ok(envelope) = &result {
                        self.cache.put(key, envelope.clone()).await;
                    }
                    result
                }
            }
        } else {
            self.call_host(host, decl, config, &resolved.value, value, path, depth)
                .await
        };
        let envelope = match envelope {
            Ok(envelope) => envelope,
            Err(message) => return self.error_check(&leaf.ext, path, message, 0),
        };
        match envelope.verdict {
            PassFail::Pass => CheckResult::passing(&leaf.ext),
            PassFail::Fail => {
                let Some(wire) = envelope.delta else {
                    return self.error_check(
                        &leaf.ext,
                        path,
                        "fail verdict without a delta".to_string(),
                        0,
                    );
                };
                if let Err(message) = self
                    .validate_delta(decl, &wire, depth.saturating_sub(1))
                    .await
                {
                    return self.error_check(
                        &leaf.ext,
                        path,
                        format!("delta failed its schema: {message}"),
                        0,
                    );
                }
                CheckResult::failing(
                    &leaf.ext,
                    vec![wire_to_delta(wire, path, DeltaKind::Constraint, &leaf.ext)],
                )
            }
        }
    }

    /// Sampling round (docs/03 §Voting): N independent executions, malformed
    /// results rejected and resampled, quorum, then the vote policy.
    #[allow(clippy::too_many_arguments)]
    async fn run_sampling(
        self,
        leaf: &LeafSpec,
        decl: &ExtensionDecl,
        host: &dyn ExtensionHost,
        config: &Value,
        resolved: &ResolvedSettings,
        value: &Value,
        path: &Path,
        depth: u32,
    ) -> CheckResult {
        let sampling = leaf.sampling.unwrap_or_default();
        let effective_depth = sampling.depth.min(depth);
        if effective_depth == 0 {
            self.push_notice(path, format!("skipped '{}': depth exhausted", leaf.ext));
            return CheckResult::skipped(&leaf.ext);
        }
        let quorum = sampling.quorum();
        // Bound futures allocation before constructing it. The shared counter
        // remains the atomic authority when sibling checks race for budget.
        let available = self.shared.samples_left.load(Ordering::Relaxed);
        let max_attempts = sampling.samples.saturating_mul(2).min(available);

        let mut votes_pass: u32 = 0;
        let mut fail_wires: Vec<WireDelta> = Vec::new();
        let mut errors: Vec<String> = Vec::new();
        let mut attempts: u32 = 0;
        let mut pending = sampling.samples.min(max_attempts);

        while pending > 0 && attempts < max_attempts {
            let batch = pending
                .min(max_attempts - attempts)
                .min(MAX_SAMPLE_IN_FLIGHT_PER_JOB);
            attempts += batch;
            pending -= batch;
            let futures: Vec<_> = (0..batch)
                .map(|_| {
                    self.sample_once(
                        decl,
                        host,
                        config,
                        &resolved.value,
                        value,
                        path,
                        effective_depth,
                    )
                })
                .collect();
            let mut budget_exhausted = false;
            for result in join_all(futures).await {
                match result {
                    Ok(SampleVote::Pass) => votes_pass += 1,
                    Ok(SampleVote::Fail(wire)) => fail_wires.push(wire),
                    Err(message) => {
                        budget_exhausted |= message.contains("sample budget exhausted");
                        errors.push(message);
                        pending += 1;
                    }
                }
            }
            if budget_exhausted {
                break;
            }
        }

        let votes_fail = fail_wires.len() as u32;
        let valid = votes_pass + votes_fail;
        let tally = VoteTally {
            pass: votes_pass,
            fail: votes_fail,
            errors: errors.len() as u32,
            samples: attempts,
        };
        if let Some(monitor) = self.monitor {
            monitor.votes(&leaf.ext, &tally);
        }

        if valid < quorum {
            let message = format!("quorum not met: {valid} valid votes of {quorum} needed");
            self.push_error(
                path,
                &leaf.ext,
                format!("{message}; sample errors: {}", errors.join(" | ")),
                errors.len() as u32,
            );
            let mut check = CheckResult::erroring(&leaf.ext, message);
            check.votes = Some(tally);
            return check;
        }

        if sampling.vote.passes(votes_pass, votes_fail) {
            let mut check = CheckResult::passing(&leaf.ext);
            check.votes = Some(tally);
            return check;
        }

        // Merge failing votes into one delta: distinct messages kept, identical
        // ones deduplicated, tally attached (docs/03).
        let mut messages: Vec<String> = Vec::new();
        for wire in &fail_wires {
            if !messages.contains(&wire.message) {
                messages.push(wire.message.clone());
            }
        }
        let mut distinct_data: Vec<&Value> = Vec::new();
        for wire in &fail_wires {
            if let Some(data) = wire.data.as_ref() {
                if !distinct_data.contains(&data) {
                    distinct_data.push(data);
                }
            }
        }
        let merged = Delta {
            path: path.to_string(),
            kind: DeltaKind::Semantic,
            message: messages.join("\n"),
            expected: fail_wires.iter().find_map(|w| w.expected.clone()),
            actual: fail_wires.iter().find_map(|w| w.actual.clone()),
            source: leaf.ext.clone(),
            data: match distinct_data.as_slice() {
                [single] => Some((*single).clone()),
                _ => None,
            },
            votes: Some(tally),
        };
        let mut check = CheckResult::failing(&leaf.ext, vec![merged]);
        check.votes = Some(tally);
        check
    }

    #[allow(clippy::too_many_arguments)]
    async fn sample_once(
        self,
        decl: &ExtensionDecl,
        host: &dyn ExtensionHost,
        config: &Value,
        settings: &Value,
        value: &Value,
        path: &Path,
        effective_depth: u32,
    ) -> Result<SampleVote, String> {
        let taken = self
            .shared
            .samples_left
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_sub(1))
            .is_ok();
        if !taken {
            return Err("sample budget exhausted".to_string());
        }
        let envelope = self
            .call_host(host, decl, config, settings, value, path, effective_depth)
            .await?;
        match envelope.verdict {
            PassFail::Pass => Ok(SampleVote::Pass),
            PassFail::Fail => {
                let wire = envelope
                    .delta
                    .ok_or_else(|| "fail verdict without a delta".to_string())?;
                self.validate_delta(decl, &wire, effective_depth.saturating_sub(1))
                    .await
                    .map_err(|e| format!("delta failed its schema: {e}"))?;
                Ok(SampleVote::Fail(wire))
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn call_host(
        self,
        host: &dyn ExtensionHost,
        decl: &ExtensionDecl,
        config: &Value,
        settings: &Value,
        value: &Value,
        path: &Path,
        depth: u32,
    ) -> Result<Envelope, String> {
        let deadline_ms = self.remaining_ms();
        if deadline_ms == Some(0) {
            return Err("deadline exceeded".to_string());
        }
        let path_string = path.to_string();
        let call = HostCall {
            ext: &decl.name,
            config,
            settings,
            value,
            path: &path_string,
            root: decl.needs.root.then_some(self.root),
            env: decl.needs.env.then_some(self.env),
            depth,
            deadline_ms,
        };
        let start = Instant::now();
        let mut result = match deadline_ms {
            Some(ms) => tokio::time::timeout(Duration::from_millis(ms), host.verify(call))
                .await
                .unwrap_or_else(|_| Err("deadline exceeded".to_string())),
            None => host.verify(call).await,
        };
        if let Ok(envelope) = &result {
            let accounting = (|| {
                let mut usage = self.shared.usage.lock().unwrap();
                let mut next = *usage;
                next.samples = next
                    .samples
                    .checked_add(1)
                    .ok_or("host sample usage overflow")?;
                if let Some(wire) = &envelope.usage {
                    next.try_add_wire(wire)?;
                }
                *usage = next;
                Ok::<(), &'static str>(())
            })();
            if let Err(message) = accounting {
                result = Err(message.to_string());
            }
        }
        if let Some(monitor) = self.monitor {
            let outcome = match &result {
                Ok(_) => CallOutcome::Ok,
                Err(m) if m.contains("timed out") || m.contains("deadline exceeded") => {
                    CallOutcome::Timeout
                }
                Err(_) => CallOutcome::Error,
            };
            let usage = result.as_ref().ok().and_then(|e| e.usage.as_ref());
            monitor.call(
                &decl.name,
                outcome,
                start.elapsed().as_millis() as u64,
                usage,
            );
        }
        result
    }

    /// The recursive step (docs/03 §3): a delta is a typed value, verified
    /// against the extension's delta_schema at `depth − 1`.
    async fn validate_delta(
        self,
        decl: &ExtensionDecl,
        wire: &WireDelta,
        depth: u32,
    ) -> Result<(), String> {
        let mut delta_value = json!({ "message": wire.message });
        if let Some(data) = &wire.data {
            delta_value["data"] = data.clone();
        }
        let schema = decl
            .delta_schema
            .as_ref()
            .unwrap_or_else(|| default_delta_schema());
        let sinks = Sinks::default();
        let stop = AtomicBool::new(false);
        let sub = Engine {
            reg: self.reg,
            cache: self.cache,
            settings: self.settings,
            monitor: self.monitor,
            opts: self.opts,
            env: self.env,
            root: &delta_value,
            shared: self.shared,
            sinks: &sinks,
            stop: &stop,
        };
        let result = sub
            .verify_node(schema, &delta_value, Path::root(), depth)
            .await;
        if result.verdict != Verdict::Pass {
            let mut deltas = Vec::new();
            result.collect_deltas(&mut deltas);
            let messages: Vec<String> = deltas.into_iter().map(|d| d.message).collect();
            if messages.is_empty() {
                Err(format!(
                    "delta schema was {}, not pass",
                    match result.verdict {
                        Verdict::Inconclusive => "inconclusive",
                        Verdict::Fail => "fail",
                        Verdict::Pass => unreachable!("non-pass branch"),
                    }
                ))
            } else {
                Err(messages.join("; "))
            }
        } else {
            Ok(())
        }
    }

    fn should_stop(self) -> bool {
        self.opts.fail_fast && self.stop.load(Ordering::Relaxed)
    }

    fn with_stop<'scope>(self, stop: &'scope AtomicBool) -> Engine<'scope>
    where
        'e: 'scope,
    {
        Engine {
            reg: self.reg,
            cache: self.cache,
            settings: self.settings,
            monitor: self.monitor,
            opts: self.opts,
            env: self.env,
            root: self.root,
            shared: self.shared,
            sinks: self.sinks,
            stop,
        }
    }

    fn remaining_ms(self) -> Option<u64> {
        self.shared.deadline.map(|deadline| {
            deadline
                .saturating_duration_since(Instant::now())
                .as_millis() as u64
        })
    }

    fn push_notice(self, path: &Path, message: String) {
        self.sinks.notices.lock().unwrap().push(Notice {
            path: path.to_string(),
            message,
            source: "engine".to_string(),
        });
    }

    fn push_error(self, path: &Path, source: &str, error: String, samples_lost: u32) {
        self.sinks.errors.lock().unwrap().push(CheckError {
            path: path.to_string(),
            source: source.to_string(),
            error,
            samples_lost,
        });
    }

    fn error_check(
        self,
        source: &str,
        path: &Path,
        message: String,
        samples_lost: u32,
    ) -> CheckResult {
        self.push_error(path, source, message.clone(), samples_lost);
        CheckResult::erroring(source, message)
    }
}

fn all_of_result(results: Vec<CheckResult>) -> CheckResult {
    if !results.is_empty() && results.iter().all(|c| c.skipped) {
        return CheckResult::skipped("all_of");
    }
    let verdict = fold_spec_checks(&results);
    let mut deltas = Vec::new();
    let mut error = None;
    for result in &results {
        if result.verdict == Verdict::Fail {
            deltas.extend(result.deltas.iter().cloned());
        }
        if result.verdict == Verdict::Inconclusive && error.is_none() {
            error = result.error.clone();
        }
    }
    CheckResult {
        source: "all_of".to_string(),
        verdict,
        deltas,
        votes: None,
        skipped: false,
        variant: None,
        error,
    }
}

/// Skipped checks are ignored unless every check was skipped (docs/03 §3).
pub(crate) fn fold_spec_checks(checks: &[CheckResult]) -> Verdict {
    let executed: Vec<&CheckResult> = checks.iter().filter(|c| !c.skipped).collect();
    if executed.is_empty() {
        if checks.is_empty() {
            Verdict::Pass
        } else {
            Verdict::Inconclusive
        }
    } else {
        executed
            .iter()
            .fold(Verdict::Pass, |acc, check| acc.and(check.verdict))
    }
}

fn children_verdict(children: &Option<Children>) -> Verdict {
    match children {
        Some(Children::Fields(fields)) => fields
            .values()
            .fold(Verdict::Pass, |acc, child| acc.and(child.verdict)),
        Some(Children::Items(items)) => items
            .iter()
            .fold(Verdict::Pass, |acc, child| acc.and(child.verdict)),
        None => Verdict::Pass,
    }
}

fn wire_to_delta(wire: WireDelta, path: &Path, kind: DeltaKind, source: &str) -> Delta {
    Delta {
        path: path.to_string(),
        kind,
        message: wire.message,
        expected: wire.expected,
        actual: wire.actual,
        source: source.to_string(),
        data: wire.data,
        votes: None,
    }
}

fn value_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn coerce_numeric_string(ty: &Type, value: &Value) -> Option<Value> {
    let Value::String(s) = value else {
        return None;
    };
    match ty {
        Type::Int => s.trim().parse::<i64>().ok().map(Value::from),
        Type::Float => s
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number),
        _ => None,
    }
}

/// Replace `{ "$env": "dot.path" }` holes with values from the request env
/// (docs/02 §Dynamic config).
pub(crate) fn resolve_config(config: &Value, env: &Value) -> Result<Value, String> {
    match config {
        Value::Object(map) => {
            if map.len() == 1 {
                if let Some(reference) = map.get("$env") {
                    let dot_path = reference
                        .as_str()
                        .ok_or("$env reference must be a string path")?;
                    return lookup_env(env, dot_path)
                        .cloned()
                        .ok_or_else(|| format!("missing env field '{dot_path}'"));
                }
            }
            let mut out = serde_json::Map::new();
            for (key, child) in map {
                out.insert(key.clone(), resolve_config(child, env)?);
            }
            Ok(Value::Object(out))
        }
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .map(|item| resolve_config(item, env))
                .collect::<Result<_, _>>()?,
        )),
        other => Ok(other.clone()),
    }
}

fn lookup_env<'v>(env: &'v Value, dot_path: &str) -> Option<&'v Value> {
    let mut current = env;
    for segment in dot_path.split('.') {
        current = current.get(segment)?;
    }
    Some(current)
}

fn default_delta_schema() -> &'static Node {
    static SCHEMA: OnceLock<Node> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        Node::from_value(json!({
            "type": "object",
            "fields": {
                "message": { "type": "str" },
                "data": { "type": "any", "required": false }
            }
        }))
        .expect("default delta schema is well-formed")
    })
}

// Keep Arc in the signature story honest: the registry hands out Arc'd hosts,
// and the engine only ever borrows them for the duration of a call.
#[allow(dead_code)]
fn _assert_host_object_safe(_: &Arc<dyn ExtensionHost>) {}
