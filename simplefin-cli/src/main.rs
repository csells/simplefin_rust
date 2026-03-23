use std::process::ExitCode;

use asupersync::Cx;
use asupersync::runtime::RuntimeBuilder;
use chrono::{DateTime, NaiveDate};
use clap::{Parser, Subcommand};

use rust_decimal::Decimal;
use std::str::FromStr;

use simplefin::{
    AccessClient, AccessCredentials, AccountQueryParams, BalanceHistoryFilter, BridgeClient,
    JsonStorage, ManualAccount, SimplefinError, Storage, TransactionFilter,
    DEFAULT_BRIDGE_ROOT_URL,
};

#[derive(Parser)]
#[command(name = "simplefin", about = "SimpleFIN Bridge CLI client")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Exchange a setup token for an access URL
    #[command(alias = "c")]
    Claim {
        /// The setup token from the SimpleFIN Bridge UI
        setup_token: String,
        /// SimpleFIN bridge root URL
        #[arg(short, long, default_value = DEFAULT_BRIDGE_ROOT_URL)]
        bridge: String,
    },

    /// Query the bridge for supported protocol versions
    #[command(alias = "i")]
    Info {
        /// SimpleFIN bridge root URL
        #[arg(short, long, default_value = DEFAULT_BRIDGE_ROOT_URL)]
        bridge: String,
    },

    /// Collect all financial data idempotently into local storage
    #[command(alias = "l")]
    Collect {
        /// SimpleFIN access URL (overrides SIMPLEFIN_ACCESS_URL env var)
        #[arg(short, long)]
        url: Option<String>,
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Show per-account breakdown
        #[arg(short, long)]
        verbose: bool,
    },

    /// Add or update a manual account balance (for accounts not in SimpleFIN)
    #[command(alias = "a")]
    AddBalance {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Account name (e.g. "My 401k")
        #[arg(short, long)]
        name: String,
        /// Organization name (e.g. "My Provider")
        #[arg(short, long)]
        org: String,
        /// Current balance
        #[arg(short, long)]
        balance: String,
        /// Currency code
        #[arg(short, long, default_value = "USD")]
        currency: String,
        /// How often to prompt for a balance update (days, default: 1)
        #[arg(short, long, default_value = "1")]
        refresh_days: u32,
    },

    /// Show manual accounts whose balances are stale and need updating
    #[command(alias = "t")]
    Stale {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
    },

    /// Query collected data as JSON
    #[command(alias = "q")]
    Query {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Filter to a single account (ID or name)
        #[arg(short, long)]
        account: Option<String>,
        /// Filter to an organization (ID or name)
        #[arg(short, long)]
        org: Option<String>,
        /// Start date filter (epoch, ISO-8601, or YYYY-MM-DD)
        #[arg(long)]
        start_date: Option<String>,
        /// End date filter (epoch, ISO-8601, or YYYY-MM-DD)
        #[arg(long)]
        end_date: Option<String>,
        /// Include pending transactions
        #[arg(short, long)]
        pending: bool,
    },

    /// Show categorized net worth summary with changes since last collection
    #[command(alias = "s")]
    Summary {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Include per-account breakdown within each category
        #[arg(short, long)]
        detail: bool,
    },

    /// Analyze spending by category over a date range
    #[command(alias = "p")]
    Spending {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Start date (epoch, ISO-8601, or YYYY-MM-DD)
        #[arg(long)]
        start_date: Option<String>,
        /// End date (epoch, ISO-8601, or YYYY-MM-DD)
        #[arg(long)]
        end_date: Option<String>,
    },

    /// Find and optionally remove orphaned data files
    Cleanup {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Actually remove orphaned files (dry-run by default)
        #[arg(long)]
        remove: bool,
    },
}

fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();

    let rt = match RuntimeBuilder::current_thread().build() {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("failed to build runtime: {e}");
            return ExitCode::FAILURE;
        }
    };

    rt.block_on(async {
        let cx = Cx::for_request();
        match run(&cx, cli.command).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("Error: {e}");
                ExitCode::FAILURE
            }
        }
    })
}

async fn run(cx: &Cx, command: Commands) -> simplefin::Result<()> {
    match command {
        Commands::Claim {
            setup_token,
            bridge,
        } => handle_claim(cx, &setup_token, &bridge).await,
        Commands::Info { bridge } => handle_info(cx, &bridge).await,
        Commands::Collect {
            url,
            storage,
            verbose,
        } => {
            let access_url = get_access_url(url.as_deref())?;
            handle_collect(cx, &access_url, &storage, verbose).await
        }
        Commands::AddBalance {
            storage,
            name,
            org,
            balance,
            currency,
            refresh_days,
        } => handle_add_balance(&storage, &name, &org, &balance, &currency, refresh_days),
        Commands::Stale { storage } => handle_stale(&storage),
        Commands::Query {
            storage,
            account,
            org,
            start_date,
            end_date,
            pending,
        } => handle_query(
            &storage,
            account.as_deref(),
            org.as_deref(),
            start_date.as_deref(),
            end_date.as_deref(),
            pending,
        ),
        Commands::Summary { storage, detail } => handle_summary(&storage, detail),
        Commands::Spending {
            storage,
            start_date,
            end_date,
        } => handle_spending(&storage, start_date.as_deref(), end_date.as_deref()),
        Commands::Cleanup { storage, remove } => handle_cleanup(&storage, remove),
    }
}

fn get_access_url(cli_url: Option<&str>) -> simplefin::Result<String> {
    cli_url
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("SIMPLEFIN_ACCESS_URL").ok())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| {
            SimplefinError::InvalidArgument(
                "no access URL provided. Set SIMPLEFIN_ACCESS_URL in .env or pass --url".into(),
            )
        })
}

fn parse_date(raw: &str) -> simplefin::Result<i64> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(SimplefinError::InvalidArgument(
            "date string is empty".into(),
        ));
    }

    if let Ok(epoch) = trimmed.parse::<i64>() {
        return Ok(epoch);
    }

    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.timestamp());
    }

    if let Ok(date) = NaiveDate::parse_from_str(trimmed, "%Y-%m-%d") {
        let dt = date.and_hms_opt(0, 0, 0).unwrap().and_utc();
        return Ok(dt.timestamp());
    }

    Err(SimplefinError::InvalidArgument(format!(
        "unable to parse date \"{trimmed}\". Use ISO-8601 (e.g. 2024-01-31T00:00:00Z) or epoch seconds."
    )))
}

async fn handle_claim(cx: &Cx, setup_token: &str, bridge: &str) -> simplefin::Result<()> {
    println!("Claiming access URL from setup token...");
    let client = BridgeClient::new(Some(bridge), None);
    let credentials = client.claim_access_credentials(cx, setup_token).await?;
    println!("SIMPLEFIN_ACCESS_URL={}", credentials.access_url);
    println!("Redirect or copy the line above into your .env file.");
    Ok(())
}

async fn handle_info(cx: &Cx, bridge: &str) -> simplefin::Result<()> {
    let client = BridgeClient::new(Some(bridge), None);
    let info = client.get_info(cx).await?;
    if info.versions.is_empty() {
        println!("No protocol versions reported by the bridge.");
    } else {
        println!("Bridge supports the following protocol versions:");
        for version in &info.versions {
            println!("- {version}");
        }
    }
    Ok(())
}

fn handle_add_balance(
    storage_path: &str,
    name: &str,
    org: &str,
    balance_str: &str,
    currency: &str,
    refresh_days: u32,
) -> simplefin::Result<()> {
    let balance = Decimal::from_str(balance_str).map_err(|_| {
        SimplefinError::InvalidArgument(format!("invalid balance: \"{balance_str}\""))
    })?;

    let mut storage = JsonStorage::open(storage_path)?;

    // Use a stable ID derived from org+name so repeated calls update the same account
    let id = format!("manual-{}-{}", slug(org), slug(name));

    let account = ManualAccount {
        id: id.clone(),
        name: name.to_string(),
        org_name: org.to_string(),
        currency: currency.to_string(),
        refresh_days,
    };
    storage.upsert_manual_accounts(&[account])?;

    let now = chrono::Utc::now().timestamp();
    storage.record_balance(&id, now, balance)?;

    println!("Recorded {org} / {name}: {balance} {currency} (refresh every {refresh_days} day(s))");
    Ok(())
}

fn handle_stale(storage_path: &str) -> simplefin::Result<()> {
    let storage = JsonStorage::open(storage_path)?;
    let now = chrono::Utc::now().timestamp();
    let stale = storage.get_stale_accounts(now)?;

    if stale.is_empty() {
        println!("All manual account balances are up to date.");
        return Ok(());
    }

    let output = serde_json::to_string_pretty(&stale).map_err(|e| SimplefinError::Storage {
        message: "failed to serialize stale accounts".into(),
        source: Some(Box::new(e)),
    })?;
    println!("{output}");

    Ok(())
}

/// Convert a string to a simple slug for use as an ID component.
fn slug(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

async fn handle_collect(
    cx: &Cx,
    access_url: &str,
    storage_path: &str,
    verbose: bool,
) -> simplefin::Result<()> {
    let credentials = AccessCredentials::parse(access_url)?;
    let client = AccessClient::new(credentials, None);
    let mut storage = JsonStorage::open(storage_path)?;

    // Load previous accounts for anomaly detection
    let previous_accounts = storage
        .get_accounts(&simplefin::AccountFilter::default())?;

    // Fetch all accounts with balances to know what we have
    let balances_params = AccountQueryParams {
        balances_only: true,
        ..Default::default()
    };
    let account_set = client.get_accounts(cx, &balances_params).await?;

    // Surface any messages from the bridge
    for msg in &account_set.server_messages {
        eprintln!("Bridge: {msg}");
    }

    if account_set.accounts.is_empty() {
        println!("No accounts returned by the bridge.");
        return Ok(());
    }

    // Detect anomalies by comparing current vs previous accounts
    if !previous_accounts.is_empty() {
        let anomalies =
            simplefin::detect_anomalies(&account_set.accounts, &previous_accounts);
        for anomaly in &anomalies {
            eprintln!("{anomaly}");
        }
    }

    // Upsert organizations
    let orgs: Vec<_> = account_set
        .accounts
        .iter()
        .map(|a| a.org.clone())
        .collect();
    storage.upsert_organizations(&orgs)?;

    // Upsert accounts and record balance snapshots
    storage.upsert_accounts(&account_set.accounts)?;
    let now = chrono::Utc::now().timestamp();
    for account in &account_set.accounts {
        storage.record_balance(&account.id, now, account.balance)?;
    }

    // Fetch transactions for each account
    let mut total_new = 0usize;
    let mut total_dupes = 0usize;
    let mut account_count = 0usize;

    for account in &account_set.accounts {
        // Default to epoch 0 to fetch all available history on first collection,
        // since the SimpleFIN API returns no transactions without a start-date.
        let start_date = storage.last_collected(&account.id)?.or(Some(0));

        let params = AccountQueryParams {
            start_date,
            include_pending: true,
            account_ids: Some(vec![account.id.clone()]),
            balances_only: false,
            ..Default::default()
        };

        let result = client.get_accounts(cx, &params).await?;

        for msg in &result.server_messages {
            eprintln!("Bridge ({}): {msg}", account.name);
        }

        for fetched_account in &result.accounts {
            let txn_count = fetched_account.transactions.len();
            let new_count =
                storage.upsert_transactions(&fetched_account.id, &fetched_account.transactions)?;
            let dupe_count = txn_count - new_count;

            if let Some(max_posted) = fetched_account.transactions.iter().map(|t| t.posted).max() {
                storage.set_last_collected(&fetched_account.id, max_posted)?;
            }

            if verbose {
                println!(
                    "  {}: {} new, {} existing",
                    fetched_account.name, new_count, dupe_count
                );
            }

            total_new += new_count;
            total_dupes += dupe_count;
            account_count += 1;
        }
    }

    println!(
        "Collected {} new transactions across {} accounts ({} duplicates skipped)",
        total_new, account_count, total_dupes
    );

    Ok(())
}

fn handle_query(
    storage_path: &str,
    account: Option<&str>,
    org: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    include_pending: bool,
) -> simplefin::Result<()> {
    let storage = JsonStorage::open(storage_path)?;

    let start = start_date.map(parse_date).transpose()?;
    let end = end_date.map(parse_date).transpose()?;
    let org_id = org.map(|s| s.to_string());
    let account_id = account.map(|s| s.to_string());

    let organizations = storage.get_organizations(&simplefin::OrgFilter {
        org_id: org_id.clone(),
        ..Default::default()
    })?;

    let accounts = storage.get_accounts(&simplefin::AccountFilter {
        account_id: account_id.clone(),
        org_id: org_id.clone(),
        ..Default::default()
    })?;

    let transactions = storage.get_transactions(&TransactionFilter {
        account_id,
        org_id,
        start_date: start,
        end_date: end,
        include_pending: Some(include_pending),
    })?;

    let manual_accounts = storage.get_manual_accounts()?;
    let balance_history = storage.get_balance_history(&BalanceHistoryFilter {
        start_date: start,
        end_date: end,
        ..Default::default()
    })?;

    let unified = simplefin::unify_accounts(&accounts, &manual_accounts, &balance_history);

    let output = serde_json::json!({
        "organizations": organizations,
        "accounts": unified,
        "transactions": transactions,
        "balance_history": balance_history,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| {
            SimplefinError::Storage {
                message: "failed to serialize query output".into(),
                source: Some(Box::new(e)),
            }
        })?
    );

    Ok(())
}

fn handle_summary(storage_path: &str, detail: bool) -> simplefin::Result<()> {
    let storage = JsonStorage::open(storage_path)?;
    let config = storage.get_config()?;

    let accounts = storage.get_accounts(&simplefin::AccountFilter::default())?;
    let manual_accounts = storage.get_manual_accounts()?;
    let all_history = storage.get_balance_history(&BalanceHistoryFilter::default())?;

    let unified = simplefin::unify_accounts(&accounts, &manual_accounts, &all_history);
    let summary = simplefin::compute_net_worth_detail(&unified, &config, detail);

    // Find the two most recent distinct collection timestamps for change reporting
    let mut timestamps: Vec<i64> = all_history.iter().map(|s| s.timestamp).collect();
    timestamps.sort();
    timestamps.dedup();

    let changes = if timestamps.len() >= 2 {
        let prev_ts = timestamps[timestamps.len() - 2];
        let previous: Vec<_> = all_history
            .iter()
            .filter(|s| s.timestamp <= prev_ts)
            .cloned()
            .collect();
        simplefin::compute_changes(&unified, &all_history, &previous, &config)
    } else {
        Vec::new()
    };

    let output = serde_json::json!({
        "net_worth": summary,
        "changes": changes,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|e| {
            SimplefinError::Storage {
                message: "failed to serialize summary output".into(),
                source: Some(Box::new(e)),
            }
        })?
    );

    Ok(())
}

fn handle_spending(
    storage_path: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> simplefin::Result<()> {
    let storage = JsonStorage::open(storage_path)?;
    let config = storage.get_config()?;

    let start = start_date.map(parse_date).transpose()?;
    let end = end_date.map(parse_date).transpose()?;

    let transactions = storage.get_transactions(&TransactionFilter {
        start_date: start,
        end_date: end,
        include_pending: Some(false),
        ..Default::default()
    })?;

    let summary = simplefin::compute_spending(&transactions, &config.spending_rules);

    let output = serde_json::to_string_pretty(&summary).map_err(|e| SimplefinError::Storage {
        message: "failed to serialize spending output".into(),
        source: Some(Box::new(e)),
    })?;
    println!("{output}");

    Ok(())
}

fn handle_cleanup(storage_path: &str, remove: bool) -> simplefin::Result<()> {
    let storage = JsonStorage::open(storage_path)?;
    let orphans = storage.find_orphaned_data()?;

    if orphans.is_empty() {
        println!("No orphaned data found.");
        return Ok(());
    }

    if remove {
        println!("Removing {} orphaned file(s):", orphans.len());
        for orphan in &orphans {
            println!(
                "  {} ({:?}): {}",
                orphan.account_id, orphan.data_type, orphan.path
            );
        }
        storage.remove_orphaned_data(&orphans)?;
        println!("Done.");
    } else {
        println!(
            "Found {} orphaned file(s) (use --remove to delete):",
            orphans.len()
        );
        for orphan in &orphans {
            println!(
                "  {} ({:?}): {}",
                orphan.account_id, orphan.data_type, orphan.path
            );
        }
    }

    Ok(())
}
