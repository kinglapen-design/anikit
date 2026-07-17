# -*- coding: utf-8 -*-
"""剪贴板回写探针（阶段 0 spike · 回环验证）

把 probe.py 导出的 "Flash 11 Picture" 数据写回系统剪贴板，
然后在 Animate 中 Ctrl+V 粘贴，验证元件与坐标能否还原。

用法：python probe-write.py [bin文件路径]
      默认使用 out/C32F_Flash_11_Picture.bin
"""
import ctypes
import os
import sys
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8")

u32 = ctypes.WinDLL("user32", use_last_error=True)
k32 = ctypes.WinDLL("kernel32", use_last_error=True)

RegisterClipboardFormatW = u32.RegisterClipboardFormatW
RegisterClipboardFormatW.argtypes = [wintypes.LPCWSTR]
RegisterClipboardFormatW.restype = wintypes.UINT

OpenClipboard = u32.OpenClipboard
OpenClipboard.argtypes = [wintypes.HWND]
OpenClipboard.restype = wintypes.BOOL

EmptyClipboard = u32.EmptyClipboard
EmptyClipboard.argtypes = []
EmptyClipboard.restype = wintypes.BOOL

SetClipboardData = u32.SetClipboardData
SetClipboardData.argtypes = [wintypes.UINT, wintypes.HANDLE]
SetClipboardData.restype = wintypes.HANDLE

CloseClipboard = u32.CloseClipboard
CloseClipboard.argtypes = []
CloseClipboard.restype = wintypes.BOOL

GMEM_MOVEABLE = 0x2000

GlobalAlloc = k32.GlobalAlloc
GlobalAlloc.argtypes = [wintypes.UINT, ctypes.c_size_t]
GlobalAlloc.restype = wintypes.HGLOBAL

GlobalLock = k32.GlobalLock
GlobalLock.argtypes = [wintypes.HGLOBAL]
GlobalLock.restype = wintypes.LPVOID

GlobalUnlock = k32.GlobalUnlock
GlobalUnlock.argtypes = [wintypes.HGLOBAL]
GlobalUnlock.restype = wintypes.BOOL

FORMAT_NAME = "Flash 11 Picture"


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    src = sys.argv[1] if len(sys.argv) > 1 else os.path.join(here, "out", "C32F_Flash_11_Picture.bin")
    with open(src, "rb") as f:
        data = f.read()
    print("载荷：%s（%d 字节）" % (src, len(data)))

    fmt = RegisterClipboardFormatW(FORMAT_NAME)
    if not fmt:
        print("RegisterClipboardFormat 失败"); return
    print("注册格式 %s -> 0x%04X" % (FORMAT_NAME, fmt))

    if not OpenClipboard(None):
        print("无法打开剪贴板（可能被占用），请重试"); return
    try:
        EmptyClipboard()
        h = GlobalAlloc(GMEM_MOVEABLE, len(data))
        if not h:
            print("GlobalAlloc 失败"); return
        ptr = GlobalLock(h)
        ctypes.memmove(ptr, data, len(data))
        GlobalUnlock(h)
        if not SetClipboardData(fmt, h):
            print("SetClipboardData 失败"); return
        print("已写入剪贴板。")
    finally:
        CloseClipboard()

    print()
    print("请切换到 Animate，在舞台上 Ctrl+V 粘贴：")
    print("  元件出现且位置正确 -> 回环验证成功，主方案完全成立；")
    print("  粘贴无反应或内容不对 -> 把现象告诉我，可能需要同时写入 OLE 关联格式。")


if __name__ == "__main__":
    main()
