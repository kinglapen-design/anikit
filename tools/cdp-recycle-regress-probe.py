# CDP 复现：回收站基础操作回归诊断（2026-08-21，用户实机报"单选选中 / 双击名称改名被改坏"）
# 静态审查未见 5.2 改动触回收站链路，本探针实机复现定位断环：
# ① 进回收站视口渲染条目卡；② 单击 = 选中（.asset-card.selected）+ 详情面板显示；
# ③ 双击名称不进重命名（只读闸）；④ 右键 = 13 稿回收站菜单；⑤ 双击卡片行为观察。
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets
import asyncio

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9374
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 1, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "P1", folderId: "root", name: "参考.png", w: 640, h: 480, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true, frameCount: null, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
        size: 1000, createdAt: null, modifiedAt: "2026-08-21T10:00:00+08:00", importedAt: "2026-08-21T10:00:00+08:00" },
      { assetId: "S1", folderId: "root", name: "走路.png", w: 640, h: 480, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true, frameCount: 3, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: null, thumbFrame: "走路-001.png", fps: 24,
        size: 3000, createdAt: null, modifiedAt: "2026-08-21T10:00:00+08:00", importedAt: "2026-08-21T10:00:00+08:00" },
    ];
    const recycle = [
      { assetId: "R1", folderId: "recycle", name: "废稿.png", w: 640, h: 480, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: false, frameCount: null, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
        size: 2000, createdAt: null, modifiedAt: "2026-08-21T10:00:00+08:00", importedAt: "2026-08-21T10:00:00+08:00",
        deletedAt: "2026-08-21T11:00:00+08:00", placementCount: 1 },
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] },
      list_recycle: recycle, list_recent_assets: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.png", pending: false, animPreview: null },
      get_asset_detail: {
        assetId: "R1", originName: "废稿.png", ext: "png", size: 2000, width: 640, height: 480,
        duration: null, importedAt: "2026-08-21T10:00:00+08:00", modifiedAt: "2026-08-21T10:00:00+08:00",
        tags: [], links: [], note: "", placements: [], archives: [],
      },
      get_notes: [],
      list_sequence_frames: {
        fps: 24, thumbFrame: "走路-001.png",
        frames: [1, 2, 3].map((n) => ({
          name: `走路-00${n}.png`,
          fullPath: `E:/T/frames/走路-00${n}.png`,
          stripPath: `E:/T/cache/走路-00${n}.png.webp`,
        })),
      },
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


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-recycle-regress-")
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

            # 前置动线（模拟用户实机路径）：素材卡右键 → 打开序列帧包 → 包内单击帧 → 再导航
            await ev("""(() => {
              const cards = [...document.querySelectorAll('.asset-card')];
              const c = cards.find((x) => x.textContent.includes('走路'));
              const r = c.getBoundingClientRect();
              c.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.left + 20, clientY: r.top + 20 }));
            })()""")
            await asyncio.sleep(0.4)
            await ev("""(() => {
              const m = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.textContent.includes('打开序列帧包'));
              if (m) m.click();
            })()""")
            await asyncio.sleep(1.0)
            in_pack = await ev("document.querySelectorAll('.frame-card').length")
            print("包内帧卡:", in_pack)
            # 包内单击 + 双击帧（真实鼠标事件，pointer 链路全真）
            fc = await ev("(() => { const r = document.querySelector('.frame-card').getBoundingClientRect(); return { x: r.left + 30, y: r.top + 30 }; })()")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": fc["x"], "y": fc["y"], "button": "left", "clickCount": 1}, session=sess)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": fc["x"], "y": fc["y"], "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.3)
            sel_in_pack = await ev("document.querySelectorAll('.frame-card.selected').length")
            print("包内选中:", sel_in_pack)

            # 进回收站视口（侧栏快速访问"回收站"项）
            await ev("""(() => {
              const it = [...document.querySelectorAll('.tree-item')].find((x) => x.textContent.includes('回收站'));
              if (it) it.click();
            })()""")
            await asyncio.sleep(1.0)

            s1 = await ev("""({
              cards: document.querySelectorAll('.asset-card').length,
              names: [...document.querySelectorAll('.asset-card .title')].map((t) => t.textContent.trim()),
            })""")
            print("回收站视口:", json.dumps(s1, ensure_ascii=False))
            if not s1 or s1["cards"] != 1:
                fails.append(f"回收站条目卡未渲染（{s1}）")

            # ② 单击选中（真实鼠标事件——suppressClick 等指针链路全真）
            rc = await ev("(() => { const r = document.querySelector('.asset-card').getBoundingClientRect(); return { x: r.left + 30, y: r.top + 30 }; })()")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": rc["x"], "y": rc["y"], "button": "left", "clickCount": 1}, session=sess)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": rc["x"], "y": rc["y"], "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.4)
            s2 = await ev("""({
              selected: document.querySelectorAll('.asset-card.selected').length,
              detailBlocks: document.querySelectorAll('.detail-panel .detail-block').length,
            })""")
            print("单击后:", json.dumps(s2, ensure_ascii=False))
            if not s2 or s2["selected"] != 1:
                # 诊断：点击点位实际命中什么（覆盖层？）、body 残留类、拖拽痕迹
                diag = await ev(f"""(() => {{
                  const hit = document.elementFromPoint({rc["x"]}, {rc["y"]});
                  return {{
                    hit: hit ? hit.tagName + '.' + String(hit.className).slice(0, 80) : null,
                    bodyClass: document.body.className,
                    userSelect: document.body.style.userSelect,
                    marquee: !!document.querySelector('.marquee'),
                    zoomMask: !!document.querySelector('.zoom-mask'),
                    seqPackLeftover: !!document.querySelector('.seq-pack'),
                    frameCards: document.querySelectorAll('.frame-card').length,
                    menuOpen: !!document.querySelector('.app-menu'),
                  }};
                }})()""")
                print("诊断:", json.dumps(diag, ensure_ascii=False))
                fails.append(f"单击未选中（selected={s2 and s2['selected']}）——选中链路断")
            if s2 and s2["detailBlocks"] < 1:
                fails.append("选中后详情面板为空——recycleEntries 回退失效")

            # ③ 双击名称 = 不进重命名（回收站只读闸）
            await ev("document.querySelector('.asset-card .title')?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(0.4)
            has_input = await ev("!!document.querySelector('.asset-card .rename-input')")
            print("双击名称出重命名输入框:", has_input)
            if has_input:
                fails.append("回收站双击名称进了重命名——只读闸失效")

            # ④ 右键 = 回收站菜单（还原文件 / 还原全部文件 / 清空回收站）
            await ev("""(() => {
              const c = document.querySelector('.asset-card');
              const r = c.getBoundingClientRect();
              c.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.left + 20, clientY: r.top + 20 }));
            })()""")
            await asyncio.sleep(0.4)
            menu = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => m.textContent.trim())")
            print("回收站菜单:", json.dumps(menu, ensure_ascii=False))
            for needle in ["还原文件", "还原全部文件", "清空回收站"]:
                if not any(needle in (m or "") for m in (menu or [])):
                    fails.append(f"回收站菜单缺「{needle}」（{menu}）")
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")

            # ⑤ 双击卡片行为观察（预览是否打开——回收站预览无闸？仅打印供定性）
            await ev("document.querySelector('.asset-card')?.dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(1.2)
            preview_open = await ev("!!document.querySelector('.preview-stage')")
            print("双击卡片打开预览:", preview_open, "（现状观察，供定性）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 回收站选中 / 详情 / 只读闸 / 菜单全链路正常——用户实机问题不在此（查环境或更上游）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


sys.exit(asyncio.run(main()))
