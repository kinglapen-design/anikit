#!/usr/bin/env python3
"""逐像素反推参考波形（左图）的渲染算法：用同一音频跑候选算法，与参考 webp 比对。

用法：tools/.venv/Scripts/python tools/waveform-match.py
"""

import struct
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parent.parent
REF_WEBP = Path(r"E:/02_Assets/测试/asstes/AN2607262209-4LJN5.info/路人B_thumbnail.webp")
PCM = ROOT / "src-tauri" / "target" / "ref.pcm"

W, H = 640, 272
MARGIN = 10
BAR_A = 153


def load_pcm():
    data = PCM.read_bytes()
    n = len(data) // 4
    samples = struct.unpack(f"<{n}f", data)
    l = samples[0::2]
    r = samples[1::2]
    return l, r


def column_ranges(samples, sw):
    """每像素 1 窗口的 min/max 列表（0..1 归一）"""
    max_abs = max((abs(s) for s in samples), default=0.0) or 1.0
    step = len(samples) / sw
    cols = []
    for x in range(sw):
        s0 = int(x * step)
        s1 = max(int((x + 1) * step), s0 + 1)
        seg = samples[s0:s1] or (0.0,)
        cols.append((max(seg) / max_abs, min(seg) / max_abs))
    return cols


def render(cols_l, cols_r):
    """满色 1px 纵线（无柔化），返回 alpha 二维数组 [y][x]"""
    cy, max_half = H / 2, H / 2 - MARGIN
    img = [[0] * W for _ in range(H)]
    for x in range(W):
        for cols, top in ((cols_l, True), (cols_r, False)):
            hi, lo = cols[x]
            y0 = cy - hi * max_half
            y1 = cy - lo * max_half
            lb, hb = (cy - max_half, cy) if top else (cy, cy + max_half)
            y0, y1 = max(y0, lb), min(y1, hb)
            for y in range(int(y0), min(int(y1), H - 1) + 1):
                img[y][x] = BAR_A
    return img


def soften(img, mode):
    """max(原图, 均值模糊)：mode = 'box3' / 'vertical'"""
    out = [row[:] for row in img]
    for y in range(H):
        for x in range(W):
            if mode == "box3":
                vals = [img[yy][xx] for yy in range(max(0, y - 1), min(H, y + 2)) for xx in range(max(0, x - 1), min(W, x + 2))]
            else:
                vals = [img[yy][x] for yy in range(max(0, y - 1), min(H, y + 2))]
            m = max(img[y][x], sum(vals) // len(vals))
            if m:
                out[y][x] = m
    return out


def compare(candidate, ref_a, name):
    same = sum(1 for y in range(H) for x in range(W) if abs(candidate[y][x] - ref_a[y][x]) <= 25)
    print(f"{name}: 匹配 {same / (W * H):.1%}")


def main():
    ref = Image.open(REF_WEBP).convert("RGBA")
    ref_a = [[ref.getpixel((x, y))[3] for x in range(W)] for y in range(H)]
    l, r = load_pcm()
    base = render(column_ranges(l, W), column_ranges(r, W))
    compare(base, ref_a, "A 满色1px 无柔化")
    compare(soften(base, "box3"), ref_a, "B 满色+3x3柔化(c159e15)")
    compare(soften(base, "vertical"), ref_a, "C 满色+竖向柔化(c639e35)")


if __name__ == "__main__":
    main()
