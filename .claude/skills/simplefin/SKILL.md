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
1. **Collect** (`l`) — pulls account balances and transactions from the SimpleFIN API into local JSON storage, and records a balance snapshot for every account (deduped — skips if balance unchanged)
2. **Add-balance** (`a`) — adds or updates a manual account balance (for accounts not connected to SimpleFIN)
3. **Query** (`q`) — reads local storage and outputs filtered JSON with unified accounts (SimpleFIN + manual merged), transactions, and balance history
4. **Summary** (`s`) — outputs categorized net worth (Cash, Investments, Other Assets, Credit Cards, Loans) with changes since last collection

The CLI binary is built from this workspace. Build it if needed with `cargo build -p simplefin-cli`.

## Storage location

Data is stored on Google Drive, **outside** the repo (this is a public repo — no financial
data in git):

```
SIMPLEFIN_DATA="$HOME/Library/CloudStorage/GoogleDrive-csells@sellsbrothers.com/My Drive/data/finances/simplefin-data"
```

All `--storage` / `-s` flags below use `"$SIMPLEFIN_DATA"` as the path.

## Step 1: Ensure data is fresh

Before answering any financial question, check if data exists and is reasonably recent:

```bash
ls "$SIMPLEFIN_DATA/accounts.json" 2>/dev/null
```

If no data exists, or the user explicitly asks to refresh/collect/pull, run:

```bash
cargo run -p simplefin-cli -- collect --storage "$SIMPLEFIN_DATA" --verbose 2>&1
```

This requires `SIMPLEFIN_ACCESS_URL` to be set in `.env` at the workspace root.
Collection is idempotent — safe to run multiple times. It fetches all accounts and
transactions incrementally, and records a balance snapshot for every account on each run.

If collection fails with a credentials error, tell the user they need to set up their
`.env` file with `SIMPLEFIN_ACCESS_URL=<their access URL>`.

### IMPORTANT: Surface all bridge messages

The SimpleFIN bridge prints warnings and errors to stderr during collection. These
are lines that start with `Bridge:` or `Bridge (account):`. **You MUST read every
bridge message and report them clearly to the user.** These messages indicate real
problems — authentication failures, date range caps, sync issues, or missing data.
If you ignore them, the user will get incomplete or stale data without knowing it.

After collection, always:
1. Report any bridge messages prominently (not buried in a summary)
2. Call out which accounts were affected
3. Flag any accounts that returned 0 transactions — this may indicate a problem
4. If an account shows a $0 balance or no data, tell the user explicitly

The user depends on this data being accurate and complete. Silent failures are
unacceptable — if something looks wrong, say so.

## Step 2: Check for stale manual accounts

Some accounts aren't connected through SimpleFIN and require manual balance entry.
Each manual account has a `refresh_days` setting (e.g., daily for 401k/HSA, monthly
for real estate/vehicles). Check which ones need updating:

```bash
cargo run -p simplefin-cli -- stale --storage "$SIMPLEFIN_DATA"
```

If any accounts are stale, the output is a JSON array with account details. Ask the
user for current balances for each stale account, then update them:

```bash
# Example: update a manual account balance
cargo run -p simplefin-cli -- add-balance --storage "$SIMPLEFIN_DATA" \
  --name "My 401k" --org "My Provider" --balance 25000.00 --refresh-days 1
```

If no accounts are stale, the output is: "All manual account balances are up to date."

Each `add-balance` call records a timestamped balance snapshot. The `--refresh-days`
flag controls how often the account is considered stale (default: 1 day).

## Step 3: Query the data

```bash
# All data (includes manual accounts and balance history)
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

### Step 3b: Get a net worth summary

For net worth questions, use `summary` instead of `query` — it does the classification
and math in Rust so you don't have to:

```bash
cargo run -p simplefin-cli -- summary --storage "$SIMPLEFIN_DATA"
```

The output includes `net_worth` (categorized totals, total_assets, total_liabilities,
net_worth) and `changes` (per-account balance deltas since the previous collection).
This is the fastest way to answer "what's my net worth?" — just read the JSON output.

### Query output format

The query output is JSON with four top-level keys:

```json
{
  "organizations": [...],      // Banks/institutions from SimpleFIN
  "accounts": [...],           // Unified accounts (SimpleFIN + manual, each has "source" field)
  "transactions": [...],       // Transactions with context
  "balance_history": [...]     // Balance snapshots over time (all accounts)
}
```

### Account fields (unified)
- `id` — unique account ID (SimpleFIN IDs or "manual-{org}-{name}" for manual)
- `name` — account display name (e.g., "Joint Checking (3365)")
- `org_name` — institution name (e.g., "Vanguard", "Chase")
- `balance` — current balance as decimal string
- `available_balance` — available balance (null for manual accounts)
- `balance_date` — epoch timestamp of when balance was last updated (null if no history)
- `currency` — currency code (may be empty for USD)
- `source` — `"simplefin"` or `"manual"`

### Transaction fields
- `id` — unique transaction ID
- `account_id` — which account this belongs to
- `account_name` — human-readable account name
- `org_name` — institution name
- `posted` — epoch timestamp
- `amount` — decimal string (negative = debit, positive = credit)
- `description` — transaction description/memo
- `pending` — boolean

### Balance history fields
- `account_id` — account this snapshot belongs to (SimpleFIN or manual)
- `timestamp` — epoch timestamp when this balance was recorded
- `balance` — balance at that point in time as decimal string

## Step 4: Analyze and answer

Parse the JSON output and answer the user's question. Common analyses:

- **Net worth**: Use the `summary` subcommand — it returns categorized totals and grand total, computed in Rust. No manual arithmetic needed.
- **Net worth by category**: Also in the `summary` output — categories are Cash, Investments, Other Assets, Credit Cards, Loans.
- **Net worth trends**: Group `balance_history` by timestamp, sum all balances at each collection time to show total net worth over time.
- **Changes since last collection**: The `summary` output includes a `changes` array showing per-account balance deltas.
- **Spending**: Sum negative transaction amounts over a period
- **Income**: Sum positive transaction amounts over a period
- **Account summary**: Use `query` — accounts are unified (SimpleFIN + manual in one list with `source` field)
- **Transaction search**: Filter transactions by description, amount, date
- **Trends**: Compare balances across time using `balance_history`

When presenting financial data:
- Format currency amounts with commas and two decimal places (e.g., $1,234.56)
- Convert epoch timestamps to human-readable dates
- Group accounts by institution for readability
- Negative amounts are debits/spending; positive amounts are credits/income
- Include manual accounts in net worth calculations
- If balances seem stale (balance-date is old), mention when they were last updated

## Account classification

When presenting net worth or account summaries, accounts are grouped into five
categories:

| Category | What goes here |
|----------|---------------|
| **Cash** | Checking and savings accounts |
| **Investments** | Brokerage, IRA, 401(k), Roth IRA accounts |
| **Other Assets** | Real estate, vehicles, HSA |
| **Credit Cards** | Credit card accounts — balances are liabilities |
| **Loans** | Mortgage and other loans |

### Classification rules

The classification is done in Rust by `classify_account()`. The summary command handles
this automatically. The rules are heuristic-based (checking account/org name keywords).
If the automatic classification is wrong for a specific account, it can be overridden
in `config.json` in the data directory via `classification_overrides` (maps account ID
to category).

### Per-user configuration

User-specific settings live in `config.json` in the data directory (not in the repo).
This file supports:

- `excluded_account_patterns` — account names matching these patterns (case-insensitive)
  are excluded from net worth and change calculations. Useful for authorized-user
  duplicates, closed accounts, or test accounts.
- `classification_overrides` — maps account IDs to specific categories, overriding the
  heuristic classifier.

The `summary` command reads this config automatically. When accounts are misclassified
or need to be excluded, update this file — don't hardcode fixes in the Rust source.

## Important notes

- Financial amounts are precise decimals, not floats — trust them as-is
- Balance history accumulates over time — each `collect` or `add-balance` adds a new snapshot
- Some accounts (like brokerage) may not have transactions but will have balances
- The `currency` field may be empty — assume USD in that case
- Investment account balances reflect total portfolio value
- Always collect fresh data before answering if the user asks about "current" or "latest" state
- **Never suppress or ignore bridge messages** — they are the only way to know if data is incomplete
