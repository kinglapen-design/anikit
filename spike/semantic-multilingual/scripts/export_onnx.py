from __future__ import annotations

import json
import os
from pathlib import Path
import shutil

import onnx
import torch
from transformers import AutoTokenizer

from modeling import CONTEXT_LENGTH, IMAGE_SIZE, load_text_tower, load_vision_tower

OPSET = 17


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    weights = root / "weights"
    models = root / "models"
    output = root / "outputs"
    models.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    torch.set_grad_enabled(False)
    torch.set_num_threads(4)

    official = onnx.load(weights / "text/onnx/model.onnx", load_external_data=False)
    official_outputs = [item.name for item in official.graph.output]
    if official_outputs != ["last_hidden_state"]:
        raise RuntimeError(f"Unexpected official ONNX outputs: {official_outputs}")
    del official

    tokenizer = AutoTokenizer.from_pretrained(weights / "text", local_files_only=True)
    if not tokenizer.is_fast:
        raise RuntimeError("A fast tokenizer is required for tokenizer.json parity")
    backend = tokenizer.backend_tokenizer
    backend.enable_truncation(max_length=CONTEXT_LENGTH)
    backend.enable_padding(
        length=CONTEXT_LENGTH,
        pad_id=tokenizer.pad_token_id,
        pad_token=tokenizer.pad_token,
    )
    tokenizer_path = models / "tokenizer.json"
    backend.save(str(tokenizer_path))
    shutil.copyfile(weights / "text/tokenizer_config.json", models / "tokenizer_config.json")
    shutil.copyfile(weights / "text/special_tokens_map.json", models / "special_tokens_map.json")

    text = load_text_tower(weights / "text")
    text_inputs = (
        torch.ones((1, CONTEXT_LENGTH), dtype=torch.int64),
        torch.ones((1, CONTEXT_LENGTH), dtype=torch.int64),
    )
    print("Exporting complete FP32 text tower", flush=True)
    torch.onnx.export(
        text,
        text_inputs,
        models / "text.onnx",
        input_names=["input_ids", "attention_mask"],
        output_names=["unnorm_text_features", "text_features"],
        opset_version=OPSET,
        do_constant_folding=True,
    )
    del text

    vision, clip = load_vision_tower(weights / "image")
    if clip.config.projection_dim != 512 or clip.config.vision_config.image_size != IMAGE_SIZE:
        raise RuntimeError("The selected image tower is not CLIP ViT-B/32 with 512-D projection")
    print("Exporting original OpenAI FP32 vision tower", flush=True)
    torch.onnx.export(
        vision,
        torch.zeros((1, 3, IMAGE_SIZE, IMAGE_SIZE), dtype=torch.float32),
        models / "vision.onnx",
        input_names=["pixel_values"],
        output_names=["unnorm_image_features", "image_features"],
        opset_version=OPSET,
        do_constant_folding=True,
    )
    del vision, clip

    for filename in ("text.onnx", "vision.onnx"):
        model = onnx.load(models / filename, load_external_data=False)
        onnx.checker.check_model(model)
        inputs = [item.name for item in model.graph.input]
        outputs = [item.name for item in model.graph.output]
        print(f"{filename}: inputs={inputs}, outputs={outputs}", flush=True)
        del model

    decision = {
        "official_text_onnx": "weights/text/onnx/model.onnx",
        "official_text_onnx_outputs": official_outputs,
        "reuse_decision": "re-exported because the official ONNX omits mean pooling and the 768-to-512 Dense projection",
        "text_input_shape": [1, CONTEXT_LENGTH],
        "image_input_shape": [1, 3, IMAGE_SIZE, IMAGE_SIZE],
        "dynamic_axes": False,
        "opset": OPSET,
        "precision": "FP32",
    }
    (output / "export-decision.json").write_text(
        json.dumps(decision, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
