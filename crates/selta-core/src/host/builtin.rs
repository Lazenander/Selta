//! The in-process host: deterministic builtins (docs/05 §Builtins).
//! `cmd` runs only named templates from a server-side allowlist.

use std::collections::HashMap;
use std::io::Write;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{
    ConfigPreflight, Determinism, EffectClass, Envelope, ExtensionDecl, ExtensionHost, HostCall,
    InputDomain, InputKind, Needs, WireDelta,
};
use crate::schema::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdTemplate {
    pub run: Vec<String>,
    #[serde(default)]
    pub input: CmdInput,
    #[serde(default = "default_timeout_ms")]
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CmdInput {
    #[default]
    File,
    Stdin,
}

fn default_timeout_ms() -> u64 {
    10_000
}

pub trait CmdTemplates: Send + Sync {
    fn get(&self, name: &str) -> Option<CmdTemplate>;
}

impl CmdTemplates for HashMap<String, CmdTemplate> {
    fn get(&self, name: &str) -> Option<CmdTemplate> {
        HashMap::get(self, name).cloned()
    }
}

pub struct BuiltinHost {
    cmds: Option<Arc<dyn CmdTemplates>>,
}

impl BuiltinHost {
    pub fn new(cmds: Option<Arc<dyn CmdTemplates>>) -> Self {
        BuiltinHost { cmds }
    }

    pub fn decls() -> Vec<ExtensionDecl> {
        let decl = |name: &str,
                    effect_class: EffectClass,
                    accepted_input: InputDomain,
                    config_preflight: Option<ConfigPreflight>,
                    config_schema: Value| ExtensionDecl {
            name: name.to_string(),
            semantic_revision: format!("selta.builtin.{name}.v1"),
            cacheable: effect_class == EffectClass::Pure,
            determinism: Determinism::Deterministic,
            effect_class,
            accepted_input,
            config_schema: Some(Node::from_value(config_schema).expect("builtin config schema")),
            config_preflight,
            needs: Needs::default(),
            settings_schema: None,
            delta_schema: None,
        };
        vec![
            decl(
                "one_of",
                EffectClass::Pure,
                InputDomain::any(),
                Some(preflight_one_of),
                json!({ "type": "object", "fields": {
                    "values": { "type": "array", "item": { "type": "any" } }
                }}),
            ),
            decl(
                "range",
                EffectClass::Pure,
                InputDomain::new([InputKind::Int, InputKind::Float])
                    .expect("non-empty builtin input domain"),
                Some(preflight_range),
                json!({ "type": "object", "fields": {
                    "min": { "type": "float", "required": false },
                    "max": { "type": "float", "required": false }
                }}),
            ),
            decl(
                "regex",
                EffectClass::Pure,
                InputDomain::new([InputKind::Str]).expect("non-empty builtin input domain"),
                Some(preflight_regex),
                json!({ "type": "object", "fields": {
                    "pattern": { "type": "str" }
                }}),
            ),
            decl(
                "len",
                EffectClass::Pure,
                InputDomain::new([InputKind::Str, InputKind::Array])
                    .expect("non-empty builtin input domain"),
                Some(preflight_len),
                json!({ "type": "object", "fields": {
                    "min": { "type": "int", "required": false },
                    "max": { "type": "int", "required": false }
                }}),
            ),
            decl(
                "non_empty",
                EffectClass::Pure,
                InputDomain::new([InputKind::Str, InputKind::Array, InputKind::Object])
                    .expect("non-empty builtin input domain"),
                None,
                json!({ "type": "object", "fields": {} }),
            ),
            decl(
                "cmd",
                EffectClass::ProcessIo,
                InputDomain::any(),
                None,
                json!({ "type": "object", "fields": {
                    "name": { "type": "str" },
                    "args": { "type": "array", "item": { "type": "str" }, "required": false }
                }}),
            ),
        ]
    }

    pub fn pure_decls() -> Vec<ExtensionDecl> {
        Self::decls()
            .into_iter()
            .filter(|declaration| declaration.effect_class == EffectClass::Pure)
            .collect()
    }
}

fn preflight_one_of(config: &Value) -> Result<(), String> {
    let values = config
        .get("values")
        .and_then(Value::as_array)
        .ok_or("one_of: config.values must be an array")?;
    if values.is_empty() {
        return Err("one_of: config.values must not be empty".to_string());
    }
    Ok(())
}

fn numeric_bounds(config: &Value, extension: &str) -> Result<(Option<f64>, Option<f64>), String> {
    let parse = |name: &str| -> Result<Option<f64>, String> {
        match config.get(name) {
            Some(value) => value
                .as_f64()
                .map(Some)
                .ok_or_else(|| format!("{extension}: config.{name} must be a number")),
            None => Ok(None),
        }
    };
    let min = parse("min")?;
    let max = parse("max")?;
    if min.is_none() && max.is_none() {
        return Err(format!("{extension}: config requires min, max, or both"));
    }
    if matches!((min, max), (Some(min), Some(max)) if min > max) {
        return Err(format!(
            "{extension}: config.min must not exceed config.max"
        ));
    }
    Ok((min, max))
}

fn preflight_range(config: &Value) -> Result<(), String> {
    numeric_bounds(config, "range").map(|_| ())
}

fn preflight_regex(config: &Value) -> Result<(), String> {
    let pattern = config
        .get("pattern")
        .and_then(Value::as_str)
        .ok_or("regex: config.pattern must be a string")?;
    regex::Regex::new(pattern)
        .map(|_| ())
        .map_err(|error| format!("regex: invalid pattern: {error}"))
}

fn preflight_len(config: &Value) -> Result<(), String> {
    let parse = |name: &str| -> Result<Option<u64>, String> {
        match config.get(name) {
            Some(value) => value
                .as_u64()
                .map(Some)
                .ok_or_else(|| format!("len: config.{name} must be a non-negative integer")),
            None => Ok(None),
        }
    };
    let min = parse("min")?;
    let max = parse("max")?;
    if min.is_none() && max.is_none() {
        return Err("len: config requires min, max, or both".to_string());
    }
    if matches!((min, max), (Some(min), Some(max)) if min > max) {
        return Err("len: config.min must not exceed config.max".to_string());
    }
    Ok(())
}

#[async_trait]
impl ExtensionHost for BuiltinHost {
    async fn verify(&self, call: HostCall<'_>) -> Result<Envelope, String> {
        match call.ext {
            "one_of" => one_of(call.config, call.value),
            "range" => range(call.config, call.value),
            "regex" => regex_check(call.config, call.value),
            "len" => len_check(call.config, call.value),
            "non_empty" => non_empty(call.value),
            "cmd" => self.run_cmd(call.config, call.value).await,
            other => Err(format!("builtin host: unknown extension '{other}'")),
        }
    }
}

fn compact(value: &Value) -> String {
    let s = value.to_string();
    if s.chars().count() > 120 {
        let truncated: String = s.chars().take(120).collect();
        format!("{truncated}…")
    } else {
        s
    }
}

fn one_of(config: &Value, value: &Value) -> Result<Envelope, String> {
    let values = config
        .get("values")
        .and_then(Value::as_array)
        .ok_or("one_of: config.values must be an array")?;
    if values.contains(value) {
        return Ok(Envelope::pass());
    }
    let expected = Value::Array(values.clone());
    Ok(Envelope::fail(WireDelta {
        message: format!(
            "expected one of {}, got {}",
            compact(&expected),
            compact(value)
        ),
        data: None,
        expected: Some(compact(&expected)),
        actual: Some(compact(value)),
    }))
}

fn bound_text(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(lo), Some(hi)) => format!("{lo}..={hi}"),
        (Some(lo), None) => format!(">= {lo}"),
        (None, Some(hi)) => format!("<= {hi}"),
        (None, None) => "any number".to_string(),
    }
}

fn range(config: &Value, value: &Value) -> Result<Envelope, String> {
    let min = config.get("min").and_then(Value::as_f64);
    let max = config.get("max").and_then(Value::as_f64);
    let n = value.as_f64().ok_or("range: value is not a number")?;
    let ok = min.map(|lo| n >= lo).unwrap_or(true) && max.map(|hi| n <= hi).unwrap_or(true);
    if ok {
        return Ok(Envelope::pass());
    }
    let expected = bound_text(min, max);
    Ok(Envelope::fail(WireDelta {
        message: format!("expected a value in {expected}, got {n}"),
        data: None,
        expected: Some(expected),
        actual: Some(n.to_string()),
    }))
}

fn regex_check(config: &Value, value: &Value) -> Result<Envelope, String> {
    let pattern = config
        .get("pattern")
        .and_then(Value::as_str)
        .ok_or("regex: config.pattern must be a string")?;
    let re = regex::Regex::new(pattern).map_err(|e| format!("regex: invalid pattern: {e}"))?;
    let s = value.as_str().ok_or("regex: value is not a string")?;
    if re.is_match(s) {
        return Ok(Envelope::pass());
    }
    Ok(Envelope::fail(WireDelta {
        message: format!("string does not match /{pattern}/"),
        data: None,
        expected: Some(format!("/{pattern}/")),
        actual: Some(compact(value)),
    }))
}

fn len_check(config: &Value, value: &Value) -> Result<Envelope, String> {
    let measured = match value {
        Value::String(s) => s.chars().count(),
        Value::Array(a) => a.len(),
        _ => return Err("len: value must be a string or array".to_string()),
    };
    let min = config.get("min").and_then(Value::as_u64);
    let max = config.get("max").and_then(Value::as_u64);
    let m = measured as u64;
    let ok = min.map(|lo| m >= lo).unwrap_or(true) && max.map(|hi| m <= hi).unwrap_or(true);
    if ok {
        return Ok(Envelope::pass());
    }
    let expected = bound_text(min.map(|v| v as f64), max.map(|v| v as f64));
    Ok(Envelope::fail(WireDelta {
        message: format!("expected length {expected}, got {measured}"),
        data: None,
        expected: Some(format!("length {expected}")),
        actual: Some(measured.to_string()),
    }))
}

fn non_empty(value: &Value) -> Result<Envelope, String> {
    let empty = match value {
        Value::String(s) => s.is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => return Err("non_empty: value must be a string, array, or object".to_string()),
    };
    if empty {
        Ok(Envelope::fail(WireDelta::message("value is empty")))
    } else {
        Ok(Envelope::pass())
    }
}

impl BuiltinHost {
    async fn run_cmd(&self, config: &Value, value: &Value) -> Result<Envelope, String> {
        let templates = self
            .cmds
            .as_ref()
            .ok_or("cmd: no command templates configured on this host")?;
        let name = config
            .get("name")
            .and_then(Value::as_str)
            .ok_or("cmd: config.name must be a string")?;
        let template = templates
            .get(name)
            .ok_or_else(|| format!("cmd: '{name}' is not in the allowlist"))?;
        let extra_args: Vec<String> = config
            .get("args")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();

        let payload = match value {
            Value::String(s) => s.clone().into_bytes(),
            other => serde_json::to_vec_pretty(other).map_err(|e| format!("cmd: {e}"))?,
        };

        let mut file_guard = None;
        let file_path = if template.input == CmdInput::File {
            // A tool-friendly name: no leading dot (rustc derives crate names
            // from file names), underscore-safe.
            let mut file = tempfile::Builder::new()
                .prefix("selta_")
                .tempfile()
                .map_err(|e| format!("cmd: temp file: {e}"))?;
            file.write_all(&payload)
                .map_err(|e| format!("cmd: temp file: {e}"))?;
            let path = file.path().to_string_lossy().into_owned();
            file_guard = Some(file);
            Some(path)
        } else {
            None
        };

        let argv: Vec<String> = template
            .run
            .iter()
            .chain(extra_args.iter())
            .map(|arg| match &file_path {
                Some(p) => arg.replace("{file}", p),
                None => arg.clone(),
            })
            .collect();
        let (program, args) = argv
            .split_first()
            .ok_or("cmd: template has an empty command")?;

        let mut command = tokio::process::Command::new(program);
        command
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        command.stdin(if template.input == CmdInput::Stdin {
            Stdio::piped()
        } else {
            Stdio::null()
        });

        let mut child = command
            .spawn()
            .map_err(|e| format!("cmd: failed to spawn '{program}': {e}"))?;
        if template.input == CmdInput::Stdin {
            use tokio::io::AsyncWriteExt;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(&payload)
                    .await
                    .map_err(|e| format!("cmd: stdin: {e}"))?;
            }
        }

        let output = tokio::time::timeout(
            Duration::from_millis(template.timeout_ms),
            child.wait_with_output(),
        )
        .await
        .map_err(|_| format!("cmd: '{name}' timed out after {} ms", template.timeout_ms))?
        .map_err(|e| format!("cmd: {e}"))?;
        drop(file_guard);

        if output.status.success() {
            return Ok(Envelope::pass());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        let truncated: String = stderr.chars().take(2000).collect();
        Ok(Envelope::fail(WireDelta {
            message: if truncated.trim().is_empty() {
                format!("'{name}' exited with {}", output.status)
            } else {
                truncated
            },
            data: Some(json!({ "exit_code": output.status.code() })),
            expected: None,
            actual: None,
        }))
    }
}
