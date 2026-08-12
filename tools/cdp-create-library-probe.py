# CDP 实测：菜单"新建资源库" = 16 稿独立弹窗（2026-08-12）
# 桩有库进主界面 → 侧栏"切换资源库" → 菜单"新建资源库" → 弹窗（confirm 形态/标题/按钮序确认左取消右）
# → 输名确认 → create_library(auto_import=false) 被调 → 弹窗关 + toast + 侧栏库名更新。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9340
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { create_library: [] };
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    if (cmd === "create_library") window.__calls.create_library.push(args);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      create_library: { name: args?.name ?? "新库", path: "E:/T/S/" + (args?.name ?? "新库"), assetCount: 0, folderCount: 0, autoImport: false },
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
};
"""

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
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def click(expr):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)
                return True

            # 侧栏"切换资源库"按钮 → 菜单 → 新建资源库
            await click("document.querySelector('.library-switch .icon-btn')")
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('新建资源库'))")
            await asyncio.sleep(0.5)
            info = await ev("({ modal: !!document.querySelector('.app-modal.confirm'), title: document.querySelector('.app-modal .title')?.textContent ?? null, buttons: [...document.querySelectorAll('.app-modal .app-btn')].map((b) => b.textContent.trim()), wizard: !!document.querySelector('.wizard-card') })")
            print("弹窗:", json.dumps(info, ensure_ascii=False))
            if not info.get("modal") or info.get("wizard"):
                print("FAIL：未出 16 稿弹窗或误弹向导")
                return 3

            # 输名 → 确认（截图在确认前，留弹窗本体）
            await ev("(() => { const i = document.querySelector('.app-modal .app-input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '新库'); i.dispatchEvent(new Event('input', { bubbles: true })); })()")
            r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open("tools/style-preview/create-library-modal.png", "wb") as f:
                f.write(base64.b64decode(r["result"]["data"]))
            print("截图: create-library-modal")
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(0.8)
            print("create_library 调用:", await ev("JSON.stringify(window.__calls.create_library)"))
            print("弹窗已关:", await ev("!document.querySelector('.app-modal')"))
            print("toast:", await ev("document.querySelector('.toast-stack')?.textContent?.trim() ?? null"))
            print("侧栏库名:", await ev("document.querySelector('.lib-name')?.textContent ?? null"))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
