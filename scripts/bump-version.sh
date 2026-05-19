#!/usr/bin/env bash
# Bump Shelfie version across Cargo, npm workspaces, OpenAPI, and package-lock.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

read_version() {
  grep -m1 '^version = ' backend/Cargo.toml | sed -E 's/^version = "(.*)"/\1/'
}

validate_semver() {
  if [[ ! "$1" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?(\+[0-9A-Za-z.-]+)?$ ]]; then
    echo "Invalid semver: $1 (expected X.Y.Z)" >&2
    exit 1
  fi
}

bump_semver() {
  local part="$1" ver="$2"
  local maj min pat
  IFS=. read -r maj min pat <<<"${ver%%-*}"
  pat="${pat%%[^0-9]*}"
  case "$part" in
    patch) pat=$((pat + 1)) ;;
    minor) min=$((min + 1)); pat=0 ;;
    major) maj=$((maj + 1)); min=0; pat=0 ;;
    *)
      echo "Unknown bump part: $part (use patch, minor, or major)" >&2
      exit 1
      ;;
  esac
  local suffix=""
  if [[ "$ver" == *-* ]]; then
    suffix="-${ver#*-}"
  fi
  echo "${maj}.${min}.${pat}${suffix}"
}

usage() {
  cat <<EOF
Usage: $(basename "$0") <version>
       $(basename "$0") patch|minor|major
       $(basename "$0") --show

Updates version in:
  - backend/Cargo.toml
  - package.json, frontend/package.json
  - package-lock.json (workspace entries)
  - backend/src/openapi.rs
  - openapi/openapi.json

Current version: $(read_version)
EOF
}

set_version() {
  local ver="$1"
  validate_semver "$ver"

  sed -i "s/^version = .*/version = \"${ver}\"/" backend/Cargo.toml

  python3 - <<PY
import json
from pathlib import Path

ver = "${ver}"
root = Path("${ROOT}")

for rel in ("package.json", "frontend/package.json"):
    p = root / rel
    data = json.loads(p.read_text(encoding="utf-8"))
    data["version"] = ver
    p.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

lock_path = root / "package-lock.json"
lock = json.loads(lock_path.read_text(encoding="utf-8"))
lock["version"] = ver
lock["packages"][""]["version"] = ver
lock["packages"]["frontend"]["version"] = ver
lock_path.write_text(json.dumps(lock, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
PY

  sed -i "s/^\([[:space:]]*version = \)\"[^\"]*\"/\1\"${ver}\"/" backend/src/openapi.rs

  python3 - <<PY
from pathlib import Path

p = Path("${ROOT}") / "openapi" / "openapi.json"
lines = p.read_text(encoding="utf-8").splitlines()
for i, line in enumerate(lines):
    if '"version":' in line and i < 20:
        indent = line[: line.index('"')]
        lines[i] = f'{indent}"version": "${ver}"'
        break
else:
    raise SystemExit("Could not find info.version in openapi/openapi.json")
text = "\n".join(lines)
if p.read_bytes().endswith(b"\n"):
    text += "\n"
p.write_text(text, encoding="utf-8")
PY

  echo "Version set to ${ver}"
  echo "  backend/Cargo.toml"
  echo "  package.json, frontend/package.json, package-lock.json"
  echo "  backend/src/openapi.rs, openapi/openapi.json"
}

if [[ $# -eq 0 ]] || [[ "${1:-}" == "-h" ]] || [[ "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

case "${1:-}" in
  --show)
    read_version
    exit 0
    ;;
  patch | minor | major)
    current="$(read_version)"
    set_version "$(bump_semver "$1" "$current")"
    ;;
  *)
    set_version "$1"
    ;;
esac
