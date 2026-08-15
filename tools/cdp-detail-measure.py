# 一次性量测探针（2026-08-15 走查"分隔线不通栏 / padding 不对"）：
# 真实 dev app + 桩，选中文件卡片后逐块 dump computed style 与 getBoundingClientRect——
# 块左右 x vs 面板内容区左右 x（通栏判定）/ 各块 padding / border-top / box-shadow /
# 缩略图框宽（稿值 218）/ 横向溢出元素排查（scrollWidth > clientWidth 且 overflow-x 可见）。
# 结果打印 JSON，供修前对照；硬断言在 cdp-archive-probe.py。
import asyncio
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9361
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => encodeURI("/@fs/" + p),
  transformCallback: (cb) => 1,
  invoke: (cmd, args) => {
    const STILL = "__STILL__";
    const folders = [
      { id: "F-DIR", name: "素材", parent: null, kind: null },
      { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
      { id: "F-ARC", name: "设定集", parent: "F-DIR", kind: "archive" },
    ];
    const placements = [
      { assetId: "A1", folderId: "F-ARC", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A1", folderId: "F-SUB", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ];
    const nodeOf = (f) => ({ id: f.id, name: f.name, count: placements.filter((p) => p.folderId === f.id).length, kind: f.kind, block: f.parent === null ? "dir" : null, icon: null, children: folders.filter((c) => c.parent === f.id).map(nodeOf) });
    const R = {
      get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 2, folderCount: 3 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_placements: () => placements,
      list_folder_tree: () => ({ blocks: [{ id: "dir", name: "目录", icon: null }], nodes: folders.filter((f) => f.parent === null).map(nodeOf) }),
      list_archives: () => [{ folderId: "F-ARC", badge: { text: "设定", color: "#B38427" }, avatar: null, props: ["奇幻"], descriptions: [{ title: "来源", content: "官方设定集" }] }],
      list_recycle: [], list_recent_assets: [], list_inbox_snapshot: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产"], list_recent_libraries: [],
    };
    if (cmd === "save_ui_settings") return Promise.resolve(null);
    if (cmd === "get_asset_detail") {
      return Promise.resolve({
        assetId: "A1", originName: "主角-立绘.webp", ext: "webp", size: 204800, width: 800, height: 600, duration: null,
        importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00",
        tags: ["主角"], links: ["https://example.com/ref"], note: "",
        placements: [
          { folderId: "F-ARC", blockName: "目录", path: ["素材", "设定集"], name: "主角-立绘.webp", origin: true },
          { folderId: "F-SUB", blockName: "目录", path: ["素材", "子文件夹"], name: "主角-立绘.webp", origin: false },
        ],
        archives: [{ folderId: "F-ARC", name: "设定集" }],
      });
    }
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(typeof v === "function" ? v() : v);
  },
};
""".replace("__STILL__", STILL)

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


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    dev_proc = None
    if not dev_server_up():
        print("启动 pnpm dev ...")
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
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
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

            # 展开「素材」→ 进视口 → 选中文件卡片
            async def click(expr):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": r["x"], "y": r["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)

            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .expand')")
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.5)
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(1.0)

            dump = await ev(r"""(() => {
              const body = document.querySelector('.detail-body');
              const panel = document.querySelector('.detail-panel');
              if (!body || !panel) return { error: 'no panel' };
              const bb = body.getBoundingClientRect();
              const r2 = (el) => { const r = el.getBoundingClientRect(); return { l: Math.round(r.left * 10) / 10, r: Math.round(r.right * 10) / 10, w: Math.round(r.width * 10) / 10 }; };
              const blocks = [...panel.querySelectorAll('.detail-block')].map((b) => {
                const cs = getComputedStyle(b);
                return { cls: b.className.replace('detail-block', '').trim(), ...r2(b), padding: cs.padding, gap: cs.gap,
                         borderTop: cs.borderTopWidth, shadow: cs.boxShadow === 'none' ? 'none' : cs.boxShadow.slice(-60) };
              });
              const tg = panel.querySelector('.thumb-group');
              const cb = panel.querySelector('.custom-body');
              const thumb = panel.querySelector('.thumb');
              const root = panel.querySelector('.section-custom');
              const rootCs = getComputedStyle(root);
              // 横向溢出排查：右侧面板内 scrollWidth > clientWidth 的元素
              const overflow = [...document.querySelectorAll('.sidebar-right *')].filter((el) => el.scrollWidth > el.clientWidth + 1).map((el) => ({
                cls: (typeof el.className === 'string' ? el.className : '').slice(0, 50), sw: el.scrollWidth, cw: el.clientWidth, ox: getComputedStyle(el).overflowX,
              }));
              // 超宽元素（右缘超出面板内容区）
              const tooWide = [...panel.querySelectorAll('*')].map((el) => ({ el, r: el.getBoundingClientRect() }))
                .filter((x) => x.r.right > bb.right + 0.6 || x.r.left < bb.left - 0.6)
                .map((x) => ({ cls: (typeof x.el.className === 'string' ? x.el.className : '').slice(0, 50), ...r2(x.el) }));
              return {
                innerW: window.innerWidth,
                body: { ...r2(body), clientW: body.clientWidth, scrollW: body.scrollWidth, ox: getComputedStyle(body).overflowX },
                panel: r2(panel),
                rootPadding: rootCs.padding, rootGap: rootCs.gap, rootShadow: rootCs.boxShadow === 'none' ? 'none' : rootCs.boxShadow.slice(-60),
                blocks,
                thumbGroup: { ...r2(tg), padding: getComputedStyle(tg).padding, gap: getComputedStyle(tg).gap },
                customBody: { ...r2(cb), padding: getComputedStyle(cb).padding, gap: getComputedStyle(cb).gap, borderTop: getComputedStyle(cb).borderTopWidth, borderTopColor: getComputedStyle(cb).borderTopColor },
                thumb: r2(thumb),
                overflow,
                tooWide,
              };
            })()""")
            print(json.dumps(dump, ensure_ascii=False, indent=1))
            return 0
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


import sys
sys.exit(asyncio.run(main()))
