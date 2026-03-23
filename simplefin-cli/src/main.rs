mod format;

use std::process::ExitCode;

use asupersync::Cx;
use asupersync::runtime::RuntimeBuilder;
use chrono::{DateTime, NaiveDate};
use clap::{Parser, Subcommand, ValueEnum};
use serde::Serialize;

use rust_decimal::Decimal;
use std::str::FromStr;

use simplefin::{
    AccessClient, AccessCredentials, AccountQueryParams, BalanceHistoryFilter, BridgeClient,
    JsonStorage, ManualAccount, SimplefinError, Storage, TransactionFilter, WarningRecord,
    DEFAULT_BRIDGE_ROOT_URL,
};

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Json,
    Text,
}

#[derive(Parser)]
#[command(name = "simplefin", about = "SimpleFIN Bridge CLI client")]
struct Cli {
    /// Output raw JSON without the envelope wrapper
    #[arg(long, global = true)]
    raw: bool,

    /// Output format
    #[arg(long, global = true, default_value = "json")]
    format: OutputFormat,

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
        /// Show net worth over the last N collection timestamps
        #[arg(long)]
        history: Option<usize>,
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

    /// View and modify account configuration (classifications, display names, exclusions)
    #[command(alias = "cfg")]
    Configure {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// List all accounts with their current configuration
        #[arg(long)]
        list: bool,
        /// Account ID to configure
        #[arg(long)]
        set: Option<String>,
        /// Display name to assign
        #[arg(long)]
        name: Option<String>,
        /// Category override (cash, investments, other_assets, credit_cards, loans)
        #[arg(long)]
        category: Option<String>,
        /// Exclude this account from net worth calculations
        #[arg(long)]
        exclude: bool,
        /// Include this account in net worth calculations (remove exclusion)
        #[arg(long)]
        include: bool,
    },

    /// Show storage status: last collection time, account counts, stale accounts, warnings
    #[command(alias = "st")]
    Status {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
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

    /// Manage spending classification patterns stored in the data directory
    #[command(alias = "sr")]
    SpendingRules {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// List all current patterns
        #[arg(long)]
        list: bool,
        /// Add a pattern for a category (e.g. --add "hulu|disney" --category entertainment)
        #[arg(long)]
        add: Option<String>,
        /// Category for the pattern being added
        #[arg(long)]
        category: Option<String>,
        /// Remove a pattern by substring match
        #[arg(long)]
        remove: Option<String>,
        /// Reset patterns to defaults (overwrites all customizations)
        #[arg(long)]
        reset: bool,
    },

    /// Detect recurring expenses from transaction patterns
    #[command(alias = "r")]
    Recurring {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Minimum number of occurrences to consider recurring (default: 2)
        #[arg(long, default_value = "2")]
        min_occurrences: usize,
    },

    /// Analyze spending trends over time (month-over-month by category)
    #[command(alias = "tr")]
    Trends {
        /// Storage directory path
        #[arg(short, long)]
        storage: String,
        /// Number of months to analyze (default: 6)
        #[arg(long, default_value = "6")]
        months: usize,
    },

    /// Print JSON Schema for a given output type
    Schema {
        /// Output type: summary, query, spending, status, configure, accounts, transactions, stale, warnings, history, changes, recurring, trends
        output_type: String,
    },
}

/// Identifies which command produced the output, for text formatting.
enum CommandKind {
    Summary,
    Status,
    Spending,
    Query,
    Stale,
    Configure,
    Message,
    Cleanup,
    Schema,
    Recurring,
    Trends,
}

/// The result of a command handler, containing data and optionally a storage
/// path for loading persisted warnings into the envelope.
struct CommandOutput {
    data: serde_json::Value,
    storage_path: Option<String>,
    kind: CommandKind,
}

/// Structured envelope wrapping all CLI output.
#[derive(Serialize)]
struct Envelope {
    data: serde_json::Value,
    warnings: Vec<String>,
    errors: Vec<String>,
}

fn main() -> ExitCode {
    dotenvy::dotenv().ok();
    let cli = Cli::parse();
    let raw = cli.raw;
    let output_format = cli.format.clone();

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
            Ok(output) => {
                match output_format {
                    OutputFormat::Text => {
                        print!("{}", format_output_text(&output));
                    }
                    OutputFormat::Json => {
                        if raw {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&output.data).unwrap_or_default()
                            );
                        } else {
                            let warnings =
                                load_warnings_for_envelope(output.storage_path.as_deref());
                            let envelope = Envelope {
                                data: output.data,
                                warnings,
                                errors: Vec::new(),
                            };
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&envelope).unwrap_or_default()
                            );
                        }
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                match output_format {
                    OutputFormat::Text => {
                        eprintln!("Error: {e}");
                    }
                    OutputFormat::Json => {
                        if raw {
                            eprintln!("Error: {e}");
                        } else {
                            let envelope = Envelope {
                                data: serde_json::Value::Null,
                                warnings: Vec::new(),
                                errors: vec![e.to_string()],
                            };
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&envelope).unwrap_or_default()
                            );
                        }
                    }
                }
                ExitCode::FAILURE
            }
        }
    })
}

/// Route output through the appropriate text formatter based on command kind.
fn format_output_text(output: &CommandOutput) -> String {
    match output.kind {
        CommandKind::Summary => format::format_summary(&output.data),
        CommandKind::Status => format::format_status(&output.data),
        CommandKind::Spending => format::format_spending(&output.data),
        CommandKind::Query => format::format_query(&output.data),
        CommandKind::Stale => format::format_stale(&output.data),
        CommandKind::Configure => format::format_configure(&output.data),
        CommandKind::Recurring => format::format_recurring(&output.data),
        CommandKind::Trends => format::format_trends(&output.data),
        CommandKind::Message | CommandKind::Cleanup | CommandKind::Schema => {
            format::format_message(&output.data)
        }
    }
}

/// Load persisted warnings from storage for inclusion in the envelope.
fn load_warnings_for_envelope(storage_path: Option<&str>) -> Vec<String> {
    let Some(path) = storage_path else {
        return Vec::new();
    };
    let Ok(storage) = JsonStorage::open(path) else {
        return Vec::new();
    };
    let Ok(Some(record)) = storage.get_warnings() else {
        return Vec::new();
    };
    let mut warnings = Vec::new();
    for anomaly in &record.anomalies {
        warnings.push(anomaly.to_string());
    }
    for msg in &record.bridge_messages {
        warnings.push(format!("Bridge: {msg}"));
    }
    warnings
}

async fn run(cx: &Cx, command: Commands) -> simplefin::Result<CommandOutput> {
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
        Commands::Configure {
            storage,
            list,
            set,
            name,
            category,
            exclude,
            include,
        } => handle_configure(
            &storage,
            list,
            set.as_deref(),
            name.as_deref(),
            category.as_deref(),
            exclude,
            include,
        ),
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
        Commands::Summary {
            storage,
            detail,
            history,
        } => handle_summary(&storage, detail, history),
        Commands::Spending {
            storage,
            start_date,
            end_date,
        } => handle_spending(&storage, start_date.as_deref(), end_date.as_deref()),
        Commands::Status { storage } => handle_status(&storage),
        Commands::Cleanup { storage, remove } => handle_cleanup(&storage, remove),
        Commands::SpendingRules {
            storage,
            list,
            add,
            category,
            remove,
            reset,
        } => handle_spending_rules(&storage, list, add.as_deref(), category.as_deref(), remove.as_deref(), reset),
        Commands::Recurring {
            storage,
            min_occurrences,
        } => handle_recurring(&storage, min_occurrences),
        Commands::Trends { storage, months } => handle_trends(&storage, months),
        Commands::Schema { output_type } => handle_schema(&output_type),
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

async fn handle_claim(
    cx: &Cx,
    setup_token: &str,
    bridge: &str,
) -> simplefin::Result<CommandOutput> {
    eprintln!("Claiming access URL from setup token...");
    let client = BridgeClient::new(Some(bridge), None);
    let credentials = client.claim_access_credentials(cx, setup_token).await?;
    Ok(CommandOutput {
        data: serde_json::json!({
            "access_url": credentials.access_url,
            "message": "Add the access_url value to your .env file as SIMPLEFIN_ACCESS_URL"
        }),
        storage_path: None,
        kind: CommandKind::Message,
    })
}

async fn handle_info(cx: &Cx, bridge: &str) -> simplefin::Result<CommandOutput> {
    let client = BridgeClient::new(Some(bridge), None);
    let info = client.get_info(cx).await?;
    Ok(CommandOutput {
        data: serde_json::json!({ "versions": info.versions }),
        storage_path: None,
        kind: CommandKind::Message,
    })
}

fn handle_add_balance(
    storage_path: &str,
    name: &str,
    org: &str,
    balance_str: &str,
    currency: &str,
    refresh_days: u32,
) -> simplefin::Result<CommandOutput> {
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

    Ok(CommandOutput {
        data: serde_json::json!({
            "id": id,
            "name": name,
            "org": org,
            "balance": balance.to_string(),
            "currency": currency,
            "refresh_days": refresh_days,
            "message": format!("Recorded {org} / {name}: {balance} {currency} (refresh every {refresh_days} day(s))")
        }),
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Message,
    })
}

fn handle_stale(storage_path: &str) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;
    let now = chrono::Utc::now().timestamp();
    let stale = storage.get_stale_accounts(now)?;

    Ok(CommandOutput {
        data: serde_json::to_value(&stale).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize stale accounts".into(),
            source: Some(Box::new(e)),
        })?,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Stale,
    })
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
) -> simplefin::Result<CommandOutput> {
    let credentials = AccessCredentials::parse(access_url)?;
    let client = AccessClient::new(credentials, None);
    let mut storage = JsonStorage::open(storage_path)?;

    // Load previous accounts for anomaly detection
    let previous_accounts = storage.get_accounts(&simplefin::AccountFilter::default())?;

    // Fetch all accounts with balances to know what we have
    let balances_params = AccountQueryParams {
        balances_only: true,
        ..Default::default()
    };
    let account_set = client.get_accounts(cx, &balances_params).await?;

    // Collect bridge messages for persistence
    let mut all_bridge_messages: Vec<String> = Vec::new();

    // Surface any messages from the bridge
    for msg in &account_set.server_messages {
        eprintln!("Bridge: {msg}");
        all_bridge_messages.push(msg.clone());
    }

    if account_set.accounts.is_empty() {
        return Ok(CommandOutput {
            data: serde_json::json!({
                "new_transactions": 0,
                "accounts": 0,
                "duplicates_skipped": 0,
                "message": "No accounts returned by the bridge."
            }),
            storage_path: Some(storage_path.to_string()),
            kind: CommandKind::Message,
        });
    }

    // Detect anomalies by comparing current vs previous accounts
    let anomalies = if !previous_accounts.is_empty() {
        let detected =
            simplefin::detect_anomalies(&account_set.accounts, &previous_accounts);
        for anomaly in &detected {
            eprintln!("{anomaly}");
        }
        detected
    } else {
        Vec::new()
    };

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
            all_bridge_messages.push(format!("{}: {msg}", account.name));
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
                eprintln!(
                    "  {}: {} new, {} existing",
                    fetched_account.name, new_count, dupe_count
                );
            }

            total_new += new_count;
            total_dupes += dupe_count;
            account_count += 1;
        }
    }

    // Persist warnings for later retrieval by status/summary
    let warning_record = WarningRecord {
        timestamp: now,
        anomalies,
        bridge_messages: all_bridge_messages,
    };
    storage.save_warnings(&warning_record)?;

    let is_first_run = previous_accounts.is_empty();

    let mut data = serde_json::json!({
        "new_transactions": total_new,
        "accounts": account_count,
        "duplicates_skipped": total_dupes,
        "first_run": is_first_run,
        "message": format!(
            "Collected {} new transactions across {} accounts ({} duplicates skipped)",
            total_new, account_count, total_dupes
        )
    });

    if is_first_run {
        data["hint"] = serde_json::json!(
            "First collection complete. Run 'simplefin configure --list' to review account classifications."
        );
        eprintln!("Hint: Run 'simplefin configure --list' to review account classifications.");
    }

    Ok(CommandOutput {
        data,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Message,
    })
}

fn handle_query(
    storage_path: &str,
    account: Option<&str>,
    org: Option<&str>,
    start_date: Option<&str>,
    end_date: Option<&str>,
    include_pending: bool,
) -> simplefin::Result<CommandOutput> {
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

    Ok(CommandOutput {
        data: serde_json::json!({
            "organizations": organizations,
            "accounts": unified,
            "transactions": transactions,
            "balance_history": balance_history,
        }),
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Query,
    })
}

fn handle_summary(
    storage_path: &str,
    detail: bool,
    history: Option<usize>,
) -> simplefin::Result<CommandOutput> {
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

    let mut data = serde_json::json!({
        "net_worth": summary,
        "changes": changes,
    });

    if let Some(n) = history {
        let time_series =
            simplefin::compute_net_worth_history(&all_history, &unified, &config, n);
        data["history"] = serde_json::to_value(&time_series).unwrap_or_default();
    }

    Ok(CommandOutput {
        data,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Summary,
    })
}

fn handle_spending(
    storage_path: &str,
    start_date: Option<&str>,
    end_date: Option<&str>,
) -> simplefin::Result<CommandOutput> {
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

    // User rules (from config) take priority, then patterns from storage
    let patterns = storage.get_spending_patterns()?;
    let mut rules = config.spending_rules.clone();
    rules.extend(patterns);

    let summary = simplefin::compute_spending(&transactions, &rules);

    Ok(CommandOutput {
        data: serde_json::to_value(&summary).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize spending output".into(),
            source: Some(Box::new(e)),
        })?,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Spending,
    })
}

fn handle_configure(
    storage_path: &str,
    list: bool,
    set_id: Option<&str>,
    display_name: Option<&str>,
    category: Option<&str>,
    exclude: bool,
    include: bool,
) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;

    if list || set_id.is_none() {
        // List mode: show all accounts with configuration
        let accounts = storage.get_accounts(&simplefin::AccountFilter::default())?;
        let manual_accounts = storage.get_manual_accounts()?;
        let all_history = storage.get_balance_history(&BalanceHistoryFilter::default())?;
        let config = storage.get_config()?;

        let unified = simplefin::unify_accounts(&accounts, &manual_accounts, &all_history);

        let account_configs: Vec<serde_json::Value> = unified
            .iter()
            .map(|account| {
                let classification = simplefin::classify_for_display(account, &config);
                let excluded = simplefin::account_is_excluded(account, &config);
                let display_name = simplefin::display_name_for(account, &config);
                serde_json::json!({
                    "id": account.id,
                    "name": account.name,
                    "display_name": display_name,
                    "org_name": account.org_name,
                    "source": account.source,
                    "balance": account.balance.to_string(),
                    "heuristic_classification": classification.heuristic,
                    "effective_classification": classification.effective,
                    "overridden": classification.overridden,
                    "confident": classification.confident,
                    "excluded": excluded,
                })
            })
            .collect();

        return Ok(CommandOutput {
            data: serde_json::json!({ "accounts": account_configs }),
            storage_path: Some(storage_path.to_string()),
            kind: CommandKind::Configure,
        });
    }

    // Set mode: modify config for a specific account
    let account_id = set_id.unwrap();
    let mut config = storage.get_config()?;
    let mut changes = Vec::new();

    if let Some(name) = display_name {
        config
            .display_names
            .insert(account_id.to_string(), name.to_string());
        changes.push(format!("display name set to \"{name}\""));
    }

    if let Some(cat_str) = category {
        let cat = parse_category(cat_str)?;
        config
            .classification_overrides
            .insert(account_id.to_string(), cat);
        changes.push(format!("classification set to {cat}"));
    }

    if exclude {
        if !config.excluded_account_ids.contains(&account_id.to_string()) {
            config.excluded_account_ids.push(account_id.to_string());
        }
        changes.push("excluded from net worth".to_string());
    }

    if include {
        config
            .excluded_account_ids
            .retain(|id| id != account_id);
        // Also remove from pattern-based exclusions if the ID matches
        changes.push("included in net worth".to_string());
    }

    if changes.is_empty() {
        return Err(SimplefinError::InvalidArgument(
            "no changes specified. Use --name, --category, --exclude, or --include".into(),
        ));
    }

    storage.set_config(&config)?;

    Ok(CommandOutput {
        data: serde_json::json!({
            "account_id": account_id,
            "changes": changes,
            "message": format!("Updated {account_id}: {}", changes.join(", "))
        }),
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Message,
    })
}

fn parse_category(s: &str) -> simplefin::Result<simplefin::AccountCategory> {
    match s.to_lowercase().replace(' ', "_").as_str() {
        "cash" => Ok(simplefin::AccountCategory::Cash),
        "investments" => Ok(simplefin::AccountCategory::Investments),
        "other_assets" | "otherassets" => Ok(simplefin::AccountCategory::OtherAssets),
        "credit_cards" | "creditcards" => Ok(simplefin::AccountCategory::CreditCards),
        "loans" => Ok(simplefin::AccountCategory::Loans),
        _ => Err(SimplefinError::InvalidArgument(format!(
            "unknown category \"{s}\". Valid: cash, investments, other_assets, credit_cards, loans"
        ))),
    }
}

fn handle_status(storage_path: &str) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;
    let now = chrono::Utc::now().timestamp();
    let status = simplefin::compute_status(&storage, now)?;

    Ok(CommandOutput {
        data: serde_json::to_value(&status).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize status".into(),
            source: Some(Box::new(e)),
        })?,
        storage_path: None, // status already includes warnings, skip envelope duplication
        kind: CommandKind::Status,
    })
}

fn handle_cleanup(storage_path: &str, remove: bool) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;
    let orphans = storage.find_orphaned_data()?;

    if remove && !orphans.is_empty() {
        storage.remove_orphaned_data(&orphans)?;
    }

    Ok(CommandOutput {
        data: serde_json::json!({
            "orphaned_files": orphans.len(),
            "removed": remove && !orphans.is_empty(),
            "orphans": serde_json::to_value(&orphans).unwrap_or_default(),
        }),
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Cleanup,
    })
}

fn handle_spending_rules(
    storage_path: &str,
    _list: bool,
    add: Option<&str>,
    category: Option<&str>,
    remove: Option<&str>,
    reset: bool,
) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;

    if reset {
        let defaults = simplefin::default_spending_patterns();
        storage.set_spending_patterns(&defaults)?;
        return Ok(CommandOutput {
            data: serde_json::json!({
                "message": format!("Reset spending patterns to defaults ({} rules)", defaults.len()),
                "rule_count": defaults.len(),
            }),
            storage_path: Some(storage_path.to_string()),
            kind: CommandKind::Message,
        });
    }

    if let Some(pattern) = add {
        let cat_str = category.ok_or_else(|| {
            SimplefinError::InvalidArgument("--category is required when adding a pattern".into())
        })?;
        let cat = parse_spending_category(cat_str)?;
        let mut patterns = storage.get_spending_patterns()?;
        patterns.insert(
            0,
            simplefin::SpendingRule {
                pattern: pattern.to_string(),
                category: cat.clone(),
            },
        );
        storage.set_spending_patterns(&patterns)?;
        return Ok(CommandOutput {
            data: serde_json::json!({
                "message": format!("Added pattern \"{pattern}\" -> {cat}"),
                "rule_count": patterns.len(),
            }),
            storage_path: Some(storage_path.to_string()),
            kind: CommandKind::Message,
        });
    }

    if let Some(pattern_to_remove) = remove {
        let mut patterns = storage.get_spending_patterns()?;
        let before = patterns.len();
        patterns.retain(|r| !r.pattern.to_lowercase().contains(&pattern_to_remove.to_lowercase()));
        let removed = before - patterns.len();
        storage.set_spending_patterns(&patterns)?;
        return Ok(CommandOutput {
            data: serde_json::json!({
                "message": format!("Removed {removed} pattern(s) matching \"{pattern_to_remove}\""),
                "removed": removed,
                "rule_count": patterns.len(),
            }),
            storage_path: Some(storage_path.to_string()),
            kind: CommandKind::Message,
        });
    }

    // Default: list mode
    let patterns = storage.get_spending_patterns()?;
    let config = storage.get_config()?;
    let data = serde_json::json!({
        "patterns": patterns.iter().map(|r| serde_json::json!({
            "pattern": r.pattern,
            "category": r.category,
        })).collect::<Vec<_>>(),
        "pattern_count": patterns.len(),
        "user_rules": config.spending_rules.iter().map(|r| serde_json::json!({
            "pattern": r.pattern,
            "category": r.category,
        })).collect::<Vec<_>>(),
        "user_rule_count": config.spending_rules.len(),
    });

    Ok(CommandOutput {
        data,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Message,
    })
}

fn parse_spending_category(s: &str) -> simplefin::Result<simplefin::SpendingCategory> {
    match s.to_lowercase().replace(' ', "_").as_str() {
        "restaurants" => Ok(simplefin::SpendingCategory::Restaurants),
        "groceries" => Ok(simplefin::SpendingCategory::Groceries),
        "utilities" => Ok(simplefin::SpendingCategory::Utilities),
        "transportation" => Ok(simplefin::SpendingCategory::Transportation),
        "shopping" => Ok(simplefin::SpendingCategory::Shopping),
        "entertainment" => Ok(simplefin::SpendingCategory::Entertainment),
        "healthcare" => Ok(simplefin::SpendingCategory::Healthcare),
        "housing" => Ok(simplefin::SpendingCategory::Housing),
        "insurance" => Ok(simplefin::SpendingCategory::Insurance),
        "subscriptions" => Ok(simplefin::SpendingCategory::Subscriptions),
        "education" => Ok(simplefin::SpendingCategory::Education),
        "personal_care" | "personalcare" => Ok(simplefin::SpendingCategory::PersonalCare),
        "pets" => Ok(simplefin::SpendingCategory::Pets),
        "income" => Ok(simplefin::SpendingCategory::Income),
        "transfer" => Ok(simplefin::SpendingCategory::Transfer),
        "other" => Ok(simplefin::SpendingCategory::Other),
        _ => Err(SimplefinError::InvalidArgument(format!(
            "unknown spending category \"{s}\". Valid: restaurants, groceries, utilities, transportation, shopping, entertainment, healthcare, housing, insurance, subscriptions, education, personal_care, pets, income, transfer, other"
        ))),
    }
}

fn handle_recurring(
    storage_path: &str,
    min_occurrences: usize,
) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;
    let config = storage.get_config()?;

    let transactions = storage.get_transactions(&TransactionFilter {
        include_pending: Some(false),
        ..Default::default()
    })?;

    let patterns = storage.get_spending_patterns()?;
    let mut rules = config.spending_rules.clone();
    rules.extend(patterns);

    let summary = simplefin::detect_recurring(&transactions, &rules, min_occurrences);

    Ok(CommandOutput {
        data: serde_json::to_value(&summary).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize recurring output".into(),
            source: Some(Box::new(e)),
        })?,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Recurring,
    })
}

fn handle_trends(storage_path: &str, months: usize) -> simplefin::Result<CommandOutput> {
    let storage = JsonStorage::open(storage_path)?;
    let config = storage.get_config()?;

    let transactions = storage.get_transactions(&TransactionFilter {
        include_pending: Some(false),
        ..Default::default()
    })?;

    let patterns = storage.get_spending_patterns()?;
    let mut rules = config.spending_rules.clone();
    rules.extend(patterns);

    let summary = simplefin::compute_trends(&transactions, &rules, months);

    Ok(CommandOutput {
        data: serde_json::to_value(&summary).map_err(|e| SimplefinError::Storage {
            message: "failed to serialize trends output".into(),
            source: Some(Box::new(e)),
        })?,
        storage_path: Some(storage_path.to_string()),
        kind: CommandKind::Trends,
    })
}

fn handle_schema(output_type: &str) -> simplefin::Result<CommandOutput> {
    use schemars::schema_for;

    let schema = match output_type {
        "summary" => schema_for!(simplefin::NetWorthSummary),
        "accounts" => schema_for!(simplefin::UnifiedAccount),
        "transactions" => schema_for!(simplefin::TransactionWithContext),
        "query" => {
            // Query returns both accounts and transactions
            serde_json::from_value(serde_json::json!({
                "type": "object",
                "properties": {
                    "accounts": schema_for!(Vec<simplefin::UnifiedAccount>),
                    "transactions": schema_for!(Vec<simplefin::TransactionWithContext>),
                }
            }))
            .unwrap()
        }
        "spending" => schema_for!(simplefin::SpendingSummary),
        "status" => schema_for!(simplefin::StorageStatus),
        "stale" => schema_for!(Vec<simplefin::StaleAccount>),
        "warnings" => schema_for!(simplefin::WarningRecord),
        "history" => schema_for!(Vec<simplefin::NetWorthTimePoint>),
        "changes" => schema_for!(Vec<simplefin::BalanceChange>),
        "recurring" => schema_for!(simplefin::RecurringSummary),
        "trends" => schema_for!(simplefin::TrendsSummary),
        other => {
            return Err(SimplefinError::InvalidArgument(format!(
                "unknown schema type '{other}'. Valid types: summary, query, accounts, transactions, spending, status, stale, warnings, history, changes"
            )));
        }
    };

    let data = serde_json::to_value(schema)
        .map_err(|e| SimplefinError::DataFormat {
            message: format!("failed to serialize schema: {e}"),
            source: None,
        })?;

    Ok(CommandOutput {
        data,
        storage_path: None,
        kind: CommandKind::Schema,
    })
}
