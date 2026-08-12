# CDP 实测：切库 keepOpen（2026-08-12 用户定：状态切换类执行后菜单保持）
# 打开菜单 → 点"设计资产" → 菜单仍开 + 对勾移到设计资产 → 点"动画资产"切回 → 菜单仍开 + 勾回；
# 再点"新建资源库"（一次性任务类）→ 菜单关闭（对照组）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9342
URL = "http://localhost:5373"

STUB = r"""
window.__current = "动画资产";
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    if (cmd === "open_library") window.__current = args?.name;
    const LIB = () => ({ name: window.__current, path: "E:/T/S/" + window.__current, assetCount: 0, folderCount: 0 });
    const R = {
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产", "设计资产"],
      list_recent_libraries: [],
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
    };
    if (cmd === "get_current_library") return Promise.resolve(LIB());
    if (cmd === "open_library") return Promise.resolve(LIB());
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
                await asyncio.sleep(0.7)
                return True

            state = lambda: ev("({ menuOpen: !!document.querySelector('.app-menu'), current: document.querySelector('.app-menu .lib-item.current .label')?.textContent ?? null })")

            # 打开切换资源库菜单
            await click("document.querySelector('.library-switch .icon-btn')")
            print("开菜单:", json.dumps(await state(), ensure_ascii=False))
            # 点设计资产 → 仍开 + 勾移动
            await click("[...document.querySelectorAll('.app-menu .lib-item')].find((it) => it.textContent.includes('设计资产'))")
            print("切设计资产:", json.dumps(await state(), ensure_ascii=False))
            # 点动画资产切回 → 仍开 + 勾回
            await click("[...document.querySelectorAll('.app-menu .lib-item')].find((it) => it.textContent.includes('动画资产'))")
            print("切回动画资产:", json.dumps(await state(), ensure_ascii=False))
            # 对照：点"新建资源库"（一次性任务类）→ 菜单关 + 16 稿弹窗出
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('新建资源库'))")
            await asyncio.sleep(0.5)
            print("新建资源库后:", json.dumps(await ev("({ menuOpen: !!document.querySelector('.app-menu'), modal: !!document.querySelector('.app-modal.confirm') })"), ensure_ascii=False))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
