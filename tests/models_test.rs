use rust_decimal::Decimal;
use std::str::FromStr;

use simplefin::{Account, AccountSet, BridgeInfo, Organization, Transaction};

// ── BridgeInfo ──────────────────────────────────────────────────────────────

#[test]
fn bridge_info_parses_multiple_versions() {
    let json = r#"{"versions":["1.0-draft.1","1.0-draft.2","1.0"]}"#;
    let info: BridgeInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.versions, vec!["1.0-draft.1", "1.0-draft.2", "1.0"]);
}

#[test]
fn bridge_info_empty_versions() {
    let json = r#"{"versions":[]}"#;
    let info: BridgeInfo = serde_json::from_str(json).unwrap();
    assert!(info.versions.is_empty());
}

#[test]
fn bridge_info_single_version() {
    let json = r#"{"versions":["1.0-draft.1"]}"#;
    let info: BridgeInfo = serde_json::from_str(json).unwrap();
    assert_eq!(info.versions.len(), 1);
    assert_eq!(info.versions[0], "1.0-draft.1");
}

// ── Organization ────────────────────────────────────────────────────────────

#[test]
fn organization_full_fields() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "domain": "example.com",
        "name": "Example Bank",
        "url": "https://www.example.com",
        "id": "org-12345"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.sfin_url, "https://sfin.example.com");
    assert_eq!(org.domain.as_deref(), Some("example.com"));
    assert_eq!(org.name.as_deref(), Some("Example Bank"));
    assert_eq!(org.url.as_deref(), Some("https://www.example.com"));
    assert_eq!(org.id.as_deref(), Some("org-12345"));
}

#[test]
fn organization_minimal_fields() {
    let json = r#"{"sfin-url": "https://sfin.minimal.example"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.sfin_url, "https://sfin.minimal.example");
    assert!(org.domain.is_none());
    assert!(org.name.is_none());
    assert!(org.url.is_none());
    assert!(org.id.is_none());
}

#[test]
fn organization_display_name_prefers_name() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "name": "My Bank",
        "domain": "mybank.com",
        "id": "org-1"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.display_name(), "My Bank");
}

#[test]
fn organization_display_name_falls_back_to_domain() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "domain": "mybank.com",
        "id": "org-1"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.display_name(), "mybank.com");
}

#[test]
fn organization_display_name_falls_back_to_id() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "id": "org-1"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.display_name(), "org-1");
}

#[test]
fn organization_display_name_falls_back_to_sfin_url() {
    let json = r#"{"sfin-url": "https://sfin.example.com"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.display_name(), "https://sfin.example.com");
}

#[test]
fn organization_key_prefers_id() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "domain": "mybank.com",
        "id": "org-1"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.key(), "org-1");
}

#[test]
fn organization_key_falls_back_to_domain() {
    let json = r#"{
        "sfin-url": "https://sfin.example.com",
        "domain": "mybank.com"
    }"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.key(), "mybank.com");
}

#[test]
fn organization_key_falls_back_to_sfin_url() {
    let json = r#"{"sfin-url": "https://sfin.example.com"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    assert_eq!(org.key(), "https://sfin.example.com");
}

#[test]
fn organization_roundtrip_serialization() {
    let json = r#"{"sfin-url":"https://sfin.example.com","domain":"example.com","name":"Example Bank","url":"https://www.example.com","id":"org-12345"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    let reserialized = serde_json::to_string(&org).unwrap();
    let org2: Organization = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(org.sfin_url, org2.sfin_url);
    assert_eq!(org.domain, org2.domain);
    assert_eq!(org.name, org2.name);
    assert_eq!(org.url, org2.url);
    assert_eq!(org.id, org2.id);
}

#[test]
fn organization_minimal_roundtrip_omits_none_fields() {
    let json = r#"{"sfin-url":"https://sfin.example.com"}"#;
    let org: Organization = serde_json::from_str(json).unwrap();
    let reserialized = serde_json::to_string(&org).unwrap();
    // None fields should be omitted
    assert!(!reserialized.contains("domain"));
    assert!(!reserialized.contains("\"name\""));
    assert!(!reserialized.contains("\"url\""));
    assert!(!reserialized.contains("\"id\""));
}

// ── Transaction ─────────────────────────────────────────────────────────────

#[test]
fn transaction_full_fields() {
    let json = r#"{
        "id": "tx-001",
        "posted": 1706745600,
        "amount": "-45.99",
        "description": "GROCERY STORE #1234",
        "transacted_at": 1706659200,
        "pending": false,
        "extra": {"memo": "weekly groceries"}
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.id, "tx-001");
    assert_eq!(tx.posted, 1706745600);
    assert_eq!(tx.amount, Decimal::from_str("-45.99").unwrap());
    assert_eq!(tx.description, "GROCERY STORE #1234");
    assert_eq!(tx.transacted_at, Some(1706659200));
    assert!(!tx.pending);
    assert!(tx.extra.is_some());
    assert_eq!(tx.extra.as_ref().unwrap()["memo"], "weekly groceries");
}

#[test]
fn transaction_minimal_fields() {
    let json = r#"{
        "id": "tx-002",
        "posted": 1706745600,
        "amount": "100.00",
        "description": "PAYROLL DEPOSIT"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.id, "tx-002");
    assert_eq!(tx.posted, 1706745600);
    assert_eq!(tx.amount, Decimal::from_str("100.00").unwrap());
    assert_eq!(tx.description, "PAYROLL DEPOSIT");
    assert!(tx.transacted_at.is_none());
    assert!(!tx.pending); // default false
    assert!(tx.extra.is_none());
}

#[test]
fn transaction_amount_as_number() {
    let json = r#"{
        "id": "tx-003",
        "posted": 1706745600,
        "amount": -12.50,
        "description": "COFFEE SHOP"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("-12.50").unwrap());
}

#[test]
fn transaction_amount_as_string() {
    let json = r#"{
        "id": "tx-004",
        "posted": 1706745600,
        "amount": "-12.50",
        "description": "COFFEE SHOP"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("-12.50").unwrap());
}

#[test]
fn transaction_pending_as_true() {
    let json = r#"{
        "id": "tx-005",
        "posted": 1706745600,
        "amount": "-5.00",
        "description": "PENDING TX",
        "pending": true
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert!(tx.pending);
}

#[test]
fn transaction_pending_as_number_one() {
    let json = r#"{
        "id": "tx-006",
        "posted": 1706745600,
        "amount": "-5.00",
        "description": "PENDING TX",
        "pending": 1
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert!(tx.pending);
}

#[test]
fn transaction_pending_as_number_zero() {
    let json = r#"{
        "id": "tx-007",
        "posted": 1706745600,
        "amount": "-5.00",
        "description": "NOT PENDING TX",
        "pending": 0
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert!(!tx.pending);
}

#[test]
fn transaction_pending_as_null() {
    let json = r#"{
        "id": "tx-008",
        "posted": 1706745600,
        "amount": "-5.00",
        "description": "NULL PENDING TX",
        "pending": null
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert!(!tx.pending);
}

#[test]
fn transaction_posted_iso8601() {
    let json = r#"{
        "id": "tx-009",
        "posted": 1706745600,
        "amount": "0",
        "description": "TEST"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    // 1706745600 = 2024-02-01T00:00:00Z
    assert_eq!(tx.posted_iso8601(), "2024-02-01T00:00:00Z");
}

#[test]
fn transaction_transacted_at_iso8601() {
    let json = r#"{
        "id": "tx-010",
        "posted": 1706745600,
        "amount": "0",
        "description": "TEST",
        "transacted_at": 1706659200
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    // 1706659200 = 2024-01-31T00:00:00Z
    assert_eq!(tx.transacted_at_iso8601(), Some("2024-01-31T00:00:00Z".to_string()));
}

#[test]
fn transaction_transacted_at_absent() {
    let json = r#"{
        "id": "tx-011",
        "posted": 1706745600,
        "amount": "0",
        "description": "TEST"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert!(tx.transacted_at_iso8601().is_none());
    assert!(tx.transacted_at_datetime().is_none());
}

#[test]
fn transaction_zero_epoch() {
    let json = r#"{
        "id": "tx-012",
        "posted": 0,
        "amount": "0",
        "description": "EPOCH ZERO"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.posted, 0);
    assert_eq!(tx.posted_iso8601(), "1970-01-01T00:00:00Z");
}

#[test]
fn transaction_large_amount() {
    let json = r#"{
        "id": "tx-013",
        "posted": 1706745600,
        "amount": "999999999.99",
        "description": "BIG AMOUNT"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("999999999.99").unwrap());
}

#[test]
fn transaction_negative_large_amount() {
    let json = r#"{
        "id": "tx-014",
        "posted": 1706745600,
        "amount": "-999999999.99",
        "description": "BIG NEGATIVE"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("-999999999.99").unwrap());
}

#[test]
fn transaction_zero_amount() {
    let json = r#"{
        "id": "tx-015",
        "posted": 1706745600,
        "amount": "0.00",
        "description": "ZERO"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("0.00").unwrap());
}

#[test]
fn transaction_amount_many_decimal_places() {
    let json = r#"{
        "id": "tx-016",
        "posted": 1706745600,
        "amount": "123.456789",
        "description": "PRECISE"
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    assert_eq!(tx.amount, Decimal::from_str("123.456789").unwrap());
}

#[test]
fn transaction_roundtrip_serialization() {
    let json = r#"{
        "id": "tx-round",
        "posted": 1706745600,
        "amount": "-45.99",
        "description": "ROUND TRIP TEST",
        "transacted_at": 1706659200,
        "pending": true,
        "extra": {"category": "food"}
    }"#;
    let tx: Transaction = serde_json::from_str(json).unwrap();
    let reserialized = serde_json::to_string(&tx).unwrap();
    let tx2: Transaction = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(tx.id, tx2.id);
    assert_eq!(tx.posted, tx2.posted);
    assert_eq!(tx.amount, tx2.amount);
    assert_eq!(tx.description, tx2.description);
    assert_eq!(tx.transacted_at, tx2.transacted_at);
    assert_eq!(tx.pending, tx2.pending);
    assert_eq!(tx.extra, tx2.extra);
}

// ── Account ─────────────────────────────────────────────────────────────────

#[test]
fn account_full_fields_with_transactions() {
    let json = r#"{
        "org": {
            "sfin-url": "https://sfin.example.com",
            "domain": "example.com",
            "name": "Example Bank",
            "url": "https://www.example.com",
            "id": "org-12345"
        },
        "id": "acct-001",
        "name": "Checking Account",
        "currency": "USD",
        "balance": "1234.56",
        "available-balance": "1200.00",
        "balance-date": 1706745600,
        "transactions": [
            {
                "id": "tx-001",
                "posted": 1706745600,
                "amount": "-45.99",
                "description": "GROCERY STORE",
                "pending": false
            },
            {
                "id": "tx-002",
                "posted": 1706659200,
                "amount": "2500.00",
                "description": "PAYROLL",
                "pending": false
            }
        ],
        "extra": {"account_type": "checking"}
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.id, "acct-001");
    assert_eq!(account.name, "Checking Account");
    assert_eq!(account.currency, "USD");
    assert_eq!(account.balance, Decimal::from_str("1234.56").unwrap());
    assert_eq!(account.available_balance, Some(Decimal::from_str("1200.00").unwrap()));
    assert_eq!(account.balance_date, 1706745600);
    assert_eq!(account.transactions.len(), 2);
    assert_eq!(account.transactions[0].id, "tx-001");
    assert_eq!(account.transactions[0].amount, Decimal::from_str("-45.99").unwrap());
    assert_eq!(account.transactions[1].id, "tx-002");
    assert_eq!(account.transactions[1].amount, Decimal::from_str("2500.00").unwrap());
    assert!(account.extra.is_some());

    // Organization
    assert_eq!(account.org.sfin_url, "https://sfin.example.com");
    assert_eq!(account.org.name.as_deref(), Some("Example Bank"));
    assert_eq!(account.org.id.as_deref(), Some("org-12345"));
}

#[test]
fn account_minimal_no_transactions() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.minimal.example"},
        "id": "acct-002",
        "name": "Savings",
        "currency": "EUR",
        "balance": "5000.00",
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.id, "acct-002");
    assert_eq!(account.name, "Savings");
    assert_eq!(account.currency, "EUR");
    assert_eq!(account.balance, Decimal::from_str("5000.00").unwrap());
    assert!(account.available_balance.is_none());
    assert!(account.transactions.is_empty());
    assert!(account.extra.is_none());
}

#[test]
fn account_balance_as_number() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-003",
        "name": "Test",
        "currency": "USD",
        "balance": 1234.56,
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.balance, Decimal::from_str("1234.56").unwrap());
}

#[test]
fn account_available_balance_as_number() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-004",
        "name": "Test",
        "currency": "USD",
        "balance": "1000.00",
        "available-balance": 950.25,
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.available_balance, Some(Decimal::from_str("950.25").unwrap()));
}

#[test]
fn account_available_balance_null() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-005",
        "name": "Test",
        "currency": "USD",
        "balance": "1000.00",
        "available-balance": null,
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert!(account.available_balance.is_none());
}

#[test]
fn account_negative_balance() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-006",
        "name": "Credit Card",
        "currency": "USD",
        "balance": "-3456.78",
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.balance, Decimal::from_str("-3456.78").unwrap());
}

#[test]
fn account_balance_date_iso8601() {
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-007",
        "name": "Test",
        "currency": "USD",
        "balance": "0",
        "balance-date": 1706745600
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    assert_eq!(account.balance_date_iso8601(), "2024-02-01T00:00:00Z");
}

#[test]
fn account_roundtrip_serialization() {
    let json = r#"{
        "org": {
            "sfin-url": "https://sfin.example.com",
            "name": "Test Bank",
            "id": "org-1"
        },
        "id": "acct-round",
        "name": "Round Trip Account",
        "currency": "GBP",
        "balance": "999.99",
        "available-balance": "888.88",
        "balance-date": 1706745600,
        "transactions": [
            {
                "id": "tx-rt-1",
                "posted": 1706745600,
                "amount": "-10.00",
                "description": "TEST TX",
                "pending": true
            }
        ]
    }"#;
    let account: Account = serde_json::from_str(json).unwrap();
    let reserialized = serde_json::to_string(&account).unwrap();
    let account2: Account = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(account.id, account2.id);
    assert_eq!(account.name, account2.name);
    assert_eq!(account.currency, account2.currency);
    assert_eq!(account.balance, account2.balance);
    assert_eq!(account.available_balance, account2.available_balance);
    assert_eq!(account.balance_date, account2.balance_date);
    assert_eq!(account.transactions.len(), account2.transactions.len());
    assert_eq!(account.transactions[0].id, account2.transactions[0].id);
    assert_eq!(account.transactions[0].pending, account2.transactions[0].pending);
}

// ── AccountSet ──────────────────────────────────────────────────────────────

/// Full realistic API response matching SimpleFIN v1 wire format with multiple
/// accounts across two organizations, each with transactions.
const FULL_API_RESPONSE: &str = r#"{
    "errors": [
        "Connection to First National is degraded",
        "Scheduled maintenance window: 2024-02-01 02:00-04:00 UTC"
    ],
    "accounts": [
        {
            "org": {
                "sfin-url": "https://sfin.firstnational.example",
                "domain": "firstnational.example",
                "name": "First National Bank",
                "url": "https://www.firstnational.example",
                "id": "org-fn-001"
            },
            "id": "ACT-CHK-9876",
            "name": "Premier Checking",
            "currency": "USD",
            "balance": "4523.17",
            "available-balance": "4023.17",
            "balance-date": 1706832000,
            "transactions": [
                {
                    "id": "TXN-FN-001",
                    "posted": 1706832000,
                    "amount": "-89.47",
                    "description": "WHOLE FOODS MARKET #10234",
                    "transacted_at": 1706745600,
                    "pending": false,
                    "extra": {"category": "groceries", "check_number": null}
                },
                {
                    "id": "TXN-FN-002",
                    "posted": 1706745600,
                    "amount": "3250.00",
                    "description": "ACME CORP PAYROLL",
                    "transacted_at": 1706745600,
                    "pending": false
                },
                {
                    "id": "TXN-FN-003",
                    "posted": 1706832000,
                    "amount": "-12.99",
                    "description": "NETFLIX.COM",
                    "pending": true
                },
                {
                    "id": "TXN-FN-004",
                    "posted": 1706832000,
                    "amount": "-2500.00",
                    "description": "RENT PAYMENT - UNIT 4B",
                    "transacted_at": 1706745600,
                    "pending": 0
                }
            ],
            "extra": {"account_type": "checking", "routing_number": "021000021"}
        },
        {
            "org": {
                "sfin-url": "https://sfin.firstnational.example",
                "domain": "firstnational.example",
                "name": "First National Bank",
                "url": "https://www.firstnational.example",
                "id": "org-fn-001"
            },
            "id": "ACT-SAV-5432",
            "name": "High-Yield Savings",
            "currency": "USD",
            "balance": "25000.00",
            "available-balance": "25000.00",
            "balance-date": 1706832000,
            "transactions": [
                {
                    "id": "TXN-FN-005",
                    "posted": 1706745600,
                    "amount": "12.34",
                    "description": "INTEREST PAYMENT",
                    "pending": false
                }
            ]
        },
        {
            "org": {
                "sfin-url": "https://sfin.globalcu.example",
                "domain": "globalcu.example",
                "name": "Global Credit Union",
                "url": "https://www.globalcu.example",
                "id": "org-gcu-002"
            },
            "id": "ACT-CC-1111",
            "name": "Visa Platinum",
            "currency": "USD",
            "balance": "-1847.23",
            "balance-date": 1706832000,
            "transactions": [
                {
                    "id": "TXN-GCU-001",
                    "posted": 1706832000,
                    "amount": "-347.89",
                    "description": "AMAZON.COM*MK4TY3",
                    "transacted_at": 1706659200,
                    "pending": false,
                    "extra": {"merchant_category_code": "5942"}
                },
                {
                    "id": "TXN-GCU-002",
                    "posted": 1706745600,
                    "amount": "500.00",
                    "description": "PAYMENT RECEIVED - THANK YOU",
                    "pending": false
                },
                {
                    "id": "TXN-GCU-003",
                    "posted": 1706832000,
                    "amount": "-65.00",
                    "description": "SHELL OIL 57442",
                    "pending": 1
                }
            ]
        },
        {
            "org": {
                "sfin-url": "https://sfin.globalcu.example",
                "domain": "globalcu.example",
                "name": "Global Credit Union",
                "url": "https://www.globalcu.example",
                "id": "org-gcu-002"
            },
            "id": "ACT-AUTO-2222",
            "name": "Auto Loan",
            "currency": "USD",
            "balance": "-18750.00",
            "available-balance": null,
            "balance-date": 1706832000,
            "transactions": []
        }
    ]
}"#;

#[test]
fn full_api_response_parses_all_accounts() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    assert_eq!(account_set.accounts.len(), 4);
}

#[test]
fn full_api_response_parses_server_messages() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    assert_eq!(account_set.server_messages.len(), 2);
    assert_eq!(account_set.server_messages[0], "Connection to First National is degraded");
    assert!(account_set.server_messages[1].contains("maintenance"));
}

#[test]
fn full_api_response_first_account_all_fields() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let acct = &account_set.accounts[0];
    assert_eq!(acct.id, "ACT-CHK-9876");
    assert_eq!(acct.name, "Premier Checking");
    assert_eq!(acct.currency, "USD");
    assert_eq!(acct.balance, Decimal::from_str("4523.17").unwrap());
    assert_eq!(acct.available_balance, Some(Decimal::from_str("4023.17").unwrap()));
    assert_eq!(acct.balance_date, 1706832000);
    assert_eq!(acct.transactions.len(), 4);
    assert!(acct.extra.is_some());
    assert_eq!(acct.extra.as_ref().unwrap()["account_type"], "checking");
    assert_eq!(acct.extra.as_ref().unwrap()["routing_number"], "021000021");

    // Organization
    assert_eq!(acct.org.sfin_url, "https://sfin.firstnational.example");
    assert_eq!(acct.org.domain.as_deref(), Some("firstnational.example"));
    assert_eq!(acct.org.name.as_deref(), Some("First National Bank"));
    assert_eq!(acct.org.url.as_deref(), Some("https://www.firstnational.example"));
    assert_eq!(acct.org.id.as_deref(), Some("org-fn-001"));
}

#[test]
fn full_api_response_first_account_transactions() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let txns = &account_set.accounts[0].transactions;

    // TXN-FN-001: grocery, not pending, has transacted_at, has extra
    let tx0 = &txns[0];
    assert_eq!(tx0.id, "TXN-FN-001");
    assert_eq!(tx0.posted, 1706832000);
    assert_eq!(tx0.amount, Decimal::from_str("-89.47").unwrap());
    assert_eq!(tx0.description, "WHOLE FOODS MARKET #10234");
    assert_eq!(tx0.transacted_at, Some(1706745600));
    assert!(!tx0.pending);
    assert!(tx0.extra.is_some());
    assert_eq!(tx0.extra.as_ref().unwrap()["category"], "groceries");

    // TXN-FN-002: payroll, positive amount
    let tx1 = &txns[1];
    assert_eq!(tx1.id, "TXN-FN-002");
    assert_eq!(tx1.amount, Decimal::from_str("3250.00").unwrap());
    assert_eq!(tx1.description, "ACME CORP PAYROLL");
    assert!(!tx1.pending);

    // TXN-FN-003: pending=true (boolean)
    let tx2 = &txns[2];
    assert_eq!(tx2.id, "TXN-FN-003");
    assert_eq!(tx2.amount, Decimal::from_str("-12.99").unwrap());
    assert!(tx2.pending);
    assert!(tx2.transacted_at.is_none());

    // TXN-FN-004: pending=0 (number, should be false)
    let tx3 = &txns[3];
    assert_eq!(tx3.id, "TXN-FN-004");
    assert_eq!(tx3.amount, Decimal::from_str("-2500.00").unwrap());
    assert!(!tx3.pending);
    assert_eq!(tx3.transacted_at, Some(1706745600));
}

#[test]
fn full_api_response_savings_account() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let acct = &account_set.accounts[1];
    assert_eq!(acct.id, "ACT-SAV-5432");
    assert_eq!(acct.name, "High-Yield Savings");
    assert_eq!(acct.balance, Decimal::from_str("25000.00").unwrap());
    assert_eq!(acct.available_balance, Some(Decimal::from_str("25000.00").unwrap()));
    assert_eq!(acct.transactions.len(), 1);
    assert_eq!(acct.transactions[0].amount, Decimal::from_str("12.34").unwrap());
    assert!(acct.extra.is_none());
}

#[test]
fn full_api_response_credit_card_negative_balance() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let acct = &account_set.accounts[2];
    assert_eq!(acct.id, "ACT-CC-1111");
    assert_eq!(acct.name, "Visa Platinum");
    assert_eq!(acct.balance, Decimal::from_str("-1847.23").unwrap());
    assert!(acct.available_balance.is_none()); // not provided
    assert_eq!(acct.transactions.len(), 3);
    assert_eq!(acct.org.id.as_deref(), Some("org-gcu-002"));

    // TXN-GCU-003: pending=1 (number, should be true)
    let tx_pending = &acct.transactions[2];
    assert_eq!(tx_pending.id, "TXN-GCU-003");
    assert!(tx_pending.pending);
}

#[test]
fn full_api_response_auto_loan_null_available_balance() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let acct = &account_set.accounts[3];
    assert_eq!(acct.id, "ACT-AUTO-2222");
    assert_eq!(acct.name, "Auto Loan");
    assert_eq!(acct.balance, Decimal::from_str("-18750.00").unwrap());
    assert!(acct.available_balance.is_none()); // explicitly null
    assert!(acct.transactions.is_empty());
}

#[test]
fn full_api_response_filter_by_org_first_national() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-fn-001");
    assert_eq!(filtered.accounts.len(), 2);
    assert_eq!(filtered.accounts[0].id, "ACT-CHK-9876");
    assert_eq!(filtered.accounts[1].id, "ACT-SAV-5432");
    // Server messages preserved
    assert_eq!(filtered.server_messages.len(), 2);
}

#[test]
fn full_api_response_filter_by_org_global_cu() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-gcu-002");
    assert_eq!(filtered.accounts.len(), 2);
    assert_eq!(filtered.accounts[0].id, "ACT-CC-1111");
    assert_eq!(filtered.accounts[1].id, "ACT-AUTO-2222");
}

#[test]
fn full_api_response_filter_by_nonexistent_org() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-does-not-exist");
    assert!(filtered.accounts.is_empty());
    assert_eq!(filtered.server_messages.len(), 2); // messages still preserved
}

#[test]
fn account_set_no_errors_no_accounts() {
    let json = r#"{"errors": [], "accounts": []}"#;
    let account_set: AccountSet = serde_json::from_str(json).unwrap();
    assert!(account_set.server_messages.is_empty());
    assert!(account_set.accounts.is_empty());
}

#[test]
fn account_set_roundtrip_full_response() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let reserialized = serde_json::to_string_pretty(&account_set).unwrap();
    let account_set2: AccountSet = serde_json::from_str(&reserialized).unwrap();
    assert_eq!(account_set.accounts.len(), account_set2.accounts.len());
    assert_eq!(account_set.server_messages, account_set2.server_messages);
    for (a, b) in account_set.accounts.iter().zip(account_set2.accounts.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.name, b.name);
        assert_eq!(a.currency, b.currency);
        assert_eq!(a.balance, b.balance);
        assert_eq!(a.available_balance, b.available_balance);
        assert_eq!(a.balance_date, b.balance_date);
        assert_eq!(a.transactions.len(), b.transactions.len());
        assert_eq!(a.org.sfin_url, b.org.sfin_url);
        assert_eq!(a.org.id, b.org.id);
        for (tx_a, tx_b) in a.transactions.iter().zip(b.transactions.iter()) {
            assert_eq!(tx_a.id, tx_b.id);
            assert_eq!(tx_a.posted, tx_b.posted);
            assert_eq!(tx_a.amount, tx_b.amount);
            assert_eq!(tx_a.description, tx_b.description);
            assert_eq!(tx_a.transacted_at, tx_b.transacted_at);
            assert_eq!(tx_a.pending, tx_b.pending);
        }
    }
}

// ── Deserialization error cases ─────────────────────────────────────────────

#[test]
fn transaction_rejects_invalid_amount_type() {
    let json = r#"{
        "id": "tx-err",
        "posted": 1706745600,
        "amount": true,
        "description": "BAD"
    }"#;
    let result = serde_json::from_str::<Transaction>(json);
    assert!(result.is_err());
}

#[test]
fn transaction_rejects_invalid_amount_string() {
    let json = r#"{
        "id": "tx-err",
        "posted": 1706745600,
        "amount": "not-a-number",
        "description": "BAD"
    }"#;
    let result = serde_json::from_str::<Transaction>(json);
    assert!(result.is_err());
}

#[test]
fn account_rejects_missing_required_fields() {
    // Missing "name"
    let json = r#"{
        "org": {"sfin-url": "https://sfin.example.com"},
        "id": "acct-err",
        "currency": "USD",
        "balance": "100",
        "balance-date": 1706745600
    }"#;
    let result = serde_json::from_str::<Account>(json);
    assert!(result.is_err());
}

#[test]
fn organization_rejects_missing_sfin_url() {
    let json = r#"{"name": "No SFIN URL"}"#;
    let result = serde_json::from_str::<Organization>(json);
    assert!(result.is_err());
}

// ── Multi-currency ──────────────────────────────────────────────────────────

#[test]
fn accounts_with_different_currencies() {
    let json = r#"{
        "errors": [],
        "accounts": [
            {
                "org": {"sfin-url": "https://sfin.example.com", "id": "org-1"},
                "id": "acct-usd",
                "name": "USD Account",
                "currency": "USD",
                "balance": "1000.00",
                "balance-date": 1706745600
            },
            {
                "org": {"sfin-url": "https://sfin.example.com", "id": "org-1"},
                "id": "acct-eur",
                "name": "EUR Account",
                "currency": "EUR",
                "balance": "2000.00",
                "balance-date": 1706745600
            },
            {
                "org": {"sfin-url": "https://sfin.example.com", "id": "org-1"},
                "id": "acct-jpy",
                "name": "JPY Account",
                "currency": "JPY",
                "balance": "150000",
                "balance-date": 1706745600
            }
        ]
    }"#;
    let account_set: AccountSet = serde_json::from_str(json).unwrap();
    assert_eq!(account_set.accounts.len(), 3);
    assert_eq!(account_set.accounts[0].currency, "USD");
    assert_eq!(account_set.accounts[1].currency, "EUR");
    assert_eq!(account_set.accounts[2].currency, "JPY");
    assert_eq!(account_set.accounts[2].balance, Decimal::from_str("150000").unwrap());
}
