# CDP 实测：切换资源库菜单 08 稿库 item（2026-08-12）
# ① 库 item 结构 = 抓手 + 品牌徽章 + 库名 + 当前库对勾 + 删除×；
# ② 抓手拖动第二个库到第一位：让位 transform + pointerup 后 reorder_libraries(["设计资产","动画资产"])；
# ③ 删除× → 菜单关 + 确认弹窗 → 删除 → delete_library 被调。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9341
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { reorder_libraries: [], delete_library: [] };
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    if (cmd === "reorder_libraries") window.__calls.reorder_libraries.push(args.order);
    if (cmd === "delete_library") { window.__calls.delete_library.push(args.name); return Promise.resolve(["设计资产"]); }
    const LIB = { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 0 };
    const R = {
      get_current_library: LIB,
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产", "设计资产"],
      list_recent_libraries: [],
      reorder_libraries: ["设计资产", "动画资产"],
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

            async def mouse(t, x, y, button="left"):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "buttons": 1, "x": x, "y": y, "clickCount": 1}, session=sess)

            # 打开切换资源库菜单
            rect = await ev("(() => { const b = document.querySelector('.library-switch .icon-btn').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            await mouse("mousePressed", rect["x"], rect["y"]); await mouse("mouseReleased", rect["x"], rect["y"])
            await asyncio.sleep(0.8)

            struct = await ev("[...document.querySelectorAll('.app-menu .lib-item')].map((it) => ({ label: it.querySelector('.label')?.textContent, grip: !!it.querySelector('.grip'), badge: !!it.querySelector('.badge'), check: !!it.querySelector('.check'), remove: !!it.querySelector('.remove') }))")
            print("库 item 结构:", json.dumps(struct, ensure_ascii=False))
            import base64
            r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open("tools/style-preview/library-menu.png", "wb") as f:
                f.write(base64.b64decode(r["result"]["data"]))
            print("截图: library-menu")

            # 拖动第二个库（设计资产）抓手到第一位
            grip = await ev("(() => { const g = document.querySelectorAll('.app-menu .lib-item')[1].querySelector('.grip').getBoundingClientRect(); return { x: g.left + g.width / 2, y: g.top + g.height / 2 }; })()")
            await mouse("mousePressed", grip["x"], grip["y"])
            for step in range(1, 6):
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": grip["x"], "y": grip["y"] - step * 10, "buttons": 1}, session=sess)
                await asyncio.sleep(0.05)
            mid = await ev("(() => { const items = [...document.querySelectorAll('.app-menu .lib-item')]; return items.map((it) => it.style.transform || '-'); })()")
            print("拖动中 transform:", json.dumps(mid))
            await mouse("mouseReleased", grip["x"], grip["y"] - 50)
            await asyncio.sleep(0.6)
            print("reorder 调用:", await ev("JSON.stringify(window.__calls.reorder_libraries)"))
            print("菜单新顺序:", await ev("JSON.stringify([...document.querySelectorAll('.app-menu .lib-item .label')].map((l) => l.textContent))"))

            # 删除×（设计资产，现在第一位）
            rm = await ev("(() => { const b = document.querySelectorAll('.app-menu .lib-item')[0].querySelector('.remove').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            await mouse("mousePressed", rm["x"], rm["y"]); await mouse("mouseReleased", rm["x"], rm["y"])
            await asyncio.sleep(0.6)
            print("菜单已关:", await ev("!document.querySelector('.app-menu')"))
            print("确认弹窗:", await ev("document.querySelector('.app-modal.alert .title')?.textContent ?? null"))
            # 确认删除
            btn = await ev("(() => { const b = [...document.querySelectorAll('.app-modal .app-btn')].find((x) => x.textContent.includes('删除')).getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            await mouse("mousePressed", btn["x"], btn["y"]); await mouse("mouseReleased", btn["x"], btn["y"])
            await asyncio.sleep(0.6)
            print("delete_library 调用:", await ev("JSON.stringify(window.__calls.delete_library)"))
            print("toast:", await ev("document.querySelector('.toast-stack')?.textContent?.trim() ?? null"))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
