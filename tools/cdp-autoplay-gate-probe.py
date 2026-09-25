# CDP 实测：2.3 自动播放数量闸 + 小动画播放源 + 可见性联动（2026-08-12 二轮）
# 2026-09-25 T0 验证网修复：选择器换代适配 + 共享桩 _probe_stub
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

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9347
URL = "http://localhost:5373"

# 启动链默认桩表（get_window_fx_mode / list_relations / list_archives / list_inbox_snapshot 等）
# 由 _probe_stub.DEFAULT_STUBS 兜底；本探针只覆盖业务数据
STUB = make_stub(
    {
        "get_current_library": "LIB",
        "get_ui_settings": "{ autoPlayGif: true }",
        "detect_doc_apps": "[]",
        "list_placements": "placements",
        "list_space_libraries": '["旧库"]',
        "list_recent_libraries": '[{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }]',
        "get_asset_file_path": '{ path: "E:/T/src.gif", pending: false, animPreview: "E:/T/anim_cache.webp" }',
    },
    handlers=r"""
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 40, folderCount: 0, autoImport: false };
    const placements = Array.from({ length: 40 }, (_, i) => ({
      assetId: `A${i}`, folderId: "root", name: `动图${i}.gif`,
      w: 480, h: 360, ext: "gif", thumbPath: "E:/T/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: 24, animated: true, duration: 1, bpm: null,
      importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
    }));
""",
)

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

            # 2026-09-25 T0：静态封面 = img.fc-thumb（ThumbContent），播放层 = img.anim-img（AnimImgLayer，
            # 2026-09-02 卡片层 WS 帧流退役回滚原生 img）；旧 .thumb img 选择器随 FileCard 重构失效。
            # T0 二轮：「可见」口径 = 与 .grid-viewport（overflow:hidden 裁剪容器，AssetGrid.vue:626-630）
            # 求交，与业务侧 IntersectionObserver 的裁剪语义一致——旧口径用 window 视口，滚动后顶部
            # 被网格视口裁掉的半行卡（bot < 网格顶沿）被误判"可见"（实跑 DIAG 实证 A30-32）
            STATS = """(() => {
              const gv = document.querySelector('.grid-viewport').getBoundingClientRect();
              const inView = (c) => { const r = c.getBoundingClientRect(); return r.bottom > gv.top && r.top < gv.bottom && r.right > gv.left && r.left < gv.right; };
              const playingImg = (c) => { const img = c.querySelector('img.anim-img'); return img != null && img.src.includes('anim_cache.webp'); };
              const cards = [...document.querySelectorAll('.asset-card')];
              return {
                cards: cards.length,
                visible: cards.filter(inView).length,
                playing: [...document.querySelectorAll('.asset-card img.anim-img')].filter((i) => i.src.includes('anim_cache.webp')).length,
                playingVisible: cards.filter((c) => inView(c) && playingImg(c)).length,
                playingHidden: cards.filter((c) => !inView(c) && playingImg(c)).length,
                srcLeaks: [...document.querySelectorAll('.asset-card img.anim-img')].filter((i) => i.src.includes('src.gif')).length,
              };
            })()"""

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
                if fails:
                    # 2026-09-25 T0 二轮诊断：可见卡逐张列 anim-img 挂载/src/加载态，区分
                    # 「层未挂载（闸/IO）」与「src 未赋值（加载链时序）」
                    diag = await ev("""
                      (() => {
                        const gv = document.querySelector('.grid-viewport').getBoundingClientRect();
                        return [...document.querySelectorAll('.asset-card')].map((c) => {
                          const r = c.getBoundingClientRect();
                          if (!(r.bottom > gv.top && r.top < gv.bottom)) return null;
                          const img = c.querySelector('img.anim-img');
                          return { key: c.getAttribute('data-key'), top: Math.round(r.top), bot: Math.round(r.bottom),
                                   img: !!img, src: img ? (img.getAttribute('src') ?? '') : null, nw: img ? img.naturalWidth : -1 };
                        }).filter(Boolean);
                      })()
                    """)
                    print("DIAG 可见卡明细:", json.dumps(diag, ensure_ascii=False))
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
