# CDP 实测：向导第三步占位图标（2026-08-12，图标压暗 / 悬停彩色 / 点击无反应）
# 桩 detect_browsers 返回彩色块 data URL（红/绿/蓝 1px PNG），走到第三步：
# 截图 A = 默认压暗（灰）；mouseMoved 悬停中间块 → 截图 B = 该块彩色，其余仍压暗。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9336
URL = "http://localhost:5373"

# 1x1 纯色 PNG（红 / 绿 / 蓝）
def png(rgb: str) -> str:
    # 预生成的 1x1 纯色 png 模板（Pillow 造）
    import io
    try:
        from PIL import Image
        buf = io.BytesIO()
        Image.new("RGB", (1, 1), tuple(int(rgb[i : i + 2], 16) for i in (0, 2, 4))).save(buf, "PNG")
        return "data:image/png;base64," + base64.b64encode(buf.getvalue()).decode()
    except ImportError:
        # 无 Pillow 退化为 1x1 红
        return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="

STUB = (
    r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: null,
      get_ui_settings: {},
      list_recent_libraries: [],
      list_placements: [],
      list_folder_tree: [],
      list_recycle: [],
      list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: [],
      create_space: { name: args?.name ?? "S", path: "E:/T/" + (args?.name ?? "S") },
      create_library: { name: args?.name ?? "L", path: "E:/T/S/" + (args?.name ?? "L"), assetCount: 0, folderCount: 0, autoImport: false },
      detect_doc_apps: [],
      detect_browsers: [
        { id: "chrome", installed: true, icon: "__RED__" },
        { id: "firefox", installed: true, icon: "__GREEN__" },
        { id: "edge", installed: true, icon: "__BLUE__" },
      ],
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve("E:/T");
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
};
"""
    .replace("__RED__", png("e02b2b"))
    .replace("__GREEN__", png("2ba02e"))
    .replace("__BLUE__", png("2b5ce0"))
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
        m = raw.get("method", "")
        if m == "Runtime.exceptionThrown":
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

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            async def click(expr):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)
                return True

            async def type_input(sel, value):
                await ev(f"(() => {{ const i = document.querySelector('{sel}'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '{value}'); i.dispatchEvent(new Event('input', {{ bubbles: true }})); return i.value; }})()")

            # 帧 0 → 第一步
            await click("[...document.querySelectorAll('.entry-actions .app-btn')].find((b) => b.textContent.includes('新建储存空间'))")
            await type_input(".step-space .app-input", "S")
            await click("[...document.querySelectorAll('.step-space .app-btn')].find((b) => b.textContent.includes('选择保存路径'))")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await asyncio.sleep(0.4)
            # 第二步 → 第三步
            await type_input(".step-library .app-input", "L")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await asyncio.sleep(0.8)

            info = await ev("({ step: document.querySelector('.step-name')?.textContent, icons: [...document.querySelectorAll('.step-placeholder .block .icon')].map((i) => getComputedStyle(i).filter), blocks: document.querySelectorAll('.step-placeholder .block').length })")
            print("第三步:", json.dumps(info, ensure_ascii=False))
            await shot("wizard-icons-dim")

            # 悬停中间块 → filter none
            rect = await ev("(() => { const b = document.querySelectorAll('.step-placeholder .block')[1].getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": rect["x"], "y": rect["y"]}, session=sess)
            await asyncio.sleep(0.5)
            hover = await ev("[...document.querySelectorAll('.step-placeholder .block .icon')].map((i) => getComputedStyle(i).filter)")
            print("悬停后 filter:", json.dumps(hover))
            await shot("wizard-icons-hover")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
