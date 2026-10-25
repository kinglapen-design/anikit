from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import urllib.request

from huggingface_hub import hf_hub_download

REPO_ID = "google/siglip2-base-patch16-512"
REVISION = "a89f5c5093f902bf39d3cd4d81d2c09867f0724b"
OPENAI_REVISION = "d05afc436d78f1c48dc0dbf8e5980a9d471f35f6"
FILES = (
    "README.md",
    "config.json",
    "model.safetensors",
    "preprocessor_config.json",
    "special_tokens_map.json",
    "tokenizer.json",
    "tokenizer.model",
    "tokenizer_config.json",
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def record(root: Path, path: Path, **extra: object) -> dict[str, object]:
    return {
        "path": str(path.relative_to(root)).replace("\\", "/"),
        "bytes": path.stat().st_size,
        "sha256": sha256(path),
        **extra,
    }


def download_url(url: str, destination: Path) -> None:
    if destination.is_file():
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_suffix(destination.suffix + ".part")
    request = urllib.request.Request(url, headers={"User-Agent": "semantic-siglip-spike/1.0"})
    with urllib.request.urlopen(request) as response, partial.open("wb") as output:
        while chunk := response.read(8 * 1024 * 1024):
            output.write(chunk)
    partial.replace(destination)


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    hf_home = Path(os.environ["HF_HOME"]).resolve()
    if root not in hf_home.parents:
        raise RuntimeError(f"HF_HOME must be inside {root}: {hf_home}")
    destination = root / "weights" / "siglip2"
    downloaded = []
    for filename in FILES:
        value = hf_hub_download(
            repo_id=REPO_ID,
            filename=filename,
            revision=REVISION,
            local_dir=destination,
        )
        item = record(
            root,
            Path(value),
            repo_id=REPO_ID,
            revision=REVISION,
            repo_path=filename,
            url=f"https://huggingface.co/{REPO_ID}/resolve/{REVISION}/{filename}",
        )
        downloaded.append(item)
        print(json.dumps(item, ensure_ascii=False), flush=True)

    notices = root / "notices"
    notices.mkdir(parents=True, exist_ok=True)
    card = notices / "Google-SigLIP2-README.md"
    shutil.copyfile(destination / "README.md", card)
    apache_url = "https://www.apache.org/licenses/LICENSE-2.0.txt"
    apache = notices / "Apache-2.0.txt"
    download_url(apache_url, apache)

    baseline = root.parent / "semantic-multilingual"
    samples = []
    for source, relative_destination, url in (
        (
            baseline / "assets/CLIP.png",
            "assets/CLIP.png",
            f"https://raw.githubusercontent.com/openai/CLIP/{OPENAI_REVISION}/CLIP.png",
        ),
        (
            baseline / "notices/OpenAI-CLIP-LICENSE.txt",
            "notices/OpenAI-CLIP-LICENSE.txt",
            f"https://raw.githubusercontent.com/openai/CLIP/{OPENAI_REVISION}/LICENSE",
        ),
        (
            baseline / "notices/OpenAI-CLIP-README.md",
            "notices/OpenAI-CLIP-README.md",
            f"https://raw.githubusercontent.com/openai/CLIP/{OPENAI_REVISION}/README.md",
        ),
    ):
        target = root / relative_destination
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(source, target)
        samples.append(record(root, target, source_url=url, read_only_source=str(source)))

    report = {
        "repo_id": REPO_ID,
        "revision": REVISION,
        "license": "apache-2.0",
        "license_api": f"https://huggingface.co/api/models/{REPO_ID}",
        "downloads": downloaded,
        "notice_files": [
            record(root, card),
            record(root, apache, url=apache_url),
            *samples,
        ],
    }
    output = root / "outputs"
    output.mkdir(parents=True, exist_ok=True)
    (output / "download-manifest.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
