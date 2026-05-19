#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
cargo run --quiet -p shelfie-backend --bin export-openapi
echo "Wrote $ROOT/openapi/openapi.json"
