#!/usr/bin/env bash
# Start pelisearch-server on a dynamic port for SDK integration tests.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="$ROOT/target/debug/pelisearch-server"
DATA_DIR="${1:-$(mktemp -d)}"
PORT="${PELISEARCH_TEST_PORT:-0}"

if [[ ! -x "$BIN" ]]; then
  echo "Building pelisearch-server..."
  cargo build -p pelisearch-server --manifest-path "$ROOT/Cargo.toml"
fi

pick_port() {
  python3 - <<'PY'
import socket
s = socket.socket()
s.bind(("127.0.0.1", 0))
print(s.getsockname()[1])
s.close()
PY
}

if [[ "$PORT" == "0" ]]; then
  PORT="$(pick_port)"
fi

export PELISEARCH_TEST_DATA_DIR="$DATA_DIR"
export PELISEARCH_TEST_PORT="$PORT"
export PELISEARCH_TEST_URL="http://127.0.0.1:${PORT}"

echo "Starting server on $PELISEARCH_TEST_URL (data: $DATA_DIR)"
"$BIN" --port "$PORT" --data-path "$DATA_DIR" &
SERVER_PID=$!
export PELISEARCH_TEST_PID="$SERVER_PID"

cleanup() {
  if kill -0 "$SERVER_PID" 2>/dev/null; then
    kill "$SERVER_PID" 2>/dev/null || true
    wait "$SERVER_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

for _ in $(seq 1 100); do
  if curl -sf "$PELISEARCH_TEST_URL/health" >/dev/null; then
    echo "Server ready (pid=$SERVER_PID)"
    exit 0
  fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then
    echo "Server exited unexpectedly"
    exit 1
  fi
  sleep 0.1
done

echo "Server did not become ready in time"
exit 1
