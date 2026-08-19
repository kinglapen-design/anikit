# CDP 实测：回收站 item 右键调不出（2026-08-11）
# Edge 无头打开 vite dev 页面 + __TAURI_INTERNALS__ 桩（假库数据：回收站 1 条 123.md），
# 点"回收站"快速访问项 → 右键 123.md 卡片 → 检查 .app-menu 是否出现 + 控制台异常。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9333
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const entry = {
      assetId: "A1", folderId: "recycle", name: "123.md", w: null, h: null, ext: "md",
      thumbPath: null, thumbKind: null, origin: false, frameCount: null, animated: false,
      duration: null, bpm: null, importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
      deletedAt: "2026-08-11T10:00:00+08:00", placementCount: 1,
    };
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 1, folderCount: 0 },
      get_ui_settings: {},
      list_folder_tree: { blocks: [], nodes: [] },
      list_placements: [],
      list_recycle: [entry],
      list_recent_assets: [],
      list_recent_libraries: [
        { name: "动画素材库", path: "E:/test-lib", lastOpenedAt: "2026-08-11T10:00:00+08:00" },
        { name: "备用库", path: "E:/test-lib-b", lastOpenedAt: "2026-08-01T10:00:00+08:00" },
      ],
      detect_doc_apps: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    // console.warn("[stub invoke]", cmd);
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
        # 事件打印（console / 异常）
        m = raw.get("method", "")
        if m == "Runtime.consoleAPICalled":
            args = raw["params"].get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            print(f"[console.{raw['params']['type']}]", text[:300])
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
            await asyncio.sleep(6)  # 等 vite 编译 + 首屏渲染

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            print("== 页面状态 ==")
            print("库标题栏:", await ev("document.querySelector('.lib-name')?.textContent ?? null"))
            print("快速访问项数:", await ev("document.querySelectorAll('.quick .tree-item').length"))

            rect = await ev("""
              (() => {
                const btn = document.querySelector('.bar-title .icon-btn');
                if (!btn) return null;
                const b = btn.getBoundingClientRect();
                return { x: b.left + b.width / 2, y: b.top + b.height / 2 };
              })()
            """)
            print("汉堡按钮 rect:", rect)
            for t in ("mousePressed", "mouseReleased"):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": "left", "buttons": 1,
                    "x": rect["x"], "y": rect["y"], "clickCount": 1,
                }, session=sess)
            await asyncio.sleep(1)
            print("汉堡菜单:", await ev("document.querySelector('.app-menu')?.textContent?.replace(/\s+/g, ' ').slice(0, 80)"))
            print("汉堡菜单宽:", await ev("document.querySelector('.app-menu')?.style.width"))

            # hover "资源库" 项出二级
            hover = await ev("""
              (() => {
                const items = [...document.querySelectorAll('.app-menu .menu-item')];
                const it = items.find((el) => el.textContent.trim() === '资源库');
                if (!it) return null;
                it.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
                const r = it.getBoundingClientRect();
                return { x: r.left, y: r.top };
              })()
            """)
            print("hover 资源库:", hover)
            await asyncio.sleep(1)
            menus = await ev("""
              [...document.querySelectorAll('.app-menu')].map((m) => ({
                text: m.textContent.replace(/\s+/g, ' ').slice(0, 90),
                width: m.style.width,
              }))
            """)
            print("菜单元素:", json.dumps(menus, ensure_ascii=False))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
