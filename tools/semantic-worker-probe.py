import json
import math
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKER = Path(os.environ.get("ANIKIT_SEMANTIC_WORKER_EXE", str(ROOT / "src-tauri" / "target" / "debug" / "semantic-worker.exe")))
MODEL = Path(os.environ.get("ANIKIT_SEMANTIC_MODEL_DIR", str(ROOT / "spike" / "semantic-multilingual")))
RUNTIME = Path(os.environ.get("ANIKIT_SEMANTIC_RUNTIME_DIR", str(ROOT / "spike" / "semantic-clip" / "runtime")))
IMAGE = Path(os.environ.get("ANIKIT_SEMANTIC_TEST_IMAGE", str(ROOT / "spike" / "semantic-multilingual" / "assets" / "CLIP.png")))


def run_backend(backend: str) -> dict:
    command = [
        str(WORKER),
        "--runtime-dir", str(RUNTIME),
        "--text-model", str(MODEL / "models" / "text.onnx"),
        "--vision-model", str(MODEL / "models" / "vision.onnx"),
        "--tokenizer", str(MODEL / "models" / "tokenizer.json"),
        "--backend", backend,
    ]
    env = dict(os.environ)
    env["PYTHONIOENCODING"] = "utf-8"
    process = subprocess.Popen(
        command,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0),
        env=env,
    )
    try:
        ready = process.stdout.readline()
        if not ready:
            raise RuntimeError(process.stderr.read())
        assert json.loads(ready)["type"] == "ready", ready
        results = {}
        for request in [
            {"type": "embedText", "id": 1, "text": "机器人洗碗"},
            {"type": "embedImage", "id": 2, "path": str(IMAGE)},
        ]:
            process.stdin.write(json.dumps(request, ensure_ascii=False) + "\n")
            process.stdin.flush()
            line = process.stdout.readline()
            if not line:
                raise RuntimeError(process.stderr.read())
            response = json.loads(line)
            assert response["type"] == "result", response
            assert response["id"] == request["id"], response
            vector = response["embedding"]
            assert len(vector) == 512, len(vector)
            assert all(math.isfinite(value) for value in vector)
            norm = math.sqrt(sum(value * value for value in vector))
            assert abs(norm - 1.0) < 0.00001, norm
            results[request["type"]] = {
                "actualBackend": response["actualBackend"],
                "dimension": len(vector),
                "norm": norm,
            }
        process.stdin.write('{"type":"shutdown","id":3}\n')
        process.stdin.flush()
        process.wait(timeout=20)
        assert process.returncode == 0, process.returncode
        if backend == "directml":
            assert "directml" in results["embedText"]["actualBackend"].lower(), results
        else:
            assert results["embedText"]["actualBackend"] == "cpu", results
        return results
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=5)


def main() -> int:
    for path in [WORKER, RUNTIME / "onnxruntime.dll", MODEL / "models" / "text.onnx", MODEL / "models" / "vision.onnx", MODEL / "models" / "tokenizer.json", IMAGE]:
        if not path.exists():
            raise FileNotFoundError(path)
    report = {backend: run_backend(backend) for backend in ["cpu", "directml"]}
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
