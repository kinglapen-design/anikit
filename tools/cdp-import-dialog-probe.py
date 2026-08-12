# CDP 实测：导入三弹窗 = figma 27_导入 稿（2026-08-12，2.2）
# 桩有库进主界面 → 新增菜单"导入本地文件" → 导入文件弹窗（§1 单组）截图
# → "导入本地文件夹"（classify 桩改 folder）→ 导入文件夹弹窗（§2 双组+分隔线）截图
# → 确认后 detect_sequences 桩命中 → 检测到序列帧弹窗（§3 单组，来源组不重复）截图。
# 校验：plain 壳 / 标题 / 单选项 / 按钮序（确认左取消右）/ 88 定宽 / 分隔线。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9345
URL = "http://localhost:5373"

STUB = r"""
window.__classifyKind = "files";
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      classify_paths: { kind: window.__classifyKind, fileCount: 2 },
      detect_sequences: [{ name: "走路", ext: "png", frameCount: 24 }],
      import_paths: 1,
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve(["E:/T/a.png", "E:/T/b.png"]);
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

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            async def open_import(menu_label):
                await click("document.querySelector('.add-btn-wrap .icon-btn')")
                ok = await click(f"[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('{menu_label}'))")
                await asyncio.sleep(0.8)
                return ok

            INFO = """({
              plain: !!document.querySelector('.app-modal.plain'),
              title: document.querySelector('.app-modal .title')?.textContent ?? null,
              groups: document.querySelectorAll('.app-modal .option-group').length,
              divided: !!document.querySelector('.app-modal .option-group.divided'),
              radios: [...document.querySelectorAll('.app-modal .app-radio')].map((r) => ({ t: r.textContent.trim(), on: r.classList.contains('on') })),
              buttons: [...document.querySelectorAll('.app-modal .app-btn')].map((b) => b.textContent.trim()),
              btnWidth: document.querySelector('.app-modal .app-btn')?.getBoundingClientRect().width ?? null,
              width: document.querySelector('.app-modal')?.getBoundingClientRect().width ?? null,
            })"""

            fails = []

            # ① 导入文件（§1：单组，无分隔线）
            await open_import("导入本地文件")
            info = json.loads(json.dumps(await ev(INFO)))
            print("导入文件:", json.dumps(info, ensure_ascii=False))
            if not (info["plain"] and info["title"] == "导入文件" and info["groups"] == 1 and not info["divided"]):
                fails.append("导入文件结构")
            if info["buttons"] != ["确认", "取消"] or info["btnWidth"] != 88:
                fails.append("按钮序/定宽")
            await shot("import-files")
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('取消'))")

            # ② 导入文件夹（§2：双组，来源组带分隔线，默认 复制+保留层级）
            await ev("window.__classifyKind = 'folder'")
            await open_import("导入本地文件夹")
            info = json.loads(json.dumps(await ev(INFO)))
            print("导入文件夹:", json.dumps(info, ensure_ascii=False))
            on_labels = [r["t"] for r in info["radios"] if r["on"]]
            if not (info["plain"] and info["title"] == "导入文件夹" and info["groups"] == 2 and info["divided"]):
                fails.append("导入文件夹结构")
            if on_labels != ["复制", "保留层级"]:
                fails.append(f"默认选中态 {on_labels}")
            await shot("import-folder")

            # ③ 确认 → 检测到序列帧（§3：单组，来源组不重复）
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(1.0)
            info = json.loads(json.dumps(await ev(INFO)))
            print("序列帧:", json.dumps(info, ensure_ascii=False))
            if not (info["plain"] and info["title"] == "检测到序列帧" and info["groups"] == 1):
                fails.append("序列帧结构")
            if [r["t"] for r in info["radios"]] != ["打包导入", "分散导入"]:
                fails.append("序列帧单选项")
            await shot("import-sequence")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 三弹窗结构/默认态/按钮均贴稿")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
