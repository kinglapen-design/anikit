# CDP 实测：2.3 自动播放数量闸 + 小动画播放源（2026-08-12）
# 桩 40 张动图卡片 + autoPlayGif 开 + get_asset_file_path 返回 animPreview：
# ① 播放中卡片（img src 含 anim_cache.webp）≤ 16（闸门上限，与密度解耦）；
# ② 播放源指向 animPreview（小动画），不指源文件 src.gif；
# ③ 滚动到底部：滚出卡片释放名额、新滚入获得，播放总数仍 ≤ 16。
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
      list_placements: placements, list_folder_tree: [], list_recycle: [], list_recent_assets: [],
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
              playing: [...document.querySelectorAll('.asset-card .thumb img')].filter((i) => i.src.includes('anim_cache.webp')).length,
              srcLeaks: [...document.querySelectorAll('.asset-card .thumb img')].filter((i) => i.src.includes('src.gif')).length,
            })"""

            # ①② 初始：等 300ms 延迟 + loadAnim 异步
            await asyncio.sleep(2.0)
            diag = await ev("""({
              wizard: !!document.querySelector('.wizard-card'),
              dockHost: !!document.querySelector('.dock-host'),
              dockview: !!document.querySelector('.dv-dockview'),
              appLen: document.querySelector('#app')?.innerHTML?.length ?? -1,
              appHead: (document.querySelector('#app')?.innerHTML ?? '').slice(0, 300),
              viewport: !!document.querySelector('.grid-viewport'),
              cards: document.querySelectorAll('.asset-card').length,
            })""")
            print("诊断:", json.dumps(diag, ensure_ascii=False))
            s1 = json.loads(json.dumps(await ev(STATS)))
            print("初始:", json.dumps(s1, ensure_ascii=False))
            fails = []
            if s1["cards"] < 17:
                print("!! 挂载卡片数不足（虚拟窗口比预期小），无法验证上限")
                fails.append("样本不足")
            if s1["playing"] > 16:
                fails.append(f"播放数超闸 {s1['playing']}")
            if s1["playing"] == 0:
                fails.append("无播放（链路未通）")
            if s1["srcLeaks"] > 0:
                fails.append(f"{s1['srcLeaks']} 张播放源指源文件未走小动画")

            # ③ 滚动到底部：滚出释放、新滚入获得，总数仍 ≤16
            await ev("(() => { const v = document.querySelector('.grid-viewport'); v.scrollTop = v.scrollHeight; return true; })()")
            await asyncio.sleep(2.0)
            s2 = json.loads(json.dumps(await ev(STATS)))
            print("滚底:", json.dumps(s2, ensure_ascii=False))
            if s2["playing"] > 16:
                fails.append(f"滚底播放数超闸 {s2['playing']}")
            if s2["playing"] == 0:
                fails.append("滚底无播放（名额未释放唤醒）")
            if s2["srcLeaks"] > 0:
                fails.append(f"滚底 {s2['srcLeaks']} 张播放源指源文件")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 闸门 ≤16 两态成立、播放源全走小动画、滚动释放唤醒正常")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
