use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::datetime_utils::{epoch_to_datetime, to_iso8601};

use super::serde_helpers::{deserialize_decimal, deserialize_pending};

/// Immutable view of a transaction within a SimpleFIN account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Identifier for the transaction.
    pub id: String,

    /// Date the transaction posted (epoch seconds).
    pub posted: i64,

    /// Monetary amount of the transaction.
    #[serde(deserialize_with = "deserialize_decimal")]
    pub amount: Decimal,

    /// Provider-supplied description.
    pub description: String,

    /// Timestamp when the transaction occurred, when provided (epoch seconds).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transacted_at: Option<i64>,

    /// Indicates whether the transaction is still pending.
    #[serde(default, deserialize_with = "deserialize_pending")]
    pub pending: bool,
}

impl Transaction {
    /// Returns the posted date as a `DateTime<Utc>`.
    pub fn posted_datetime(&self) -> DateTime<Utc> {
        epoch_to_datetime(self.posted)
    }

    /// Returns the posted date formatted as an ISO-8601 string with Z suffix.
    pub fn posted_iso8601(&self) -> String {
        to_iso8601(self.posted_datetime())
    }

    /// Returns the transacted_at date as a `DateTime<Utc>`, if present.
    pub fn transacted_at_datetime(&self) -> Option<DateTime<Utc>> {
        self.transacted_at.map(epoch_to_datetime)
    }

    /// Returns the transacted_at date formatted as an ISO-8601 string with Z suffix.
    pub fn transacted_at_iso8601(&self) -> Option<String> {
        self.transacted_at_datetime().map(to_iso8601)
    }
}
