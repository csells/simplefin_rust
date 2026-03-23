use rust_decimal::Decimal;

/// Format a Decimal as a currency string with commas and $ prefix.
pub fn format_currency(d: Decimal) -> String {
    let is_negative = d < Decimal::ZERO;
    let abs = d.abs();
    let whole = abs.trunc().to_string();
    let frac = abs.fract();
    let cents = (frac * Decimal::ONE_HUNDRED)
        .round()
        .abs()
        .to_string();
    let cents = if cents.len() == 1 {
        format!("0{cents}")
    } else {
        cents
    };

    // Add commas to whole part
    let with_commas = add_commas(&whole);

    if is_negative {
        format!("-${with_commas}.{cents}")
    } else {
        format!("${with_commas}.{cents}")
    }
}

fn add_commas(s: &str) -> String {
    let bytes: Vec<u8> = s.bytes().rev().collect();
    let chunks: Vec<String> = bytes
        .chunks(3)
        .map(|chunk| {
            chunk
                .iter()
                .rev()
                .map(|&b| b as char)
                .collect::<String>()
        })
        .collect();
    let mut chunks = chunks;
    chunks.reverse();
    chunks.join(",")
}

/// Format the summary output as a human-readable table.
pub fn format_summary(data: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("Net Worth Summary\n");
    out.push_str(&"═".repeat(50));
    out.push('\n');

    if let Some(nw) = data.get("net_worth") {
        if let Some(categories) = nw.get("categories").and_then(|c| c.as_array()) {
            let mut in_liabilities = false;
            for cat in categories {
                let label = cat.get("label").and_then(|l| l.as_str()).unwrap_or("?");
                let total = parse_decimal(cat.get("total"));
                let is_liability = matches!(label, "Credit Cards" | "Loans");

                if is_liability && !in_liabilities {
                    out.push('\n');
                    in_liabilities = true;
                }

                out.push_str(&format!("  {:<26} {:>20}\n", label, format_currency(total)));

                // Show account details if present
                if let Some(accounts) = cat.get("accounts").and_then(|a| a.as_array()) {
                    for acct in accounts {
                        let name = acct.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                        let bal = parse_decimal(acct.get("balance"));
                        out.push_str(&format!(
                            "    {:<24} {:>20}\n",
                            name,
                            format_currency(bal)
                        ));
                    }
                }
            }
        }

        out.push_str(&"─".repeat(50));
        out.push('\n');

        let total_assets = parse_decimal(nw.get("total_assets"));
        let total_liabilities = parse_decimal(nw.get("total_liabilities"));
        let net_worth = parse_decimal(nw.get("net_worth"));

        out.push_str(&format!(
            "  {:<26} {:>20}\n",
            "Total Assets",
            format_currency(total_assets)
        ));
        out.push_str(&format!(
            "  {:<26} {:>20}\n",
            "Total Liabilities",
            format_currency(total_liabilities)
        ));
        out.push_str(&format!(
            "  {:<26} {:>20}\n",
            "NET WORTH",
            format_currency(net_worth)
        ));
    }

    // Changes section
    if let Some(changes) = data.get("changes").and_then(|c| c.as_array())
        && !changes.is_empty()
    {
        out.push('\n');
        out.push_str("Changes Since Last Collection\n");
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for change in changes {
            let name = change
                .get("account_name")
                .and_then(|n| n.as_str())
                .unwrap_or("?");
            let delta = parse_decimal(change.get("change"));
            let sign = if delta >= Decimal::ZERO { "+" } else { "" };
            out.push_str(&format!(
                "  {:<26} {:>20}\n",
                name,
                format!("{sign}{}", format_currency(delta))
            ));
        }
    }

    // History section
    if let Some(history) = data.get("history").and_then(|h| h.as_array())
        && !history.is_empty()
    {
        out.push('\n');
        out.push_str("Net Worth History\n");
        out.push_str(&"─".repeat(50));
        out.push('\n');
        for point in history {
            let ts = point.get("timestamp").and_then(|t| t.as_i64()).unwrap_or(0);
            let nw = parse_decimal(point.get("net_worth"));
            let date = format_timestamp(ts);
            out.push_str(&format!("  {:<26} {:>20}\n", date, format_currency(nw)));
        }
    }

    out
}

/// Format the status output as a human-readable dashboard.
pub fn format_status(data: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("Storage Status\n");
    out.push_str(&"═".repeat(50));
    out.push('\n');

    let last_ago = data
        .get("last_collection_ago")
        .and_then(|v| v.as_str())
        .unwrap_or("never");
    out.push_str(&format!("  Last collection:    {last_ago}\n"));

    let accounts = data
        .get("account_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let manual = data
        .get("manual_account_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    out.push_str(&format!("  Accounts:           {accounts} SimpleFIN + {manual} manual\n"));

    if let Some(stale) = data.get("stale_manual_accounts").and_then(|v| v.as_array()) {
        if stale.is_empty() {
            out.push_str("  Stale accounts:     none\n");
        } else {
            let names: Vec<&str> = stale.iter().filter_map(|v| v.as_str()).collect();
            out.push_str(&format!("  Stale accounts:     {}\n", names.join(", ")));
        }
    }

    if let Some(warnings) = data.get("warnings") {
        if let Some(anomalies) = warnings.get("anomalies").and_then(|a| a.as_array())
            && !anomalies.is_empty()
        {
            out.push_str(&format!("  Anomalies:          {}\n", anomalies.len()));
        }
        if let Some(msgs) = warnings.get("bridge_messages").and_then(|m| m.as_array())
            && !msgs.is_empty()
        {
            out.push_str(&format!("  Bridge messages:    {}\n", msgs.len()));
        }
    }

    out
}

/// Format spending output as a human-readable table.
pub fn format_spending(data: &serde_json::Value) -> String {
    let mut out = String::new();
    out.push_str("Spending Summary\n");
    out.push_str(&"═".repeat(60));
    out.push('\n');

    if let Some(categories) = data.get("categories").and_then(|c| c.as_array()) {
        for cat in categories {
            let label = cat.get("label").and_then(|l| l.as_str()).unwrap_or("?");
            let total = parse_decimal(cat.get("total"));
            let count = cat
                .get("transaction_count")
                .and_then(|c| c.as_u64())
                .unwrap_or(0);
            out.push_str(&format!(
                "  {:<26} {:>20}  ({} txns)\n",
                label,
                format_currency(total),
                count
            ));
        }
    }

    out.push_str(&"─".repeat(60));
    out.push('\n');

    let total_spending = parse_decimal(data.get("total_spending"));
    let total_income = parse_decimal(data.get("total_income"));
    let net = parse_decimal(data.get("net"));

    out.push_str(&format!(
        "  {:<26} {:>20}\n",
        "Total Spending",
        format_currency(total_spending)
    ));
    out.push_str(&format!(
        "  {:<26} {:>20}\n",
        "Total Income",
        format_currency(total_income)
    ));
    out.push_str(&format!(
        "  {:<26} {:>20}\n",
        "Net",
        format_currency(net)
    ));

    out
}

/// Format the query output as a human-readable table.
pub fn format_query(data: &serde_json::Value) -> String {
    let mut out = String::new();

    if let Some(accounts) = data.get("accounts").and_then(|a| a.as_array()) {
        out.push_str(&format!("Accounts ({})\n", accounts.len()));
        out.push_str(&"═".repeat(70));
        out.push('\n');
        for acct in accounts {
            let name = acct.get("name").and_then(|n| n.as_str()).unwrap_or("?");
            let org = acct.get("org_name").and_then(|o| o.as_str()).unwrap_or("?");
            let bal = parse_decimal(acct.get("balance"));
            let source = acct.get("source").and_then(|s| s.as_str()).unwrap_or("?");
            out.push_str(&format!(
                "  {:<30} {:<15} {:>20}  [{}]\n",
                name,
                org,
                format_currency(bal),
                source
            ));
        }
    }

    if let Some(txns) = data.get("transactions").and_then(|t| t.as_array())
        && !txns.is_empty()
    {
        out.push('\n');
        out.push_str(&format!("Transactions ({})\n", txns.len()));
        out.push_str(&"─".repeat(70));
        out.push('\n');
        for txn in txns {
            let desc = txn
                .get("description")
                .and_then(|d| d.as_str())
                .unwrap_or("?");
            let amount = parse_decimal(txn.get("amount"));
            let ts = txn.get("posted").and_then(|p| p.as_i64()).unwrap_or(0);
            let date = format_timestamp(ts);
            let acct = txn
                .get("account_name")
                .and_then(|a| a.as_str())
                .unwrap_or("?");
            out.push_str(&format!(
                "  {} {:<25} {:>15}  {}\n",
                date,
                truncate(desc, 25),
                format_currency(amount),
                acct
            ));
        }
    }

    out
}

/// Format stale accounts as a human-readable list.
pub fn format_stale(data: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(arr) = data.as_array() {
        if arr.is_empty() {
            out.push_str("All manual account balances are up to date.\n");
        } else {
            out.push_str("Stale Manual Accounts\n");
            out.push_str(&"═".repeat(50));
            out.push('\n');
            for acct in arr {
                let name = acct.get("name").and_then(|n| n.as_str()).unwrap_or("?");
                let org = acct.get("org_name").and_then(|o| o.as_str()).unwrap_or("?");
                let days = acct.get("days_since_update").and_then(|d| d.as_u64());
                let age = match days {
                    Some(d) => format!("{d} days ago"),
                    None => "never".to_string(),
                };
                out.push_str(&format!("  {name} ({org}) — last updated {age}\n"));
            }
        }
    }
    out
}

/// Format the configure list output as a human-readable table.
pub fn format_configure(data: &serde_json::Value) -> String {
    let mut out = String::new();
    if let Some(accounts) = data.get("accounts").and_then(|a| a.as_array()) {
        out.push_str("Account Configuration\n");
        out.push_str(&"═".repeat(80));
        out.push('\n');
        for acct in accounts {
            let name = acct
                .get("display_name")
                .and_then(|n| n.as_str())
                .unwrap_or("?");
            let id = acct.get("id").and_then(|i| i.as_str()).unwrap_or("?");
            let effective = acct
                .get("effective_classification")
                .and_then(|c| c.as_str())
                .unwrap_or("?");
            let overridden = acct
                .get("overridden")
                .and_then(|o| o.as_bool())
                .unwrap_or(false);
            let excluded = acct
                .get("excluded")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            let bal = parse_decimal(acct.get("balance"));

            let confident = acct
                .get("confident")
                .and_then(|c| c.as_bool())
                .unwrap_or(true);

            let mut flags = Vec::new();
            if overridden {
                flags.push("overridden");
            }
            if excluded {
                flags.push("excluded");
            }
            if !confident && !overridden {
                flags.push("? review");
            }
            let flag_str = if flags.is_empty() {
                String::new()
            } else {
                format!("  [{}]", flags.join(", "))
            };

            out.push_str(&format!(
                "  {:<30} {:>14}  {:<16}{}\n",
                truncate(name, 30),
                format_currency(bal),
                effective,
                flag_str
            ));
            out.push_str(&format!("    ID: {id}\n"));
        }
    }
    out
}

/// Format a general message or simple JSON value as text.
pub fn format_message(data: &serde_json::Value) -> String {
    if let Some(msg) = data.get("message").and_then(|m| m.as_str()) {
        format!("{msg}\n")
    } else {
        // Fall back to pretty JSON for types we don't have a formatter for
        serde_json::to_string_pretty(data).unwrap_or_default()
    }
}

fn parse_decimal(v: Option<&serde_json::Value>) -> Decimal {
    match v {
        Some(serde_json::Value::String(s)) => s.parse().unwrap_or(Decimal::ZERO),
        Some(serde_json::Value::Number(n)) => {
            if let Some(f) = n.as_f64() {
                Decimal::try_from(f).unwrap_or(Decimal::ZERO)
            } else {
                Decimal::ZERO
            }
        }
        _ => Decimal::ZERO,
    }
}

fn format_timestamp(ts: i64) -> String {
    chrono::DateTime::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..max - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn format_currency_positive() {
        assert_eq!(
            format_currency(Decimal::from_str("1234.56").unwrap()),
            "$1,234.56"
        );
    }

    #[test]
    fn format_currency_negative() {
        assert_eq!(
            format_currency(Decimal::from_str("-1234.56").unwrap()),
            "-$1,234.56"
        );
    }

    #[test]
    fn format_currency_zero() {
        assert_eq!(format_currency(Decimal::ZERO), "$0.00");
    }

    #[test]
    fn format_currency_large() {
        assert_eq!(
            format_currency(Decimal::from_str("1234567.89").unwrap()),
            "$1,234,567.89"
        );
    }

    #[test]
    fn format_currency_small() {
        assert_eq!(
            format_currency(Decimal::from_str("0.50").unwrap()),
            "$0.50"
        );
    }

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate("hello world", 8), "hello w…");
    }
}
