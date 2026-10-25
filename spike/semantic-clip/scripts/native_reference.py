from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import time

import numpy as np
import onnxruntime as ort


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    output = root / "outputs"
    tensor_path = output / "native_image_input.f32"
    raw_input = np.fromfile(tensor_path, dtype="<f4")
    if raw_input.size != 1 * 3 * 224 * 224 or not np.isfinite(raw_input).all():
        raise RuntimeError(f"Invalid native tensor: {raw_input.size}")
    model_input = raw_input.reshape(1, 3, 224, 224)
    reference = json.loads((output / "reference.json").read_text(encoding="utf-8"))
    options = ort.SessionOptions()
    options.intra_op_num_threads = 4
    options.log_severity_level = 2
    session = ort.InferenceSession(
        str(root / "models" / "vision.onnx"), sess_options=options, providers=["CPUExecutionProvider"]
    )
    if session.get_providers() != ["CPUExecutionProvider"]:
        raise RuntimeError(f"Unexpected providers: {session.get_providers()}")
    start = time.perf_counter()
    raw = session.run(["unnorm_image_features"], {"image": model_input})[0]
    elapsed_ms = (time.perf_counter() - start) * 1000.0
    if raw.shape != (1, 512) or not np.isfinite(raw).all():
        raise RuntimeError(f"Invalid vision output: {raw.shape}")
    unit = raw / np.linalg.norm(raw, axis=-1, keepdims=True)
    text_units = {item["text"]: np.asarray(item["normalized"], dtype=np.float32) for item in reference["texts"]}
    similarities = {text: float(unit[0] @ text_unit) for text, text_unit in text_units.items()}
    logits = np.asarray([100.0 * similarities[text] for text in reference["official_candidates"]], dtype=np.float32)
    exponentials = np.exp(logits - logits.max())
    probabilities = (exponentials / exponentials.sum()).tolist()
    report = {
        "tensor_path": "outputs/native_image_input.f32",
        "tensor_sha256": hashlib.sha256(tensor_path.read_bytes()).hexdigest(),
        "tensor_source": "Rust native JPEG decode + direct 224x224 CatmullRom bicubic + RGB NCHW normalize",
        "python_providers": session.get_providers(),
        "python_vision_inference_ms": elapsed_ms,
        "image": {
            "unnormalized": raw[0].tolist(),
            "normalized": unit[0].tolist(),
        },
        "cosine_similarities": similarities,
        "official_candidate_probabilities": probabilities,
    }
    destination = output / "native-python-reference.json"
    destination.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "tensor_sha256": report["tensor_sha256"],
        "cosine_similarities": similarities,
        "official_candidate_probabilities": probabilities,
        "python_vision_inference_ms": elapsed_ms,
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
