from __future__ import annotations

import os
from pathlib import Path
import runpy
import sys

from torch.onnx import register_custom_op_symbolic
from torch.onnx.symbolic_opset14 import scaled_dot_product_attention

register_custom_op_symbolic(
    "aten::scaled_dot_product_attention", scaled_dot_product_attention, 13
)

script = (
    Path(os.environ["SPIKE_ROOT"])
    / "vendor"
    / "Chinese-CLIP"
    / "cn_clip"
    / "deploy"
    / "pytorch_to_onnx.py"
)
sys.argv = [str(script), *sys.argv[1:]]
runpy.run_path(str(script), run_name="__main__")
