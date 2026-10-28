#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

port=${E2E_PORT:-18787}
(cd go && go build -o ../target/powdemo ./cmd/powdemo)
./target/powdemo -addr "127.0.0.1:${port}" -root . -results "$(mktemp -d)" &
server=$!
trap 'kill $server' EXIT
for _ in $(seq 50); do curl -sf "http://127.0.0.1:${port}/demo/" >/dev/null && break; sleep 0.1; done
POW_DEMO_URL="http://127.0.0.1:${port}" pnpm --filter @nextmoe/pow-widget test:e2e
