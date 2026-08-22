# CDP 实测：阶段四动图卡帧流播放（2026-08-20）
# 桩 40 张动图卡 + autoPlayGif 开；start_anim_stream 桩返回假会话（python WS server 推 160×90 彩帧）：
# ① 可见动图卡 canvas 出帧（160×90）；② thumb img 不换源（恒缩略图，旧 img 播放路径已拆）；
# ③ 滚出视口停会话（stop_anim_stream 被调）+ 新可见卡出帧；④ 在播 canvas 数 ≤ 50（闸口径不变）；
# ⑤ 卡片尺寸变化 → set_anim_stream_size 热更新（2026-08-20 尺寸跟随：ResizeObserver → 防抖 → invoke）。
# 前端链路全真（AnimStreamSession / AnimCanvasLayer / 闸门），帧源由 python WS 桩供给。
import asyncio
import json
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9364
WS_PORT = 9363
URL = "http://localhost:5373"

STUB = r"""
window.__starts = 0; window.__stops = 0; window.__sid = 0; window.__resizes = 0;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 40, folderCount: 0, autoImport: false };
    const placements = Array.from({ length: 40 }, (_, i) => ({
      assetId: `A${i}`, folderId: "root", name: `动图${i}.gif`,
      w: 480, h: 360, ext: "gif", thumbPath: "E:/T/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: 24, animated: true, duration: 1, bpm: null,
      importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
    }));
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: true },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.gif", pending: false },
    };
    if (cmd === "start_anim_stream") {
      window.__starts++;
      return Promise.resolve({ session: ++window.__sid, port: WS_PORT });
    }
    if (cmd === "stop_anim_stream") {
      window.__stops++;
      return Promise.resolve(null);
    }
    if (cmd === "set_anim_stream_size") {
      window.__resizes++;
      return Promise.resolve(null);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
""".replace("WS_PORT", str(WS_PORT))

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


async def frame_server():
    """假帧流 server：每连接 20fps 推 160x90 RGBA 彩帧（协议 v2，同后端 frame_stream，
    29 字节头 [tag=0][w][h][idx][pts_ms f64][seq]，idx = 源帧索引 0 起 24 帧循环，
    pts = idx×50ms——动图前端忽略 pts，视频探针才有 pts 语义）。
    160x90 而非 320x180：9 连接 × 20fps × 230KB 会把单线程事件循环压垮（CDP 客户端
    recv 饿死、探针卡死，2026-08-20 实测）；1/4 数据量后 CDP 与推流共存。"""
    async def handler(ws):
        seq = 0
        w, h = 160, 90
        try:
            while True:
                seq += 1
                idx = (seq - 1) % 24
                header = (
                    b"\x00"
                    + w.to_bytes(4, "little") + h.to_bytes(4, "little")
                    + idx.to_bytes(4, "little")
                    + struct.pack("<d", idx * 50.0)
                    + seq.to_bytes(8, "little")
                )
                rgba = bytes([seq % 256, 100, 200, 255]) * (w * h)
                await ws.send(header + rgba)
                await asyncio.sleep(0.05)
        except websockets.ConnectionClosed:
            pass
    return await websockets.serve(handler, "127.0.0.1", WS_PORT)


async def main():
    server = await frame_server()
    # 独立 user-data-dir：Edge 单实例机制下不带它会连进用户日常浏览器（标签页/扩展全在、
    # 流程不可控，2026-08-20 实测）；隔离实例互不干扰
    profile = tempfile.mkdtemp(prefix="cdp-anim-stream-")
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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            STATS = """({
              cards: document.querySelectorAll('.asset-card').length,
              visible: [...document.querySelectorAll('.asset-card')].filter((c) => { const r = c.getBoundingClientRect(); return r.bottom > 0 && r.top < innerHeight; }).length,
              canvases: document.querySelectorAll('.asset-card canvas.anim-canvas').length,
              framed: [...document.querySelectorAll('.asset-card canvas.anim-canvas')].filter((c) => c.width === 160 && c.height === 90).length,
              framedVisible: [...document.querySelectorAll('.asset-card')].filter((c) => { const r = c.getBoundingClientRect(); const vis = r.bottom > 0 && r.top < innerHeight; const cv = c.querySelector('canvas.anim-canvas'); return vis && cv && cv.width === 160; }).length,
              imgSwapped: [...document.querySelectorAll('.asset-card .thumb img')].filter((i) => i.src.includes('src.gif') || i.src.includes('anim_cache')).length,
              starts: window.__starts, stops: window.__stops,
            })"""

            async def check(tag, expect_stops):
                s = json.loads(json.dumps(await ev(STATS)))
                print(tag, json.dumps(s, ensure_ascii=False))
                fails = []
                if s["visible"] == 0 or s["framed"] == 0:
                    fails.append("无可见/无出帧（链路未通）")
                if s["framedVisible"] != s["visible"]:
                    fails.append(f"可见卡未全出帧（可见 {s['visible']} 出帧 {s['framedVisible']}）")
                if s["canvases"] > 50:
                    fails.append(f"在播 canvas 超闸 {s['canvases']}")
                if s["imgSwapped"] > 0:
                    fails.append(f"{s['imgSwapped']} 张 img 被换源（旧播放路径残留）")
                if expect_stops and s["stops"] == 0:
                    fails.append("滚出视口未停会话（stop_anim_stream 未调用）")
                return fails

            # ①②④ 初始（顶部）：等 IO 回调 + 300ms 进入延迟 + 会话启动 + 首帧
            await asyncio.sleep(2.5)
            fails = await check("顶部:", expect_stops=False)

            # ③ 滚到底部：原可见区停会话，新可见区出帧
            grid_xy = await ev("(() => { const v = document.querySelector('.grid-viewport'); const b = v.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            for _ in range(30):
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseWheel", "x": grid_xy["x"], "y": grid_xy["y"], "deltaX": 0, "deltaY": 600}, session=sess)
                await asyncio.sleep(0.05)
            await asyncio.sleep(2.5)
            fails += await check("滚底:", expect_stops=True)

            # ⑤ 尺寸跟随：改可见在播卡 .thumb 高度 → ResizeObserver → 防抖 150ms → set_anim_stream_size
            resized = await ev("""(() => {
              const vis = [...document.querySelectorAll('.asset-card')].filter((c) => {
                const r = c.getBoundingClientRect();
                return r.bottom > 0 && r.top < innerHeight && c.querySelector('canvas.anim-canvas');
              });
              for (const c of vis) {
                const t = c.querySelector('.thumb');
                t.style.height = (t.getBoundingClientRect().height / 2) + 'px';
              }
              return vis.length;
            })()""")
            await asyncio.sleep(1.5)  # 布局重排 + ResizeObserver + 150ms 防抖 + invoke
            resizes = await ev("window.__resizes")
            print(f"尺寸跟随: 改动卡 {resized} 张, set_anim_stream_size 调用 {resizes} 次")
            if resized > 0 and resizes == 0:
                fails.append("卡片尺寸变化未触发 set_anim_stream_size（尺寸跟随链路断）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 帧流播放全链路成立（canvas 出帧 / img 不换源 / 滚出停会话 / ≤50 闸 / 尺寸跟随热更新）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
        server.close()


sys.exit(asyncio.run(main()))
