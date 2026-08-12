# CDP 实测：向导第一步错误行遮挡按钮点击修复验证（2026-08-12 走查）
# 到第一步 → 输名称 → 点下一步触发错误行 → 点"选择保存路径"按钮**底缘 padding 区**
# （非文字中心，y = rect.bottom - 2）→ chosen-path 出现 = 修复生效。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9337
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: null, get_ui_settings: {}, detect_doc_apps: [],
      list_recent_libraries: [], list_placements: [], list_folder_tree: [],
      list_recycle: [], list_recent_assets: [], detect_browsers: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
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

            async def click_xy(x, y):
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": x, "y": y, "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)

            async def click(expr):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:60])
                    return False
                await click_xy(rect["x"], rect["y"])
                return True

            # 帧 0 → 第一步，输名称，点下一步触发错误行
            await click("[...document.querySelectorAll('.entry-actions .app-btn')].find((b) => b.textContent.includes('新建储存空间'))")
            await ev("(() => { const i = document.querySelector('.step-space .app-input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, 'S'); i.dispatchEvent(new Event('input', { bubbles: true })); })()")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            err = await ev("document.querySelector('.step-error')?.textContent ?? null")
            print("错误行:", err)

            # 点按钮底缘 padding 区（y = bottom - 2，修复前此处落在 .step-error 上点不中）
            edge = await ev("(() => { const b = [...document.querySelectorAll('.step-space .app-btn')].find((x) => x.textContent.includes('选择保存路径')).getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.bottom - 2 }; })()")
            hit = await ev(f"document.elementFromPoint({edge['x']}, {edge['y']})?.closest('.app-btn') ? 'button' : (document.elementFromPoint({edge['x']}, {edge['y']})?.className ?? 'null')")
            print("底缘命中元素:", hit)
            await click_xy(edge["x"], edge["y"])
            chosen = await ev("document.querySelector('.chosen-path')?.textContent ?? null")
            print("底缘点击后 chosen-path:", chosen)
            if chosen:
                print("PASS：边缘点击触发路径选择")
                return 0
            print("FAIL：边缘点击未触发")
            return 3
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
