#!/usr/bin/env bash
# S2 spike: which story-registration mechanism survives a wasm32 build?
# Every case expects 4 stories (checksum 131) registered across 3 crates.
set -uo pipefail
cd "$(dirname "$0")/../.."
T=wasm32-unknown-unknown
R=target/$T/release
D=target/$T/debug
run() { node spikes/s2-registry/runner/run.mjs "$1" "$2"; }

echo "=============================================================="
echo " S2 — story registration on wasm32-unknown-unknown"
echo "=============================================================="
echo
echo "### 1. linkme (link sections)"
touch spikes/s2-registry/core/src/lib.rs
LINKME_OUT=$(cargo build --release --target $T -p s2-wasmapp 2>&1 || true)
if grep -q 'not implemented for this platform' <<<"$LINKME_OUT"; then
  echo "  rustc: $(grep -m1 'not implemented for this platform' <<<"$LINKME_OUT")"
  echo "  VERDICT: UNUSABLE — linkme has no wasm32 support at all."
else
  echo "  VERDICT: unexpectedly compiled — re-check."
fi
echo "  (same code compiles fine for the host target:)"
cargo build --release -p s2-wasmapp >/dev/null 2>&1 && echo "  host build OK — so this is a platform gap, not a code bug." 
echo
echo "### 2. inventory (life-before-main ctors)"
cargo build --release --target $T -p s2-inv-wasmapp       >/dev/null 2>&1
cargo build           --target $T -p s2-inv-wasmapp       >/dev/null 2>&1
cargo build --release --target $T -p s2-inv-wasmapp-unref >/dev/null 2>&1
run $R/s2_inv_wasmapp.wasm       "inventory · RELEASE (lto, codegen-units=1)"
run $D/s2_inv_wasmapp.wasm       "inventory · DEBUG   (codegen-units=256, dx serve default)"
run $R/s2_inv_wasmapp_unref.wasm "inventory · RELEASE, component crate never referenced"
echo
echo "### 3. build-script codegen (explicit references)"
cargo build --release --target $T -p s2-cg-wasmapp >/dev/null 2>&1
cargo build           --target $T -p s2-cg-wasmapp >/dev/null 2>&1
run $R/s2_cg_wasmapp.wasm "codegen · RELEASE"
run $D/s2_cg_wasmapp.wasm "codegen · DEBUG (codegen-units=256)"
