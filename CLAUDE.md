# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Rust client library + CLI for the [SimpleFIN Bridge](https://www.simplefin.org/) financial data aggregation API. Ported from the Dart project at `../simplefin_dart`. Uses the **Asupersync** async runtime (NOT Tokio).

## Project Structure

This is a Cargo workspace mono-repo with two first-class crates:

- **`simplefin/`** — Library crate. API client, data models, credentials, storage abstraction, financial analysis.
- **`simplefin-cli/`** — Binary crate (`simplefin` executable). CLI for setup, data collection, querying, and net worth summaries.

## Build & Test

```bash
cargo build          # Build library + CLI
cargo clippy         # Lint (must pass with zero warnings)
cargo test           # Run all tests (265 currently)
cargo run -p simplefin-cli -- --help  # Run the CLI
```

## Runtime: Asupersync (NOT Tokio)

This project uses [Asupersync](https://github.com/Dicklesworthstone/asupersync) as its async runtime. **Forbidden crates in core code:** `tokio`, `hyper`, `reqwest`, `axum`, `async-std`, `smol`.

Key patterns:
- All async functions take `cx: &Cx` as first parameter (after `&self`)
- HTTP via `asupersync::http::h1::{HttpClient, HttpClientBuilder, Method}`
- Response type has `.status: u16` and `.body: Vec<u8>`
- Headers passed as `Vec<(String, String)>`, body as `Vec<u8>`
- Runtime bootstrap: `RuntimeBuilder::current_thread().build()?.block_on(async { ... })`
- `Cx::for_request()` for CLI-style capability context

## Architecture

**Library modules (`simplefin/src/`):**
- `models/` — Five serde-annotated structs: `BridgeInfo`, `Organization`, `Account`, `AccountSet`, `Transaction`. All derive both `Serialize` and `Deserialize`. Custom deserializers in `models/serde_helpers.rs` handle Decimal-from-string-or-number and pending-from-bool-or-number.
- `credentials.rs` — `SetupToken` (Base64 decode → claim URL) and `AccessCredentials` (parse access URL → extract Basic Auth username/password, build endpoint URLs).
- `clients/` — `BridgeClient` (bridge info + claim token exchange) and `AccessClient` (account/transaction queries). Both use asupersync's native HTTP client.
- `storage/` — `Storage` trait for persisting collected data, plus `JsonStorage` (JSON-file-based default implementation). Filter types: `OrgFilter`, `AccountFilter`, `TransactionFilter`. `UnifiedAccount` merges SimpleFIN and manual accounts into one type. `unify_accounts()` combines both sources. Balance snapshots deduped when unchanged. `DataConfig` stores per-user settings (exclusion patterns, excluded account IDs, classification overrides) in the data directory. `ManualAccount` includes `refresh_days` for staleness checking. `StaleAccount` reports which manual accounts need balance updates. `WarningRecord` persists anomalies and bridge messages from collection. `StorageStatus` and `compute_status()` provide a quick snapshot of storage state.
- `analysis.rs` — Financial analysis: `classify_account()` (five categories), `compute_net_worth()`/`compute_net_worth_detail()` and `compute_changes()` accept `&DataConfig` for exclusions, classification overrides/rules, and display names. `compute_net_worth_history()` reconstructs net worth at historical timestamps. `classify_for_display()` returns both heuristic and effective classifications with confidence flags. Classification priority: ID override > classification rules > heuristic classifier.
- `anomaly.rs` — Anomaly detection: `detect_anomalies()` compares current vs previous account balances, flagging balances dropped to zero, large changes (>20%), disappeared accounts, and new accounts.
- `spending.rs` — Spending analysis: `classify_transaction()` and `compute_spending()` categorize transactions into spending categories (Restaurants, Groceries, Utilities, Transportation, Shopping, Entertainment, Healthcare, Housing, Insurance, Subscriptions, Education, Personal Care, Pets, Income, Transfer) using built-in keyword patterns and optional custom rules.
- `error.rs` — Single `SimplefinError` enum with variants: `InvalidSetupToken`, `DataFormat`, `Api`, `Http`, `InvalidArgument`, `Storage`.

**CLI (`simplefin-cli/src/`):**
- `main.rs` — Clap-derived CLI with subcommands: `claim` (c), `info` (i), `collect` (l), `add-balance` (a), `stale` (t), `query` (q), `summary` (s), `spending` (p), `configure` (cfg), `status` (st), `schema`, `cleanup`. Global flags: `--format json|text`, `--raw` (bare JSON without envelope). All commands output a structured envelope `{"data":..., "warnings":..., "errors":...}` by default.
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
