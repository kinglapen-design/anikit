#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

COMMIT="31863c707501bf1605d36842f43deb78793dbc5d"
EXPECTED_SHA256="48b04d626eec503243e5af8fda5da1bd15ef066a51c43bc882b1ffb1f69ac750"
ARCHIVE="$SPIKE_ROOT/.cache/chinese-clip.tar.gz"
REPO="$SPIKE_ROOT/vendor/Chinese-CLIP"

if [[ -f "$REPO/cn_clip/deploy/pytorch_to_onnx.py" ]]; then
  echo "Official Chinese-CLIP source already present at $REPO"
  exit 0
fi

mkdir -p "$SPIKE_ROOT/vendor"
if [[ ! -f "$ARCHIVE" ]]; then
  curl --fail --location --retry 2 --output "$ARCHIVE" \
    "https://github.com/OFA-Sys/Chinese-CLIP/archive/$COMMIT.tar.gz"
fi
ACTUAL_SHA256="$(sha256sum "$ARCHIVE" | cut -d' ' -f1)"
if [[ "$ACTUAL_SHA256" != "$EXPECTED_SHA256" ]]; then
  echo "Source archive SHA256 mismatch: $ACTUAL_SHA256" >&2
  exit 1
fi

tar --force-local -xzf "$ARCHIVE" -C "$SPIKE_ROOT/vendor"
mv "$SPIKE_ROOT/vendor/Chinese-CLIP-$COMMIT" "$REPO"
