from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import time

import numpy as np
import onnx
import onnxruntime as ort
from PIL import Image
import torch
from tokenizers import Tokenizer
from transformers import AutoTokenizer, CLIPImageProcessor

from modeling import (
    CONTEXT_LENGTH,
    EMBEDDING_DIM,
    IMAGE_SIZE,
    load_text_tower,
    load_vision_tower,
)

TEXTS = ["机器人洗碗", "a diagram", "a dog", "a cat"]
CANDIDATES = TEXTS[1:]
TEXT_REPO = "sentence-transformers/clip-ViT-B-32-multilingual-v1"
TEXT_REVISION = "58edf8cada9e398793dca955574a48cbb7f18be2"
IMAGE_REPO = "openai/clip-vit-base-patch32"
IMAGE_REVISION = "3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268"
OPENAI_REVISION = "d05afc436d78f1c48dc0dbf8e5980a9d471f35f6"


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def file_record(root: Path, path: Path) -> dict[str, object]:
    return {
        "path": str(path.relative_to(root)).replace("\\", "/"),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
    }


def elapsed(start: float) -> float:
    return round((time.perf_counter() - start) * 1000.0, 3)


def max_diff(left: np.ndarray, right: np.ndarray) -> float:
    return float(np.max(np.abs(left - right)))


def assert_vector(raw: np.ndarray, unit: np.ndarray) -> float:
    if raw.shape != (1, EMBEDDING_DIM) or unit.shape != (1, EMBEDDING_DIM):
        raise RuntimeError(f"Unexpected output shapes: {raw.shape}, {unit.shape}")
    if not np.isfinite(raw).all() or not np.isfinite(unit).all():
        raise RuntimeError("Non-finite embedding")
    norm = float(np.linalg.norm(unit[0]))
    if abs(norm - 1.0) > 1e-5:
        raise RuntimeError(f"Invalid normalized L2 norm: {norm}")
    return norm


def make_session(path: Path) -> tuple[ort.InferenceSession, float]:
    options = ort.SessionOptions()
    options.log_severity_level = 2
    options.intra_op_num_threads = 4
    options.graph_optimization_level = ort.GraphOptimizationLevel.ORT_ENABLE_ALL
    start = time.perf_counter()
    session = ort.InferenceSession(
        str(path), sess_options=options, providers=["CPUExecutionProvider"]
    )
    if session.get_providers() != ["CPUExecutionProvider"]:
        raise RuntimeError(f"Unexpected Python providers: {session.get_providers()}")
    return session, elapsed(start)


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    weights = root / "weights"
    models = root / "models"
    output = root / "outputs"
    output.mkdir(parents=True, exist_ok=True)
    torch.set_grad_enabled(False)
    torch.set_num_threads(4)

    tokenizer = AutoTokenizer.from_pretrained(weights / "text", local_files_only=True)
    rust_tokenizer = Tokenizer.from_file(str(models / "tokenizer.json"))
    text_tower = load_text_tower(weights / "text")
    vision_tower, clip = load_vision_tower(weights / "image")
    text_session, text_load_ms = make_session(models / "text.onnx")
    vision_session, vision_load_ms = make_session(models / "vision.onnx")
    timings: dict[str, object] = {
        "python_text_session_load_ms": text_load_ms,
        "python_vision_session_load_ms": vision_load_ms,
        "python_text_inference_ms": [],
    }

    text_records = []
    normalized_text: dict[str, np.ndarray] = {}
    for text in TEXTS:
        official = tokenizer(
            text,
            max_length=CONTEXT_LENGTH,
            padding="max_length",
            truncation=True,
            return_tensors="pt",
        )
        ids = list(rust_tokenizer.encode(text).ids)
        official_ids = official["input_ids"][0].tolist()
        if ids != official_ids or len(ids) != CONTEXT_LENGTH:
            raise RuntimeError(f"Token IDs differ for {text!r}: {ids} != {official_ids}")
        attention_mask = official["attention_mask"][0].tolist()
        if list(rust_tokenizer.encode(text).attention_mask) != attention_mask:
            raise RuntimeError(f"Attention mask differs for {text!r}")
        with torch.inference_mode():
            expected_raw, expected_unit = text_tower(
                official["input_ids"], official["attention_mask"]
            )
        start = time.perf_counter()
        raw, unit = text_session.run(
            ["unnorm_text_features", "text_features"],
            {"input_ids": official["input_ids"].numpy(), "attention_mask": official["attention_mask"].numpy()},
        )
        timings["python_text_inference_ms"].append(elapsed(start))
        norm = assert_vector(raw, unit)
        raw_diff = max_diff(raw, expected_raw.numpy())
        unit_diff = max_diff(unit, expected_unit.numpy())
        if raw_diff > 2e-4 or unit_diff > 2e-4:
            raise RuntimeError(f"Text ONNX/PyTorch mismatch for {text!r}: {raw_diff}, {unit_diff}")
        normalized_text[text] = unit[0]
        text_records.append({
            "text": text,
            "token_ids": ids,
            "attention_mask": attention_mask,
            "shape": list(unit.shape),
            "unnormalized": raw[0].tolist(),
            "normalized": unit[0].tolist(),
            "normalized_l2": norm,
            "pytorch_max_abs_diff_unnormalized": raw_diff,
            "pytorch_max_abs_diff_normalized": unit_diff,
        })

    processor = CLIPImageProcessor.from_pretrained(weights / "image", local_files_only=True)
    image_path = root / "assets/CLIP.png"
    image = Image.open(image_path).convert("RGB")
    image_input = processor(images=image, return_tensors="pt")["pixel_values"]
    if tuple(image_input.shape) != (1, 3, IMAGE_SIZE, IMAGE_SIZE):
        raise RuntimeError(f"Unexpected image input shape: {image_input.shape}")
    image_input = image_input.contiguous()
    raw_input_path = output / "image_input.f32"
    image_input.numpy().astype(np.float32, copy=False).tofile(raw_input_path)
    with torch.inference_mode():
        expected_raw, expected_unit = vision_tower(image_input)
        official_image_features = clip.get_image_features(pixel_values=image_input)
    if not torch.allclose(expected_raw, official_image_features, atol=1e-6, rtol=1e-6):
        raise RuntimeError("Vision wrapper differs from CLIPModel.get_image_features")
    start = time.perf_counter()
    raw_image, unit_image = vision_session.run(
        ["unnorm_image_features", "image_features"], {"pixel_values": image_input.numpy()}
    )
    timings["python_vision_inference_ms"] = elapsed(start)
    image_norm = assert_vector(raw_image, unit_image)
    raw_diff = max_diff(raw_image, expected_raw.numpy())
    unit_diff = max_diff(unit_image, expected_unit.numpy())
    if raw_diff > 2e-4 or unit_diff > 2e-4:
        raise RuntimeError(f"Vision ONNX/PyTorch mismatch: {raw_diff}, {unit_diff}")

    similarities = {text: float(unit_image[0] @ normalized_text[text]) for text in TEXTS}
    logit_scale = float(clip.logit_scale.exp().item())
    candidate_logits = np.asarray([logit_scale * similarities[text] for text in CANDIDATES])
    candidate_probabilities = torch.softmax(torch.from_numpy(candidate_logits), dim=0).tolist()
    best = CANDIDATES[int(np.argmax(candidate_probabilities))]
    if best != "a diagram":
        raise RuntimeError(f"Unexpected official sample classification: {candidate_probabilities}")

    reference = {
        "text_model": TEXT_REPO,
        "text_model_revision": TEXT_REVISION,
        "image_model": IMAGE_REPO,
        "image_model_revision": IMAGE_REVISION,
        "openai_source_revision": OPENAI_REVISION,
        "context_length": CONTEXT_LENGTH,
        "python_providers": {
            "text": text_session.get_providers(),
            "vision": vision_session.get_providers(),
        },
        "texts": text_records,
        "image": {
            **file_record(root, image_path),
            "source_url": f"https://raw.githubusercontent.com/openai/CLIP/{OPENAI_REVISION}/CLIP.png",
            "license": "MIT (OpenAI CLIP repository notice retained)",
            "input_tensor": file_record(root, raw_input_path),
            "input_shape": list(image_input.shape),
            "input_layout": "little-endian float32 NCHW; official CLIPImageProcessor resize/center-crop/normalize",
            "shape": list(unit_image.shape),
            "unnormalized": raw_image[0].tolist(),
            "normalized": unit_image[0].tolist(),
            "normalized_l2": image_norm,
            "pytorch_max_abs_diff_unnormalized": raw_diff,
            "pytorch_max_abs_diff_normalized": unit_diff,
        },
        "cosine_similarities": similarities,
        "official_candidates": CANDIDATES,
        "official_candidate_probabilities": candidate_probabilities,
        "logit_scale": logit_scale,
        "official_sample_best": best,
        "timings_ms": timings,
    }
    (output / "reference.json").write_text(
        json.dumps(reference, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    generated = [models / "text.onnx", models / "vision.onnx", models / "tokenizer.json"]
    manifest = {
        "text_model": TEXT_REPO,
        "text_model_revision": TEXT_REVISION,
        "text_license": "apache-2.0",
        "image_model": IMAGE_REPO,
        "image_model_revision": IMAGE_REVISION,
        "image_license_evidence": "OpenAI official CLIP repository MIT; HF mirror has no license tag",
        "openai_source_revision": OPENAI_REVISION,
        "context_length": CONTEXT_LENGTH,
        "embedding_dimension": EMBEDDING_DIM,
        "image_shape": [1, 3, IMAGE_SIZE, IMAGE_SIZE],
        "precision": "FP32",
        "files": [file_record(root, path) for path in generated],
        "checkpoints": [
            file_record(root, weights / "text/model.safetensors"),
            file_record(root, weights / "text/2_Dense/model.safetensors"),
            file_record(root, weights / "image/pytorch_model.bin"),
        ],
        "onnx": {},
    }
    for kind, filename in (("text", "text.onnx"), ("vision", "vision.onnx")):
        model = onnx.load(models / filename, load_external_data=False)
        manifest["onnx"][kind] = {
            "ir_version": model.ir_version,
            "opsets": {item.domain: item.version for item in model.opset_import},
            "inputs": [{
                "name": item.name,
                "element_type": item.type.tensor_type.elem_type,
                "shape": [dim.dim_value for dim in item.type.tensor_type.shape.dim],
            } for item in model.graph.input],
            "outputs": [{
                "name": item.name,
                "element_type": item.type.tensor_type.elem_type,
                "shape": [dim.dim_value for dim in item.type.tensor_type.shape.dim],
            } for item in model.graph.output],
        }
        del model
    (output / "model-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps({
        "token_ids": {record["text"]: record["token_ids"] for record in text_records},
        "text_l2": {record["text"]: record["normalized_l2"] for record in text_records},
        "image_l2": image_norm,
        "cosine_similarities": similarities,
        "official_candidate_probabilities": candidate_probabilities,
        "official_sample_best": best,
        "timings_ms": timings,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
