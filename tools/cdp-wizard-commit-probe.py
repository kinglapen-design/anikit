# CDP 实测：向导"完成才落盘"定案（2026-08-12 用户走查：输完路径没跑完不应算完成）
# 场景 A：第一步/第二步"下一步"后桩计数 create_space/create_library 均为 0（仅暂存）；
#         第四步"完成"后各 1 次，toast + 向导关闭。
# 场景 B（新标签页）：走到第二步后点 X → exit_app 被调且 create_* 仍 0（中途关闭零落盘）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9339
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { create_space: 0, create_library: 0, exit_app: 0 };
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    if (cmd in window.__calls) window.__calls[cmd]++;
    const R = {
      get_current_library: null, get_ui_settings: {}, detect_doc_apps: [],
      list_recent_libraries: [], list_placements: [], list_folder_tree: [],
      list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: [],
      create_space: { name: args?.name ?? "S", path: "E:/T/" + (args?.name ?? "S") },
      create_library: { name: args?.name ?? "L", path: "E:/T/S/" + (args?.name ?? "L"), assetCount: 0, folderCount: 0, autoImport: !!args?.auto_import },
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve("E:/T");
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
            async def new_session():
                target = await send(ws, "Target.createTarget", {"url": "about:blank"})
                tid = target["result"]["targetId"]
                sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
                await send(ws, "Runtime.enable", session=sess)
                await send(ws, "Page.enable", session=sess)
                await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
                await send(ws, "Page.navigate", {"url": URL}, session=sess)
                await asyncio.sleep(6)
                return sess

            async def ev(sess, expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def click(sess, expr):
                rect = await ev(sess, f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)
                return True

            async def type_input(sess, sel, value):
                await ev(sess, f"(() => {{ const i = document.querySelector('{sel}'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '{value}'); i.dispatchEvent(new Event('input', {{ bubbles: true }})); }})()")

            calls = lambda sess: ev(sess, "JSON.stringify(window.__calls)")

            # —— 场景 A：完成才落盘 ——
            a = await new_session()
            await click(a, "[...document.querySelectorAll('.entry-actions .app-btn')].find((b) => b.textContent.includes('新建储存空间'))")
            await type_input(a, ".step-space .app-input", "S")
            await click(a, "[...document.querySelectorAll('.step-space .app-btn')].find((b) => b.textContent.includes('选择保存路径'))")
            await click(a, "[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await type_input(a, ".step-library .app-input", "L")
            await click(a, "[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            print("A 第二步后（应全 0）:", await calls(a))
            await click(a, "[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")  # 第三步
            await click(a, "[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('完成'))")  # 第四步完成
            await asyncio.sleep(0.8)
            print("A 完成后（create 各 1）:", await calls(a))
            print("A 向导已关:", await ev(a, "!document.querySelector('.wizard-card')"))
            print("A toast:", await ev(a, "document.querySelector('.toast-stack')?.textContent?.trim() ?? null"))

            # —— 场景 B：中途 X = 退出且零落盘 ——
            b = await new_session()
            await click(b, "[...document.querySelectorAll('.entry-actions .app-btn')].find((b) => b.textContent.includes('新建储存空间'))")
            await type_input(b, ".step-space .app-input", "S")
            await click(b, "[...document.querySelectorAll('.step-space .app-btn')].find((b) => b.textContent.includes('选择保存路径'))")
            await click(b, "[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await type_input(b, ".step-library .app-input", "L")
            await click(b, "document.querySelector('.wizard-card .titlebar .icon-btn')")
            await asyncio.sleep(0.5)
            print("B 第二步后 X（exit 1，create 全 0）:", await calls(b))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
