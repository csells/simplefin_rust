# simplefin-app

A personal finance app on SimpleFIN data, written once in Rust with
[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) and the
[Ely](https://elygpui.com/) component library. The same code runs in a phone
or desktop browser (WebAssembly), on Android, and on desktop (macOS, Linux,
Windows). Its features follow Empower's
Personal Dashboard; see [`specs/empower-parity.md`](../specs/empower-parity.md)
for the feature catalog and the priority tiers this build follows.

## What it does

| Tab | Features |
|---|---|
| Home | Net worth with 1-day/90-day change and history chart, budget pace, this month's cash flow, upcoming recurring charges, stale-balance and anomaly alerts, recent activity. Syncs on launch when the last sync is over 8 hours old. |
| Accounts | Accounts grouped Cash / Investments / Credit Cards / Loans / Other Assets with group totals; account detail with available balance, balance history chart and its transactions. |
| Activity | Search, account-type filter, 30-day / 90-day / 1-year / all ranges; transaction detail with rename, recategorize (optionally as a rule for the merchant), and hide-duplicate. |
| Plan | Monthly budget (user limit or suggested 3-month average) with pace, last-month comparison, 3/12-month averages and top categories; month-by-month income vs spending; recurring subscriptions and bills. |
| More | Sync now, dark mode, disconnect, recent problems from the error log, data location. |

Setup takes a SimpleFIN Bridge setup token (claimed once) or an existing access
URL; the Connect screen links to bridge.simplefin.org to create one. The access
URL is stored owner-only in the app's private data directory, or in the
browser's `localStorage` for the web build.

## Architecture

- All finance logic lives in the `simplefin` library; the app only renders it.
- `store.rs` — `FinanceStore`, the single owner of app data. Views observe it
  and re-render on notify. Snapshot loads run on GPUI's background executor.
- `jobs.rs` — claiming setup tokens and collecting accounts through the
  library's transport-independent pipeline: natively over the library's
  Asupersync client on a dedicated thread (`background.rs`), in a browser over
  GPUI's `fetch` client (SimpleFIN Bridge allows CORS).
- `env.rs`, `files.rs`, `credentials.rs` — per-platform storage: disk natively,
  `localStorage` in a browser, all behind the library's `FileStore`.
- `snapshot.rs` — an immutable, fully computed view of local storage for
  rendering (`Snapshot::load`).
- `actions.rs` — user writes (transaction edits, rules, budget).
- `error_log.rs` — every user-visible failure is also appended to
  `app_errors.jsonl` in the data directory and to the platform log.
- `screens/` — one module per tab plus pushed detail pages; `nav.rs` draws a
  bottom bar below 720 px and a sidebar above.
- `android.rs` / `web.rs` / `main.rs` — Android (`android_main`), browser
  (`#[wasm_bindgen(start)]`) and desktop entry points.

Data: Android app-internal storage (`files/simplefin`); desktop
`$SIMPLEFIN_DATA` (shared with the CLI) or the platform data dir (`SimpleFIN`);
web, the origin's `localStorage` (nothing is stored server-side).

## Dependencies that need attention

- **Ely** is patched in the workspace `Cargo.toml` to the `mobile-targets`
  branch of [csells/Ely-GPUI-Components](https://github.com/csells/Ely-GPUI-Components),
  a fork of upstream Ely with one commit that moves Ely's `gpui_platform`
  dependency off Android/iOS (which have no GPUI desktop backend). `Cargo.lock`
  pins the exact commit. Once upstream accepts the change, delete the
  `[patch]` and run `cargo update -p ely-gpui-component`.
- **gpui-mobile** comes from `longbridge/gpui-mobile` at a pinned rev; it is the
  fork that tracks the same `gpui-pre 0.3.8` snapshot as Ely.

## Building for the web

Requires the `wasm32-unknown-unknown` target, `wasm-bindgen-cli` at the
`wasm-bindgen` version in `Cargo.lock`, and optionally a current `wasm-opt`
(binaryen).

```bash
simplefin-app/web/build.sh <out-dir>   # builds with the `web` profile, runs wasm-bindgen + wasm-opt
```

Serve `<out-dir>` over HTTP(S) as static files (`.wasm` as `application/wasm`);
precompressing with gzip roughly halves the download. GPUI's web platform needs
`RUSTC_BOOTSTRAP=1` (the script sets it). The page runs single-threaded, uses
WebGPU where available and WebGL2 otherwise.

Testing note: Chrome's device-scale emulation (`deviceScaleFactor` > 1) does not
scale `devicePixelContentBox`, so every GPUI web app, Ely's gallery included,
renders shrunken under emulated high-DPI; test at scale 1 or on a real device.

## Building for Android

Requires Rust with the `aarch64-linux-android` target, the Android SDK
(platform 35, build-tools 35), and NDK 27.

```bash
cargo ndk -t arm64-v8a -P 31 build -p simplefin-app --lib   # or plain cargo with an NDK linker
mkdir -p simplefin-app/android/gradle/app/src/main/jniLibs/arm64-v8a
cp target/aarch64-linux-android/debug/libsimplefin_app.so \
   simplefin-app/android/gradle/app/src/main/jniLibs/arm64-v8a/
cd simplefin-app/android/gradle && ./gradlew assembleDebug
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

Use API level 31+ for the native build (`aws-lc-sys` needs `getentropy`).

## Desktop

```bash
cargo run -p simplefin-app
```

## Tests

```bash
cargo test -p simplefin-app --lib
```

## Known gaps

- Keys injected with `adb shell input text` (and possibly hardware keyboards)
  don't reach GPUI text fields through gpui-mobile; the on-screen keyboard works.
  The IME's Enter/action key doesn't submit; use the Connect button.
- gpui-mobile's `Platform::open_url` is a stub on Android, so links go through
  its `url_launcher` package (`ui::open_external`); not yet exercised on a device.
- The web build keeps all data in `localStorage` (about 5 MB per origin), which
  can fill up with years of transactions.
- Content isn't yet padded for the on-screen keyboard; screens scroll instead.
- Toasts anchor to the window edge and can overlap Android's navigation bar.
- Planner-tier Empower features (retirement planner, holdings, fee analyzer)
  need data SimpleFIN doesn't provide; see the parity spec's "Later" tier.
- iOS isn't wired up yet (gpui-mobile supports it).
