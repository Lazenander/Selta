//! Exact evaluation of the closed candidate-1 finite-world corpus.
//!
//! This is intentionally generic algebraic machinery. It knows nothing about
//! candidate assessment packages, claims, evidence, interpreters, or
//! projections.

use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const WORLD_REVISION: &str = "selta.evidence.finite-world/s2-1";
const RESULT_REVISION: &str = "selta.evidence.finite-result/s2-1";
const MAX_SAFE_INT: u64 = 9_007_199_254_740_991;
const MAX_DECIMAL_DIGITS: usize = 4096;

type Result<T> = std::result::Result<T, FiniteError>;

#[derive(Debug, Error)]
#[error("invalid finite-world program: {0}")]
pub(crate) struct FiniteError(String);

fn invalid<T>(message: impl Into<String>) -> Result<T> {
    Err(FiniteError(message.into()))
}

#[derive(Debug)]
pub(crate) enum Program {
    WorldTable(WorldTable),
    RepeatEvent(RepeatEvent),
    Transition(Transition),
}

#[derive(Debug, Serialize, PartialEq, Eq)]
pub(crate) struct ResultLedger {
    revision: String,
    results: Vec<ResultEntry>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
struct ResultEntry {
    name: String,
    value: ResultValue,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ResultValue {
    Boolean {
        value: bool,
    },
    SafeInt {
        value: u64,
    },
    Token {
        value: String,
    },
    Rational {
        numerator: String,
        denominator: String,
    },
    OneMinusPower {
        base_numerator: String,
        base_denominator: String,
        exponent: u64,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Scalar {
    Bool(bool),
    SafeInt(u64),
    Token(String),
}

impl Scalar {
    fn into_result(self) -> ResultValue {
        match self {
            Self::Bool(value) => ResultValue::Boolean { value },
            Self::SafeInt(value) => ResultValue::SafeInt { value },
            Self::Token(value) => ResultValue::Token { value },
        }
    }
}

#[derive(Debug)]
pub(crate) struct WorldTable {
    rows: Vec<Row>,
    queries: Vec<Query>,
}

#[derive(Debug)]
struct Row {
    id: String,
    weight: BigUint,
    facts: BTreeMap<String, Scalar>,
}

#[derive(Debug)]
enum Query {
    WeightedEvent {
        name: String,
        event: Selector,
        given: Option<Selector>,
    },
    DistinctValues {
        name: String,
        fields: Vec<String>,
        rows: Vec<usize>,
    },
    Argmax {
        name: String,
        score_field: String,
        project_field: String,
        rows: Vec<usize>,
    },
}

impl Query {
    fn name(&self) -> &str {
        match self {
            Self::WeightedEvent { name, .. }
            | Self::DistinctValues { name, .. }
            | Self::Argmax { name, .. } => name,
        }
    }
}

#[derive(Debug)]
struct Selector(Vec<Vec<(String, Scalar)>>);

#[derive(Debug)]
pub(crate) struct RepeatEvent {
    probability: Rational,
    trials: Vec<Trial>,
}

#[derive(Debug)]
struct Trial {
    name: String,
    count: u64,
    representation: Representation,
}

#[derive(Clone, Copy, Debug)]
enum Representation {
    Reduced,
    Factored,
}

#[derive(Debug)]
struct Rational {
    numerator: BigUint,
    denominator: BigUint,
}

#[derive(Debug)]
pub(crate) struct Transition {
    initial: Scalar,
    next: BTreeMap<Scalar, Scalar>,
    observations: Vec<Observation>,
}

#[derive(Debug)]
struct Observation {
    name: String,
    steps: u64,
}

/// Parses and relationally admits one schema-valid finite-world value.
///
/// The stable Selta schema boundary remains the caller's responsibility. This
/// function additionally rejects malformed wire shapes so its unit tests do
/// not rely on that outer layer.
pub(crate) fn admit(value: Value) -> Result<Program> {
    let wire: WireProgram = serde_json::from_value(value)
        .map_err(|error| FiniteError(format!("wire shape: {error}")))?;
    let program = match wire {
        WireProgram::WorldTable {
            revision,
            rows,
            queries,
        } => {
            require_revision(&revision)?;
            Program::WorldTable(admit_world_table(rows, queries)?)
        }
        WireProgram::RepeatEvent {
            revision,
            single_event,
            trials,
        } => {
            require_revision(&revision)?;
            Program::RepeatEvent(admit_repeat_event(single_event, trials)?)
        }
        WireProgram::Transition {
            revision,
            states,
            initial,
            transitions,
            observations,
        } => {
            require_revision(&revision)?;
            Program::Transition(admit_transition(
                states,
                initial,
                transitions,
                observations,
            )?)
        }
    };

    // Evaluation is also the final relational-admission pass: it proves that
    // all referenced facts exist and that every required aggregate fits.
    evaluate(&program)?;
    Ok(program)
}

pub(crate) fn evaluate(program: &Program) -> Result<ResultLedger> {
    let results = match program {
        Program::WorldTable(world) => evaluate_world_table(world)?,
        Program::RepeatEvent(repeat) => evaluate_repeat_event(repeat)?,
        Program::Transition(transition) => evaluate_transition(transition)?,
    };
    Ok(ResultLedger {
        revision: RESULT_REVISION.to_owned(),
        results,
    })
}

/// Compares the computed result with an admitted retained oracle value.
/// Whitespace and object member order are presentation, not evaluator output.
pub(crate) fn compare_result(actual: &ResultLedger, expected: &Value) -> Result<()> {
    let actual = serde_json_canonicalizer::to_vec(actual)
        .map_err(|error| FiniteError(format!("cannot canonicalize result: {error}")))?;
    let expected = serde_json_canonicalizer::to_vec(expected)
        .map_err(|error| FiniteError(format!("cannot canonicalize oracle: {error}")))?;
    if actual == expected {
        Ok(())
    } else {
        invalid("computed result differs from retained oracle")
    }
}

fn require_revision(revision: &str) -> Result<()> {
    if revision == WORLD_REVISION {
        Ok(())
    } else {
        invalid("wrong finite-world revision")
    }
}

fn admit_world_table(rows: Vec<WireRow>, queries: Vec<WireQuery>) -> Result<WorldTable> {
    if rows.is_empty() || queries.is_empty() {
        return invalid("world tables require at least one row and one query");
    }

    let mut admitted_rows = Vec::with_capacity(rows.len());
    let mut row_indices = BTreeMap::new();
    for row in rows {
        require_token(&row.id, "row id")?;
        if row_indices
            .insert(row.id.clone(), admitted_rows.len())
            .is_some()
        {
            return invalid(format!("duplicate row id {}", row.id));
        }
        let weight = parse_nat(&row.weight, true, "row weight")?;
        if row.facts.is_empty() {
            return invalid(format!("row {} has no facts", row.id));
        }
        let mut facts = BTreeMap::new();
        for fact in row.facts {
            require_token(&fact.name, "fact name")?;
            let name = fact.name;
            if facts
                .insert(name.clone(), admit_scalar(fact.value)?)
                .is_some()
            {
                return invalid(format!("row {} repeats fact {name}", row.id));
            }
        }
        admitted_rows.push(Row {
            id: row.id,
            weight,
            facts,
        });
    }

    require_wire_query_names(&queries)?;
    let mut admitted_queries = Vec::with_capacity(queries.len());
    for query in queries {
        let query = match query {
            WireQuery::WeightedEvent { name, event, given } => Query::WeightedEvent {
                name,
                event: admit_selector(event)?,
                given: given.map(admit_selector).transpose()?,
            },
            WireQuery::DistinctValues { name, fields, rows } => {
                if fields.is_empty() {
                    return invalid(format!("query {name} has no fields"));
                }
                for field in &fields {
                    require_token(field, "distinct field")?;
                }
                Query::DistinctValues {
                    name,
                    fields,
                    rows: admit_row_subset(rows, &row_indices, false)?,
                }
            }
            WireQuery::Argmax {
                name,
                score_field,
                project_field,
                rows,
                tie_break,
            } => {
                require_token(&score_field, "argmax score field")?;
                require_token(&project_field, "argmax projection field")?;
                match tie_break {
                    WireTieBreak::RowId => {}
                }
                Query::Argmax {
                    name,
                    score_field,
                    project_field,
                    rows: admit_row_subset(rows, &row_indices, true)?,
                }
            }
        };
        admitted_queries.push(query);
    }

    Ok(WorldTable {
        rows: admitted_rows,
        queries: admitted_queries,
    })
}

fn require_wire_query_names(queries: &[WireQuery]) -> Result<()> {
    let mut previous: Option<&str> = None;
    for query in queries {
        let name = query.name();
        require_token(name, "query name")?;
        if previous.is_some_and(|prior| prior >= name) {
            return invalid("query names must be unique and strictly ordered");
        }
        previous = Some(name);
    }
    Ok(())
}

fn admit_selector(selector: WireSelector) -> Result<Selector> {
    let mut conjunctions = Vec::with_capacity(selector.any_of.len());
    let mut previous_conjunction: Option<Vec<u8>> = None;
    for conjunction in selector.any_of {
        let encoded = serde_json_canonicalizer::to_vec(&conjunction)
            .map_err(|error| FiniteError(format!("cannot canonicalize selector: {error}")))?;
        if previous_conjunction
            .as_ref()
            .is_some_and(|previous| previous >= &encoded)
        {
            return invalid("selector conjunctions must be unique and JCS-ordered");
        }
        previous_conjunction = Some(encoded);

        let mut clauses = Vec::with_capacity(conjunction.all_of.len());
        let mut previous_clause: Option<(String, Vec<u8>)> = None;
        for clause in conjunction.all_of {
            require_token(&clause.fact, "selector fact")?;
            let equals_jcs = serde_json_canonicalizer::to_vec(&clause.equals)
                .map_err(|error| FiniteError(format!("cannot canonicalize scalar: {error}")))?;
            let key = (clause.fact.clone(), equals_jcs);
            if previous_clause
                .as_ref()
                .is_some_and(|previous| previous >= &key)
            {
                return invalid("selector clauses must be unique and canonically ordered");
            }
            previous_clause = Some(key);
            clauses.push((clause.fact, admit_scalar(clause.equals)?));
        }
        conjunctions.push(clauses);
    }
    Ok(Selector(conjunctions))
}

fn admit_row_subset(
    rows: Option<Vec<String>>,
    row_indices: &BTreeMap<String, usize>,
    require_nonempty: bool,
) -> Result<Vec<usize>> {
    match rows {
        None => Ok((0..row_indices.len()).collect()),
        Some(ids) => {
            if require_nonempty && ids.is_empty() {
                return invalid("argmax row subset cannot be empty");
            }
            let mut previous: Option<&str> = None;
            let mut indices = Vec::with_capacity(ids.len());
            for id in &ids {
                require_token(id, "row subset id")?;
                if previous.is_some_and(|prior| prior >= id.as_str()) {
                    return invalid("row subset must be a unique lexical set");
                }
                let index = row_indices
                    .get(id)
                    .copied()
                    .ok_or_else(|| FiniteError(format!("unknown row id {id}")))?;
                previous = Some(id);
                indices.push(index);
            }
            Ok(indices)
        }
    }
}

fn evaluate_world_table(world: &WorldTable) -> Result<Vec<ResultEntry>> {
    let mut results = Vec::with_capacity(world.queries.len());
    for query in &world.queries {
        let value = match query {
            Query::WeightedEvent { event, given, .. } => {
                let event_rows = matching_rows(event, &world.rows)?;
                let given_rows = match given {
                    Some(selector) => matching_rows(selector, &world.rows)?,
                    None => (0..world.rows.len()).collect(),
                };
                if given_rows.is_empty() {
                    return invalid("weighted-event conditioning set is empty");
                }
                let given_set = given_rows.iter().copied().collect::<BTreeSet<_>>();
                if event_rows.iter().any(|index| !given_set.contains(index)) {
                    return invalid("weighted-event rows are not a subset of given rows");
                }
                let numerator = sum_weights(&world.rows, &event_rows)?;
                let denominator = sum_weights(&world.rows, &given_rows)?;
                rational_result(numerator, denominator)?
            }
            Query::DistinctValues { fields, rows, .. } => {
                let mut tuples = BTreeSet::new();
                for index in rows {
                    let row = &world.rows[*index];
                    let mut tuple = Vec::with_capacity(fields.len());
                    for field in fields {
                        tuple.push(row.facts.get(field).cloned().ok_or_else(|| {
                            FiniteError(format!("row {} is missing fact {field}", row.id))
                        })?);
                    }
                    tuples.insert(tuple);
                }
                let value = u64::try_from(tuples.len())
                    .map_err(|_| FiniteError("distinct count does not fit u64".into()))?;
                if value > MAX_SAFE_INT {
                    return invalid("distinct count exceeds SafeInt");
                }
                ResultValue::SafeInt { value }
            }
            Query::Argmax {
                score_field,
                project_field,
                rows,
                ..
            } => {
                let mut best: Option<(u64, &str, Scalar)> = None;
                for index in rows {
                    let row = &world.rows[*index];
                    let score = match row.facts.get(score_field) {
                        Some(Scalar::SafeInt(value)) => *value,
                        Some(_) => return invalid(format!("row {} has non-integer score", row.id)),
                        None => {
                            return invalid(format!(
                                "row {} is missing score fact {score_field}",
                                row.id
                            ))
                        }
                    };
                    let projected = row.facts.get(project_field).cloned().ok_or_else(|| {
                        FiniteError(format!(
                            "row {} is missing projected fact {project_field}",
                            row.id
                        ))
                    })?;
                    let replace = best.as_ref().is_none_or(|(best_score, best_id, _)| {
                        score > *best_score || (score == *best_score && row.id.as_str() < *best_id)
                    });
                    if replace {
                        best = Some((score, row.id.as_str(), projected));
                    }
                }
                best.ok_or_else(|| FiniteError("argmax has no rows".into()))?
                    .2
                    .into_result()
            }
        };
        results.push(ResultEntry {
            name: query.name().to_owned(),
            value,
        });
    }
    Ok(results)
}

fn matching_rows(selector: &Selector, rows: &[Row]) -> Result<Vec<usize>> {
    let mut matched = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        // Resolve every clause before evaluating the DNF so short-circuiting
        // cannot hide an invalid fact reference.
        for conjunction in &selector.0 {
            for (fact, _) in conjunction {
                if !row.facts.contains_key(fact) {
                    return invalid(format!("row {} is missing selector fact {fact}", row.id));
                }
            }
        }
        let matches = selector.0.iter().any(|conjunction| {
            conjunction
                .iter()
                .all(|(fact, expected)| row.facts.get(fact) == Some(expected))
        });
        if matches {
            matched.push(index);
        }
    }
    Ok(matched)
}

fn sum_weights(rows: &[Row], indices: &[usize]) -> Result<BigUint> {
    let mut sum = BigUint::zero();
    for index in indices {
        sum += &rows[*index].weight;
        require_digit_bound(&sum, "weight aggregate")?;
    }
    Ok(sum)
}

fn admit_repeat_event(single_event: WireRational, trials: Vec<WireTrial>) -> Result<RepeatEvent> {
    if trials.is_empty() {
        return invalid("repeat-event requires at least one trial");
    }
    let probability = admit_rational(single_event)?;
    let mut previous: Option<&str> = None;
    let mut admitted_trials = Vec::with_capacity(trials.len());
    for trial in &trials {
        require_token(&trial.name, "trial name")?;
        if previous.is_some_and(|prior| prior >= trial.name.as_str()) {
            return invalid("trial names must be unique and strictly ordered");
        }
        require_safe_int(trial.count, "trial count")?;
        if trial.count == 0 {
            return invalid("trial count must be positive");
        }
        previous = Some(&trial.name);
        admitted_trials.push(Trial {
            name: trial.name.clone(),
            count: trial.count,
            representation: match trial.representation {
                WireRepresentation::Reduced => Representation::Reduced,
                WireRepresentation::Factored => Representation::Factored,
            },
        });
    }
    Ok(RepeatEvent {
        probability,
        trials: admitted_trials,
    })
}

fn evaluate_repeat_event(repeat: &RepeatEvent) -> Result<Vec<ResultEntry>> {
    let failure_numerator = &repeat.probability.denominator - &repeat.probability.numerator;
    let failure_denominator = repeat.probability.denominator.clone();
    let width = decimal_digits(&failure_denominator).max(decimal_digits(&failure_numerator));
    let mut results = Vec::with_capacity(repeat.trials.len());

    for trial in &repeat.trials {
        let value = match trial.representation {
            Representation::Factored => ResultValue::OneMinusPower {
                base_numerator: failure_numerator.to_str_radix(10),
                base_denominator: failure_denominator.to_str_radix(10),
                exponent: trial.count,
            },
            Representation::Reduced => {
                if trial.count > (MAX_DECIMAL_DIGITS / width) as u64 {
                    return invalid(format!(
                        "trial {} exceeds the reduced-power digit bound",
                        trial.name
                    ));
                }
                let exponent = u32::try_from(trial.count)
                    .map_err(|_| FiniteError("bounded exponent does not fit u32".into()))?;
                let denominator = failure_denominator.pow(exponent);
                let failure = failure_numerator.pow(exponent);
                rational_result(denominator.clone() - failure, denominator)?
            }
        };
        results.push(ResultEntry {
            name: trial.name.clone(),
            value,
        });
    }
    Ok(results)
}

fn admit_transition(
    states: Vec<WireScalar>,
    initial: WireScalar,
    transitions: Vec<WireTransition>,
    observations: Vec<WireObservation>,
) -> Result<Transition> {
    if states.is_empty() || transitions.is_empty() || observations.is_empty() {
        return invalid("transition programs require states, transitions, and observations");
    }

    let mut state_set = BTreeSet::new();
    for state in states {
        let state = admit_scalar(state)?;
        if !state_set.insert(state) {
            return invalid("transition states must be unique");
        }
    }
    let initial = admit_scalar(initial)?;
    if !state_set.contains(&initial) {
        return invalid("initial transition state is not listed");
    }

    let mut next = BTreeMap::new();
    for transition in transitions {
        let from = admit_scalar(transition.from)?;
        let to = admit_scalar(transition.to)?;
        if !state_set.contains(&from) || !state_set.contains(&to) {
            return invalid("transition endpoint is not a listed state");
        }
        if next.insert(from, to).is_some() {
            return invalid("transition function repeats a source state");
        }
    }
    if next.len() != state_set.len() {
        return invalid("transition function is not total");
    }

    let mut previous: Option<&str> = None;
    let mut admitted_observations = Vec::with_capacity(observations.len());
    for observation in &observations {
        require_token(&observation.name, "observation name")?;
        if previous.is_some_and(|prior| prior >= observation.name.as_str()) {
            return invalid("observation names must be unique and strictly ordered");
        }
        require_safe_int(observation.steps, "observation steps")?;
        previous = Some(&observation.name);
        admitted_observations.push(Observation {
            name: observation.name.clone(),
            steps: observation.steps,
        });
    }

    Ok(Transition {
        initial,
        next,
        observations: admitted_observations,
    })
}

fn evaluate_transition(transition: &Transition) -> Result<Vec<ResultEntry>> {
    let mut first_index = BTreeMap::new();
    let mut sequence = Vec::new();
    let mut current = transition.initial.clone();
    while !first_index.contains_key(&current) {
        first_index.insert(current.clone(), sequence.len());
        sequence.push(current.clone());
        current = transition
            .next
            .get(&current)
            .cloned()
            .ok_or_else(|| FiniteError("transition function became partial".into()))?;
    }
    let cycle_start = first_index[&current];
    let cycle_len = sequence.len() - cycle_start;

    let mut results = Vec::with_capacity(transition.observations.len());
    for observation in &transition.observations {
        let steps = observation.steps;
        let index = if steps < sequence.len() as u64 {
            steps as usize
        } else {
            cycle_start + ((steps - cycle_start as u64) % cycle_len as u64) as usize
        };
        results.push(ResultEntry {
            name: observation.name.clone(),
            value: sequence[index].clone().into_result(),
        });
    }
    Ok(results)
}

fn admit_rational(wire: WireRational) -> Result<Rational> {
    let numerator = parse_nat(&wire.numerator, false, "probability numerator")?;
    let denominator = parse_nat(&wire.denominator, true, "probability denominator")?;
    if numerator > denominator {
        return invalid("probability numerator exceeds denominator");
    }
    if numerator.gcd(&denominator) != BigUint::one() {
        return invalid("probability fraction is not reduced");
    }
    Ok(Rational {
        numerator,
        denominator,
    })
}

fn rational_result(numerator: BigUint, denominator: BigUint) -> Result<ResultValue> {
    if denominator.is_zero() || numerator > denominator {
        return invalid("invalid generated probability fraction");
    }
    let divisor = numerator.gcd(&denominator);
    let numerator = numerator / &divisor;
    let denominator = denominator / divisor;
    require_digit_bound(&numerator, "result numerator")?;
    require_digit_bound(&denominator, "result denominator")?;
    Ok(ResultValue::Rational {
        numerator: numerator.to_str_radix(10),
        denominator: denominator.to_str_radix(10),
    })
}

fn parse_nat(text: &str, positive: bool, label: &str) -> Result<BigUint> {
    let canonical = text == "0"
        || (!text.is_empty()
            && !text.starts_with('0')
            && text.as_bytes().iter().all(u8::is_ascii_digit));
    if !canonical || text.len() > MAX_DECIMAL_DIGITS || (positive && text == "0") {
        return invalid(format!("{label} is not a bounded canonical decimal"));
    }
    BigUint::parse_bytes(text.as_bytes(), 10)
        .ok_or_else(|| FiniteError(format!("cannot parse {label}")))
}

fn require_digit_bound(value: &BigUint, label: &str) -> Result<()> {
    if decimal_digits(value) <= MAX_DECIMAL_DIGITS {
        Ok(())
    } else {
        invalid(format!(
            "{label} exceeds {MAX_DECIMAL_DIGITS} decimal digits"
        ))
    }
}

fn decimal_digits(value: &BigUint) -> usize {
    value.to_str_radix(10).len()
}

fn admit_scalar(value: WireScalar) -> Result<Scalar> {
    match value {
        WireScalar::Bool(value) => Ok(Scalar::Bool(value)),
        WireScalar::Int(value) => {
            require_safe_int(value, "scalar integer")?;
            Ok(Scalar::SafeInt(value))
        }
        WireScalar::Token(value) => {
            require_token(&value, "scalar token")?;
            Ok(Scalar::Token(value))
        }
    }
}

fn require_safe_int(value: u64, label: &str) -> Result<()> {
    if value <= MAX_SAFE_INT {
        Ok(())
    } else {
        invalid(format!("{label} exceeds SafeInt"))
    }
}

fn require_token(value: &str, label: &str) -> Result<()> {
    let bytes = value.as_bytes();
    let valid = !bytes.is_empty()
        && bytes.len() <= 256
        && bytes[0].is_ascii_alphanumeric()
        && bytes[1..]
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':' | b'-'));
    if valid {
        Ok(())
    } else {
        invalid(format!("{label} is not a bounded canonical token"))
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WireProgram {
    WorldTable {
        revision: String,
        rows: Vec<WireRow>,
        queries: Vec<WireQuery>,
    },
    RepeatEvent {
        revision: String,
        single_event: WireRational,
        trials: Vec<WireTrial>,
    },
    Transition {
        revision: String,
        states: Vec<WireScalar>,
        initial: WireScalar,
        transitions: Vec<WireTransition>,
        observations: Vec<WireObservation>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRow {
    id: String,
    weight: String,
    facts: Vec<WireFact>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireFact {
    name: String,
    value: WireScalar,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum WireScalar {
    Bool(bool),
    Int(u64),
    Token(String),
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum WireQuery {
    WeightedEvent {
        name: String,
        event: WireSelector,
        #[serde(default)]
        given: Option<WireSelector>,
    },
    DistinctValues {
        name: String,
        fields: Vec<String>,
        #[serde(default)]
        rows: Option<Vec<String>>,
    },
    Argmax {
        name: String,
        score_field: String,
        project_field: String,
        #[serde(default)]
        rows: Option<Vec<String>>,
        tie_break: WireTieBreak,
    },
}

impl WireQuery {
    fn name(&self) -> &str {
        match self {
            Self::WeightedEvent { name, .. }
            | Self::DistinctValues { name, .. }
            | Self::Argmax { name, .. } => name,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireSelector {
    any_of: Vec<WireConjunction>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireConjunction {
    all_of: Vec<WireClause>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WireClause {
    fact: String,
    equals: WireScalar,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireTieBreak {
    RowId,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireRational {
    numerator: String,
    denominator: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTrial {
    name: String,
    count: u64,
    representation: WireRepresentation,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum WireRepresentation {
    Reduced,
    Factored,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireTransition {
    from: WireScalar,
    to: WireScalar,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObservation {
    name: String,
    steps: u64,
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use serde_json::{json, Value};
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::stable::StableSelta;

    fn evaluate_value(value: Value) -> Result<ResultLedger> {
        let program = admit(value)?;
        evaluate(&program)
    }

    fn simple_world(rows: Value, queries: Value) -> Value {
        json!({
            "revision": WORLD_REVISION,
            "kind": "world_table",
            "rows": rows,
            "queries": queries
        })
    }

    #[test]
    fn all_retained_finite_pairs_admit_evaluate_and_match_exact_hashes() {
        const CASES: [(&str, &str); 10] = [
            (
                "adaptive-holdout",
                "8210b763159e6fa34c151c88bd48edf7d97d28a84055cb0561db34ee9d146ea1",
            ),
            (
                "best-of-n-proxy",
                "2ac09220e60abb41e95c1c3b69e8584e478052d6f7dec6dc7eaec27d878be3f1",
            ),
            (
                "cascade-false-reject",
                "abf3c3f6fc7be7ccfc31cba674a4b5303fa9be4b2b4f0a0ebe93299b408f141f",
            ),
            (
                "correlated-majority",
                "ffdb796f4f791c29949d95beb8db8561d96b25a3a6b4a40d9df69342e3629163",
            ),
            (
                "correlated-unanimity",
                "66df7c37cae5b03338195e2b11e84557541ba9418464a375feefd2206b0c3b91",
            ),
            (
                "genuine-disagreement",
                "a16485f4fecced4da227f36fa79e4fa357ef74560c737172de510ff806bf356e",
            ),
            (
                "incompetent-independent",
                "bf3750580165ece3d39f66b48e6dbf27d2ba795aee9345f83c524ccbf4549576",
            ),
            (
                "optional-stop",
                "fc9147e9436e1095263ae7a4d90a4b808db73d6d1961c13ef35b5c5839ead780",
            ),
            (
                "recursive-fixed-point",
                "880d0af5671a7e6c22c581c0dea3ddb3df4a58f4c8e4e7fb220d52ec90c8ba19",
            ),
            (
                "valid-log-omission",
                "5a80675bada698d907209c2d902319f26fbabd3d47e7ca65fcd0c848e7a144e3",
            ),
        ];

        let candidate = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let finite = candidate.join("finite");
        let schemas = candidate.join("schemas");
        let stable = StableSelta::new();
        let world_schema = stable
            .admit_schema(&fs::read(schemas.join("finite-world.schema.json")).unwrap())
            .unwrap();
        let result_schema = stable
            .admit_schema(&fs::read(schemas.join("finite-result.schema.json")).unwrap())
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();

        for (case, expected_hash) in CASES {
            let input: Value = serde_json::from_slice(
                &fs::read(finite.join(format!("{case}.input.json"))).unwrap(),
            )
            .unwrap();
            let expected_bytes = fs::read(finite.join(format!("{case}.expected.json"))).unwrap();
            let expected: Value = serde_json::from_slice(&expected_bytes).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(&expected_bytes)),
                expected_hash,
                "retained raw hash for {case}"
            );
            runtime
                .block_on(stable.verify_value(&world_schema, &input))
                .unwrap();
            runtime
                .block_on(stable.verify_value(&result_schema, &expected))
                .unwrap();
            let actual = evaluate_value(input).unwrap();
            let actual_value = serde_json::to_value(&actual).unwrap();
            runtime
                .block_on(stable.verify_value(&result_schema, &actual_value))
                .unwrap();
            compare_result(&actual, &expected)
                .unwrap_or_else(|error| panic!("finite corpus case {case} differs: {error}"));
        }
    }

    #[test]
    fn weighted_dnf_counts_a_row_once_and_preserves_exact_json_types() {
        let program = simple_world(
            json!([
                {"id":"a","weight":"1","facts":[
                    {"name":"x","value":true},{"name":"y","value":"1"}
                ]},
                {"id":"b","weight":"1","facts":[
                    {"name":"x","value":false},{"name":"y","value":1}
                ]}
            ]),
            json!([{
                "kind":"weighted_event","name":"event",
                "event":{"any_of":[
                    {"all_of":[{"fact":"y","equals":"1"}]},
                    {"all_of":[{"fact":"x","equals":true}]}
                ]}
            }]),
        );
        let result = evaluate_value(program).unwrap();
        assert_eq!(
            result.results[0].value,
            ResultValue::Rational {
                numerator: "1".into(),
                denominator: "2".into()
            }
        );
    }

    #[test]
    fn selectors_are_canonical_and_short_circuiting_cannot_hide_missing_facts() {
        let unsorted = simple_world(
            json!([{"id":"a","weight":"1","facts":[
                {"name":"x","value":true},{"name":"y","value":true}
            ]}]),
            json!([{
                "kind":"weighted_event","name":"event","event":{"any_of":[
                    {"all_of":[{"fact":"y","equals":true},{"fact":"x","equals":true}]}
                ]}
            }]),
        );
        assert!(admit(unsorted).is_err());

        let missing = simple_world(
            json!([{"id":"a","weight":"1","facts":[{"name":"x","value":true}]}]),
            json!([{
                "kind":"weighted_event","name":"event","event":{"any_of":[
                    {"all_of":[]},
                    {"all_of":[{"fact":"z","equals":true}]}
                ]}
            }]),
        );
        assert!(admit(missing).is_err());
    }

    #[test]
    fn empty_selector_truth_values_and_given_subset_are_enforced() {
        let false_event = simple_world(
            json!([{"id":"a","weight":"9","facts":[{"name":"x","value":true}]}]),
            json!([{"kind":"weighted_event","name":"event","event":{"any_of":[]}}]),
        );
        let result = evaluate_value(false_event).unwrap();
        assert_eq!(
            result.results[0].value,
            ResultValue::Rational {
                numerator: "0".into(),
                denominator: "1".into()
            }
        );

        let outside_given = simple_world(
            json!([
                {"id":"a","weight":"1","facts":[{"name":"x","value":true}]},
                {"id":"b","weight":"1","facts":[{"name":"x","value":false}]}
            ]),
            json!([{
                "kind":"weighted_event","name":"event",
                "event":{"any_of":[{"all_of":[{"fact":"x","equals":true}]}]},
                "given":{"any_of":[{"all_of":[{"fact":"x","equals":false}]}]}
            }]),
        );
        assert!(admit(outside_given).is_err());
    }

    #[test]
    fn canonical_decimal_and_probability_rules_are_closed() {
        for (numerator, denominator) in [("00", "1"), ("1", "0"), ("2", "1"), ("2", "4")] {
            let value = json!({
                "revision":WORLD_REVISION,"kind":"repeat_event",
                "single_event":{"numerator":numerator,"denominator":denominator},
                "trials":[{"name":"x","count":1,"representation":"reduced"}]
            });
            assert!(admit(value).is_err(), "accepted {numerator}/{denominator}");
        }
        let too_long = "9".repeat(MAX_DECIMAL_DIGITS + 1);
        let value = json!({
            "revision":WORLD_REVISION,"kind":"repeat_event",
            "single_event":{"numerator":"1","denominator":too_long},
            "trials":[{"name":"x","count":1,"representation":"factored"}]
        });
        assert!(admit(value).is_err());
    }

    #[test]
    fn exact_weight_aggregate_digit_crossing_is_rejected() {
        let weight = "9".repeat(MAX_DECIMAL_DIGITS);
        let exact = simple_world(
            json!([
                {"id":"a","weight":weight,"facts":[{"name":"x","value":true}]}
            ]),
            json!([{
                "kind":"weighted_event","name":"event",
                "event":{"any_of":[{"all_of":[]}]}
            }]),
        );
        assert!(admit(exact).is_ok());

        let crossing = simple_world(
            json!([
                {"id":"a","weight":weight,"facts":[{"name":"x","value":true}]},
                {"id":"b","weight":weight,"facts":[{"name":"x","value":true}]}
            ]),
            json!([{
                "kind":"weighted_event","name":"event",
                "event":{"any_of":[{"all_of":[]}]}
            }]),
        );
        assert!(admit(crossing).is_err());
    }

    #[test]
    fn row_subsets_are_canonical_and_distinct_tuples_keep_scalar_kinds() {
        let rows = json!([
            {"id":"a","weight":"1","facts":[{"name":"x","value":true}]},
            {"id":"b","weight":"1","facts":[{"name":"x","value":1}]},
            {"id":"c","weight":"1","facts":[{"name":"x","value":"1"}]}
        ]);
        let program = simple_world(
            rows.clone(),
            json!([{"kind":"distinct_values","name":"count","fields":["x"]}]),
        );
        let result = evaluate_value(program).unwrap();
        assert_eq!(result.results[0].value, ResultValue::SafeInt { value: 3 });

        let unsorted = simple_world(
            rows,
            json!([{
                "kind":"distinct_values","name":"count","fields":["x"],
                "rows":["b","a"]
            }]),
        );
        assert!(admit(unsorted).is_err());
    }

    #[test]
    fn argmax_defaults_to_all_rows_and_uses_smallest_row_id_for_ties() {
        let program = simple_world(
            json!([
                {"id":"z","weight":"1","facts":[
                    {"name":"score","value":7},{"name":"value","value":"z-choice"}
                ]},
                {"id":"a","weight":"1","facts":[
                    {"name":"score","value":7},{"name":"value","value":"a-choice"}
                ]}
            ]),
            json!([{
                "kind":"argmax","name":"best","score_field":"score",
                "project_field":"value","tie_break":"row_id"
            }]),
        );
        let result = evaluate_value(program).unwrap();
        assert_eq!(
            result.results[0].value,
            ResultValue::Token {
                value: "a-choice".into()
            }
        );
    }

    #[test]
    fn reduced_repeat_uses_the_preflight_bound_and_factored_never_expands() {
        let exact = json!({
            "revision":WORLD_REVISION,"kind":"repeat_event",
            "single_event":{"numerator":"1","denominator":"10"},
            "trials":[{"name":"x","count":2048,"representation":"reduced"}]
        });
        assert!(admit(exact).is_ok());

        let crossing = json!({
            "revision":WORLD_REVISION,"kind":"repeat_event",
            "single_event":{"numerator":"1","denominator":"10"},
            "trials":[{"name":"x","count":2049,"representation":"reduced"}]
        });
        assert!(admit(crossing).is_err());

        let factored = json!({
            "revision":WORLD_REVISION,"kind":"repeat_event",
            "single_event":{"numerator":"1","denominator":"10"},
            "trials":[{"name":"x","count":MAX_SAFE_INT,"representation":"factored"}]
        });
        let result = evaluate_value(factored).unwrap();
        assert_eq!(
            result.results[0].value,
            ResultValue::OneMinusPower {
                base_numerator: "9".into(),
                base_denominator: "10".into(),
                exponent: MAX_SAFE_INT
            }
        );
    }

    #[test]
    fn repeat_event_handles_zero_and_one_exactly() {
        for (numerator, expected) in [("0", ("0", "1")), ("1", ("1", "1"))] {
            let value = json!({
                "revision":WORLD_REVISION,"kind":"repeat_event",
                "single_event":{"numerator":numerator,"denominator":"1"},
                "trials":[{"name":"x","count":3,"representation":"reduced"}]
            });
            let result = evaluate_value(value).unwrap();
            assert_eq!(
                result.results[0].value,
                ResultValue::Rational {
                    numerator: expected.0.into(),
                    denominator: expected.1.into()
                }
            );
        }
    }

    #[test]
    fn transition_uses_tail_and_cycle_for_maximum_safe_steps() {
        let value = json!({
            "revision":WORLD_REVISION,"kind":"transition",
            "states":["tail","a","b"],"initial":"tail",
            "transitions":[
                {"from":"tail","to":"a"},{"from":"a","to":"b"},{"from":"b","to":"a"}
            ],
            "observations":[
                {"name":"n-0","steps":0},{"name":"n-1","steps":1},
                {"name":"n-max","steps":MAX_SAFE_INT}
            ]
        });
        let result = evaluate_value(value).unwrap();
        assert_eq!(
            result
                .results
                .iter()
                .map(|entry| &entry.value)
                .collect::<Vec<_>>(),
            vec![
                &ResultValue::Token {
                    value: "tail".into()
                },
                &ResultValue::Token { value: "a".into() },
                &ResultValue::Token { value: "a".into() },
            ]
        );
    }

    #[test]
    fn transition_rejects_partial_duplicate_and_unknown_edges() {
        let partial = json!({
            "revision":WORLD_REVISION,"kind":"transition",
            "states":["a","b"],"initial":"a",
            "transitions":[{"from":"a","to":"b"}],
            "observations":[{"name":"x","steps":0}]
        });
        assert!(admit(partial).is_err());

        let duplicate = json!({
            "revision":WORLD_REVISION,"kind":"transition",
            "states":["a","b"],"initial":"a",
            "transitions":[{"from":"a","to":"a"},{"from":"a","to":"b"}],
            "observations":[{"name":"x","steps":0}]
        });
        assert!(admit(duplicate).is_err());

        let unknown = json!({
            "revision":WORLD_REVISION,"kind":"transition",
            "states":["a"],"initial":"a",
            "transitions":[{"from":"a","to":"b"}],
            "observations":[{"name":"x","steps":0}]
        });
        assert!(admit(unknown).is_err());
    }

    #[test]
    fn names_are_strictly_ordered_and_unknown_members_are_rejected() {
        let unordered = simple_world(
            json!([{"id":"a","weight":"1","facts":[{"name":"x","value":true}]}]),
            json!([
                {"kind":"distinct_values","name":"z","fields":["x"]},
                {"kind":"distinct_values","name":"a","fields":["x"]}
            ]),
        );
        assert!(admit(unordered).is_err());

        let unknown = json!({
            "revision":WORLD_REVISION,"kind":"repeat_event","extra":true,
            "single_event":{"numerator":"1","denominator":"2"},
            "trials":[{"name":"x","count":1,"representation":"reduced"}]
        });
        assert!(admit(unknown).is_err());
    }

    #[test]
    fn result_comparison_is_jcs_value_equality() {
        let actual = evaluate_value(json!({
            "revision":WORLD_REVISION,"kind":"repeat_event",
            "single_event":{"numerator":"1","denominator":"2"},
            "trials":[{"name":"x","count":1,"representation":"reduced"}]
        }))
        .unwrap();
        let expected = json!({
            "results":[{"value":{"denominator":"2","numerator":"1","kind":"rational"},"name":"x"}],
            "revision":RESULT_REVISION
        });
        compare_result(&actual, &expected).unwrap();
        let wrong = json!({
            "revision":RESULT_REVISION,
            "results":[{"name":"x","value":{"kind":"rational","numerator":"1","denominator":"3"}}]
        });
        assert!(compare_result(&actual, &wrong).is_err());
    }
}
