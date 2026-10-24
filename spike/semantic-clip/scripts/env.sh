#!/usr/bin/env bash
set -euo pipefail

SPIKE_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -W)"
export SPIKE_ROOT
export HF_HOME="$SPIKE_ROOT/.cache/huggingface"
export HF_HUB_CACHE="$HF_HOME/hub"
export HUGGINGFACE_HUB_CACHE="$HF_HUB_CACHE"
export HF_HUB_DISABLE_TELEMETRY=1
export TORCH_HOME="$SPIKE_ROOT/.cache/torch"
export XDG_CACHE_HOME="$SPIKE_ROOT/.cache"
export PIP_CACHE_DIR="$SPIKE_ROOT/.cache/pip"
export UV_CACHE_DIR="$SPIKE_ROOT/.cache/uv"
export CARGO_HOME="$SPIKE_ROOT/.cargo"
export CARGO_TARGET_DIR="$SPIKE_ROOT/rust/target"
export TMPDIR="$SPIKE_ROOT/.cache/tmp"
export TMP="$TMPDIR"
export TEMP="$TMPDIR"
export PYTHONNOUSERSITE=1
export PYTHONDONTWRITEBYTECODE=1

mkdir -p "$HF_HUB_CACHE" "$TORCH_HOME" "$PIP_CACHE_DIR" "$UV_CACHE_DIR" \
  "$CARGO_HOME" "$CARGO_TARGET_DIR" "$TMPDIR"

PYTHON="$SPIKE_ROOT/.venv/Scripts/python.exe"
export PYTHON
