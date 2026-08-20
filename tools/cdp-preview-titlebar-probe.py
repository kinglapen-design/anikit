# CDP 实测：预览标题栏图标方向（2026-08-20，阶段五 5.1 复核——用户实机报"返回朝下 / 上·下一个显示 ↓↑"）
# 根因：1.1 图标换血后 chevron-large 由朝下变朝右（18px 箭头），PreviewTitleBar 旧 rot-ccw/rot-cw（±90°）
# 约定失效。修复：返回 = chevron-large rotate(180°) 朝左；上/下一个 = nav-history 与面包屑后退/前进同款。
# 断言：① 返回 svg computed transform = rotate180 矩阵；② 上/下一个 svg path 与面包屑 nav-history 同 d；
# ③ 上一个无旋转、下一个 rotate(180°)；④ 标题栏截图留档 style-preview/preview-titlebar.png。
import asyncio
import base64
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9367
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 3, folderCount: 0, autoImport: false };
    const placements = Array.from({ length: 3 }, (_, i) => ({
      assetId: `A${i}`, folderId: "root", name: `测试图${i}.png`,
      w: 800, h: 600, ext: "png", thumbPath: "E:/T/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: null, animated: false, duration: null, bpm: null,
      importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
    }));
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.png", pending: false, animPreview: null },
      get_notes: [],
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = [str(a.get("value", a.get("description", "")))[:300] for a in raw["params"]["args"]]
            print("[CONSOLE]", raw["params"]["type"], " ".join(args)[:500])


# 期望矩阵：rotate(180deg) === matrix(-1, 0, 0, -1, 0, 0)
ROT180 = "matrix(-1, 0, 0, -1, 0, 0)"
# 图标 d 前缀（IconButton 走 v-tip 无 title 属性，按图标 path 与 DOM 序定位）
NAV_HISTORY_D = "M11.6251 2.5"   # nav-history（朝左 chevron，面包屑后退/前进）
CHEVRON_LARGE_D = "M9 15.5"      # chevron-large（朝右箭头，预览返回）


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-preview-titlebar-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            # 轮询等卡片渲染（vite 冷编译 / HMR 恢复可能超 6s）
            cards = 0
            for _ in range(40):
                await asyncio.sleep(0.5)
                cards = await ev("document.querySelectorAll('.asset-card').length") or 0
                if cards > 0:
                    break
            print("卡片数:", cards, "body 长度:", await ev("document.body.innerHTML.length"))
            if cards == 0:
                print("body 片段:", (await ev("document.body.innerHTML.slice(0, 300)")) or "")
                print("FAIL: 应用未渲染（无 .asset-card）")
                return 3

            fails = []

            # 面包屑对照：后退 / 前进（nav-history）存在性
            crumb_ds = await ev("""[...document.querySelectorAll('.crumb-group .icon-btn svg path')]
              .map((p) => p.getAttribute('d')?.slice(0, 20) ?? null)""")
            print("面包屑按钮 d 前缀:", crumb_ds)
            if not crumb_ds or not any(d and d.startswith(NAV_HISTORY_D) for d in crumb_ds):
                fails.append("面包屑 nav-history 未找到——对照基准缺失")

            # 双击第一张卡 → 预览模式
            await ev("document.querySelector('.asset-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(2)

            stage = await ev("!!document.querySelector('.preview-stage')")
            if not stage:
                fails.append("预览模式未打开（无 .preview-stage）")

            # 返回 = nav-group 唯一按钮；上/下一个 = btn-group 最后两钮
            info = await ev("""(() => {
              const read = (btn) => {
                const svg = btn?.querySelector("svg");
                return svg ? { d: svg.querySelector("path")?.getAttribute("d")?.slice(0, 20) ?? null,
                               transform: getComputedStyle(svg).transform } : null;
              };
              const back = read(document.querySelector('.preview-titlebar .nav-group .icon-btn'));
              const group = [...document.querySelectorAll('.preview-titlebar .btn-group .icon-btn')];
              return { back, prev: read(group[group.length - 2]), next: read(group[group.length - 1]),
                       btnCount: group.length };
            })()""")
            print("预览标题栏:", json.dumps(info, ensure_ascii=False))
            back = (info or {}).get("back")
            prev = (info or {}).get("prev")
            nxt = (info or {}).get("next")

            # ① 返回朝左 = chevron-large（朝右箭头）rotate(180°)
            if not back or not back["d"] or not back["d"].startswith(CHEVRON_LARGE_D):
                fails.append(f"返回图标 ≠ chevron-large（{back and back['d']}）")
            if not back or back["transform"] != ROT180:
                fails.append(f"返回未旋转 180° 朝左（{back and back['transform']}）")
            # ② 上一个 = nav-history 无旋转（朝左）；③ 下一个 = nav-history rotate(180°)（朝右）
            if not prev or not prev["d"] or not prev["d"].startswith(NAV_HISTORY_D):
                fails.append(f"上一个图标 ≠ nav-history（{prev and prev['d']}）")
            if not prev or prev["transform"] not in ("none", ""):
                fails.append(f"上一个不应旋转（{prev and prev['transform']}）")
            if not nxt or not nxt["d"] or not nxt["d"].startswith(NAV_HISTORY_D):
                fails.append(f"下一个图标 ≠ nav-history（{nxt and nxt['d']}）")
            if not nxt or nxt["transform"] != ROT180:
                fails.append(f"下一个未旋转 180° 朝右（{nxt and nxt['transform']}）")

            # ④ 标题栏截图留档（按 .preview-titlebar 实测 rect 截）
            tb = await ev("(() => { const r = document.querySelector('.preview-titlebar')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width, height: r.height } : null; })()")
            if tb:
                shot = await send(ws, "Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": tb["x"], "y": tb["y"], "width": tb["width"], "height": tb["height"], "scale": 2},
                }, session=sess)
                out = "tools/style-preview/preview-titlebar.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(shot["result"]["data"]))
                print("截图:", out, f"({tb['width']:.0f}x{tb['height']:.0f})")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 预览标题栏图标方向成立（返回朝左 rotate180 / 上·下一个 = nav-history 面包屑同款）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


sys.exit(asyncio.run(main()))
