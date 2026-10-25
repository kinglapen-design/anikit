from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import urllib.request

from huggingface_hub import hf_hub_download

TEXT_REPO = "sentence-transformers/clip-ViT-B-32-multilingual-v1"
TEXT_REVISION = "58edf8cada9e398793dca955574a48cbb7f18be2"
IMAGE_REPO = "openai/clip-vit-base-patch32"
IMAGE_REVISION = "3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268"
OPENAI_REVISION = "d05afc436d78f1c48dc0dbf8e5980a9d471f35f6"

TEXT_FILES = (
    "README.md",
    "config.json",
    "config_sentence_transformers.json",
    "modules.json",
    "sentence_bert_config.json",
    "1_Pooling/config.json",
    "2_Dense/config.json",
    "2_Dense/model.safetensors",
    "model.safetensors",
    "onnx/model.onnx",
    "tokenizer.json",
    "tokenizer_config.json",
    "special_tokens_map.json",
    "vocab.txt",
)
IMAGE_FILES = (
    "README.md",
    "config.json",
    "preprocessor_config.json",
    "pytorch_model.bin",
)
OPENAI_FILES = {
    "LICENSE": "notices/OpenAI-CLIP-LICENSE.txt",
    "README.md": "notices/OpenAI-CLIP-README.md",
    "model-card.md": "notices/OpenAI-CLIP-model-card.md",
    "clip/clip.py": "notices/OpenAI-CLIP-clip.py",
    "CLIP.png": "assets/CLIP.png",
}


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
    request = urllib.request.Request(url, headers={"User-Agent": "semantic-multilingual-spike/1.0"})
    with urllib.request.urlopen(request) as response, partial.open("wb") as output:
        while chunk := response.read(8 * 1024 * 1024):
            output.write(chunk)
    partial.replace(destination)


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    hf_home = Path(os.environ["HF_HOME"]).resolve()
    if root not in hf_home.parents:
        raise RuntimeError(f"HF_HOME must be inside {root}: {hf_home}")

    downloads = []
    for repo, revision, files, folder in (
        (TEXT_REPO, TEXT_REVISION, TEXT_FILES, "text"),
        (IMAGE_REPO, IMAGE_REVISION, IMAGE_FILES, "image"),
    ):
        destination = root / "weights" / folder
        for filename in files:
            value = hf_hub_download(
                repo_id=repo,
                filename=filename,
                revision=revision,
                local_dir=destination,
            )
            item = record(
                root,
                Path(value),
                repo_id=repo,
                revision=revision,
                repo_path=filename,
                url=f"https://huggingface.co/{repo}/resolve/{revision}/{filename}",
            )
            downloads.append(item)
            print(json.dumps(item, ensure_ascii=False), flush=True)

    source_files = []
    for repo_path, relative_destination in OPENAI_FILES.items():
        url = f"https://raw.githubusercontent.com/openai/CLIP/{OPENAI_REVISION}/{repo_path}"
        destination = root / relative_destination
        download_url(url, destination)
        item = record(root, destination, url=url, revision=OPENAI_REVISION)
        source_files.append(item)
        print(json.dumps(item, ensure_ascii=False), flush=True)

    apache_url = "https://www.apache.org/licenses/LICENSE-2.0.txt"
    apache_path = root / "notices" / "Apache-2.0.txt"
    download_url(apache_url, apache_path)
    source_files.append(record(root, apache_path, url=apache_url))

    for source, destination_name, url in (
        (
            root / "weights/text/README.md",
            "SentenceTransformers-clip-multilingual-README.md",
            f"https://huggingface.co/{TEXT_REPO}/resolve/{TEXT_REVISION}/README.md",
        ),
        (
            root / "weights/image/README.md",
            "OpenAI-clip-hf-README.md",
            f"https://huggingface.co/{IMAGE_REPO}/resolve/{IMAGE_REVISION}/README.md",
        ),
    ):
        destination = root / "notices" / destination_name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(source.read_bytes())
        source_files.append(record(root, destination, url=url))

    report = {
        "text": {"repo_id": TEXT_REPO, "revision": TEXT_REVISION, "license": "apache-2.0"},
        "image": {
            "repo_id": IMAGE_REPO,
            "revision": IMAGE_REVISION,
            "license_evidence": "OpenAI official CLIP repository MIT; HF mirror has no license tag",
        },
        "openai_source_revision": OPENAI_REVISION,
        "downloads": downloads,
        "source_and_notice_files": source_files,
    }
    output = root / "outputs"
    output.mkdir(parents=True, exist_ok=True)
    (output / "download-manifest.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


if __name__ == "__main__":
    main()
