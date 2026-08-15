# 一次性复现：⑦-B 面板内拖拽前.detail-panel 为什么空（选择是否被 Esc 清掉）。
import asyncio
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9362
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => encodeURI("/@fs/" + p),
  transformCallback: (cb) => 1,
  invoke: (cmd, args) => {
    const STILL = "__STILL__";
    const folders = [
      { id: "F-DIR", name: "素材", parent: null, kind: null },
      { id: "F-ARC", name: "设定集", parent: "F-DIR", kind: "archive" },
    ];
    const placements = [
      { assetId: "A1", folderId: "F-ARC", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ];
    const nodeOf = (f) => ({ id: f.id, name: f.name, count: placements.filter((p) => p.folderId === f.id).length, kind: f.kind, block: f.parent === null ? "dir" : null, icon: null, children: folders.filter((c) => c.parent === f.id).map(nodeOf) });
    const R = {
      get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 1, folderCount: 2 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_placements: () => placements,
      list_folder_tree: () => ({ blocks: [{ id: "dir", name: "目录", icon: null }], nodes: folders.filter((f) => f.parent === null).map(nodeOf) }),
      list_archives: () => [{ folderId: "F-ARC", badge: { text: "设定", color: "#B38427" }, avatar: null, props: ["奇幻"], descriptions: [{ title: "来源", content: "官方设定集" }] }],
      list_recycle: [], list_recent_assets: [], list_inbox_snapshot: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产"], list_recent_libraries: [],
    };
    if (cmd === "save_ui_settings") return Promise.resolve(null);
    if (cmd === "get_asset_detail") {
      return Promise.resolve({
        assetId: "A1", originName: "主角-立绘.webp", ext: "webp", size: 204800, width: 800, height: 600, duration: null,
        importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00",
        tags: ["主角"], links: [], note: "",
        placements: [{ folderId: "F-ARC", blockName: "目录", path: ["素材", "设定集"], name: "主角-立绘.webp", origin: true }],
        archives: [{ folderId: "F-ARC", name: "设定集" }],
      });
    }
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(typeof v === "function" ? v() : v);
  },
};
""".replace("__STILL__", STILL)

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
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:300])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    dev_proc = None
    if not dev_server_up():
        print("启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
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
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=50 * 1024 * 1024) as ws:
            target = await send(ws, "Target.createTarget", {"url": "about:blank"})
            tid = target["result"]["targetId"]
            sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
            await send(ws, "Runtime.enable", session=sess)
            await send(ws, "Page.enable", session=sess)
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": x, "y": y, "clickCount": 1}, session=sess)

            async def click(expr):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)

            state = '(() => ({ panel: !!document.querySelector(".detail-panel"), titles: [...document.querySelectorAll(".detail-panel .detail-block .section-header .title")].map((t) => t.textContent), menu: !!document.querySelector(".app-menu") }))()'

            # 展开素材 → 进视口 → 选中档案卡片
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .expand')")
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.5)
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            await asyncio.sleep(0.8)
            print("选中档案后:", json.dumps(await ev(state), ensure_ascii=False))

            # 开 22 稿面板 → Esc（挂现场记录：bubble 晚序监听，记录 Esc 时 .app-menu 是否还在 DOM）
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            print("开面板后:", json.dumps(await ev(state), ensure_ascii=False))
            await ev("""window.__escLog = [];
              window.addEventListener('keydown', (e) => { if (e.key === 'Escape') window.__escLog.push({ phase: 'bubble-late', menu: !!document.querySelector('.app-menu') }); });
              window.addEventListener('keydown', (e) => { if (e.key === 'Escape') window.__escLog.push({ phase: 'capture-late', menu: !!document.querySelector('.app-menu') }); }, true);""")
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.3)
            print("Esc 现场:", json.dumps(await ev("window.__escLog")))
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)
            print("Esc 后:", json.dumps(await ev(state), ensure_ascii=False))

            # 重选档案卡片 + dump 形态与区块高度
            await ev("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')?.click()")
            await asyncio.sleep(0.8)
            diag = await ev("""(() => {
              const p = document.querySelector('.detail-panel');
              if (!p) return { panel: false };
              return {
                panel: true,
                linkItems: p.querySelectorAll('.section-links .link-item').length,
                blocks: [...p.querySelectorAll('.detail-block')].map((b) => ({
                  t: b.querySelector('.section-header .title')?.textContent ?? '(pinned)',
                  h: b.offsetHeight, pinned: b.classList.contains('pinned'), notes: b.classList.contains('detail-notes'),
                })),
              };
            })()""")
            print("重选后诊断:", json.dumps(diag, ensure_ascii=False))

            # 滚到底 + 提示词 header rect
            await ev("document.querySelector('.detail-body').scrollTop = 99999")
            await asyncio.sleep(0.3)
            ph = await ev("""(() => {
              const h = [...document.querySelectorAll('.detail-panel .detail-block')].find((b) => b.querySelector('.section-header .title')?.textContent === '提示词')?.querySelector('.section-header');
              if (!h) return null;
              const b = h.getBoundingClientRect();
              return { x: b.left + b.width / 2, y: b.top + b.height / 2, iw: window.innerWidth, ih: window.innerHeight };
            })()""")
            print("提示词 header rect:", json.dumps(ph))
            if ph:
                await mouse("mousePressed", ph["x"], ph["y"])
                await asyncio.sleep(0.1)
                for dy in (-40, -80, -120):
                    await mouse("mouseMoved", ph["x"], ph["y"] + dy)
                    await asyncio.sleep(0.1)
                await mouse("mouseReleased", ph["x"], ph["y"] - 120)
                await asyncio.sleep(0.6)
            print("面板内拖拽后:", json.dumps(await ev(state), ensure_ascii=False))
            print("拖拽后高度:", json.dumps(await ev("""[...document.querySelectorAll('.detail-panel .detail-block')].map((b) => (b.querySelector('.section-header .title')?.textContent ?? '(pinned)') + ':' + b.offsetHeight)"""), ensure_ascii=False))
            return 0
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


import sys
sys.exit(asyncio.run(main()))
