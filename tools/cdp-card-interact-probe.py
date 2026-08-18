# CDP 回归探针：3.4 七轮卡片交互五修复（2026-08-18）
#   全部视图：
#   S1 点击图片卡 meta 简介行 → 选中（T5：简介区 = 卡片本体）
#   S2 悬停音频卡 → 悬停层挂载 → 点缩略图 → 选中（T4：音频层 preventDefault 抑制 click 已拆）
#   S3 悬停视频卡 → 点缩略图 → 选中（T3 capture 后移后 click 正常派发）
#   S4 双击素材卡名称 → 就地重命名输入框出现 + 预览不开（T3）
#   S5 meta 行按下拖动 30px → 不出框选矩形（T5：简介区不作框选起点；起拖拽是预期）
#   文件夹视图（folder:f1）：
#   S6 双击文件夹卡片名称 → 重命名输入框出现 + 不进入该文件夹（T3）
#   S7 拖第二列文件夹卡到首列左侧 → .folder-drop-gap 出现且不覆盖首卡（T2）
#   S8 松手 → reorder_folders 被调（间隙排序路径）
#   S9 拖素材卡到首列左侧 → .drop-gap（Teleport body）不覆盖首卡（T2 素材侧）
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-card-interact-probe.py
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9359
URL = "http://localhost:5373"
SHOT = "tools/style-preview/card-interact.png"
SLEEP = 0.3

# 桩：f1 角色（子 f1-1 主角 / f1-2 配角）+ f2 场景；root 三条素材（图/音频/视频），f1 内一条图D。
STUB = r"""
window.__calls = { reorder_folders: [], move_folders: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 3, children: [
      { id: "f1-1", name: "主角", count: 0, children: [] },
      { id: "f1-2", name: "配角", count: 0, children: [] },
    ] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const PLACEMENTS = [
  { assetId: "AN-A", folderId: "root", name: "图A.png", w: 800, h: 600, ext: "png",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: null, bpm: null, importedFrom: null, noteCount: null },
  { assetId: "AN-B", folderId: "root", name: "音B.mp3", w: null, h: null, ext: "mp3",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: 66, bpm: null, importedFrom: null, noteCount: null },
  { assetId: "AN-C", folderId: "root", name: "视C.mp4", w: 1920, h: 1080, ext: "mp4",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: 120, bpm: null, importedFrom: null, noteCount: null },
  { assetId: "AN-D", folderId: "f1", name: "图D.png", w: 800, h: 600, ext: "png",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: null, bpm: null, importedFrom: null, noteCount: null },
  { assetId: "AN-E", folderId: "f1", name: "图E.png", w: 800, h: 600, ext: "png",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: null, bpm: null, importedFrom: null, noteCount: null },
];
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 4 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], detect_doc_apps: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
      get_media_source: { path: "E:/test-lib/x.mp4", pending: false },
      get_asset_file_path: { path: null, pending: false, animPreview: null },
      get_hires_waveform: null,
    };
    if (cmd === "reorder_folders" || cmd === "move_folders") {
      window.__calls[cmd].push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(cmd === "move_folders" ? (args.folderIds ?? []).length : null);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
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
        m = raw.get("method", "")
        if m == "Runtime.consoleAPICalled":
            args = raw["params"].get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            print(f"[console.{raw['params']['type']}]", text[:300])
        elif m == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    dev_proc = None
    if dev_server_up():
        print("5373 已在监听，复用现有 dev server")
    else:
        print("5373 未监听，启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
        if not dev_server_up():
            print("dev server 启动超时")
            return 2
        print("dev server 就绪")

    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    fails = []

    def check(name, cond, extra=""):
        print(("PASS " if cond else "FAIL ") + name + (f" | {extra}" if extra else ""))
        if not cond:
            fails.append(name)

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

            async def mouse(t, x, y, buttons, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons, "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def click_xy(x, y):
                await mouse("mousePressed", x, y, 1)
                await mouse("mouseReleased", x, y, 0)
                await asyncio.sleep(SLEEP)

            async def dblclick_xy(x, y):
                await mouse("mousePressed", x, y, 1, clicks=1)
                await mouse("mouseReleased", x, y, 0, clicks=1)
                await asyncio.sleep(0.08)
                await mouse("mousePressed", x, y, 1, clicks=2)
                await mouse("mouseReleased", x, y, 0, clicks=2)
                await asyncio.sleep(SLEEP + 0.2)

            async def rect_of(sel_expr):
                return await ev(f"""
                  (() => {{
                    const el = {sel_expr};
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ left: b.left, right: b.right, top: b.top, width: b.width,
                              x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def shot(path):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            # 等素材卡渲染（全部视图）
            for _ in range(30):
                n = await ev("document.querySelectorAll('.asset-card').length")
                if n and n >= 3:
                    break
                await asyncio.sleep(0.5)

            # ---- S1 点击图片卡 meta 行 → 选中 ----
            meta1 = await rect_of("[...document.querySelectorAll('.asset-card')][0]?.querySelector('.meta')")
            check("S0 图片卡 meta 行在", meta1 is not None)
            if meta1:
                await click_xy(meta1["x"], meta1["y"])
            sel = await ev("[...document.querySelectorAll('.asset-card')].findIndex((c) => c.classList.contains('selected'))")
            check("S1 点 meta 行选中卡片", sel == 0, f"selected index={sel}")

            # ----S2 音频卡：悬停出层 → 点缩略图选中 ----
            audio_thumb = await rect_of("[...document.querySelectorAll('.asset-card')][1]?.querySelector('.thumb')")
            if audio_thumb:
                await mouse("mouseMoved", audio_thumb["x"], audio_thumb["y"], 0, button="none")
                await asyncio.sleep(0.6)
                layer = await ev("!!document.querySelectorAll('.asset-card')[1]?.querySelector('.hover-audio')")
                check("S2a 音频悬停层挂载", layer)
                await click_xy(audio_thumb["x"], audio_thumb["y"])
                sel = await ev("[...document.querySelectorAll('.asset-card')].findIndex((c) => c.classList.contains('selected'))")
                check("S2b 点音频缩略图选中", sel == 1, f"selected index={sel}")
            else:
                check("S2 音频卡定位", False)

            # ---- S3 视频卡：点缩略图选中 ----
            video_thumb = await rect_of("[...document.querySelectorAll('.asset-card')][2]?.querySelector('.thumb')")
            if video_thumb:
                await mouse("mouseMoved", video_thumb["x"], video_thumb["y"], 0, button="none")
                await asyncio.sleep(0.6)
                layer = await ev("!!document.querySelectorAll('.asset-card')[2]?.querySelector('.hover-video')")
                check("S3a 视频悬停层挂载", layer)
                await click_xy(video_thumb["x"], video_thumb["y"])
                sel = await ev("[...document.querySelectorAll('.asset-card')].findIndex((c) => c.classList.contains('selected'))")
                check("S3b 点视频缩略图选中", sel == 2, f"selected index={sel}")
            else:
                check("S3 视频卡定位", False)

            # ---- S4 双击素材卡名称 → 重命名不开预览 ----
            title1 = await rect_of("[...document.querySelectorAll('.asset-card')][0]?.querySelector('.title')")
            if title1:
                await dblclick_xy(title1["x"], title1["y"])
            check("S4a 双击名称出重命名输入框", await ev("!!document.querySelector('.asset-card .rename-input')"))
            check("S4b 预览未开", (await ev("document.querySelectorAll('.preview-stage').length")) == 0)
            # Esc 退出命名态
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(SLEEP)

            # ---- S5 meta 行按下拖动 → 不出框选 ----
            if meta1:
                await mouse("mousePressed", meta1["x"], meta1["y"], 1)
                await mouse("mouseMoved", meta1["x"] + 30, meta1["y"] - 30, 1)
                await asyncio.sleep(SLEEP)
                check("S5a 简介行拖动不起框选", (await ev("document.querySelectorAll('.marquee').length")) == 0)
                check("S5b 简介行拖动 = 起拖拽", await ev("document.body.classList.contains('dragging')"))
                # 移到空白松手（无落点）
                await mouse("mouseMoved", meta1["x"] + 60, meta1["y"] - 60, 1)
                await mouse("mouseReleased", meta1["x"] + 60, meta1["y"] - 60, 0)
                await asyncio.sleep(SLEEP + 0.2)

            # ---- 进文件夹视图 folder:f1 ----
            t1 = await rect_of("document.querySelector('.tree-item[data-folder-id=\"f1\"]')")
            if t1:
                await click_xy(t1["x"], t1["y"])
                await asyncio.sleep(0.8)
            fcs = await ev("document.querySelectorAll('.folder-card').length")
            check("S6a 文件夹条带渲染 2 张", fcs == 2, str(fcs))

            # ---- S6 双击文件夹卡名称 → 重命名不进入 ----
            fname = await rect_of("[...document.querySelectorAll('.folder-card')][1]?.querySelector('.folder-name')")
            if fname:
                await dblclick_xy(fname["x"], fname["y"])
            check("S6b 双击名称出重命名输入框", await ev("!!document.querySelector('.folder-card .rename-input')"))
            cur = await ev("document.querySelector('.tree-item.selected')?.getAttribute('data-folder-id') ?? null")
            check("S6c 未进入配角（仍在 f1）", cur == "f1", str(cur))
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(SLEEP)

            # ---- S7 拖第二列文件夹卡到首卡左侧 → 间隙竖块不覆盖首卡 ----
            card2 = await rect_of("document.querySelectorAll('.folder-card')[1]")
            card1 = await rect_of("document.querySelectorAll('.folder-card')[0]")
            if card2 and card1:
                await mouse("mousePressed", card2["x"], card2["y"], 1)
                await mouse("mouseMoved", card2["x"] - 30, card2["y"], 1)  # 超阈值起拖
                await asyncio.sleep(SLEEP)
                # 首卡左侧 8%（before 区间）
                tx = round(card1["left"] + card1["width"] * 0.08)
                await mouse("mouseMoved", tx, card1["y"], 1)
                await asyncio.sleep(SLEEP)
                gap = await rect_of("document.querySelector('.folder-drop-gap')")
                check("S7a 首列前插竖块出现", gap is not None)
                if gap:
                    check("S7b 竖块不覆盖首卡", gap["right"] <= card1["left"] + 1,
                          f"gap right={gap['right']} card left={card1['left']}")
                await shot(SHOT)
                # ---- S8 松手 → reorder_folders ----
                await mouse("mouseReleased", tx, card1["y"], 0)
                await asyncio.sleep(0.6)
            calls = await ev("window.__calls.reorder_folders")
            check("S8 间隙排序 reorder_folders 被调", len(calls) == 1, json.dumps(calls, ensure_ascii=False))

            # ---- S9 素材卡拖到首列左侧 → 竖块不覆盖首卡 ----
            acards = await ev("""
              (() => [...document.querySelectorAll('.asset-card')].map((c) => {
                const b = c.getBoundingClientRect();
                return { left: b.left, right: b.right, width: b.width,
                         x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) };
              }))()
            """)
            if acards and len(acards) >= 2:
                a0, a1 = acards[0], acards[1]
                await mouse("mousePressed", a1["x"], a1["y"], 1)
                await mouse("mouseMoved", a1["x"] - 10, a1["y"] - 40, 1)
                await asyncio.sleep(SLEEP)
                tx = round(a0["left"] + a0["width"] * 0.05)
                await mouse("mouseMoved", tx, a0["y"], 1)
                await asyncio.sleep(SLEEP)
                gap = await rect_of("document.querySelector('.drop-gap')")
                check("S9a 素材首列前插竖块出现", gap is not None)
                if gap:
                    check("S9b 竖块不覆盖首卡", gap["right"] <= a0["left"] + 1,
                          f"gap right={gap['right']} card left={a0['left']}")
                await mouse("mouseReleased", tx, a0["y"], 0)
                await asyncio.sleep(SLEEP)
            else:
                check("S9 素材卡定位", False, str(acards))

    finally:
        proc.terminate()
        if dev_proc is not None:
            dev_proc.terminate()

    print("\n=== 结果 ===")
    print(f"失败 {len(fails)} 项：" + ("、".join(fails) if fails else "无"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
