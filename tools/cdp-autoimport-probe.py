# CDP 实测：自动导入关闭时点快速访问"自动导入" → 确认弹窗（2026-08-12，2.1 开关接业务）
# 桩：当前库 autoImport=false → 点"自动导入"项应出确认弹窗而非直接进视图；
# 点"开启" → set_auto_import(true) 被调 → 弹窗关闭并进 quick:inbox 视图；截图留档。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9335
URL = "http://localhost:5373"
SHOT = "tools/style-preview/autoimport-prompt.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "测试库", path: "E:/AniKitTest/测试空间/测试库", assetCount: 0, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {},
      detect_doc_apps: [],
      list_placements: [],
      list_folder_tree: [],
      list_recycle: [],
      list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    if (cmd === "set_auto_import") {
      console.warn("[stub] set_auto_import", JSON.stringify(args));
      return Promise.resolve({ ...LIB, autoImport: args?.enabled ?? true });
    }
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
        m = raw.get("method", "")
        if m == "Runtime.consoleAPICalled":
            args = raw["params"].get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            if "[stub]" in text or raw["params"]["type"] in ("error",):
                print(f"[console.{raw['params']['type']}]", text[:200])
        elif m == "Runtime.exceptionThrown":
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
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)
                return True

            state = lambda: ev("({ modal: document.querySelector('.app-modal.confirm .title')?.textContent ?? null, selected: document.querySelector('.quick .tree-item.selected, .quick .tree-item.domain-selected')?.textContent?.trim() ?? null })")

            print("初始:", json.dumps(await state(), ensure_ascii=False))
            # 点"自动导入"快速访问项 → 应出弹窗、不进视图
            await click("[...document.querySelectorAll('.quick .tree-item')].find((el) => el.textContent.includes('自动导入'))")
            s1 = await state()
            print("点自动导入:", json.dumps(s1, ensure_ascii=False))
            if s1.get("modal") != "开启自动导入":
                print("!! 弹窗未出现或标题不符")
                return 3
            r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            import base64
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(r["result"]["data"]))
            print("截图:", SHOT)
            # 点"开启" → set_auto_import(true) + 弹窗关闭 + 进视图
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('开启'))")
            await asyncio.sleep(0.5)
            print("开启后:", json.dumps(await state(), ensure_ascii=False))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
