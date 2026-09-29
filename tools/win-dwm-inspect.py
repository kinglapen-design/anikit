# 只读 DWM 体检探针（2026-09-27 云母失效/边框丢失排查）：
# 对正在运行的 AniKit 主窗口枚举窗口样式 + DWM 属性 + 边缘剖线像素，零写入。
# 用法：tools/.venv/Scripts/python.exe tools/win-dwm-inspect.py [窗口标题关键词]
import ctypes
import sys
from ctypes import wintypes

import mss
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

u = ctypes.windll.user32
dwm = ctypes.windll.dwmapi
u.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))  # Per-Monitor V2

# 窗口样式位
GWL_STYLE = -16
GWL_EXSTYLE = -20
STYLE_BITS = [
    (0x00C00000, "WS_CAPTION"), (0x00080000, "WS_SYSMENU"), (0x00020000, "WS_MINIMIZEBOX"),
    (0x00010000, "WS_MAXIMIZEBOX"), (0x00040000, "WS_THICKFRAME"), (0x80000000, "WS_POPUP"),
    (0x10000000, "WS_VISIBLE"), (0x02000000, "WS_MAXIMIZE"), (0x20000000, "WS_MINIMIZE"),
    (0x00800000, "WS_BORDER"), (0x00400000, "WS_DLGFRAME"), (0x04000000, "WS_CLIPSIBLINGS"),
    (0x08000000, "WS_CLIPCHILDREN"), (0x00020000, "WS_GROUP"),
]
EX_BITS = [
    (0x00000080, "WS_EX_TOOLWINDOW"), (0x00080000, "WS_EX_LAYERED"),
    (0x00000200, "WS_EX_ACCEPTFILES"), (0x00000001, "WS_EX_DLGMODALFRAME"),
    (0x02000000, "WS_EX_COMPOSITED"), (0x00000008, "WS_EX_TOPMOST"),
    (0x00040000, "WS_EX_APPWINDOW"), (0x00000100, "WS_EX_WINDOWEDGE"),
    (0x08000000, "WS_EX_NOACTIVATE"),
]

# DWM 属性号
DWMWA = [
    (1, "NCRENDERING_ENABLED"), (2, "NCRENDERING_POLICY"), (6, "TRANSITIONS_FORCEDISABLED"),
    (9, "EXTENDED_FRAME_BOUNDS(rect)"), (20, "USE_IMMERSIVE_DARK_MODE"),
    (33, "WINDOW_CORNER_PREFERENCE"), (34, "BORDER_COLOR"), (35, "CAPTION_COLOR"),
    (36, "TEXT_COLOR"), (37, "VISIBLE_FRAME_BORDER_THICKNESS"), (38, "SYSTEMBACKDROP_TYPE"),
    (1029, "MICA_EFFECT(1029)"), (1030, "REDIRECTIONBITMAP_ALPHA(1030)"),
]


def find_windows(keyword: str):
    found = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)
    def cb(hwnd, lp):
        if u.IsWindowVisible(hwnd):
            n = u.GetWindowTextLengthW(hwnd)
            if n:
                buf = ctypes.create_unicode_buffer(n + 1)
                u.GetWindowTextW(hwnd, buf, n + 1)
                if keyword.lower() in buf.value.lower():
                    pid = wintypes.DWORD()
                    u.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
                    found.append((hwnd, buf.value, pid.value))
        return True

    u.EnumWindows(cb, 0)
    return found


def dump(hwnd: int, title: str, pid: int):
    print(f"\n===== hwnd={hwnd} pid={pid} title={title!r} =====")
    style = u.GetWindowLongPtrW(hwnd, GWL_STYLE)
    ex = u.GetWindowLongPtrW(hwnd, GWL_EXSTYLE)
    print(f"STYLE    = {style:#010x} : {[n for b, n in STYLE_BITS if style & b]}")
    print(f"EXSTYLE  = {ex:#010x} : {[n for b, n in EX_BITS if ex & b]}")
    print(f"IsZoomed = {bool(u.IsZoomed(hwnd))}")
    print(f"DwmIsCompositionEnabled = {ctypes.byref(v := wintypes.BOOL()) and (dwm.DwmIsCompositionEnabled(ctypes.byref(v)), bool(v.value))[1]}")

    for attr, name in DWMWA:
        if "rect" in name:
            r = wintypes.RECT()
            hr = dwm.DwmGetWindowAttribute(wintypes.HWND(hwnd), attr, ctypes.byref(r), ctypes.sizeof(r))
            print(f"  DWMWA[{attr:>4}] {name:<32} hr={hr:#010x} -> ({r.left},{r.top},{r.right},{r.bottom})" if hr == 0 else f"  DWMWA[{attr:>4}] {name:<32} hr={hr:#010x}")
        else:
            v = wintypes.DWORD()
            hr = dwm.DwmGetWindowAttribute(wintypes.HWND(hwnd), attr, ctypes.byref(v), ctypes.sizeof(v))
            val = f"{v.value:#010x} ({v.value})" if hr == 0 else "-"
            print(f"  DWMWA[{attr:>4}] {name:<32} hr={hr:#010x} -> {val}")

    # 截屏 + 四边剖线（边框线检测：窗口外缘向内 12px 逐列采样）
    rc = wintypes.RECT()
    u.GetWindowRect(hwnd, ctypes.byref(rc))
    w, h = rc.right - rc.left, rc.bottom - rc.top
    pad = 16
    box = (max(0, rc.left - pad), max(0, rc.top - pad), rc.right + pad, rc.bottom + pad)
    with mss.mss() as sct:
        img = Image.frombytes("RGB", (box[2] - box[0], box[3] - box[1]), sct.grab(box).rgb)
    img.save("tools/style-preview/dwm-inspect-full.png")
    # 顶边/左边中点剖线：窗外 → 窗内 16px 的像素序列
    ox, oy = rc.left - box[0], rc.top - box[1]
    for label, (x, y), (dx, dy) in [
        ("top-mid", (ox + w // 2, oy), (0, 1)),
        ("left-mid", (ox, oy + h // 2), (1, 0)),
        ("right-mid", (ox + w - 1, oy + h // 2), (-1, 0)),
        ("bottom-mid", (ox + w // 2, oy + h - 1), (0, -1)),
    ]:
        seq = [img.getpixel((x + dx * i, y + dy * i)) for i in range(-6, 10)]
        print(f"  剖线 {label:<11} 窗外→窗内: {seq}")
    # 窗口左上/右上角 40x40 放大存证
    for label, (cx, cy) in [("tl", (ox, oy)), ("tr", (ox + w - 40, oy))]:
        img.crop((cx - 8, cy - 8, cx + 48, cy + 48)).resize((224, 224), Image.NEAREST).save(f"tools/style-preview/dwm-inspect-{label}.png")
    print(f"  rect=({rc.left},{rc.top},{rc.right},{rc.bottom}) {w}x{h}，截图存 tools/style-preview/dwm-inspect-*.png")


def main():
    kw = sys.argv[1] if len(sys.argv) > 1 else "AniKit"
    wins = find_windows(kw)
    if not wins:
        print(f"未找到标题含 {kw!r} 的可见窗口")
        return
    for hwnd, title, pid in wins:
        dump(hwnd, title, pid)


if __name__ == "__main__":
    main()
