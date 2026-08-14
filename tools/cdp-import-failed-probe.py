# CDP 实测：导入失败弹窗 = AppModal alert 错误模板（2026-08-14 用户定，替掉旧手写壳）
# 桩有库进主界面 → 新增菜单"导入本地文件" → 导入弹窗确认 → import_paths 桩经 Channel 回调
# 推 finished（3 条失败）→ FailedListDialog 出现。
# 校验：alert 壳 + 48 error 徽章 / 标题"导入未完成"常态色 / 文件名竖排 3 行 gap 8px /
# 按钮序主左次右（重试 / 退出）/ 重试 = 按原参数重投失败路径（import_paths 二次调用带原 structure/source）
# / 退出 = 纯关闭不重投。截图留档 tools/style-preview/import-failed.png。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9348
URL = "http://localhost:5373"

# Channel 桩同 cdp-duplicate-dialog-probe：transformCallback 存 window.__cbs；
# import_paths 捕获 onEvent channel id，300ms 后推 started + finished（3 条失败）；
# __importCalls 记录每次调用参数（重试断言用）
STUB = r"""
window.__cbs = {};
window.__cbId = 0;
window.__importCalls = [];
window.__failed = [
  { path: "E:/T/损坏文件A.png", reason: "文件损坏或格式不支持" },
  { path: "E:/T/被占用B.psd", reason: "文件正被其他软件占用" },
  { path: "E:/T/超长文件名用来验证换行表现的一段非常非常非常非常长的文件名C.fla", reason: "读取失败" },
];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    (window.__cmds = window.__cmds || []).push(cmd);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      classify_paths: { kind: "files", fileCount: 3 },
      detect_sequences: [],
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve(["E:/T/损坏文件A.png", "E:/T/被占用B.psd", "E:/T/x.fla"]);
    if (cmd === "import_paths") {
      window.__importCalls.push({ paths: args.paths, structure: args.structure, source: args.source, sequenceMode: args.sequenceMode });
      const chId = typeof args.onEvent === "object" ? args.onEvent.id : Number(String(args.onEvent).split(":").pop());
      setTimeout(() => {
        const cb = window.__cbs[chId];
        if (!cb) return;
        // Channel 消息包装形状：{ index, message }（@tauri-apps/api core.js Channel）
        cb({ index: 0, message: { type: "started", total: 3 } });
        cb({ index: 1, message: { type: "finished", imported: 0, conflicts: [], failed: window.__failed } });
      }, 300);
      return Promise.resolve(1);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


async def main():
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,1200", "about:blank"],
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
                await asyncio.sleep(0.5)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            fails = []

            async def open_failed_dialog():
                await click("document.querySelector('.add-btn-wrap .icon-btn')")
                await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('导入本地文件'))")
                await asyncio.sleep(0.8)
                await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('确认'))")
                await asyncio.sleep(1.5)

            # 进弹窗：菜单"导入本地文件" → 导入弹窗确认 → 桩推 finished（3 失败）
            await open_failed_dialog()
            shown = await ev("!!document.querySelector('.app-modal.alert')")
            print("失败弹窗(alert):", shown, "import 调用:", await ev("window.__importCalls.length"))
            if not shown:
                print("FAIL: 弹窗未出现")
                return 3

            info = await ev("""({
              title: document.querySelector('.app-modal.alert .title')?.textContent ?? null,
              titleColor: getComputedStyle(document.querySelector('.app-modal.alert .title')).color,
              badge: !!document.querySelector('.app-modal.alert .alert-badge'),
              badgeSize: document.querySelector('.app-modal.alert .alert-badge')?.getBoundingClientRect().width ?? null,
              items: [...document.querySelectorAll('.fail-item')].map((e) => e.textContent),
              gap: getComputedStyle(document.querySelector('.fail-list')).rowGap,
              buttons: [...document.querySelectorAll('.app-modal.alert .actions .app-btn')].map((b) => b.textContent.trim()),
            })""")
            print("弹窗:", json.dumps(info, ensure_ascii=False))
            if info["title"] != "导入未完成":
                fails.append(f"标题 {info['title']}")
            if not info["badge"] or info["badgeSize"] != 48:
                fails.append(f"徽章 {info['badge']} {info['badgeSize']}")
            if len(info["items"]) != 3 or "——" in "".join(info["items"]) or info["items"][0] != "损坏文件A.png":
                fails.append(f"文件名清单 {info['items']}")
            if info["gap"] != "8px":
                fails.append(f"文件名间距 {info['gap']}")
            if info["buttons"] != ["重试", "退出"]:
                fails.append(f"按钮序 {info['buttons']}")

            # 截图留档
            await shot("import-failed")

            # ① 重试：按原参数重投失败路径（3 个失败文件名对应源路径），弹窗关闭
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('重试'))")
            await asyncio.sleep(1.5)
            calls = await ev("window.__importCalls")
            print("重试后 import 调用:", json.dumps(calls, ensure_ascii=False))
            if len(calls) != 2:
                fails.append(f"重试未重投（调用数 {len(calls)}）")
            else:
                retry = calls[1]
                want = ["E:/T/损坏文件A.png", "E:/T/被占用B.psd", "E:/T/超长文件名用来验证换行表现的一段非常非常非常非常长的文件名C.fla"]
                if retry["paths"] != want:
                    fails.append(f"重投路径 {retry['paths']}")
                if retry["structure"] != calls[0]["structure"] or retry["source"] != calls[0]["source"]:
                    fails.append("重试未继承原导入参数")
            # 重试本身也失败（桩恒失败）→ 弹窗再现；按退出 = 纯关闭不重投
            again = await ev("!!document.querySelector('.app-modal.alert')")
            print("重投失败弹窗再现:", again)
            if not again:
                fails.append("重投失败后弹窗未再现")
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('退出'))")
            await asyncio.sleep(0.8)
            closed = await ev("!document.querySelector('.app-modal.alert')")
            total = await ev("window.__importCalls.length")
            print("退出: 关闭", closed, "调用数", total)
            if not closed or total != 2:
                fails.append(f"退出 {closed} {total}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: alert 壳/徽章/文件名竖排 8px/按钮序/重试重投/退出纯关闭均贴口径")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
