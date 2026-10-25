#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

if [[ ! -x "$PYTHON" ]]; then
  echo "Read-only Python environment not found: $PYTHON" >&2
  exit 1
fi

"$PYTHON" -c "import sys, torch, transformers, onnx, onnxruntime, tokenizers; print(sys.version); print('torch', torch.__version__); print('transformers', transformers.__version__); print('onnx', onnx.__version__); print('onnxruntime', onnxruntime.__version__); print('tokenizers', tokenizers.__version__)"
