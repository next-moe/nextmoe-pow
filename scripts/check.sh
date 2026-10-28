#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

budget=${WASM_GZIP_BUDGET:-40960}

cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo run -q -p nextmoe-pow --example vectors | diff -u testdata/vectors.json - \
  || { echo "testdata/vectors.json is stale: regenerate it with the vectors example"; exit 1; }

(cd go && test -z "$(gofmt -l .)" && go vet ./... && go test ./...)

pnpm build:wasm
size=$(gzip -9c packages/solver/wasm/solver_bg.wasm | wc -c)
echo "solver_bg.wasm: ${size} bytes gzip (budget ${budget})"
[ "$size" -le "$budget" ]
pnpm build
pnpm typecheck
pnpm test
