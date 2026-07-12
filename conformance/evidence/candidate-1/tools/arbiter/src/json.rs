//! Neutral, bounded I-JSON parsing and RFC 8785 serialization.
//!
//! This module deliberately knows nothing about candidate assessment values.

use std::collections::HashSet;

use anyhow::{bail, Context, Result};
use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Number, Value};

const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Inclusive limits for one JSON input.
///
/// The root is one value at depth one. Object member names are not values;
/// decoded member names and decoded string values both use `max_string_bytes`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ParseLimits {
    pub(crate) max_bytes: usize,
    pub(crate) max_depth: usize,
    pub(crate) max_values: usize,
    pub(crate) max_string_bytes: usize,
}

impl ParseLimits {
    pub(crate) const fn new(
        max_bytes: usize,
        max_depth: usize,
        max_values: usize,
        max_string_bytes: usize,
    ) -> Self {
        Self {
            max_bytes,
            max_depth,
            max_values,
            max_string_bytes,
        }
    }

    pub(crate) const SCHEMA: Self = Self::new(1_048_576, 128, 100_000, 1_048_576);
}

/// Parse exactly one bounded, duplicate-safe I-JSON value.
pub(crate) fn parse_ijson(source: &[u8], limits: ParseLimits) -> Result<Value> {
    if source.len() > limits.max_bytes {
        bail!(
            "JSON source is {} bytes; limit is {}",
            source.len(),
            limits.max_bytes
        );
    }

    let mut state = ParseState { limits, values: 0 };
    let mut deserializer = serde_json::Deserializer::from_slice(source);
    let value = ValueSeed {
        state: &mut state,
        depth: 1,
    }
    .deserialize(&mut deserializer)
    .map_err(|error| anyhow::anyhow!("invalid I-JSON: {error}"))?;
    deserializer
        .end()
        .map_err(|error| anyhow::anyhow!("trailing data after the I-JSON value: {error}"))?;
    Ok(value)
}

/// Serialize one candidate-number-admitted I-JSON value using RFC 8785 JCS.
pub(crate) fn jcs(value: &Value) -> Result<Vec<u8>> {
    validate_materialized_ijson(value)?;
    serde_json_canonicalizer::to_vec(value).context("RFC 8785 serialization failed")
}

struct ParseState {
    limits: ParseLimits,
    values: usize,
}

impl ParseState {
    fn enter_value(&mut self, depth: usize) -> std::result::Result<(), String> {
        if depth > self.limits.max_depth {
            return Err(format!(
                "JSON nesting depth {depth} exceeds limit {}",
                self.limits.max_depth
            ));
        }
        self.values = self
            .values
            .checked_add(1)
            .ok_or_else(|| "JSON value count overflow".to_string())?;
        if self.values > self.limits.max_values {
            return Err(format!(
                "JSON value count exceeds limit {}",
                self.limits.max_values
            ));
        }
        Ok(())
    }

    fn child_depth(&self, depth: usize) -> std::result::Result<usize, String> {
        depth
            .checked_add(1)
            .ok_or_else(|| "JSON nesting depth overflow".to_string())
    }

    fn check_string(&self, value: &str) -> std::result::Result<(), String> {
        if value.len() > self.limits.max_string_bytes {
            return Err(format!(
                "decoded JSON string is {} bytes; limit is {}",
                value.len(),
                self.limits.max_string_bytes
            ));
        }
        check_unicode_scalars(value)
    }
}

struct ValueSeed<'a> {
    state: &'a mut ParseState,
    depth: usize,
}

impl<'de> DeserializeSeed<'de> for ValueSeed<'_> {
    type Value = Value;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        self.state
            .enter_value(self.depth)
            .map_err(D::Error::custom)?;
        deserializer.deserialize_any(ValueVisitor {
            state: self.state,
            depth: self.depth,
        })
    }
}

struct ValueVisitor<'a> {
    state: &'a mut ParseState,
    depth: usize,
}

impl<'de> Visitor<'de> for ValueVisitor<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an I-JSON value")
    }

    fn visit_bool<E>(self, value: bool) -> std::result::Result<Self::Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E> {
        Ok(Value::Number(Number::from(value)))
    }

    fn visit_f64<E>(self, value: f64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| E::custom("I-JSON numbers must be finite binary64 values"))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.state.check_string(value).map_err(E::custom)?;
        Ok(Value::String(value.to_owned()))
    }

    fn visit_string<E>(self, value: String) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.state.check_string(&value).map_err(E::custom)?;
        Ok(Value::String(value))
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A>(self, mut sequence: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let child_depth = self
            .state
            .child_depth(self.depth)
            .map_err(A::Error::custom)?;
        let mut values = Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(4096));
        while let Some(value) = sequence.next_element_seed(ValueSeed {
            state: &mut *self.state,
            depth: child_depth,
        })? {
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A>(self, mut object: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let child_depth = self
            .state
            .child_depth(self.depth)
            .map_err(A::Error::custom)?;
        let capacity = object.size_hint().unwrap_or(0).min(4096);
        let mut names = HashSet::with_capacity(capacity);
        let mut values = Map::new();

        while let Some(name) = object.next_key::<String>()? {
            self.state.check_string(&name).map_err(A::Error::custom)?;
            if !names.insert(name.clone()) {
                return Err(A::Error::custom(format!(
                    "duplicate JSON object member {name:?}"
                )));
            }
            let value = object.next_value_seed(ValueSeed {
                state: &mut *self.state,
                depth: child_depth,
            })?;
            values.insert(name, value);
        }
        Ok(Value::Object(values))
    }
}

fn validate_materialized_ijson(value: &Value) -> Result<()> {
    match value {
        Value::Null | Value::Bool(_) => Ok(()),
        Value::Number(number) => validate_materialized_number(number),
        Value::String(value) => check_unicode_scalars(value).map_err(anyhow::Error::msg),
        Value::Array(values) => {
            for value in values {
                validate_materialized_ijson(value)?;
            }
            Ok(())
        }
        Value::Object(values) => {
            for (name, value) in values {
                check_unicode_scalars(name).map_err(anyhow::Error::msg)?;
                validate_materialized_ijson(value)?;
            }
            Ok(())
        }
    }
}

fn validate_materialized_number(number: &Number) -> Result<()> {
    if let Some(value) = number.as_u64() {
        if value <= MAX_SAFE_INTEGER {
            return Ok(());
        }
        bail!("integer exceeds the candidate SafeInt domain");
    }
    if number.as_i64().is_some() {
        bail!("negative integer is outside the candidate SafeInt domain");
    }
    if !number.as_f64().is_some_and(f64::is_finite) {
        bail!("number is not a finite binary64 value")
    }
    let canonical = serde_json_canonicalizer::to_string(number)
        .context("cannot establish canonical number spelling")?;
    let integer = canonical
        .strip_prefix('-')
        .unwrap_or(&canonical)
        .bytes()
        .all(|byte| byte.is_ascii_digit());
    if !integer {
        return Ok(());
    }
    if canonical.starts_with('-') {
        bail!("canonical number spelling is a negative integer");
    }
    let value = canonical
        .parse::<u64>()
        .context("canonical integer does not fit u64")?;
    if value <= MAX_SAFE_INTEGER {
        Ok(())
    } else {
        bail!("canonical integer exceeds the candidate SafeInt domain")
    }
}

fn check_unicode_scalars(value: &str) -> std::result::Result<(), String> {
    if let Some(character) = value.chars().find(|character| is_noncharacter(*character)) {
        return Err(format!(
            "I-JSON string contains Unicode noncharacter U+{:04X}",
            character as u32
        ));
    }
    Ok(())
}

fn is_noncharacter(character: char) -> bool {
    let value = character as u32;
    (0xFDD0..=0xFDEF).contains(&value) || value & 0xFFFF == 0xFFFE || value & 0xFFFF == 0xFFFF
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const SMALL: ParseLimits = ParseLimits::new(1024, 8, 32, 32);

    #[test]
    fn parses_one_duplicate_safe_ijson_value() {
        let value = parse_ijson(br#" { "b": [true, null], "a": "x" } "#, SMALL).unwrap();
        assert_eq!(value, json!({"a": "x", "b": [true, null]}));
    }

    #[test]
    fn rejects_duplicates_before_later_syntax() {
        let error = parse_ijson(br#"{"a":0,"a":1,]}"#, SMALL)
            .unwrap_err()
            .to_string();
        assert!(error.contains("duplicate"), "{error}");
    }

    #[test]
    fn rejects_nested_duplicates() {
        assert!(parse_ijson(br#"{"x":{"a":0,"a":1}}"#, SMALL).is_err());
    }

    #[test]
    fn rejects_invalid_utf8_surrogates_noncharacters_and_out_of_range_numbers() {
        assert!(parse_ijson(b"\"\xff\"", SMALL).is_err());
        assert!(parse_ijson(br#""\uD800""#, SMALL).is_err());
        assert!(parse_ijson(br#""\uFFFF""#, SMALL).is_err());
        assert!(parse_ijson(b"\"\xEF\xB7\x90\"", SMALL).is_err());
        assert!(parse_ijson(b"1e400", SMALL).is_err());
    }

    #[test]
    fn accepts_unsafe_candidate_integer_at_the_neutral_parse_stage() {
        assert_eq!(
            parse_ijson(b"9007199254740992", SMALL).unwrap(),
            json!(9_007_199_254_740_992_u64)
        );
    }

    #[test]
    fn limits_are_inclusive_and_use_the_frozen_counting_convention() {
        let exact = ParseLimits::new(9, 2, 3, 1);
        assert_eq!(parse_ijson(br#"["a",0]"#, exact).unwrap(), json!(["a", 0]));
        assert!(parse_ijson(br#"[[0]]"#, exact).is_err());
        assert!(parse_ijson(br#"[0,1,2]"#, exact).is_err());
        assert!(parse_ijson(br#"{"aa":0}"#, exact).is_err());
        assert!(parse_ijson(br#""aa""#, exact).is_err());
    }

    #[test]
    fn rejects_trailing_data() {
        assert!(parse_ijson(b"{} []", SMALL).is_err());
    }

    #[test]
    fn emits_rfc_8785_bytes() {
        let value = parse_ijson(br#"{"b":1e30,"a":"x"}"#, SMALL).unwrap();
        assert_eq!(jcs(&value).unwrap(), br#"{"a":"x","b":1e+30}"#);
    }

    #[test]
    fn jcs_rejects_programmatically_created_noncharacters() {
        assert!(jcs(&Value::String('\u{FFFF}'.to_string())).is_err());
    }

    #[test]
    fn jcs_requires_integer_domain_proof_before_canonicalization() {
        assert!(jcs(&json!(9_007_199_254_740_991_u64)).is_ok());
        assert!(jcs(&json!(9_007_199_254_740_992_u64)).is_err());
        assert!(jcs(&json!(-1_i64)).is_err());
        assert!(jcs(&json!(-9_007_199_254_740_992_i64)).is_err());
        assert!(jcs(&json!(-1.0_f64)).is_err());
        assert!(jcs(&json!(-1.5_f64)).is_ok());
        assert!(jcs(&json!(9_007_199_254_740_992.0_f64)).is_err());
        assert!(jcs(&json!(1e20_f64)).is_err());
        assert!(jcs(&json!(1e30_f64)).is_ok());
    }
}
