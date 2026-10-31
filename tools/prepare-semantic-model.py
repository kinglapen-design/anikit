"""把已验证的多语 CLIP 权重整理为受管离线模型包；不下载、不移动源文件。"""
import hashlib
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SOURCE = ROOT / "spike" / "semantic-multilingual"
DEST = ROOT / "assets" / "models" / "multilingual-v1"
TEXT_REVISION = "58edf8cada9e398793dca955574a48cbb7f18be2"
IMAGE_REVISION = "3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268"
REQUIRED_MODEL_FILES = ["text.onnx", "vision.onnx", "tokenizer.json"]
REQUIRED_NOTICES = [
    "THIRD_PARTY_NOTICES.md",
    "Apache-2.0.txt",
    "OpenAI-CLIP-LICENSE.txt",
    "SentenceTransformers-clip-multilingual-README.md",
    "OpenAI-clip-hf-README.md",
    "OpenAI-CLIP-clip.py",
    "OpenAI-CLIP-model-card.md",
    "OpenAI-CLIP-README.md",
]


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def copy_file(source: Path, dest: Path) -> None:
    if not source.is_file():
        raise FileNotFoundError(source)
    dest.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, dest)


def main() -> int:
    source = Path(sys.argv[1]).resolve() if len(sys.argv) > 1 else SOURCE
    dest = Path(sys.argv[2]).resolve() if len(sys.argv) > 2 else DEST
    if source == dest:
        raise ValueError("源目录与模型包目录不能相同")
    manifest_path = source / "outputs" / "model-manifest.json"
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if (manifest["text_model_revision"], manifest["image_model_revision"]) != (TEXT_REVISION, IMAGE_REVISION):
        raise ValueError("模型 revision 与产品固定版本不一致")
    if manifest["embedding_dimension"] != 512 or manifest.get("precision") != "FP32":
        raise ValueError("只接受已定案的 512 维 FP32 模型")
    if manifest.get("text_license") != "apache-2.0" or "MIT" not in manifest.get("image_license_evidence", ""):
        raise ValueError("模型许可证据不完整")
    records = {entry["path"]: entry for entry in manifest["files"]}
    for name in REQUIRED_MODEL_FILES:
        key = f"models/{name}"
        path = source / key
        record = records[key]
        if path.stat().st_size != record["bytes"] or sha256(path) != record["sha256"]:
            raise ValueError(f"模型文件校验失败：{key}")
    for name in REQUIRED_NOTICES:
        if not (source / "notices" / name).is_file():
            raise FileNotFoundError(f"缺许可材料：{name}")
    for name in ["onnxruntime.dll", "onnxruntime_providers_shared.dll", "DirectML.dll"]:
        if not (source / "runtime" / name).is_file():
            raise FileNotFoundError(f"缺私有运行时：{name}")
    copy_file(manifest_path, dest / "model-manifest.json")
    for name in [*REQUIRED_MODEL_FILES, "tokenizer_config.json", "special_tokens_map.json"]:
        copy_file(source / "models" / name, dest / "models" / name)
    for path in (source / "notices").iterdir():
        if path.is_file():
            copy_file(path, dest / "notices" / path.name)
    for path in (source / "runtime" / "licenses").iterdir():
        if path.is_file():
            copy_file(path, dest / "runtime" / "licenses" / path.name)
    copy_file(source / "outputs" / "runtime-manifest.json", dest / "outputs" / "runtime-manifest.json")
    if (source / "assets" / "CLIP.png").is_file():
        copy_file(source / "assets" / "CLIP.png", dest / "assets" / "CLIP.png")
    for name in REQUIRED_MODEL_FILES:
        key = f"models/{name}"
        if sha256(dest / key) != records[key]["sha256"]:
            raise ValueError(f"模型包写入后校验失败：{key}")
    size = sum((dest / "models" / name).stat().st_size for name in REQUIRED_MODEL_FILES)
    print(f"离线模型包已校验：{dest}")
    print(f"模型必要文件：{size} bytes（{size / 1024 / 1024:.1f} MiB）；不含 spike 原始权重与评估集")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"模型包准备失败：{error}", file=sys.stderr)
        sys.exit(1)
