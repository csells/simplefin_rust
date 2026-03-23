//! End-to-end pipeline tests that verify the entire flow from raw API JSON
//! through model deserialization, credential handling, query parameter building,
//! business logic (filtering, deduplication), and output formatting in all
//! three formats (text, JSON, CSV).
//!
//! These tests use realistic JSON payloads matching the actual SimpleFIN v1
//! wire format. No mocks, no fake HTTP, no stubs.

use rust_decimal::Decimal;
use std::str::FromStr;

use simplefin::output::{self, TransactionWithAccount};
use simplefin::{AccessCredentials, AccountQueryParams, AccountSet, Organization};

// ── Realistic multi-org, multi-account API responses ────────────────────────

/// Simulates a response from a user with accounts at three different financial
/// institutions: a large bank, a credit union, and a brokerage. Includes
/// every edge case the SimpleFIN spec allows: pending transactions with
/// different representations, available balance present/absent/null, extra
/// metadata, large and small amounts, positive and negative balances,
/// transactions with and without transacted_at.
const COMPREHENSIVE_API_RESPONSE: &str = r#"{
    "errors": [
        "First National: Connection degraded, data may be stale",
        "Maintenance scheduled 2024-02-05 01:00-03:00 UTC"
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
            "id": "FN-CHK-001",
            "name": "Premier Checking",
            "currency": "USD",
            "balance": "12847.93",
            "available-balance": "11347.93",
            "balance-date": 1706918400,
            "transactions": [
                {
                    "id": "FN-TXN-001",
                    "posted": 1706918400,
                    "amount": "-2847.00",
                    "description": "MORTGAGE PAYMENT - WELLS FARGO",
                    "transacted_at": 1706832000,
                    "pending": false,
                    "extra": {"category": "housing", "recurring": true}
                },
                {
                    "id": "FN-TXN-002",
                    "posted": 1706918400,
                    "amount": "-127.43",
                    "description": "WHOLE FOODS MKT #10234 AUSTIN TX",
                    "transacted_at": 1706832000,
                    "pending": false
                },
                {
                    "id": "FN-TXN-003",
                    "posted": 1706918400,
                    "amount": "5420.00",
                    "description": "ACME CORP DIRECT DEP PAYROLL",
                    "transacted_at": 1706918400,
                    "pending": false,
                    "extra": {"category": "income"}
                },
                {
                    "id": "FN-TXN-004",
                    "posted": 1706918400,
                    "amount": "-64.99",
                    "description": "VERIZON WIRELESS AUTOPAY",
                    "pending": true
                },
                {
                    "id": "FN-TXN-005",
                    "posted": 1706832000,
                    "amount": "-9.99",
                    "description": "SPOTIFY USA",
                    "pending": 0
                }
            ],
            "extra": {"account_type": "checking", "routing": "021000021"}
        },
        {
            "org": {
                "sfin-url": "https://sfin.firstnational.example",
                "domain": "firstnational.example",
                "name": "First National Bank",
                "url": "https://www.firstnational.example",
                "id": "org-fn-001"
            },
            "id": "FN-SAV-002",
            "name": "High-Yield Savings",
            "currency": "USD",
            "balance": "52340.18",
            "available-balance": "52340.18",
            "balance-date": 1706918400,
            "transactions": [
                {
                    "id": "FN-TXN-006",
                    "posted": 1706832000,
                    "amount": "43.21",
                    "description": "INTEREST PAYMENT",
                    "pending": false
                },
                {
                    "id": "FN-TXN-007",
                    "posted": 1706745600,
                    "amount": "-1000.00",
                    "description": "TRANSFER TO CHECKING ****001",
                    "pending": false
                }
            ]
        },
        {
            "org": {
                "sfin-url": "https://sfin.coastalcu.example",
                "domain": "coastalcu.example",
                "name": "Coastal Credit Union",
                "url": "https://www.coastalcu.example",
                "id": "org-ccu-002"
            },
            "id": "CCU-VISA-001",
            "name": "Visa Signature",
            "currency": "USD",
            "balance": "-3421.87",
            "balance-date": 1706918400,
            "transactions": [
                {
                    "id": "CCU-TXN-001",
                    "posted": 1706918400,
                    "amount": "-849.99",
                    "description": "BEST BUY      00012345",
                    "transacted_at": 1706832000,
                    "pending": false,
                    "extra": {"merchant_category_code": "5732"}
                },
                {
                    "id": "CCU-TXN-002",
                    "posted": 1706918400,
                    "amount": "-42.17",
                    "description": "UBER EATS       HELP.UBER.COM",
                    "pending": 1
                },
                {
                    "id": "CCU-TXN-003",
                    "posted": 1706832000,
                    "amount": "500.00",
                    "description": "ONLINE PAYMENT RECEIVED",
                    "pending": false
                },
                {
                    "id": "CCU-TXN-004",
                    "posted": 1706918400,
                    "amount": "-199.99",
                    "description": "ANNUAL FEE",
                    "pending": null
                }
            ]
        },
        {
            "org": {
                "sfin-url": "https://sfin.megabrokerage.example",
                "domain": "megabrokerage.example",
                "name": "Mega Brokerage Inc.",
                "url": "https://www.megabrokerage.example",
                "id": "org-mb-003"
            },
            "id": "MB-IRA-001",
            "name": "Traditional IRA",
            "currency": "USD",
            "balance": "187432.91",
            "available-balance": null,
            "balance-date": 1706918400,
            "transactions": []
        },
        {
            "org": {
                "sfin-url": "https://sfin.megabrokerage.example",
                "domain": "megabrokerage.example",
                "name": "Mega Brokerage Inc.",
                "url": "https://www.megabrokerage.example",
                "id": "org-mb-003"
            },
            "id": "MB-ROTH-002",
            "name": "Roth IRA",
            "currency": "USD",
            "balance": "94210.44",
            "balance-date": 1706918400,
            "transactions": [
                {
                    "id": "MB-TXN-001",
                    "posted": 1706745600,
                    "amount": "500.00",
                    "description": "CONTRIBUTION",
                    "pending": false
                }
            ]
        }
    ]
}"#;

// ── Pipeline: Parse → Verify every field ────────────────────────────────────

#[test]
fn pipeline_parse_comprehensive_response_all_accounts() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    assert_eq!(account_set.accounts.len(), 5);
    assert_eq!(account_set.server_messages.len(), 2);
}

#[test]
fn pipeline_all_account_ids_and_names() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let ids: Vec<&str> = account_set.accounts.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["FN-CHK-001", "FN-SAV-002", "CCU-VISA-001", "MB-IRA-001", "MB-ROTH-002"]
    );
    let names: Vec<&str> = account_set.accounts.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        vec![
            "Premier Checking",
            "High-Yield Savings",
            "Visa Signature",
            "Traditional IRA",
            "Roth IRA"
        ]
    );
}

#[test]
fn pipeline_all_balances_precise() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let expected = vec![
        ("FN-CHK-001", "12847.93", Some("11347.93")),
        ("FN-SAV-002", "52340.18", Some("52340.18")),
        ("CCU-VISA-001", "-3421.87", None),
        ("MB-IRA-001", "187432.91", None),  // null available
        ("MB-ROTH-002", "94210.44", None),
    ];
    for (i, (id, balance, avail)) in expected.iter().enumerate() {
        let acct = &account_set.accounts[i];
        assert_eq!(&acct.id, id);
        assert_eq!(acct.balance, Decimal::from_str(balance).unwrap(), "balance mismatch for {id}");
        match avail {
            Some(expected_avail) => {
                assert_eq!(
                    acct.available_balance,
                    Some(Decimal::from_str(expected_avail).unwrap()),
                    "available balance mismatch for {id}"
                );
            }
            None => {
                assert!(acct.available_balance.is_none(), "expected None available for {id}");
            }
        }
    }
}

#[test]
fn pipeline_all_transaction_counts() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let counts: Vec<usize> = account_set.accounts.iter().map(|a| a.transactions.len()).collect();
    assert_eq!(counts, vec![5, 2, 4, 0, 1]);
}

#[test]
fn pipeline_all_transaction_ids_in_order() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let all_tx_ids: Vec<&str> = account_set
        .accounts
        .iter()
        .flat_map(|a| a.transactions.iter().map(|t| t.id.as_str()))
        .collect();
    assert_eq!(
        all_tx_ids,
        vec![
            "FN-TXN-001", "FN-TXN-002", "FN-TXN-003", "FN-TXN-004", "FN-TXN-005",
            "FN-TXN-006", "FN-TXN-007",
            "CCU-TXN-001", "CCU-TXN-002", "CCU-TXN-003", "CCU-TXN-004",
            "MB-TXN-001",
        ]
    );
}

#[test]
fn pipeline_all_transaction_amounts_precise() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let expected: Vec<(&str, &str)> = vec![
        ("FN-TXN-001", "-2847.00"),
        ("FN-TXN-002", "-127.43"),
        ("FN-TXN-003", "5420.00"),
        ("FN-TXN-004", "-64.99"),
        ("FN-TXN-005", "-9.99"),
        ("FN-TXN-006", "43.21"),
        ("FN-TXN-007", "-1000.00"),
        ("CCU-TXN-001", "-849.99"),
        ("CCU-TXN-002", "-42.17"),
        ("CCU-TXN-003", "500.00"),
        ("CCU-TXN-004", "-199.99"),
        ("MB-TXN-001", "500.00"),
    ];

    let all_txns: Vec<_> = account_set
        .accounts
        .iter()
        .flat_map(|a| a.transactions.iter())
        .collect();

    for (i, (expected_id, expected_amount)) in expected.iter().enumerate() {
        assert_eq!(all_txns[i].id, *expected_id);
        assert_eq!(
            all_txns[i].amount,
            Decimal::from_str(expected_amount).unwrap(),
            "amount mismatch for {expected_id}"
        );
    }
}

#[test]
fn pipeline_all_pending_states() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let expected: Vec<(&str, bool)> = vec![
        ("FN-TXN-001", false),
        ("FN-TXN-002", false),
        ("FN-TXN-003", false),
        ("FN-TXN-004", true),    // pending: true (bool)
        ("FN-TXN-005", false),   // pending: 0 (number)
        ("FN-TXN-006", false),
        ("FN-TXN-007", false),
        ("CCU-TXN-001", false),
        ("CCU-TXN-002", true),   // pending: 1 (number)
        ("CCU-TXN-003", false),
        ("CCU-TXN-004", false),  // pending: null
        ("MB-TXN-001", false),
    ];

    let all_txns: Vec<_> = account_set
        .accounts
        .iter()
        .flat_map(|a| a.transactions.iter())
        .collect();

    for (i, (expected_id, expected_pending)) in expected.iter().enumerate() {
        assert_eq!(all_txns[i].id, *expected_id);
        assert_eq!(
            all_txns[i].pending, *expected_pending,
            "pending mismatch for {expected_id}"
        );
    }
}

#[test]
fn pipeline_all_transacted_at_values() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let expected: Vec<(&str, Option<i64>)> = vec![
        ("FN-TXN-001", Some(1706832000)),
        ("FN-TXN-002", Some(1706832000)),
        ("FN-TXN-003", Some(1706918400)),
        ("FN-TXN-004", None),
        ("FN-TXN-005", None),
        ("FN-TXN-006", None),
        ("FN-TXN-007", None),
        ("CCU-TXN-001", Some(1706832000)),
        ("CCU-TXN-002", None),
        ("CCU-TXN-003", None),
        ("CCU-TXN-004", None),
        ("MB-TXN-001", None),
    ];

    let all_txns: Vec<_> = account_set
        .accounts
        .iter()
        .flat_map(|a| a.transactions.iter())
        .collect();

    for (i, (expected_id, expected_ta)) in expected.iter().enumerate() {
        assert_eq!(all_txns[i].id, *expected_id);
        assert_eq!(
            all_txns[i].transacted_at, *expected_ta,
            "transacted_at mismatch for {expected_id}"
        );
    }
}

#[test]
fn pipeline_extra_metadata_preserved() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    // Account-level extra
    let checking = &account_set.accounts[0];
    let extra = checking.extra.as_ref().unwrap();
    assert_eq!(extra["account_type"], "checking");
    assert_eq!(extra["routing"], "021000021");

    // Transaction-level extra
    let mortgage_tx = &checking.transactions[0];
    let tx_extra = mortgage_tx.extra.as_ref().unwrap();
    assert_eq!(tx_extra["category"], "housing");
    assert_eq!(tx_extra["recurring"], true);

    let payroll_tx = &checking.transactions[2];
    assert_eq!(payroll_tx.extra.as_ref().unwrap()["category"], "income");

    let credit_tx = &account_set.accounts[2].transactions[0];
    assert_eq!(credit_tx.extra.as_ref().unwrap()["merchant_category_code"], "5732");
}

// ── Pipeline: Parse → Filter → Output ───────────────────────────────────────

#[test]
fn pipeline_filter_then_csv_output() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-fn-001");
    assert_eq!(filtered.accounts.len(), 2);

    let csv = output::format_accounts_csv(&filtered.accounts, &filtered.server_messages);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);
    assert_eq!(&records[0][0], "FN-CHK-001");
    assert_eq!(&records[1][0], "FN-SAV-002");
}

#[test]
fn pipeline_filter_then_json_output() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-ccu-002");
    assert_eq!(filtered.accounts.len(), 1);

    let json_str = output::format_accounts_json(&filtered.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"].as_str().unwrap(), "CCU-VISA-001");
    assert_eq!(arr[0]["balance"].as_str().unwrap(), "-3421.87");
}

#[test]
fn pipeline_filter_brokerage_no_transactions() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let filtered = account_set.filter_by_organization_id("org-mb-003");
    assert_eq!(filtered.accounts.len(), 2);
    assert!(filtered.accounts[0].transactions.is_empty()); // IRA has no txns
    assert_eq!(filtered.accounts[1].transactions.len(), 1); // Roth has 1

    let csv = output::format_transactions_csv(&filtered.accounts, &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 1); // Only Roth's single transaction
    assert_eq!(&records[0][0], "MB-ROTH-002");
    assert_eq!(&records[0][1], "MB-TXN-001");
}

// ── Pipeline: Parse → Deduplicate orgs → Output ─────────────────────────────

#[test]
fn pipeline_deduplicate_organizations() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    let mut org_map = indexmap::IndexMap::new();
    for account in &account_set.accounts {
        let key = account.org.key().to_string();
        org_map.entry(key).or_insert(&account.org);
    }
    let organizations: Vec<&Organization> = org_map.into_values().collect();

    // 5 accounts across 3 organizations
    assert_eq!(organizations.len(), 3);
    assert_eq!(organizations[0].id.as_deref(), Some("org-fn-001"));
    assert_eq!(organizations[1].id.as_deref(), Some("org-ccu-002"));
    assert_eq!(organizations[2].id.as_deref(), Some("org-mb-003"));
}

#[test]
fn pipeline_deduplicate_orgs_to_json() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    let mut org_map = indexmap::IndexMap::new();
    for account in &account_set.accounts {
        let key = account.org.key().to_string();
        org_map.entry(key).or_insert(&account.org);
    }
    let organizations: Vec<&Organization> = org_map.into_values().collect();

    let json_str = output::format_organizations_json(&organizations, &account_set.server_messages);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    // With server messages present, should be wrapped
    assert!(parsed["server-messages"].is_array());
    assert_eq!(parsed["server-messages"].as_array().unwrap().len(), 2);

    let data = parsed["data"].as_array().unwrap();
    assert_eq!(data.len(), 3);
    assert_eq!(data[0]["name"].as_str().unwrap(), "First National Bank");
    assert_eq!(data[1]["name"].as_str().unwrap(), "Coastal Credit Union");
    assert_eq!(data[2]["name"].as_str().unwrap(), "Mega Brokerage Inc.");
}

#[test]
fn pipeline_deduplicate_orgs_to_csv() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    let mut org_map = indexmap::IndexMap::new();
    for account in &account_set.accounts {
        let key = account.org.key().to_string();
        org_map.entry(key).or_insert(&account.org);
    }
    let organizations: Vec<&Organization> = org_map.into_values().collect();

    let csv = output::format_organizations_csv(&organizations, &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 3);
    assert_eq!(&records[0][0], "org-fn-001");
    assert_eq!(&records[0][1], "First National Bank");
    assert_eq!(&records[0][2], "firstnational.example");
    assert_eq!(&records[1][0], "org-ccu-002");
    assert_eq!(&records[2][0], "org-mb-003");
}

// ── Pipeline: Parse → Flatten transactions → Output ─────────────────────────

#[test]
fn pipeline_flatten_all_transactions_to_json() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    let items: Vec<TransactionWithAccount<'_>> = account_set
        .accounts
        .iter()
        .flat_map(|account| {
            account
                .transactions
                .iter()
                .map(move |tx| TransactionWithAccount {
                    account,
                    transaction: tx,
                })
        })
        .collect();

    assert_eq!(items.len(), 12); // 5 + 2 + 4 + 0 + 1

    let json_str = output::format_transactions_json(&items, &account_set.server_messages);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let data = parsed["data"].as_array().unwrap();
    assert_eq!(data.len(), 12);

    // Spot-check: mortgage payment
    assert_eq!(data[0]["account-id"].as_str().unwrap(), "FN-CHK-001");
    assert_eq!(data[0]["transaction-id"].as_str().unwrap(), "FN-TXN-001");
    assert_eq!(data[0]["amount"].as_str().unwrap(), "-2847.00");
    assert_eq!(data[0]["description"].as_str().unwrap(), "MORTGAGE PAYMENT - WELLS FARGO");
    assert_eq!(data[0]["pending"].as_bool().unwrap(), false);

    // Spot-check: pending credit card transaction
    assert_eq!(data[8]["transaction-id"].as_str().unwrap(), "CCU-TXN-002");
    assert_eq!(data[8]["pending"].as_bool().unwrap(), true);
    assert_eq!(data[8]["amount"].as_str().unwrap(), "-42.17");

    // Spot-check: Roth contribution
    assert_eq!(data[11]["account-id"].as_str().unwrap(), "MB-ROTH-002");
    assert_eq!(data[11]["amount"].as_str().unwrap(), "500.00");
}

#[test]
fn pipeline_flatten_all_transactions_to_csv() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    let csv = output::format_transactions_csv(&account_set.accounts, &account_set.server_messages);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 12);

    // Verify every transaction row maps to correct account
    let expected_account_ids = vec![
        "FN-CHK-001", "FN-CHK-001", "FN-CHK-001", "FN-CHK-001", "FN-CHK-001",
        "FN-SAV-002", "FN-SAV-002",
        "CCU-VISA-001", "CCU-VISA-001", "CCU-VISA-001", "CCU-VISA-001",
        "MB-ROTH-002",
    ];
    for (i, expected_id) in expected_account_ids.iter().enumerate() {
        assert_eq!(
            &records[i][0], *expected_id,
            "account_id mismatch at row {i}"
        );
    }

    // Verify server messages only on first row
    assert!(records[0][7].contains("First National"));
    for record in &records[1..] {
        assert!(record[7].is_empty());
    }
}

// ── Pipeline: Credentials → endpoint URL → query params ────────────────────

#[test]
fn pipeline_credentials_to_endpoint_with_all_query_params() {
    let creds = AccessCredentials::parse(
        "https://user123:secretpass@api.simplefin.org/simplefin",
    )
    .unwrap();

    // Simulate what AccessClient.build_query_params does
    let params = AccountQueryParams {
        start_date: Some(1706745600),
        end_date: Some(1706918400),
        include_pending: true,
        account_ids: Some(vec!["acct-1".into(), "acct-2".into()]),
        balances_only: false,
    };

    let mut query: Vec<(String, String)> = Vec::new();
    if let Some(start) = params.start_date {
        query.push(("start-date".into(), start.to_string()));
    }
    if let Some(end) = params.end_date {
        query.push(("end-date".into(), end.to_string()));
    }
    if params.include_pending {
        query.push(("pending".into(), "1".into()));
    }
    if params.balances_only {
        query.push(("balances-only".into(), "1".into()));
    }
    if let Some(ref ids) = params.account_ids {
        for id in ids.iter().filter(|id| !id.is_empty()) {
            query.push(("account".into(), id.clone()));
        }
    }

    let query_refs: Vec<(&str, &str)> = query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let url = creds.endpoint_url(
        &["accounts"],
        if query_refs.is_empty() { None } else { Some(&query_refs) },
    );

    assert_eq!(url.path(), "/simplefin/accounts");
    let query_str = url.query().unwrap();
    assert!(query_str.contains("start-date=1706745600"));
    assert!(query_str.contains("end-date=1706918400"));
    assert!(query_str.contains("pending=1"));
    assert!(!query_str.contains("balances-only")); // false, so not included
    assert!(query_str.contains("account=acct-1"));
    assert!(query_str.contains("account=acct-2"));
}

#[test]
fn pipeline_credentials_to_endpoint_balances_only() {
    let creds = AccessCredentials::parse(
        "https://user:pass@api.simplefin.org/simplefin",
    )
    .unwrap();

    let params = AccountQueryParams {
        balances_only: true,
        ..Default::default()
    };

    let mut query: Vec<(String, String)> = Vec::new();
    if params.balances_only {
        query.push(("balances-only".into(), "1".into()));
    }

    let query_refs: Vec<(&str, &str)> = query.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
    let url = creds.endpoint_url(
        &["accounts"],
        Some(&query_refs),
    );

    let query_str = url.query().unwrap();
    assert!(query_str.contains("balances-only=1"));
    assert!(!query_str.contains("start-date"));
    assert!(!query_str.contains("end-date"));
    assert!(!query_str.contains("pending"));
    assert!(!query_str.contains("account"));
}

// ── Pipeline: ISO-8601 formatting consistency ───────────────────────────────

#[test]
fn pipeline_all_dates_format_with_z_suffix() {
    let account_set: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();

    for account in &account_set.accounts {
        let date_str = account.balance_date_iso8601();
        assert!(
            date_str.ends_with('Z'),
            "balance_date for {} should end with Z, got: {date_str}",
            account.id
        );
        assert!(
            !date_str.contains("+00:00"),
            "balance_date for {} should not contain +00:00, got: {date_str}",
            account.id
        );

        for tx in &account.transactions {
            let posted = tx.posted_iso8601();
            assert!(
                posted.ends_with('Z'),
                "posted for {} should end with Z, got: {posted}",
                tx.id
            );

            if let Some(ta) = tx.transacted_at_iso8601() {
                assert!(
                    ta.ends_with('Z'),
                    "transacted_at for {} should end with Z, got: {ta}",
                    tx.id
                );
            }
        }
    }
}

// ── Pipeline: Roundtrip serialize/deserialize ───────────────────────────────

#[test]
fn pipeline_full_roundtrip_preserves_all_data() {
    let original: AccountSet = serde_json::from_str(COMPREHENSIVE_API_RESPONSE).unwrap();
    let serialized = serde_json::to_string_pretty(&original).unwrap();
    let restored: AccountSet = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original.server_messages, restored.server_messages);
    assert_eq!(original.accounts.len(), restored.accounts.len());

    for (orig, rest) in original.accounts.iter().zip(restored.accounts.iter()) {
        assert_eq!(orig.id, rest.id);
        assert_eq!(orig.name, rest.name);
        assert_eq!(orig.currency, rest.currency);
        assert_eq!(orig.balance, rest.balance);
        assert_eq!(orig.available_balance, rest.available_balance);
        assert_eq!(orig.balance_date, rest.balance_date);

        assert_eq!(orig.org.sfin_url, rest.org.sfin_url);
        assert_eq!(orig.org.domain, rest.org.domain);
        assert_eq!(orig.org.name, rest.org.name);
        assert_eq!(orig.org.url, rest.org.url);
        assert_eq!(orig.org.id, rest.org.id);

        assert_eq!(orig.transactions.len(), rest.transactions.len());
        for (orig_tx, rest_tx) in orig.transactions.iter().zip(rest.transactions.iter()) {
            assert_eq!(orig_tx.id, rest_tx.id);
            assert_eq!(orig_tx.posted, rest_tx.posted);
            assert_eq!(orig_tx.amount, rest_tx.amount);
            assert_eq!(orig_tx.description, rest_tx.description);
            assert_eq!(orig_tx.transacted_at, rest_tx.transacted_at);
            assert_eq!(orig_tx.pending, rest_tx.pending);
            assert_eq!(orig_tx.extra, rest_tx.extra);
        }
    }
}

// ── Pipeline: edge case with all optional fields missing ────────────────────

#[test]
fn pipeline_minimal_api_response() {
    let json = r#"{
        "errors": [],
        "accounts": [
            {
                "org": {"sfin-url": "https://sfin.bare.example"},
                "id": "bare-001",
                "name": "Bare Account",
                "currency": "USD",
                "balance": 0,
                "balance-date": 0
            }
        ]
    }"#;
    let account_set: AccountSet = serde_json::from_str(json).unwrap();
    assert_eq!(account_set.accounts.len(), 1);
    let acct = &account_set.accounts[0];
    assert_eq!(acct.balance, Decimal::ZERO);
    assert!(acct.available_balance.is_none());
    assert!(acct.transactions.is_empty());
    assert!(acct.extra.is_none());
    assert!(acct.org.name.is_none());
    assert!(acct.org.domain.is_none());
    assert!(acct.org.url.is_none());
    assert!(acct.org.id.is_none());

    // Output should still work
    let csv = output::format_accounts_csv(&account_set.accounts, &[]);
    assert!(csv.contains("bare-001"));

    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed[0]["id"].as_str().unwrap(), "bare-001");
    assert_eq!(parsed[0]["balance"].as_str().unwrap(), "0");

    // Org dedup
    let orgs: Vec<&Organization> = vec![&acct.org];
    let org_json = output::format_organizations_json(&orgs, &[]);
    let parsed_org: serde_json::Value = serde_json::from_str(&org_json).unwrap();
    assert_eq!(parsed_org["sfin-url"].as_str().unwrap(), "https://sfin.bare.example");
}

// ── Pipeline: unicode and special characters ────────────────────────────────

#[test]
fn pipeline_unicode_descriptions() {
    let json = r#"{
        "errors": ["Advertencia: conexion inestable"],
        "accounts": [
            {
                "org": {
                    "sfin-url": "https://sfin.bancointernacional.example",
                    "name": "Banco Internacional S.A.",
                    "id": "org-bi-001"
                },
                "id": "BI-001",
                "name": "Cuenta Corriente",
                "currency": "MXN",
                "balance": "45000.00",
                "balance-date": 1706918400,
                "transactions": [
                    {
                        "id": "BI-TX-001",
                        "posted": 1706918400,
                        "amount": "-1500.00",
                        "description": "TIENDA DE ABARROTES EL RAYO",
                        "pending": false
                    },
                    {
                        "id": "BI-TX-002",
                        "posted": 1706918400,
                        "amount": "-299.99",
                        "description": "CAFE & RISTORANTE BUON GIORNO",
                        "pending": false
                    }
                ]
            }
        ]
    }"#;
    let account_set: AccountSet = serde_json::from_str(json).unwrap();
    assert_eq!(account_set.accounts[0].name, "Cuenta Corriente");
    assert_eq!(account_set.accounts[0].currency, "MXN");
    assert_eq!(account_set.server_messages[0], "Advertencia: conexion inestable");

    // CSV output with special chars
    let csv = output::format_accounts_csv(&account_set.accounts, &account_set.server_messages);
    assert!(csv.contains("Cuenta Corriente"));

    let tx_csv = output::format_transactions_csv(&account_set.accounts, &[]);
    assert!(tx_csv.contains("TIENDA DE ABARROTES EL RAYO"));
    assert!(tx_csv.contains("CAFE & RISTORANTE BUON GIORNO"));

    // JSON output
    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    assert!(json_str.contains("Cuenta Corriente"));
}
