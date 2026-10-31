# CDP 实测：语义搜索、语义片段预览定位、语义分析任务进度与取消。
# 运行：tools/.venv/Scripts/python.exe tools/cdp-semantic-probe.py
# 5373 未监听时自起 pnpm dev；仅清理本探针启动的 dev / Edge 进程树与假帧流端口。
import asyncio
import base64
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

from _probe_stub import make_stub

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

ROOT = Path(__file__).resolve().parent.parent
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
FFMPEG = ROOT / "src-tauri" / "binaries" / "ffmpeg-x86_64-pc-windows-msvc.exe"
URL = "http://127.0.0.1:5373"
CDP_PORT = 9391
WS_PORT = 9392
TASK_ID = "task-semantic-1"
EXPECTED_FILTERS = {
    "colors": [],
    "duration": None,
    "tags": [],
    "exts": ["seq", "gif", "mp4"],
    "folderIds": [],
    "ratios": [],
    "linkStates": [],
    "size": None,
    "imported": None,
    "modified": None,
}

HEAD = r"""
window.__calls = {
  search_assets: [], semantic_search: [], start_semantic_analysis: [], cancel_semantic_analysis: [],
  start_anim_stream: [], seek_anim_stream: [],
};
window.__evL = {};
window.__bootCalls = [];
window.__probeErrors = [];
window.addEventListener("error", (e) => window.__probeErrors.push(String(e.error?.stack ?? e.message)));
window.addEventListener("unhandledrejection", (e) => window.__probeErrors.push(String(e.reason?.stack ?? e.reason)));
const LIB = { name: "语义探针库", path: "E:/semantic-probe-lib", assetCount: 4, folderCount: 2, autoImport: false };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [{ id: "f1", name: "镜头文件夹", count: 3, children: [
    { id: "f2", name: "子文件夹", count: 1, children: [] },
  ] }],
};
const mk = (assetId, folderId, name, ext, extra = {}) => ({
  assetId, folderId, name, w: 640, h: 360, ext,
  thumbPath: null, thumbKind: null, thumbW: null, thumbH: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null,
  importedFrom: null, noteCount: null, thumbFrame: null, fps: null, playAssetId: null,
  size: 1000, createdAt: "2026-10-01T10:00:00+08:00", modifiedAt: "2026-10-02T10:00:00+08:00",
  importedAt: "2026-10-03T10:00:00+08:00", tags: [], ...extra,
});
const PLACEMENTS = [
  mk("V1", "f1", "镜头.mp4", "mp4", { duration: 12, fps: 24 }),
  mk("S1", "f1", "爆炸序列.png", "png", { frameCount: 16, fps: 1 }),
  mk("A1", "f2", "动作.gif", "gif", { frameCount: 12, animated: true, duration: 2.4 }),
  mk("D1", "root", "干扰项.png", "png"),
];
const seg = (segmentId, assetId, startMs, endMs, startFrame, endFrame, score, kind, text) => ({
  segmentId, assetId, startMs, endMs, startFrame, endFrame, score, kind, text,
});
const BEST_S = seg(101, "S1", 250, 900, 9, 12, 0.96, "sequence", "爆炸关键姿势");
const OTHER_S = seg(102, "S1", 1500, 1800, 13, 15, 0.61, "sequence", "爆炸余波");
const BEST_V = seg(201, "V1", 4250, 6000, 102, 144, 0.82, "video", "转身镜头");
const OTHER_V = seg(202, "V1", 500, 900, 12, 21, 0.55, "video", "入场");
const BEST_A = seg(301, "A1", 1400, 2300, 7, 10, 0.98, "anim", "变 delay 动作峰值");
const OTHER_A = seg(302, "A1", 100, 500, 1, 3, 0.52, "anim", "准备动作");
const HIT_S = { placements: [{ assetId: "S1", folderId: "f1" }], best: BEST_S,
  segments: [OTHER_S, BEST_S], matchCount: 2 };
const HIT_V = { placements: [{ assetId: "V1", folderId: "f1" }], best: BEST_V,
  segments: [OTHER_V, BEST_V], matchCount: 2 };
const HIT_A = { placements: [{ assetId: "A1", folderId: "f2" }], best: BEST_A,
  segments: [OTHER_A, BEST_A], matchCount: 2 };
// 定位探针只验 seek，不验连续播放：媒体 play 一到即暂停，避免 currentTime 离目标后继续走带。
document.addEventListener("play", (e) => { if (e.target instanceof HTMLMediaElement) e.target.pause(); }, true);
"""

HANDLERS = r"""
    window.__bootCalls.push(cmd);
    if (cmd === "plugin:event|listen") {
      window.__evL[args?.event] = args?.handler;
      return Promise.resolve(1);
    }
    if (cmd === "search_assets") {
      window.__calls.search_assets.push(JSON.parse(JSON.stringify(args ?? {})));
      const scope = args?.scope ?? {};
      const query = (args?.query ?? "").trim();
      if (query) return Promise.resolve({ placements: [], archives: [] });
      const exts = scope.filters?.exts ?? [];
      const inView = (p) => scope.global || scope.viewKey === "quick:all" ||
        (scope.viewKey === "folder:f1" && (p.folderId === "f1" || (scope.recursive && p.folderId === "f2")));
      const extOk = (p) => !exts.length || exts.includes(p.ext.toLowerCase()) ||
        (exts.includes("seq") && p.frameCount != null && !p.animated);
      return Promise.resolve({ placements: PLACEMENTS.filter((p) => inView(p) && extOk(p))
        .map((p) => ({ assetId: p.assetId, folderId: p.folderId })), archives: [] });
    }
    if (cmd === "semantic_search") {
      window.__calls.semantic_search.push(JSON.parse(JSON.stringify(args ?? {})));
      const hits = args?.query === "镜头" ? [HIT_S, HIT_V] : args?.query === "动作" ? [HIT_A, HIT_V] : [];
      return Promise.resolve(structuredClone({ hits, actualBackend: "stub-cpu", modelRevision: "cdp-semantic-v1" }));
    }
    if (cmd === "get_media_source") {
      return Promise.resolve({ path: "E:/semantic-probe/video.webm", transcoded: false, fps: 24 });
    }
    if (cmd === "list_sequence_frames") {
      return Promise.resolve({ fps: 1, thumbFrame: null, frames: Array.from({ length: 16 }, (_, i) => ({
        name: `爆炸-${String(i + 1).padStart(4, "0")}.png`,
        fullPath: `E:/semantic-probe/frames/爆炸-${String(i + 1).padStart(4, "0")}.png`,
        stripPath: null,
      })) });
    }
    if (cmd === "start_anim_stream") {
      window.__calls.start_anim_stream.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve({ session: 41, port: WS_PORT });
    }
    if (cmd === "seek_anim_stream") {
      window.__calls.seek_anim_stream.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "start_semantic_analysis") {
      window.__calls.start_semantic_analysis.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(TASK_ID);
    }
    if (cmd === "cancel_semantic_analysis") {
      window.__calls.cancel_semantic_analysis.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
""".replace("WS_PORT", str(WS_PORT)).replace("TASK_ID", json.dumps(TASK_ID))

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
            detail = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(detail.get("exception", {}).get("description", detail.get("text")))[:700])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = [str(a.get("value", a.get("description", "")))[:240] for a in raw["params"]["args"]]
            print("[CONSOLE-ERR]", " ".join(args)[:600])


def listening_pids(port: int) -> list[int]:
    try:
        out = subprocess.run(["netstat", "-ano", "-p", "tcp"], capture_output=True, text=True).stdout
    except Exception:
        return []
    pids = []
    for line in out.splitlines():
        parts = line.split()
        if len(parts) >= 5 and parts[3] == "LISTENING" and parts[1].endswith(f":{port}"):
            try:
                pids.append(int(parts[4]))
            except ValueError:
                pass
    return pids


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1).close()
        return True
    except Exception:
        return False


def kill_tree(pid: int) -> None:
    subprocess.run(["taskkill", "/PID", str(pid), "/F", "/T"], capture_output=True)


def make_video(temp_dir: Path) -> str:
    out = temp_dir / "semantic-video.webm"
    cmd = [
        str(FFMPEG), "-hide_banner", "-loglevel", "error", "-y",
        "-f", "lavfi", "-i", "testsrc2=duration=12:size=64x64:rate=24",
        "-an", "-c:v", "libvpx-vp9", "-deadline", "realtime", "-cpu-used", "8",
        "-crf", "40", "-b:v", "0", str(out),
    ]
    subprocess.run(cmd, check=True, capture_output=True, timeout=60)
    return base64.b64encode(out.read_bytes()).decode("ascii")


async def frame_server():
    async def handler(ws):
        # 协议 v2：tag/u32 w/u32 h/u32 frameIndex/f64 ptsMs/u64 seq/RGBA。
        # 只推首帧后保持连接；seek 目标帧断言不被后续自动播放帧覆盖。
        w, h = 2, 2
        packet = struct.pack("<BIIIdQ", 0, w, h, 0, 0.0, 1) + bytes([40, 80, 120, 255]) * (w * h)
        try:
            await ws.send(packet)
            await ws.wait_closed()
        except websockets.ConnectionClosed:
            pass

    return await websockets.serve(handler, "127.0.0.1", WS_PORT)


async def run_probe(ws, video_data: str) -> int:
    convert = (
        '(p) => p === "E:/semantic-probe/video.webm" ? '
        + json.dumps("data:video/webm;base64," + video_data)
        + ' : "asset://localhost/" + encodeURIComponent(p)'
    )
    stub = make_stub(
        {
            "get_current_library": "LIB",
            "get_ui_settings": "{ autoPlayGif: false, autoPlayVideo: false, doubleClickOpen: 'preview' }",
            "list_folder_tree": "TREE",
            "list_placements": "PLACEMENTS",
            "list_recent_libraries": "[]",
            "list_space_libraries": '["语义探针库"]',
            "get_notes": "[]",
            "prepare_anim_strips": "{ items: [] }",
            "get_asset_file_path": "{ path: null, pending: false, animPreview: null }",
        },
        head=HEAD,
        handlers=HANDLERS,
        convert_file_src=convert,
    )
    target = await send(ws, "Target.createTarget", {"url": "about:blank"})
    tid = target["result"]["targetId"]
    sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
    await send(ws, "Runtime.enable", session=sess)
    await send(ws, "Page.enable", session=sess)
    await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": stub}, session=sess)
    await send(ws, "Page.navigate", {"url": URL}, session=sess)

    async def ev(expr):
        r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
        if "exceptionDetails" in r.get("result", {}):
            detail = r["result"]["exceptionDetails"]
            print("[EV-EXC]", str(detail.get("exception", {}).get("description", detail.get("text")))[:600])
            return {"__error__": str(detail.get("text", "eval error"))}
        return r.get("result", {}).get("result", {}).get("value")

    async def until(fn, timeout=6.0, interval=0.1):
        deadline = time.time() + timeout
        value = None
        while time.time() < deadline:
            value = await fn()
            if value:
                return value
            await asyncio.sleep(interval)
        return value

    async def rect(expr):
        return await ev(f"""(() => {{ const el = {expr}; if (!el) return null;
          const r = el.getBoundingClientRect();
          return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }}; }})()""")

    async def click_rect(pos, modifiers=0):
        if not pos:
            return False
        for kind, buttons in (("mousePressed", 1), ("mouseReleased", 0)):
            await send(ws, "Input.dispatchMouseEvent", {
                "type": kind, "button": "left", "buttons": buttons, "modifiers": modifiers,
                "x": pos["x"], "y": pos["y"], "clickCount": 1,
            }, session=sess)
        await asyncio.sleep(0.25)
        return True

    async def click_el(expr, modifiers=0):
        return await click_rect(await rect(expr), modifiers)

    async def double_el(expr):
        pos = await rect(expr)
        if not pos:
            return False
        for count in (1, 2):
            for kind, buttons in (("mousePressed", 1), ("mouseReleased", 0)):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": kind, "button": "left", "buttons": buttons,
                    "x": pos["x"], "y": pos["y"], "clickCount": count,
                }, session=sess)
            await asyncio.sleep(0.06)
        await asyncio.sleep(0.2)
        return True

    async def click_text(selector, text):
        expr = f"""[...document.querySelectorAll({json.dumps(selector)})]
          .find((el) => el.textContent.trim() === {json.dumps(text)})"""
        return await click_el(expr)

    async def grid_keys():
        return await ev("[...document.querySelectorAll('.asset-card')].map((c) => c.getAttribute('data-key'))") or []

    async def grid_names():
        return await ev("[...document.querySelectorAll('.asset-card .kc-name')].map((c) => c.textContent.trim())") or []

    async def set_input(text):
        return await ev(f"""(() => {{ const i = document.querySelector('.fbs-input'); if (!i) return false;
          i.focus(); i.value = {json.dumps(text)}; i.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()""")

    async def snapshot():
        return await ev("""(() => {
          const pinia = document.querySelector('#app')?.__vue_app__?.config.globalProperties.$pinia;
          const s = pinia?.state.value;
          if (!s) return null;
          const id = s.tabs.activeTabId;
          const p = s[`preview#${id}`];
          const m = s[`previewMedia#${id}`];
          const sp = s.search.panels.get(id);
          const v = document.querySelector('.preview-stage video') ?? document.querySelector('video');
          return {
            id, assetKey: p?.assetKey ?? null, kind: p?.kind ?? null, frame: p?.frame ?? null,
            mediaTime: m?.currentTime ?? null, mediaSrc: m?.srcPath ?? null, mediaDuration: m?.duration ?? null,
            video: v ? { currentTime: v.currentTime, readyState: v.readyState, paused: v.paused } : null,
            selected: [...s.selection.selected], mode: sp?.mode ?? null,
            semanticOrder: sp ? [...sp.semanticOrder] : [], filters: sp?.filters ?? null,
            showSubfolders: s.ui.showSubfolders, semanticActive: s.semanticTasks.active ?? null,
          };
        })()""")

    def js(expr):
        async def wait_():
            return await ev(expr)
        return wait_

    def grid_count(expected):
        async def wait_():
            return len(await grid_keys()) == expected
        return wait_

    def grid_is(expected):
        async def wait_():
            return await grid_keys() == expected
        return wait_

    def state_pred(predicate):
        async def wait_():
            return bool(predicate(await snapshot()))
        return wait_

    def call_count(name, expected):
        async def wait_():
            return len(await ev(f"window.__calls.{name}") or []) == expected
        return wait_

    def call_list(name):
        async def wait_():
            return await ev(f"window.__calls.{name}") or []
        return wait_

    async def close_preview():
        await ev("document.querySelector('.preview-titlebar .nav-group .ic-btn')?.click(); true")
        await until(state_pred(lambda s: s["assetKey"] is None))
        return await until(js("!document.querySelector('.preview-root') && document.querySelectorAll('.asset-card').length > 0"))

    fails = []

    def check(name, cond, detail=""):
        suffix = f" — {detail}" if detail else ""
        print(f"[{'PASS' if cond else 'FAIL'}] {name}{suffix}")
        if not cond:
            fails.append(f"{name}: {detail}")

    async def boot_diag():
        return await ev("""({
          href: location.href, readyState: document.readyState,
          tauri: typeof window.__TAURI_INTERNALS__, bootCalls: window.__bootCalls ?? null,
          errors: window.__probeErrors ?? null,
          body: document.body?.innerText?.slice(0, 1200) ?? null,
          appHtml: document.querySelector('#app')?.innerHTML?.slice(0, 1200) ?? null,
          pinia: !!document.querySelector('#app')?.__vue_app__?.config.globalProperties.$pinia,
        })""")

    # Vite 冷启动先让模块图与首屏真实落地，再强制 reload；否则会在 app 尚未执行时反复打断首载。
    preloaded = await until(grid_count(4), timeout=45)
    print("[BOOT] 首次卡片挂载:", "PASS" if preloaded is True else "FAIL")
    if preloaded is not True:
        print("[BOOT-DIAG]", json.dumps(await boot_diag(), ensure_ascii=False))
        return 2
    # 踩坑 029/041/064/081：所有业务断言前强制 reload，排除 dev 页面旧码。
    await send(ws, "Page.reload", {"ignoreCache": True}, session=sess)
    ready = await until(grid_count(4), timeout=45)
    check("S0 强制 reload 后新页面与四张初始卡就绪", ready is True, f"cards={await grid_keys()}")
    if ready is not True:
        print("[BOOT-DIAG]", json.dumps(await boot_diag(), ensure_ascii=False))
        return 2

    # 文件夹视口 + 非默认 recursive=false，后代 A1 不参与第一组搜索。
    await click_el("document.querySelector('.tree-item[data-folder-id=\"f1\"]')")
    await until(grid_count(3))
    await ev("""(() => { const b = [...document.querySelectorAll('.panel-toolbar .btn-group .ic-btn')][3];
      if (!b) return false; b.click(); return true; })()""")
    await until(js("[...document.querySelectorAll('.app-menu .label')].some((x) => x.textContent.trim() === '显示子文件夹内容')"))
    await ev("""(() => { const row = [...document.querySelectorAll('.app-menu .row')]
      .find((r) => r.querySelector('.label')?.textContent.trim() === '显示子文件夹内容');
      const sw = row?.querySelector('.app-switch'); if (!sw) return false; sw.click(); return true; })()""")
    recursive_off = await until(state_pred(lambda s: s["showSubfolders"] is False))
    await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); true")
    check("S1a folder:f1 且递归关闭", recursive_off is True and sorted(await grid_keys()) == ["S1|f1", "V1|f1"],
          f"state={await snapshot()} cards={await grid_keys()}")

    # 非空 filters：seq + gif + mp4，候选弹窗与条件均走真实 UI。
    await click_el("document.querySelector('.fbs-ic.filter')")
    await click_text(".fbs-chip", "扩展名")
    await until(js("!!document.querySelector('.fp-root')"))
    for ext in ("seq", "gif", "mp4"):
        await click_text(".fp-root .tag-btn", ext)
        await asyncio.sleep(0.2)
    await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); true")
    filter_state = (await snapshot())["filters"]
    check("S1b 非空扩展名筛选已入 per-panel 状态", filter_state == EXPECTED_FILTERS,
          json.dumps(filter_state, ensure_ascii=False))

    # 搜索图标菜单 = 新码运行态确认；选择手动语义后等空查询重跑结束，再清零调用基线。
    await click_el("document.querySelector('.fbs-ic.search')")
    labels = await until(js("[...document.querySelectorAll('.app-menu .menu-item .label')].map((x) => x.textContent.trim())"))
    checked = await ev("document.querySelector('.app-menu .menu-item.checked .label')?.textContent.trim() ?? null")
    check("S2a 搜索图标打开智能/分词/语义菜单", labels == ["智能搜索", "分词搜索", "语义搜索"] and checked == "智能搜索",
          f"labels={labels} checked={checked}")
    await click_text(".app-menu .menu-item", "语义搜索")
    mode_ok = await until(state_pred(lambda s: s["mode"] == "semantic"))
    await asyncio.sleep(0.7)
    check("S2b 选择语义搜索进入 manual semantic", mode_ok is True, f"mode={(await snapshot())['mode']}")
    await ev("window.__calls.search_assets = []; window.__calls.semantic_search = []; true")

    await set_input("镜头")
    first_cards = await until(grid_is(["S1|f1", "V1|f1"]), interval=0.05)
    await until(call_count("semantic_search", 1))
    semantic_calls = await ev("window.__calls.semantic_search")
    keyword_calls = await ev("window.__calls.search_assets")
    first_args = semantic_calls[0] if semantic_calls else {}
    first_scope = first_args.get("scope", {})
    check("S3a 手动语义调用 semantic_search 而非 search_assets", len(semantic_calls) == 1 and keyword_calls == [],
          f"semantic={semantic_calls} search_assets={keyword_calls}")
    check("S3b scope viewKey/recursive/global/filters/limit/query 正确",
          first_args.get("query") == "镜头" and first_args.get("limit") == 30
          and first_scope.get("viewKey") == "folder:f1" and first_scope.get("recursive") is False
          and first_scope.get("global") is False and first_scope.get("filters") == EXPECTED_FILTERS,
          json.dumps(first_args, ensure_ascii=False))
    check("S4a 两个 asset 按相关度驱动网格且不重复 segment 卡片", first_cards is True and await grid_names() == ["爆炸序列.序列帧", "镜头.mp4"],
          f"keys={await grid_keys()} names={await grid_names()} state={(await snapshot())['semanticOrder']}")
    result_count = await ev("document.querySelector('.result-count')?.textContent.trim() ?? null")
    check("S4b 结果计数 = 2（不是 segment 数 4）", result_count == "2", str(result_count))

    # 单击只选择；双击视频用 best.startMs/1000 定位。
    await click_el("document.querySelector('.asset-card[data-key=\"S1|f1\"]')")
    single = await snapshot()
    await asyncio.sleep(0.3)
    single_after = await snapshot()
    check("S5 单击只选择，不打开预览", single["selected"] == ["S1|f1"] and single["assetKey"] is None
          and single_after["assetKey"] is None and not await ev("!!document.querySelector('.preview-root')"),
          json.dumps(single_after, ensure_ascii=False))

    await double_el("document.querySelector('.asset-card[data-key=\"V1|f1\"]')")
    video_ready = await until(state_pred(lambda s: s["kind"] == "video" and abs((s["mediaTime"] or 0) - 4.25) < 0.001),
                              timeout=8, interval=0.05)
    await until(state_pred(lambda s: (s["video"] or {}).get("readyState", 0) >= 1), timeout=5)
    video = await snapshot()
    video_index = await ev("({ cur: document.querySelector('.idx-cur')?.textContent, total: document.querySelector('.idx-total')?.textContent })")
    check("S6a 双击 video：previewMedia currentTime = best.startMs/1000", video_ready is True
          and video["assetKey"] == "V1|f1" and video["mediaSrc"] == "E:/semantic-probe/video.webm"
          and video["video"] is not None and abs(video["video"]["currentTime"] - 4.25) < 0.001,
          json.dumps(video, ensure_ascii=False))
    check("S6b previewList 第二项 = V1，总数 2", video_index == {"cur": "2", "total": "2"}, str(video_index))
    await close_preview()

    await double_el("document.querySelector('.asset-card[data-key=\"S1|f1\"]')")
    seq_ready = await until(state_pred(lambda s: s["kind"] == "sequence" and s["frame"] == 9), timeout=5, interval=0.02)
    seq = await snapshot()
    seq_index = await ev("({ cur: document.querySelector('.idx-cur')?.textContent, total: document.querySelector('.idx-total')?.textContent })")
    check("S7a 双击 sequence：frame = 原始 startFrame 9（非平均 fps 换算）", seq_ready is True and seq["assetKey"] == "S1|f1",
          json.dumps(seq, ensure_ascii=False))
    check("S7b previewList 首项 = 高相关度 S1，总数 2", seq_index == {"cur": "1", "total": "2"}, str(seq_index))
    await close_preview()

    # ~ 全局 + 同一路径 recursive=false：A1 在 f2，仅 global=true 基底可取。
    await set_input("~动作")
    second_cards = await until(grid_is(["A1|f2", "V1|f1"]), interval=0.05)
    await until(call_count("semantic_search", 2))
    semantic_calls = await ev("window.__calls.semantic_search")
    second_args = semantic_calls[-1] if semantic_calls else {}
    second_scope = second_args.get("scope", {})
    check("S8 全局语义 scope 正确且 query 剥 ~", second_cards is True and second_args.get("query") == "动作"
          and second_scope.get("viewKey") == "folder:f1" and second_scope.get("recursive") is False
          and second_scope.get("global") is True and second_scope.get("filters") == EXPECTED_FILTERS,
          json.dumps(second_args, ensure_ascii=False))
    check("S8b 手动语义全过程未回退普通 search_assets", await ev("window.__calls.search_assets") == [],
          json.dumps(await ev("window.__calls.search_assets"), ensure_ascii=False))

    await double_el("document.querySelector('.asset-card[data-key=\"A1|f2\"]')")
    anim_ready = await until(state_pred(lambda s: s["kind"] == "anim" and s["frame"] == 7), timeout=6, interval=0.02)
    anim = await snapshot()
    anim_seek = await ev("window.__calls.seek_anim_stream")
    anim_start = await ev("window.__calls.start_anim_stream")
    check("S9 双击 anim：变 delay 片段按 startFrame 7 seek（非 startMs×平均 fps）", anim_ready is True
          and anim["assetKey"] == "A1|f2" and len(anim_start) == 1
          and anim_seek == [{"session": 41, "frameIndex": 7}],
          f"state={json.dumps(anim, ensure_ascii=False)} seek={anim_seek} start={anim_start}")
    await close_preview()

    # 清空关键字并切回分词模式，恢复纯筛选结果供多选分析；调用计数断言已在上面完成。
    await ev("document.querySelector('.fbs-input')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); true")
    await click_el("document.querySelector('.fbs-ic.search')")
    await click_text(".app-menu .menu-item", "分词搜索")
    restored = await until(grid_is(["V1|f1", "S1|f1"]))
    check("S10a 分析前置：文件夹内两张卡恢复", restored is True, f"cards={await grid_keys()}")
    await click_el("document.querySelector('.asset-card[data-key=\"V1|f1\"]')")
    await click_el("document.querySelector('.asset-card[data-key=\"S1|f1\"]')", modifiers=2)
    selected = (await snapshot())["selected"]
    check("S10b 多选 V1 + S1", sorted(selected) == ["S1|f1", "V1|f1"], str(selected))

    await ev("""(() => { const c = document.querySelector('.asset-card[data-key="V1|f1"]'); if (!c) return false;
      const r = c.getBoundingClientRect();
      c.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.left + 20, clientY: r.top + 20 })); return true; })()""")
    await until(js("[...document.querySelectorAll('.app-menu .menu-item .label')].some((x) => x.textContent.trim() === 'AI 工具集')"))
    await ev("""(() => { const row = [...document.querySelectorAll('.app-menu .menu-item')]
      .find((x) => x.querySelector('.label')?.textContent.trim() === 'AI 工具集'); if (!row) return false;
      row.dispatchEvent(new MouseEvent('mouseenter', { bubbles: false })); return true; })()""")
    await until(js("[...document.querySelectorAll('.app-menu .menu-item .label')].some((x) => x.textContent.trim() === '分析文件向量')"))
    await click_text(".app-menu .menu-item", "分析文件向量")
    starts = await until(call_list("start_semantic_analysis"))
    expected_start = {"request": {"assetIds": ["V1", "S1"], "recursive": False}}
    check("S11a 右键分析文件向量：start_semantic_analysis assetIds 正确", starts == [expected_start],
          json.dumps(starts, ensure_ascii=False))

    progress_event = {
        "taskId": TASK_ID, "status": "running", "currentAssetId": "S1", "completed": 1, "total": 2,
        "actualBackend": "stub-cpu", "error": None,
    }
    event_ok = await ev(f"""(() => {{ const h = window.__evL['semantic-analysis-progress']; if (h == null) return false;
      window.__cbs[h]({{ event: 'semantic-analysis-progress', payload: {json.dumps(progress_event, ensure_ascii=False)} }}); return true; }})()""")
    active_ok = await until(state_pred(lambda s: bool(s["semanticActive"]) and s["semanticActive"].get("completed") == 1))
    check("S11b semantic-analysis-progress 事件接入 task store", event_ok is True and active_ok is True,
          json.dumps((await snapshot())["semanticActive"], ensure_ascii=False))
    progress = await ev("""(() => { const root = document.querySelector('.import-progress');
      if (!root) return null; return { text: root.querySelector('.label')?.textContent.trim(),
        width: root.querySelector('.fill')?.style.width ?? null, buttons: root.querySelectorAll('.ic-btn').length }; })()""")
    progress_ok = progress is not None and progress.get("text") == "正在语义分析 (1 / 2)" and progress.get("width") == "50%" and progress.get("buttons") == 1
    check("S11c 复用现有导入进度条并显示语义进度/取消钮", progress_ok, json.dumps(progress, ensure_ascii=False))

    cancel_via = "ui"
    if progress_ok:
        await click_el("document.querySelector('.import-progress .ic-btn')")
    else:
        # 诊断 fallback 只验证 store cancelActive 的命令封装，绝不算作进度条 UI 通过。
        cancel_via = "store-fallback"
        await ev("""(async () => { const pinia = document.querySelector('#app')?.__vue_app__?.config.globalProperties.$pinia;
          const tasks = pinia?._s?.get('semanticTasks'); if (!tasks) return false;
          await tasks.cancelActive(); return true; })()""")
    cancels = await until(call_list("cancel_semantic_analysis"))
    check("S11d cancel_semantic_analysis taskId 参数正确", cancels == [{"taskId": TASK_ID}],
          f"via={cancel_via} calls={json.dumps(cancels, ensure_ascii=False)}")

    print()
    if fails:
        print(f"失败 {len(fails)} 项：")
        for failure in fails:
            print(" -", failure)
        return 1
    print("全部 PASS")
    return 0


async def main() -> int:
    code = 2
    dev_proc = None
    edge = None
    server = None
    started_dev = False
    started_edge = False
    cleanup_ok = True
    with tempfile.TemporaryDirectory(prefix="cdp-semantic-") as temp:
        temp_dir = Path(temp)
        try:
            if listening_pids(CDP_PORT) or listening_pids(WS_PORT):
                print(f"环境阻塞：探针端口被占用 CDP={listening_pids(CDP_PORT)} WS={listening_pids(WS_PORT)}")
                return 2
            print("生成临时真实 VP9/WebM 视频 ...")
            video_data = make_video(temp_dir)
            if not dev_server_up():
                print("启动 pnpm dev (5373) ...")
                dev_proc = subprocess.Popen("pnpm dev", shell=True, cwd=ROOT,
                                            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                started_dev = True
                for _ in range(120):
                    if dev_server_up():
                        break
                    time.sleep(0.5)
                if not dev_server_up():
                    print("dev server 启动超时")
                    return 2
            else:
                print("复用既有 dev server (5373)；结束时不会清理该外部进程")
            # 在启动浏览器前直接检查 dev 所供源码，再经 reload + 菜单做运行态二次确认。
            semantic_source = urllib.request.urlopen(URL + "/src/lib/semantic.ts", timeout=5).read().decode("utf-8")
            host_source = urllib.request.urlopen(URL + "/src/components/FloatBarHost.vue", timeout=5).read().decode("utf-8")
            source_ok = "semantic_search" in semantic_source and "语义搜索" in host_source and "onSearchMode" in host_source
            print("新码源码检查:", "PASS" if source_ok else "FAIL")
            if not source_ok:
                return 2

            server = await frame_server()
            edge = subprocess.Popen([
                EDGE, f"--remote-debugging-port={CDP_PORT}", f"--user-data-dir={temp_dir / 'edge-profile'}",
                "--headless=new", "--disable-gpu", "--window-size=1600,1000", "--no-first-run", "about:blank",
            ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            started_edge = True
            version = None
            for _ in range(60):
                try:
                    version = json.load(urllib.request.urlopen(f"http://127.0.0.1:{CDP_PORT}/json/version", timeout=1))
                    break
                except Exception:
                    time.sleep(0.25)
            if not version:
                print("CDP 端口不可达")
                return 2
            async with websockets.connect(version["webSocketDebuggerUrl"], max_size=50 * 1024 * 1024) as ws:
                code = await run_probe(ws, video_data)
        except Exception as exc:
            print("探针环境/执行异常:", repr(exc))
            code = 2
        finally:
            if started_edge and edge is not None:
                kill_tree(edge.pid)
                try:
                    edge.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    pass
            if server is not None:
                server.close()
                await server.wait_closed()
            if started_dev and dev_proc is not None:
                kill_tree(dev_proc.pid)
                # pnpm 壳退出后 Vite node 可能脱离树；启动前已确认无监听，此端口只归本探针。
                for pid in listening_pids(5373):
                    kill_tree(pid)
                try:
                    dev_proc.wait(timeout=8)
                except subprocess.TimeoutExpired:
                    pass
            await asyncio.sleep(0.4)
            checks = []
            if started_edge:
                checks.append((f"Edge CDP {CDP_PORT}", not listening_pids(CDP_PORT)))
            if server is not None:
                checks.append((f"假帧流 {WS_PORT}", not listening_pids(WS_PORT)))
            if started_dev:
                checks.append(("dev 5373", not listening_pids(5373)))
            for name, ok in checks:
                print(f"[CLEANUP] {name}: {'PASS' if ok else 'FAIL'}")
                cleanup_ok = cleanup_ok and ok
    return code if cleanup_ok else 3


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
