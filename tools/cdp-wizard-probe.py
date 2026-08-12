# CDP 实测：2.1 初次打开向导（38 稿五帧走查，2026-08-12）
# Edge 无头 + __TAURI_INTERNALS__ 桩（无空间无库 → 强制向导；plugin:dialog|open 桩返回假路径）。
# 走查：帧 0 入口 → 第一步（空名报错 → 输名 → 未选路径报错 → 选路径 → 下一步创建空间）
# → 第二步（库名 + 自动导入开关）→ 第三 / 四步占位 → 完成 toast + 向导关闭；关键帧截图留档。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9334
URL = "http://localhost:5373"
SHOT_DIR = "tools/style-preview"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: null,
      get_ui_settings: {},
      detect_doc_apps: [],
      list_recent_libraries: [],
      list_placements: [],
      list_folder_tree: [],
      list_recycle: [],
      list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: [],
      create_space: { name: args?.name ?? "测试空间", path: "E:/AniKitTest/" + (args?.name ?? "测试空间") },
      create_library: { name: args?.name ?? "测试库", path: "E:/AniKitTest/测试空间/" + (args?.name ?? "测试库"), assetCount: 0, folderCount: 0 },
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve("E:/AniKitTest");
    if (Object.prototype.hasOwnProperty.call(R, cmd)) {
      const v = typeof R[cmd] === "function" ? R[cmd]() : R[cmd];
      console.warn("[stub]", cmd, JSON.stringify(v));
      return Promise.resolve(v);
    }
    return Promise.resolve(null);
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

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                import base64, os
                os.makedirs(SHOT_DIR, exist_ok=True)
                with open(f"{SHOT_DIR}/{name}.png", "wb") as f:
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

            state = lambda: ev("({ step0: !!document.querySelector('.entry-actions'), stepTitle: document.querySelector('.step-name')?.textContent ?? null, err: document.querySelector('.step-error')?.textContent ?? null, fill: document.querySelector('.progress-fill')?.style.width ?? null, card: !!document.querySelector('.wizard-card'), toast: document.querySelector('.toast-stack')?.textContent?.trim() ?? null })")

            print("== 帧 0 入口 ==", json.dumps(await state(), ensure_ascii=False))
            await shot("wizard-0-entry")

            # 点"新建储存空间" → 第一步
            await click("[...document.querySelectorAll('.entry-actions .app-btn')].find((b) => b.textContent.includes('新建储存空间'))")
            print("== 第一步 ==", json.dumps(await state(), ensure_ascii=False))

            # 空名下一步 → 错误
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            print("空名下一步:", json.dumps(await state(), ensure_ascii=False))

            # 输入名称
            await ev("(() => { const i = document.querySelector('.step-space .app-input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '测试空间'); i.dispatchEvent(new Event('input', { bubbles: true })); return i.value; })()")
            # 未选路径下一步 → 错误
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            print("未选路径下一步:", json.dumps(await state(), ensure_ascii=False))

            # 选路径（桩返回 E:/AniKitTest）→ 显示已选路径 → 下一步创建空间
            await click("[...document.querySelectorAll('.step-space .app-btn')].find((b) => b.textContent.includes('选择保存路径'))")
            print("已选路径:", await ev("document.querySelector('.chosen-path')?.textContent ?? null"))
            await shot("wizard-1-space")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await asyncio.sleep(0.5)
            print("== 第二步 ==", json.dumps(await state(), ensure_ascii=False))

            # 输库名 + 开自动导入 → 下一步创建库
            await ev("(() => { const i = document.querySelector('.step-library .app-input'); const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set; set.call(i, '测试库'); i.dispatchEvent(new Event('input', { bubbles: true })); return i.value; })()")
            await click("document.querySelector('.step-library .app-switch')")
            await shot("wizard-2-library")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await asyncio.sleep(0.5)
            print("== 第三步 ==", json.dumps(await state(), ensure_ascii=False))
            await shot("wizard-3-placeholder")

            # 上一步回退 → 再下一步（createdLibName 不重建）
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('上一步'))")
            print("回退第二步:", json.dumps(await state(), ensure_ascii=False))
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            await asyncio.sleep(0.5)
            print("再进第三步:", json.dumps(await state(), ensure_ascii=False))

            # 第四步 → 完成
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('下一步'))")
            print("== 第四步 ==", json.dumps(await state(), ensure_ascii=False))
            await shot("wizard-4-final")
            await click("[...document.querySelectorAll('.step-row .app-btn')].find((b) => b.textContent.includes('完成'))")
            await asyncio.sleep(1)
            print("== 完成后 ==", json.dumps(await state(), ensure_ascii=False))
            print("侧栏库名:", await ev("document.querySelector('.lib-name')?.textContent ?? null"))
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
