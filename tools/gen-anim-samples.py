#!/usr/bin/env python3
"""生成动图测试样本（tools/test-assets/，git 入库，供 cargo test 用）：

- anim3.gif  / anim3.webp：3 帧 64x48（红/绿/蓝），每帧 200ms，循环播放
- still.webp：单帧静态 WEBP（动图判定必须返回 false）

用法：tools/.venv/Scripts/python tools/gen-anim-samples.py（Pillow 在 tools/.venv）
"""

from pathlib import Path

from PIL import Image

OUT = Path(__file__).resolve().parent / "test-assets"
OUT.mkdir(exist_ok=True)

COLORS = [(220, 40, 40, 255), (40, 200, 40, 255), (40, 40, 220, 255)]
FRAMES = [Image.new("RGBA", (64, 48), c) for c in COLORS]


def main() -> None:
    FRAMES[0].save(
        OUT / "anim3.gif",
        save_all=True,
        append_images=FRAMES[1:],
        duration=200,
        loop=0,
    )
    FRAMES[0].save(
        OUT / "anim3.webp",
        save_all=True,
        append_images=FRAMES[1:],
        duration=200,
        loop=0,
    )
    FRAMES[0].save(OUT / "still.webp")
    for f in sorted(OUT.glob("anim3.*")) + [OUT / "still.webp"]:
        print(f"{f.name}: {f.stat().st_size}B")


if __name__ == "__main__":
    main()
