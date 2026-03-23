use chrono::{DateTime, SecondsFormat, Utc};

/// Converts epoch seconds to a `DateTime<Utc>`.
pub fn epoch_to_datetime(epoch: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(epoch, 0).unwrap_or_default()
}

/// Formats a `DateTime<Utc>` as an ISO-8601 string with `Z` suffix.
pub fn to_iso8601(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339_opts(SecondsFormat::Secs, true)
}
