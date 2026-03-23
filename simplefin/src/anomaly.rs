use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::models::Account;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Anomaly {
    BalanceDroppedToZero {
        account_id: String,
        account_name: String,
        previous_balance: Decimal,
    },
    LargeBalanceChange {
        account_id: String,
        account_name: String,
        previous_balance: Decimal,
        current_balance: Decimal,
        change_percent: Decimal,
    },
    AccountDisappeared {
        account_id: String,
        account_name: String,
        last_known_balance: Decimal,
    },
    NewAccount {
        account_id: String,
        account_name: String,
        balance: Decimal,
    },
}

impl std::fmt::Display for Anomaly {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BalanceDroppedToZero {
                account_name,
                previous_balance,
                ..
            } => {
                write!(
                    f,
                    "WARNING: {account_name} balance dropped to $0 (was {previous_balance})"
                )
            }
            Self::LargeBalanceChange {
                account_name,
                previous_balance,
                current_balance,
                change_percent,
                ..
            } => {
                write!(
                    f,
                    "WARNING: {account_name} balance changed {change_percent}% ({previous_balance} -> {current_balance})"
                )
            }
            Self::AccountDisappeared {
                account_name,
                last_known_balance,
                ..
            } => {
                write!(
                    f,
                    "WARNING: {account_name} disappeared (last balance: {last_known_balance})"
                )
            }
            Self::NewAccount {
                account_name,
                balance,
                ..
            } => {
                write!(
                    f,
                    "NOTE: New account appeared: {account_name} (balance: {balance})"
                )
            }
        }
    }
}

/// Detect anomalies by comparing current accounts against previous accounts.
/// The threshold for "large" balance changes is 20%.
pub fn detect_anomalies(
    current_accounts: &[Account],
    previous_accounts: &[Account],
) -> Vec<Anomaly> {
    let mut anomalies = Vec::new();
    let threshold = Decimal::new(20, 0); // 20%

    // Check each current account against previous
    for current in current_accounts {
        if let Some(previous) = previous_accounts.iter().find(|a| a.id == current.id) {
            // Balance dropped to zero
            if current.balance == Decimal::ZERO && previous.balance != Decimal::ZERO {
                anomalies.push(Anomaly::BalanceDroppedToZero {
                    account_id: current.id.clone(),
                    account_name: current.name.clone(),
                    previous_balance: previous.balance,
                });
                continue;
            }

            // Large balance change (only if previous balance is nonzero to avoid div by zero)
            if previous.balance != Decimal::ZERO {
                let change = current.balance - previous.balance;
                let pct = (change * Decimal::ONE_HUNDRED / previous.balance).abs();
                if pct > threshold {
                    anomalies.push(Anomaly::LargeBalanceChange {
                        account_id: current.id.clone(),
                        account_name: current.name.clone(),
                        previous_balance: previous.balance,
                        current_balance: current.balance,
                        change_percent: pct.round_dp(1),
                    });
                }
            }
        } else {
            // New account
            anomalies.push(Anomaly::NewAccount {
                account_id: current.id.clone(),
                account_name: current.name.clone(),
                balance: current.balance,
            });
        }
    }

    // Check for disappeared accounts
    for previous in previous_accounts {
        if !current_accounts.iter().any(|a| a.id == previous.id) {
            anomalies.push(Anomaly::AccountDisappeared {
                account_id: previous.id.clone(),
                account_name: previous.name.clone(),
                last_known_balance: previous.balance,
            });
        }
    }

    anomalies
}
