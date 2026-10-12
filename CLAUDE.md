# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust client library + CLI for the [SimpleFIN Bridge](https://www.simplefin.org/) financial data aggregation API. Ported from the Dart project at `../simplefin_dart`. Uses the **Asupersync** async runtime (NOT Tokio).

## Project Structure

This is a Cargo workspace mono-repo with three first-class crates:

- **`simplefin/`** — Library crate. API client, data models, credentials, storage abstraction, financial analysis.
- **`simplefin-cli/`** — Binary crate (`simplefin` executable). CLI for setup, data collection, querying, and net worth summaries.
- **`simplefin-app/`** — GPUI + Ely app (web/WebAssembly, Android and desktop) with Empower-style features over the library. See `simplefin-app/README.md`; it builds Ely from a sibling checkout via the workspace `[patch]`.

## Build & Test

```bash
cargo build          # Build library + CLI
cargo clippy         # Lint (must pass with zero warnings)
cargo test -p simplefin -p simplefin-cli  # Run library + CLI tests (389 currently)
cargo run -p simplefin-cli -- --help  # Run the CLI
```

## Runtime: Asupersync (NOT Tokio)

This project uses [Asupersync](https://github.com/Dicklesworthstone/asupersync) as its async runtime. **Forbidden crates in core code:** `tokio`, `hyper`, `reqwest`, `axum`, `async-std`, `smol`.

Key patterns:
- Native client async functions take `cx: &Cx` as first parameter (after `&self`). The sync pipeline does not: `collect_data(fetcher, storage, now, on_event)` takes an `AccountFetcher` (no `cx`); natively use `AccessClient::fetcher(cx)`
- asupersync and `clients/` are native-only (`cfg(not(target_arch = "wasm32"))`); the library also builds for `wasm32-unknown-unknown` (check with `cargo check -p simplefin --target wasm32-unknown-unknown`). Don't use `std::time::SystemTime::now()` in library code (it panics on wasm32); use chrono (`wasmbind` is enabled on wasm32) or take `now` as a parameter
- HTTP via `asupersync::http::h1::{HttpClient, HttpClientBuilder, Method}`
- Response type has `.status: u16` and `.body: Vec<u8>`
- Headers passed as `Vec<(String, String)>`, body as `Vec<u8>`
- Runtime bootstrap: `RuntimeBuilder::current_thread().build()?.block_on(async { ... })`
- `Cx::for_request()` for CLI-style capability context
- TLS roots are target-specific in `simplefin/Cargo.toml`: `tls-native-roots` on desktop, `tls-webpki-roots` on Android/iOS (no native store there). Verify with `cargo tree -p simplefin --target aarch64-linux-android -e features | grep -i webpki`.

## Architecture

**Library modules (`simplefin/src/`):**
- `models/` — Five serde-annotated structs: `BridgeInfo`, `Organization`, `Account`, `AccountSet`, `Transaction`. All derive both `Serialize` and `Deserialize`. Custom deserializers in `models/serde_helpers.rs` handle Decimal-from-string-or-number and pending-from-bool-or-number.
- `credentials.rs` — `SetupToken` (Base64 decode → claim URL) and `AccessCredentials` (parse access URL → extract Basic Auth username/password, build endpoint URLs).
- `requests.rs` — Transport-independent HTTP: `AccountQueryParams`, `HttpRequestSpec` (method, credential-free URL, headers incl. Basic Auth), `AccessCredentials::accounts_request()`, `SetupToken::claim_request()`, `parse_accounts_response()`/`parse_claim_response()` (status/body → typed result or error). Any transport (asupersync natively, browser `fetch` on wasm32) reuses these; host transport failures map to `SimplefinError::Transport`.
- `clients/` (native only) — `BridgeClient` (bridge info + claim token exchange) and `AccessClient` (account/transaction queries; `fetcher(cx)` gives an `AccessFetcher` for `collect_data`). Both send `requests.rs` specs over asupersync's native HTTP client.
- `sync.rs` — The collection pipeline shared by CLI and app: `collect_data()` (anomaly detection vs previous accounts, org/account upserts, deduped balance snapshots, per-account incremental fetch with corrupted-state recovery from `max_stored_posted`, sync state that never moves backwards and ignores pending/zero-posted, persisted `WarningRecord`). Returns a `CollectReport` and reports progress via `CollectEvent` callbacks (no printing). Network is abstracted behind the transport-independent `AccountFetcher` trait (`fetch_accounts(&self, params)`; natively `AccessClient::fetcher(cx)`, faked in tests).
- `storage/` — `Storage` trait for persisting collected data, plus `JsonStorage<F: FileStore>` (JSON-file-based default implementation). All file I/O goes through the `FileStore` trait (read/atomic write/list/remove on relative `/`-separated paths): `DiskFiles` (native only; `JsonStorage::open(dir)`) or `MemoryFiles` (all targets, clones share one map, `entries()`/`from_entries()` let a host persist it; `JsonStorage::with_files(files)`). `OrphanedData.path` is store-relative. Filter types: `OrgFilter`, `AccountFilter`, `TransactionFilter`. `UnifiedAccount` merges SimpleFIN and manual accounts into one type. `unify_accounts()` combines both sources. Balance snapshots deduped when unchanged. `DataConfig` stores per-user settings (exclusion patterns, excluded account IDs, classification overrides) in the data directory. `ManualAccount` includes `refresh_days` for staleness checking. `StaleAccount` reports which manual accounts need balance updates. `WarningRecord` persists anomalies and bridge messages from collection. `StorageStatus` and `compute_status()` provide a quick snapshot of storage state. `DataConfig` also holds `category_kinds` (category → `CategoryKind` overrides) and `budget` (`BudgetConfig`); both are serde-default/skip-empty so older config files load unchanged. `get_transaction_edits()`/`set_transaction_edits()` persist the edit overlay in `transaction_edits.json`.
- `analysis.rs` — Financial analysis: `classify_account()` (five categories), `compute_net_worth()`/`compute_net_worth_detail()` and `compute_changes()` accept `&DataConfig` for exclusions, classification overrides/rules, and display names. `compute_net_worth_history()` reconstructs net worth at historical timestamps. `classify_for_display()` returns both heuristic and effective classifications with confidence flags. Classification priority: ID override > classification rules > heuristic classifier.
- Net worth deltas (in `analysis.rs`): `reconstruct_accounts_at()`/`net_worth_at()` rebuild balances at any timestamp (shared by `compute_net_worth_history()`), `compute_net_worth_change()` gives the change over the last N days (optionally for one `AccountCategory`; window clamped to the earliest snapshot), and `compute_category_history()` gives a per-category series. `account_category()`, `transaction_account_category()` and `transaction_account_excluded()` expose effective account classification/exclusion.
- `anomaly.rs` — Anomaly detection: `detect_anomalies()` compares current vs previous account balances, flagging balances dropped to zero, large changes (>20%), disappeared accounts, and new accounts.
- `spending.rs` — Spending analysis: `classify_transaction()` and `compute_spending()` categorize transactions into spending categories using data-driven keyword patterns stored in `spending_patterns.json` in the data directory. Categories are plain strings, not an enum — users create new categories (e.g., "donations") simply by adding a rule via the CLI. `default_spending_patterns()` provides the initial seed with 16 default categories. `category_label()` converts snake_case to display names. `OTHER_CATEGORY` ("other") is the fallback for unmatched transactions. Patterns support `|` separated keywords. User rules from `config.json` take priority over patterns; `load_spending_rules()`/`assemble_spending_rules()` build that order and `add_user_spending_rule()` inserts a highest-priority rule. `compute_spending()` takes a `Categorizer` and skips hidden transactions.
- `categories.rs` — `CategoryKind` (Income, Expense, Transfer). Default kind is data-driven by name (`"income"` → Income, `"transfer"` → Transfer, else Expense), overridable via `DataConfig::category_kinds`. `normalize_category_name()` is the one place category names are normalized. `Categorizer` bundles rules + edits + kind overrides and is what every analysis takes, so precedence is identical everywhere: per-transaction edit > user spending rules (`config.json`) > patterns > "other". `Categorizer::load(storage)` assembles it.
- `edits.rs` — Per-transaction user overlay: `TransactionEdit` (description rename, category override, `hidden`) in `TransactionEdits`, keyed by account ID then transaction ID (SimpleFIN IDs are only unique per account). Stored separately from transactions, so it survives re-collection. `effective_description()`/`effective_category()` (rules match the original provider description). `rule_from_transaction()`/`apply_category_to_future()` create an "apply to future" user spending rule from the normalized merchant, replacing any older rule for the same pattern.
- `month.rs` — `YearMonth`, a UTC calendar month (`YYYY-MM`) with epoch bounds and ranges.
- `cashflow.rs` — `compute_cash_flow()` per UTC month over a range: income, expense (positive magnitudes), net and per-category breakdown. `cash_flow_entry()` is the single income/expense definition: excludes pending, hidden, Transfer-kind categories, `Loans` accounts and config-excluded accounts. Income adds the signed amount and expense adds its negation, so refunds reduce expense and net equals the sum of included amounts.
- `budget.rs` — `BudgetConfig` (overall `monthly_limit`, per-category `category_limits`) and `compute_budget_status(.., now)`: spent vs limit, linear-pace expected spend, last month through the same day (clamped), top 5 expense categories + "other" (real "other" spending folds into that bucket), 3- and 12-month averages of complete months clipped to the start of history (extrapolating the current month when none exist), per-category limit status, and a suggested limit (3-month average) when none is set. Built on cash flow.
- `query.rs` — `TransactionQuery` (free text over effective description/payee/memo, effective category, `AccountCategory`, date range start-inclusive/end-exclusive, include-hidden) returning `TransactionView`s newest first. `transaction_date()` falls back to `transacted_at` for pending rows without a posted date.
- `recurring.rs` — Recurring expense detection: `detect_recurring()` groups transactions by normalized merchant name, detects regular intervals (weekly, monthly, quarterly, annual), and estimates monthly cost. Includes public `normalize_merchant()` for stripping POS prefixes and trailing IDs. Groups by the original description so renaming one occurrence does not split a series; skips hidden transactions.
- `trends.rs` — Spending trend analysis: `compute_trends()` computes month-over-month spending for Expense-kind categories (skipping hidden), calculates monthly averages, and detects trend direction (up, down, stable) by comparing first-half vs second-half averages.
- `error.rs` — Single `SimplefinError` enum with variants: `InvalidSetupToken`, `DataFormat`, `Api`, `Http`, `InvalidArgument`, `Storage`, `Transport` (host transports); `Http` is native-only.

**CLI (`simplefin-cli/src/`):**
- `main.rs` — Clap-derived CLI (`collect` delegates to `simplefin::collect_data` and renders its `CollectEvent`s to stderr; analyses use `Categorizer::load`) with subcommands: `claim` (c), `info` (i), `collect` (l), `add-balance` (a), `stale` (t), `query` (q), `summary` (s), `spending` (p), `spending-rules` (sr), `recurring` (r), `trends` (tr), `configure` (cfg), `status` (st), `schema`, `cleanup`. Global flags: `--format json|text`, `--raw` (bare JSON without envelope). All commands output a structured envelope `{"data":..., "warnings":..., "errors":...}` by default.
- `format.rs` — Human-readable text formatters for each command output type (summary table, status dashboard, spending breakdown, etc.).
- Loads `.env` via dotenvy for `SIMPLEFIN_ACCESS_URL`. Storage path via `--storage` flag or `SIMPLEFIN_DATA` env var.

## API Flow

1. User gets a setup token from the SimpleFIN Bridge UI
2. `claim` subcommand: Base64-decodes token → POSTs to claim URL → receives access URL with embedded credentials
3. Access URL stored as `SIMPLEFIN_ACCESS_URL` in `.env`
4. `collect` subcommand: Fetches all accounts/transactions, persists idempotently to local JSON storage, records balance snapshots (deduped), persists warnings/anomalies
5. `status` subcommand: Quick assessment of storage state (last collection, account counts, stale accounts, warnings)
6. `query` subcommand: Reads from local storage, outputs unified accounts (SimpleFIN + manual) with filtered JSON
7. `summary` subcommand: Computes categorized net worth, balance changes, and optional historical net worth time series
8. `configure` subcommand: View/modify account classifications, display names, and exclusions

## Data-Driven Classification Architecture

Classification and analysis use a layered, data-driven approach. **No user-specific data belongs in source code.**

**Spending classification** is fully data-driven:
- Patterns stored in `spending_patterns.json` in the user's data directory (not in the repo)
- Seeded from `default_spending_patterns()` on first use, then fully user-editable
- User rules from `config.json` `spending_rules` take highest priority
- Managed via `spending-rules` CLI subcommand (add/remove/list/reset)
- Unclassified transactions (category "Other") are surfaced with description and amount so the user can teach the classifier

**Account classification** uses a priority chain:
1. Per-account ID overrides in `config.json` (highest priority, user-set)
2. Classification rules in `config.json` (pattern-based, user-set)
3. Heuristic classifier in `classify_account()` (fallback only)

The heuristic classifier currently contains US-specific institution keywords (Vanguard, Schwab, Chase, etc.) as a fallback. This is documented in `specs/futures.md` Problem 2 as a future externalization target — the plan is to move all heuristic keywords into data-driven locale presets so the Rust code has zero domain knowledge baked in.

**What's intentionally in code (schema, not data):**
- `AccountCategory` enum (Cash, Investments, OtherAssets, CreditCards, Loans) — defines the classification schema
- `CategoryKind` enum (Income, Expense, Transfer) — the schema for how a category participates in cash flow; which category has which kind is data (defaults by name, overrides in `config.json`)
- Spending categories are **data-driven strings**, not an enum — categories are defined by whatever appears in `spending_patterns.json`. Adding `--category donations` via the CLI creates a new category with zero code changes.
- Algorithmic constants (anomaly 20% threshold, trend 10% threshold, recurring interval tolerances) — documented in `specs/futures.md` as future config targets

**What's in the user's data directory (data, not schema):**
- `spending_patterns.json` — spending classification patterns
- `config.json` — account overrides, classification rules, exclusions, spending rules
- `warnings.json` — persisted anomalies and bridge messages
- `transaction_edits.json` — per-transaction user edits (rename, category, hidden)

## Key Conventions

- JSON wire format uses hyphenated keys (`sfin-url`, `available-balance`, `balance-date`) — handled via `#[serde(rename = "...")]`
- The API's `"errors"` field contains informational messages, not errors — renamed to `server_messages` in Rust
- Financial amounts use `rust_decimal::Decimal` (not floats)
- Timestamps stored as `i64` epoch seconds in structs; helper methods (`posted_iso8601()`, `balance_date_iso8601()`) format with `Z` suffix
- `AccessCredentials` derives `Clone` so it can be reused across multiple clients
- Storage is idempotent: transactions deduped by ID, account/org metadata updated to latest, balance snapshots deduped when unchanged
- Account classification uses five categories (Cash, Investments, Other Assets, Credit Cards, Loans)
- User-specific settings (exclusions, classification overrides) stored in `config.json` in the data directory, not in the repo
- Manual accounts have per-account `refresh_days` for staleness checking

## Architecture Best Practices

These apply to all code in this project — frontend and server:

- TDD — write tests first, then implementation. Code isn't done until the tests pass.
- DRY — extract shared logic into utilities/modules. No copy-paste duplication.
- Separation of Concerns — each module handles one distinct responsibility.
- Single Responsibility — every file/function/module has exactly one reason to change.
- Clear Abstractions — expose intent through small, stable interfaces. Hide implementation details.
- Low Coupling, High Cohesion — modules are self-contained with minimal cross-dependencies.
- KISS — keep solutions as simple as possible. Complexity must justify itself.
- YAGNI — don't build for hypothetical future requirements. Solve today's problem.
- Prefer Non-Nullable — use Option/undefined sparingly. Default to required fields.
- Prefer Async Notifications — push over poll. The DataBroker pattern, not setInterval.
- Eliminate Race Conditions — no dropped or corrupted data from concurrent access.
- Errors Are Not Optional — log errors and inform the user of them, don't hide them. Every failure must be tracked in a centralized log so it can be used to improve the app over time.
- Idiomatic Project Layout — follow language/framework conventions for folder structure, lints, and tooling.
- Write for Maintainability — clear, readable code that future developers can understand without archaeology.
- No PII in the repo — never include real account numbers, names, balances, institution-specific details, email addresses, file paths containing usernames, or any personally identifying financial data in code, tests, docs, examples, or commit messages. Use generic placeholders instead (e.g., "Checking (1234)", "My 401k", "Duplicate Account"). User-specific data belongs in the data directory, not in source control.
- Pre-commit PII scan — before every commit, scan all staged changes for PII patterns: real names, email addresses, account numbers, specific balances, addresses, employer names, institution-specific account details, and paths containing usernames or email addresses. If any PII is found, fix it before committing. This is a blocking requirement, not optional.
