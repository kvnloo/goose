use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const ACTIVITY_IDENTITY_KEY_VERSION: &str = "v1";
pub const ACTIVITY_IDENTITY_KEY_PREFIX: &str = "goose:activity-session:v1:";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActivityIdentityInput {
    pub source: String,
    pub provenance: Value,
    pub start_time: String,
    pub end_time: String,
    pub activity_type: String,
    #[serde(default)]
    pub raw_identifiers: Vec<String>,
    #[serde(default)]
    pub labels: Vec<String>,
}

pub fn activity_idempotency_key(input: &ActivityIdentityInput) -> String {
    let canonical_payload = canonical_activity_identity_payload(input);
    let digest = Sha256::digest(canonical_payload.as_bytes());
    format!("{ACTIVITY_IDENTITY_KEY_PREFIX}{}", hex::encode(digest))
}

fn canonical_activity_identity_payload(input: &ActivityIdentityInput) -> String {
    let payload = Value::Object(
        [
            (
                "activity_type".to_string(),
                Value::String(normalized_marker(&input.activity_type)),
            ),
            (
                "end_time".to_string(),
                Value::String(canonical_time(&input.end_time)),
            ),
            (
                "labels".to_string(),
                Value::Array(
                    canonical_labels(&input.labels)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            ),
            ("provenance".to_string(), input.provenance.clone()),
            (
                "raw_identifiers".to_string(),
                Value::Array(
                    canonical_raw_identifiers(&input.raw_identifiers)
                        .into_iter()
                        .map(Value::String)
                        .collect(),
                ),
            ),
            (
                "source".to_string(),
                Value::String(input.source.trim().to_string()),
            ),
            (
                "start_time".to_string(),
                Value::String(canonical_time(&input.start_time)),
            ),
            (
                "version".to_string(),
                Value::String(ACTIVITY_IDENTITY_KEY_VERSION.to_string()),
            ),
        ]
        .into_iter()
        .collect(),
    );
    canonical_json(&payload)
}

fn canonical_raw_identifiers(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_labels(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalized_marker(value))
        .filter(|value| !value.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => {
            serde_json::to_string(value).expect("JSON strings are always serializable")
        }
        Value::Array(values) => {
            let mut output = String::from("[");
            for (index, item) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&canonical_json(item));
            }
            output.push(']');
            output
        }
        Value::Object(map) => {
            let mut keys = map.keys().collect::<Vec<_>>();
            keys.sort();

            let mut output = String::from("{");
            for (index, key) in keys.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(
                    &serde_json::to_string(key).expect("JSON object keys are always serializable"),
                );
                output.push(':');
                output.push_str(&canonical_json(
                    map.get(*key).expect("sorted key must exist in map"),
                ));
            }
            output.push('}');
            output
        }
    }
}

fn canonical_time(value: &str) -> String {
    parse_utc_instant(value)
        .map(|instant| instant.0.to_string())
        .unwrap_or_else(|| value.trim().to_string())
}

fn normalized_marker(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|char| {
            if char.is_ascii_alphanumeric() {
                char
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct UtcInstant(i128);

fn parse_utc_instant(value: &str) -> Option<UtcInstant> {
    crate::rfc3339::parse_utc_instant_nanos(value).map(UtcInstant)
}
