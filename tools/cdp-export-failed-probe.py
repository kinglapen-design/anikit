# CDP 实测：导出失败弹窗 + 重试（2026-08-14 用户定：导出也有失败清单，按钮重试/退出）
# 桩有库 + 一张素材卡 → 右键卡片出菜单 → hover"导出文件"出二级 → 点"导出到本地磁盘"
# → 目录选择桩返回 → export_assets 桩经 Channel 推 finished（2 条失败，含 path/target）
# → FailedListDialog（导出未完成）出现 → 重试 → export_retry 收到原样 failed（源→目标原路覆盖）
# → 推全成 finished → 弹窗关闭 + 成功 toast。
# 截图留档 tools/style-preview/export-failed.png。
import asyncio
import base64
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
window.__cbs = {};
window.__cbId = 0;
window.__exportCalls = [];
window.__retryCalls = [];
window.__failed = [
  { path: "E:/T/S/库/assets/AN-A.info/图A.png", target: "E:/T/导出/图A.png", reason: "目标磁盘写入失败" },
  { path: "E:/T/S/库/assets/AN-B.info/走路-1.png", target: "E:/T/导出/走路/走路-1.png", reason: "权限不足" },
];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    (window.__cmds = window.__cmds || []).push(cmd);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 1, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [{ assetId: "AN-A", folderId: "root", name: "图A.png", w: 800, h: 600, ext: "png",
        thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: null }],
      list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve("E:/T/导出");
    if (cmd === "export_assets" || cmd === "export_retry") {
      // JSON 脱壳：前端传的是 Pinia 响应式代理，CDP returnByValue 读回会成空对象（真 Tauri IPC 无此问题）
      window.__exportCalls.push(JSON.parse(JSON.stringify({ cmd, assetIds: args.assetIds ?? null, items: args.items ?? null })));
      const chId = typeof args.onEvent === "object" ? args.onEvent.id : Number(String(args.onEvent).split(":").pop());
      setTimeout(() => {
        const cb = window.__cbs[chId];
        if (!cb) return;
        if (cmd === "export_assets") {
          cb({ index: 0, message: { type: "started", total: 2 } });
          cb({ index: 1, message: { type: "finished", copied: 0, failed: window.__failed, error: null } });
        } else {
          // 重试全成：dialog 应关 + 成功 toast
          cb({ index: 0, message: { type: "started", total: 2 } });
          cb({ index: 1, message: { type: "finished", copied: 2, failed: [], error: null } });
        }
      }, 300);
      return Promise.resolve(2);
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = raw["params"]["args"]
            print("[CONSOLE]", " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


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

            # 右键素材卡出菜单（右键未选中项会先单选）
            print("卡片数:", await ev("document.querySelectorAll('.asset-card').length"))
            await ev("""
              (() => {
                const card = document.querySelector('.asset-card');
                if (!card) return false;
                const r = card.getBoundingClientRect();
                card.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.left + 20, clientY: r.top + 20 }));
                return true;
              })()
            """)
            await asyncio.sleep(0.8)
            menu = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => m.textContent.trim())")
            print("一级菜单项数:", len(menu) if menu else 0)
            if not menu or not any("导出文件" in m for m in menu):
                print("FAIL: 右键菜单未出或无导出文件", menu)
                return 3

            # hover "导出文件" 出二级 → 点 "导出到本地磁盘"
            await ev("""
              (() => {
                const it = [...document.querySelectorAll('.app-menu .menu-item')].find((el) => el.textContent.includes('导出文件'));
                if (it) it.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));
              })()
            """)
            await asyncio.sleep(0.8)
            clicked = await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('导出到本地磁盘'))")
            await asyncio.sleep(1.5)
            print("点导出到本地磁盘:", clicked, "export 调用:", await ev("window.__exportCalls.length"))

            shown = await ev("!!document.querySelector('.app-modal.alert')")
            print("失败弹窗(alert):", shown)
            if not shown:
                print("FAIL: 弹窗未出现; cmds:", await ev("(window.__cmds || []).join(',')"))
                return 3

            info = await ev("""({
              title: document.querySelector('.app-modal.alert .title')?.textContent ?? null,
              items: [...document.querySelectorAll('.fail-item')].map((e) => e.textContent),
              gap: getComputedStyle(document.querySelector('.fail-list')).rowGap,
              buttons: [...document.querySelectorAll('.app-modal.alert .actions .app-btn')].map((b) => b.textContent.trim()),
            })""")
            print("弹窗:", json.dumps(info, ensure_ascii=False))
            if info["title"] != "导出未完成":
                fails.append(f"标题 {info['title']}")
            if info["items"] != ["图A.png", "走路-1.png"]:
                fails.append(f"文件名清单 {info['items']}")
            if info["gap"] != "8px":
                fails.append(f"间距 {info['gap']}")
            if info["buttons"] != ["重试", "退出"]:
                fails.append(f"按钮序 {info['buttons']}")

            await shot("export-failed")

            # 重试：export_retry 收到原样 failed（path + target 保留）；全成后弹窗关 + toast
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('重试'))")
            await asyncio.sleep(1.5)
            calls = await ev("window.__exportCalls")
            print("export 调用:", json.dumps(calls, ensure_ascii=False))
            retry = next((c for c in calls if c["cmd"] == "export_retry"), None)
            if retry is None:
                fails.append("重试未调 export_retry")
            else:
                items = retry["items"]
                ok = (
                    len(items) == 2
                    and items[0]["path"].endswith("图A.png")
                    and items[0]["target"] == "E:/T/导出/图A.png"
                    and items[1]["target"] == "E:/T/导出/走路/走路-1.png"
                )
                if not ok:
                    fails.append(f"重投项不符 {json.dumps(items, ensure_ascii=False)}")
            closed = await ev("!document.querySelector('.app-modal.alert')")
            toast = await ev("document.querySelector('.toast-stack')?.textContent ?? ''")
            print("重试后: 弹窗关", closed, "toast:", toast)
            if not closed:
                fails.append("重试全成后弹窗未关")
            if "已导出 2 个文件" not in (toast or ""):
                fails.append(f"成功 toast 缺失 {toast!r}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 弹窗/文件名清单/按钮序/重试原路重投/全成收尾均贴口径")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
