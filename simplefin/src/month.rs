//! Calendar months in UTC, used to bucket transactions by `YYYY-MM`.

use std::fmt;
use std::str::FromStr;

use chrono::{DateTime, Datelike, NaiveDate};

use crate::error::SimplefinError;

const SECONDS_PER_DAY: i64 = 86_400;

/// A UTC calendar month. Orders chronologically; displays and parses as `YYYY-MM`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct YearMonth {
    year: i32,
    month: u32,
}

impl YearMonth {
    /// Create a month; `month` must be 1–12.
    pub fn new(year: i32, month: u32) -> Option<Self> {
        (1..=12).contains(&month).then_some(Self { year, month })
    }

    /// The UTC month containing an epoch timestamp.
    pub fn from_epoch(ts: i64) -> Self {
        let dt = DateTime::from_timestamp(ts, 0).unwrap_or_default();
        Self {
            year: dt.year(),
            month: dt.month(),
        }
    }

    pub fn year(&self) -> i32 {
        self.year
    }

    pub fn month(&self) -> u32 {
        self.month
    }

    /// The following month.
    pub fn next(self) -> Self {
        if self.month == 12 {
            Self {
                year: self.year + 1,
                month: 1,
            }
        } else {
            Self {
                month: self.month + 1,
                ..self
            }
        }
    }

    /// The preceding month.
    pub fn prev(self) -> Self {
        if self.month == 1 {
            Self {
                year: self.year - 1,
                month: 12,
            }
        } else {
            Self {
                month: self.month - 1,
                ..self
            }
        }
    }

    /// The month `n` months earlier.
    pub fn minus_months(self, n: u32) -> Self {
        (0..n).fold(self, |m, _| m.prev())
    }

    /// Epoch seconds at 00:00:00 UTC on the first day of the month.
    pub fn start_epoch(&self) -> i64 {
        NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc().timestamp())
            .unwrap_or_default()
    }

    /// Epoch seconds at the start of the next month (exclusive end).
    pub fn end_epoch(&self) -> i64 {
        self.next().start_epoch()
    }

    /// Number of days in the month.
    pub fn days(&self) -> u32 {
        ((self.end_epoch() - self.start_epoch()) / SECONDS_PER_DAY) as u32
    }

    /// Whether an epoch timestamp falls in this month.
    pub fn contains(&self, ts: i64) -> bool {
        ts >= self.start_epoch() && ts < self.end_epoch()
    }

    /// Every month from `start` through `end`, inclusive (empty if `start > end`).
    pub fn range_inclusive(start: Self, end: Self) -> Vec<Self> {
        let mut months = Vec::new();
        let mut m = start;
        while m <= end {
            months.push(m);
            m = m.next();
        }
        months
    }
}

impl fmt::Display for YearMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}", self.year, self.month)
    }
}

impl FromStr for YearMonth {
    type Err = SimplefinError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid =
            || SimplefinError::InvalidArgument(format!("invalid month \"{s}\", expected YYYY-MM"));
        let (year, month) = s.trim().split_once('-').ok_or_else(invalid)?;
        let year = year.parse().map_err(|_| invalid())?;
        let month = month.parse().map_err(|_| invalid())?;
        Self::new(year, month).ok_or_else(invalid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_epoch_uses_utc() {
        // 2024-03-15 00:00:00 UTC
        assert_eq!(YearMonth::from_epoch(1_710_460_800).to_string(), "2024-03");
        // 2024-02-29 23:59:59 UTC is still February
        assert_eq!(YearMonth::from_epoch(1_709_251_199).to_string(), "2024-02");
    }

    #[test]
    fn next_and_prev_wrap_years() {
        let dec = YearMonth::new(2023, 12).unwrap();
        assert_eq!(dec.next(), YearMonth::new(2024, 1).unwrap());
        assert_eq!(dec.next().prev(), dec);
        assert_eq!(YearMonth::new(2024, 3).unwrap().minus_months(3), dec);
    }

    #[test]
    fn epochs_and_days() {
        let feb = YearMonth::new(2024, 2).unwrap();
        assert_eq!(feb.start_epoch(), 1_706_745_600);
        assert_eq!(feb.days(), 29);
        assert!(feb.contains(feb.start_epoch()));
        assert!(!feb.contains(feb.end_epoch()));
        assert_eq!(YearMonth::new(2023, 2).unwrap().days(), 28);
    }

    #[test]
    fn parses_and_rejects() {
        assert_eq!(
            "2024-07".parse::<YearMonth>().unwrap(),
            YearMonth::new(2024, 7).unwrap()
        );
        assert!("2024-13".parse::<YearMonth>().is_err());
        assert!("July".parse::<YearMonth>().is_err());
    }

    #[test]
    fn inclusive_range() {
        let a = YearMonth::new(2023, 11).unwrap();
        let b = YearMonth::new(2024, 2).unwrap();
        let labels: Vec<String> = YearMonth::range_inclusive(a, b)
            .iter()
            .map(|m| m.to_string())
            .collect();
        assert_eq!(labels, ["2023-11", "2023-12", "2024-01", "2024-02"]);
        assert!(YearMonth::range_inclusive(b, a).is_empty());
    }
}
