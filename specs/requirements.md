# Requirements

## 1. Project Structure

### 1.1 Cargo Workspace
- The repo MUST be a Cargo workspace with two crates:
  - `simplefin` — library crate
  - `simplefin-cli` — binary crate, depends on `simplefin`
- Both crates are first-class deliverables.

## 2. Library: Data Models

### 2.1 Core Types
- **BridgeInfo** — Protocol version list from the bridge server.
- **Organization** — Financial institution identity: `sfin_url` (required), plus optional `name`, `domain`, `url`, `id`.
- **Account** — Financial account: org, id, name, currency (ISO-4217), balance, optional available_balance, balance_date, transactions list.
- **Transaction** — Individual transaction: id, posted date, amount, description, optional transacted_at, pending flag.
- **AccountSet** — API response wrapper: list of accounts plus server messages.

### 2.2 Serialization
- All models MUST derive both `Serialize` and `Deserialize` for round-trip fidelity.

### 2.3 Financial Precision
- All monetary amounts (`balance`, `available_balance`, `amount`) MUST use `Decimal` (not floating-point).
- Amounts in the JSON wire format may arrive as strings or numbers; the deserializer MUST handle both.

### 2.4 Timestamp Handling
- Timestamps are stored as `i64` epoch seconds internally.
- Helper methods provide ISO-8601 formatted strings with `Z` (UTC) suffix.
- The `pending` field in wire format may be a boolean or a number (0 = false, non-zero = true); the deserializer MUST handle both.

### 2.5 Wire Format
- JSON keys use hyphens (`sfin-url`, `balance-date`, `available-balance`), mapped to snake_case Rust fields via serde rename.
- The API's `"errors"` array contains informational messages, not errors. It MUST be exposed as `server_messages` to avoid confusion.

### 2.6 Organization Utilities
- `display_name()` — Returns the best human-readable name: `name` > `domain` > `id` > `sfin_url`, in priority order.
- `key()` — Returns a deduplication key: `id` if present, otherwise `domain`.

### 2.7 AccountSet Filtering
- `filter_by_organization_id(org_id)` — Returns a new AccountSet containing only accounts whose organization matches the given ID, preserving server messages.

## 3. Library: Credentials

### 3.1 Setup Token
- Parse a Base64-encoded setup token into a claim URL.
- MUST support both standard and URL-safe Base64 encoding.
- MUST validate: successful Base64 decode, valid UTF-8, valid URL with http/https scheme and host.

### 3.2 Access Credentials
- Parse an access URL (e.g., `https://user:pass@host/path`) into components: base URL, username, password.
- Percent-decode username and password from the URL.
- Provide a `basic_auth_header_value()` method returning `"Basic {base64(user:pass)}"`.
- Provide an `endpoint_url()` method for building API endpoint URLs with path segments and query parameters.
- Access credentials MUST be `Clone`-able for reuse across multiple clients.

## 4. Library: HTTP Clients

### 4.1 BridgeClient
- `get_info()` — GET `{root}/info`, returns BridgeInfo.
- `claim_access_credentials(setup_token)` — Base64-decode the token, POST to the claim URL, return AccessCredentials parsed from the response body.
- Configurable root URL (defaults to `https://bridge.simplefin.org/simplefin`).
- Configurable user agent string.

### 4.2 AccessClient
- `get_accounts(params)` — GET `{base_url}/accounts` with Basic Auth header, returns AccountSet.
- Query parameters:
  - `start-date` — Filter transactions from this epoch timestamp.
  - `end-date` — Filter transactions to this epoch timestamp.
  - `pending=1` — Include pending transactions.
  - `account` — Filter to specific account ID(s) (may appear multiple times).
  - `balances-only=1` — Return accounts without transactions.
- Configurable user agent string.

### 4.3 Runtime
- All async functions take `cx: &Cx` as their first parameter (Asupersync capability context).
- HTTP via `asupersync::http::h1::{HttpClient, HttpClientBuilder, Method}`.
- Forbidden crates in core library code: `tokio`, `hyper`, `reqwest`, `axum`, `async-std`, `smol`.

## 5. Library: Storage

### 5.1 Storage Trait
The library MUST provide a `Storage` trait with the following operations:

**Write (upsert for idempotency):**
- `upsert_organizations(orgs)` — Insert or update organizations.
- `upsert_accounts(accounts)` — Insert or update accounts (balance updated to latest).
- `upsert_transactions(account_id, txns)` — Insert or update transactions (deduped by transaction ID).

**Read (for querying collected data):**
- `get_organizations(filter)` — Retrieve organizations, optionally filtered.
- `get_accounts(filter)` — Retrieve accounts, optionally filtered.
- `get_transactions(filter)` — Retrieve transactions, optionally filtered.

**Collection state:**
- `last_collected(account_id)` — Returns the most recent transaction timestamp for an account (epoch seconds), used for incremental fetching.

**Manual accounts:**
- `get_manual_accounts()` — Retrieve manually-tracked accounts.
- `upsert_manual_account(account)` — Add or update a manual account with balance snapshot.

**Balance history:**
- `get_balance_history(account_id)` — Retrieve timestamped balance snapshots.
- `record_balance_snapshot(account_id, snapshot)` — Record a new balance data point (deduped when unchanged).

**Spending patterns:**
- `get_spending_patterns()` — Retrieve spending classification patterns from `spending_patterns.json`. Auto-seeds from `default_spending_patterns()` if file doesn't exist.
- `set_spending_patterns(patterns)` — Persist updated spending patterns.

**Configuration:**
- `get_config()` — Retrieve user configuration from `config.json`.
- `set_config(config)` — Persist updated configuration.

**Warnings:**
- `get_warnings()` — Retrieve persisted warnings/anomalies.
- `set_warnings(warnings)` — Persist warnings from collection.

### 5.2 Filters
- **OrgFilter** — Filter by org ID or name.
- **AccountFilter** — Filter by account ID, name, or org.
- **TransactionFilter** — Filter by account, org, date range, pending status.

### 5.3 Default Implementation
- The library MUST ship with at least one default `Storage` implementation.
- The storage backend is an implementation detail — the spec does not mandate a specific backend (flat files, SQLite, etc.).
- When [FrankenSQLite](https://github.com/Dicklesworthstone/frankensqlite) ships async support ([issue #49](https://github.com/Dicklesworthstone/frankensqlite/issues/49)), it becomes the preferred backend.

### 5.4 Idempotency Guarantees
- Running `collect` twice with the same data MUST produce the same storage state.
- Transactions are deduped by transaction ID.
- Account balances and metadata are updated to the latest values.
- Organization metadata is updated to the latest values.

## 6. Library: Error Handling

### 6.1 Error Types
A single `SimplefinError` enum with variants:
- **InvalidSetupToken** — Token parsing failed (message + optional source).
- **DataFormat** — Response data doesn't match expected format (message + optional source).
- **Api** — Server returned non-success status (URI, status code, message, response body).
- **Http** — Low-level HTTP error (from asupersync).
- **InvalidArgument** — Caller provided invalid arguments.

### 6.2 Result Type
- Library provides `type Result<T> = std::result::Result<T, SimplefinError>`.

## 7. Library: Testability
- The library MUST be fully testable.
- All external dependencies (HTTP, storage) MUST be abstractable for testing.
- How testability is achieved is an implementation detail.

## 8. CLI: General

### 8.1 Structure
- Built with `clap` derive macros.
- Each subcommand has a single-letter alias.
- Access URL sourced from `SIMPLEFIN_ACCESS_URL` environment variable (loaded from `.env` via dotenvy), overridable with `--url`.
- Exit code 0 on success, 1 on failure.

### 8.2 Subcommands
The CLI has the following subcommands:

### 8.3 Output Format
- All commands output a structured envelope by default: `{"data":..., "warnings":..., "errors":...}`
- `--raw` flag outputs bare JSON (no envelope)
- `--format text` outputs human-readable tables
- Warnings are automatically populated from persisted collection data

## 9. CLI: `claim` (alias: `c`)
- **Input:** Positional `setup_token`, optional `--bridge` URL.
- **Output:** `SIMPLEFIN_ACCESS_URL={url}` (suitable for pasting into `.env`).
- Exchanges a setup token for an access URL via the bridge.

## 10. CLI: `info` (alias: `i`)
- **Input:** Optional `--bridge` URL.
- **Output:** List of protocol versions supported by the bridge.

## 11. CLI: `collect` (alias: `l`)
- **Input:** Optional `--url`, required storage location (path or connection string).
- **Behavior:**
  - Fetches all organizations, accounts, and transactions from the SimpleFIN API.
  - On first run: fetches all available transaction history.
  - On subsequent runs: fetches incrementally from the last collected timestamp per account.
  - Persists data idempotently via the `Storage` trait.
  - Transactions deduped by ID; account/org metadata updated to latest.
  - Records balance snapshots (deduped when unchanged).
  - Runs anomaly detection, persists warnings.
- **Output (default):** One-line summary, e.g., "Collected 47 new transactions across 5 accounts (12 duplicates skipped)".
- **Output (`--verbose`):** Per-account breakdown of what was added/updated.

## 12. CLI: `query` (alias: `q`)
- **Input:** Required storage location, optional filters:
  - `--account <id or name>` — Filter to a single account.
  - `--org <id or name>` — Filter to all accounts for an organization.
  - `--start-date <date>` — Filter transactions from this date.
  - `--end-date <date>` — Filter transactions to this date.
  - `--pending` — Include/exclude pending transactions.
- **Date parsing:** Accepts epoch seconds, ISO-8601 (RFC 3339), or date-only (`YYYY-MM-DD` → midnight UTC).
- **No filters:** Dumps all collected data.

## 12a. CLI: `status` (alias: `st`)
- **Output:** Last collection time, account counts, stale manual accounts, warnings.

## 12b. CLI: `add-balance` (alias: `a`)
- **Input:** `--name`, `--org`, `--balance`, optional `--refresh-days` (default 1).
- Adds or updates a manual account balance for accounts not in SimpleFIN.

## 12c. CLI: `stale` (alias: `t`)
- **Output:** Manual accounts whose balances exceed their `refresh_days` threshold.

## 12d. CLI: `summary` (alias: `s`)
- **Output:** Categorized net worth (Cash, Investments, Other Assets, Credit Cards, Loans).
- `--detail` — Per-account breakdown within each category.
- `--history N` — Net worth at each of the last N collection timestamps.

## 12e. CLI: `spending` (alias: `p`)
- **Output:** Spending by category with totals, plus unclassified transaction list (description + amount).
- `--start-date`, `--end-date` — Filter date range.
- Classification is fully data-driven via `spending_patterns.json`.

## 12f. CLI: `spending-rules` (alias: `sr`)
- Manages spending classification patterns stored in `spending_patterns.json`.
- `--list` — Show all patterns.
- `--add PATTERN --category CAT` — Add a new pattern (prepended at highest priority).
- `--remove PATTERN` — Remove patterns matching a substring.
- `--reset` — Reset to default patterns.

## 12g. CLI: `recurring` (alias: `r`)
- **Output:** Detected recurring expenses with merchant, amount, frequency, category.
- `--min-occurrences N` — Minimum transaction count to consider (default 2).

## 12h. CLI: `trends` (alias: `tr`)
- **Output:** Month-over-month spending by category with trend direction.
- `--months N` — Number of months to analyze (default 6).

## 12i. CLI: `configure` (alias: `cfg`)
- View and modify account classifications, display names, and exclusions.
- `--list` — Show all accounts with heuristic and effective classifications.
- `--set ACCOUNT_ID` with `--name`, `--category`, `--exclude`, `--include`.

## 12j. CLI: `schema`
- Prints JSON Schema for any output type (summary, status, spending, etc.).

## 12k. CLI: `cleanup`
- Finds orphaned data files. `--remove` to delete them.

## 13. Library: Analysis

### 13.1 Data-Driven Classification
- **Spending classification** MUST be fully data-driven. Patterns are stored in the user's data directory (`spending_patterns.json`), not hardcoded in the binary. The library provides `default_spending_patterns()` as seed data only. User-specific patterns from `config.json` take priority over stored patterns, which take priority over defaults.
- **Account classification** MUST use a priority chain: per-account ID overrides > classification rules > heuristic fallback. The heuristic classifier exists as a convenience fallback; it MUST NOT be the only classification path. Users MUST be able to override any classification via config without modifying code.
- Unclassified transactions (spending category "Other") MUST be surfaced with both description and amount so users can teach the classifier.

### 13.2 Account Categories
Five categories defined as a Rust enum: Cash, Investments, OtherAssets, CreditCards, Loans. Each has an asset/liability designation used for net worth computation. Future: user-defined categories (see `specs/futures.md` Problem 1).

### 13.3 Spending Categories
Spending categories are **data-driven strings**, not a Rust enum. Categories are implicitly defined by whatever appears in `spending_patterns.json` — adding a rule with `--category donations` creates a new category with zero code changes. The library ships 16 default categories as seed data (Restaurants, Groceries, Utilities, Transportation, Shopping, Entertainment, Healthcare, Housing, Insurance, Subscriptions, Education, Personal Care, Pets, Income, Transfer, Other). `category_label()` converts snake_case names to display labels. `OTHER_CATEGORY` ("other") is the fallback for unmatched transactions.

### 13.4 Anomaly Detection
Compares current vs previous account balances during collection, flags: balances dropped to zero, large changes (>20%), disappeared accounts, new accounts. Warnings persisted to `warnings.json`.

### 13.5 Recurring Expense Detection
Groups transactions by normalized merchant name (POS prefix stripping, trailing ID removal), detects regular intervals (weekly/monthly/quarterly/annual), estimates monthly cost.

### 13.6 Spending Trend Analysis
Month-over-month spending by category, monthly averages, trend direction (up/down/stable) via first-half vs second-half comparison.

## 14. Tests

### 14.1 Credential Tests
- Setup token parsing: valid tokens, invalid Base64, missing scheme, missing host.
- Access credential parsing: valid URLs, percent-decoding, Basic Auth header generation, endpoint URL construction.

### 14.2 Model Tests
- Deserialization of all model types from JSON.
- Serialization round-trip for all model types.
- Custom deserializer coverage: Decimal-from-string, Decimal-from-number, pending-from-bool, pending-from-number.

### 14.3 Pipeline Tests
- End-to-end flow from raw API JSON through model deserialization, credential handling, query parameter building, business logic (filtering, deduplication), and output.

### 14.4 Storage Tests
- Upsert idempotency: inserting the same data twice produces the same state.
- Incremental collection: `last_collected` tracks per-account state correctly.
- Filter correctness: all filter combinations return expected results.
- Spending pattern storage: get/set round-trip, auto-seeding from defaults.

### 14.5 Analysis Tests
- Account classification: heuristic fallback, override priority, confidence flags, investment pre-check (401k/403b/457/pension over cash keywords).
- Spending classification: data-driven patterns, pipe-separated keywords, custom rule priority, empty rules produce "Other", unclassified surfacing with amounts.
- Recurring detection: monthly/weekly/quarterly detection, irregular rejection, min-occurrences, merchant normalization (POS prefix stripping).
- Trend analysis: direction detection, category aggregation, month bucketing.
- Anomaly detection: zero-balance, large-change, disappeared/new accounts.

### 14.6 CLI Tests
- `collect` summary output accuracy.
- `query` JSON output correctness with various filter combinations.
