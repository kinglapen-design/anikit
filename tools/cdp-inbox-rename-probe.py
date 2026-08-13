# CDP 实测：工程文件卡片双击重命名（2026-08-13 用户反馈"双击无法进入"）
# 桩一条收件箱工程文件 → 自动导入视图 → 双击名称 → 断言输入框出现/聚焦/词干选区（后缀不选）
# → 输入新名 Enter → 断言 rename_inbox_entry 收到（newName 带原扩展名）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9349
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { rename_inbox_entry: [] };
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    if (cmd === "rename_inbox_entry") window.__calls.rename_inbox_entry.push(args);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: true };
    const R = {
      get_current_library: LIB,
      get_current_space: { name: "S", autoImport: true },
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      list_inbox_snapshot: [{ name: "无限穿越01.fla", isDir: false, size: 2555000, modifiedAt: 0, autoImportable: false, occupied: false, iconPath: null }],
      rename_inbox_entry: null,
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
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

            async def mouse(expr, clicks):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for i, cc in enumerate(clicks):
                    for t in ("mousePressed", "mouseReleased"):
                        await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": cc}, session=sess)
                    await asyncio.sleep(0.08)
                return True

            # 进"自动导入"视图
            await mouse("[...document.querySelectorAll('.quick-item, [class*=quick] *')].find((m) => m.textContent.trim() === '自动导入')", [1])
            await asyncio.sleep(1.0)
            has_card = await ev("!!document.querySelector('.disk-card')")
            print("工程文件卡片:", has_card)
            if not has_card:
                # 兜底：侧栏项选择器不定，dump 快速访问 DOM
                print("quick DOM:", await ev("[...document.querySelectorAll('[class*=quick]')].map((e) => e.className.toString()).slice(0,5)"))
                return 3

            # 双击前读名称光标（双击后 title 被 input 替换，读不到）
            cursor_before = await ev("getComputedStyle(document.querySelector('.disk-card .title')).cursor")

            # 双击名称
            await mouse("document.querySelector('.disk-card .title')", [1, 2])
            await asyncio.sleep(0.6)
            st = await ev("""(() => {
              const i = document.querySelector('.disk-card .rename-input');
              return {
                exists: !!i,
                focused: document.activeElement === i,
                value: i?.value ?? null,
                sel: i ? [i.selectionStart, i.selectionEnd] : null,
              };
            })()""")
            print("双击后:", json.dumps(st, ensure_ascii=False), "名称光标:", cursor_before)
            fails = []
            if not st["exists"] or not st["focused"]:
                fails.append("输入框未出现/未聚焦")
            if st["value"] != "无限穿越01.fla":
                fails.append(f"初值错 {st['value']}")
            if st["sel"] != [0, 6]:
                fails.append(f"词干选区错 {st['sel']}（期望 [0,6]，.fla 不入选）")
            if cursor_before != "text":
                fails.append(f"名称悬停光标 {cursor_before} ≠ text")

            if not fails:
                # 输入新名（替换词干选区）→ Enter → 断言命令带原扩展名
                await send(ws, "Input.insertText", {"text": "新场景"}, session=sess)
                await asyncio.sleep(0.3)
                await ev("document.querySelector('.disk-card .rename-input')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))")
                await asyncio.sleep(0.8)
                calls = await ev("JSON.stringify(window.__calls.rename_inbox_entry)")
                print("rename 调用:", calls)
                try:
                    args = json.loads(calls)[0]
                    if args.get("name") != "无限穿越01.fla" or args.get("newName") != "新场景.fla":
                        fails.append(f"参数错 {calls}")
                except Exception:
                    fails.append(f"未调用 {calls}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 双击进入 + 词干选区 + 提交带原扩展名")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
