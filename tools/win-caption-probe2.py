# 实机走查第 17 条探针二轮——玻璃延伸 × 云母 × 原生按钮关系实证（063 手法）
#   一轮结论：C1 顶 1px 玻璃无效（按钮仍在）；C2 摘 SYSMENU|MIN|MAX 按钮没了但 webview
#   内容渲染崩（整窗空玻璃）出局。二轮问一个问题：**云母到底由谁承载**——
#   DWMWA_SYSTEMBACKDROP_TYPE(38) / 旧 MICA_EFFECT(1029) 属性，还是 -1 玻璃延伸本身。
#   候选：C3 玻璃全 0 / C4 {0,0,1,0} 经典阴影边 / C5 玻璃全 0 + 强制 backdrop=DWMSBT_MAINWINDOW。
#   每轮拍完恢复 -1；开头先读三个 DWM 属性现值存档。
#   输出：tools/style-preview/caption-probe2/{C3,C4,C5}/{tr,top,left,br,tb,full}.png
# 用法：tools/.venv/Scripts/python.exe tools/win-caption-probe2.py
import ctypes
import os
import sys
import time
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

OUT = "tools/style-preview/caption-probe2"
PAD = 14

u = ctypes.windll.user32
dwm = ctypes.windll.dwmapi
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))

DWMWA_NCRENDERING_POLICY = 2
DWMWA_SYSTEMBACKDROP_TYPE = 38
DWMWA_MICA_EFFECT = 1029  # Win11 21H2  undocumented


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


def get_attr(hwnd, attr):
    v = ctypes.c_int(0)
    hr = dwm.DwmGetWindowAttribute(wintypes.HWND(hwnd), attr, ctypes.byref(v), 4)
    return hr, v.value


def set_attr(hwnd, attr, val):
    v = ctypes.c_int(val)
    hr = dwm.DwmSetWindowAttribute(wintypes.HWND(hwnd), attr, ctypes.byref(v), 4)
    print(f"  DwmSetWindowAttribute({attr}, {val}) = {hr:#x}")


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
    ox, oy = l - box[0], t - box[1]  # 窗口左上图内偏移（clamp 安全）
    crops = {
        "tr": (ox + w - 300, oy, ox + w + PAD, oy + 100),
        "top": (ox, oy - PAD, ox + w, oy + 16),
        "left": (ox - PAD, oy, ox + 16, oy + 420),
        "br": (ox + w - 90, oy + h - 90, ox + w + PAD, oy + h + PAD),
        "tb": (ox + w // 2 - 210, oy, ox + w // 2 + 210, oy + 50),
    }
    for name, (x0, y0, x1, y1) in crops.items():
        img.crop((max(0, x0), max(0, y0), min(img.width, x1), min(img.height, y1))).save(f"{OUT}/{tag}/{name}.png")
    print(f"  [{tag}] rect={l},{t},{r},{b}")


def main():
    hwnd = find_main_window()
    print(f"hwnd={hwnd}")
    for attr, name in [(DWMWA_NCRENDERING_POLICY, "NCRP"), (DWMWA_SYSTEMBACKDROP_TYPE, "SYSTEMBACKDROP_TYPE(38)"), (DWMWA_MICA_EFFECT, "MICA_EFFECT(1029)")]:
        hr, v = get_attr(hwnd, attr)
        print(f"  attr {name}: hr={hr:#x} value={v}")
    u.ShowWindow(hwnd, 9)
    time.sleep(1.0)

    with mss.mss() as sct:
        print("== R0 基线常态（玻璃-1） ==")
        shots(sct, hwnd, "R0")
        print("== C3 玻璃 {0,0,0,0} ==")
        set_margins(hwnd, 0, 0, 0, 0)
        time.sleep(0.8)
        shots(sct, hwnd, "C3")
        print("== C4 玻璃 {0,0,1,0} ==")
        set_margins(hwnd, 0, 0, 1, 0)
        time.sleep(0.8)
        shots(sct, hwnd, "C4")
        print("== C5 玻璃 {0,0,0,0} + SYSTEMBACKDROP_TYPE=2(MAINWINDOW) ==")
        set_margins(hwnd, 0, 0, 0, 0)
        set_attr(hwnd, DWMWA_SYSTEMBACKDROP_TYPE, 2)
        time.sleep(0.8)
        shots(sct, hwnd, "C5")
        hr, v = get_attr(hwnd, DWMWA_SYSTEMBACKDROP_TYPE)
        print(f"  C5 后 38 号属性回读: hr={hr:#x} value={v}")
        print("== 恢复基线 ==")
        set_margins(hwnd, -1, -1, -1, -1)


if __name__ == "__main__":
    main()
