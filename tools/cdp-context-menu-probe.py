# CDP 实测：右键菜单新稿（2026-09-01，组件/上下文菜单 五件 + 用户容器定案；二十五轮口径更新）
# A 容器/分组/item：宽度内容自适应（无内联 width，最长 item 决定）/ 名称→36px→右件最小间距 /
#   右件贴右缘 / 无搜索区 / 无左图标 / 组间 1px 五级线（组盒内底 inset）/ item 26 高 /
#   快捷键原串纯文本 / 展开箭头；
# B hover 排序依据 → flyout 宽自适应（< 250，废 300）+ checkable 对勾（自定义选中恒显）+ 对勾贴右缘；
# E hover 更改图标 → 网格面板 31 格（默认 + 30 新双色）/ 宽 186（5×30+4×6+pad 12）/ gap 6 / 双色原值；
# C 更改颜色内嵌色块行：10 色块 / 默认位选中 / 点色不关菜单 + set_folder_color 调用 + 外环跟随回写；
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
window.__calls = { set_folder_color: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 0, children: [
      { id: "f1-1", name: "主角", count: 0, children: [] },
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
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    if (cmd === "set_folder_color") {
      window.__calls.set_folder_color.push(JSON.parse(JSON.stringify(args ?? {})));
      const n = findNode(TREE.nodes, args.folderId);
      if (n) n.color = args.color;
      return Promise.resolve(null);
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

            async def mouse(t, x, y, button="left"):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "buttons": 2 if button == "right" else (0 if t == "mouseMoved" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

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
            check("E2 网格 31 格（默认 + 30 新双色）", await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.icon-grid .cell').length === 31"), str(await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.icon-grid .cell').length")))
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
            check("C3 set_folder_color 调用 green", await ev("window.__calls.set_folder_color.length === 1 && window.__calls.set_folder_color[0].folderId === 'f1' && window.__calls.set_folder_color[0].color === 'green'"))
            check("C4 菜单不关", await ev("document.querySelector('.app-menu') != null"))
            check("C5 外环跟随到绿块", await ev("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色')?.querySelectorAll('.sw')[5].classList.contains('cur')"))
            await shot("ctx-menu-swatches")
            # 回点默认位清色
            sw0 = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色').querySelectorAll('.sw')[0]; const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mousePressed", sw0["x"], sw0["y"]); await mouse("mouseReleased", sw0["x"], sw0["y"])
            await asyncio.sleep(0.5)
            check("C6 清色调用 null", await ev("window.__calls.set_folder_color.length === 2 && window.__calls.set_folder_color[1].color === null"))

            # Esc 关菜单
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.4)
            check("C7 Esc 关菜单", await ev("!document.querySelector('.app-menu')"))

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
