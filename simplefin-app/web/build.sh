#!/usr/bin/env bash
# Builds the browser version of simplefin-app into OUT_DIR.
# Needs: rustup target wasm32-unknown-unknown, wasm-bindgen-cli matching the
# wasm-bindgen version in Cargo.lock, and binaryen's wasm-opt.
set -euo pipefail
out=$(mkdir -p "${1:?usage: simplefin-app/web/build.sh <out dir>}" && cd "$1" && pwd)
root=$(cd "$(dirname "$0")/../.." && pwd)
cd "$root"
version=$(sed -n '/^name = "wasm-bindgen"$/{n;s/^version = "\(.*\)"$/\1/p;}' Cargo.lock)
wasm-bindgen --version | grep -qx "wasm-bindgen $version" || {
  echo "wasm-bindgen $version needed: cargo install wasm-bindgen-cli --version $version --locked" >&2
  exit 1
}
# gpui_web uses an unstable feature for its threads, as Zed's web examples do.
RUSTC_BOOTSTRAP=1 cargo build -p simplefin-app --lib --target wasm32-unknown-unknown --profile web
wasm="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/web/simplefin_app.wasm"
wasm-bindgen "$wasm" --out-dir "$out" --out-name simplefin_app --target web --no-typescript \
  --remove-name-section --remove-producers-section
# Optional size pass; an old wasm-opt that can't parse the module is skipped.
if command -v wasm-opt >/dev/null; then
  wasm-opt -Oz --enable-bulk-memory --enable-nontrapping-float-to-int --enable-sign-ext \
    --enable-mutable-globals --enable-reference-types --enable-multivalue \
    "$out/simplefin_app_bg.wasm" -o "$out/simplefin_app_bg.opt.wasm" \
    && mv "$out/simplefin_app_bg.opt.wasm" "$out/simplefin_app_bg.wasm" \
    || { rm -f "$out/simplefin_app_bg.opt.wasm"; echo "wasm-opt failed; keeping the unoptimized wasm" >&2; }
fi
build=$(date +%s)
sed "s/__BUILD__/$build/g" simplefin-app/web/index.html > "$out/index.html"
echo "built $(wc -c < "$out/simplefin_app_bg.wasm" | tr -d ' ') byte wasm into $out (build $build)"
