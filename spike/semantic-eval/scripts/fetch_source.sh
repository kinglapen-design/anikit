#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

URL="https://rail-berkeley.github.io/bridgedata/traj_links.csv"
EXPECTED="a9c971ba2dbd3b5f55fa091bf1873f26d1ded6fd06d3808f9497f68bb6729567"
DESTINATION="$EVAL_ROOT/.cache/traj_links.csv"
if [[ ! -f "$DESTINATION" ]]; then
  curl --fail --location --retry 2 --output "$DESTINATION" "$URL"
fi
ACTUAL="$(sha256sum "$DESTINATION" | cut -d' ' -f1)"
if [[ "$ACTUAL" != "$EXPECTED" ]]; then
  echo "traj_links.csv SHA256 mismatch: $ACTUAL" >&2
  exit 1
fi
"$PYTHON" "$EVAL_ROOT/scripts/source_stats.py"
