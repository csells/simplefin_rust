# Vision

After losing Mint and Personal Capital, I decided that a stable place to track my accounts and transactions is in order. So I found [SimpleFIN](https://www.simplefin.org/), which allows you to put the credentials in for your various financial institutions and tracks their balance and transactions, exposing them with a simple, secure REST API for only $15/year.

There are several existing OSS apps that plug into SimpleFIN, but of course, I wanted to build my own, so `simplefin_rust` was born. This is a Rust port of the original [Dart implementation](https://github.com/csells/simplefin_dart), motivated by skipping the Dart/Flutter installation and improving performance.

## Project Structure

This is a Cargo workspace mono-repo with two first-class crates:

1. **`simplefin` (library crate)** — Type-safe, correct Rust types for the SimpleFIN API, a persistent storage abstraction for collected data, suitable for embedding in other Rust applications. Library API design takes priority when library and CLI concerns conflict.
2. **`simplefin-cli` (binary crate)** — A command-line tool for setting up credentials, collecting financial data idempotently, and querying collected data — for both human and agent use.

The library covers:

- Claim an access URL from a one-time setup token.
- Query bridge metadata (`GET /info`).
- Retrieve accounts and transactions with rich typed models.
- Persist and query collected data via a pluggable storage backend.
- Financial analysis: categorized net worth, balance change detection, anomaly detection.
- Spending analysis: data-driven transaction classification, recurring expense detection, trend analysis.
- Manual account tracking for institutions SimpleFIN can't reach.

## Design Principles

- **Correctness over convenience** — Financial amounts use `Decimal`, not floats. Timestamps are handled precisely. Wire format quirks are absorbed by the library so callers don't have to.
- **Data-driven, not hardcoded** — Classification patterns, spending rules, and user preferences live in the data directory, not in source code. The binary ships sensible defaults that are seeded on first use and fully user-editable thereafter. The goal is zero user-specific data in the codebase. Where heuristics remain in code (e.g., the account classification fallback), they are documented as future externalization targets in `specs/futures.md`.
- **Simplicity** — Minimal dependencies, straightforward API surface, no over-abstraction. The library mirrors the SimpleFIN API closely rather than inventing its own abstractions.
- **Testability** — The library must be fully testable. All external dependencies must be abstractable for testing.
- **Asupersync runtime** — Uses the [Asupersync](https://github.com/Dicklesworthstone/asupersync) async runtime instead of Tokio, because Asupersync rocks.
- **Transparency** — Server messages (the API's misleadingly-named `"errors"` field) are surfaced to callers, not hidden. They contain useful operational information. Anomalies and warnings are persisted and included in every command's output envelope. Silent failures are unacceptable.
- **Teachable** — When the system can't classify something, it surfaces the unknown item (with enough context to identify it) so the user can teach it. Over time, the "Other" bucket shrinks as the user's patterns accumulate.
