# CDP 实测：预览窗口动图帧流 + 解包废除（2026-08-20，阶段四）
# 口径：动图全链路不依赖磁盘帧图（frames/ 导入解包已废除）——
# ① 预览播放走 WS 帧流出帧；② 播放条帧号/总帧数（metadata frameCount 撑）且帧号推进；
# ③ 暂停/恢复（set_anim_stream_playing）；④ 播放条拖动定位（seek_anim_stream）；
# ⑤ 设封面 / 笔记不再禁用——设封面调 set_thumb_frame 传 `f:{索引}`（origin 抽帧）；
# ⑥ 帧条按帧索引渲染（24 帧块，prepare_anim_strips 供给 stripPath）。
# 前端链路全真（preview store / PlayBarFrames / PreviewFrameStrip / PreviewAnimCanvas），帧源 python WS 桩。
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
PORT = 9366
WS_PORT = 9365
URL = "http://localhost:5373"

STUB = r"""
window.__starts = 0; window.__stops = 0; window.__sid = 0;
window.__playings = []; window.__seeks = []; window.__rates = 0; window.__sizes = 0;
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
      get_ui_settings: { autoPlayGif: false }, // 卡片不自动播放：预览独享会话，计数干净
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.gif", pending: false },
      get_notes: [],
      prepare_anim_strips: { items: [] }, // 首调缓存为空，事件渐进（桩不补，帧块占位即可）
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
      window.__sizes++;
      return Promise.resolve(null);
    }
    if (cmd === "set_anim_stream_playing") {
      window.__playings.push(args.playing);
      return Promise.resolve(null);
    }
    if (cmd === "set_anim_stream_rate") {
      window.__rates++;
      return Promise.resolve(null);
    }
    if (cmd === "seek_anim_stream") {
      window.__seeks.push(args.frameIndex);
      return Promise.resolve(null);
    }
    if (cmd === "set_thumb_frame") {
      window.__thumbKeys = window.__thumbKeys || [];
      window.__thumbKeys.push(args.frameName);
      return Promise.resolve({ thumbPath: "E:/T/thumb.webp", modifiedAt: "2026-08-20T18:30:00+08:00" });
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
    """假帧流 server（协议 v2，同 cdp-anim-stream-probe 口径）：每连接 20fps 推 160x90 RGBA 彩帧，
    29 字节头 [tag=0][w][h][idx][pts_ms f64][seq]，idx 24 帧循环（对齐桩 frameCount），pts = idx×50ms。"""
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
    # 独立 user-data-dir：Edge 单实例机制下不带它会连进用户日常浏览器（踩坑见 anim-stream 探针）
    profile = tempfile.mkdtemp(prefix="cdp-preview-anim-")
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

            fails = []

            # 版本戳机制存在性（网格态）：首卡缩略图 src 应带 ?v=
            thumb_src0 = await ev("document.querySelector('.asset-card .thumb img')?.src ?? null")
            if not thumb_src0 or "?v=" not in thumb_src0:
                fails.append(f"卡片缩略图 URL 无 ?v= 版本戳（{thumb_src0 and thumb_src0[:80]}）")

            # 双击第一张卡 → 预览模式
            await ev("document.querySelector('.asset-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(2.5)  # 预览加载 + start_anim_stream + WS 连接 + 首帧

            # ① 预览 canvas 出帧 + 播放条 + 帧条（帧索引渲染 24 块）+ 无禁用钮
            s1 = await ev("""({
              stage: !!document.querySelector('.preview-stage'),
              canvas: (() => { const c = document.querySelector('.preview-stage canvas'); return c ? { w: c.width, h: c.height } : null; })(),
              barText: document.querySelector('.play-bar .frame-text')?.textContent?.trim() ?? null,
              disabledBtns: document.querySelectorAll('.play-bar button[disabled]').length,
              stripFrames: document.querySelectorAll('.frame-strip .frame').length,
              starts: window.__starts,
            })""")
            print("预览打开:", json.dumps(s1, ensure_ascii=False))
            if not s1 or not s1["stage"]:
                fails.append("预览模式未打开（无 .preview-stage）")
            if not s1 or not s1["canvas"] or s1["canvas"]["w"] != 160:
                fails.append(f"预览 canvas 未出帧（{s1 and s1['canvas']}）——帧流播放链路断")
            if not s1 or not s1["barText"] or not s1["barText"].endswith("/ 24"):
                fails.append(f"播放条总帧数不是 24（{s1 and s1['barText']}）——frameCount 撑分母失效")
            if s1 and s1["stripFrames"] != 24:
                diag = await ev("""(() => {
                  const s = document.querySelector('.frame-strip');
                  const btns = [...document.querySelectorAll('.play-bar .icon-btn')].length;
                  return { strip: s ? s.outerHTML.slice(0, 200) : null, iconBtns: btns };
                })()""")
                print("帧条诊断:", json.dumps(diag, ensure_ascii=False))
                fails.append(f"帧条帧块数 {s1 and s1['stripFrames']}（期望 24）——帧索引渲染失效")

            # ② 帧号随播放推进（两次采样分子不同）
            t1 = (await ev("document.querySelector('.play-bar .frame-text')?.textContent?.trim()")) or ""
            await asyncio.sleep(0.6)
            t2 = (await ev("document.querySelector('.play-bar .frame-text')?.textContent?.trim()")) or ""
            print(f"帧号推进: {t1} → {t2}")
            if t1 == t2:
                fails.append(f"帧号未推进（{t1} → {t2}）——播放未驱动")

            # ③ 暂停 / 恢复：点播放条第一钮（播放/暂停）
            await ev("document.querySelector('.play-bar .icon-btn').click()")
            await asyncio.sleep(0.3)
            await ev("document.querySelector('.play-bar .icon-btn').click()")
            await asyncio.sleep(0.3)
            playings = await ev("window.__playings")
            print("播放控制调用:", playings)
            if playings != [False, True]:
                fails.append(f"暂停/恢复命令不符（{playings}，期望 [False, True]）")

            # ④ 播放条拖动定位（CDP 真鼠标事件，pointer capture 需要有效 pointerId）
            prog = await ev("(() => { const r = document.querySelector('.play-bar .progress').getBoundingClientRect(); return { x: r.left + r.width * 0.5, y: r.top + r.height / 2 }; })()")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.15)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.4)
            seeks = await ev("window.__seeks")
            print("seek 调用:", seeks)
            if not seeks or not all(0 <= v <= 23 for v in seeks):
                fails.append(f"播放条拖动未产生合法 seek（{seeks}）")

            # ⑤ 设封面不再禁用：点设封面钮（.icon-btn 第 3 个：play/笔记/设封面/全屏）→ set_thumb_frame 传 `f:{索引}`
            await ev("document.querySelectorAll('.play-bar .icon-btn')[2]?.click()")
            await asyncio.sleep(0.3)
            thumb_keys = await ev("window.__thumbKeys || []")
            print("设封面调用:", thumb_keys)
            if not thumb_keys or not all(str(k).startswith("f:") for k in thumb_keys):
                fails.append(f"设封面未传 f: 帧索引（{thumb_keys}）——抽帧分支未接")
            # toast 反馈（2026-08-20 用户：点击无反馈）
            await asyncio.sleep(0.3)
            toasts = await ev("[...document.querySelectorAll('.toast-stack .toast .message')].map((t) => t.textContent.trim())")
            print("toast:", toasts)
            if not any("已设为预览图" in t for t in (toasts or [])):
                fails.append(f"设封面成功无 toast 反馈（{toasts}）")
            # modifiedAt 同步驱动 URL 版本戳变化：Esc 关预览回首卡 src（v 参数应为桩返回的时刻）
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(1.0)
            thumb_src1 = await ev("document.querySelector('.asset-card .thumb img')?.src ?? null")
            print("设封面前后 src:", (thumb_src0 or "")[:70], "→", (thumb_src1 or "")[:70])
            if not thumb_src1 or "2026-08-20" not in thumb_src1:
                fails.append(f"设封面后版本戳未更新（{thumb_src1 and thumb_src1[:90]}）——modifiedAt 同步失效")
            # 无禁用钮（解包废除后设封面/笔记恒可用）
            if s1 and s1["disabledBtns"] != 0:
                fails.append(f"禁用钮数 {s1 and s1['disabledBtns']}（期望 0——解包废除后恒可用）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 预览动图解包废除成立（帧流出帧 / 帧号推进 / 暂停恢复 / 拖动定位 / 设封面 f: 抽帧 + toast + 版本戳 / 帧条 24 块）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
        server.close()


sys.exit(asyncio.run(main()))
