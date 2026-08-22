# CDP 实测：播放条 / 帧条贴 13_播放条稿（2026-08-20，阶段五 5.1 复核——用户定：
# 播放条与搜索栏同位同尺寸 46 高通栏直角 border-top 暗-悬停；帧选择同样式；图标按实际删减）。
# 三变体覆盖：动图（PlayBarFrames + 帧条，假 WS 帧流撑起 animTotal）/ 视频（PlayBarVideo 全量左组）/
# 音频（PlayBarAudio 波形）。断言容器 computed style + 图标色档 + 帧条两端按钮旋转方向 + 截图留档。
import asyncio
import base64
import json
import re
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9368
WS_PORT = 9364
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 3, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "A0", folderId: "root", name: "动图.gif", w: 480, h: 360, ext: "gif",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: 24, animated: true, duration: 1, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "A1", folderId: "root", name: "视频.mp4", w: 1920, h: 1080, ext: "mp4",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: 10, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: 24 },
      { assetId: "A2", folderId: "root", name: "音频.mp3", w: null, h: null, ext: "mp3",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: 10, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_notes: [],
      prepare_anim_strips: { items: [] },
    };
    if (cmd === "get_asset_file_path") {
      const ext = { A0: "gif", A1: "mp4", A2: "mp3" }[args.assetKey ?? ""] ?? "gif";
      return Promise.resolve({ path: `E:/T/src.${ext}`, pending: false });
    }
    if (cmd === "start_anim_stream") return Promise.resolve({ session: 1, port: WS_PORT });
    // 视频 = 自研帧流（5.3：不再有 get_media_source 视频转码）；hasAudio=true 出音量钮（左组 6 件）
    if (cmd === "start_video_stream")
      return Promise.resolve({ session: 2, port: WS_PORT, hasAudio: true, fps: 24, duration: 10 });
    if (cmd === "get_media_source") return Promise.resolve({ path: "E:/T/src.mp3", transcoded: false });
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = [str(a.get("value", a.get("description", "")))[:200] for a in raw["params"]["args"]]
            print("[CONSOLE-ERR]", " ".join(args)[:400])


async def frame_server():
    """假帧流 server（协议 v2）：每连接 20fps 推 160x90 RGBA 彩帧，29 字节头
    [tag=0][w][h][idx][pts_ms f64][seq]，idx 24 帧循环（pts = idx×50ms）。"""
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


# 容器贴稿断言（13 稿：46 高 / 直角 / border-top 1px / C3 / 无 inset 画框）
BAR_STYLE_JS = """(() => {
  const bar = document.querySelector('.preview-footer .play-bar');
  if (!bar) return null;
  const cs = getComputedStyle(bar);
  const icon = bar.querySelector('.icon-btn');
  return {
    height: cs.height, borderRadius: cs.borderRadius, borderTopWidth: cs.borderTopWidth,
    boxShadow: cs.boxShadow, background: cs.backgroundColor, padding: cs.padding,
    iconColor: icon ? getComputedStyle(icon).color : null,
    progressTrack: (() => { const p = bar.querySelector('.progress::before'); return null; })(),
  };
})()"""

STRIP_STYLE_JS = """(() => {
  const strip = document.querySelector('.frame-strip');
  if (!strip) return null;
  const cs = getComputedStyle(strip);
  const btns = [...strip.querySelectorAll('.icon-btn svg')].map((s) => getComputedStyle(s).transform);
  return {
    borderRadius: cs.borderRadius, borderTopWidth: cs.borderTopWidth, boxShadow: cs.boxShadow,
    background: cs.backgroundColor, padding: cs.padding,
    btnTransforms: btns, btnCount: btns.length,
    btnDs: [...strip.querySelectorAll('.icon-btn svg path')].map((p) => p.getAttribute('d')?.slice(0, 20) ?? null),
    frameRadius: (() => { const f = strip.querySelector('.frame'); return f ? getComputedStyle(f).borderRadius : null; })(),
  };
})()"""

# 视频左组按钮朝向断言（2026-08-20 修：seek 原样 ◀◀ = 快退 / flip-h = 快进；step-frame-vertical
# 原样 ◀| = 上一帧 / flip-h = 下一帧）——读 svg computed transform 序列比对
VIDEO_LEFT_JS = """(() => {
  const group = document.querySelector('.preview-footer .play-bar .btn-group');
  if (!group) return null;
  return [...group.querySelectorAll('.icon-btn svg')].map((s) => getComputedStyle(s).transform);
})()"""
FLIP = "matrix(-1, 0, 0, 1, 0, 0)"  # scaleX(-1)

C3_RGB = "rgb(24, 25, 25)"  # #181919
LABEL_RGBA = "rgba(234, 255, 247, 0.341)"  # 文字/标签 0.3412（浏览器舍入）


def rot_deg(matrix: str):
    """computed matrix(a,b,c,d,e,f) → 旋转角度（度）；非旋转/无 → None。"""
    m = re.match(r"matrix\(([^)]+)\)", matrix or "")
    if not m:
        return None
    a, b = (float(x) for x in m.group(1).split(",")[:2])
    import math
    return round(math.degrees(math.atan2(b, a)))


async def main():
    server = await frame_server()
    profile = tempfile.mkdtemp(prefix="cdp-playbar-")
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

            # 等卡片渲染
            cards = 0
            for _ in range(40):
                await asyncio.sleep(0.5)
                cards = await ev("document.querySelectorAll('.asset-card').length") or 0
                if cards > 0:
                    break
            if cards < 3:
                print("FAIL: 卡片未渲染（", cards, "）")
                return 3

            fails = []

            async def check_bar(tag):
                s = await ev(BAR_STYLE_JS)
                print(f"[{tag}] 播放条:", json.dumps(s, ensure_ascii=False))
                if not s:
                    fails.append(f"[{tag}] 播放条未渲染")
                    return
                if s["height"] != "46px":
                    fails.append(f"[{tag}] 播放条高 {s['height']}（期望 46px）")
                if s["borderRadius"] != "0px":
                    fails.append(f"[{tag}] 播放条圆角 {s['borderRadius']}（期望 0 直角）")
                if s["borderTopWidth"] != "1px":
                    fails.append(f"[{tag}] 播放条上边框 {s['borderTopWidth']}（期望 1px）")
                if s["boxShadow"] != "none":
                    fails.append(f"[{tag}] 播放条仍有 inset 画框（{s['boxShadow']}）")
                if s["background"] != C3_RGB:
                    fails.append(f"[{tag}] 播放条底色 {s['background']}（期望 C3 {C3_RGB}）")
                if s["padding"] != "8px 16px":
                    fails.append(f"[{tag}] 播放条内边距 {s['padding']}（期望 8px 16px）")
                # 浏览器 alpha 舍入到 2 位小数（0.3412 → 0.34），前缀比到 0.34
                if s["iconColor"] and not s["iconColor"].startswith("rgba(234, 255, 247, 0.34"):
                    fails.append(f"[{tag}] 播放条图标色 {s['iconColor']}（期望标签档 {LABEL_RGBA}）")

            async def shot_footer(name):
                rect = await ev("(() => { const r = document.querySelector('.preview-footer')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width, height: r.height } : null; })()")
                if not rect:
                    return
                shot = await send(ws, "Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 2},
                }, session=sess)
                out = f"tools/style-preview/{name}.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(shot["result"]["data"]))
                print("截图:", out, f"({rect['width']:.0f}x{rect['height']:.0f})")

            async def open_card(i, wait=2.0):
                await ev(f"document.querySelectorAll('.asset-card')[{i}].dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}))")
                await asyncio.sleep(wait)

            async def close_preview():
                await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
                await asyncio.sleep(0.8)

            # 场景 A：动图（PlayBarFrames + 帧条；等帧流撑起 animTotal）
            await open_card(0, 3.0)
            total = await ev("!!document.querySelector('.preview-stage')")
            if not total:
                fails.append("[动图] 预览未打开")
            await check_bar("动图")
            strip = await ev(STRIP_STYLE_JS)
            print("[动图] 帧条:", json.dumps(strip, ensure_ascii=False))
            if not strip:
                fails.append("[动图] 帧条未渲染（帧流未撑起 animTotal？）")
            else:
                if strip["borderRadius"] != "0px":
                    fails.append(f"[动图] 帧条圆角 {strip['borderRadius']}（期望 0）")
                if strip["borderTopWidth"] != "1px" or strip["boxShadow"] != "none":
                    fails.append(f"[动图] 帧条边框/画框不符（top {strip['borderTopWidth']} / shadow {strip['boxShadow']}）")
                if strip["padding"] != "8px 16px":
                    fails.append(f"[动图] 帧条内边距 {strip['padding']}（期望 8px 16px）")
                # 两端按钮 = nav-history 面包屑同款：上一帧不旋转（朝左）/ 下一帧 rotate(180°)
                ds = strip.get("btnDs") or []
                if not all(d and d.startswith("M11.6251 2.5") for d in ds):
                    fails.append(f"[动图] 帧条两端图标 ≠ nav-history（{ds}）")
                rots = [rot_deg(t) for t in strip["btnTransforms"]]
                if len(rots) != 2 or rots[0] is not None or rots[1] != 180:
                    fails.append(f"[动图] 帧条两端按钮方向错（{rots}，期望 [不旋转, 180] 即 ‹ ›）")
            # 帧数文本色 = 常态档
            ft = await ev("(() => { const t = document.querySelector('.play-bar .frame-text'); return t ? getComputedStyle(t).color : null; })()")
            print("[动图] 帧数文本色:", ft)
            if ft and not ft.startswith("rgba(247, 255, 252, 0.694"):
                fails.append(f"[动图] 帧数文本色 {ft}（期望常态档）")
            await shot_footer("playbar-frames")
            await close_preview()

            # 场景 B：视频（PlayBarVideo 全量左组 6 件）
            await open_card(1, 2.0)
            await check_bar("视频")
            left_n = await ev("document.querySelector('.preview-footer .play-bar .btn-group')?.childElementCount ?? 0")
            print("[视频] 左组按钮数:", left_n)
            if left_n != 6:
                fails.append(f"[视频] 左组 {left_n} 件（期望 6：播放/快退/快进/上帧/下帧/音量）")
            # 左组朝向序列：播放 none / 快退 none（◀◀）/ 快进 flip / 上帧 none（◀|）/ 下帧 flip / 音量 none
            transforms = await ev(VIDEO_LEFT_JS)
            print("[视频] 左组 transform:", transforms)
            if transforms:
                expect = ["none", "none", FLIP, "none", FLIP, "none"]
                if transforms != expect:
                    fails.append(f"[视频] 左组朝向序列不符（{transforms}，期望 {expect}）")
            # hover 提亮 + tip tag 化（下一帧钮：非禁用）：真鼠标悬停 → 图标色提亮常态 + 400ms 出 tag 化 tip
            btn_rect = await ev("(() => { const b = document.querySelectorAll('.preview-footer .play-bar .btn-group .icon-btn')[4]; const r = b.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            color0 = await ev("getComputedStyle(document.querySelectorAll('.preview-footer .play-bar .btn-group .icon-btn')[4]).color")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": btn_rect["x"], "y": btn_rect["y"]}, session=sess)
            await asyncio.sleep(0.2)
            color1 = await ev("getComputedStyle(document.querySelectorAll('.preview-footer .play-bar .btn-group .icon-btn')[4]).color")
            print("[视频] 下一帧 hover 色:", color0, "→", color1)
            if color1 == color0 or not (color1 or "").startswith("rgba(247, 255, 252, 0.694"):
                fails.append(f"[视频] hover 未提亮到常态（{color0} → {color1}）")
            await asyncio.sleep(0.6)  # v-tip 400ms 延迟
            tip = await ev("(() => { const t = document.querySelector('.app-tip.show'); return t ? { text: t.textContent, kbd: [...t.querySelectorAll('kbd')].map((k) => k.textContent) } : null; })()")
            print("[视频] 下一帧 tip:", json.dumps(tip, ensure_ascii=False))
            if not tip or "下一帧" not in tip["text"] or tip["kbd"] != ["."]:
                fails.append(f"[视频] 下一帧 tip 未 tag 化（{tip}，期望 文本「下一帧」+ kbd [.]）")
            await shot_footer("playbar-video")
            await close_preview()

            # 场景 C：音频（PlayBarAudio 波形）
            await open_card(2, 2.0)
            await check_bar("音频")
            wave = await ev("!!document.querySelector('.preview-footer .play-bar .wave-progress')")
            if not wave:
                fails.append("[音频] 波形进度条未渲染")
            await shot_footer("playbar-audio")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 播放条/帧条贴 13 稿成立（三变体 46 高通栏直角 border-top C3 / 图标标签档 / 帧条 ←→ 修正）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
        server.close()


sys.exit(asyncio.run(main()))
