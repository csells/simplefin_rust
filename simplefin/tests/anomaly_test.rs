use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use simplefin::anomaly::{detect_anomalies, Anomaly};
use simplefin::models::{Account, Organization};

fn test_org() -> Organization {
    Organization {
        sfin_url: "https://example.com/org1".to_string(),
        domain: Some("example.com".to_string()),
        name: Some("Test Bank".to_string()),
        url: None,
        id: Some("org1".to_string()),
    }
}

fn make_account(id: &str, name: &str, balance: Decimal) -> Account {
    Account {
        org: test_org(),
        id: id.to_string(),
        name: name.to_string(),
        currency: "USD".to_string(),
        balance,
        available_balance: None,
        balance_date: 1000,
        transactions: Vec::new(),
    }
}

#[test]
fn detects_balance_dropped_to_zero() {
    let previous = vec![make_account("1", "Checking", dec!(1000))];
    let current = vec![make_account("1", "Checking", dec!(0))];
    let anomalies = detect_anomalies(&current, &previous);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(&anomalies[0], Anomaly::BalanceDroppedToZero { .. }));
}

#[test]
fn detects_large_balance_increase() {
    let previous = vec![make_account("1", "Brokerage", dec!(1000))];
    let current = vec![make_account("1", "Brokerage", dec!(1500))];
    let anomalies = detect_anomalies(&current, &previous);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(
        &anomalies[0],
        Anomaly::LargeBalanceChange { .. }
    ));
}

#[test]
fn detects_large_balance_decrease() {
    let previous = vec![make_account("1", "Brokerage", dec!(1000))];
    let current = vec![make_account("1", "Brokerage", dec!(700))];
    let anomalies = detect_anomalies(&current, &previous);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(
        &anomalies[0],
        Anomaly::LargeBalanceChange { .. }
    ));
}

#[test]
fn ignores_small_balance_change() {
    let previous = vec![make_account("1", "Checking", dec!(1000))];
    let current = vec![make_account("1", "Checking", dec!(950))];
    let anomalies = detect_anomalies(&current, &previous);
    assert!(anomalies.is_empty());
}

#[test]
fn detects_disappeared_account() {
    let previous = vec![make_account("1", "Old Account", dec!(500))];
    let current = vec![];
    let anomalies = detect_anomalies(&current, &previous);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(
        &anomalies[0],
        Anomaly::AccountDisappeared { .. }
    ));
}

#[test]
fn detects_new_account() {
    let previous = vec![];
    let current = vec![make_account("1", "New Account", dec!(1000))];
    let anomalies = detect_anomalies(&current, &previous);
    assert_eq!(anomalies.len(), 1);
    assert!(matches!(&anomalies[0], Anomaly::NewAccount { .. }));
}

#[test]
fn no_anomalies_when_stable() {
    let accounts = vec![make_account("1", "Checking", dec!(1000))];
    let anomalies = detect_anomalies(&accounts, &accounts);
    assert!(anomalies.is_empty());
}

#[test]
fn handles_zero_previous_balance() {
    let previous = vec![make_account("1", "Empty", dec!(0))];
    let current = vec![make_account("1", "Empty", dec!(1000))];
    let anomalies = detect_anomalies(&current, &previous);
    // Should not flag as large change (would be div by zero), and not dropped to zero
    assert!(anomalies.is_empty());
}
