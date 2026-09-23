# CDP 探针：左栏收展「展开侧边栏」钮回归（2026-09-23 踩坑 085 钉子）
# 根因：dockview-theme.css 的 `.dv-pre-actions-container:not(:has(.icon-btn))` 选择器用的是 1.2 时代
# 类名 icon-btn，按钮族封装（aa35652a）改 ic-btn 后失联——容器恒 display:none，收起左栏后展开钮
# 在 DOM 里但不可见（computed 颜色正常，display 才是病根；截图目验佐证）。
#   E1  收起前：pre-actions 容器 display:none（无钮不占位）
#   E2  点 SidebarLeft 收起钮：dock-host 挂 left-collapsed + 左栏宽收 0
#   E3  展开钮可见：pre-actions display:flex + 钮 30×30 + svg 18×18
#   E4  点展开钮：左栏回 254 + pre-actions 回 display:none
#   截图 sb-expand-probe.png（收起后标签行首钮目验）
# 桩：最小主窗口（同右栏探针瘦版）。
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-sidebar-expand-probe.py
import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19700 + os.getpid() % 300
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-sbexpand-{os.getpid()}")
SHOT = "tools/style-preview/sb-expand-probe.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 3 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "角色", count: 3, totalSize: 6000,
            children: [{ id: "f1a", name: "主角", count: 1, totalSize: 3000, children: [] }] },
        ] },
      list_placements: [
        { assetId: "a1", folderId: "f1", name: "雪牧_自动.fla", ext: "fla", size: 1000,
          w: null, h: null, thumbKind: null, thumbPath: null, origin: true,
          frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
          createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
          importedAt: "2026-07-12T14:26:00+08:00" },
      ],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [], list_relations: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null, save_ui_settings: null,
    };
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(v === null ? null : structuredClone(v));
  },
};
window.__cbId = 0; window.__cbs = {};
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


async def main():
    fails = []

    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "--disk-cache-size=1",
        f"--user-data-dir={PROFILE}", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        version = None
        for _ in range(30):
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if ok is False or detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            def dump():
                return """(() => {
                  const host = document.querySelector('.dock-host');
                  const sb = document.querySelector('.sidebar-left');
                  const pre = document.querySelector('.dock-host .dv-pre-actions-container');
                  const btn = pre?.querySelector('button');
                  const svg = btn?.querySelector('svg');
                  return { hostCls: host?.className ?? null, sbW: sb?.offsetWidth ?? null,
                           preDisp: pre ? getComputedStyle(pre).display : null,
                           btnW: btn?.offsetWidth ?? null, btnH: btn?.offsetHeight ?? null,
                           svgW: svg ? svg.getBoundingClientRect().width : null }; // SVGElement 无 offsetWidth（HTMLElement 属性），须走 bounding rect
                })()"""

            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                if await ev("!!document.querySelector('.sidebar-left')"):
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            # ---------- E1 收起前：pre-actions 无钮 display:none ----------
            d = await ev(dump())
            check("E1 展开态：pre-actions 无钮不占位（display none）",
                  isinstance(d, dict) and d.get("preDisp") == "none" and d.get("sbW") == 254,
                  json.dumps(d))

            # ---------- E2 收起：dock-host left-collapsed + 左栏宽 0 ----------
            await ev("document.querySelector('.sidebar-left .menu-bar .ic-btn').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)  # 400ms 收展动画 + 余量
            d = await ev(dump())
            check("E2 收起：dock-host 挂 left-collapsed + 左栏宽收 0",
                  isinstance(d, dict) and "left-collapsed" in (d.get("hostCls") or "") and d.get("sbW") == 0,
                  json.dumps(d))

            # ---------- E3 展开钮可见（踩坑 085 钉：DOM 在 + display flex + 几何非零） ----------
            check("E3 展开钮可见：pre-actions display:flex + 钮 30×30 + svg 18",
                  isinstance(d, dict) and d.get("preDisp") == "flex"
                  and d.get("btnW") == 30 and d.get("btnH") == 30 and d.get("svgW") == 18,
                  json.dumps(d))
            shot = await send(ws, "Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": 0, "y": 0, "width": 500, "height": 120, "scale": 2},
            }, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print(f"saved {SHOT}")

            # ---------- E4 点展开钮：左栏回 254 + 钮退场 ----------
            await ev("document.querySelector('.dock-host .dv-pre-actions-container button').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)
            d = await ev(dump())
            check("E4 点展开钮：左栏回 254 + pre-actions 回 display:none",
                  isinstance(d, dict) and d.get("sbW") == 254 and d.get("preDisp") == "none"
                  and "left-collapsed" not in (d.get("hostCls") or ""),
                  json.dumps(d))
    finally:
        edge.kill()
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(edge.pid)], capture_output=True)
        if spawned:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(spawned.pid)], capture_output=True)

    print("\nALL PASS" if not fails else f"\n{len(fails)} FAIL: " + "; ".join(fails))
    return 1 if fails else 0


sys.exit(asyncio.run(main()))
