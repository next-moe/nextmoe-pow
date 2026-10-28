#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."

port=${E2E_PORT:-18788}
cargo build --release -p nextmoe_pow_flutter
(cd go && go build -o ../target/powdemo ./cmd/powdemo)
./target/powdemo -addr "127.0.0.1:${port}" -root . -results "$(mktemp -d)" &
server=$!
trap 'kill $server' EXIT
for _ in $(seq 50); do curl -sf "http://127.0.0.1:${port}/demo/" >/dev/null && break; sleep 0.1; done

cd flutter
flutter pub get
flutter analyze
POW_DEMO_URL="http://127.0.0.1:${port}" flutter test
(cd example && flutter pub get && flutter analyze)
