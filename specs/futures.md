# Future: Generalizing SimpleFIN Rust for a Wider Audience

This document captures what needs to change to make this tool useful beyond a
single US-based author. The core insight: the storage, analysis, and CLI layers
have nothing SimpleFIN-specific about them. The tool is really a personal
finance engine that happens to have a SimpleFIN collector. The hardcoded parts
reflect one person's accounts, not deliberate design choices.

## Problem 1: Fixed Account Categories

**Current state:** Five hardcoded categories as a Rust enum in `analysis.rs`:
Cash, Investments, OtherAssets, CreditCards, Loans. The asset-vs-liability
classification is also hardcoded. Users cannot add, remove, or rename
categories.

**Impact:** A Canadian user can't distinguish RRSP from TFSA. A UK user can't
separate ISAs. Someone with rental properties can't split "Real Estate" from
"Vehicles" within OtherAssets. A freelancer can't add "Business" as a category.

**Future direction:** Categories defined in config, not in Rust. Each category
has a name, display label, and an asset/liability flag. The Rust code becomes a
generic engine that reads category definitions at runtime. Ship a default set
(the current five) but let users define their own.

**Design consideration:** This is the deepest refactor -- categories touch
analysis, net worth computation, classification, formatting, schema generation,
and serialization. The enum would become a string-keyed type with config-driven
metadata. Think carefully about what "user-defined categories" means for JSON
schema output (it becomes dynamic).

## Problem 2: US-Specific Account Classification Heuristic

**Current state:** `classify_account()` in `analysis.rs` is a wall of
`contains()` checks for US-specific terms: "401", "ira", "roth", "sapphire",
"freedom", "skymiles", and US-specific institutions: Vanguard, Schwab,
Fidelity, Chase, American Express, HealthEquity.

**Impact:** Non-US users get almost everything classified as OtherAssets
(the default bucket). The `configure` command mitigates this, but the defaults
are so US-centric that non-US users would need to override nearly everything
on first use.

**Future direction:** Move all heuristic keywords out of Rust and into
data-driven presets. Ship locale-specific preset files (e.g., `us.toml`,
`uk.toml`, `canada.toml`, `germany.toml`). The Rust code becomes a generic
pattern-matching engine with zero domain knowledge baked in. Users pick a
preset on first run or define their own rules from scratch. Community can
contribute presets for their country.

**Preset structure (strawman):**
```toml
[meta]
name = "United States"
locale = "en-US"
currency = "USD"

[[rules]]
pattern = "401|ira|roth|brokerage"
field = "name"
category = "investments"

[[rules]]
pattern = "vanguard|schwab|fidelity"
field = "org"
category = "investments"

# ... etc
```

**Design consideration:** The existing `classification_rules` in `DataConfig`
already implement this pattern -- they just aren't the primary path. The shift
is making rules-from-config the *only* path, with presets as the starting
point. The hardcoded `classify_account()` function goes away entirely.

## Problem 3: US-Specific Spending Keywords

**Current state:** `BUILTIN_RULES` in `spending.rs` contains 77+ vendor
keywords, all US-specific: Chipotle, Trader Joe's, Fred Meyer, WinCo,
Albertson's, PG&E, TriMet (Portland transit), CVS, Walgreens. Some are
regional to the Pacific Northwest.

**Impact:** A user in Germany, Japan, or even New York gets almost zero
keyword matches -- everything falls to "Other". The spending analysis is
effectively broken for anyone who doesn't shop at the author's stores.

**Future direction:** Same preset approach as account classification. Move
all spending keywords to locale-specific preset files. The `SpendingCategory`
enum has the same problem as `AccountCategory` -- it should also be
user-defined. Ship US defaults but make them replaceable.

**Design consideration:** Spending categories are simpler than account
categories (no asset/liability distinction), so this could be tackled first
as a warm-up for the account category refactor.

## Problem 4: Currency Handling

**Current state:**
- `format_currency()` in `format.rs` hardcodes the `$` symbol
- `add-balance` defaults to `"USD"`
- Net worth sums balances across currencies without conversion
- No awareness of currency symbols, decimal separators, or grouping

**Impact:** A EUR user sees `$` on all amounts. A multi-currency user (e.g.,
USD checking + GBP savings) gets a net worth number that's meaningless because
it sums different currencies as if they were the same.

**Future direction (incremental):**
1. **Quick win:** Use the account's `currency` field to pick the right symbol
   in `format_currency()`. A simple lookup table (USD->$, EUR->\u20ac, GBP->\u00a3,
   JPY->\u00a5) covers 90% of users.
2. **Medium effort:** Group net worth by currency when accounts span multiple
   currencies. Show separate totals per currency instead of one meaningless sum.
3. **Full solution:** Support exchange rates (manual or fetched) to convert
   everything to a base currency. Could be as simple as a
   `currency_rates: HashMap<String, Decimal>` in config.

**Design consideration:** Multi-currency net worth is genuinely hard to get
right. The "group by currency" approach is honest and useful without requiring
exchange rate infrastructure. Let users who want a single number provide their
own rates.

## Problem 5: Anomaly Detection Threshold

**Current state:** `anomaly.rs` hardcodes a 20% threshold for "large balance
change" detection. One threshold for all accounts.

**Impact:** A checking account going from $500 to $400 (20%) is routine
(rent payment). A brokerage going from $500K to $400K (20%) is a market crash.
A credit card swinging 50% month-to-month is normal. The single threshold
produces both false positives and false negatives.

**Future direction:** Configurable thresholds in `DataConfig`:
```json
{
  "anomaly_threshold_percent": 20,
  "anomaly_thresholds_by_category": {
    "credit_cards": 50,
    "investments": 10
  },
  "anomaly_thresholds_by_account": {
    "some-checking-id": 30
  }
}
```
Priority: per-account > per-category > global default.

**Design consideration:** This is a small, self-contained change. Good
candidate for a first PR toward generalization.

## Problem 6: English-Only Text Output

**Current state:** `format.rs` has 30+ hardcoded English strings: "Net Worth
Summary", "Total Assets", "Changes Since Last Collection", etc.

**Impact:** Only affects `--format text` (JSON output is language-agnostic).
Non-English speakers can still use JSON mode with their own presentation layer.

**Future direction:** This is the lowest priority. Options range from a simple
string table in config to a full i18n framework. Given that the primary
consumer is an AI agent (which reads JSON), and human users can use
`--format text` as a convenience, not a requirement, this is probably not
worth the complexity unless the tool gains a significant non-English user base.

**Pragmatic approach:** Extract strings into a single `const` block or small
struct so they're at least centralized, even if not yet configurable.

## Problem 7: SimpleFIN Coupling

**Current state:** The crate is named `simplefin`. The storage layer, analysis
engine, anomaly detection, spending classification, and most of the CLI have
nothing to do with SimpleFIN specifically. Only `clients/` and `credentials.rs`
are SimpleFIN-specific.

**Impact:** The tool can't accept data from other sources (Plaid, CSV import,
manual entry only, OFX/QFX files). The name implies it's only useful if you
use SimpleFIN Bridge.

**Future direction:** Restructure into:
- A generic personal finance engine crate (storage, analysis, categories,
  spending, anomaly detection)
- A SimpleFIN connector crate (API client, credentials, collection logic)
- A CLI that wires them together and could support additional connectors

**Design consideration:** This is a large structural change but doesn't
require rewriting logic -- it's mostly moving code between crates and
adjusting imports. The payoff is that the engine becomes useful as a library
for anyone building personal finance tools in Rust, regardless of their data
source. It also opens the door to community-contributed connectors (Plaid,
CSV, OFX, Teller, etc.).

## Problem 8: Ledger/Plaintext Accounting Export

**Not a hardcoding problem, but the highest-leverage missing feature.**

The plaintext accounting community (ledger, hledger, beancount -- 15K+ combined
GitHub stars) represents the largest existing audience of CLI-first finance
users. An export command that outputs transactions in ledger/hledger/beancount
format would instantly make this tool useful to that community as a data
pipeline: SimpleFIN -> collect -> export -> ledger.

The official `sfin2ledger` (Python, 12 stars) does this but requires a
separate tool and Python runtime. Building it into the Rust CLI would be
a significant differentiator.

## Problem 9: POS Prefix Noise in Transaction Descriptions

**Current state:** Bank and POS systems prepend vendor-specific prefixes to
transaction descriptions: "Ext Credit Card Debit", "SQ *", "TST*", "DD *",
"PP*", "PURCHASE AUTHORIZED ON", "RECURRING PAYMENT", etc. These make
merchant normalization and pattern matching harder.

**Impact:** Classification rules need to account for these prefixes, and
recurring expense detection groups the same merchant under different names
depending on which card was used.

**Future direction:** A description normalizer pipeline that strips known
prefixes before classification. The `recurring.rs` module already has
`normalize_merchant()` for its own use; this should be promoted to a shared
utility used by both spending classification and recurring detection.

## Problem 10: Amount-Based Classification Signals

**Current state:** Transaction classification is purely description-based.
Amount is ignored.

**Impact:** A $3,000 transfer looks the same as a $3 one. A $15 monthly
charge is probably a subscription; a $15 one-time charge is probably shopping.
Amount patterns could disambiguate borderline classifications.

**Future direction:** Add optional amount ranges to spending rules:
```json
{"pattern": "some vendor", "category": "subscriptions", "min_amount": -50, "max_amount": -5}
```
This keeps the data-driven approach while adding a second signal dimension.

## Problem 11: Score-Based Classification

**Current state:** Classification is first-match-wins with binary keyword
matching. When a transaction matches multiple categories, only rule order
determines the result.

**Impact:** "CVS" could be healthcare or shopping. "Costco" could be
groceries or shopping. The current system picks one based on rule order with
no way to express ambiguity or confidence.

**Future direction:** Score-based classification where each matching rule
contributes a weighted score. The category with the highest total score wins.
Custom rules get higher weight than defaults. This defers the "which category
is right?" question until all evidence is collected rather than short-circuiting
on the first match.

## Problem 12: Data Portability and Export

**Current state:** Data is stored as JSON files in a proprietary directory
layout. No import or export capability.

**Impact:** Users can't migrate to/from other tools, can't back up to
standard formats, can't feed data into external analysis tools.

**Future direction:** Export commands for common formats:
- CSV export (accounts, transactions, balance history)
- OFX/QFX import (bank downloads)
- Ledger/hledger/beancount format (see Problem 8)
- JSON export with documented schema (already partially done via `query`)

## Problem 13: Storage Scaling

**Current state:** All data is in flat JSON files. Every read/write loads
the entire file into memory.

**Impact:** Works fine for typical personal finance (thousands of
transactions). Would struggle with decades of history or very active
accounts. Transaction dedup is O(n) per account.

**Future direction:** SQLite backend as an alternative to JSON files. The
`Storage` trait already abstracts the backend, so adding a SQLite
implementation is non-disruptive. JSON stays as the default for simplicity
and portability; SQLite is opt-in for users who need performance.

## Implementation Priority

Ordered by impact-to-effort ratio:

1. **Anomaly thresholds in config** -- Small, self-contained, immediately
   useful. Good first PR.
2. **Currency symbol from account data** -- Quick win in `format_currency()`.
3. ~~**Spending presets**~~ -- **DONE.** Spending patterns now stored in
   `spending_patterns.json` in the data directory. Seeded from defaults on
   first use. Editable via `spending-rules` CLI subcommand.
4. **Account classification presets** -- Same pattern as spending. Heuristic
   becomes fallback-only.
5. **User-defined account categories** -- The big refactor. Requires touching
   every layer.
6. **Multi-currency net worth** -- Group by currency, optional exchange rates.
7. **POS prefix normalization** -- Promote `normalize_merchant()` to shared
   utility.
8. **Amount-based classification signals** -- Extend SpendingRule with
   optional amount ranges.
9. **Score-based classification** -- Replace first-match-wins with weighted
   scoring.
10. **Data portability** -- CSV/OFX export, ledger format.
11. **Crate restructure** -- Separate engine from SimpleFIN connector.
12. **SQLite storage backend** -- Opt-in for users who need scale.
13. **Ledger export** -- New feature, not a fix, but high community value.
14. **i18n for text output** -- Low priority unless adoption demands it.
