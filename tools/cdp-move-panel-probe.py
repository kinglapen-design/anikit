# CDP 回归探针：3.4 移动面板（2026-09-08 换代 = 布局/模态窗口/移动 稿，BaseModal framed 壳；旧 29 稿口径退役）
#   S1 右键树 f2 →「移动文件夹」→ 面板结构：标题/宽 460/搜索框（TextInput 左置图标）/
#      区块标题「目录 / 3」（f2 被排除）/ f1-1 可见（默认全展开）/ 面板内无 f2
#   S2 hover f1 中部 → drop-into；上部 → drop-before（三分判定，TreeRowItem dropZone）
#   S3 点击 f1 中部 → ConfirmModal（warning）：标题"移动文件夹" + accent 高亮段「场景」「角色」+
#      按钮次左主右 = 取消 / 移动文件类（确认在右）
#   S4 确认 → move_folders{folderIds:["f2"], targetParent:"f1"} + 面板关闭
#   S5 右键档案 f3 →「移动档案」→ 标题"移动档案"；X 关闭
#   S6 素材卡选中 → Ctrl+Y → 标题"移动文件" → 点 f1 中部 → 确认 → move_placements{targetFolder:"f1"}
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-move-panel-probe.py
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
PORT = 9357
URL = "http://localhost:5373"
SHOT = "tools/style-preview/move-panel.png"
SHOT2 = "tools/style-preview/move-panel-confirm.png"
SLEEP = 0.3

# 桩：f1 角色（子 f1-1 主角）/ f2 场景 / f3 设定集（档案）；1 条 root 素材 AN-A。
# move_folders / move_placements 真实改数据（refresh 重拉），调用参数 JSON 脱壳入 __calls。
STUB = r"""
window.__calls = { move_folders: [], move_placements: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 0, children: [
      { id: "f1-1", name: "主角", count: 0, children: [] },
    ] },
    { id: "f2", name: "场景", count: 0, children: [] },
    { id: "f3", name: "设定集", count: 0, kind: "archive", children: [] },
  ],
};
const PLACEMENTS = [{ assetId: "AN-A", folderId: "root", name: "图A.png", w: 800, h: 600, ext: "png",
  thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, noteCount: null }];
const findNode = (nodes, id) => {
  for (const n of nodes) {
    if (n.id === id) return n;
    const h = findNode(n.children, id);
    if (h) return h;
  }
  return null;
};
const detach = (nodes, id) => {
  const i = nodes.findIndex((n) => n.id === id);
  if (i >= 0) return nodes.splice(i, 1)[0];
  for (const n of nodes) {
    const h = detach(n.children, id);
    if (h) return h;
  }
  return null;
};
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 1, folderCount: 4 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], detect_doc_apps: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "move_folders") {
      window.__calls.move_folders.push(JSON.parse(JSON.stringify(args ?? {})));
      for (const id of args.folderIds ?? []) {
        const n = detach(TREE.nodes, id);
        if (!n) continue;
        const tp = args.targetParent && args.targetParent !== "root" ? findNode(TREE.nodes, args.targetParent) : null;
        if (tp) tp.children.push(n); else TREE.nodes.push(n);
      }
      return Promise.resolve((args.folderIds ?? []).length);
    }
    if (cmd === "move_placements") {
      window.__calls.move_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      for (const it of args.items ?? []) {
        const p = PLACEMENTS.find((x) => x.assetId === it.assetId && x.folderId === it.folderId);
        if (p) p.folderId = args.targetFolder;
      }
      return Promise.resolve((args.items ?? []).length);
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

            async def tree_item_rect(fid):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def panel_item_rect(name, ratio=0.5):
                return await ev(f"""
                  (() => {{
                    const el = [...document.querySelectorAll('.move-panel .tree-item')]
                      .find((i) => i.textContent.trim() === {json.dumps(name)});
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height * {ratio}) }};
                  }})()
                """)

            async def menu_item_center(text):
                return await ev(f"""
                  (() => {{
                    const it = [...document.querySelectorAll('.app-menu .menu-item')]
                      .find((m) => m.querySelector('.label')?.textContent === {json.dumps(text)});
                    if (!it) return null;
                    const b = it.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def right_click_tree(fid):
                c = await tree_item_rect(fid)
                if not c:
                    return False
                await mouse("mousePressed", c["x"], c["y"], 2, button="right")
                await mouse("mouseReleased", c["x"], c["y"], 0, button="right")
                await asyncio.sleep(0.6)
                return bool(await ev("!!document.querySelector('.app-menu')"))

            async def click_menu(text):
                c = await menu_item_center(text)
                if not c:
                    return False
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.6)
                return True

            async def shot(path):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            # ---- S1 右键 f2 → 移动文件夹 → 面板结构 ----
            if not await right_click_tree("f2"):
                check("S1 右键 f2 开菜单", False)
            else:
                check("S1a 右键 f2 开菜单", True)
                if await click_menu("移动文件夹"):
                    await asyncio.sleep(0.6)
            panel = await ev("""
              (() => {
                const m = document.querySelector('.move-panel .bm-modal');
                if (!m) return null;
                return {
                  title: m.querySelector('.bm-titlebar .bm-title')?.textContent ?? "",
                  width: Math.round(m.getBoundingClientRect().width),
                  search: !!m.querySelector('.mv-search .ti-root input'),
                  blockTitle: m.querySelector('.st .txt')?.textContent ?? "",
                  blockCount: m.querySelector('.st .num')?.textContent?.trim() ?? "",
                  items: [...m.querySelectorAll('.tree-item')].map((i) => i.textContent.trim()),
                };
              })()
            """)
            check("S1b 面板开", panel is not None)
            if panel:
                check("S1c 标题=移动文件夹", panel["title"] == "移动文件夹", panel["title"])
                check("S1d 宽=460", panel["width"] == 460, str(panel["width"]))
                check("S1e 搜索框在", panel["search"])
                check("S1f 区块标题=目录", panel["blockTitle"] == "目录", panel["blockTitle"])
                check("S1g 计数=3（f2 排除）", panel["blockCount"] == "3", panel["blockCount"])
                check("S1h 默认全展开（主角可见）", "主角" in panel["items"], ",".join(panel["items"]))
                check("S1i 面板内无 f2 场景", "场景" not in panel["items"], ",".join(panel["items"]))

            # ---- S2 hover 三分判定 ----
            mid = await panel_item_rect("角色", 0.5)
            if mid:
                await mouse("mouseMoved", mid["x"], mid["y"], 0, button="none")
                await asyncio.sleep(SLEEP)
                zone = await ev("document.querySelector('.move-panel .tree-item.drop-into')?.textContent?.trim() ?? null")
                check("S2a 中部 hover=into", zone == "角色", str(zone))
                top = await panel_item_rect("角色", 0.08)
                await mouse("mouseMoved", top["x"], top["y"], 0, button="none")
                await asyncio.sleep(SLEEP)
                zone = await ev("document.querySelector('.move-panel .tree-item.drop-before')?.textContent?.trim() ?? null")
                check("S2b 上部 hover=before", zone == "角色", str(zone))
                await mouse("mouseMoved", mid["x"], mid["y"], 0, button="none")
                await asyncio.sleep(SLEEP)
                await shot(SHOT)
            else:
                check("S2 面板内定位角色", False)

            # ---- S3 点击中部 → ConfirmModal 确认窗（2026-09-08 AppConfirm 换代 ConfirmModal） ----
            if mid:
                await click_xy(mid["x"], mid["y"])
                await asyncio.sleep(0.6)
            conf = await ev("""
              (() => {
                const m = document.querySelector('.kit-modal');
                if (!m) return null;
                return {
                  title: m.querySelector('.title')?.textContent ?? "",
                  accents: [...m.querySelectorAll('.accent')].map((a) => a.textContent),
                  buttons: [...m.querySelectorAll('.actions .btn')].map((b) => b.textContent.trim()),
                  confirmKind: [...m.querySelectorAll('.actions .btn.confirm')].map((b) => b.className),
                  body: m.querySelector('.body')?.textContent ?? "",
                };
              })()
            """)
            check("S3a 确认窗 ConfirmModal 开", conf is not None)
            if conf:
                check("S3b 标题=移动文件夹", conf["title"] == "移动文件夹", conf["title"])
                check("S3c 高亮段=场景/角色", conf["accents"] == ["场景", "角色"], json.dumps(conf["accents"], ensure_ascii=False))
                check("S3d 按钮次左主右=取消/移动文件夹", conf["buttons"] == ["取消", "移动文件夹"], json.dumps(conf["buttons"], ensure_ascii=False))
                check("S3e 确认钮 warning 档", any("warning" in k for k in conf["confirmKind"]), str(conf["confirmKind"]))
                check("S3f 文案句式", conf["body"] == "是否将「场景」移动至「角色」中", conf["body"])
                await shot(SHOT2)

            # ---- S4 确认执行 ----
            btn = await ev("""
              (() => {
                const b = [...document.querySelectorAll('.kit-modal .actions .btn')]
                  .find((x) => x.textContent.trim() === "移动文件夹");
                if (!b) return null;
                const r = b.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if btn:
                await click_xy(btn["x"], btn["y"])
                await asyncio.sleep(0.8)
            calls = await ev("window.__calls.move_folders")
            check("S4a move_folders 被调", len(calls) == 1, json.dumps(calls, ensure_ascii=False))
            if calls:
                c0 = calls[0]
                check("S4b 参数 folderIds/targetParent",
                      c0.get("folderIds") == ["f2"] and c0.get("targetParent") == "f1",
                      json.dumps(c0, ensure_ascii=False))
            check("S4c 面板关闭", not await ev("!!document.querySelector('.move-panel')"))
            check("S4d 确认窗关闭", not await ev("!!document.querySelector('.kit-modal')"))

            # ---- S5 档案 → 移动档案 ----
            if await right_click_tree("f3") and await click_menu("移动档案"):
                await asyncio.sleep(0.5)
            t5 = await ev("document.querySelector('.move-panel .bm-modal .bm-title')?.textContent ?? null")
            check("S5a 档案标题=移动档案", t5 == "移动档案", str(t5))
            x = await ev("""
              (() => {
                const b = document.querySelector('.move-panel .bm-titlebar .ic-btn');
                if (!b) return null;
                const r = b.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if x:
                await click_xy(x["x"], x["y"])
                await asyncio.sleep(0.5)
            check("S5b X 关闭面板", not await ev("!!document.querySelector('.move-panel')"))

            # ---- S6 素材 Ctrl+Y → 移动文件 ----
            card = await ev("""
              (() => {
                const c = document.querySelector('.asset-card');
                if (!c) return null;
                const b = c.getBoundingClientRect();
                return { x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) };
              })()
            """)
            if card:
                await click_xy(card["x"], card["y"])
                await asyncio.sleep(0.5)
                await send(ws, "Input.dispatchKeyEvent", {
                    "type": "keyDown", "key": "y", "code": "KeyY", "modifiers": 2, "windowsVirtualKeyCode": 89,
                }, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {
                    "type": "keyUp", "key": "y", "code": "KeyY", "modifiers": 2, "windowsVirtualKeyCode": 89,
                }, session=sess)
                await asyncio.sleep(0.6)
            check("S6a 素材卡选中+Ctrl+Y", card is not None)
            t6 = await ev("document.querySelector('.move-panel .bm-modal .bm-title')?.textContent ?? null")
            check("S6b 标题=移动文件", t6 == "移动文件", str(t6))
            mid6 = await panel_item_rect("角色", 0.5)
            if mid6:
                await mouse("mouseMoved", mid6["x"], mid6["y"], 0, button="none")
                await asyncio.sleep(SLEEP)
                await click_xy(mid6["x"], mid6["y"])
                await asyncio.sleep(0.6)
                btn6 = await ev("""
                  (() => {
                    const b = [...document.querySelectorAll('.kit-modal .actions .btn')]
                      .find((x) => x.textContent.trim() === "移动文件");
                    if (!b) return null;
                    const r = b.getBoundingClientRect();
                    return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
                  })()
                """)
                if btn6:
                    await click_xy(btn6["x"], btn6["y"])
                    await asyncio.sleep(0.8)
            calls6 = await ev("window.__calls.move_placements")
            check("S6c move_placements 被调", len(calls6) == 1, json.dumps(calls6, ensure_ascii=False))
            if calls6:
                c1 = calls6[0]
                check("S6d 参数 items/targetFolder",
                      c1.get("targetFolder") == "f1" and (c1.get("items") or [{}])[0].get("assetId") == "AN-A",
                      json.dumps(c1, ensure_ascii=False))
            check("S6e 面板关闭", not await ev("!!document.querySelector('.move-panel')"))

    finally:
        proc.terminate()
        if dev_proc is not None:
            dev_proc.terminate()

    print("\n=== 结果 ===")
    print(f"失败 {len(fails)} 项：" + ("、".join(fails) if fails else "无"))
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
