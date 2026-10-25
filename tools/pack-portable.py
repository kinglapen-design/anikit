# 组装 + 打包 AniKit 免安装包（既定流程：tauri build --no-bundle 产物 → portable 目录 → zip）
# 用法：tools/.venv/Scripts/python.exe tools/pack-portable.py [YYYYMMDD]
# 来源：开发日志 2026-09-26_19 / 2026-10-03_09 既定流程
import io
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
OUT_DIR = ROOT / "release" / "anikit-portable-new"
ZIP_ROOT_NAME = "anikit-portable"  # zip 包内根目录名（用户既定口径）


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
