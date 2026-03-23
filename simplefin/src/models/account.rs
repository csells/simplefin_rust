use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::datetime_utils::{epoch_to_datetime, to_iso8601};

use super::organization::Organization;
use super::serde_helpers::{deserialize_decimal, deserialize_optional_decimal};
use super::transaction::Transaction;

/// Represents a financial account exposed by a SimpleFIN server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Organization that owns the account.
    pub org: Organization,

    /// Account identifier assigned by the provider.
    pub id: String,

    /// Human-friendly account name.
    pub name: String,

    /// ISO-4217 currency code for monetary values.
    pub currency: String,

    /// Current posted balance.
    #[serde(deserialize_with = "deserialize_decimal")]
    pub balance: Decimal,

    /// Provider-reported available balance, when supplied.
    #[serde(
        rename = "available-balance",
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_optional_decimal"
    )]
    pub available_balance: Option<Decimal>,

    /// Timestamp when the balance was last refreshed (epoch seconds).
    #[serde(rename = "balance-date")]
    pub balance_date: i64,

    /// Transactions returned alongside the account.
    #[serde(default)]
    pub transactions: Vec<Transaction>,
}

impl Account {
    /// Returns the balance date as a `DateTime<Utc>`.
    pub fn balance_datetime(&self) -> DateTime<Utc> {
        epoch_to_datetime(self.balance_date)
    }

    /// Returns the balance date formatted as an ISO-8601 string with Z suffix.
    pub fn balance_date_iso8601(&self) -> String {
        to_iso8601(self.balance_datetime())
    }
}
