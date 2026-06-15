#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SDK_TESTS="$(cd "$(dirname "$0")" && pwd)"
DATA_DIR="$(mktemp -d)"
BIN="$ROOT/target/debug/pelisearch-server"

if [[ ! -x "$BIN" ]]; then
  echo "Building pelisearch-server..."
  cargo build -p pelisearch-server --manifest-path "$ROOT/Cargo.toml"
fi

PORT="$(python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
)"

export PELISEARCH_TEST_URL="http://127.0.0.1:${PORT}"
export PELISEARCH_TEST_PORT="$PORT"
export PELISEARCH_TEST_DATA_DIR="$DATA_DIR"

echo "Starting server on $PELISEARCH_TEST_URL"
"$BIN" --port "$PORT" --data-path "$DATA_DIR" &
SERVER_PID=$!

cleanup() {
  if kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
  rm -rf "$DATA_DIR"
}
trap cleanup EXIT

for _ in $(seq 1 100); do
  if curl -sf "$PELISEARCH_TEST_URL/health" >/dev/null; then
    echo "Server ready"
    break
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server exited unexpectedly"
    exit 1
  fi
  sleep 0.1
done

echo "==> JavaScript SDK integration tests"
cd "$ROOT/sdk/javascript"
npm run build
cd "$SDK_TESTS/javascript"
npm install --silent
npm test

echo "==> Python SDK integration tests"
cd "$ROOT/sdk/python"
if [[ ! -d .venv ]]; then
  python3 -m venv .venv
fi
# shellcheck disable=SC1091
source .venv/bin/activate
pip install -e ".[dev]" -q
cd "$SDK_TESTS/python"
python -m pytest -q

echo "==> Go SDK integration tests"
cd "$ROOT/sdk/go/pelisearch"
go test -v -count=1 .

echo "==> Rust SDK integration tests"
cd "$ROOT"
cargo test -p pelisearch --test integration -- --nocapture

echo "All SDK integration tests passed."
