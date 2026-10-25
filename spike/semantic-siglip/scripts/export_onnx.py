from __future__ import annotations

import json
import os
from pathlib import Path
import shutil

import onnx
import torch
from transformers import AutoTokenizer

from modeling import CONTEXT_LENGTH, IMAGE_SIZE, load_towers

OPSET = 17


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    weights = root / "weights/siglip2"
    models = root / "models"
    output = root / "outputs"
    models.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    torch.set_grad_enabled(False)
    torch.set_num_threads(4)

    tokenizer = AutoTokenizer.from_pretrained(weights, local_files_only=True)
    if not tokenizer.is_fast:
        raise RuntimeError("A fast tokenizer is required for tokenizer.json parity")
    backend = tokenizer.backend_tokenizer
    backend.enable_truncation(max_length=CONTEXT_LENGTH)
    backend.enable_padding(
        length=CONTEXT_LENGTH,
        pad_id=tokenizer.pad_token_id,
        pad_token=tokenizer.pad_token,
    )
    backend.save(str(models / "tokenizer.json"))
    shutil.copyfile(weights / "tokenizer_config.json", models / "tokenizer_config.json")
    shutil.copyfile(weights / "special_tokens_map.json", models / "special_tokens_map.json")

    text, vision, model = load_towers(weights)
    text_inputs = torch.ones((1, CONTEXT_LENGTH), dtype=torch.int64)
    print("Exporting complete FP32 text tower with final-token pooling and projection", flush=True)
    torch.onnx.export(
        text,
        text_inputs,
        models / "text.onnx",
        input_names=["input_ids"],
        output_names=["unnorm_text_features", "text_features"],
        opset_version=OPSET,
        do_constant_folding=True,
    )
    del text
    print("Exporting complete FP32 vision tower with attention pooling head", flush=True)
    torch.onnx.export(
        vision,
        torch.zeros((1, 3, IMAGE_SIZE, IMAGE_SIZE), dtype=torch.float32),
        models / "vision.onnx",
        input_names=["pixel_values"],
        output_names=["unnorm_image_features", "image_features"],
        opset_version=OPSET,
        do_constant_folding=True,
    )
    del vision, model

    for filename in ("text.onnx", "vision.onnx"):
        onnx_model = onnx.load(models / filename, load_external_data=False)
        onnx.checker.check_model(onnx_model)
        print(
            f"{filename}: inputs={[item.name for item in onnx_model.graph.input]}, "
            f"outputs={[item.name for item in onnx_model.graph.output]}",
            flush=True,
        )
        del onnx_model

    decision = {
        "official_onnx": "not present in the pinned repository file list",
        "text_pooling": "final LayerNorm -> last token -> Linear(768,768), matching SiglipModel.get_text_features",
        "vision_pooling": "SiglipMultiheadAttentionPoolingHead, matching SiglipModel.get_image_features",
        "embedding_dimension": 768,
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
