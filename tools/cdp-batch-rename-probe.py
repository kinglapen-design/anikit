# CDP 回归探针：3.4 批量重命名（37 稿，2026-08-18）
#   S1 双选素材卡 → 右键「批量重命名」→ 弹窗结构：标题/宽 973/表头双列/2 行/segmented 两项/
#      确认钮初始禁用（无输入无变化项）
#   S2 序号命名（默认）：名称输"场景" → 新名预览 场景01/场景02（位宽 2，起始 1），原名卡不变，确认可用
#   S3 替换命名：切 segmented → 查找"场景"替换"镜头" → 原名卡"场景"段 .hit 品牌色高亮，
#      新名 镜头-1/镜头-2 实时预览
#   S4 确认 → rename_placements 被调（2 项 newName 正确）+ 弹窗关闭
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-batch-rename-probe.py
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
PORT = 9358
URL = "http://localhost:5373"
SHOT = "tools/style-preview/batch-rename.png"
SLEEP = 0.3

STUB = r"""
window.__calls = { rename_placements: [] };
const PLACEMENTS = [
  { assetId: "AN-A", folderId: "root", name: "场景-1.png", w: 800, h: 600, ext: "png",
    thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
    duration: null, bpm: null, importedFrom: null, noteCount: null },
  { assetId: "AN-B", folderId: "root", name: "场景-2.png", w: 800, h: 600, ext: "png",
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
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 2, folderCount: 0 },
      get_ui_settings: {},
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }], nodes: [] },
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], detect_doc_apps: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "rename_placements") {
      window.__calls.rename_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      for (const it of args.items ?? []) {
        const p = PLACEMENTS.find((x) => x.assetId === it.assetId && x.folderId === it.folderId);
        if (p) p.name = it.newName;
      }
      return Promise.resolve({ renamed: (args.items ?? []).length, failed: [] });
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

            async def mouse(t, x, y, buttons, button="left", clicks=1, modifiers=0):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons, "x": x, "y": y,
                    "clickCount": clicks, "modifiers": modifiers,
                }, session=sess)

            async def click_xy(x, y, modifiers=0, button="left"):
                await mouse("mousePressed", x, y, 1, button=button, modifiers=modifiers)
                await mouse("mouseReleased", x, y, 0, button=button, modifiers=modifiers)

            async def center(sel_expr):
                return await ev(f"""
                  (() => {{
                    const el = {sel_expr};
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def type_text(sel_expr, text):
                c = await center(sel_expr)
                if not c:
                    return False
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.2)
                await send(ws, "Input.insertText", {"text": text}, session=sess)
                await asyncio.sleep(0.4)
                return True

            async def shot(path):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            cards = await ev("""
              (() => [...document.querySelectorAll('.asset-card')].map((c) => {
                const b = c.getBoundingClientRect();
                return { x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) };
              }))()
            """)
            check("S0 素材卡渲染 2 张", len(cards) == 2, str(len(cards)))
            if len(cards) < 2:
                raise SystemExit(1)

            # ---- S1 双选 → 右键 → 批量重命名 ----
            await click_xy(cards[0]["x"], cards[0]["y"])
            await asyncio.sleep(0.3)
            await click_xy(cards[1]["x"], cards[1]["y"], modifiers=2)  # Ctrl 加选
            await asyncio.sleep(0.3)
            check("S1a 双选入域", await ev("document.querySelectorAll('.asset-card.selected').length") == 2)
            await click_xy(cards[1]["x"], cards[1]["y"], button="right")
            await asyncio.sleep(0.6)
            mi = await center("""
              [...document.querySelectorAll('.app-menu .menu-item')]
                .find((m) => m.querySelector('.label')?.textContent === "批量重命名")
            """)
            check("S1b 菜单项=批量重命名", mi is not None)
            if mi:
                await click_xy(mi["x"], mi["y"])
                await asyncio.sleep(0.6)
            dlg = await ev("""
              (() => {
                const m = document.querySelector('.rename-dialog .app-modal');
                if (!m) return null;
                return {
                  title: m.querySelector('.titlebar .title')?.textContent ?? "",
                  width: Math.round(m.getBoundingClientRect().width),
                  heads: [...m.querySelectorAll('.rd-head .col')].map((c) => c.textContent),
                  rows: m.querySelectorAll('.rd-row').length,
                  segs: [...m.querySelectorAll('.app-seg .item')].map((s) => s.textContent),
                  confirmDisabled: [...m.querySelectorAll('.rd-foot .app-btn')]
                    .find((b) => b.textContent.trim() === "确认")?.disabled ?? null,
                };
              })()
            """)
            check("S1c 弹窗开", dlg is not None)
            if dlg:
                check("S1d 标题=批量重命名", dlg["title"] == "批量重命名", dlg["title"])
                check("S1e 宽=973", dlg["width"] == 973, str(dlg["width"]))
                check("S1f 表头双列", dlg["heads"] == ["原始文件名", "新的文件名"], json.dumps(dlg["heads"], ensure_ascii=False))
                check("S1g 行数=2", dlg["rows"] == 2, str(dlg["rows"]))
                check("S1h segmented 两项", dlg["segs"] == ["序号命名", "替换命名"], json.dumps(dlg["segs"], ensure_ascii=False))
                check("S1i 确认初始禁用", dlg["confirmDisabled"] is True)

            # ---- S2 序号命名预览 ----
            await type_text("[...document.querySelectorAll('.rd-fields .app-input-shell')][0]?.querySelector('input')", "场景")
            prev = await ev("[...document.querySelectorAll('.rd-row')].map((r) => r.querySelectorAll('.rd-card')[1].querySelector('.rd-name').textContent)")
            check("S2a 序号预览 场景01/场景02", prev == ["场景01.png", "场景02.png"], json.dumps(prev, ensure_ascii=False))
            orig = await ev("[...document.querySelectorAll('.rd-row')].map((r) => r.querySelectorAll('.rd-card')[0].querySelector('.rd-name').textContent)")
            check("S2b 原名不变", orig == ["场景-1.png", "场景-2.png"], json.dumps(orig, ensure_ascii=False))
            en = await ev("[...document.querySelectorAll('.rd-foot .app-btn')].find((b) => b.textContent.trim() === '确认')?.disabled")
            check("S2c 确认可用", en is False)

            # ---- S3 替换命名 + 高亮 ----
            seg2 = await center("[...document.querySelectorAll('.app-seg .item')].find((s) => s.textContent === '替换命名')")
            if seg2:
                await click_xy(seg2["x"], seg2["y"])
                await asyncio.sleep(0.4)
            # 切模式后第一输入框 = 查找文本
            await type_text("[...document.querySelectorAll('.rd-fields .app-input-shell')][0]?.querySelector('input')", "场景")
            await type_text("[...document.querySelectorAll('.rd-fields .app-input-shell')][1]?.querySelector('input')", "镜头")
            prev3 = await ev("[...document.querySelectorAll('.rd-row')].map((r) => r.querySelectorAll('.rd-card')[1].querySelector('.rd-name').textContent)")
            check("S3a 替换预览 镜头-1/镜头-2", prev3 == ["镜头-1.png", "镜头-2.png"], json.dumps(prev3, ensure_ascii=False))
            hits = await ev("[...document.querySelectorAll('.rd-row')].map((r) => [...r.querySelectorAll('.rd-card')[0].querySelectorAll('.hit')].map((h) => h.textContent).join(''))")
            check("S3b 原名匹配段高亮", hits == ["场景", "场景"], json.dumps(hits, ensure_ascii=False))
            await shot(SHOT)

            # ---- S4 确认执行 ----
            btn = await center("[...document.querySelectorAll('.rd-foot .app-btn')].find((b) => b.textContent.trim() === '确认')")
            if btn:
                await click_xy(btn["x"], btn["y"])
                await asyncio.sleep(0.8)
            calls = await ev("window.__calls.rename_placements")
            check("S4a rename_placements 被调", len(calls) == 1, json.dumps(calls, ensure_ascii=False))
            if calls:
                items = calls[0].get("items", [])
                names = [i.get("newName") for i in items]
                check("S4b 新名清单", names == ["镜头-1.png", "镜头-2.png"], json.dumps(names, ensure_ascii=False))
            check("S4c 弹窗关闭", not await ev("!!document.querySelector('.rename-dialog')"))

    finally:
        proc.terminate()
        if dev_proc is not None:
            dev_proc.terminate()

    print("\n=== 结果 ===")
    print(f"失败 {len(fails)} 项：" + ("、".join(fails) if fails else "无"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
