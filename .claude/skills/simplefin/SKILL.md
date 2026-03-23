---
name: simplefin
description: >
  Collect and analyze personal financial data from SimpleFIN Bridge. Use this skill whenever
  the user asks about their finances, account balances, transactions, spending, savings,
  net worth, or any financial question that could be answered from bank/brokerage data.
  Also use when the user wants to pull fresh data, refresh accounts, sync their financial
  information, add manual account balances, or view net worth trends over time.
  Trigger on phrases like "how much", "what's my balance", "spending on",
  "transactions", "collect my data", "refresh finances", "net worth", "pull latest",
  "financial summary", "add balance", "manual account", "trends", or any question
  about money, accounts, or transactions.
---

# SimpleFIN Financial Data Skill

This skill lets you collect financial data from the SimpleFIN Bridge API, add manual
account balances for accounts not in SimpleFIN, track net worth over time, and answer
questions about all of it.

## How it works

The project at the workspace root has a CLI tool (`simplefin`) with these subcommands:
1. **Collect** (`l`) -- pulls account balances and transactions from the SimpleFIN API into local JSON storage, records balance snapshots, and persists any warnings/anomalies
2. **Add-balance** (`a`) -- adds or updates a manual account balance (for accounts not connected to SimpleFIN)
3. **Status** (`st`) -- quick snapshot of storage state: last collection time, account counts, stale accounts, warnings
4. **Query** (`q`) -- reads local storage and outputs filtered JSON with unified accounts (SimpleFIN + manual merged), transactions, and balance history
5. **Summary** (`s`) -- outputs categorized net worth (Cash, Investments, Other Assets, Credit Cards, Loans) with changes since last collection and optional history
6. **Spending** (`p`) -- categorizes transactions into spending categories with totals
7. **Stale** (`t`) -- lists manual accounts whose balances need updating
8. **Configure** (`cfg`) -- view and modify account classifications, display names, and exclusions
9. **Spending-rules** (`sr`) -- manage spending classification patterns stored in the data directory
10. **Recurring** (`r`) -- detect recurring expenses from transaction patterns
11. **Trends** (`tr`) -- analyze spending trends over time (month-over-month by category)
12. **Schema** -- outputs JSON Schema for any output type (for programmatic consumers)
13. **Cleanup** -- finds and removes orphaned data files

The CLI binary is built from this workspace. Build it if needed with `cargo build -p simplefin-cli`.

## Output format

All commands output a structured **envelope** by default:

```json
{
  "data": { ... },
  "warnings": ["WARNING: Account balance dropped to $0 (was 1234.56)", ...],
  "errors": []
}
```

- `data` contains the command-specific output
- `warnings` are populated from persisted collection warnings (anomalies and bridge messages)
- `errors` contains error messages on failure (with non-zero exit code)

Use `--raw` to get bare JSON (no envelope) when you only need the data.

Use `--format text` for human-readable table output instead of JSON.

## Storage location

Data is stored **outside** the repo (this is a public repo -- no financial data in git).
The `SIMPLEFIN_DATA` environment variable must point to the storage directory. Set it
in `.env` at the workspace root alongside `SIMPLEFIN_ACCESS_URL`.

All `--storage` / `-s` flags below use `"$SIMPLEFIN_DATA"` as the path.

## Step 1: Check storage status

Before answering any financial question, check the current state:

```bash
cargo run -p simplefin-cli -- status --storage "$SIMPLEFIN_DATA"
```

This returns:
- When the last collection happened (and how long ago)
- Number of SimpleFIN and manual accounts
- Which manual accounts have stale balances
- Any warnings/anomalies from the most recent collection

If `last_collection_time` is null or stale, or the user explicitly asks to refresh, run collect.

## Step 2: Collect fresh data (if needed)

```bash
cargo run -p simplefin-cli -- collect --storage "$SIMPLEFIN_DATA" --verbose
```

This requires `SIMPLEFIN_ACCESS_URL` to be set in `.env` at the workspace root.
Collection is idempotent -- safe to run multiple times. It fetches all accounts and
transactions incrementally, records balance snapshots, and persists any warnings.

Warnings and anomalies are now persisted and included in the envelope automatically.
Check the `warnings` array in the response for:
- Balance anomalies (dropped to zero, large changes, disappeared/new accounts)
- Bridge messages (authentication issues, sync problems, date range caps)

**Always report warnings prominently to the user.** Silent failures are unacceptable.

If collection fails with a credentials error, tell the user they need to set up their
`.env` file with `SIMPLEFIN_ACCESS_URL=<their access URL>`.

## Step 3: Check for stale manual accounts

Some accounts aren't connected through SimpleFIN and require manual balance entry.
Each manual account has a `refresh_days` setting. Check which ones need updating:

```bash
cargo run -p simplefin-cli -- stale --storage "$SIMPLEFIN_DATA"
```

If any accounts are stale, ask the user for current balances, then update them:

```bash
cargo run -p simplefin-cli -- add-balance --storage "$SIMPLEFIN_DATA" \
  --name "My 401k" --org "My Provider" --balance 25000.00 --refresh-days 1
```

## Step 4: Query and analyze

### Net worth summary

```bash
# Category totals only
cargo run -p simplefin-cli -- summary --storage "$SIMPLEFIN_DATA"

# With per-account breakdown within each category
cargo run -p simplefin-cli -- summary --storage "$SIMPLEFIN_DATA" --detail

# With net worth history over last N collection timestamps
cargo run -p simplefin-cli -- summary --storage "$SIMPLEFIN_DATA" --history 10

# Combine detail and history
cargo run -p simplefin-cli -- summary --storage "$SIMPLEFIN_DATA" --detail --history 10

# Human-readable table format
cargo run -p simplefin-cli -- --format text summary --storage "$SIMPLEFIN_DATA" --detail
```

The output includes:
- `net_worth` -- categorized totals, total_assets, total_liabilities, net_worth
- `changes` -- per-account balance deltas since the previous collection
- `history` (when `--history N` is used) -- net worth at each of the last N collection timestamps

### Raw data query

```bash
# All data
cargo run -p simplefin-cli -- query --storage "$SIMPLEFIN_DATA"

# Filter by organization
cargo run -p simplefin-cli -- query --storage "$SIMPLEFIN_DATA" --org "investor.vanguard.com"

# Filter by account
cargo run -p simplefin-cli -- query --storage "$SIMPLEFIN_DATA" --account "ACC-ID-HERE"

# Filter by date range
cargo run -p simplefin-cli -- query --storage "$SIMPLEFIN_DATA" --start-date 2024-01-01 --end-date 2024-12-31

# Include pending transactions
cargo run -p simplefin-cli -- query --storage "$SIMPLEFIN_DATA" --pending
```

### Spending analysis

```bash
# All time
cargo run -p simplefin-cli -- spending --storage "$SIMPLEFIN_DATA"

# Specific date range
cargo run -p simplefin-cli -- spending --storage "$SIMPLEFIN_DATA" \
  --start-date 2024-01-01 --end-date 2024-02-01
```

The spending output includes an `unclassified` array with objects containing both
`description` and `amount` for transactions that fell into "Other". **Always show
both the description and amount** when asking the user about unclassified transactions
-- the amount helps them identify what the charge was. Then teach the classifier:

```bash
# Add a pattern for unclassified transactions
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" \
  --add "some merchant name" --category restaurants
```

This saves the pattern to `spending_patterns.json` in the data directory so
future runs classify it correctly. Over time, the "Other" bucket shrinks as
the user teaches the system about their specific merchants.

**Categories are data-driven** — any string is a valid category. To create
a new category (e.g., "donations"), just add a rule with that category name:

```bash
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" \
  --add "red cross|salvation army|united way" --category donations
```

No code changes needed. The new category appears in spending output automatically.

### Manage spending patterns

Spending classification uses data-driven patterns, not hardcoded rules. Patterns
are stored in `spending_patterns.json` and editable:

```bash
# List all patterns
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" --list

# Add a new pattern (highest priority)
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" \
  --add "local coffee shop" --category restaurants

# Remove patterns matching a substring
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" \
  --remove "chipotle"

# Reset to defaults
cargo run -p simplefin-cli -- spending-rules --storage "$SIMPLEFIN_DATA" --reset
```

### Recurring expense detection

Detect subscriptions and recurring charges:

```bash
# Default: 2+ occurrences
cargo run -p simplefin-cli -- recurring --storage "$SIMPLEFIN_DATA"

# Require 3+ occurrences
cargo run -p simplefin-cli -- recurring --storage "$SIMPLEFIN_DATA" --min-occurrences 3

# Human-readable
cargo run -p simplefin-cli -- --format text recurring --storage "$SIMPLEFIN_DATA"
```

Output includes merchant name, average amount, frequency (weekly/monthly/quarterly/annual),
occurrence count, and estimated monthly total across all recurring expenses.

### Spending trends

Analyze month-over-month spending by category:

```bash
# Last 6 months (default)
cargo run -p simplefin-cli -- trends --storage "$SIMPLEFIN_DATA"

# Last 12 months
cargo run -p simplefin-cli -- trends --storage "$SIMPLEFIN_DATA" --months 12

# Human-readable
cargo run -p simplefin-cli -- --format text trends --storage "$SIMPLEFIN_DATA"
```

Shows per-category monthly averages and trend direction (up/down/stable).

### Data cleanup

```bash
# Dry run -- show orphaned files
cargo run -p simplefin-cli -- cleanup --storage "$SIMPLEFIN_DATA"

# Remove orphaned files
cargo run -p simplefin-cli -- cleanup --storage "$SIMPLEFIN_DATA" --remove
```

### JSON Schema for output types

```bash
# Get schema for any output type
cargo run -p simplefin-cli -- schema summary
cargo run -p simplefin-cli -- schema status
cargo run -p simplefin-cli -- schema query
cargo run -p simplefin-cli -- schema spending
cargo run -p simplefin-cli -- schema stale
cargo run -p simplefin-cli -- schema warnings
cargo run -p simplefin-cli -- schema history
cargo run -p simplefin-cli -- schema changes
cargo run -p simplefin-cli -- schema accounts
cargo run -p simplefin-cli -- schema transactions
cargo run -p simplefin-cli -- schema recurring
cargo run -p simplefin-cli -- schema trends
```

## Step 5: Account configuration

Review and adjust how accounts are classified, named, and included:

```bash
# List all accounts with their classifications and flags
cargo run -p simplefin-cli -- configure --storage "$SIMPLEFIN_DATA" --list

# Human-readable format (flags low-confidence classifications with [? review])
cargo run -p simplefin-cli -- --format text configure --storage "$SIMPLEFIN_DATA" --list

# Set a display name
cargo run -p simplefin-cli -- configure --storage "$SIMPLEFIN_DATA" \
  --set "ACCOUNT-ID" --name "My Friendly Name"

# Override classification
cargo run -p simplefin-cli -- configure --storage "$SIMPLEFIN_DATA" \
  --set "ACCOUNT-ID" --category investments

# Exclude from net worth
cargo run -p simplefin-cli -- configure --storage "$SIMPLEFIN_DATA" \
  --set "ACCOUNT-ID" --exclude

# Re-include in net worth
cargo run -p simplefin-cli -- configure --storage "$SIMPLEFIN_DATA" \
  --set "ACCOUNT-ID" --include
```

The `--list` output shows for each account:
- Heuristic classification (what the algorithm guessed)
- Effective classification (after overrides/rules)
- Whether it's overridden or excluded
- A `confident` flag -- when false, the heuristic may be wrong and needs review

Valid categories: `cash`, `investments`, `other_assets`, `credit_cards`, `loans`

## Account classification

Accounts are grouped into five categories:

| Category | What goes here |
|----------|---------------|
| **Cash** | Checking and savings accounts |
| **Investments** | Brokerage, IRA, 401(k), Roth IRA accounts |
| **Other Assets** | Real estate, vehicles, HSA |
| **Credit Cards** | Credit card accounts -- balances are liabilities |
| **Loans** | Mortgage and other loans |

Classification priority: account ID override > classification rules > heuristic classifier.

After first collection, run `configure --list` with `--format text` to review classifications.
Accounts flagged with `[? review]` have low-confidence heuristic matches and may need
manual override via `configure --set`.

## Important notes

- Financial amounts are precise decimals, not floats -- trust them as-is
- Balance history accumulates over time -- each `collect` or `add-balance` adds a new snapshot
- Some accounts (like brokerage) may not have transactions but will have balances
- The `currency` field may be empty -- assume USD in that case
- Investment account balances reflect total portfolio value
- Always check `status` before answering -- if data is stale, collect first
- Warnings in the envelope are automatically populated from persisted collection data
- **Never suppress or ignore warnings** -- they are the only way to know if data is incomplete

## Presenting financial data

- Format currency amounts with commas and two decimal places (e.g., $1,234.56)
- Convert epoch timestamps to human-readable dates
- Group accounts by institution for readability
- Negative amounts are debits/spending; positive amounts are credits/income
- Include manual accounts in net worth calculations
- If balances seem stale (balance-date is old), mention when they were last updated
- Use `--format text` for the user when they want a quick overview
- Use JSON (default) when you need to process or analyze the data programmatically
