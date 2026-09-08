# 实机走查第 17 条探针三轮——终案候选实证（063 手法）
#   二轮结论：① 38 号 backdrop 属性 = 2 已在基线挂上，常态玻璃全 0 与 -1 像素级一致
#   （云母由 backdrop 属性承载、边缘/角/阴影不变）；② 最大化时 DWM 关掉 backdrop
#   （玻璃 0 → 纯黑，须前端回落实色底）。三轮定案验证：
#   C6 常态 {10000,10000,0,10000}——顶边 0 玻璃杀 inset 细条按钮（走查视频红块），
#      左右/下大 margins 兜住角部月牙；C7 最大化 {0,0,0,0}——原生三钮杀灭确认 +
#      标题栏黑底范围（= 前端实色补丁要盖的区域）。拍完恢复 -1 常态。
#   输出：tools/style-preview/caption-probe3/{C6,C7}/{tr,top,left,br,tb,full}.png
# 用法：tools/.venv/Scripts/python.exe tools/win-caption-probe3.py
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

OUT = "tools/style-preview/caption-probe3"
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
        "left": (ox - PAD, oy, ox + 16, oy + 420),
        "br": (ox + w - 90, oy + h - 90, ox + w + PAD, oy + h + PAD),
        "tb": (ox + w // 2 - 210, oy, ox + w // 2 + 210, oy + 50),
    }
    for name, (x0, y0, x1, y1) in crops.items():
        img.crop((max(0, x0), max(0, y0), min(img.width, x1), min(img.height, y1))).save(f"{OUT}/{tag}/{name}.png")
    print(f"  [{tag}] rect={l},{t},{r},{b} zoomed={u.IsZoomed(hwnd)}")


def main():
    hwnd = find_main_window()
    print(f"hwnd={hwnd}")
    u.ShowWindow(hwnd, 9)
    time.sleep(1.0)

    with mss.mss() as sct:
        print("== C6 常态 {10000,10000,0,10000} ==")
        set_margins(hwnd, 10000, 10000, 0, 10000)
        time.sleep(0.8)
        shots(sct, hwnd, "C6")
        print("== C7 最大化 {0,0,0,0} ==")
        set_margins(hwnd, 0, 0, 0, 0)
        u.ShowWindow(hwnd, 3)
        time.sleep(1.2)
        shots(sct, hwnd, "C7")
        print("== 恢复：SW_RESTORE + 玻璃 -1 ==")
        u.ShowWindow(hwnd, 9)
        time.sleep(0.8)
        set_margins(hwnd, -1, -1, -1, -1)


if __name__ == "__main__":
    main()
