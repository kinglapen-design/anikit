#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

if [[ ! -x "$PYTHON" ]]; then
  BASELINE="$SPIKE_ROOT/../semantic-clip/.venv"
  if [[ -x "$BASELINE/Scripts/python.exe" ]]; then
    cp -a "$BASELINE" "$SPIKE_ROOT/.venv"
  else
    py -V:Astral/CPython3.12.14 -m venv "$SPIKE_ROOT/.venv"
  fi
fi

"$PYTHON" -m pip install --disable-pip-version-check --upgrade pip
"$PYTHON" -m pip install --disable-pip-version-check \
  --index-url https://download.pytorch.org/whl/cpu torch==2.6.0 torchvision==0.21.0
"$PYTHON" -m pip install --disable-pip-version-check \
  -r "$SPIKE_ROOT/scripts/requirements.txt"
"$PYTHON" -c "import sys, torch, transformers, onnx, onnxruntime, tokenizers; print(sys.version); print('torch', torch.__version__); print('transformers', transformers.__version__); print('onnx', onnx.__version__); print('onnxruntime', onnxruntime.__version__); print('tokenizers', tokenizers.__version__)"
