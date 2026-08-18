# CDP 回归探针：3.3 自定义文件夹图标颜色（2026-08-18，同步语义二轮修订）
# 05 稿 §3 更改图标 / §4 更改颜色网格面板（CtxItem.panel + ContextMenu flyout 分流渲染）：
#   S1 右键 f1 → hover「更改图标」→ 面板结构：.icon-grid 20 格 / 宽 230 / 首格 cur / 无复选框
#   S2 点 animal 格 → set_folder_icon{f1, animal}（无 applyToChildren 参数）+ 菜单关闭 + svg 变化
#   S3 f1 点 scene → 默认影响子级：f1-1 / f1-1-1 跟随（同步语义：一致跟随，无复选框）
#   S3b f1-1 单独 animal → f1 换 weapon：f1-1 不跟随（已区分阻断）
#   S4 右键 f2 → hover「更改颜色」→ 面板结构：.color-grid 10 格 / 宽 235 / 无复选框 / 首格 cur
#   S5 点红色格 → set_folder_color{f2, red} + f2(root) --tint-bg = var(--custom-red)
#   S6 f1 设蓝色（末位格）→ f1(root) = custom-blue；f1-1/f1-1-1(child 继承) = custom-blue-bg；
#      缩进线无 --guide-color（2026-08-18 用户定：不变色，半透明线自然叠色）；f2 红不受影响
#   S6b 颜色同步（用户场景 2）：f1-1 单独 red → f1 改 green 不跟随；f1-1 改回 green 后
#      f1 改 red → f1-1 重新跟随（孙级视觉 red-bg）
#   S7 f2 颜色回默认（首格）：重开时 cells[1](红) cur → 点首格 → color:null → f2 无 --tint-bg
#   S8 区块管理行「默认图标」下拉 = 网格面板（openPanelMenu 整体面板菜单）
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-folder-icon-color-probe.py
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
PORT = 9356
URL = "http://localhost:5373"
SHOT = "tools/style-preview/folder-icon-panel.png"
SHOT2 = "tools/style-preview/folder-color-panel.png"
SHOT3 = "tools/style-preview/folder-tint-tree.png"
SLEEP = 0.3

# 桩：f1（子 f1-1，孙 f1-1-1）+ f2 两顶级；set_folder_icon / set_folder_color 真实改 TREE
# （icon applyToChildren 时递归整支），refresh 重拉 list_folder_tree（深拷贝防同引用）。
STUB = r"""
window.__calls = { set_folder_icon: [], set_folder_color: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 0, children: [
      { id: "f1-1", name: "主角", count: 0, children: [
        { id: "f1-1-1", name: "雪牧", count: 0, children: [] },
      ] },
    ] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const findNode = (nodes, id) => {
  for (const n of nodes) {
    if (n.id === id) return n;
    const h = findNode(n.children, id);
    if (h) return h;
  }
  return null;
};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 4 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: [],
      list_recycle: [],
      list_recent_assets: [],
      list_recent_libraries: [],
      list_inbox_snapshot: [],
      detect_doc_apps: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null,
      redo: null,
    };
    if (cmd === "set_folder_icon") {
      window.__calls.set_folder_icon.push({ folderId: args.folderId, icon: args.icon });
      const node = findNode(TREE.nodes, args.folderId);
      if (node) {
        // 同步语义（与后端一致）：后代图标 == 本节点旧图标 → 跟随写入并下钻；!= → 阻断
        const oldChain = node.icon ?? null;
        const stack = [node];
        while (stack.length) {
          const p = stack.pop();
          p.icon = args.icon;
          for (const c of p.children) {
            if ((c.icon ?? null) === oldChain) stack.push(c);
          }
        }
      }
      return Promise.resolve(null);
    }
    if (cmd === "set_folder_color") {
      window.__calls.set_folder_color.push({ folderId: args.folderId, color: args.color });
      const node = findNode(TREE.nodes, args.folderId);
      if (node) {
        // 同步语义（与后端一致）：无色后代穿透不写；显式 == 旧链路色 → 跟随写入；!= → 阻断
        //（桩节点均为 f1 后代、无更上祖先，oldChain = 本节点色即可，不模拟上溯）
        const oldChain = node.color ?? null;
        const apply = (n) => { if (args.color) n.color = args.color; else delete n.color; };
        apply(node);
        const stack = [node];
        while (stack.length) {
          const p = stack.pop();
          for (const c of p.children) {
            const v = c.color ?? null;
            if (v === null) stack.push(c);
            else if (v === oldChain) { apply(c); stack.push(c); }
          }
        }
      }
      return Promise.resolve(null);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
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

            async def item_rect(fid):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
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

            async def right_click(fid):
                c = await item_rect(fid)
                if not c:
                    return False
                await mouse("mousePressed", c["x"], c["y"], 2, button="right")
                await mouse("mouseReleased", c["x"], c["y"], 0, button="right")
                await asyncio.sleep(0.6)
                return bool(await ev("!!document.querySelector('.app-menu')"))

            async def hover_menu(text):
                c = await menu_item_center(text)
                if not c:
                    return False
                await mouse("mouseMoved", c["x"], c["y"], 0, button="none")
                await asyncio.sleep(0.6)
                return True

            async def close_menu():
                await send(ws, "Input.dispatchKeyEvent", {
                    "type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27,
                }, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {
                    "type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27,
                }, session=sess)
                await asyncio.sleep(0.4)
                return not await ev("!!document.querySelector('.app-menu')")

            async def cell_center(sel, idx):
                return await ev(f"""
                  (() => {{
                    const cells = document.querySelectorAll({json.dumps(sel)});
                    if (cells.length <= {idx}) return null;
                    const b = cells[{idx}].getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def panel_width(panel_sel):
                return await ev(f"""
                  (() => {{
                    const m = document.querySelector({json.dumps(panel_sel)})?.closest('.app-menu');
                    return m ? Math.round(m.getBoundingClientRect().width) : null;
                  }})()
                """)

            async def deco_svg(fid):
                return await ev(f"document.querySelector('.tree-item[data-folder-id=\"{fid}\"] .deco svg')?.outerHTML ?? null")

            async def tint_bg(fid):
                return await ev(f"document.querySelector('.tree-item[data-folder-id=\"{fid}\"]')?.style.getPropertyValue('--tint-bg') || null")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            n = 0
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            print("树 item 数:", n)
            if not n or n < 2:
                print("树 item 未渲染，探针无法执行")
                return 2

            # ── S1：更改图标面板结构 ──
            print("S1 右键 f1 → hover「更改图标」面板结构")
            check("S1 右键菜单出现", await right_click("f1"))
            check("S1 hover「更改图标」", await hover_menu("更改图标"))
            panel = await ev("!!document.querySelector('.app-menu .icon-grid')")
            check("S1 图标面板出现", bool(panel))
            cells = await ev("document.querySelectorAll('.app-menu .icon-grid .cell').length")
            check("S1 图标格 = 20（默认 + 19 自定义）", cells == 20, str(cells))
            w = await panel_width(".icon-grid")
            check("S1 面板宽 = 230", w == 230, str(w))
            cur = await ev("[...document.querySelectorAll('.app-menu .icon-grid .cell')].findIndex((c) => c.classList.contains('cur'))")
            check("S1 首格 cur（无自定义图标 = 默认格选中）", cur == 0, str(cur))
            cb = await ev("!!document.querySelector('.app-menu .icon-grid')?.closest('.app-menu')?.querySelector('.app-checkbox')")
            check("S1 无复选框（同步语义：默认影响子级）", not cb)
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT)

            # ── S2：点 animal 格 ──
            print("S2 点 animal 格 → 换图标 + 菜单关闭")
            svg_before = await deco_svg("f1")
            c = await cell_center(".app-menu .icon-grid .cell", 1)
            check("S2 animal 格可点", bool(c))
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            call = await ev("window.__calls.set_folder_icon.slice(-1)[0] ?? null")
            print("  调用:", json.dumps(call, ensure_ascii=False))
            check(
                "S2 set_folder_icon(f1, animal)（无 applyToChildren 参数）",
                bool(call) and call["folderId"] == "f1" and call["icon"] == "animal" and "applyToChildren" not in call,
                json.dumps(call, ensure_ascii=False),
            )
            check("S2 菜单已关闭", not await ev("!!document.querySelector('.app-menu')"))
            svg_after = await deco_svg("f1")
            check("S2 f1 图标已换（svg 变化）", bool(svg_before) and bool(svg_after) and svg_before != svg_after)

            # ── S3：默认传播（无复选框，子级一致即跟随）──
            print("S3 f1 点 scene 格 → 默认影响子级（f1-1/f1-1-1 跟随）")
            check("S3 右键菜单出现", await right_click("f1"))
            check("S3 hover「更改图标」", await hover_menu("更改图标"))
            c = await cell_center(".app-menu .icon-grid .cell", 14)  # scene
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            call = await ev("window.__calls.set_folder_icon.slice(-1)[0] ?? null")
            print("  调用:", json.dumps(call, ensure_ascii=False))
            check("S3 set_folder_icon(f1, scene)", bool(call) and call["icon"] == "scene", json.dumps(call, ensure_ascii=False))
            # 展开 f1 / f1-1 看子级、孙级跟随
            async def expand(fid):
                ex = await ev(f"""
                  (() => {{
                    const b = document.querySelector('.tree-item[data-folder-id="{fid}"] .expand');
                    if (!b) return null;
                    const r = b.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if ex:
                    await click_xy(ex["x"], ex["y"])
                    await asyncio.sleep(0.6)

            await expand("f1")
            svg_f1 = await deco_svg("f1")
            svg_sub = await deco_svg("f1-1")
            check("S3 f1-1 图标跟随（与 f1 同 svg）", bool(svg_f1) and svg_f1 == svg_sub)
            await expand("f1-1")
            svg_g = await deco_svg("f1-1-1")
            check("S3 f1-1-1 孙级跟随（与 f1 同 svg）", bool(svg_f1) and svg_f1 == svg_g)

            # ── S3b：子级单独区分后，父级再改不跟随 ──
            print("S3b f1-1 单独换 animal → f1 换 weapon，f1-1 不跟随")
            check("S3b 右键 f1-1 菜单出现", await right_click("f1-1"))
            check("S3b hover「更改图标」", await hover_menu("更改图标"))
            c = await cell_center(".app-menu .icon-grid .cell", 1)  # animal
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            check("S3b 右键 f1 菜单出现", await right_click("f1"))
            check("S3b hover「更改图标」", await hover_menu("更改图标"))
            c = await cell_center(".app-menu .icon-grid .cell", 19)  # weapon 末位
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            svg_f1 = await deco_svg("f1")
            svg_sub = await deco_svg("f1-1")
            svg_g = await deco_svg("f1-1-1")
            check("S3b f1 已换 weapon（svg 变化）", bool(svg_f1) and svg_f1 != svg_sub)
            check("S3b f1-1 保持 animal（已区分不跟随）", bool(svg_sub) and svg_sub == svg_g, "f1-1 与其孙级一致支不受影响")

            # ── S4：更改颜色面板结构 ──
            print("S4 右键 f2 → hover「更改颜色」面板结构")
            check("S4 右键菜单出现", await right_click("f2"))
            check("S4 hover「更改颜色」", await hover_menu("更改颜色"))
            panel = await ev("!!document.querySelector('.color-grid')")
            check("S4 颜色面板出现", bool(panel))
            cells = await ev("document.querySelectorAll('.color-grid .cell').length")
            check("S4 颜色格 = 10（默认 + 9 色）", cells == 10, str(cells))
            w = await panel_width(".color-grid")
            check("S4 面板宽 = 235", w == 235, str(w))
            cur = await ev("[...document.querySelectorAll('.color-grid .cell')].findIndex((c) => c.classList.contains('cur'))")
            check("S4 首格 cur（无自定义颜色 = 默认格选中）", cur == 0, str(cur))
            cb = await ev("!!document.querySelector('.color-grid')?.closest('.app-menu')?.querySelector('.app-checkbox')")
            check("S4 颜色面板无复选框（颜色仅写本节点）", not cb)
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT2, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT2)

            # ── S5：f2 点红色格 ──
            print("S5 f2 点红色格 → root 底 = custom-red")
            c = await cell_center(".color-grid .cell", 1)
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            call = await ev("window.__calls.set_folder_color.slice(-1)[0] ?? null")
            print("  调用:", json.dumps(call, ensure_ascii=False))
            check(
                "S5 set_folder_color(f2, red)",
                bool(call) and call["folderId"] == "f2" and call["color"] == "red",
                json.dumps(call, ensure_ascii=False),
            )
            tb = await tint_bg("f2")
            check("S5 f2 --tint-bg = var(--custom-red)", tb == "var(--custom-red)", str(tb))

            # ── S6：f1 设蓝色 → 子级底色版本跟随 + 缩进线变色 ──
            print("S6 f1 点蓝色格（末位）→ 子级 -bg 跟随 + 缩进线随色")
            check("S6 右键菜单出现", await right_click("f1"))
            check("S6 hover「更改颜色」", await hover_menu("更改颜色"))
            c = await cell_center(".color-grid .cell", 9)  # blue = 稿序末位
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            call = await ev("window.__calls.set_folder_color.slice(-1)[0] ?? null")
            check("S6 set_folder_color(f1, blue)", bool(call) and call["color"] == "blue", json.dumps(call, ensure_ascii=False))
            tb1 = await tint_bg("f1")
            tb11 = await tint_bg("f1-1")
            tb111 = await tint_bg("f1-1-1")
            gc = await ev("""
              (() => {
                const el = document.querySelector('.tree-item[data-folder-id="f1"]')?.closest('.ft-node')?.querySelector('.ft-children');
                return el ? el.style.getPropertyValue('--guide-color') || null : null;
              })()
            """)
            print("  f1:", tb1, " f1-1:", tb11, " f1-1-1:", tb111, " guide:", gc)
            check("S6 f1(root) --tint-bg = var(--custom-blue)", tb1 == "var(--custom-blue)", str(tb1))
            check("S6 f1-1(child 继承) --tint-bg = var(--custom-blue-bg)", tb11 == "var(--custom-blue-bg)", str(tb11))
            check("S6 f1-1-1(孙级继承) --tint-bg = var(--custom-blue-bg)", tb111 == "var(--custom-blue-bg)", str(tb111))
            check("S6 缩进线不变色（无 --guide-color，半透明线自然叠色）", not gc, str(gc))
            tb2 = await tint_bg("f2")
            check("S6 f2 红不受影响", tb2 == "var(--custom-red)", str(tb2))
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT3, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT3)

            # ── S6b：颜色同步——已区分不跟随 / 改回一致重新跟随（用户场景 2）──
            print("S6b f1-1 单独设 red → f1 改 green 不跟随；f1-1 改回一致后 f1 再改重新跟随")
            # 此时 f1 = blue（S6），f1-1 无色（视觉跟随 blue）
            check("S6b 右键 f1-1 菜单出现", await right_click("f1-1"))
            check("S6b hover「更改颜色」", await hover_menu("更改颜色"))
            c = await cell_center(".color-grid .cell", 1)  # red
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            tb11 = await tint_bg("f1-1")
            check("S6b f1-1 单独设 red", tb11 == "var(--custom-red-bg)", str(tb11))
            check("S6b 右键 f1 菜单出现", await right_click("f1"))
            check("S6b hover「更改颜色」", await hover_menu("更改颜色"))
            c = await cell_center(".color-grid .cell", 4)  # green
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            tb1 = await tint_bg("f1")
            tb11 = await tint_bg("f1-1")
            check("S6b f1 改 green", tb1 == "var(--custom-green)", str(tb1))
            check("S6b f1-1 已区分不跟随（保持 red）", tb11 == "var(--custom-red-bg)", str(tb11))
            # f1-1 改回与 f1 一致（green）→ f1 再改 red → f1-1 重新跟随
            check("S6b 右键 f1-1 菜单出现(2)", await right_click("f1-1"))
            check("S6b hover「更改颜色」(2)", await hover_menu("更改颜色"))
            c = await cell_center(".color-grid .cell", 4)  # green
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            check("S6b 右键 f1 菜单出现(2)", await right_click("f1"))
            check("S6b hover「更改颜色」(2)", await hover_menu("更改颜色"))
            c = await cell_center(".color-grid .cell", 1)  # red
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            tb1 = await tint_bg("f1")
            tb11 = await tint_bg("f1-1")
            tb111 = await tint_bg("f1-1-1")
            print("  f1:", tb1, " f1-1:", tb11, " f1-1-1:", tb111)
            check("S6b f1 改 red", tb1 == "var(--custom-red)", str(tb1))
            check("S6b f1-1 改回一致后重新跟随（red）", tb11 == "var(--custom-red-bg)", str(tb11))
            check("S6b f1-1-1 孙级视觉跟随（red-bg）", tb111 == "var(--custom-red-bg)", str(tb111))

            # ── S7：f2 颜色回默认 ──
            print("S7 f2 重开颜色面板 → 红格 cur → 点首格恢复默认")
            check("S7 右键菜单出现", await right_click("f2"))
            check("S7 hover「更改颜色」", await hover_menu("更改颜色"))
            cur = await ev("[...document.querySelectorAll('.color-grid .cell')].findIndex((c) => c.classList.contains('cur'))")
            check("S7 红格 cur（当前色回显）", cur == 1, str(cur))
            c = await cell_center(".color-grid .cell", 0)
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.8)
            call = await ev("window.__calls.set_folder_color.slice(-1)[0] ?? null")
            check("S7 set_folder_color(f2, null)", bool(call) and call["color"] is None, json.dumps(call, ensure_ascii=False))
            tb = await tint_bg("f2")
            check("S7 f2 恢复默认（无 --tint-bg）", tb is None, str(tb))

            # ── S8：区块管理行「默认图标」下拉 = 网格面板（openPanelMenu，2026-08-18 换血）──
            print("S8 区块管理弹窗 → 图标下拉 → 网格面板选择")
            await ev("document.querySelector('.btn-group > .icon-btn')?.dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)
            modal = await ev("!!document.querySelector('.block-manage')")
            check("S8 区块管理弹窗打开", bool(modal))
            pick = await ev("""
              (() => {
                const p = document.querySelector('.block-manage .bm-row .icon-pick');
                if (!p) return null;
                const b = p.getBoundingClientRect();
                return { x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) };
              })()
            """)
            svg_pick_before = await ev("document.querySelector('.block-manage .bm-row .icon-pick .app-icon')?.innerHTML ?? null")
            if pick:
                await click_xy(pick["x"], pick["y"])
                await asyncio.sleep(0.6)
            grid = await ev("!!document.querySelector('.app-menu .icon-grid')")
            check("S8 下拉弹出 = 图标网格面板", bool(grid))
            cells = await ev("document.querySelectorAll('.app-menu .icon-grid .cell').length")
            check("S8 网格 20 格", cells == 20, str(cells))
            w = await panel_width(".icon-grid")
            check("S8 面板宽 = 230", w == 230, str(w))
            c = await cell_center(".app-menu .icon-grid .cell", 19)  # weapon 末位
            if c:
                await click_xy(c["x"], c["y"])
                await asyncio.sleep(0.5)
            check("S8 选中后菜单关闭", not await ev("!!document.querySelector('.app-menu')"))
            svg_pick_after = await ev("document.querySelector('.block-manage .bm-row .icon-pick .app-icon')?.innerHTML ?? null")
            check(
                "S8 草稿图标已写（icon-pick svg 变化）",
                svg_pick_before is not None and svg_pick_after is not None and svg_pick_before != svg_pick_after,
            )

            print()
            if fails:
                print("FAIL 汇总:")
                for f_ in fails:
                    print(" -", f_)
                return 1
            print("ALL PASS")
            return 0
    finally:
        proc.terminate()
        if dev_proc is not None:
            subprocess.run(
                ["taskkill", "/PID", str(dev_proc.pid), "/T", "/F"],
                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            )
            print("已杀掉探针自起的 dev server")


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
