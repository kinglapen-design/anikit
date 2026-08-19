#!/usr/bin/env python3
"""下载 ffmpeg / ffprobe sidecar 二进制（5.1 动图音视频解码底座）。

来源：BtbN/FFmpeg-Builds 的 win64 LGPL 静态构建（持续维护、LGPL 无 GPL 传染顾虑）。
产物：src-tauri/binaries/{ffmpeg,ffprobe}-x86_64-pc-windows-msvc.exe（Tauri externalBin 命名约定）。
二进制不入 git（.gitignore 已排除 src-tauri/binaries/）；首次开发 / 换机后运行一次：
    python tools/fetch-ffmpeg.py

--dev-lib（阶段四·自研解码器）：下载 ffmpeg-next 编译用的 lgpl-shared dev 库
（dll + MSVC .lib + 头文件）到 src-tauri/vendor/ffmpeg/。
版本锁 autobuild-2025-08-31（avcodec 62.13，ffmpeg 8.0 时期——与 ffmpeg-next 8.1.0 对齐；
最新 master 已 9.0-dev 会版本漂移，踩坑 038）。vendor/ 不入 git。
"""

import shutil
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

URL = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
DEV_LIB_URL = "https://github.com/BtbN/FFmpeg-Builds/releases/download/autobuild-2025-08-31-13-00/ffmpeg-N-120856-g9893d66add-win64-lgpl-shared.zip"
DEV_LIB_DIR = "ffmpeg-N-120856-g9893d66add-win64-lgpl-shared"
TRIPLE = "x86_64-pc-windows-msvc"
BINARIES = ("ffmpeg", "ffprobe")


def fetch_sidecar(root: Path) -> int:
    dest_dir = root / "src-tauri" / "binaries"
    dest_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory() as tmp:
        zip_path = Path(tmp) / "ffmpeg.zip"
        print(f"下载 {URL} ...")
        urllib.request.urlretrieve(URL, zip_path)
        print(f"下载完成（{zip_path.stat().st_size / 1024 / 1024:.1f} MB），解包 ...")

        with zipfile.ZipFile(zip_path) as zf:
            for name in BINARIES:
                member = next((m for m in zf.namelist() if m.endswith(f"bin/{name}.exe")), None)
                if member is None:
                    print(f"错误：压缩包内未找到 bin/{name}.exe", file=sys.stderr)
                    return 1
                dest = dest_dir / f"{name}-{TRIPLE}.exe"
                with zf.open(member) as src, dest.open("wb") as out:
                    shutil.copyfileobj(src, out)
                print(f"  {dest}（{dest.stat().st_size / 1024 / 1024:.1f} MB）")

    print("完成。dev / 构建均可直接使用。")
    return 0


def fetch_dev_lib(root: Path) -> int:
    dest_dir = root / "src-tauri" / "vendor"
    dest_dir.mkdir(parents=True, exist_ok=True)
    final = dest_dir / "ffmpeg"
    if (final / "bin").is_dir() and (final / "include").is_dir() and (final / "lib").is_dir():
        print(f"已存在 {final}（幂等跳过；要换版本先删目录）")
        return 0

    with tempfile.TemporaryDirectory() as tmp:
        zip_path = Path(tmp) / "ffmpeg-dev.zip"
        print(f"下载 {DEV_LIB_URL} ...")
        urllib.request.urlretrieve(DEV_LIB_URL, zip_path)
        print(f"下载完成（{zip_path.stat().st_size / 1024 / 1024:.1f} MB），解包 ...")

        with zipfile.ZipFile(zip_path) as zf:
            zf.extractall(tmp)
        extracted = Path(tmp) / DEV_LIB_DIR
        if not extracted.is_dir():
            print(f"错误：压缩包内未找到 {DEV_LIB_DIR}", file=sys.stderr)
            return 1
        if final.exists():
            shutil.rmtree(final)
        shutil.move(str(extracted), final)
        # 精简：exe 用不上（库级调用），doc/share 不需要
        for exe in ("ffmpeg.exe", "ffplay.exe", "ffprobe.exe"):
            p = final / "bin" / exe
            if p.exists():
                p.unlink()
        shutil.rmtree(final / "doc", ignore_errors=True)
        shutil.rmtree(final / "share", ignore_errors=True)

    print(f"完成。dev 库就位 {final}（FFMPEG_DIR 已由 src-tauri/.cargo/config.toml 指向）")
    return 0


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    if "--dev-lib" in sys.argv:
        return fetch_dev_lib(root)
    return fetch_sidecar(root)


if __name__ == "__main__":
    sys.exit(main())

