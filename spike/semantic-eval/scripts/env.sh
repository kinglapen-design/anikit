#!/usr/bin/env bash
set -euo pipefail

EVAL_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -W)"
CLIP_ROOT="$(cd "$EVAL_ROOT/../semantic-clip" && pwd -W)"
export EVAL_ROOT CLIP_ROOT
source "$CLIP_ROOT/scripts/env.sh"
mkdir -p "$EVAL_ROOT/.cache" "$EVAL_ROOT/data" "$EVAL_ROOT/logs" "$EVAL_ROOT/outputs" "$EVAL_ROOT/manifest"
