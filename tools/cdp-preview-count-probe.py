# CDP 实测：预览计数跟随视口刷新 + 侧栏区块标题递归文件夹总数（2026-10-03 双修验证）
# BUG2 根因：tabs.viewKeyOf 直读 dockview params（非响应式，store 自述「api 非响应式」），
#   preview.total/index 是 pinia computed——首次打开预览算出 total 后 viewKey 变化不构成依赖，
#   缓存不失效（总数停留首视口值；按钮 disabled 按缓存 total 判定 = 亮，step 按实时列表越界
#   return = 点击无反应）。修复 = viewKeyOf 先读 tabs 响应式镜像建立依赖，取值仍以 api 实时为准。
# BUG1：LibraryView 区块标题 count = sec.nodes.length（仅顶级）→ blockSections 增 folderCount
#   递归文件夹总数（含全部后代，与 MovePanel countNodes 同口径）。
# 场景树：项目区块 = 项目A(2文件){子B(1){子C(0)}} + 项目D(1) → 递归文件夹 4；目录区块 = 目录夹E{子F(1)} → 2。
#   placements 共 6：FA×2 / FB×1 / FD×1 / FF×1 / root×1（showSubfolders 默认 true → FA 视口递归 3 项）。
# 断言：① 侧栏「项目」/「目录」区块标题数 = 4 / 2（修复前 2 / 1）；② 全部视口预览 1/6（基线）；
#   ③ 项目D 预览 1/1 且下一个禁用（修复前 1/6 残留、下一个亮、点击无反应）；
#   ④ 项目A 预览 1/3，点下一个 → 2/3（导航真实有效）。
import asyncio
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9368
URL = "http://localhost:5373"


def placement(aid, fid, name):
    return {
        "assetId": aid, "folderId": fid, "name": name,
        "w": 800, "h": 600, "ext": "png", "thumbPath": "E:/T/thumb.webp", "thumbKind": "asset",
        "origin": True, "frameCount": None, "animated": False, "duration": None, "bpm": None,
        "importedFrom": None, "noteCount": None, "thumbFrame": None, "fps": None,
        "size": 1000, "createdAt": None, "modifiedAt": "2026-10-01T00:00:00+08:00",
        "importedAt": "2026-10-01T00:00:00+08:00", "tags": [], "hidden": False,
    }


def folder(fid, name, block, count, children):
    return {"id": fid, "name": name, "count": count, "totalSize": 0, "block": block, "children": children}


TREE = {
    "blocks": [{"id": "dir", "name": "目录", "icon": None}, {"id": "b1", "name": "项目", "icon": None}],
    "nodes": [
        folder("FA", "项目A", "b1", 2, [folder("FB", "子B", None, 1, [folder("FC", "子C", None, 0, [])])]),
        folder("FD", "项目D", "b1", 1, []),
        folder("FE", "目录夹E", "dir", 1, [folder("FF", "子F", None, 1, [])]),
    ],
}
PLACEMENTS = [
    placement("A1", "FA", "图A1.png"), placement("A2", "FA", "图A2.png"),
    placement("A3", "FB", "图B3.png"), placement("A4", "FD", "图D4.png"),
    placement("A5", "FF", "图F5.png"), placement("A6", "root", "图R6.png"),
]

STUB = (
    "window.__TAURI_INTERNALS__ = {\n"
    "  metadata: { currentWindow: { label: \"main\" }, currentWebview: { label: \"main\" } },\n"
    "  invoke: (cmd, args) => {\n"
    f"    const R = {json.dumps({
        'get_current_library': {"name": "旧库", "path": "E:/T/S/旧库", "assetCount": 6, "folderCount": 6, "autoImport": False},
        'get_ui_settings': {"autoPlayGif": False},
        'detect_doc_apps': [],
        'list_placements': PLACEMENTS,
        'list_folder_tree': TREE,
        'list_recycle': [], 'list_recent_assets': [], 'list_archives': [],
        'get_undo_state': {"canUndo": False, "canRedo": False, "undoLabel": "", "redoLabel": ""},
        'list_space_libraries': ["旧库"],
        'list_recent_libraries': [{"name": "旧库", "path": "E:/T/S/旧库", "lastOpenedAt": ""}],
        'get_asset_file_path': {"path": "E:/T/src.png", "pending": False, "animPreview": None},
        'get_notes': [],
    }, ensure_ascii=False)};\n"
    "    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);\n"
    "  },\n"
    "  transformCallback: () => 0,\n"
    "  convertFileSrc: (p) => \"asset://localhost/\" + encodeURIComponent(p),\n"
    "};\n"
)

_msg_id = 0


async def send(ws, method, params=None, session=None):
    global _msg_id
    _msg_id += 1
    mid = _msg_id
    payload = {"id": mid, "method": method, "params": params or {}}
    if session:
        payload["sessionId"] = session
    await ws.send(json.dumps(payload))
    while True:
        raw = json.loads(await ws.recv())
        if raw.get("id") == mid:
            return raw
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-preview-count-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    try:
        version = None
        for _ in range(50):
            try:
                version = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json/version", timeout=1))
                break
            except Exception:
                time.sleep(0.3)
        if not version:
            print("CDP 端口不可达")
            return 2
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=50 * 1024 * 1024) as ws:
            target = await send(ws, "Target.createTarget", {"url": "about:blank"})
            tid = target["result"]["targetId"]
            sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
            await send(ws, "Runtime.enable", session=sess)
            await send(ws, "Page.enable", session=sess)
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def wait_cards(n, timeout=20):
                for _ in range(int(timeout * 2)):
                    c = await ev("document.querySelectorAll('.asset-card').length") or 0
                    if c == n:
                        return c
                    await asyncio.sleep(0.5)
                return await ev("document.querySelectorAll('.asset-card').length") or 0

            fails = []

            # 等首屏（全部视口 6 卡）
            if await wait_cards(6) != 6:
                print("body 片段:", (await ev("document.body.innerHTML.slice(0, 300)")) or "")
                print("FAIL: 应用未渲染（全部视口 ≠ 6 卡）")
                return 3

            # —— 断言①：区块标题递归文件夹总数（项目 = 4 / 目录 = 2）——
            titles = await ev("""[...document.querySelectorAll('.library-view .st.block')]
              .map((s) => ({ t: s.querySelector('.txt')?.textContent, n: s.querySelector('.num')?.textContent }))""")
            print("区块标题:", json.dumps(titles, ensure_ascii=False))
            tmap = {x["t"]: x["n"] for x in (titles or [])}
            if tmap.get("项目") != "4":
                fails.append(f"「项目」区块标题 = {tmap.get('项目')}，期望递归总数 4")
            if tmap.get("目录") != "2":
                fails.append(f"「目录」区块标题 = {tmap.get('目录')}，期望递归总数 2")

            read_idx = "(() => ({ cur: document.querySelector('.preview-titlebar .idx-cur')?.textContent, total: document.querySelector('.preview-titlebar .idx-total')?.textContent }))()"
            # 按钮族类名 2026-09-23 后 = .ic-btn（旧 .icon-btn 作废）；下一个 = btn-group 最后一钮
            next_btn = "[...document.querySelectorAll('.preview-titlebar .btn-group .ic-btn')].at(-1)"
            back_btn = "document.querySelector('.preview-titlebar .nav-group .ic-btn')"

            async def open_preview_and_read():
                await ev("document.querySelector('.asset-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
                for _ in range(20):
                    await asyncio.sleep(0.5)
                    v = await ev(read_idx)
                    if v and v.get("total"):
                        return v
                return await ev(read_idx)

            # —— 断言②：全部视口预览 = 1/6（基线，首视口缓存值）——
            v = await open_preview_and_read()
            print("全部视口预览:", json.dumps(v, ensure_ascii=False))
            if not v or v.get("cur") != "1" or v.get("total") != "6":
                fails.append(f"全部视口预览 = {v}，期望 1/6")
            await ev(f"{back_btn}.click()")
            await asyncio.sleep(1)

            # —— 断言③：项目D（1 文件）预览 = 1/1，下一个禁用（核心：总数随视口刷新）——
            await ev("document.querySelector('[data-folder-id=\"FD\"]').click()")
            if await wait_cards(1) != 1:
                fails.append("项目D 视口 ≠ 1 卡")
            v = await open_preview_and_read()
            disabled = await ev(f"{next_btn}?.disabled")
            print("项目D 预览:", json.dumps(v, ensure_ascii=False), "下一个禁用:", disabled)
            if not v or v.get("cur") != "1" or v.get("total") != "1":
                fails.append(f"项目D 预览 = {v}，期望 1/1（总数未随视口刷新）")
            if disabled is not True:
                fails.append(f"项目D 单文件下一个应禁用，实际 disabled={disabled}")
            await ev(f"{back_btn}.click()")
            await asyncio.sleep(1)

            # —— 断言④：项目A（递归 3 项）预览 = 1/3，点下一个 → 2/3 ——
            await ev("document.querySelector('[data-folder-id=\"FA\"]').click()")
            if await wait_cards(3) != 3:
                fails.append("项目A 视口（递归）≠ 3 卡")
            v = await open_preview_and_read()
            print("项目A 预览:", json.dumps(v, ensure_ascii=False))
            if not v or v.get("cur") != "1" or v.get("total") != "3":
                fails.append(f"项目A 预览 = {v}，期望 1/3")
            await ev(f"{next_btn}.click()")
            await asyncio.sleep(1.5)
            v2 = await ev(read_idx)
            print("点下一个后:", json.dumps(v2, ensure_ascii=False))
            if not v2 or v2.get("cur") != "2" or v2.get("total") != "3":
                fails.append(f"项目A 点下一个后 = {v2}，期望 2/3（导航无效）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 区块标题递归文件夹总数（4/2）+ 预览计数随视口刷新（6→1→3）+ 单文件禁用/多文件可导航")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


sys.exit(asyncio.run(main()))
