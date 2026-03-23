use rust_decimal::Decimal;
use serde::{Deserialize, Deserializer};

/// Deserializes a `Decimal` from either a JSON string or number.
pub fn deserialize_decimal<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match &value {
        serde_json::Value::String(s) => {
            s.parse::<Decimal>().map_err(serde::de::Error::custom)
        }
        serde_json::Value::Number(n) => {
            let s = n.to_string();
            s.parse::<Decimal>().map_err(serde::de::Error::custom)
        }
        _ => Err(serde::de::Error::custom(
            "expected string or number for decimal",
        )),
    }
}

/// Deserializes an optional `Decimal` from either a JSON string or number.
pub fn deserialize_optional_decimal<'de, D>(
    deserializer: D,
) -> Result<Option<Decimal>, D::Error>
where
    D: Deserializer<'de>,
{
    let value: Option<serde_json::Value> = Option::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::String(s)) => s
            .parse::<Decimal>()
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(serde_json::Value::Number(n)) => {
            let s = n.to_string();
            s.parse::<Decimal>()
                .map(Some)
                .map_err(serde::de::Error::custom)
        }
        _ => Err(serde::de::Error::custom(
            "expected string or number for decimal",
        )),
    }
}

/// Deserializes a bool from a JSON boolean, number (0 = false), or null (false).
pub fn deserialize_pending<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    match &value {
        serde_json::Value::Bool(b) => Ok(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i != 0)
            } else if let Some(f) = n.as_f64() {
                Ok(f != 0.0)
            } else {
                Err(serde::de::Error::custom(
                    "unexpected number format for pending",
                ))
            }
        }
        serde_json::Value::Null => Ok(false),
        _ => Err(serde::de::Error::custom(
            "expected bool or number for pending",
        )),
    }
}
