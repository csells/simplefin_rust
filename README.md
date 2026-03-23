# simplefin_rust

After losing Mint and Personal Capital, I decided that a stable place to track
my accounts and transactions is in order. So I found
[SimpleFIN](https://www.simplefin.org/), which allows you to put the
credentials in for your various financial institutions and tracks their balance
and transactions, exposing them with a simple, secure REST API for only $15/year.

This project is a Rust client library + CLI that collects financial data from
SimpleFIN, tracks balances over time (including manual accounts for things
SimpleFIN can't reach), and computes a categorized net worth summary. Ported
from the [Dart implementation](https://github.com/csells/simplefin_dart).

## Project Structure

This is a Cargo workspace with two crates:

- **`simplefin/`** -- Library crate. API client, data models, storage, and
  analysis (account classification, net worth computation, change detection).
- **`simplefin-cli/`** -- Binary crate (`simplefin` executable). CLI for setup,
  data collection, manual balance entry, querying, and net worth summaries.

## Getting Started

### Prerequisites

- Rust 2024 edition (1.85+)
- A [SimpleFIN Bridge](https://beta-bridge.simplefin.org/auth/login) account ($15/year)

### Setup

```bash
# Clone and build
git clone https://github.com/csells/simplefin_rust.git
cd simplefin_rust
cargo build

# Get your SimpleFIN access URL
# 1. Sign in at https://beta-bridge.simplefin.org/auth/login
# 2. Create a setup token at https://bridge.simplefin.org/simplefin/create
# 3. Claim it:
cargo run -p simplefin-cli -- claim <SETUP_TOKEN>

# 4. Save the output to .env:
echo 'SIMPLEFIN_ACCESS_URL=https://...' > .env
```

## CLI Usage

```
$ simplefin --help
SimpleFIN Bridge CLI client

Commands:
  claim           Exchange a setup token for an access URL
  info            Query the bridge for supported protocol versions
  collect         Collect all financial data idempotently into local storage
  add-balance     Add or update a manual account balance (for accounts not in SimpleFIN)
  status          Show storage status: last collection, account counts, stale accounts, warnings
  stale           Show manual accounts whose balances are stale and need updating
  query           Query collected data as JSON
  summary         Show categorized net worth summary with changes and optional history
  spending        Analyze spending by category over a date range
  spending-rules  Manage spending classification patterns stored in the data directory
  recurring       Detect recurring expenses from transaction patterns
  trends          Analyze spending trends over time (month-over-month by category)
  configure       View and modify account classifications, display names, exclusions
  schema          Print JSON Schema for a given output type
  cleanup         Find and optionally remove orphaned data files

Global flags:
  --format json|text   Output format (default: json)
  --raw                Output bare JSON without envelope wrapper
```

### Output format

All commands output a structured envelope by default:

```json
{
  "data": { ... },
  "warnings": ["WARNING: Account balance dropped to $0 (was 1234.56)"],
  "errors": []
}
```

Use `--raw` for bare JSON (no envelope). Use `--format text` for human-readable
table output.

### Collect financial data

Fetches all accounts and transactions from SimpleFIN, stores them locally. Safe
to run repeatedly -- transactions are deduped by ID, balance snapshots are
deduped when unchanged. Warnings and anomalies are persisted and included in
the envelope for subsequent commands.

```bash
simplefin collect --storage ./data --verbose
```

Bridge messages (auth issues, date caps) are printed to stderr so you'll always
know if an account has problems.

### Check storage status

Quick snapshot of the current state -- useful before other operations:

```bash
simplefin status --storage ./data
```

Returns last collection time, account counts, stale manual accounts, and any
warnings from the most recent collection.

### Add manual account balances

For accounts SimpleFIN can't reach (401k providers, HSA, real estate, vehicles):

```bash
simplefin add-balance -s ./data -n "My 401k" -o "My Provider" -b 25000.00 -r 1
simplefin add-balance -s ./data -n "Home" -o Manual -b 400000 -r 30
```

The `-r` flag sets how often the account should be refreshed (in days). Daily
accounts (401k, HSA) use `-r 1`; slow-moving assets (home, car) use `-r 30`.
Default is 1 day.

Each call records a timestamped balance snapshot for trend tracking. Run again
with a new balance to add another data point.

### Check for stale balances

See which manual accounts need a balance update:

```bash
simplefin stale -s ./data
```

Returns JSON with stale accounts (name, days since last update, refresh frequency)
or a message that everything is up to date.

### Query collected data

Returns unified JSON with all accounts (SimpleFIN + manual merged), transactions,
and balance history:

```bash
# All data
simplefin query -s ./data

# Filter by organization
simplefin query -s ./data --org "investor.vanguard.com"

# Filter by date range
simplefin query -s ./data --start-date 2024-01-01 --end-date 2024-12-31

# Include pending transactions
simplefin query -s ./data --pending
```

### Net worth summary

Categorized net worth with changes since the last collection, computed in Rust:

```bash
simplefin summary -s ./data

# Include per-account breakdown within each category
simplefin summary -s ./data --detail

# Show net worth over the last 10 collection timestamps
simplefin summary -s ./data --history 10

# Human-readable table
simplefin --format text summary -s ./data --detail
```

### Configure accounts

View and modify account classifications, display names, and exclusions:

```bash
# List all accounts with their classifications
simplefin configure -s ./data --list

# Human-readable with confidence flags
simplefin --format text configure -s ./data --list

# Set a display name
simplefin configure -s ./data --set "ACCOUNT-ID" --name "My Friendly Name"

# Override classification
simplefin configure -s ./data --set "ACCOUNT-ID" --category investments

# Exclude/include from net worth
simplefin configure -s ./data --set "ACCOUNT-ID" --exclude
simplefin configure -s ./data --set "ACCOUNT-ID" --include
```

Valid categories: `cash`, `investments`, `other_assets`, `credit_cards`, `loans`.

Configuration is stored in `config.json` in the data directory (not in the
repo). It also supports `excluded_account_patterns`, `classification_rules`,
and `spending_rules` for bulk matching -- see `schema` command for details.

### JSON Schema

Get the JSON Schema for any output type:

```bash
simplefin schema summary
simplefin schema status
simplefin schema spending
simplefin schema accounts
simplefin schema transactions
```

### Spending analysis

Analyze spending by category over a date range:

```bash
# All time
simplefin spending -s ./data

# Specific date range
simplefin spending -s ./data --start-date 2024-01-01 --end-date 2024-02-01
```

Transactions are classified into categories (Restaurants, Groceries, Utilities,
Transportation, Shopping, Entertainment, Healthcare, Housing, Insurance,
Subscriptions, Education, Personal Care, Pets, Income, Transfer, Other) using
data-driven keyword patterns stored in `spending_patterns.json` in the data
directory. Patterns are seeded with defaults on first use and customizable via
the `spending-rules` command. Custom rules in `config.json` take priority.

### Manage spending patterns

Spending classification patterns are stored in `spending_patterns.json` in the
data directory, not hardcoded in the binary. On first use, the file is seeded
with sensible defaults. Customize to match your transactions:

```bash
# List all current patterns
simplefin spending-rules -s ./data --list

# Add a new pattern (placed at highest priority)
simplefin spending-rules -s ./data --add "local coffee" --category restaurants

# Remove patterns matching a substring
simplefin spending-rules -s ./data --remove "chipotle"

# Reset to defaults (overwrites all customizations)
simplefin spending-rules -s ./data --reset
```

User rules in `config.json` (via `spending_rules`) take priority over patterns
in `spending_patterns.json`. Unclassified transactions (category "Other") are
reported in the spending output so you know what to teach.

### Recurring expense detection

Detect subscriptions and recurring charges from transaction patterns:

```bash
# Detect recurring expenses (default: 2+ occurrences)
simplefin recurring -s ./data

# Require at least 3 occurrences
simplefin recurring -s ./data --min-occurrences 3

# Human-readable table
simplefin --format text recurring -s ./data
```

The detector normalizes merchant names (stripping POS prefixes and trailing IDs),
groups by merchant, and checks for regular intervals. Output includes average
amount, frequency, estimated next occurrence, and estimated monthly total.

### Spending trends

Analyze spending trends over time by category:

```bash
# Last 6 months (default)
simplefin trends -s ./data

# Last 12 months
simplefin trends -s ./data --months 12

# Human-readable table
simplefin --format text trends -s ./data
```

Shows month-over-month spending by category, monthly averages, and trend
direction (up, down, stable) based on recent vs earlier months.

### Cleanup orphaned data

Find and remove data files for accounts that no longer exist:

```bash
# Dry run — show what would be removed
simplefin cleanup -s ./data

# Actually remove orphaned files
simplefin cleanup -s ./data --remove
```

### Anomaly detection

During `collect`, the CLI automatically compares incoming account balances
against the previously stored values and warns about:

- Balances that dropped to $0
- Large balance changes (>20%)
- Accounts that disappeared
- New accounts that appeared

Warnings are persisted to `warnings.json` and automatically included in the
envelope output of subsequent commands.

## Library API

The `simplefin` crate can be used directly for programmatic access:

```rust
use simplefin::{AccessCredentials, AccessClient, AccountQueryParams};
use asupersync::Cx;

async fn fetch_accounts(cx: &Cx, access_url: &str) -> simplefin::Result<()> {
    let credentials = AccessCredentials::parse(access_url)?;
    let client = AccessClient::new(credentials, None);

    let accounts = client.get_accounts(cx, &AccountQueryParams::default()).await?;

    for msg in &accounts.server_messages {
        eprintln!("Bridge: {msg}");
    }

    for account in &accounts.accounts {
        println!("{}: {}", account.name, account.balance);
    }

    Ok(())
}
```

### Analysis

The library includes built-in financial analysis:

```rust
use simplefin::{classify_account, compute_net_worth, unify_accounts, AccountCategory, DataConfig};

// Classify accounts into categories
let category = classify_account("Brokerage Account", "Vanguard");
assert_eq!(category, AccountCategory::Investments);

// Compute categorized net worth from unified accounts
let unified = unify_accounts(&simplefin_accounts, &manual_accounts, &balance_history);
let config = DataConfig::default(); // or load from config.json
let summary = compute_net_worth(&unified, &config);
println!("Net worth: {}", summary.net_worth);
```

### Storage

Data is stored as JSON files with atomic writes (write-to-tmp + rename):

```
{storage_dir}/
  organizations.json         -- institution metadata
  accounts.json              -- account balances (transactions stripped)
  transactions/{account}.json -- per-account transaction history
  manual_accounts.json       -- manually-tracked accounts (with refresh_days)
  balance_history/{account}.json -- balance snapshots over time
  config.json                -- user-specific settings (exclusions, overrides)
  spending_patterns.json     -- spending classification patterns (seeded from defaults)
  warnings.json              -- anomalies and bridge messages from last collection
  state.json                 -- incremental collection bookmarks
```

## Server Messages

The SimpleFIN bridge may return informational messages alongside account data.
These communicate auth failures, date range caps, or sync problems. The CLI
prints them to stderr:

```
Bridge: Authentication failed for account XYZ
Bridge (Savings): Requested date range exceeds limit of 90 days and was capped.
```

Always check these -- they explain why accounts may have missing data.

## Claude Code Skill

This project includes a [Claude Code](https://claude.ai/claude-code) skill at
`.claude/skills/simplefin/SKILL.md` that lets you interact with your financial
data using natural language. Just ask questions like:

- "What's my net worth?"
- "How much did I spend last month?"
- "Pull fresh data from SimpleFIN"
- "What's my balance at Vanguard?"
- "Add my 401k balance: $22,315"

The skill auto-triggers on financial questions, collects fresh data when needed,
prompts for manual account balances when they're stale, and uses the `summary`
command for net worth calculations so the math happens in Rust rather than in
the LLM's head.

## Development

```bash
cargo build          # Build library + CLI
cargo clippy         # Lint (zero warnings required)
cargo test           # Run all tests (277 currently)
```

## Resources

- [SimpleFIN Protocol Specification](https://www.simplefin.org/protocol.html)
- [SimpleFIN Bridge developer docs](https://beta-bridge.simplefin.org/info/developers)
- [Dart implementation](https://github.com/csells/simplefin_dart) (original port source)
