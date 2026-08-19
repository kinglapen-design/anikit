# CDP 实测：2.3 自动播放数量闸 + 小动画播放源 + 可见性联动（2026-08-12 二轮）
# 桩 40 张动图卡片 + autoPlayGif 开 + get_asset_file_path 返回 animPreview：
# ① 播放卡 == 真实可见卡（IO 联动：可见才配持有名额，不触闸时可见全播——修"头尾可见卡排队等死"）；
# ② 播放数 ≤ 50（anim 池上限，2026-08-12 用户定 6列×7行=42 也能看）；
# ③ 播放源指向 animPreview（小动画），不指源文件 src.gif；
# ④ 滚动到底部：原可见区滚出即停播释放，新可见区全播（无残留死名额）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9347
URL = "http://localhost:5373"

STUB = r"""
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
      get_asset_file_path: { path: "E:/T/src.gif", pending: false, animPreview: "E:/T/anim_cache.webp" },
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

            STATS = """({
              cards: document.querySelectorAll('.asset-card').length,
              visible: [...document.querySelectorAll('.asset-card')].filter((c) => { const r = c.getBoundingClientRect(); return r.bottom > 0 && r.top < innerHeight; }).length,
              playing: [...document.querySelectorAll('.asset-card .thumb img')].filter((i) => i.src.includes('anim_cache.webp')).length,
              playingVisible: [...document.querySelectorAll('.asset-card')].filter((c) => { const r = c.getBoundingClientRect(); const vis = r.bottom > 0 && r.top < innerHeight; const img = c.querySelector('.thumb img'); return vis && img && img.src.includes('anim_cache.webp'); }).length,
              playingHidden: [...document.querySelectorAll('.asset-card')].filter((c) => { const r = c.getBoundingClientRect(); const vis = r.bottom > 0 && r.top < innerHeight; const img = c.querySelector('.thumb img'); return !vis && img && img.src.includes('anim_cache.webp'); }).length,
              srcLeaks: [...document.querySelectorAll('.asset-card .thumb img')].filter((i) => i.src.includes('src.gif')).length,
            })"""

            async def check(tag):
                s = json.loads(json.dumps(await ev(STATS)))
                print(tag, json.dumps(s, ensure_ascii=False))
                fails = []
                if s["visible"] == 0 or s["playing"] == 0:
                    fails.append("无可见/无播放（链路未通）")
                if s["playing"] != s["playingVisible"] or s["playingHidden"] > 0:
                    fails.append(f"有不可见卡在播（{s['playingHidden']}）")
                if s["playingVisible"] != s["visible"]:
                    fails.append(f"可见卡未全播（可见 {s['visible']} 播 {s['playingVisible']}）")
                if s["playing"] > 50:
                    fails.append(f"播放数超闸 {s['playing']}")
                if s["srcLeaks"] > 0:
                    fails.append(f"{s['srcLeaks']} 张播放源指源文件未走小动画")
                return fails

            # ①②③ 初始（顶部）：等 IO 回调 + 300ms 延迟 + loadAnim 异步
            await asyncio.sleep(2.0)
            fails = await check("顶部:")

            # ④ 滚动到底部：原可见区停播释放，新可见区全播。
            # 注意：虚拟滚动由内部 pos 状态驱动 canvas transform，直接设 scrollTop 只移视觉不动状态
            # （首轮 FAIL 根因）——必须派真实 wheel 事件走 AssetGrid 的滚动链路
            grid_xy = await ev("(() => { const v = document.querySelector('.grid-viewport'); const b = v.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            for _ in range(30):
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseWheel", "x": grid_xy["x"], "y": grid_xy["y"], "deltaX": 0, "deltaY": 600}, session=sess)
                await asyncio.sleep(0.05)
            await asyncio.sleep(2.0)
            fails += await check("滚底:")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 可见即播/滚出即停两态成立、≤50 闸、播放源全走小动画")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
