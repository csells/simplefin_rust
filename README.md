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
  claim        Exchange a setup token for an access URL
  info         Query the bridge for supported protocol versions
  collect      Collect all financial data idempotently into local storage
  add-balance  Add or update a manual account balance (for accounts not in SimpleFIN)
  stale        Show manual accounts whose balances are stale and need updating
  query        Query collected data as JSON
  summary      Show categorized net worth summary with changes since last collection
  spending     Analyze spending by category over a date range
  cleanup      Find and optionally remove orphaned data files
```

### Collect financial data

Fetches all accounts and transactions from SimpleFIN, stores them locally. Safe
to run repeatedly -- transactions are deduped by ID, balance snapshots are
deduped when unchanged.

```bash
simplefin collect --storage ./data --verbose
```

Output:
```
  Savings (1234): 12 new, 0 existing
  Checking (5678): 45 new, 23 existing
Collected 57 new transactions across 8 accounts (23 duplicates skipped)
```

Bridge messages (auth issues, date caps) are printed to stderr so you'll always
know if an account has problems.

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
```

Output:
```json
{
  "net_worth": {
    "categories": [
      { "category": "cash", "label": "Cash", "total": "5000.00" },
      { "category": "investments", "label": "Investments", "total": "50000.00" },
      { "category": "other_assets", "label": "Other Assets", "total": "10000.00" },
      { "category": "credit_cards", "label": "Credit Cards", "total": "-1500.00" },
      { "category": "loans", "label": "Loans", "total": "-150000.00" }
    ],
    "total_assets": "65000.00",
    "total_liabilities": "-151500.00",
    "net_worth": "-86500.00"
  },
  "changes": [
    {
      "account_name": "Checking (1234)",
      "previous_balance": "4500.00",
      "current_balance": "5000.00",
      "change": "500.00",
      "category": "cash"
    }
  ]
}
```

### Configuration

User-specific settings are stored in `config.json` in the data directory (not
in the repo):

```json
{
  "excluded_account_patterns": ["Duplicate Account"],
  "classification_overrides": {
    "some-account-id": "cash"
  }
}
```

- **`excluded_account_patterns`** -- account names matching these patterns
  (case-insensitive) are excluded from net worth and change calculations.
  Useful for authorized-user duplicates.
- **`classification_overrides`** -- maps account IDs to specific categories
  (`cash`, `investments`, `other_assets`, `credit_cards`, `loans`),
  overriding the heuristic classifier.
- **`classification_rules`** -- ordered list of pattern-matching rules checked
  before the heuristic classifier. Each rule has a `pattern` (substring),
  `field` (`name` or `org`), and target `category`. First match wins.
- **`display_names`** -- maps account IDs to friendly display names shown in
  summary output instead of the raw account name.
- **`spending_rules`** -- custom rules for classifying transactions into
  spending categories, overriding the built-in keyword patterns.

### Spending analysis

Analyze spending by category over a date range:

```bash
# All time
simplefin spending -s ./data

# Specific date range
simplefin spending -s ./data --start-date 2024-01-01 --end-date 2024-02-01
```

Transactions are classified into categories (Restaurants, Groceries, Utilities,
Transportation, Shopping, Entertainment, Healthcare, Income, Transfer, Other)
using built-in keyword patterns and optional custom rules in `config.json`.

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

Warnings are printed to stderr alongside bridge messages.

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
cargo test           # Run all tests (186 currently)
```

## Resources

- [SimpleFIN Protocol Specification](https://www.simplefin.org/protocol.html)
- [SimpleFIN Bridge developer docs](https://beta-bridge.simplefin.org/info/developers)
- [Dart implementation](https://github.com/csells/simplefin_dart) (original port source)
