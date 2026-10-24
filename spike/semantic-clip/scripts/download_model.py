from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

from huggingface_hub import hf_hub_download

REPO_ID = "OFA-Sys/chinese-clip-vit-base-patch16"
REVISION = "36e679e65c2a2fead755ae21162091293ad37834"
FILES = (
    "clip_cn_vit-b-16.pt",
    "vocab.txt",
    "config.json",
    "preprocessor_config.json",
    "README.md",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    hf_home = Path(os.environ["HF_HOME"]).resolve()
    if root not in hf_home.parents:
        raise RuntimeError(f"HF_HOME must be inside {root}: {hf_home}")
    destination = root / "weights" / "chinese-clip-vit-base-patch16"
    destination.mkdir(parents=True, exist_ok=True)
    downloaded = []
    for filename in FILES:
        value = hf_hub_download(
            repo_id=REPO_ID,
            filename=filename,
            revision=REVISION,
            local_dir=destination,
            local_dir_use_symlinks=False,
        )
        path = Path(value)
        record = {
            "path": str(path.relative_to(root)).replace("\\", "/"),
            "bytes": path.stat().st_size,
            "sha256": sha256(path),
        }
        downloaded.append(record)
        print(json.dumps(record, ensure_ascii=False), flush=True)
    report = {
        "repo_id": REPO_ID,
        "revision": REVISION,
        "license_status": "unconfirmed; internal spike only; do not redistribute",
        "files": downloaded,
    }
    output = root / "outputs"
    output.mkdir(parents=True, exist_ok=True)
    (output / "download-manifest.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
