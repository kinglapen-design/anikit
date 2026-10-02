# Everything 1.4 WM_COPYDATA IPC 协议验证探针（2026-10-03，工作空间搜索立项前置）
# 依据：Everything_IPC.h（voidtools 官方 IPC SDK）+ Windhawk better-file-sizes 参照实现。
# 验证点：①FindWindow 探测（1.5a / 1.4 两类名）②WM_USER 版本查询 ③QUERY2 查询往返
#         ④LIST2/ITEM2 数据布局解析（full_path_and_name + size + date_modified）⑤item flags 口径。
# 纯 ctypes 零依赖，只读查询，对 Everything 无副作用。
# 用法：tools/.venv/Scripts/python.exe tools/everything-ipc-probe.py [搜索词]
import ctypes
import ctypes.wintypes as wt
import struct
import sys
import time

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

user32 = ctypes.windll.user32

EVERYTHING_IPC_WNDCLASS = "EVERYTHING_TASKBAR_NOTIFICATION"
EVERYTHING_IPC_WNDCLASS_15A = "EVERYTHING_TASKBAR_NOTIFICATION_(1.5a)"
WM_USER = 0x0400
WM_COPYDATA = 0x004A
COPYDATA_QUERY2W = 18
REQ_FULL_PATH_AND_NAME = 0x00000004
REQ_SIZE = 0x00000010
REQ_DATE_MODIFIED = 0x00000040
SORT_NAME_ASCENDING = 1
HWND_MESSAGE = wt.HWND(-3)
MSGFLT_ALLOW = 1
SMTO_BLOCK = 0x0001
SMTO_ABORTIFHUNG = 0x0002

WNDPROCTYPE = ctypes.WINFUNCTYPE(ctypes.c_long, wt.HWND, ctypes.c_uint, ctypes.wintypes.WPARAM, ctypes.wintypes.LPARAM)

user32.RegisterClassExW.argtypes = [ctypes.c_void_p]
user32.CreateWindowExW.argtypes = [wt.DWORD, wt.LPCWSTR, wt.LPCWSTR, wt.DWORD, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int, wt.HWND, wt.HMENU, wt.HINSTANCE, wt.LPVOID]
user32.CreateWindowExW.restype = wt.HWND
user32.SendMessageTimeoutW.argtypes = [wt.HWND, ctypes.c_uint, ctypes.c_size_t, ctypes.c_void_p, ctypes.c_uint, ctypes.c_uint, ctypes.POINTER(ctypes.c_size_t)]
user32.SendMessageTimeoutW.restype = ctypes.c_size_t
user32.SendMessageW.argtypes = [wt.HWND, ctypes.c_uint, ctypes.wintypes.WPARAM, ctypes.wintypes.LPARAM]
user32.SendMessageW.restype = ctypes.c_long
user32.FindWindowW.argtypes = [wt.LPCWSTR, wt.LPCWSTR]
user32.FindWindowW.restype = wt.HWND
user32.ChangeWindowMessageFilterEx.argtypes = [wt.HWND, ctypes.c_uint, wt.DWORD, ctypes.c_void_p]
user32.PeekMessageW.argtypes = [ctypes.c_void_p, wt.HWND, ctypes.c_uint, ctypes.c_uint, ctypes.c_uint]
user32.DefWindowProcW.argtypes = [wt.HWND, ctypes.c_uint, ctypes.wintypes.WPARAM, ctypes.wintypes.LPARAM]
user32.DefWindowProcW.restype = ctypes.c_long


class COPYDATASTRUCT(ctypes.Structure):
    _fields_ = [("dwData", ctypes.c_size_t), ("cbData", wt.DWORD), ("lpData", ctypes.c_void_p)]


class MSG(ctypes.Structure):
    _fields_ = [("hwnd", wt.HWND), ("message", ctypes.c_uint), ("wParam", ctypes.wintypes.WPARAM), ("lParam", ctypes.wintypes.LPARAM), ("time", wt.DWORD), ("pt", wt.POINT)]


class WNDCLASSEXW(ctypes.Structure):
    _fields_ = [("cbSize", ctypes.c_uint), ("style", ctypes.c_uint), ("lpfnWndProc", WNDPROCTYPE), ("cbClsExtra", ctypes.c_int), ("cbWndExtra", ctypes.c_int), ("hInstance", wt.HINSTANCE), ("hIcon", wt.HANDLE), ("hCursor", wt.HANDLE), ("hbrBackground", wt.HANDLE), ("lpszMenuName", wt.LPCWSTR), ("lpszClassName", wt.LPCWSTR), ("hIconSm", wt.HANDLE)]


reply_hits = []  # [(path, flags, size, date_mod_raw)]
reply_event = {"done": False, "reply_id": None, "tot": 0}
query_mode = {"v2": True}  # False = QUERY1(dwData=2)


def wnd_proc(hwnd, msg, wparam, lparam):
    if msg == WM_COPYDATA:
        cds = ctypes.cast(lparam, ctypes.POINTER(COPYDATASTRUCT)).contents
        if cds.dwData == reply_event["reply_id"]:
            base = cds.lpData
            if query_mode["v2"]:
                tot, num, offset, req_flags, sort = struct.unpack_from("<5I", ctypes.string_at(base, 20))
                reply_event["tot"] = tot
                items = []
                for i in range(num):
                    flags, data_off = struct.unpack_from("<2I", ctypes.string_at(base + 20 + i * 8, 8))
                    # 数据布局（pack 1）：full_path_and_name = DWORD 长度(不含\0) + UTF-16 串(含\0)；随后 size i64、date_modified FILETIME
                    p = base + data_off
                    (nlen,) = struct.unpack_from("<I", ctypes.string_at(p, 4))
                    name = ctypes.wstring_at(p + 4, nlen)
                    q = p + 4 + (nlen + 1) * 2
                    (size,) = struct.unpack_from("<q", ctypes.string_at(q, 8))
                    (dm_lo, dm_hi) = struct.unpack_from("<2I", ctypes.string_at(q + 8, 8))
                    items.append((name, flags, size, (dm_hi << 32) | dm_lo))
                reply_hits.extend(items)
            else:
                # QUERY1 LISTW：7 DWORD (totfolders,totfiles,totitems,numfolders,numfiles,numitems,offset) + items{flags,filename_offset,path_offset}
                totf, totfl, tot, numf, numfl, num, offset = struct.unpack_from("<7I", ctypes.string_at(base, 28))
                reply_event["tot"] = tot
                items = []
                for i in range(num):
                    flags, fn_off, p_off = struct.unpack_from("<3I", ctypes.string_at(base + 28 + i * 12, 12))
                    fname = ctypes.wstring_at(base + fn_off)
                    fpath = ctypes.wstring_at(base + p_off)
                    items.append((fpath + fname, flags, -1, 0))
                reply_hits.extend(items)
            reply_event["done"] = True
        return 1
    return user32.DefWindowProcW(hwnd, msg, wparam, lparam)


def main():
    term = sys.argv[1] if len(sys.argv) > 1 else "anikit"
    sflags = int(sys.argv[2]) if len(sys.argv) > 2 else 0  # search_flags: 1=case 2=wholeword 4=matchpath 8=regex 0x10=diacritics
    max_results = int(sys.argv[4]) if len(sys.argv) > 4 else 20  # 0xFFFFFFFF = 全量
    if max_results < 0:
        max_results = 0xFFFFFFFF
    print(f"[输入] {term!r} search_flags={sflags} max={max_results}")

    # ① 探测（1.5a 优先，回落 1.4）
    h15a = user32.FindWindowW(EVERYTHING_IPC_WNDCLASS_15A, None)
    h14 = user32.FindWindowW(EVERYTHING_IPC_WNDCLASS, None)
    h_ev = h15a or h14
    print(f"[探测] 1.5a 窗口: {h15a} / 1.4 窗口: {h14}")
    if not h_ev:
        print("[失败] 未找到 Everything 窗口（Everything 未运行？）")
        return 1

    # ② 版本查询：SendMessage(hwnd, WM_USER, 0/1/2/3, 0)
    ver = [user32.SendMessageW(h_ev, WM_USER, i, 0) for i in range(4)]
    print(f"[版本] {'.'.join(str(v) for v in ver)}")

    # ③ 自建 message-only 接收窗
    proc = WNDPROCTYPE(wnd_proc)  # 必须保活
    wc = WNDCLASSEXW()
    wc.cbSize = ctypes.sizeof(WNDCLASSEXW)
    wc.lpfnWndProc = proc
    wc.hInstance = ctypes.windll.kernel32.GetModuleHandleW(None)
    wc.lpszClassName = "anikit-everything-ipc-probe"
    if not user32.RegisterClassExW(ctypes.byref(wc)):
        err = ctypes.GetLastError()
        if err != 1410:  # ERROR_CLASS_ALREADY_EXISTS
            print(f"[失败] RegisterClassExW: {err}")
            return 1
    hwnd = user32.CreateWindowExW(0, wc.lpszClassName, None, 0, 0, 0, 0, 0, HWND_MESSAGE, None, wc.hInstance, None)
    if not hwnd:
        print(f"[失败] CreateWindowExW: {ctypes.GetLastError()}")
        return 1
    user32.ChangeWindowMessageFilterEx(hwnd, WM_COPYDATA, MSGFLT_ALLOW, None)
    print(f"[接收窗] hwnd={hwnd}")

    # ④ QUERY2 查询（原样透传搜索串；argv[3]="v1" 走 QUERY1 对照）
    search = term
    if len(sys.argv) > 3 and sys.argv[3] == "v1":
        query_mode["v2"] = False
    query_bytes = (search + "\0").encode("utf-16-le")
    if query_mode["v2"]:
        header = struct.pack("<7I", hwnd, 42, sflags, 0, max_results,  # max_results=20
                             REQ_FULL_PATH_AND_NAME | REQ_SIZE | REQ_DATE_MODIFIED, SORT_NAME_ASCENDING)
        dw_data = COPYDATA_QUERY2W
    else:
        # QUERY1: HWND(8) + ULONG_PTR(8) + search_flags + offset + max_results + 串
        header = struct.pack("<QQ3I", hwnd, 42, sflags, 0, max_results)
        dw_data = 2
    buf = header + query_bytes
    reply_event["reply_id"] = 42

    cds = COPYDATASTRUCT(dwData=dw_data, cbData=len(buf), lpData=0)
    raw = ctypes.create_string_buffer(buf, len(buf))
    cds.lpData = ctypes.addressof(raw)

    t0 = time.perf_counter()
    pdw = ctypes.c_size_t(0)
    ok = user32.SendMessageTimeoutW(h_ev, WM_COPYDATA, hwnd, ctypes.byref(cds), SMTO_BLOCK | SMTO_ABORTIFHUNG, 3000, ctypes.byref(pdw))
    print(f"[发送] SendMessageTimeout 返回 {ok}（耗时 {(time.perf_counter()-t0)*1000:.1f}ms）")
    if not ok:
        print(f"[失败] 发送超时/被拒: {ctypes.GetLastError()}")
        return 1

    # ⑤ 泵消息等回包
    msg = MSG()
    deadline = time.perf_counter() + 5
    while not reply_event["done"] and time.perf_counter() < deadline:
        while user32.PeekMessageW(ctypes.byref(msg), None, 0, 0, 1):
            user32.TranslateMessage(ctypes.byref(msg))
            user32.DispatchMessageW(ctypes.byref(msg))
        time.sleep(0.001)
    dt = (time.perf_counter() - t0) * 1000

    if not reply_event["done"]:
        print("[失败] 5 秒内未收到回包")
        return 1
    print(f"[回包] 总命中 {reply_event['tot']}，返回 {len(reply_hits)} 条（全程 {dt:.1f}ms）")
    for path, flags, size, dm in reply_hits[:10]:
        kind = []
        if flags & 1: kind.append("FOLDER")
        if flags & 2: kind.append("FILE")
        if flags & 4: kind.append("DRIVE")
        if flags & 8: kind.append("ROOT")
        print(f"  flags={flags:02x} [{'|'.join(kind)}] size={size:>12} {path}")

    user32.DestroyWindow(hwnd)
    print("[通过] 协议往返验证成功")
    return 0


if __name__ == "__main__":
    sys.exit(main())
