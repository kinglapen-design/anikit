# 实机走查第 17 条终案验证探针——只截不动（063 手法 + 前台断言）
#   终案已内置在应用里（window_fx::extend_frame：最大化/全屏撤玻璃、还原回 -1；
#   前端 .window.fx-mica.maximized 回落实色底）。本探针不改任何窗口属性，
#   只置前 + 截屏存证：N 常态 / M 最大化 / R 还原。
#   输出：tools/style-preview/caption-probe5/{N,M,R}/{tr,top,tb,br,full}.png
# 用法：tools/.venv/Scripts/python.exe tools/win-caption-probe5.py
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

OUT = "tools/style-preview/caption-probe5"
PAD = 14

u = ctypes.windll.user32
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))


def find_main_window() -> int:
    """按可执行名找主窗口（最小化时标题变成 com.anikit.dev-siw，标题匹配不可靠）"""
    import subprocess

    out = subprocess.check_output(
        ["powershell", "-NoProfile", "-Command",
         "(Get-Process app -ErrorAction SilentlyContinue | Select-Object -First 1 Id).Id"],
        text=True).strip()
    pid = int(out)
    found = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, lp):
        wp = wintypes.DWORD()
        u.GetWindowThreadProcessId(hwnd, ctypes.byref(wp))
        if wp.value == pid and u.IsWindowVisible(hwnd):
            r = wintypes.RECT()
            u.GetWindowRect(hwnd, ctypes.byref(r))
            # 主窗口尺寸远大于 22×22 的辅助窗
            if r.right - r.left > 200:
                found.append(hwnd)
        return True

    u.EnumWindows(cb, 0)
    if not found:
        raise RuntimeError("找不到 AniKit 主窗口")
    return found[0]


def front(hwnd) -> bool:
    for i in range(3):
        u.keybd_event(0x12, 0, 0, 0)
        u.keybd_event(0x12, 0, 2, 0)
        u.SetForegroundWindow(hwnd)
        time.sleep(0.5)
        if u.GetForegroundWindow() == hwnd:
            return True
        print(f"  置前重试 {i + 1}")
    print("  !! 置前失败")
    return False


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
    print(f"  [{tag}] rect={l},{t},{r},{b} zoomed={u.IsZoomed(hwnd)} foreground={u.GetForegroundWindow() == hwnd}")


def main():
    hwnd = find_main_window()
    print(f"hwnd={hwnd}")
    u.ShowWindow(hwnd, 9)
    time.sleep(1.0)
    with mss.MSS() as sct:
        print("== N 常态 ==")
        front(hwnd)
        shots(sct, hwnd, "N")
        print("== M 最大化 ==")
        u.ShowWindow(hwnd, 3)
        time.sleep(1.2)
        front(hwnd)
        shots(sct, hwnd, "M")
        print("== R 还原 ==")
        u.ShowWindow(hwnd, 9)
        time.sleep(1.0)
        front(hwnd)
        shots(sct, hwnd, "R")


if __name__ == "__main__":
    main()
