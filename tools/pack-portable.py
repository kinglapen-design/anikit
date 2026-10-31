# 组装 + 打包 AniKit 免安装包（既定流程：tauri build --no-bundle 产物 → portable 目录 → zip）
# 用法：tools/.venv/Scripts/python.exe tools/pack-portable.py YYYYMMDD [输出目录名]
# 来源：开发日志 2026-09-26_19 / 2026-10-03_09 既定流程
import hashlib
import io
import json
import shutil
import subprocess
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "src-tauri" / "target" / "release"
VENDOR_BIN = ROOT / "src-tauri" / "vendor" / "ffmpeg" / "bin"
BINARIES = ROOT / "src-tauri" / "binaries"
PLUGINS = ROOT / "plugins" / "animate-cep"
OBSIDIAN_PLUGIN = ROOT / "plugins" / "obsidian-anikit-paste"  # 源目录可能暂缺，存在才拷
MODEL_SOURCE = ROOT / "assets" / "models" / "multilingual-v1"
MODEL_REVISION = "58edf8cada9e398793dca955574a48cbb7f18be2+3d74acf9a28c67741b2f4f2ea7635f0aaf6f0268"
OUT_NAME = sys.argv[2] if len(sys.argv) > 2 else "anikit-portable-new"
if Path(OUT_NAME).name != OUT_NAME or OUT_NAME in {"", ".", ".."}:
    raise SystemExit("输出目录名只能是 release 下的单个目录名")
OUT_DIR = ROOT / "release" / OUT_NAME
ZIP_ROOT_NAME = "anikit-portable"  # zip 包内根目录名（用户既定口径）


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def validate_model() -> bool:
    manifest_path = MODEL_SOURCE / "model-manifest.json"
    if not manifest_path.is_file():
        print(f"缺离线模型：{manifest_path}（先跑 python tools/prepare-semantic-model.py）")
        return False
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if f"{manifest['text_model_revision']}+{manifest['image_model_revision']}" != MODEL_REVISION:
        print("离线模型 revision 与后端固定版本不一致")
        return False
    records = {item["path"]: item for item in manifest["files"]}
    for name in ["text.onnx", "vision.onnx", "tokenizer.json"]:
        key = f"models/{name}"
        path = MODEL_SOURCE / key
        if not path.is_file() or sha256(path) != records[key]["sha256"]:
            print(f"离线模型校验失败：{key}")
            return False
    for name in ["Apache-2.0.txt", "OpenAI-CLIP-LICENSE.txt", "THIRD_PARTY_NOTICES.md"]:
        if not (MODEL_SOURCE / "notices" / name).is_file():
            print(f"缺模型许可文件：{name}")
            return False
    return True


def main() -> int:
    stamp = sys.argv[1] if len(sys.argv) > 1 else ""
    if not stamp:
        print("用法：pack-portable.py YYYYMMDD")
        return 2
    # 1. 产物齐备性检查
    app_exe = TARGET / "app.exe"
    app_lib = TARGET / "deps" / "app_lib.dll"
    paste_exe = TARGET / "anikit-paste.exe"  # 编辑器贴图直链小工具（src/bin/anikit-paste.rs）
    semantic_exe = TARGET / "semantic-worker.exe"  # 语义推理隔离进程（src/bin/semantic-worker.rs）
    for p in [app_exe, app_lib, paste_exe, semantic_exe]:
        if not p.is_file():
            print(f"缺产物：{p}（先跑 pnpm tauri build --no-bundle）")
            return 2
    semantic_runtime = BINARIES / "semantic-runtime"
    for name in ["onnxruntime.dll", "onnxruntime_providers_shared.dll", "DirectML.dll"]:
        if not (semantic_runtime / name).is_file():
            print(f"缺语义运行时：{semantic_runtime / name}（先跑 python tools/fetch-semantic-runtime.py）")
            return 2
    av_dlls = sorted(VENDOR_BIN.glob("*.dll"))
    if len(av_dlls) != 7:
        print(f"vendor av*.dll 数量异常：{len(av_dlls)}（应 7）")
        return 2
    for sidecar in ["ffmpeg", "ffprobe"]:
        if not (BINARIES / f"{sidecar}-x86_64-pc-windows-msvc.exe").is_file():
            print(f"缺 sidecar：{sidecar}（先跑 python tools/fetch-ffmpeg.py）")
            return 2
    if not validate_model():
        return 2

    # 2. 组装（OUT_DIR 整目录镜像 = 产物集）
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    shutil.copy2(app_exe, OUT_DIR / "anikit.exe")
    shutil.copy2(app_lib, OUT_DIR / "app_lib.dll")
    shutil.copy2(paste_exe, OUT_DIR / "anikit-paste.exe")
    shutil.copy2(semantic_exe, OUT_DIR / "semantic-worker.exe")
    for d in av_dlls:
        shutil.copy2(d, OUT_DIR / d.name)
    shutil.copy2(BINARIES / "ffmpeg-x86_64-pc-windows-msvc.exe", OUT_DIR / "ffmpeg.exe")
    shutil.copy2(BINARIES / "ffprobe-x86_64-pc-windows-msvc.exe", OUT_DIR / "ffprobe.exe")
    if (OUT_DIR / "semantic-runtime").exists():
        shutil.rmtree(OUT_DIR / "semantic-runtime")
    shutil.copytree(semantic_runtime, OUT_DIR / "semantic-runtime")
    if (OUT_DIR / "models").exists():
        shutil.rmtree(OUT_DIR / "models")
    model_out = OUT_DIR / "models" / "multilingual-v1" / MODEL_REVISION
    shutil.copytree(MODEL_SOURCE, model_out)
    # plugins/animate-cep 整树镜像
    if (OUT_DIR / "plugins" / "animate-cep").exists():
        shutil.rmtree(OUT_DIR / "plugins" / "animate-cep")
    shutil.copytree(PLUGINS, OUT_DIR / "plugins" / "animate-cep")
    # plugins/obsidian-anikit-paste 整树镜像（源目录可能暂缺——存在才拷）
    if OBSIDIAN_PLUGIN.is_dir():
        if (OUT_DIR / "plugins" / "obsidian-anikit-paste").exists():
            shutil.rmtree(OUT_DIR / "plugins" / "obsidian-anikit-paste")
        shutil.copytree(OBSIDIAN_PLUGIN, OUT_DIR / "plugins" / "obsidian-anikit-paste")
    total = sum(f.stat().st_size for f in OUT_DIR.rglob("*") if f.is_file())
    print(f"组装完成：{OUT_DIR}（{total / 1024 / 1024:.0f} MB）")

    # 3. zip（包内根 = anikit-portable）
    zip_path = ROOT / "release" / f"anikit-v0.1.0-{stamp}.zip"
    if zip_path.exists():
        zip_path.unlink()
    with zipfile.ZipFile(zip_path, "w", zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for f in sorted(OUT_DIR.rglob("*")):
            if f.is_file():
                z.write(f, f"{ZIP_ROOT_NAME}/{f.relative_to(OUT_DIR).as_posix()}")
    print(f"打包完成：{zip_path}（{zip_path.stat().st_size / 1024 / 1024:.0f} MB）")
    return 0


if __name__ == "__main__":
    sys.exit(main())
