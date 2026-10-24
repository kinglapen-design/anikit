#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/env.sh"

REPO="$SPIKE_ROOT/vendor/Chinese-CLIP"
CHECKPOINT="$SPIKE_ROOT/weights/chinese-clip-vit-base-patch16/clip_cn_vit-b-16.pt"
export PYTHONPATH="$REPO;$REPO/cn_clip"
mkdir -p "$SPIKE_ROOT/models"

"$PYTHON" "$SPIKE_ROOT/scripts/official_export.py" \
  --model-arch ViT-B-16 \
  --pytorch-ckpt-path "$CHECKPOINT" \
  --save-onnx-path "$SPIKE_ROOT/models/vit-b-16" \
  --context-length 52 \
  --convert-text \
  --convert-vision

mv -f "$SPIKE_ROOT/models/vit-b-16.txt.fp32.onnx" "$SPIKE_ROOT/models/text.onnx"
mv -f "$SPIKE_ROOT/models/vit-b-16.img.fp32.onnx" "$SPIKE_ROOT/models/vision.onnx"
"$PYTHON" -c "import onnx, pathlib; root=pathlib.Path(r'$SPIKE_ROOT/models'); [(onnx.checker.check_model(str(root / name)), print(name, (root / name).stat().st_size)) for name in ('text.onnx', 'vision.onnx')]"
