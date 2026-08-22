# CDP 实测：5.3 视频全面接入自研帧流（2026-08-22）——
# 桩视频卡（duration 20s / fps 24 / 封面 t:5）+ 假 WS 帧流（协议 v2：29 字节头 + pts，
# 0.5s 视频时间/帧 ≈ 10 倍速，20s 一轮 2s 真实时间）：
# 阶段 A（autoPlayVideo 开）：① 自动播放起会话 startSecs=5（封面帧）loop=true；② 范围循环 =
# pts 越 15s 或回绕 0 即 seek_video_stream(5)（10s 封顶区间）；③ 卡片 canvas 出帧。
# 阶段 B（autoPlayVideo 关）：④ 悬停 = 悬停模式完整播放（startSecs=0）+ 音量胶囊（hasAudio）；
# ⑤ 鼠标横向移动 scrub → seek_video_stream 秒值随位置铺开；⑥ 双击开预览 → 播放条帧号 x/480 推进 /
# 暂停恢复 / 逐帧步进（暂停 + seek）/ 进度条点按定位 / 循环开关 / 笔记 t: key / 设封面 toast。
# 前端链路全真（VideoCanvasLayer / preview store / PlayBarVideo / streamClient），帧源 python WS 桩。
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
PORT = 9372
WS_PORT = 9371
URL = "http://localhost:5373"

# 帧流桩参数：视频时间 0.5s/帧、20s 一轮回绕（10 倍速，范围循环 / scrub 断言都落在 2s 真实周期内）
PTS_STEP = 0.5
DURATION = 20.0


def stub(autoplay_video: bool, count: int) -> str:
    return r"""
window.__starts = []; window.__stops = 0; window.__vseeks = [];
window.__playings = []; window.__loops = []; window.__notes = []; window.__vthumbs = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 6, folderCount: 0, autoImport: false };
    const placements = Array.from({ length: COUNT }, (_, i) => ({
      assetId: `V${i}`, folderId: "root", name: `视频${i}.mp4`,
      w: 1920, h: 1080, ext: "mp4", thumbPath: "E:/T/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: null, animated: false, duration: 20, bpm: null,
      importedFrom: null, noteCount: null, thumbFrame: "t:5.000", fps: 24,
    }));
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false, autoPlayVideo: AUTOPLAY },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.mp4", pending: false },
      get_notes: [],
    };
    if (cmd === "start_video_stream") {
      window.__starts.push({ startSecs: args.startSecs, loopEnabled: args.loopEnabled });
      return Promise.resolve({ session: window.__starts.length, port: WS_PORT, hasAudio: true, fps: 24, duration: 20 });
    }
    if (cmd === "stop_anim_stream") { window.__stops++; return Promise.resolve(null); }
    if (cmd === "seek_video_stream") { window.__vseeks.push(args.secs); return Promise.resolve(null); }
    if (cmd === "set_anim_stream_playing") { window.__playings.push(args.playing); return Promise.resolve(null); }
    if (cmd === "set_stream_loop") { window.__loops.push(args.loopEnabled); return Promise.resolve(null); }
    if (cmd === "set_note") {
      window.__notes.push(args);
      return Promise.resolve([{ key: args.key, text: args.text, updatedAt: Date.now() }]);
    }
    if (cmd === "set_video_thumb") {
      window.__vthumbs.push(args.atSecs);
      return Promise.resolve({ thumbPath: "E:/T/thumb.webp", modifiedAt: "2026-08-22T10:00:00+08:00" });
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
""".replace("WS_PORT", str(WS_PORT)).replace("AUTOPLAY", "true" if autoplay_video else "false").replace("COUNT", str(count))


async def frame_server():
    """假帧流（协议 v2）：每连接 20fps 推 160x90 彩帧，pts 0.5s/帧、20s 一轮回绕；
    idx = round(pts×24)（视频 idx 语义）。160x90 小帧 = CDP 与推流共存（见 anim 探针注释）。"""
    async def handler(ws):
        seq = 0
        w, h = 160, 90
        try:
            while True:
                seq += 1
                pts = ((seq - 1) * PTS_STEP) % DURATION
                idx = round(pts * 24)
                header = (
                    b"\x00"
                    + w.to_bytes(4, "little") + h.to_bytes(4, "little")
                    + idx.to_bytes(4, "little")
                    + struct.pack("<d", pts * 1000.0)
                    + seq.to_bytes(8, "little")
                )
                rgba = bytes([seq % 256, 100, 200, 255]) * (w * h)
                await ws.send(header + rgba)
                await asyncio.sleep(0.05)
        except websockets.ConnectionClosed:
            pass
    return await websockets.serve(handler, "127.0.0.1", WS_PORT)


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


async def new_page(ws, autoplay_video: bool, count: int):
    target = await send(ws, "Target.createTarget", {"url": "about:blank"})
    tid = target["result"]["targetId"]
    sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
    await send(ws, "Runtime.enable", session=sess)
    await send(ws, "Page.enable", session=sess)
    await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": stub(autoplay_video, count)}, session=sess)
    await send(ws, "Page.navigate", {"url": URL}, session=sess)
    await asyncio.sleep(6)
    return sess


async def main():
    server = await frame_server()
    # 独立 user-data-dir：Edge 单实例机制下不带它会连进用户日常浏览器（探针基建既定口径）
    profile = tempfile.mkdtemp(prefix="cdp-video-stream-")
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

            async def ev(expr, sess):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            fails = []

            # ===== 阶段 A：自动播放范围循环（封面 t:5 起 10s 封顶区间 [5,15] 循环） =====
            sess_a = await new_page(ws, autoplay_video=True, count=6)
            # 等 2 个 pts 周期（2s/轮）：范围循环 seek 应多次触发
            await asyncio.sleep(6)
            stats_a = await ev("""({
              starts: window.__starts, seeks: window.__vseeks,
              framed: [...document.querySelectorAll('.asset-card canvas')].filter((c) => c.width === 160).length,
              hud: document.querySelectorAll('.asset-card .hud').length,
            })""", sess_a)
            print("[自动播放]", json.dumps(stats_a, ensure_ascii=False))
            if not stats_a or not stats_a["starts"]:
                fails.append("[A] 自动播放未起视频会话")
            else:
                if not all(s["startSecs"] == 5 for s in stats_a["starts"]):
                    fails.append(f"[A] 起播点 ≠ 封面帧 t:5（{[s['startSecs'] for s in stats_a['starts']]}）")
                if not all(s["loopEnabled"] for s in stats_a["starts"]):
                    fails.append("[A] 卡片会话应恒 loop=true")
            if not stats_a or stats_a["framed"] == 0:
                fails.append("[A] 视频卡 canvas 未出帧（帧流链路断）")
            if stats_a and stats_a["hud"] != 0:
                fails.append(f"[A] 自动播放应无控件（hud {stats_a['hud']} 个）")
            seeks_a = (stats_a or {}).get("seeks", [])
            if len(seeks_a) < 2:
                fails.append(f"[A] 范围循环 seek 次数不足（{seeks_a}）——10s 区间循环未触发")
            if not all(abs(v - 5) < 0.01 for v in seeks_a):
                fails.append(f"[A] 范围循环 seek 目标 ≠ 5（{seeks_a[:6]}）")
            await send(ws, "Target.closeTarget", {"targetId": (await send(ws, "Target.getTargets"))["result"]["targetInfos"][-1]["targetId"]})

            # ===== 阶段 B：悬停完整播放 + scrub + 预览播放条 =====
            sess_b = await new_page(ws, autoplay_video=False, count=1)
            # ④ 悬停（真鼠标事件）：进卡 → 300ms 延迟 → 会话
            card_xy = await ev("(() => { const r = document.querySelector('.asset-card .thumb').getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2, w: r.width }; })()", sess_b)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": card_xy["x"], "y": card_xy["y"]}, session=sess_b)
            await asyncio.sleep(1.5)
            hover0 = await ev("""({
              starts: window.__starts,
              canvas: (() => { const c = document.querySelector('.asset-card canvas'); return c ? { w: c.width, h: c.height } : null; })(),
              hud: !!document.querySelector('.asset-card .hud'),
              capsule: document.querySelector('.asset-card .capsule .time')?.textContent?.trim() ?? null,
              volBtn: !!document.querySelector('.asset-card .hud-btn'),
            })""", sess_b)
            print("[悬停]", json.dumps(hover0, ensure_ascii=False))
            if not hover0 or not hover0["starts"]:
                fails.append("[B] 悬停未起视频会话")
            elif hover0["starts"][-1]["startSecs"] != 0:
                fails.append(f"[B] 悬停完整播放应从 0 起（{hover0['starts'][-1]['startSecs']}）")
            if not hover0 or not hover0["canvas"] or hover0["canvas"]["w"] != 160:
                fails.append(f"[B] 悬停 canvas 未出帧（{hover0 and hover0['canvas']}）")
            if not hover0 or not hover0["hud"]:
                fails.append("[B] 悬停模式应出控件区（胶囊 + 进度条）")
            if not hover0 or not hover0["volBtn"]:
                fails.append("[B] 有音轨应有音量钮")
            if not hover0 or not (hover0["capsule"] or "").isdigit():
                fails.append(f"[B] 胶囊应显示帧号（纯数字，timeDisplay 默认帧数）（{hover0 and hover0['capsule']}）")

            # ⑤ scrub：鼠标横向扫过卡片 → seek_video_stream 秒值铺开
            await ev("window.__vseeks = []", sess_b)
            left = card_xy["x"] - card_xy["w"] / 2
            for i in range(9):
                x = left + card_xy["w"] * (i / 8)
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": x, "y": card_xy["y"]}, session=sess_b)
                await asyncio.sleep(0.12)  # > 66ms 门控，每个位置都落实 seek
            await asyncio.sleep(0.5)
            seeks_b = await ev("window.__vseeks", sess_b)
            print("[悬停] scrub seeks:", seeks_b)
            if not seeks_b or len(seeks_b) < 4:
                fails.append(f"[B] scrub 未产生足够 seek（{seeks_b}）")
            elif not (min(seeks_b) < 4 and max(seeks_b) > 15):
                fails.append(f"[B] scrub seek 范围未铺开（[{min(seeks_b)}, {max(seeks_b)}]，期望覆盖 [0,20] 大部）")

            # ⑥ 双击开预览 → 播放条全套
            await ev("document.querySelector('.asset-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))", sess_b)
            await asyncio.sleep(2.5)
            p0 = await ev("""({
              stage: !!document.querySelector('.preview-stage'),
              canvas: (() => { const c = document.querySelector('.preview-stage canvas'); return c ? { w: c.width, h: c.height } : null; })(),
              barText: document.querySelector('.play-bar .tdc-btn')?.textContent?.trim() ?? null,
              disabledBtns: document.querySelectorAll('.play-bar button[disabled]').length,
              volBtn: !!document.querySelector('.play-bar .volume'),
              starts: window.__starts.length,
            })""", sess_b)
            print("[预览]", json.dumps(p0, ensure_ascii=False))
            if not p0 or not p0["stage"]:
                fails.append("[B] 预览模式未打开")
            if not p0 or not p0["canvas"] or p0["canvas"]["w"] != 160:
                fails.append(f"[B] 预览 canvas 未出帧（{p0 and p0['canvas']}）——预览帧流断")
            if not p0 or not p0["barText"] or not p0["barText"].endswith("/ 480"):
                fails.append(f"[B] 播放条总帧数不是 480（20s×24fps）（{p0 and p0['barText']}）")
            if p0 and p0["disabledBtns"] != 0:
                fails.append(f"[B] 播放条有禁用钮（{p0['disabledBtns']}）——帧流活着应全可用")
            if not p0 or not p0["volBtn"]:
                fails.append("[B] 播放条应有音量控件（hasAudio）")

            # 帧号推进（两采样分子不同）
            t1 = (await ev("document.querySelector('.play-bar .tdc-btn')?.textContent?.trim()", sess_b)) or ""
            await asyncio.sleep(0.6)
            t2 = (await ev("document.querySelector('.play-bar .tdc-btn')?.textContent?.trim()", sess_b)) or ""
            print(f"[预览] 帧号推进: {t1} → {t2}")
            if t1 == t2:
                fails.append(f"[B] 帧号未推进（{t1} → {t2}）")

            # 暂停 / 恢复
            await ev("window.__playings = []", sess_b)
            await ev("document.querySelector('.play-bar .icon-btn').click()", sess_b)
            await asyncio.sleep(0.3)
            await ev("document.querySelector('.play-bar .icon-btn').click()", sess_b)
            await asyncio.sleep(0.3)
            playings = await ev("window.__playings", sess_b)
            print("[预览] 播放控制:", playings)
            if playings != [False, True]:
                fails.append(f"[B] 暂停/恢复命令不符（{playings}，期望 [False, True]）")

            # 逐帧步进（下一帧钮 = 左组第 5 件；步进即暂停 + seekVideo）
            await ev("window.__vseeks = []; window.__playings = []", sess_b)
            await ev("document.querySelectorAll('.play-bar .btn-group .icon-btn')[4].click()", sess_b)
            await asyncio.sleep(0.4)
            step = await ev("({ seeks: window.__vseeks, playings: window.__playings })", sess_b)
            print("[预览] 逐帧步进:", step)
            if not step or not step["seeks"]:
                fails.append(f"[B] 逐帧步进未产生 seekVideo（{step}）")
            if not step or not step["playings"] or step["playings"][0] is not False:
                fails.append(f"[B] 逐帧步进应先暂停（playings {step and step['playings']}）")

            # 进度条点按定位（50% → seek ≈ 10s）
            await ev("window.__vseeks = []", sess_b)
            prog = await ev("(() => { const r = document.querySelector('.play-bar .progress').getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height / 2 }; })()", sess_b)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess_b)
            await asyncio.sleep(0.15)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess_b)
            await asyncio.sleep(0.4)
            seeks_bar = await ev("window.__vseeks", sess_b)
            print("[预览] 进度条定位 seeks:", seeks_bar)
            if not seeks_bar or not any(9.0 <= v <= 11.0 for v in seeks_bar):
                fails.append(f"[B] 进度条 50% 点按应 seek ≈10s（{seeks_bar}）")

            # 循环开关（右组 children：倍率[0] / 循环[1] / 笔记[2] / 设封面[3] / 全屏[4]——倍率根元素非 icon-btn）
            await ev("window.__loops = []", sess_b)
            await ev("[...document.querySelectorAll('.play-bar .btn-group')].pop().children[1].click()", sess_b)
            await asyncio.sleep(0.3)
            loops = await ev("window.__loops", sess_b)
            print("[预览] 循环开关:", loops)
            if loops != [True]:
                fails.append(f"[B] 循环开关命令不符（{loops}，期望 [True]）")

            # 笔记：打开浮层 → 写当前帧笔记（key = t:{秒}）（右组 children[2]）
            await ev("[...document.querySelectorAll('.play-bar .btn-group')].pop().children[2].click()", sess_b)
            await asyncio.sleep(0.5)
            note_open = await ev("!!document.querySelector('.notes-pop')", sess_b)
            if not note_open:
                fails.append("[B] 笔记浮层未打开")
            else:
                await ev("""(() => {
                  const ta = document.querySelector('.notes-pop textarea');
                  const setter = Object.getOwnPropertyDescriptor(window.HTMLTextAreaElement.prototype, 'value').set;
                  setter.call(ta, '这一帧有问题');
                  ta.dispatchEvent(new Event('input', { bubbles: true }));
                  ta.dispatchEvent(new Event('focusout', { bubbles: true }));
                  ta.blur();
                })()""", sess_b)
                await asyncio.sleep(0.5)
                notes_made = await ev("window.__notes", sess_b)
                print("[预览] 笔记:", notes_made)
                if not notes_made or not str(notes_made[0].get("key", "")).startswith("t:"):
                    fails.append(f"[B] 笔记 key 应为 t: 秒（{notes_made}）")

            # 设封面（右组 children[3]）→ set_video_thumb atSecs ≥ 0 + toast
            await ev("[...document.querySelectorAll('.play-bar .btn-group')].pop().children[3].click()", sess_b)
            await asyncio.sleep(0.5)
            vthumbs = await ev("window.__vthumbs", sess_b)
            toasts = await ev("[...document.querySelectorAll('.toast-stack .toast .message')].map((t) => t.textContent.trim())", sess_b)
            print("[预览] 设封面:", vthumbs, "toast:", toasts)
            if not vthumbs or not all(isinstance(v, (int, float)) and v >= 0 for v in vthumbs):
                fails.append(f"[B] 设封面未调 set_video_thumb（{vthumbs}）")
            if not any("已设为预览图" in t for t in (toasts or [])):
                fails.append(f"[B] 设封面成功无 toast（{toasts}）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 视频帧流全链路成立（自动播放封面起 10s 范围循环 / 悬停完整播放 + scrub / "
                  "预览帧号推进 / 暂停恢复 / 真逐帧 / 进度条定位 / 循环 / 笔记 t: / 设封面 toast）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
        server.close()


sys.exit(asyncio.run(main()))
