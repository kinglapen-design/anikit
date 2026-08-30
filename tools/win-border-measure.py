# 实机窗口边框像素测量（踩坑 063 手法）——主窗口 DWM 边框 + CSS 描边的物理像素分布量化
#   背景：CSS border 1px 在分数 DPR（150% → 1.5 物理 px）下光栅化拆成全实列+半透列，
#   半透列透云母亮色 = "缝隙"观感；相位随窗口尺寸变 → 粗细跳变。headless 截不到 DWM 层，
#   必须实机：mss 截主屏物理像素 + PIL 剖线读像素。
#   输出：四边中点垂直剖线（窗外 6px → 窗内 18px，逐列 RGB + 与边框色 #333 的 maxdiff）、
#   四角 20×20 NEAREST ×8 放大图；MoveWindow 驱动 4 个连续宽度各测一轮（测完恢复原几何）。
# 前置：pnpm tauri dev 实机跑着（窗口标题 "AniKit"）。
# 用法：tools/.venv/Scripts/python.exe tools/win-border-measure.py [hwnd]
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

BORDER = (51, 51, 51)  # --n-g6 #333333 = DWMWA_BORDER_COLOR 染色值
OUT = "tools/style-preview/border-measure"
PAD = 10           # 截图外扩（窗外背景也入图）
EDGE_OUT, EDGE_IN = 6, 18  # 剖线范围：窗外 6px → 窗内 18px

u = ctypes.windll.user32
# Per-Monitor V2：GetWindowRect / mss 坐标 = 物理像素（默认 DPI unaware 会被虚拟化）
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))


def find_main_window() -> int:
    found = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, lp):
        if u.IsWindowVisible(hwnd):
            n = u.GetWindowTextLengthW(hwnd)
            if n:
                buf = ctypes.create_unicode_buffer(n + 1)
                u.GetWindowTextW(hwnd, buf, n + 1)
                if buf.value == "AniKit":
                    found.append(hwnd)
        return True

    u.EnumWindows(cb, 0)
    if not found:
        raise RuntimeError("找不到标题为 AniKit 的可见窗口——先起 pnpm tauri dev")
    return found[0]


def bring_front(hwnd: int):
    u.keybd_event(0x12, 0, 0, 0)  # ALT 点按解锁 SetForegroundWindow 权限
    u.keybd_event(0x12, 0, 2, 0)
    u.ShowWindow(hwnd, 9)  # SW_RESTORE
    u.SetForegroundWindow(hwnd)
    time.sleep(0.5)


def win_rect(hwnd: int):
    r = wintypes.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(r))
    return r.left, r.top, r.right, r.bottom


def grab_region(left, top, width, height) -> Image.Image:
    with mss.MSS() as sct:
        raw = sct.grab({"left": left, "top": top, "width": width, "height": height})
        return Image.frombytes("RGB", raw.size, raw.bgra, "raw", "BGRX")


def maxdiff(px):
    return max(abs(px[i] - BORDER[i]) for i in range(3))


def scan(tag: str, img: Image.Image, rect, fails_note: list):
    """四边中点剖线 + 四角放大图。img 原点 = (rect.left-PAD, rect.top-PAD)。"""
    l, t, r, b = rect
    w, h = r - l, b - t
    dpi_scale = 144 / 96  # 打印用
    print(f"\n== {tag}  物理 rect=({l},{t},{r},{b})  {w}x{h}（逻辑 ≈ {w/dpi_scale:.2f}x{h/dpi_scale:.2f} @150%）")

    edges = {
        # 边名: (起点 x,y（窗口外 EDGE_OUT 处）, 步进 dx,dy, 中点描述)
        "left":   ((PAD - EDGE_OUT, PAD + h // 2), (1, 0)),
        "right":  ((PAD + w + EDGE_OUT - 1, PAD + h // 2), (-1, 0)),
        "top":    ((PAD + w // 2, PAD - EDGE_OUT), (0, 1)),
        "bottom": ((PAD + w // 2, PAD + h + EDGE_OUT - 1), (0, -1)),
    }
    for name, ((sx, sy), (dx, dy)) in edges.items():
        print(f"  -- {name} 边中点剖线（off 0 = 窗口物理边缘，负 = 窗外，正 = 窗内）")
        for i in range(EDGE_OUT + EDGE_IN):
            off = i - EDGE_OUT  # 相对窗口边缘
            if name == "left":
                x, y = sx + i, sy
            elif name == "right":
                x, y = sx - i, sy
            elif name == "top":
                x, y = sx, sy + i
            else:
                x, y = sx, sy - i
            px = img.getpixel((x, y))
            d = maxdiff(px)
            mark = ""
            if d <= 8:
                mark = " ← 边框色全实"
            elif d <= 60:
                mark = " ← 半透/混合"
            print(f"     off={off:+3d}  rgb{px}  Δ{d:3d}{mark}")

    os.makedirs(OUT, exist_ok=True)
    # 锚点修正：GetWindowRect 含 DWM 阴影外延（实测左/右/下 ≈9px、上 0px @150%），
    # 可视角 = rect 角内缩阴影带；40×40 区以可视角为中心
    SH = 9
    corners = {
        "TL": (PAD + SH, PAD),
        "TR": (PAD + w - 1 - SH, PAD),
        "BL": (PAD + SH, PAD + h - 1 - SH),
        "BR": (PAD + w - 1 - SH, PAD + h - 1 - SH),
    }
    SPAN = 40
    for cn, (cx, cy) in corners.items():
        x0 = max(cx - SPAN // 2, 0)
        y0 = max(cy - SPAN // 2, 0)
        crop = img.crop((x0, y0, x0 + SPAN, y0 + SPAN))
        big = crop.resize((SPAN * 8, SPAN * 8), Image.NEAREST)
        p = os.path.join(OUT, f"corner-{cn}-{tag}.png")
        big.save(p)
    print(f"  四角放大图 → {OUT}/corner-*-{tag}.png")


def measure(hwnd: int, tag: str):
    bring_front(hwnd)
    l, t, r, b = win_rect(hwnd)
    img = grab_region(l - PAD, t - PAD, (r - l) + 2 * PAD, (b - t) + 2 * PAD)
    scan(tag, img, (l, t, r, b), [])


def main():
    hwnd = int(sys.argv[1]) if len(sys.argv) > 1 else find_main_window()
    dpi = u.GetDpiForWindow(hwnd)
    print(f"hwnd={hwnd}  dpi={dpi}（缩放 {dpi/96:.0%}）")
    l0, t0, r0, b0 = win_rect(hwnd)
    w0, h0 = r0 - l0, b0 - t0
    print(f"原始几何：({l0},{t0}) {w0}x{h0} 物理")

    try:
        measure(hwnd, f"w{w0}")
        for dw in (1, 2, 3):
            u.MoveWindow(hwnd, l0, t0, w0 + dw, h0, True)
            time.sleep(0.6)  # 等重绘
            measure(hwnd, f"w{w0 + dw}")
    finally:
        u.MoveWindow(hwnd, l0, t0, w0, h0, True)  # 恢复原几何（几何持久化防抖末值 = 原尺寸）
        time.sleep(0.3)
        print(f"\n已恢复原几何 {w0}x{h0}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
