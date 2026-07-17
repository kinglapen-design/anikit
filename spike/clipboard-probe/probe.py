# -*- coding: utf-8 -*-
"""Animate 剪贴板格式探针（阶段 0 spike）

用法：先在 Animate 里复制一个元件（Ctrl+C），再运行本脚本。
输出：out/report.txt 报告 + out/ 下每个格式的完整数据（.bin）。
零依赖：仅用标准库 ctypes 调 Win32。

结构：父进程枚举格式后，为每个格式派生子进程导出数据——
某些 OLE 格式句柄直接读取会硬崩溃，子进程隔离保证报告完整。
"""
import ctypes
import datetime
import json
import os
import re
import subprocess
import sys
from ctypes import wintypes

sys.stdout.reconfigure(encoding="utf-8")

u32 = ctypes.WinDLL("user32", use_last_error=True)
k32 = ctypes.WinDLL("kernel32", use_last_error=True)

OpenClipboard = u32.OpenClipboard
OpenClipboard.argtypes = [wintypes.HWND]
OpenClipboard.restype = wintypes.BOOL

CloseClipboard = u32.CloseClipboard
CloseClipboard.argtypes = []
CloseClipboard.restype = wintypes.BOOL

EnumClipboardFormats = u32.EnumClipboardFormats
EnumClipboardFormats.argtypes = [wintypes.UINT]
EnumClipboardFormats.restype = wintypes.UINT

GetClipboardFormatNameW = u32.GetClipboardFormatNameW
GetClipboardFormatNameW.argtypes = [wintypes.UINT, wintypes.LPWSTR, ctypes.c_int]
GetClipboardFormatNameW.restype = ctypes.c_int

GetClipboardData = u32.GetClipboardData
GetClipboardData.argtypes = [wintypes.UINT]
GetClipboardData.restype = wintypes.HANDLE

GlobalSize = k32.GlobalSize
GlobalSize.argtypes = [wintypes.HGLOBAL]
GlobalSize.restype = ctypes.c_size_t

GlobalLock = k32.GlobalLock
GlobalLock.argtypes = [wintypes.HGLOBAL]
GlobalLock.restype = wintypes.LPVOID

GlobalUnlock = k32.GlobalUnlock
GlobalUnlock.argtypes = [wintypes.HGLOBAL]
GlobalUnlock.restype = wintypes.BOOL

STANDARD = {
    1: "CF_TEXT", 2: "CF_BITMAP", 3: "CF_METAFILEPICT", 4: "CF_SYLK",
    5: "CF_DIF", 6: "CF_TIFF", 7: "CF_OEMTEXT", 8: "CF_DIB",
    9: "CF_PALETTE", 10: "CF_PENDATA", 11: "CF_RIFF", 12: "CF_WAVE",
    13: "CF_UNICODETEXT", 14: "CF_ENHMETAFILE", 15: "CF_HDROP",
    16: "CF_LOCALE", 17: "CF_DIBV5", 128: "CF_OWNERDISPLAY",
    129: "CF_DSPTEXT", 130: "CF_DSPBITMAP", 131: "CF_DSPMETAFILEPICT",
    142: "CF_DSPENHMETAFILE",
}

MAX_DUMP = 64 * 1024 * 1024  # 单个格式最多导出 64MB


def fmt_name(fmt):
    buf = ctypes.create_unicode_buffer(256)
    n = GetClipboardFormatNameW(fmt, buf, 256)
    if n > 0:
        return buf.value
    if fmt in STANDARD:
        return STANDARD[fmt]
    if 512 <= fmt <= 767:
        return "CF_PRIVATE(%d)" % fmt
    if 768 <= fmt <= 1023:
        return "CF_GDIOBJ(%d)" % fmt
    return "#%d" % fmt


def dump_data(fmt):
    """返回 (声明大小, 实际数据)。位图/图元文件/OLE 流类句柄可能读不了。"""
    h = GetClipboardData(fmt)
    if not h:
        return 0, b""
    size = GlobalSize(h)
    if not size or size > MAX_DUMP:
        return size, b""
    ptr = GlobalLock(h)
    if not ptr:
        return size, b""
    try:
        data = ctypes.string_at(ptr, size)
    finally:
        GlobalUnlock(h)
    return size, data


def clean(s):
    return "".join(c for c in s if c.isprintable() or c in "\r\n\t")


def looks_text(data):
    if not data:
        return None, None
    for enc in ("utf-8", "utf-16-le"):
        s = data[:8192].decode(enc, errors="ignore")
        t = s.strip("\x00").strip()
        if not t or "\x00" in t[:200]:
            continue
        if t.startswith("<"):
            return enc, clean(t[:300])
        if enc == "utf-8":
            printable = sum(1 for c in t if c.isprintable() or c in "\r\n\t")
            if len(t) >= 16 and printable / len(t) > 0.9:
                return enc, clean(t[:300])
    return None, None


def safe_filename(name):
    return re.sub(r'[\\/:*?"<>|\s]+', "_", name)


def child_dump(fmt, outfile):
    """子进程模式：导出单个格式（此模式下的崩溃不影响父进程）。"""
    if not OpenClipboard(None):
        print(json.dumps({"error": "open failed"}))
        return
    try:
        size, data = dump_data(fmt)
    finally:
        CloseClipboard()
    if data:
        with open(outfile, "wb") as f:
            f.write(data)
    print(json.dumps({"size": size, "dumped": len(data)}))


def parent():
    here = os.path.dirname(os.path.abspath(__file__))
    outdir = os.path.join(here, "out")
    os.makedirs(outdir, exist_ok=True)

    lines = ["# 剪贴板探针报告  %s" % datetime.datetime.now().strftime("%Y-%m-%d %H:%M:%S"), ""]

    if not OpenClipboard(None):
        print("无法打开剪贴板（可能被其他程序占用），请重试。")
        return
    try:
        formats = []
        fmt = 0
        while True:
            fmt = EnumClipboardFormats(fmt)
            if fmt == 0:
                break
            formats.append(fmt)
    finally:
        CloseClipboard()

    lines.append("共 %d 种格式" % len(formats))
    lines.append("")

    for fmt in formats:
        name = fmt_name(fmt)
        fname = "%04X_%s.bin" % (fmt, safe_filename(name))
        fpath = os.path.join(outdir, fname)
        if os.path.exists(fpath):
            os.remove(fpath)

        try:
            r = subprocess.run(
                [sys.executable, os.path.abspath(__file__), "--dump", str(fmt), fpath],
                capture_output=True, text=True, timeout=60)
        except subprocess.TimeoutExpired:
            lines.append("[%d] 0x%04X  %s  <导出超时>" % (fmt, fmt, name))
            lines.append("")
            continue

        if r.returncode != 0 or not r.stdout.strip():
            lines.append("[%d] 0x%04X  %s  <子进程异常，已跳过>" % (fmt, fmt, name))
            lines.append("")
            continue

        info = json.loads(r.stdout.strip().splitlines()[-1])
        size = info.get("size", 0)
        data = b""
        if info.get("dumped") and os.path.exists(fpath):
            with open(fpath, "rb") as f:
                data = f.read()

        tag = "  <== 注册格式（重点关注）" if fmt >= 0xC000 else ""
        lines.append("[%d] 0x%04X  %s  %s 字节 -> %s%s" % (
            fmt, fmt, name, size if size else len(data), fname if data else "（无法导出）", tag))
        if data:
            lines.append("  hex(前64字节): " + data[:64].hex(" "))
        enc, preview = looks_text(data)
        if preview:
            lines.append("  文本(%s): %s" % (enc, preview.replace("\r", "").replace("\n", " | ")))
        lines.append("")

    report = "\n".join(lines)
    with open(os.path.join(outdir, "report.txt"), "w", encoding="utf-8") as f:
        f.write(report)
    print(report)
    print("报告与数据已写入 out/ 目录。")


if __name__ == "__main__":
    if len(sys.argv) == 4 and sys.argv[1] == "--dump":
        child_dump(int(sys.argv[2]), sys.argv[3])
    else:
        parent()
