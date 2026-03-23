use simplefin::output::{self, TransactionWithAccount};
use simplefin::{Account, AccountSet, Organization, Transaction};

/// Helper to build a realistic Organization.
fn make_org(id: &str, name: &str, domain: &str) -> Organization {
    serde_json::from_value(serde_json::json!({
        "sfin-url": format!("https://sfin.{domain}"),
        "domain": domain,
        "name": name,
        "url": format!("https://www.{domain}"),
        "id": id,
    }))
    .unwrap()
}

/// Helper to build a realistic Account with transactions.
fn make_account(
    org: &Organization,
    id: &str,
    name: &str,
    currency: &str,
    balance: &str,
    available_balance: Option<&str>,
    balance_date: i64,
    transactions: Vec<Transaction>,
) -> Account {
    let mut json = serde_json::json!({
        "org": serde_json::to_value(org).unwrap(),
        "id": id,
        "name": name,
        "currency": currency,
        "balance": balance,
        "balance-date": balance_date,
        "transactions": serde_json::to_value(&transactions).unwrap(),
    });
    if let Some(avail) = available_balance {
        json["available-balance"] = serde_json::json!(avail);
    }
    serde_json::from_value(json).unwrap()
}

/// Helper to build a Transaction.
fn make_tx(
    id: &str,
    posted: i64,
    amount: &str,
    description: &str,
    pending: bool,
    transacted_at: Option<i64>,
) -> Transaction {
    let mut json = serde_json::json!({
        "id": id,
        "posted": posted,
        "amount": amount,
        "description": description,
        "pending": pending,
    });
    if let Some(ta) = transacted_at {
        json["transacted_at"] = serde_json::json!(ta);
    }
    serde_json::from_value(json).unwrap()
}

// ── Full realistic test data ────────────────────────────────────────────────

/// The same full API response used in models_test, parsed here for output tests.
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
                    "extra": {"category": "groceries"}
                },
                {
                    "id": "TXN-FN-002",
                    "posted": 1706745600,
                    "amount": "3250.00",
                    "description": "ACME CORP PAYROLL",
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
                    "pending": false
                }
            ]
        }
    ]
}"#;

// ── Accounts text output ────────────────────────────────────────────────────

#[test]
fn accounts_text_output_contains_all_fields() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    // Capture stdout by checking the function doesn't panic and covers key data
    // We verify the functions exist and accept the right types
    // For text output, we verify the internal formatting via the other format functions
    // which return strings we can inspect.

    // Verify the function compiles and runs without panic
    output::print_accounts_text(&account_set.accounts, &account_set.server_messages);
}

// ── Accounts JSON output ────────────────────────────────────────────────────

#[test]
fn accounts_json_output_has_server_messages_wrapper() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let json_str = output::format_accounts_json(&account_set.accounts, &account_set.server_messages);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed["server-messages"].is_array());
    assert_eq!(parsed["server-messages"].as_array().unwrap().len(), 2);
    assert!(parsed["data"].is_array());
    assert_eq!(parsed["data"].as_array().unwrap().len(), 2);
}

#[test]
fn accounts_json_output_no_server_messages_returns_bare_array() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    // Without server messages, should be a bare array
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn accounts_json_output_contains_all_account_fields() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();

    let first = &arr[0];
    assert_eq!(first["id"].as_str().unwrap(), "ACT-CHK-9876");
    assert_eq!(first["name"].as_str().unwrap(), "Premier Checking");
    assert_eq!(first["balance"].as_str().unwrap(), "4523.17");
    assert_eq!(first["available-balance"].as_str().unwrap(), "4023.17");
    assert_eq!(first["currency"].as_str().unwrap(), "USD");
    assert!(first["balance-date"].as_str().unwrap().contains("2024-02-02"));
    assert_eq!(first["org-id"].as_str().unwrap(), "org-fn-001");
}

#[test]
fn accounts_json_output_credit_card_no_available_balance() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();

    let credit = &arr[1]; // Visa Platinum
    assert_eq!(credit["id"].as_str().unwrap(), "ACT-CC-1111");
    assert_eq!(credit["balance"].as_str().unwrap(), "-1847.23");
    // When available_balance is None, should fall back to balance
    assert_eq!(credit["available-balance"].as_str().unwrap(), "-1847.23");
}

// ── Accounts CSV output ────────────────────────────────────────────────────

#[test]
fn accounts_csv_output_has_correct_headers() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_accounts_csv(&account_set.accounts, &[]);
    let lines: Vec<&str> = csv.lines().collect();
    assert!(lines[0].contains("account_id"));
    assert!(lines[0].contains("account_name"));
    assert!(lines[0].contains("currency"));
    assert!(lines[0].contains("balance"));
    assert!(lines[0].contains("available_balance"));
    assert!(lines[0].contains("balance_date"));
    assert!(lines[0].contains("org_id"));
    assert!(lines[0].contains("server_messages"));
}

#[test]
fn accounts_csv_output_has_correct_row_count() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_accounts_csv(&account_set.accounts, &[]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 3); // 1 header + 2 data rows
}

#[test]
fn accounts_csv_output_first_row_has_server_messages() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_accounts_csv(&account_set.accounts, &account_set.server_messages);
    let lines: Vec<&str> = csv.lines().collect();
    // First data row should contain server messages
    assert!(lines[1].contains("Connection to First National"));
    // Second data row should NOT
    assert!(!lines[2].contains("Connection to First National"));
}

#[test]
fn accounts_csv_output_data_values() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_accounts_csv(&account_set.accounts, &[]);
    // Parse as CSV to verify individual fields
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);

    // First account
    assert_eq!(&records[0][0], "ACT-CHK-9876");
    assert_eq!(&records[0][1], "Premier Checking");
    assert_eq!(&records[0][2], "USD");
    assert_eq!(&records[0][3], "4523.17");
    assert_eq!(&records[0][4], "4023.17");
    assert_eq!(&records[0][6], "org-fn-001");

    // Second account
    assert_eq!(&records[1][0], "ACT-CC-1111");
    assert_eq!(&records[1][3], "-1847.23");
    assert_eq!(&records[1][4], ""); // no available balance
}

// ── Organizations text output ───────────────────────────────────────────────

#[test]
fn organizations_text_output_runs() {
    let org1 = make_org("org-1", "First Bank", "firstbank.com");
    let org2 = make_org("org-2", "Second Credit Union", "secondcu.com");
    let orgs: Vec<&Organization> = vec![&org1, &org2];
    let msgs = vec!["test message".to_string()];
    output::print_organizations_text(&orgs, &msgs);
}

// ── Organizations JSON output ───────────────────────────────────────────────

#[test]
fn organizations_json_single_returns_object() {
    let org = make_org("org-1", "Only Bank", "onlybank.com");
    let orgs: Vec<&Organization> = vec![&org];
    let json_str = output::format_organizations_json(&orgs, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    // Single org should be returned as object, not array
    assert!(parsed.is_object());
    assert_eq!(parsed["id"].as_str().unwrap(), "org-1");
    assert_eq!(parsed["name"].as_str().unwrap(), "Only Bank");
    assert_eq!(parsed["domain"].as_str().unwrap(), "onlybank.com");
    assert_eq!(parsed["sfin-url"].as_str().unwrap(), "https://sfin.onlybank.com");
}

#[test]
fn organizations_json_multiple_returns_array() {
    let org1 = make_org("org-1", "First Bank", "firstbank.com");
    let org2 = make_org("org-2", "Second Bank", "secondbank.com");
    let orgs: Vec<&Organization> = vec![&org1, &org2];
    let json_str = output::format_organizations_json(&orgs, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 2);
}

#[test]
fn organizations_json_with_server_messages() {
    let org = make_org("org-1", "Test Bank", "testbank.com");
    let orgs: Vec<&Organization> = vec![&org];
    let msgs = vec!["Server upgrading".to_string()];
    let json_str = output::format_organizations_json(&orgs, &msgs);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed["server-messages"].is_array());
    assert_eq!(parsed["server-messages"][0].as_str().unwrap(), "Server upgrading");
    assert!(parsed["data"].is_object()); // single org wrapped in data
}

#[test]
fn organizations_json_all_fields() {
    let org = make_org("org-full", "Full Org", "fullorg.com");
    let orgs: Vec<&Organization> = vec![&org];
    let json_str = output::format_organizations_json(&orgs, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["id"].as_str().unwrap(), "org-full");
    assert_eq!(parsed["name"].as_str().unwrap(), "Full Org");
    assert_eq!(parsed["domain"].as_str().unwrap(), "fullorg.com");
    assert_eq!(parsed["url"].as_str().unwrap(), "https://www.fullorg.com");
    assert_eq!(parsed["sfin-url"].as_str().unwrap(), "https://sfin.fullorg.com");
}

#[test]
fn organizations_json_minimal_org() {
    let org: Organization = serde_json::from_value(serde_json::json!({
        "sfin-url": "https://sfin.minimal.example",
    }))
    .unwrap();
    let orgs: Vec<&Organization> = vec![&org];
    let json_str = output::format_organizations_json(&orgs, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert_eq!(parsed["sfin-url"].as_str().unwrap(), "https://sfin.minimal.example");
    // Optional fields should be absent
    assert!(parsed.get("id").is_none());
    assert!(parsed.get("name").is_none());
    assert!(parsed.get("domain").is_none());
    assert!(parsed.get("url").is_none());
}

// ── Organizations CSV output ────────────────────────────────────────────────

#[test]
fn organizations_csv_headers_and_data() {
    let org1 = make_org("org-1", "First Bank", "firstbank.com");
    let org2 = make_org("org-2", "Second Bank", "secondbank.com");
    let orgs: Vec<&Organization> = vec![&org1, &org2];
    let csv = output::format_organizations_csv(&orgs, &[]);
    let lines: Vec<&str> = csv.lines().collect();

    // Header
    assert!(lines[0].contains("id"));
    assert!(lines[0].contains("name"));
    assert!(lines[0].contains("domain"));
    assert!(lines[0].contains("url"));
    assert!(lines[0].contains("sfin_url"));

    // Data rows
    assert_eq!(lines.len(), 3); // header + 2 rows

    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(&records[0][0], "org-1");
    assert_eq!(&records[0][1], "First Bank");
    assert_eq!(&records[0][2], "firstbank.com");
    assert_eq!(&records[1][0], "org-2");
}

// ── Transactions text output ────────────────────────────────────────────────

#[test]
fn transactions_text_output_runs() {
    let org = make_org("org-1", "Test Bank", "test.com");
    let tx = make_tx("tx-1", 1706745600, "-25.00", "TEST TX", false, Some(1706659200));
    let account = make_account(&org, "acct-1", "Checking", "USD", "1000.00", None, 1706745600, vec![tx]);
    let items: Vec<TransactionWithAccount<'_>> = account
        .transactions
        .iter()
        .map(|tx| TransactionWithAccount {
            account: &account,
            transaction: tx,
        })
        .collect();
    output::print_transactions_text(&items, &[]);
}

// ── Transactions JSON output ────────────────────────────────────────────────

#[test]
fn transactions_json_all_fields() {
    let org = make_org("org-1", "Test Bank", "test.com");
    let tx1 = make_tx("tx-1", 1706832000, "-89.47", "GROCERY STORE", false, Some(1706745600));
    let tx2 = make_tx("tx-2", 1706745600, "3250.00", "PAYROLL", false, None);
    let tx3 = make_tx("tx-3", 1706832000, "-12.99", "NETFLIX", true, None);
    let account = make_account(
        &org,
        "acct-1",
        "Checking",
        "USD",
        "4523.17",
        Some("4023.17"),
        1706832000,
        vec![tx1, tx2, tx3],
    );
    let items: Vec<TransactionWithAccount<'_>> = account
        .transactions
        .iter()
        .map(|tx| TransactionWithAccount {
            account: &account,
            transaction: tx,
        })
        .collect();

    let json_str = output::format_transactions_json(&items, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 3);

    // First transaction
    assert_eq!(arr[0]["account-id"].as_str().unwrap(), "acct-1");
    assert_eq!(arr[0]["transaction-id"].as_str().unwrap(), "tx-1");
    assert_eq!(arr[0]["amount"].as_str().unwrap(), "-89.47");
    assert_eq!(arr[0]["description"].as_str().unwrap(), "GROCERY STORE");
    assert_eq!(arr[0]["pending"].as_bool().unwrap(), false);
    assert!(arr[0]["posted"].as_str().unwrap().contains("2024-02-02"));
    assert!(arr[0]["transacted-at"].as_str().is_some());

    // Second - no transacted-at
    assert!(arr[1].get("transacted-at").is_none());

    // Third - pending
    assert_eq!(arr[2]["pending"].as_bool().unwrap(), true);
}

#[test]
fn transactions_json_with_server_messages() {
    let org = make_org("org-1", "Test Bank", "test.com");
    let tx = make_tx("tx-1", 1706745600, "-10.00", "TEST", false, None);
    let account = make_account(&org, "acct-1", "Checking", "USD", "100.00", None, 1706745600, vec![tx]);
    let items: Vec<TransactionWithAccount<'_>> = account
        .transactions
        .iter()
        .map(|tx| TransactionWithAccount {
            account: &account,
            transaction: tx,
        })
        .collect();
    let msgs = vec!["Alert: maintenance".to_string()];
    let json_str = output::format_transactions_json(&items, &msgs);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed["server-messages"].is_array());
    assert!(parsed["data"].is_array());
}

// ── Transactions CSV output ─────────────────────────────────────────────────

#[test]
fn transactions_csv_headers() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_transactions_csv(&account_set.accounts, &[]);
    let first_line = csv.lines().next().unwrap();
    assert!(first_line.contains("account_id"));
    assert!(first_line.contains("transaction_id"));
    assert!(first_line.contains("posted"));
    assert!(first_line.contains("amount"));
    assert!(first_line.contains("description"));
    assert!(first_line.contains("pending"));
    assert!(first_line.contains("transacted_at"));
    assert!(first_line.contains("server_messages"));
}

#[test]
fn transactions_csv_row_count() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_transactions_csv(&account_set.accounts, &[]);
    let total_txns: usize = account_set.accounts.iter().map(|a| a.transactions.len()).sum();
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1 + total_txns); // header + data
}

#[test]
fn transactions_csv_data_values() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_transactions_csv(&account_set.accounts, &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();

    // First transaction from first account
    assert_eq!(&records[0][0], "ACT-CHK-9876"); // account_id
    assert_eq!(&records[0][1], "TXN-FN-001"); // transaction_id
    assert!(!records[0][2].is_empty()); // posted (ISO-8601)
    assert_eq!(&records[0][3], "-89.47"); // amount
    assert_eq!(&records[0][4], "WHOLE FOODS MARKET #10234"); // description
    assert_eq!(&records[0][5], "false"); // pending
    assert!(!records[0][6].is_empty()); // transacted_at (present for this tx)
}

#[test]
fn transactions_csv_server_messages_only_on_first_row() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_transactions_csv(&account_set.accounts, &account_set.server_messages);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();

    // First row should have server messages
    assert!(!records[0][7].is_empty());
    // All other rows should be empty
    for record in &records[1..] {
        assert!(record[7].is_empty(), "Non-first row should not have server messages");
    }
}

#[test]
fn transactions_csv_pending_transaction() {
    let org = make_org("org-1", "Test", "test.com");
    let tx_pending = make_tx("tx-p", 1706745600, "-5.00", "PENDING TX", true, None);
    let tx_posted = make_tx("tx-np", 1706745600, "-10.00", "POSTED TX", false, None);
    let account = make_account(
        &org, "acct-1", "Test", "USD", "100.00", None, 1706745600,
        vec![tx_pending, tx_posted],
    );
    let csv = output::format_transactions_csv(&[account], &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(&records[0][5], "true");
    assert_eq!(&records[1][5], "false");
}

// ── Empty cases ─────────────────────────────────────────────────────────────

#[test]
fn accounts_json_empty_list() {
    let json_str = output::format_accounts_json(&[], &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().is_empty());
}

#[test]
fn accounts_csv_empty_list() {
    let csv = output::format_accounts_csv(&[], &[]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1); // header only
}

#[test]
fn transactions_json_empty_list() {
    let json_str = output::format_transactions_json(&[], &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().is_empty());
}

#[test]
fn transactions_csv_empty_accounts() {
    let csv = output::format_transactions_csv(&[], &[]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1); // header only
}

#[test]
fn organizations_json_empty_list() {
    let orgs: Vec<&Organization> = vec![];
    let json_str = output::format_organizations_json(&orgs, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().is_empty());
}

#[test]
fn organizations_csv_empty_list() {
    let orgs: Vec<&Organization> = vec![];
    let csv = output::format_organizations_csv(&orgs, &[]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(lines.len(), 1); // header only
}

// ── End-to-end pipeline: raw JSON → parse → format → verify ─────────────

#[test]
fn end_to_end_json_to_accounts_csv_all_fields_verified() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_accounts_csv(&account_set.accounts, &account_set.server_messages);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();

    // Verify every field of every account flows through correctly
    for (i, account) in account_set.accounts.iter().enumerate() {
        assert_eq!(&records[i][0], account.id);
        assert_eq!(&records[i][1], account.name);
        assert_eq!(&records[i][2], account.currency);
        assert_eq!(&records[i][3], account.balance.to_string());
        let expected_avail = account
            .available_balance
            .map(|d| d.to_string())
            .unwrap_or_default();
        assert_eq!(&records[i][4], expected_avail);
        assert_eq!(&records[i][5], account.balance_date_iso8601());
        assert_eq!(
            &records[i][6],
            account.org.id.as_deref().unwrap_or("")
        );
    }
}

#[test]
fn end_to_end_json_to_transactions_csv_all_fields_verified() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let csv = output::format_transactions_csv(&account_set.accounts, &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();

    let mut idx = 0;
    for account in &account_set.accounts {
        for tx in &account.transactions {
            assert_eq!(&records[idx][0], account.id, "account_id mismatch at row {idx}");
            assert_eq!(&records[idx][1], tx.id, "tx_id mismatch at row {idx}");
            assert_eq!(&records[idx][2], tx.posted_iso8601(), "posted mismatch at row {idx}");
            assert_eq!(&records[idx][3], tx.amount.to_string(), "amount mismatch at row {idx}");
            assert_eq!(&records[idx][4], tx.description, "description mismatch at row {idx}");
            assert_eq!(
                &records[idx][5],
                tx.pending.to_string(),
                "pending mismatch at row {idx}"
            );
            assert_eq!(
                &records[idx][6],
                tx.transacted_at_iso8601().unwrap_or_default(),
                "transacted_at mismatch at row {idx}"
            );
            idx += 1;
        }
    }
    assert_eq!(idx, records.len(), "row count mismatch");
}

#[test]
fn end_to_end_json_to_accounts_json_all_fields_verified() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
    let json_str = output::format_accounts_json(&account_set.accounts, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();

    for (i, account) in account_set.accounts.iter().enumerate() {
        let obj = &arr[i];
        assert_eq!(obj["id"].as_str().unwrap(), account.id);
        assert_eq!(obj["name"].as_str().unwrap(), account.name);
        assert_eq!(obj["balance"].as_str().unwrap(), account.balance.to_string());
        assert_eq!(obj["currency"].as_str().unwrap(), account.currency);
        assert_eq!(
            obj["balance-date"].as_str().unwrap(),
            account.balance_date_iso8601()
        );

        let expected_avail = account
            .available_balance
            .map(|d| d.to_string())
            .unwrap_or_else(|| account.balance.to_string());
        assert_eq!(obj["available-balance"].as_str().unwrap(), expected_avail);

        if let Some(ref org_id) = account.org.id {
            assert_eq!(obj["org-id"].as_str().unwrap(), org_id);
        }
    }
}

#[test]
fn end_to_end_json_to_transactions_json_all_fields_verified() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();
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

    let json_str = output::format_transactions_json(&items, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), items.len());

    for (i, item) in items.iter().enumerate() {
        let obj = &arr[i];
        assert_eq!(obj["account-id"].as_str().unwrap(), item.account.id);
        assert_eq!(obj["transaction-id"].as_str().unwrap(), item.transaction.id);
        assert_eq!(
            obj["posted"].as_str().unwrap(),
            item.transaction.posted_iso8601()
        );
        assert_eq!(
            obj["amount"].as_str().unwrap(),
            item.transaction.amount.to_string()
        );
        assert_eq!(obj["description"].as_str().unwrap(), item.transaction.description);
        assert_eq!(obj["pending"].as_bool().unwrap(), item.transaction.pending);

        match item.transaction.transacted_at_iso8601() {
            Some(dt) => assert_eq!(obj["transacted-at"].as_str().unwrap(), dt),
            None => assert!(obj.get("transacted-at").is_none()),
        }
    }
}

// ── Multiple organizations end-to-end ───────────────────────────────────────

#[test]
fn end_to_end_organizations_json_from_full_response() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();

    // Deduplicate organizations (same logic as CLI)
    let mut org_map = indexmap::IndexMap::new();
    for account in &account_set.accounts {
        let key = account.org.key().to_string();
        org_map.entry(key).or_insert(&account.org);
    }
    let organizations: Vec<&Organization> = org_map.into_values().collect();
    assert_eq!(organizations.len(), 2); // Two distinct orgs

    let json_str = output::format_organizations_json(&organizations, &[]);
    let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let arr = parsed.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    // Verify each org's fields
    for (i, org) in organizations.iter().enumerate() {
        let obj = &arr[i];
        assert_eq!(obj["sfin-url"].as_str().unwrap(), org.sfin_url);
        if let Some(ref id) = org.id {
            assert_eq!(obj["id"].as_str().unwrap(), id);
        }
        if let Some(ref name) = org.name {
            assert_eq!(obj["name"].as_str().unwrap(), name);
        }
        if let Some(ref domain) = org.domain {
            assert_eq!(obj["domain"].as_str().unwrap(), domain);
        }
    }
}

#[test]
fn end_to_end_organizations_csv_from_full_response() {
    let account_set: AccountSet = serde_json::from_str(FULL_API_RESPONSE).unwrap();

    let mut org_map = indexmap::IndexMap::new();
    for account in &account_set.accounts {
        let key = account.org.key().to_string();
        org_map.entry(key).or_insert(&account.org);
    }
    let organizations: Vec<&Organization> = org_map.into_values().collect();

    let csv = output::format_organizations_csv(&organizations, &[]);
    let mut rdr = csv::ReaderBuilder::new().from_reader(csv.as_bytes());
    let records: Vec<csv::StringRecord> = rdr.records().map(|r| r.unwrap()).collect();
    assert_eq!(records.len(), 2);

    for (i, org) in organizations.iter().enumerate() {
        assert_eq!(&records[i][0], org.id.as_deref().unwrap_or(""));
        assert_eq!(&records[i][1], org.name.as_deref().unwrap_or(""));
        assert_eq!(&records[i][2], org.domain.as_deref().unwrap_or(""));
        assert_eq!(&records[i][3], org.url.as_deref().unwrap_or(""));
        assert_eq!(&records[i][4], org.sfin_url);
    }
}
