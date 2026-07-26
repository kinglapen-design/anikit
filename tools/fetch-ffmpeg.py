#!/usr/bin/env python3
"""下载 ffmpeg / ffprobe sidecar 二进制（5.1 动图音视频解码底座）。

来源：BtbN/FFmpeg-Builds 的 win64 LGPL 静态构建（持续维护、LGPL 无 GPL 传染顾虑）。
产物：src-tauri/binaries/{ffmpeg,ffprobe}-x86_64-pc-windows-msvc.exe（Tauri externalBin 命名约定）。
二进制不入 git（.gitignore 已排除 src-tauri/binaries/）；首次开发 / 换机后运行一次：
    python tools/fetch-ffmpeg.py
"""

import shutil
import sys
import tempfile
import urllib.request
import zipfile
from pathlib import Path

URL = "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
TRIPLE = "x86_64-pc-windows-msvc"
BINARIES = ("ffmpeg", "ffprobe")


def main() -> int:
    root = Path(__file__).resolve().parent.parent
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


if __name__ == "__main__":
    sys.exit(main())
