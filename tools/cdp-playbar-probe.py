# CDP 实测：预览悬浮播放条（2026-09-09 预览悬浮栏接入轮重写——FloatBar play 模式 + FrameStrip +
# 倍率/音量/时间显示悬停弹层，替代旧通栏三件套 PreviewPlayBar/PreviewFrameStrip/PreviewNotes，
# 13_播放条稿直角通栏口径作废；同日二轮：宿主合并 FloatBarHost（.fbh-anchor，两侧 12 呼吸）+
# 播放条 fill 横向填满 + 弹层对齐触发件上方居中）：壳 = 高 42 / 圆角 21 / 通用/半透明-2 底 /
# 模式根 padding 12 16 / 图标 亮色/三级；帧条 = .fb-pop 圆角 8 / 半透明-1 / 5 格分页 / 与播放条同宽满幅。
# 三变体覆盖：动图（帧条 5 格分页，假 WS 帧流撑起 animTotal）/ 视频（全量组 + 图标朝向 +
# hover 提亮 + tip + 倍率/音量/时间显示悬停弹层 + 弹层对齐 + fill 伸展）/ 音频（波形进度
# WaveProgress + 删减组）。截图留档。
import asyncio
import base64
import json
import re
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

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
      return Promise.resolve({ path: `E:/T/src.${ext}`, pending: false, animPreview: null });
    }
    if (cmd === "start_anim_stream") return Promise.resolve({ session: 1, port: WS_PORT });
    if (cmd === "get_media_source") return Promise.resolve({ path: "E:/T/src.mp4", transcoded: false, fps: 24 });
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
    """假帧流 server：每连接 20fps 推 160x90 RGBA 彩帧，20 字节头 idx 24 帧循环（对齐 frameCount）。"""
    async def handler(ws):
        seq = 0
        w, h = 160, 90
        try:
            while True:
                seq += 1
                idx = (seq - 1) % 24
                header = (
                    w.to_bytes(4, "little") + h.to_bytes(4, "little")
                    + idx.to_bytes(4, "little") + seq.to_bytes(8, "little")
                )
                rgba = bytes([seq % 256, 100, 200, 255]) * (w * h)
                await ws.send(header + rgba)
                await asyncio.sleep(0.05)
        except websockets.ConnectionClosed:
            pass
    return await websockets.serve(handler, "127.0.0.1", WS_PORT)


# 悬浮播放条壳断言（多功能悬浮栏稿：42 高 / 圆角 21 / 通用/半透明-2 底 / 模式根 padding 12 16 /
# 图标 亮色/三级）
BAR_STYLE_JS = """(() => {
  const shell = document.querySelector('.fbh-anchor .fb-shell');
  if (!shell) return null;
  const root = shell.querySelector('.fbp-root');
  const cs = getComputedStyle(shell);
  const icon = shell.querySelector('.ic-btn');
  return {
    height: cs.height, borderRadius: cs.borderRadius,
    background: cs.backgroundColor, padding: root ? getComputedStyle(root).padding : null,
    iconColor: icon ? getComputedStyle(icon).color : null,
    btnCount: root ? root.querySelectorAll('.ic-btn').length : 0,
    hasRate: !!root.querySelector('.fbp-rate'),
    shellW: shell.offsetWidth,
    anchorW: document.querySelector('.fbh-anchor')?.offsetWidth ?? null,
    panelW: document.querySelector('.fbh-anchor')?.parentElement?.offsetWidth ?? null,
  };
})()"""

# 帧条断言（播放条-帧条稿：.fb-pop 圆角 8 / 半透明-1 底 / padding 8 / 5 格 129x76 /
# 两端尖括号（下一帧 flip-h）/ 与播放条同宽）
STRIP_STYLE_JS = """(() => {
  const strip = document.querySelector('.fbh-anchor .fs-root');
  if (!strip) return null;
  const cs = getComputedStyle(strip);
  const navs = [...strip.querySelectorAll('.fs-nav .app-icon')].map((s) => getComputedStyle(s).transform);
  const shell = document.querySelector('.fbh-anchor .fb-shell');
  const frame = strip.querySelector('.fs-frame');
  return {
    borderRadius: cs.borderRadius, background: cs.backgroundColor, padding: cs.padding,
    navTransforms: navs, navCount: navs.length,
    frameCount: strip.querySelectorAll('.fs-frame').length,
    frameRadius: frame ? getComputedStyle(frame).borderRadius : null,
    stripW: strip.offsetWidth, shellW: shell ? shell.offsetWidth : null,
  };
})()"""

# 视频播放条图标朝向断言（悬浮栏族口径：seek 原样 ◀◀ = 快退 / flip-h ▶▶ = 快进；
# step-frame 原样 |◀ = 上一帧 / flip-h ▶| = 下一帧）——读 .ic-btn svg computed transform 序列比对
VIDEO_BTNS_JS = """(() => {
  const root = document.querySelector('.fbh-anchor .fbp-root');
  if (!root) return null;
  return [...root.querySelectorAll('.ic-btn .app-icon')].map((s) => getComputedStyle(s).transform);
})()"""
FLIP = "matrix(-1, 0, 0, 1, 0, 0)"  # scaleX(-1)

SHELL_BG = "rgba(41, 41, 41, 0.75)"  # 通用/半透明-2（--n-g-a2，2026-09-09 用户微调 0.5→0.75）
POP_BG = "rgba(31, 31, 31, 0.93)"  # 通用/半透明-1（--n-g-a1）
L2_RGB = "rgb(191, 191, 191)"  # 亮色/二级（--n-l2）
L3_RGB = "rgb(128, 128, 128)"  # 亮色/三级（--n-l3）


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

            async def check_bar(tag, btn_count, has_rate):
                s = await ev(BAR_STYLE_JS)
                print(f"[{tag}] 播放条:", json.dumps(s, ensure_ascii=False))
                if not s:
                    fails.append(f"[{tag}] 播放条未渲染")
                    return
                if s["height"] != "42px":
                    fails.append(f"[{tag}] 播放条高 {s['height']}（期望 42px）")
                if s["borderRadius"] != "21px":
                    fails.append(f"[{tag}] 播放条圆角 {s['borderRadius']}（期望 21px 胶囊）")
                if s["background"] != SHELL_BG:
                    fails.append(f"[{tag}] 播放条底色 {s['background']}（期望 半透明-2 {SHELL_BG}）")
                if s["padding"] != "12px 16px":
                    fails.append(f"[{tag}] 模式根内边距 {s['padding']}（期望 12px 16px）")
                if s["iconColor"] != L3_RGB:
                    fails.append(f"[{tag}] 播放条图标色 {s['iconColor']}（期望 亮色/三级 {L3_RGB}）")
                if s["btnCount"] != btn_count:
                    fails.append(f"[{tag}] 图标钮 {s['btnCount']} 件（期望 {btn_count}）")
                if s["hasRate"] != has_rate:
                    fails.append(f"[{tag}] 倍速钮存在性 {s['hasRate']}（期望 {has_rate}）")
                # fill = 横向填满（2026-09-09 二轮用户定）：壳 = anchor 宽，anchor = 面板 - 两侧 12 呼吸
                if s["shellW"] != s["anchorW"]:
                    fails.append(f"[{tag}] 播放条未填满 anchor（壳 {s['shellW']} ≠ anchor {s['anchorW']}）")
                if s["anchorW"] is None or s["panelW"] is None or s["anchorW"] != s["panelW"] - 24:
                    fails.append(f"[{tag}] anchor ≠ 面板 - 两侧12（{s['anchorW']} vs panel {s['panelW']}）")

            async def shot_anchor(name):
                rect = await ev("(() => { const r = document.querySelector('.fbh-anchor')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width, height: r.height } : null; })()")
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

            async def hover(sel, idx=None):
                """真鼠标悬停（选择器 + 可选序号）→ 返回元素中心坐标。"""
                q = f"document.querySelectorAll('{sel}')[{idx}]" if idx is not None else f"document.querySelector('{sel}')"
                rect = await ev(f"(() => {{ const b = {q}; if (!b) return null; const r = b.getBoundingClientRect(); return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }}; }})()")
                if not rect:
                    return None
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": rect["x"], "y": rect["y"]}, session=sess)
                return rect

            async def open_card(i, wait=2.0):
                await ev(f"document.querySelectorAll('.asset-card')[{i}].dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}))")
                await asyncio.sleep(wait)

            async def close_preview():
                await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
                await asyncio.sleep(0.8)

            # 场景 A：动图（帧条 5 格分页；等帧流撑起 animTotal）
            await open_card(0, 3.0)
            total = await ev("!!document.querySelector('.preview-stage')")
            if not total:
                fails.append("[动图] 预览未打开")
            await check_bar("动图", 4, True)  # 播放/倍率/笔记/设封面/全屏 = 4 图标 + 1 文字
            strip = await ev(STRIP_STYLE_JS)
            print("[动图] 帧条:", json.dumps(strip, ensure_ascii=False))
            if not strip:
                fails.append("[动图] 帧条未渲染（帧流未撑起 animTotal？）")
            else:
                if strip["borderRadius"] != "8px":
                    fails.append(f"[动图] 帧条圆角 {strip['borderRadius']}（期望 8px）")
                if strip["background"] != POP_BG:
                    fails.append(f"[动图] 帧条底色 {strip['background']}（期望 半透明-1 {POP_BG}）")
                if strip["padding"] != "8px":
                    fails.append(f"[动图] 帧条内边距 {strip['padding']}（期望 8px）")
                if strip["navCount"] != 2:
                    fails.append(f"[动图] 帧条两端钮 {strip['navCount']}（期望 2）")
                # 两端尖括号：上一帧不翻转（‹）/ 下一帧 flip-h（›）
                if strip["navTransforms"] != ["none", FLIP]:
                    fails.append(f"[动图] 帧条两端钮方向错（{strip['navTransforms']}，期望 [none, flip] 即 ‹ ›）")
                if strip["frameCount"] != 5:
                    fails.append(f"[动图] 帧条格数 {strip['frameCount']}（期望 5 格分页）")
                if strip["frameRadius"] != "8px":
                    fails.append(f"[动图] 帧格圆角 {strip['frameRadius']}（期望 8px）")
                if strip["stripW"] != strip["shellW"]:
                    fails.append(f"[动图] 帧条与播放条不同宽（{strip['stripW']} vs {strip['shellW']}）")
            # 时间码：帧号 当前 亮色/二级、总计 三级（数字-大档）
            tc = await ev("""(() => {
              const tc = document.querySelector('.fbh-anchor .fbp-tc');
              if (!tc) return null;
              return { text: tc.textContent, cur: getComputedStyle(tc.querySelector('.cur')).color,
                       tot: getComputedStyle(tc.querySelector('.tot')).color };
            })()""")
            print("[动图] 时间码:", json.dumps(tc, ensure_ascii=False))
            if not tc or tc["cur"] != L2_RGB or tc["tot"] != L3_RGB:
                fails.append(f"[动图] 时间码色档不符（{tc}，期望 当前 {L2_RGB} / 总计 {L3_RGB}）")
            # 帧条选中帧 = 亮色/一级 1px 边框（2026-09-09 四轮定稿；帧流播放中 frame 推进，当前格在页内）
            cur = await ev("""(() => {
              const f = document.querySelector('.fbh-anchor .fs-frame.current');
              if (!f) return null;
              const aft = getComputedStyle(f, '::after');
              return { borderColor: aft.borderColor, opacity: aft.opacity };
            })()""")
            print("[动图] 选中帧边框:", json.dumps(cur, ensure_ascii=False))
            if not cur or cur["borderColor"] != "rgb(242, 242, 242)" or cur["opacity"] != "1":
                fails.append(f"[动图] 选中帧边框不符（{cur}，期望 亮色/一级 1px opacity 1）")
            # 进度条已播放填充 = 亮色/三级（同轮定稿；light 档独有）
            fill = await ev("""(() => {
              const f = document.querySelector('.fbh-anchor .bs-root.light .bs-fill');
              return f ? { bg: getComputedStyle(f).backgroundColor } : null;
            })()""")
            print("[动图] 进度填充:", json.dumps(fill, ensure_ascii=False))
            if not fill or fill["bg"] != L3_RGB:
                fails.append(f"[动图] 进度填充段不符（{fill}，期望 亮色/三级 {L3_RGB}）")
            await shot_anchor("playbar-frames")
            await close_preview()

            # 场景 B：视频（全量组 10 图标 + 倍速文字钮；fill 模式进度条吃余量伸展）
            await open_card(1, 2.0)
            diag = await ev("""(() => {
              const shell = document.querySelector('.fbh-anchor .fb-shell');
              const root = shell?.querySelector('.fbp-root');
              const prog = root?.querySelector('.fbp-progress');
              return { shellW: shell?.offsetWidth, progW: prog?.offsetWidth,
                       fillGrow: prog ? getComputedStyle(prog).flexGrow : null };
            })()""")
            print("[视频] 宽度诊断:", json.dumps(diag, ensure_ascii=False))
            if not diag or diag["fillGrow"] != "1" or not diag["progW"] or diag["progW"] < 327:
                fails.append(f"[视频] fill 进度条未伸展（{diag}，期望 grow 1 且实宽 ≥ 327 稿值）")
            await check_bar("视频", 10, True)
            # 朝向序列：播放 none / 快退 none（◀◀）/ 快进 flip / 上帧 none（|◀）/ 下帧 flip /
            # 音量 none / 循环 none / 笔记 none / 设封面 none / 全屏 none
            transforms = await ev(VIDEO_BTNS_JS)
            print("[视频] 图标 transform:", transforms)
            if transforms:
                expect = ["none", "none", FLIP, "none", FLIP, "none", "none", "none", "none", "none"]
                if transforms != expect:
                    fails.append(f"[视频] 图标朝向序列不符（{transforms}，期望 {expect}）")
            # hover 提亮 + tip（下一帧钮 index 4）：图标 三级 → 二级；tip 400ms 出，快捷键纯文本
            color0 = await ev("getComputedStyle(document.querySelectorAll('.fbh-anchor .fbp-root .ic-btn')[4]).color")
            await hover(".fbh-anchor .fbp-root .ic-btn", 4)
            await asyncio.sleep(0.2)
            color1 = await ev("getComputedStyle(document.querySelectorAll('.fbh-anchor .fbp-root .ic-btn')[4]).color")
            print("[视频] 下一帧 hover 色:", color0, "→", color1)
            if color0 != L3_RGB or color1 != L2_RGB:
                fails.append(f"[视频] hover 未升二级（{color0} → {color1}）")
            await asyncio.sleep(0.6)  # v-tip 400ms 延迟
            tip = await ev("(() => { const t = document.querySelector('.app-tip'); return t ? { cls: t.className, text: t.textContent } : null; })()")
            print("[视频] 下一帧 tip:", json.dumps(tip, ensure_ascii=False))
            if not tip or "show" not in tip["cls"] or "下一帧" not in tip["text"] or "." not in tip["text"]:
                fails.append(f"[视频] 下一帧 tip 不符（{tip}，期望 .show 含「下一帧」与「.」纯文本）")
            # 倍率悬停弹层（RatePop 11 档横排，悬停开 + 移走延迟关）
            await hover(".fbh-anchor .fbp-rate")
            await asyncio.sleep(0.3)
            rp = await ev("(() => { const p = document.querySelector('.fbh-anchor .rp-root'); return p ? { n: p.querySelectorAll('.rp-item').length, bg: getComputedStyle(p).backgroundColor } : null; })()")
            print("[视频] 倍率弹层:", json.dumps(rp, ensure_ascii=False))
            if not rp or rp["n"] != 11 or rp["bg"] != POP_BG:
                fails.append(f"[视频] 倍率弹层未悬停开出（{rp}，期望 11 档 + 半透明-1）")
            # 音量悬停弹层（VolumePop 宽 124 稿值；对齐触发件上方居中——2026-09-09 二轮用户定）
            await hover(".fbh-anchor .fbp-root .ic-btn", 5)
            await asyncio.sleep(0.3)
            vp = await ev("""(() => {
              const p = document.querySelector('.fbh-anchor .vp-root');
              const b = document.querySelectorAll('.fbh-anchor .fbp-root .ic-btn')[5];
              if (!p || !b) return null;
              const pr = p.getBoundingClientRect(); const br = b.getBoundingClientRect();
              return { w: p.offsetWidth, bg: getComputedStyle(p).backgroundColor,
                       popCx: pr.left + pr.width / 2, btnCx: br.left + br.width / 2 };
            })()""")
            print("[视频] 音量弹层:", json.dumps(vp, ensure_ascii=False))
            if not vp or vp["w"] != 124 or vp["bg"] != POP_BG:
                fails.append(f"[视频] 音量弹层未悬停开出（{vp}，期望 宽 124 + 半透明-1）")
            elif abs(vp["popCx"] - vp["btnCx"]) > 2:
                fails.append(f"[视频] 音量弹层未对齐触发钮（pop cx {vp['popCx']:.0f} vs btn cx {vp['btnCx']:.0f}）")
            # 时间显示悬停弹层（RatePop 复用「帧数/时间码」两档，类 fbh-tc-pop 与倍率弹层区分；
            # 对齐时间码触发区）
            await hover(".fbh-anchor .fbp-tc")
            await asyncio.sleep(0.3)
            tcp = await ev("""(() => {
              const p = document.querySelector('.fbh-anchor .fbh-tc-pop');
              const b = document.querySelector('.fbh-anchor .fbp-tc');
              if (!p) return null;
              const pr = p.getBoundingClientRect(); const br = b.getBoundingClientRect();
              return { items: [...p.querySelectorAll('.rp-item')].map((x) => x.textContent),
                       popCx: pr.left + pr.width / 2, btnCx: br.left + br.width / 2 };
            })()""")
            print("[视频] 时间显示弹层:", json.dumps(tcp, ensure_ascii=False))
            if not tcp or tcp["items"] != ["帧数", "时间码"]:
                fails.append(f"[视频] 时间显示弹层不符（{tcp}，期望 [帧数, 时间码]）")
            elif abs(tcp["popCx"] - tcp["btnCx"]) > 2:
                fails.append(f"[视频] 时间显示弹层未对齐时间码（pop cx {tcp['popCx']:.0f} vs tc cx {tcp['btnCx']:.0f}）")
            await hover(".preview-stage")  # 移出关弹层
            # 全屏悬浮栏（2026-09-09 三轮钉：preview-root z-1500 铺满盖住 content-area 内 anchor →
            # 全屏时 anchor 改 fixed 窗口级 + z 1600；STUB invoke 兜底吞 window 插件调用）
            await ev("document.querySelectorAll('.fbh-anchor .fbp-root .ic-btn')[9]?.click(); true")
            await asyncio.sleep(0.5)
            fs = await ev("""(() => {
              const a = document.querySelector('.fbh-anchor');
              if (!a) return null;
              const cs = getComputedStyle(a); const r = a.getBoundingClientRect();
              return { pos: cs.position, z: cs.zIndex, bottomGap: Math.round(window.innerHeight - r.bottom),
                       inView: r.bottom <= window.innerHeight + 1 && r.top >= -1 };
            })()""")
            print("[视频] 全屏悬浮栏:", json.dumps(fs, ensure_ascii=False))
            if not fs or fs["pos"] != "fixed" or fs["z"] != "1600" or not fs["inView"] or fs["bottomGap"] != 12:
                fails.append(f"[视频] 全屏悬浮栏未浮窗级（{fs}，期望 fixed + z1600 + 视口内距底 12）")
            await shot_anchor("playbar-video-fullscreen")
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")  # 退全屏（不退预览）
            await asyncio.sleep(0.5)
            fs_off = await ev("getComputedStyle(document.querySelector('.fbh-anchor')).position")
            if fs_off != "absolute":
                fails.append(f"[视频] 退全屏悬浮栏未回 absolute（{fs_off}）")
            await shot_anchor("playbar-video")
            await close_preview()

            # 场景 C：音频（删减组：播放/音量/倍率/循环 = 3 图标 + 1 文字；波形进度 WaveProgress）
            await open_card(2, 2.0)
            await check_bar("音频", 3, True)
            wave = await ev("(() => { const w = document.querySelector('.fbh-anchor .wp-root'); if (!w) return null; return { layers: w.querySelectorAll('.wave-layer').length, head: !!w.querySelector('.head') }; })()")
            print("[音频] 波形进度:", json.dumps(wave, ensure_ascii=False))
            if not wave or wave["layers"] != 2 or not wave["head"]:
                fails.append(f"[音频] 波形进度条未渲染（{wave}，期望 双层 mask + 播放头）")
            # 删减组已由 check_bar 钉（3 图标 + 1 文字 = 播放/音量/循环 + 倍率，其余组全删）
            await shot_anchor("playbar-audio")

            # morph 回程（稿 prose「同一容器」口径，2026-09-09 二轮）：关闭预览 → 同一 FloatBar 壳
            # 切回搜索形态（宿主根部单例不卸载，fbp-root → fbs-input + 壳宽回 250）
            await close_preview()
            back = None
            for _ in range(20):
                back = await ev("(() => { const s = document.querySelector('.fbh-anchor .fb-shell'); return s && document.querySelector('.fbs-input') ? s.offsetWidth : null; })()")
                if back == 250:
                    break
                await asyncio.sleep(0.2)
            print("[回程] 搜索形态壳宽:", back)
            if back != 250:
                fails.append(f"[回程] 关闭预览未 morph 回搜索栏 250（{back}）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 预览悬浮播放条三变体成立（42 高圆角 21 半透明壳 / 帧条 5 格同宽 / 图标朝向与 hover / 悬停弹层 / 音频波形）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()
        server.close()


sys.exit(asyncio.run(main()))
