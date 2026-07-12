//! Exact raw-byte and domain-separated SHA-256 identities.

use std::fmt;
use std::str::FromStr;

use anyhow::{bail, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use sha2::{Digest as _, Sha256};

use crate::json::jcs;

/// A SHA-256 digest with the protocol's sole textual representation.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Digest([u8; 32]);

impl Digest {
    pub(crate) const PREFIX: &'static str = "sha256:";
}

impl fmt::Debug for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, formatter)
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(Self::PREFIX)?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl FromStr for Digest {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let Some(hexadecimal) = value.strip_prefix(Self::PREFIX) else {
            bail!("digest must begin with sha256:");
        };
        if hexadecimal.len() != 64 {
            bail!("SHA-256 digest must contain exactly 64 hexadecimal digits");
        }

        let mut bytes = [0_u8; 32];
        for (index, pair) in hexadecimal.as_bytes().chunks_exact(2).enumerate() {
            bytes[index] = decode_nibble(pair[0])?
                .checked_mul(16)
                .and_then(|high| high.checked_add(decode_nibble(pair[1]).ok()?))
                .ok_or_else(|| anyhow::anyhow!("invalid SHA-256 digest"))?;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Digest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}

/// Ordinary SHA-256 over exact retained bytes.
pub(crate) fn bytes_sha256(bytes: &[u8]) -> Digest {
    finish(Sha256::digest(bytes))
}

/// `SHA-256(UTF8(tag) || 0x00 || JCS(value))`.
pub(crate) fn h(tag: &str, value: &Value) -> Result<Digest> {
    let canonical = jcs(value)?;
    Ok(domain_separated(tag, &canonical))
}

/// `SHA-256(UTF8(tag) || 0x00 || bytes)`.
pub(crate) fn hb(tag: &str, bytes: &[u8]) -> Digest {
    domain_separated(tag, bytes)
}

fn domain_separated(tag: &str, suffix: &[u8]) -> Digest {
    let mut hash = Sha256::new();
    hash.update(tag.as_bytes());
    hash.update([0]);
    hash.update(suffix);
    finish(hash.finalize())
}

fn finish(bytes: impl AsRef<[u8]>) -> Digest {
    let mut digest = [0_u8; 32];
    digest.copy_from_slice(bytes.as_ref());
    Digest(digest)
}

fn decode_nibble(value: u8) -> Result<u8> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => bail!("digest must use lowercase hexadecimal"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn hashes_exact_bytes() {
        assert_eq!(
            bytes_sha256(b"abc").to_string(),
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn domain_separation_and_jcs_are_exact() {
        let value = json!({"b": 2, "a": 1});
        assert_eq!(h("tag", &value).unwrap(), hb("tag", br#"{"a":1,"b":2}"#));
        assert_ne!(h("tag", &value).unwrap(), h("other", &value).unwrap());
    }

    #[test]
    fn semantic_identity_ignores_input_member_order() {
        let left: Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
        let right: Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
        assert_eq!(
            h("revision", &left).unwrap(),
            h("revision", &right).unwrap()
        );
        assert_ne!(
            bytes_sha256(br#"{"a":1,"b":2}"#),
            bytes_sha256(br#"{"b":2,"a":1}"#)
        );
    }

    #[test]
    fn textual_form_is_strict_and_round_trips_through_serde() {
        let digest = bytes_sha256(b"x");
        assert_eq!(digest.to_string().parse::<Digest>().unwrap(), digest);
        assert!(digest.to_string().to_uppercase().parse::<Digest>().is_err());
        assert!("ba7816bf".parse::<Digest>().is_err());
        assert_eq!(
            serde_json::from_value::<Digest>(serde_json::to_value(digest).unwrap()).unwrap(),
            digest
        );
    }
}
