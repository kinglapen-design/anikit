# 实机走查第 17 条探针四轮（终版）——前台断言 + 关键状态重测（063 手法）
#   一至三轮教训：mss 截的是合成屏，未做前台断言，截到的可能是上层别的窗口（Animate），
#   结论全部存疑。四轮每拍必做：ALT 解锁 + SetForegroundWindow + GetForegroundWindow==hwnd
#   校验（3 次重试），确保截的就是 AniKit 本体。
#   状态：B0 常态-1 / B0m 最大化-1（复现原生按钮）/ B1 常态玻璃0 / B1m 最大化玻璃0
#   （按钮是否灭、黑底范围）/ B2 常态 {10000,10000,0,10000}（顶 0 杀细条）。拍完恢复。
#   输出：tools/style-preview/caption-probe4/{B0,B0m,B1,B1m,B2}/{tr,top,tb,br,full}.png
# 用法：tools/.venv/Scripts/python.exe tools/win-caption-probe4.py
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

OUT = "tools/style-preview/caption-probe4"
PAD = 14

u = ctypes.windll.user32
dwm = ctypes.windll.dwmapi
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))


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


def front(hwnd) -> bool:
    """置前并校验（3 次重试）；GetForegroundWindow 是线程态，ALT 点按解锁权限"""
    for i in range(3):
        u.keybd_event(0x12, 0, 0, 0)
        u.keybd_event(0x12, 0, 2, 0)
        u.SetForegroundWindow(hwnd)
        time.sleep(0.5)
        if u.GetForegroundWindow() == hwnd:
            return True
        print(f"  置前重试 {i + 1}")
    print("  !! 置前失败，截图可能不是 AniKit")
    return False


def set_margins(hwnd, l, r, t, b):
    m = MARGINS(l, r, t, b)
    hr = dwm.DwmExtendFrameIntoClientArea(wintypes.HWND(hwnd), ctypes.byref(m))
    print(f"  DwmExtendFrameIntoClientArea({l},{r},{t},{b}) = {hr:#x}")


def win_rect(hwnd):
    r = wintypes.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(r))
    return r.left, r.top, r.right, r.bottom


def shots(sct, hwnd, tag):
    os.makedirs(f"{OUT}/{tag}", exist_ok=True)
    l, t, r, b = win_rect(hwnd)
    box = (max(0, l - PAD), max(0, t - PAD), r + PAD, b + PAD)
    img = Image.frombytes("RGB", (box[2] - box[0], box[3] - box[1]), sct.grab(box).rgb)
    img.save(f"{OUT}/{tag}/full.png")
    w, h = r - l, b - t
    ox, oy = l - box[0], t - box[1]
    crops = {
        "tr": (ox + w - 300, oy, ox + w + PAD, oy + 100),
        "top": (ox, oy - PAD, ox + w, oy + 16),
        "tb": (ox + w // 2 - 210, oy, ox + w // 2 + 210, oy + 50),
        "br": (ox + w - 90, oy + h - 90, ox + w + PAD, oy + h + PAD),
    }
    for name, (x0, y0, x1, y1) in crops.items():
        img.crop((max(0, x0), max(0, y0), min(img.width, x1), min(img.height, y1))).save(f"{OUT}/{tag}/{name}.png")
    fg = u.GetForegroundWindow() == hwnd
    print(f"  [{tag}] rect={l},{t},{r},{b} zoomed={u.IsZoomed(hwnd)} foreground={fg}")


def main():
    hwnd = find_main_window()
    print(f"hwnd={hwnd}")
    u.ShowWindow(hwnd, 9)
    time.sleep(0.8)

    with mss.MSS() as sct:
        print("== B0 常态 玻璃-1 ==")
        set_margins(hwnd, -1, -1, -1, -1)
        front(hwnd)
        shots(sct, hwnd, "B0")
        print("== B0m 最大化 玻璃-1（复现原生按钮） ==")
        u.ShowWindow(hwnd, 3)
        time.sleep(1.0)
        front(hwnd)
        shots(sct, hwnd, "B0m")
        u.ShowWindow(hwnd, 9)
        time.sleep(0.8)

        print("== B1 常态 玻璃0 ==")
        set_margins(hwnd, 0, 0, 0, 0)
        front(hwnd)
        shots(sct, hwnd, "B1")
        print("== B1m 最大化 玻璃0 ==")
        u.ShowWindow(hwnd, 3)
        time.sleep(1.0)
        front(hwnd)
        shots(sct, hwnd, "B1m")
        u.ShowWindow(hwnd, 9)
        time.sleep(0.8)

        print("== B2 常态 {10000,10000,0,10000} 顶0 ==")
        set_margins(hwnd, 10000, 10000, 0, 10000)
        front(hwnd)
        shots(sct, hwnd, "B2")

        print("== 恢复 玻璃-1 ==")
        set_margins(hwnd, -1, -1, -1, -1)


if __name__ == "__main__":
    main()
