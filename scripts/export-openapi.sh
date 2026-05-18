#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/backend"
cargo run --quiet --bin export-openapi
echo "Wrote $ROOT/openapi/openapi.json"
