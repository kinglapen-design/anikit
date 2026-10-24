from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import sys
import time

import numpy as np
import onnxruntime as ort
from PIL import Image
from tokenizers import Tokenizer
from tokenizers.implementations import BertWordPieceTokenizer
from tokenizers.processors import TemplateProcessing
import torch

TEXTS = ["机器人洗碗", "杰尼龟", "妙蛙种子", "小火龙", "皮卡丘"]
OFFICIAL_CANDIDATES = TEXTS[1:]
CONTEXT_LENGTH = 52


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


def normalize(value: np.ndarray) -> np.ndarray:
    return value / np.linalg.norm(value, axis=-1, keepdims=True)


def elapsed(start: float) -> float:
    return round((time.perf_counter() - start) * 1000.0, 3)


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    repo = root / "vendor" / "Chinese-CLIP"
    sys.path.insert(0, str(repo / "cn_clip"))
    sys.path.insert(0, str(repo))
    import cn_clip.clip as clip
    from clip.utils import _MODEL_INFO, image_transform

    output = root / "outputs"
    models = root / "models"
    output.mkdir(parents=True, exist_ok=True)
    vocab = root / "weights" / "chinese-clip-vit-base-patch16" / "vocab.txt"
    if sha256(vocab) != sha256(repo / "cn_clip" / "clip" / "vocab.txt"):
        raise RuntimeError("HF vocab.txt differs from the official exporter vocab.txt")

    tokenizer = BertWordPieceTokenizer(vocab=str(vocab), lowercase=True, strip_accents=True)
    cls_id = tokenizer.token_to_id("[CLS]")
    sep_id = tokenizer.token_to_id("[SEP]")
    tokenizer.post_processor = TemplateProcessing(
        single="[CLS] $A [SEP]",
        pair="[CLS] $A [SEP] $B:1 [SEP]:1",
        special_tokens=[("[CLS]", cls_id), ("[SEP]", sep_id)],
    )
    tokenizer.enable_truncation(max_length=CONTEXT_LENGTH)
    tokenizer.enable_padding(length=CONTEXT_LENGTH, pad_id=0, pad_token="[PAD]")
    tokenizer_path = models / "tokenizer.json"
    tokenizer.save(str(tokenizer_path))
    tokenizer = Tokenizer.from_file(str(tokenizer_path))

    sessions = {}
    timings = {}
    for kind, filename in (("text", "text.onnx"), ("vision", "vision.onnx")):
        options = ort.SessionOptions()
        options.log_severity_level = 2
        start = time.perf_counter()
        session = ort.InferenceSession(
            str(models / filename), sess_options=options, providers=["CPUExecutionProvider"]
        )
        timings[f"python_{kind}_session_load_ms"] = elapsed(start)
        if session.get_providers() != ["CPUExecutionProvider"]:
            raise RuntimeError(f"Unexpected Python providers: {session.get_providers()}")
        sessions[kind] = session

    text_records = []
    normalized_text = {}
    for text in TEXTS:
        official_ids = clip.tokenize([text], context_length=CONTEXT_LENGTH)[0].tolist()
        encoding = tokenizer.encode(text)
        token_ids = list(encoding.ids)
        if token_ids != official_ids:
            raise RuntimeError(f"Token IDs differ for {text!r}: {token_ids} != {official_ids}")
        model_input = np.asarray([token_ids], dtype=np.int64)
        start = time.perf_counter()
        raw = sessions["text"].run(["unnorm_text_features"], {"text": model_input})[0]
        timings.setdefault("python_text_inference_ms", []).append(elapsed(start))
        if raw.shape != (1, 512) or not np.isfinite(raw).all():
            raise RuntimeError(f"Invalid text output for {text!r}: {raw.shape}")
        unit = normalize(raw)
        norm = float(np.linalg.norm(unit[0]))
        if abs(norm - 1.0) > 1e-5:
            raise RuntimeError(f"Invalid normalized text L2 norm: {norm}")
        normalized_text[text] = unit[0]
        text_records.append(
            {
                "text": text,
                "token_ids": token_ids,
                "shape": list(raw.shape),
                "unnormalized": raw[0].tolist(),
                "normalized": unit[0].tolist(),
                "normalized_l2": norm,
            }
        )

    image_path = repo / "examples" / "pokemon.jpeg"
    resolution = _MODEL_INFO["ViT-B-16"]["input_resolution"]
    preprocess = image_transform(resolution)
    image = preprocess(Image.open(image_path)).unsqueeze(0)
    image_input = np.ascontiguousarray(image.numpy(), dtype=np.float32)
    raw_input_path = output / "image_input.f32"
    image_input.tofile(raw_input_path)
    start = time.perf_counter()
    raw_image = sessions["vision"].run(["unnorm_image_features"], {"image": image_input})[0]
    timings["python_vision_inference_ms"] = elapsed(start)
    if raw_image.shape != (1, 512) or not np.isfinite(raw_image).all():
        raise RuntimeError(f"Invalid vision output: {raw_image.shape}")
    unit_image = normalize(raw_image)
    image_norm = float(np.linalg.norm(unit_image[0]))
    if abs(image_norm - 1.0) > 1e-5:
        raise RuntimeError(f"Invalid normalized vision L2 norm: {image_norm}")

    similarities = {
        text: float(unit_image[0] @ normalized_text[text]) for text in TEXTS
    }
    candidate_logits = np.asarray(
        [100.0 * similarities[text] for text in OFFICIAL_CANDIDATES], dtype=np.float32
    )
    candidate_probabilities = torch.softmax(torch.from_numpy(candidate_logits), dim=0).tolist()
    if OFFICIAL_CANDIDATES[int(np.argmax(candidate_probabilities))] != "皮卡丘":
        raise RuntimeError(f"Unexpected official sample classification: {candidate_probabilities}")

    reference = {
        "model": "OFA-Sys/chinese-clip-vit-base-patch16",
        "model_revision": "36e679e65c2a2fead755ae21162091293ad37834",
        "source_revision": "31863c707501bf1605d36842f43deb78793dbc5d",
        "context_length": CONTEXT_LENGTH,
        "python_providers": {name: session.get_providers() for name, session in sessions.items()},
        "texts": text_records,
        "image": {
            **file_record(root, image_path),
            "input_tensor": file_record(root, raw_input_path),
            "input_shape": list(image_input.shape),
            "input_layout": "little-endian float32 NCHW, official image_transform bicubic resize and CLIP normalization",
            "shape": list(raw_image.shape),
            "unnormalized": raw_image[0].tolist(),
            "normalized": unit_image[0].tolist(),
            "normalized_l2": image_norm,
        },
        "cosine_similarities": similarities,
        "official_candidates": OFFICIAL_CANDIDATES,
        "official_candidate_probabilities": candidate_probabilities,
        "official_deployment_probabilities": [0.0012252, 0.052874, 0.00067116, 0.94523],
        "timings_ms": timings,
    }
    (output / "reference.json").write_text(
        json.dumps(reference, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )

    generated = [models / "text.onnx", models / "vision.onnx", tokenizer_path]
    manifest = {
        "model": reference["model"],
        "model_revision": reference["model_revision"],
        "source_revision": reference["source_revision"],
        "license_status": "unconfirmed; internal spike only; do not redistribute",
        "context_length": CONTEXT_LENGTH,
        "embedding_dimension": 512,
        "image_shape": [1, 3, resolution, resolution],
        "precision": "FP32",
        "files": [file_record(root, path) for path in generated],
        "checkpoint": file_record(
            root, root / "weights" / "chinese-clip-vit-base-patch16" / "clip_cn_vit-b-16.pt"
        ),
        "onnx": {},
    }
    import onnx

    for kind, filename in (("text", "text.onnx"), ("vision", "vision.onnx")):
        model = onnx.load(str(models / filename), load_external_data=False)
        manifest["onnx"][kind] = {
            "ir_version": model.ir_version,
            "opsets": {item.domain: item.version for item in model.opset_import},
            "inputs": [
                {"name": item.name, "shape": [dim.dim_value for dim in item.type.tensor_type.shape.dim]}
                for item in model.graph.input
            ],
            "outputs": [
                {"name": item.name, "shape": [dim.dim_value for dim in item.type.tensor_type.shape.dim]}
                for item in model.graph.output
            ],
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
        "timings_ms": timings,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
