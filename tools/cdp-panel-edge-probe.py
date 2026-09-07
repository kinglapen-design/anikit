# CDP 探针：中间面板左缘常态线实证（2026-09-07 走查二轮补充）——
#   E1  首个 .dv-content-container 的 border-left / 左上圆角 computed（设计线是否命中）
#   E2  几何链：拆分条 .line 与内容容器左缘的叠压关系（负 margin 2px 重叠区谁盖谁）
#   E3  像素取样：内容容器左缘 ±2px 纵向取色（线是否存在/被盖/太淡）+ 标签栏高度处对照
#   截图 tools/style-preview/panel-edge-probe.png 留档自读
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：tools/.venv/Scripts/python.exe tools/cdp-panel-edge-probe.py
import asyncio
import base64
import io
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19920 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-panel-edge-{os.getpid()}")
SHOT = "tools/style-preview/panel-edge-probe.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 1 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [{ id: "f1", name: "角色", count: 0, children: [] }] },
      list_placements: [],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
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


def kill_listening_port(port):
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                subprocess.run(["taskkill", "/PID", line.split()[-1], "/F"], capture_output=True)
    except Exception:
        pass


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
                r = await send(ws, "Runtime.evaluate",
                               {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                ok = await ev("!!document.querySelector('.window .dv-content-container')")
                if ok is True:
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            info = await ev(
                "(() => { const c = document.querySelector('.anikit-dock-theme .dv-content-container');"
                " if (!c) return null;"
                " const s = getComputedStyle(c); const r = c.getBoundingClientRect();"
                " const line = document.querySelector('.split-bar.left .line');"
                " const ls = line ? getComputedStyle(line) : null;"
                " const lr = line ? line.getBoundingClientRect() : null;"
                " const bar = document.querySelector('.split-bar.left');"
                " const br = bar ? bar.getBoundingClientRect() : null;"
                " const tabbar = document.querySelector('.anikit-dock-theme .dv-tabs-and-actions-container');"
                " const tr = tabbar ? tabbar.getBoundingClientRect() : null;"
                " return {"
                "  blw: s.borderLeftWidth, blc: s.borderLeftColor, br: s.borderRadius,"
                "  bg: s.backgroundColor, x: r.left, y: r.top, w: r.width,"
                "  lineBg: ls && ls.backgroundImage !== 'none' ? ls.backgroundImage : ls.backgroundColor,"
                "  lineW: ls && ls.width, lineX: lr && lr.left, lineWBox: lr && lr.width,"
                "  barX: br && br.left, barW: br && br.width,"
                "  tabBottom: tr && tr.bottom }; })()")
            check("E1 内容容器 border-left 1px + 圆角命中",
                  isinstance(info, dict) and info.get("blw") == "1px" and info.get("br", "").startswith("16px"),
                  json.dumps(info, ensure_ascii=False) if info else "no content container")
            if isinstance(info, dict):
                print(f"    content container: x={info['x']:.1f} y={info['y']:.1f} w={info['w']:.1f} "
                      f"border={info['blw']} {info['blc']} radius={info['br']}")
                print(f"    split-bar: x={info['barX']:.1f} w={info['barW']:.1f}（右缘 {info['barX'] + info['barW']:.1f}）"
                      f" line: x={info['lineX']:.1f} w={info['lineWBox']:.1f} bg={info['lineBg'][:60]}")
                print(f"    叠压关系：拆分条右缘超出内容容器左缘 {info['barX'] + info['barW'] - info['x']:.1f}px；"
                      f"标签栏底 {info['tabBottom']:.1f} vs 内容容器顶 {info['y']:.1f}")

            # E3 截图 + 左缘像素取样
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            png = base64.b64decode(shot["result"]["data"])
            with open(SHOT, "wb") as f:
                f.write(png)
            img = Image.open(io.BytesIO(png)).convert("RGB")
            dpr = await ev("window.devicePixelRatio || 1") or 1
            if isinstance(info, dict):
                ex = int((info["x"]) * dpr)
                ys = int((info["y"] + 120) * dpr)  # 内容区中部（避开顶部结构）
                row = [img.getpixel((xx, ys)) for xx in range(ex - 3 * int(dpr), ex + 4 * int(dpr))]
                print(f"    E3 左缘像素列（y=内容区+120, DPR={dpr}）: "
                      + " ".join(f"{p}" for p in row))
                ey = int((info["tabBottom"] - 20) * dpr)
                row2 = [img.getpixel((xx, ey)) for xx in range(ex - 3 * int(dpr), ex + 4 * int(dpr))]
                print(f"    E3 标签栏高度处像素列（用户箭头位置）: " + " ".join(f"{p}" for p in row2))
            print("saved", SHOT)

            print("RESULT:", "PASS" if not fails else f"FAIL {len(fails)}")
            return 1 if fails else 0
    finally:
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(edge.pid)], capture_output=True)
        if spawned:
            kill_listening_port(5373)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
