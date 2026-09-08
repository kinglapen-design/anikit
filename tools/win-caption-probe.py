# 实机走查第 17 条候选方案探针（踩坑 063 手法）——玻璃+caption 原生按钮根除方案实证
#   背景：DWM 层 headless 截不到，必须实机。对活窗口逐个应用候选改动 + mss 截屏 +
#   PIL 裁剪存证（人眼判读），每轮拍完恢复 063 基线（margins -1 / 原样式）。
#   候选：C1 = 玻璃 {10000,10000,1,10000}（顶 1px——caption 按钮需要 caption 高顶玻璃，
#   1px 画不下即不画；月牙缝由左右大 margins 侧向兜住）；C2 = 摘 WS_SYSMENU|MIN|MAXIMIZEBOX
#   保 WS_CAPTION（按钮 glyph 来源全摘，框架样式不动）。
#   输出：tools/style-preview/caption-probe/{C0,C0m,C1,C1m,C2,C2m}/{tr,top,left,br,tb}.png
# 前置：target\debug\app.exe 实机跑着（063 基线代码，窗口标题 "AniKit"）。
# 用法：tools/.venv/Scripts/python.exe tools/win-caption-probe.py
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

OUT = "tools/style-preview/caption-probe"
PAD = 14  # 截图外扩（窗外桌面 / 阴影 / 白环入图）

u = ctypes.windll.user32
dwm = ctypes.windll.dwmapi
# Per-Monitor V2：GetWindowRect / mss 坐标 = 物理像素
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))

GWL_STYLE = -16
WS_CAPTION = 0x00C00000
WS_SYSMENU = 0x00080000
WS_MINIMIZEBOX = 0x00020000
WS_MAXIMIZEBOX = 0x00010000
SWP = 0x0002 | 0x0001 | 0x0004 | 0x0008 | 0x0020  # NOMOVE|NOSIZE|NOZORDER|NOACTIVATE|FRAMECHANGED


class MARGINS(ctypes.Structure):
    _fields_ = [("cxLeftWidth", ctypes.c_int), ("cxRightWidth", ctypes.c_int),
                ("cyTopHeight", ctypes.c_int), ("cyBottomHeight", ctypes.c_int)]


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
        raise RuntimeError("找不到标题为 AniKit 的可见窗口")
    return found[0]


def set_margins(hwnd, l, r, t, b):
    m = MARGINS(l, r, t, b)
    hr = dwm.DwmExtendFrameIntoClientArea(wintypes.HWND(hwnd), ctypes.byref(m))
    print(f"  DwmExtendFrameIntoClientArea({l},{r},{t},{b}) = {hr:#x}")


def get_style(hwnd) -> int:
    return u.GetWindowLongPtrW(hwnd, GWL_STYLE)


def set_style(hwnd, style: int):
    u.SetWindowLongPtrW(hwnd, GWL_STYLE, style)
    u.SetWindowPos(hwnd, 0, 0, 0, 0, 0, SWP)


def win_rect(hwnd):
    r = wintypes.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(r))
    return r.left, r.top, r.right, r.bottom


def shots(sct, hwnd, tag: str):
    """整窗外扩截屏 + 五部位裁剪存证"""
    os.makedirs(f"{OUT}/{tag}", exist_ok=True)
    l, t, r, b = win_rect(hwnd)
    box = (max(0, l - PAD), max(0, t - PAD), r + PAD, b + PAD)
    img = Image.frombytes("RGB", (box[2] - box[0], box[3] - box[1]), sct.grab(box).rgb)
    img.save(f"{OUT}/{tag}/full.png")
    w, h = r - l, b - t
    ox, oy = PAD, PAD  # 窗口左上在 img 内的偏移
    crops = {
        "tr": (ox + w - 300, oy, ox + w + PAD, oy + 100),          # 右上按钮区
        "top": (ox, oy - PAD, ox + w, oy + 16),                     # 顶边细条/环
        "left": (ox - PAD, oy, ox + 16, oy + 420),                  # 左边环/阴影
        "br": (ox + w - 90, oy + h - 90, ox + w + PAD, oy + h + PAD),  # 右下角/环/阴影
        "tb": (ox + w // 2 - 210, oy, ox + w // 2 + 210, oy + 50),  # 标题栏中部（云母）
    }
    for name, (x0, y0, x1, y1) in crops.items():
        img.crop((max(0, x0), max(0, y0), min(img.width, x1), min(img.height, y1))).save(f"{OUT}/{tag}/{name}.png")
    print(f"  [{tag}] rect={l},{t},{r},{b} style={get_style(hwnd):#x} zoomed={u.IsZoomed(hwnd)}")


def main():
    hwnd = find_main_window()
    print(f"hwnd={hwnd}")
    style0 = get_style(hwnd)
    u.ShowWindow(hwnd, 9)  # SW_RESTORE：先回常态
    time.sleep(1.0)

    with mss.mss() as sct:
        print("== C0 基线（063 终态：caption + 玻璃-1）常态 ==")
        shots(sct, hwnd, "C0")
        print("== C0m 基线 最大化 ==")
        u.ShowWindow(hwnd, 3)  # SW_MAXIMIZE
        time.sleep(1.2)
        shots(sct, hwnd, "C0m")
        u.ShowWindow(hwnd, 9)
        time.sleep(1.0)

        print("== C1 玻璃 {10000,10000,1,10000} 常态 ==")
        set_margins(hwnd, 10000, 10000, 1, 10000)
        time.sleep(0.6)
        shots(sct, hwnd, "C1")
        print("== C1m 最大化 ==")
        u.ShowWindow(hwnd, 3)
        time.sleep(1.2)
        shots(sct, hwnd, "C1m")
        u.ShowWindow(hwnd, 9)
        time.sleep(1.0)

        print("== C2 摘 SYSMENU|MIN|MAX 保 CAPTION（玻璃回 -1）常态 ==")
        set_margins(hwnd, -1, -1, -1, -1)
        set_style(hwnd, style0 & ~(WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX))
        time.sleep(0.6)
        shots(sct, hwnd, "C2")
        print("== C2m 最大化 ==")
        u.ShowWindow(hwnd, 3)
        time.sleep(1.2)
        shots(sct, hwnd, "C2m")
        u.ShowWindow(hwnd, 9)
        time.sleep(1.0)

        print("== 恢复基线 ==")
        set_style(hwnd, style0)
        set_margins(hwnd, -1, -1, -1, -1)
        print(f"  style={get_style(hwnd):#x}（原 {style0:#x}）")


if __name__ == "__main__":
    main()
