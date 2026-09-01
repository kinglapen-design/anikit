# CDP 实测：右键菜单新稿（2026-09-01，组件/上下文菜单 五件 + 用户容器定案；二十五轮口径更新；同日走查修订）
# A 容器/分组/item：宽度内容自适应（无内联 width，最长 item 决定）/ 名称→36px→右件最小间距 /
#   右件贴右缘 / 无搜索区 / 无左图标 / 组间 1px 五级线（组盒内底 inset）/ item 26 高 /
#   快捷键原串纯文本 / 展开箭头；
# B hover 排序依据 → flyout 宽自适应（< 250，废 300）+ checkable 对勾（自定义选中恒显）+ 对勾贴右缘；
# E hover 更改图标 → 网格面板 30 格（默认 + 29 双色；archive 档案专属出清单）/ 宽 186 / gap 6 / 双色原值；
# C 更改颜色内嵌色块行：10 色块 / 默认位选中 / 点色不关菜单 + set_folder_colors 调用 + 外环跟随回写 /
#   「默认」= "default" 显式断链哨兵（子级能退回常态灰）；
# F 显式默认断链端到端（父绿子选默认 → 子灰父仍绿）；G 多选批量换色；H 档案节点菜单接入更改颜色；
# D 截图 menu / flyout / swatches + 设计系统预览页 menus 展区。
import asyncio
import base64
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9347
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { set_folder_colors: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 0, children: [
      { id: "f1-1", name: "主角", count: 0, children: [] },
    ] },
    { id: "f2", name: "场景", count: 0, children: [] },
    { id: "a1", name: "主角档案", kind: "archive", count: 0, children: [] },
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
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    if (cmd === "set_folder_colors") {
      window.__calls.set_folder_colors.push(JSON.parse(JSON.stringify(args ?? {})));
      for (const id of args.folderIds ?? []) { const n = findNode(TREE.nodes, id); if (n) n.color = args.color; }
      return Promise.resolve((args.folderIds ?? []).length);
    }
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 1, folderCount: 3 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: [], list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [], list_space_libraries: ["测试库"],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
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
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])


async def main():
    # 源码静态断言素材：CUSTOM_ICONS 清单体是否含 archive（E2b 用——档案专属不可选）
    icons_src = open("src/lib/customIcons.ts", encoding="utf-8").read()
    icons_list_body = icons_src.split("CUSTOM_ICONS: CustomIconDef[] = [", 1)[1].split("];", 1)[0]
    global ICONS_SRC_HAS_ARCHIVE
    ICONS_SRC_HAS_ARCHIVE = 'name: "archive"' in icons_list_body
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    fails = []

    def check(name, cond, detail=""):
        print(("PASS " if cond else "FAIL ") + name + (f" | {detail}" if detail else ""))
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

            async def mouse(t, x, y, button="left", modifiers=0):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "modifiers": modifiers, "buttons": 2 if button == "right" else (0 if t == "mouseMoved" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            # 前置：展开 f1（子级 f1-1 可见——C3b/F/G3 断言前提）
            expR = await ev("(() => { const el = document.querySelector('[data-folder-id=\"f1\"] .exp'); if (!el) return null; const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            if expR:
                await mouse("mousePressed", expR["x"], expR["y"]); await mouse("mouseReleased", expR["x"], expR["y"])
                await asyncio.sleep(0.5)
            check("A0b f1 展开（f1-1 可见）", await ev("document.querySelector('[data-folder-id=\"f1-1\"]') != null"))

            # —— A. 树节点右键 f1「角色」——
            rect = await ev("(() => { const el = document.querySelector('[data-folder-id=\"f1\"]'); if (!el) return null; const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            if not rect:
                check("A0 树节点存在", False)
                return 1
            await mouse("mousePressed", rect["x"], rect["y"], "right"); await mouse("mouseReleased", rect["x"], rect["y"], "right")
            await asyncio.sleep(0.6)

            check("A1 容器宽内容自适应（无内联 width）", await ev("document.querySelector('.app-menu')?.style.width === ''"))
            check("A2 无搜索区", await ev("!document.querySelector('.menu-search')"))
            check("A3 分组数 >= 8", await ev("document.querySelectorAll('.app-menu .menu-group').length >= 8"), str(await ev("document.querySelectorAll('.app-menu .menu-group').length")))
            check("A4 组间线 = 组盒内底 inset 五级", await ev("getComputedStyle(document.querySelector('.app-menu .menu-group')).boxShadow.includes('inset')"))
            check("A5 item 高 26", await ev("document.querySelector('.app-menu .menu-item')?.offsetHeight === 26"))
            check("A6 item 无左图标", await ev("[...document.querySelectorAll('.app-menu .menu-item .main .app-icon')].length === 0"))
            check("A7 快捷键原串大写", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '新标签页打开')?.querySelector('.shortcut')?.textContent === 'Ctrl+Enter'"))
            check("A8 排序依据带箭头", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '排序依据')?.querySelector('.arrow svg') != null"))
            check("A9 禁用项存在（粘贴文件夹）", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '粘贴文件夹')?.disabled === true"))
            check("A10 名称→右件最小间距 36（更改颜色行 label→色块组）", await ev("(() => { const it = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色'); if (!it) return false; const l = it.querySelector('.label').getBoundingClientRect(); const s = it.querySelector('.swatches').getBoundingClientRect(); return Math.abs(s.left - l.right - 36) <= 1.5; })()"))
            check("A11 容器宽=最长项（短项右件贴右缘：容器右缘-14）", await ev("(() => { const m = document.querySelector('.app-menu'); const it = [...m.querySelectorAll('.menu-item')].find((x) => x.querySelector('.label')?.textContent === '打开文件夹'); if (!it) return false; const s = it.querySelector('.shortcut').getBoundingClientRect(); return Math.abs(m.getBoundingClientRect().right - 14 - s.right) <= 1.5; })()"))
            await shot("ctx-menu")

            # —— B. hover 排序依据 → flyout ——
            sortRect = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '排序依据'); const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mouseMoved", sortRect["x"], sortRect["y"])
            await asyncio.sleep(0.5)
            check("B1 flyout 出且宽内容自适应（无内联 width）", await ev("document.querySelectorAll('.app-menu').length === 2 && document.querySelectorAll('.app-menu')[1].style.width === ''"))
            check("B2 flyout 宽 < 250（废 300 定宽）", await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth < 250"), str(await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth")))
            check("B3 flyout 五个对勾项", await ev("document.querySelectorAll('.app-menu')[1].querySelectorAll('.check-slot').length === 5"))
            check("B4 自定义选中（checked 恒显一级）", await ev("document.querySelectorAll('.app-menu')[1].querySelector('.menu-item.checked .check-slot') != null"))
            check("B5 flyout 对勾贴右缘（容器右缘-14）", await ev("(() => { const m = document.querySelectorAll('.app-menu')[1]; const c = m.querySelector('.menu-item .check-slot'); if (!c) return false; return Math.abs(m.getBoundingClientRect().right - 14 - c.getBoundingClientRect().right) <= 1.5; })()"))
            await shot("ctx-menu-flyout")

            # —— E. hover 更改图标 → 网格面板（31 格新双色 / 宽 186 / gap 6）——
            iconRect = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改图标'); const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mouseMoved", iconRect["x"], iconRect["y"])
            await asyncio.sleep(0.5)
            check("E1 图标面板出且宽 186（5×30+4×6+pad 12）", await ev("(() => { const ms = document.querySelectorAll('.app-menu'); if (ms.length !== 2 || !ms[1].querySelector('.icon-grid')) return false; return Math.abs(ms[1].offsetWidth - 186) <= 1; })()"), str(await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth")))
            check("E2 网格 30 格（默认 + 29 双色，archive 档案专属出清单）", await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.icon-grid .cell').length === 30"), str(await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.icon-grid .cell').length")))
            check("E2b CUSTOM_ICONS 清单无 archive（源码静态断言）", not ICONS_SRC_HAS_ARCHIVE)
            check("E3 网格 gap 6 行列统一", await ev("(() => { const g = document.querySelectorAll('.app-menu')[1]?.querySelector('.icon-grid'); if (!g) return false; const cs = getComputedStyle(g); return cs.columnGap === '6px' && cs.rowGap === '6px'; })()"))
            check("E4 首格双色原值（fill #BFBFBF 亮层）", await ev("document.querySelectorAll('.app-menu')[1]?.querySelector('.icon-grid .cell svg path[fill=\"#BFBFBF\"]') != null"))
            await shot("ctx-menu-iconpanel")
            # 移开（回第一组项）收 flyout
            firstRect = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '打开文件夹'); const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mouseMoved", firstRect["x"], firstRect["y"])
            await asyncio.sleep(0.4)
            check("B6 移开后 flyout 收", await ev("document.querySelectorAll('.app-menu').length === 1"))

            # —— C. 更改颜色色块行 ——
            check("C1 色块行 10 块", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色')?.querySelectorAll('.sw').length === 10"))
            check("C2 默认位选中（文件夹无色）", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色')?.querySelectorAll('.sw')[0].classList.contains('cur')"))
            # 点绿块（序：默认0 红1 蓝2 黄3 黄绿4 绿5）
            swRect = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色').querySelectorAll('.sw')[5]; const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mousePressed", swRect["x"], swRect["y"]); await mouse("mouseReleased", swRect["x"], swRect["y"])
            await asyncio.sleep(0.5)
            check("C3 set_folder_colors 调用 green（单选 = 单元素数组）", await ev("window.__calls.set_folder_colors.length === 1 && JSON.stringify(window.__calls.set_folder_colors[0].folderIds) === '[\"f1\"]' && window.__calls.set_folder_colors[0].color === 'green'"))
            check("C3b 无色子级视觉跟随父色（f1-1 图标 tint=green）", await ev("document.querySelector('[data-folder-id=\"f1-1\"] .tt-icon[data-tint=\"green\"]') != null"))
            check("C4 菜单不关", await ev("document.querySelector('.app-menu') != null"))
            check("C5 外环跟随到绿块", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色')?.querySelectorAll('.sw')[5].classList.contains('cur')"))
            await shot("ctx-menu-swatches")
            # 回点默认位清色
            sw0 = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色').querySelectorAll('.sw')[0]; const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mousePressed", sw0["x"], sw0["y"]); await mouse("mouseReleased", sw0["x"], sw0["y"])
            await asyncio.sleep(0.5)
            check("C6 默认位 = default 显式断链哨兵", await ev("window.__calls.set_folder_colors.length === 2 && window.__calls.set_folder_colors[1].color === 'default'"))

            # Esc 关菜单
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.4)
            check("C7 Esc 关菜单", await ev("!document.querySelector('.app-menu')"))

            async def esc_close():
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await asyncio.sleep(0.4)

            async def rclick(folder_id):
                r = await ev(f"(() => {{ const el = document.querySelector('[data-folder-id=\"{folder_id}\"]'); if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                await asyncio.sleep(0.6)

            async def lclick(folder_id, modifiers=0):
                r = await ev(f"(() => {{ const el = document.querySelector('[data-folder-id=\"{folder_id}\"]'); if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                await mouse("mousePressed", r["x"], r["y"], "left", modifiers); await mouse("mouseReleased", r["x"], r["y"], "left", modifiers)
                await asyncio.sleep(0.4)

            async def pick_sw(index):
                r = await ev(f"(() => {{ const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色').querySelectorAll('.sw')[{index}]; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)

            # —— F. 显式默认断链（issue 1 端到端）：父绿 → 子选「默认」→ 子回常态灰、父仍绿 ——
            await rclick("f1")  # f1 设绿
            await pick_sw(5)
            await esc_close()
            check("F1 子级视觉跟随父色（f1-1 tint=green）", await ev("document.querySelector('[data-folder-id=\"f1-1\"] .tt-icon[data-tint=\"green\"]') != null"))
            await rclick("f1-1")  # 子级菜单：自身无色 → 默认位外环
            check("F2 子级菜单默认位外环（自身无色）", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色')?.querySelectorAll('.sw')[0].classList.contains('cur')"))
            await pick_sw(0)  # 子选「默认」
            check("F3 子级选默认 → 发 default 哨兵", await ev("(() => { const c = window.__calls.set_folder_colors; return c.length === 4 && JSON.stringify(c[3].folderIds) === '[\"f1-1\"]' && c[3].color === 'default'; })()"))
            await esc_close()
            check("F4 子级退回常态灰（无 data-tint）", await ev("(() => { const el = document.querySelector('[data-folder-id=\"f1-1\"] .tt-icon'); return el && !el.getAttribute('data-tint'); })()"))
            check("F5 父级仍绿（断链不回串）", await ev("document.querySelector('[data-folder-id=\"f1\"] .tt-icon[data-tint=\"green\"]') != null"))

            # —— G. 多选批量换色（issue 2）：f1 + f2 选中集 → 一次调用两 id ——
            await lclick("f1")  # plain click 单选入域
            await lclick("f2", modifiers=2)  # Ctrl 加选
            await rclick("f1")  # f1 在选中域内 → 菜单作用于整个选中集
            await pick_sw(2)  # 蓝
            check("G1 批量调用 = 选中集两 id", await ev("(() => { const c = window.__calls.set_folder_colors; const ids = c[c.length - 1].folderIds ?? []; return ids.length === 2 && ids.includes('f1') && ids.includes('f2'); })()"), str(await ev("JSON.stringify(window.__calls.set_folder_colors.at(-1).folderIds)")))
            await esc_close()
            check("G2 两项同变色（f1/f2 tint=blue）", await ev("document.querySelector('[data-folder-id=\"f1\"] .tt-icon[data-tint=\"blue\"]') != null && document.querySelector('[data-folder-id=\"f2\"] .tt-icon[data-tint=\"blue\"]') != null"))
            check("G3 断链子级不被批量回串（f1-1 仍无 tint）", await ev("(() => { const el = document.querySelector('[data-folder-id=\"f1-1\"] .tt-icon'); return el && !el.getAttribute('data-tint'); })()"))

            # —— H. 档案节点菜单接入更改颜色（issue 3）——
            await rclick("a1")
            check("H1 档案菜单有更改颜色色块行（10 块未禁用）", await ev("(() => { const it = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色'); return it && !it.disabled && it.querySelectorAll('.sw').length === 10; })()"))
            await pick_sw(1)  # 红
            check("H2 档案换色调用", await ev("(() => { const c = window.__calls.set_folder_colors; return JSON.stringify(c[c.length - 1].folderIds) === '[\"a1\"]' && c[c.length - 1].color === 'red'; })()"))
            await esc_close()
            check("H3 档案圆点变色（a1 dot tint=red）", await ev("document.querySelector('[data-folder-id=\"a1\"] .dot[data-tint=\"red\"]') != null"))

            # —— D. 设计系统预览页 menus 展区（hash 导航不重跑 main.ts，须 reload 触发预览页挂载判定）——
            await send(ws, "Page.navigate", {"url": URL + "/#design-system-menus"}, session=sess)
            await asyncio.sleep(1)
            await send(ws, "Page.reload", session=sess)
            await asyncio.sleep(5)
            await send(ws, "Runtime.evaluate", {"expression": "document.querySelector('#design-system-menus')?.scrollIntoView()"}, session=sess)
            await asyncio.sleep(0.6)
            check("D1 预览页菜单展区存在", await ev("document.querySelectorAll('#design-system-menus .pv-menu-shell').length >= 6"), str(await ev("document.querySelectorAll('#design-system-menus .pv-menu-shell').length")))
            check("D2 预览色块行 cur 外环", await ev("getComputedStyle(document.querySelector('#design-system-menus .sw.cur')).boxShadow.includes('0px 0px 0px 1.5px')"))
            await shot("ctx-menu-preview")
            return 1 if fails else 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
