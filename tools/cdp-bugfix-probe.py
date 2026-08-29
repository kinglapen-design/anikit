# CDP 探针：阶段五前 BUG 修复轮（2026-08-20，10 项中可断言的 8 项）
#   S1 BUG7 树数量：折叠 = 递归总数 / 展开 = 直接子项数
#   S2 BUG3 树定位：双击文件夹卡片进视口 → 树展开祖先链 + 节点选中可见
#   S3 BUG9 组内拖拽：同组出 reorder 调用 / 跨组无调用
#   S4 BUG8 复制 toast：成功绿图标（#278422）非失败红（#CF4040）
#   S5 BUG10 新建文档：自定义排序下置顶（第一张卡片）
#   S6 BUG2 新建子文件夹：父级折叠态新建 → 父级展开 + 新节点命名态
#   S7 BUG5 新标签页打开：恒新开（标签数 +1）
#   S8 BUG6 面包屑：导航两次 → 后退回前视图 → 前进返回
# （BUG1 副本紧随原件 = cargo test duplicate_inserts_after_source 覆盖；
#   BUG4 打开方式对话框 = 真实系统对话框，探针不可覆盖）
# 用法：tools/.venv/Scripts/python.exe tools/cdp-bugfix-probe.py（dev server 未起会自起并自杀）
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9361
URL = "http://localhost:5373"
SLEEP = 0.35

# 桩：f1 角色（递归 4 = 直接 3 + 子夹 1；挂 PNG 分组）/ f1a 子夹 / f2 场景（空）
STUB = r"""
window.__calls = { reorder_placements: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 4, children: [
      { id: "f1a", name: "子夹", count: 1, children: [] },
    ], groups: [{ id: "g1", name: "PNG组", rules: [{ kind: "ext", values: ["png"] }] }] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mk = (assetId, name, ext, over = {}) => ({
  assetId, folderId: "f1", name, w: 800, h: 600, ext, thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 100, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T10:00:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
  ...over,
});
const PLACEMENTS = [
  mk("AN-1", "直一.png", "png"),
  mk("AN-2", "直二.png", "png"),
  mk("AN-3", "文档.md", "md", { w: null, h: null }),
  mk("AN-4", "子图.png", "png", { folderId: "f1a" }),
];
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    if (cmd === "create_document") {
      const folderId = args.targetFolder ?? "root";
      const p = mk("AN-NEW", "新建文档.txt", "txt", { folderId, w: null, h: null });
      PLACEMENTS.unshift(p);
      return Promise.resolve({ assetId: "AN-NEW", folderId });
    }
    if (cmd === "create_folder") {
      const id = "fnew-" + (args.parentId ?? "root");
      const node = { id, name: args.name, count: 0, children: [] };
      if (args.parentId) {
        const walk = (ns) => ns.forEach((n) => {
          if (n.id === args.parentId) n.children.push(node);
          walk(n.children ?? []);
        });
        walk(TREE.nodes);
      } else {
        TREE.nodes.push(node);
      }
      return Promise.resolve({ id });
    }
    if (cmd === "reorder_placements") {
      window.__calls.reorder_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 3 },
      get_ui_settings: { showSubfolders: false },
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
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
        if raw.get("method") == "Runtime.consoleAPICalled":
            args = raw.get("params", {}).get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            print(f"  [console.{raw['params'].get('type')}]", text[:200])
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw.get("params", {}).get("exceptionDetails", {})
            print("  [exception]", str(d.get("text", ""))[:200], str(d.get("exception", {}).get("description", ""))[:300])


def kill_listening_port(port):
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                subprocess.run(["taskkill", "/PID", line.split()[-1], "/F"], capture_output=True)
    except Exception:
        pass


async def main():
    fails = []
    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,1200", "--no-first-run", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        version = None
        for _ in range(30):
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
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, buttons, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons,
                    "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def click(x, y, button="left", clicks=1):
                b = 1 if button == "left" else 2
                await mouse("mousePressed", x, y, b, button, clicks)
                await mouse("mouseReleased", x, y, 0, button, clicks)
                await asyncio.sleep(SLEEP)

            async def dblclick(x, y):
                await click(x, y, clicks=2)

            async def key(k, code, vk, ctrl=False):
                mods = 2 if ctrl else 0
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": k, "code": code, "windowsVirtualKeyCode": vk, "modifiers": mods}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": k, "code": code, "windowsVirtualKeyCode": vk, "modifiers": mods}, session=sess)
                await asyncio.sleep(SLEEP)

            async def rect(sel):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('{sel}');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2), left: r.left, right: r.right, top: r.top, bottom: r.bottom }};
                  }})()
                """)

            async def click_sel(sel):
                pos = await rect(sel)
                if not pos:
                    return False
                await click(pos["x"], pos["y"])
                return True

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 等树渲染
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)

            # ── S1 BUG7 树数量：折叠 = 递归 / 展开 = 直接 ──
            print("S1 树数量（折叠递归 / 展开直接）")
            c_fold = await ev("document.querySelector('.tree-item[data-folder-id=\"f1\"] .count')?.textContent.trim() ?? null")
            check("S1a f1 折叠显示递归总数 4", c_fold == "4", str(c_fold))
            # 点箭头展开 f1（箭头按钮 .expand，单击 item 会导航——点 expand 不导航）
            await click_sel('.tree-item[data-folder-id="f1"] > .expand')
            await asyncio.sleep(0.4)
            c_open = await ev("document.querySelector('.tree-item[data-folder-id=\"f1\"] .count')?.textContent.trim() ?? null")
            c_sub = await ev("document.querySelector('.tree-item[data-folder-id=\"f1a\"] .count')?.textContent.trim() ?? null")
            check("S1b f1 展开显示直接子项数 3", c_open == "3", str(c_open))
            check("S1c f1a 显示递归总数 1", c_sub == "1", str(c_sub))

            # ── S2 BUG3 树定位：双击文件夹卡片 → 祖先链展开 + 节点选中 ──
            print("S2 双击文件夹卡片树定位")
            await click_sel('.tree-item[data-folder-id="f1"] > .expand')  # 再折叠 f1
            await asyncio.sleep(0.3)
            collapsed = await ev("document.querySelector('.tree-item[data-folder-id=\"f1a\"]') === null")
            check("S2a 预置：f1 折叠后 f1a 树节点不可见", collapsed is True, str(collapsed))
            await click_sel('.tree-item[data-folder-id="f1"] .body')  # 单击 f1 进视口
            await asyncio.sleep(0.5)
            pos = await rect('.folder-card[data-folder-id="f1a"]')
            check("S2b f1 视口出 f1a 文件夹卡片", pos is not None, "")
            if pos:
                await dblclick(pos["x"], pos["y"])
                await asyncio.sleep(0.8)
                sel = await ev("!!document.querySelector('.tree-item[data-folder-id=\"f1a\"].selected')")
                cnt = await ev("document.querySelector('.tree-item[data-folder-id=\"f1\"] .count')?.textContent.trim() ?? null")
                check("S2c 双击卡片后 f1 展开（f1a 节点选中可见）", sel is True, f"f1.count={cnt}")
                check("S2d f1 展开后数量 = 直接子项 3（BUG7 联动）", cnt == "3", str(cnt))

            # ── S3 BUG9 组内允许排序 / 跨组禁止 ──
            print("S3 组内拖拽排序")
            await click_sel('.tree-item[data-folder-id="f1"] .body')  # 回 f1 视口
            await asyncio.sleep(0.6)
            ncalls0 = await ev("window.__calls.reorder_placements.length")
            src = await rect('.asset-card[data-key="AN-1|f1"]')
            dst = await rect('.asset-card[data-key="AN-2|f1"]')
            check("S3a 同组两卡片可见", src is not None and dst is not None, f"{src} {dst}")
            if src and dst:
                # 拖 直一 → 直二 左缘间隙（同 PNG 组）
                await mouse("mousePressed", src["x"], src["y"], 1)
                await asyncio.sleep(0.1)
                tx = int(dst["left"]) + 2
                ty = dst["y"]
                for i in range(1, 6):
                    await mouse("mouseMoved", src["x"] + (tx - src["x"]) * i // 5, src["y"] + (ty - src["y"]) * i // 5, 1)
                    await asyncio.sleep(0.05)
                await mouse("mouseReleased", tx, ty, 0)
                await asyncio.sleep(0.6)
            ncalls1 = await ev("window.__calls.reorder_placements.length")
            check("S3b 同组拖拽 → reorder_placements 调用", ncalls1 == (ncalls0 or 0) + 1, f"{ncalls0} → {ncalls1}")
            # 跨组：直一（PNG组） → 文档.md（未分组）
            src = await rect('.asset-card[data-key="AN-1|f1"]')
            dst = await rect('.asset-card[data-key="AN-3|f1"]')
            if src and dst:
                await mouse("mousePressed", src["x"], src["y"], 1)
                await asyncio.sleep(0.1)
                tx = int(dst["left"]) + 2
                ty = dst["y"]
                for i in range(1, 6):
                    await mouse("mouseMoved", src["x"] + (tx - src["x"]) * i // 5, src["y"] + (ty - src["y"]) * i // 5, 1)
                    await asyncio.sleep(0.05)
                await mouse("mouseReleased", tx, ty, 0)
                await asyncio.sleep(0.6)
            ncalls2 = await ev("window.__calls.reorder_placements.length")
            check("S3c 跨组拖拽 → 无 reorder 调用", ncalls2 == ncalls1, f"{ncalls1} → {ncalls2}")

            # ── S4 BUG8 复制成功 toast 用成功图标 ──
            print("S4 复制 toast 图标")
            await click_sel('.asset-card[data-key="AN-1|f1"]')
            await key("c", "KeyC", 67, ctrl=True)
            await asyncio.sleep(0.5)
            # 多条 toast 并存（前面 reorder 的 5 秒未散）——按文案找复制那条
            tinfo = await ev("""
              (() => {
                const t = [...document.querySelectorAll('.toast')].find((el) => el.textContent.includes('已复制'));
                if (!t) return null;
                return { msg: t.querySelector('.message')?.textContent.trim() ?? '', svg: t.querySelector('.state-icon svg')?.outerHTML ?? '' };
              })()
            """)
            check("S4a 复制 toast 文案", tinfo is not None and "已复制" in tinfo.get("msg", ""), str(tinfo))
            check("S4b 成功绿图标（非失败红）", tinfo is not None and ("#278422" in tinfo.get("svg", "")) and ("#CF4040" not in tinfo.get("svg", "")), str(tinfo)[:120])

            # ── S5 BUG10 新建文档自定义排序下置顶 ──
            print("S5 新建文档置顶")
            await key("n", "KeyN", 78, ctrl=True)
            await asyncio.sleep(0.6)
            doc_item = await ev("""
              (() => {
                const items = [...document.querySelectorAll('.menu-item .label')];
                const it = items.find((el) => el.textContent.trim() === '文本文档');
                if (!it) return null;
                const r = it.closest('.menu-item').getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            check("S5a 新增菜单出「文本文档」", doc_item is not None, "")
            if doc_item:
                await click(doc_item["x"], doc_item["y"])
                await asyncio.sleep(0.8)
                first = await ev("""
                  (() => {
                    const c = document.querySelector('.asset-card');
                    if (!c) return null;
                    const inp = c.querySelector('.rename-input');
                    return inp ? inp.value : (c.querySelector('.title')?.textContent.trim() ?? null);
                  })()
                """)
                check("S5b 新建文档置顶（第一张卡片，命名态）", first is not None and str(first).startswith("新建文档"), str(first))

            # ── S6 BUG2 新建子文件夹默认展开父级 ──
            print("S6 新建子文件夹展开父级")
            await click_sel('.tree-item[data-folder-id="f1"] > .expand')  # 折叠 f1
            await asyncio.sleep(0.3)
            pos = await rect('.tree-item[data-folder-id="f1"] .body')
            if pos:
                await click(pos["x"], pos["y"], button="right")
                await asyncio.sleep(0.6)
            sub_item = await ev("""
              (() => {
                const items = [...document.querySelectorAll('.menu-item .label')];
                const it = items.find((el) => el.textContent.trim() === '新建子文件夹');
                if (!it) return null;
                const r = it.closest('.menu-item').getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            check("S6a 树右键出「新建子文件夹」", sub_item is not None, "")
            if sub_item:
                await click(sub_item["x"], sub_item["y"])
                await asyncio.sleep(0.9)
                naming = await ev("document.querySelector('.tree-item.naming .rename-input')?.value ?? null")
                check("S6b f1 展开 + 新节点命名态", naming == "新建文件夹", str(naming))

            # ── S7 BUG5 新标签页打开恒新开 ──
            print("S7 新标签页打开")
            ntabs0 = await ev("document.querySelectorAll('.dv-tab .tab').length")
            pos = await rect('.tree-item[data-folder-id="f2"] .body')
            if pos:
                await click(pos["x"], pos["y"], button="right")
                await asyncio.sleep(0.6)
            tab_item = await ev("""
              (() => {
                const items = [...document.querySelectorAll('.menu-item .label')];
                const it = items.find((el) => el.textContent.trim() === '新标签页打开');
                if (!it) return null;
                const r = it.closest('.menu-item').getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if tab_item:
                await click(tab_item["x"], tab_item["y"])
                await asyncio.sleep(0.8)
            ntabs1 = await ev("document.querySelectorAll('.dv-tab .tab').length")
            active_title = await ev("document.querySelector('.dv-tab .tab.selected .label')?.textContent.trim() ?? null")
            check("S7a 标签数 +1", ntabs1 == (ntabs0 or 0) + 1, f"{ntabs0} → {ntabs1}")
            check("S7b 新页激活且视口 = 场景", active_title == "场景", str(active_title))

            # ── S8 BUG6 面包屑后退 / 前进 ──
            print("S8 面包屑历史")
            await click_sel('.tree-item[data-folder-id="f1"] .body')  # 新页 f2 → f1
            await asyncio.sleep(0.6)
            await click_sel('.tree-item[data-folder-id="f2"] .body')  # f1 → f2
            await asyncio.sleep(0.6)
            # IconButton 用 v-tip 不落 title attribute；crumb-group 内按钮序 = 后退 / 前进（非档案视口）；
            # 多 panel 并存取可见（offsetParent 非 null）的那个工具栏
            nav_btns = """
              (() => {
                const bars = [...document.querySelectorAll('.panel-toolbar')].filter((el) => el.offsetParent !== null);
                const bar = bars[0];
                if (!bar) return [];
                return [...bar.querySelectorAll('.crumb-group .icon-btn')].map((b) => {
                  const r = b.getBoundingClientRect();
                  return { disabled: b.disabled, x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
                });
              })()
            """
            btns = await ev(nav_btns)
            check("S8a 导航两次后退钮可用", isinstance(btns, list) and len(btns) >= 2 and btns[0]["disabled"] is False, str(btns))
            if isinstance(btns, list) and len(btns) >= 2 and btns[0]["disabled"] is False:
                await click(btns[0]["x"], btns[0]["y"])
                await asyncio.sleep(0.6)
            active_title = await ev("document.querySelector('.dv-tab .tab.selected .label')?.textContent.trim() ?? null")
            check("S8b 后退回「角色」", active_title == "角色", str(active_title))
            btns = await ev(nav_btns)
            check("S8c 前进钮可用", isinstance(btns, list) and len(btns) >= 2 and btns[1]["disabled"] is False, str(btns))
            if isinstance(btns, list) and len(btns) >= 2 and btns[1]["disabled"] is False:
                await click(btns[1]["x"], btns[1]["y"])
                await asyncio.sleep(0.6)
            active_title = await ev("document.querySelector('.dv-tab .tab.selected .label')?.textContent.trim() ?? null")
            check("S8d 前进回「场景」", active_title == "场景", str(active_title))

            print()
            if fails:
                print(f"共 {len(fails)} 项失败：")
                for f in fails:
                    print("  -", f)
                return 1
            print("全部 PASS")
            return 0
    finally:
        edge.terminate()
        if spawned:
            spawned.terminate()
            kill_listening_port(5373)


sys.exit(asyncio.run(main()))
