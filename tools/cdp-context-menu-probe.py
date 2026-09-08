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
# S 段（2026-09-05 走查「上下文菜单」节修订）：S1 文件夹粘贴条件出现（A9 同步改口径）/
#   S2 文件卡（提示词骨架·缩略图重排·设为原始条件出现·导出删提示词·复制数据加标签描述）/
#   S3 复制→粘贴标签合并追加（set_asset_props 实证）/ S4 linkList 二级与徽章弹窗同构 /
#   S5 档案删项五条+克隆+导出档案文案+提示词骨架 / S6 空白视口分流（2026-09-09 收窄修订：
#   全部降只扫描——不是位置；未分类 = 候车区总入口留导入/新建文档）/
#   S7 回收站全量 19 项+占位四项禁用 / S8 混选删除两项分流 /
#   S9 虚拟视图文件卡收窄（2026-09-09 入口收窄：全部/最近使用/缩略图不是位置——位置级与
#   落点类不出现、占位项不列，留资产级/所见即所得；未分类位置视口全量对照 + 禁拖出）；
# D3-D5 带图标陈列与占位对齐。
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
window.__calls = { set_folder_colors: [], set_asset_props: [], set_archive_props: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 2, children: [
      { id: "f1-1", name: "主角", count: 0, children: [] },
    ] },
    { id: "f2", name: "场景", count: 1, children: [] },
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
// 2026-09-05 走查轮 S 段桩：A1 = 单位置文件（f1，tags 走路/跑步）；B2 = 双位置（f1 origin + f2 链接，tags 待机）；
// R1 = 回收站 fla 条目（folderId 伪值 "recycle"）；a1 档案 props [主角, NPC]
const mkP = (assetId, folderId, name, ext, origin) => ({
  assetId, folderId, name, w: 100, h: 100, thumbW: null, thumbH: null, ext,
  thumbPath: null, thumbKind: null, origin, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, size: 1234, createdAt: null,
  modifiedAt: "2026-09-05T00:00:00Z", importedAt: "2026-09-05T00:00:00Z",
  noteCount: null, thumbFrame: null, fps: null, hidden: false, playAssetId: null,
});
const PLACEMENTS = [
  mkP("A1", "f1", "走路.png", "png", true),
  mkP("B2", "f1", "雪牧.png", "png", true),
  mkP("B2", "f2", "雪牧-链接.png", "png", false),
  // 2026-09-09 入口收窄：未分类视口卡（C3|root——位置视口全量菜单 / 粘贴落 root 断言前提）
  mkP("C3", "root", "散落笔记.txt", "txt", true),
];
const RECYCLE = [mkP("R1", "recycle", "废稿.fla", "fla", false)];
const ARCHIVES = [
  { folderId: "a1", badge: null, avatar: null, props: ["主角", "NPC"],
    descriptions: [{ title: "设定", content: "雪国主角" }], links: [], createdAt: "", totalSize: 0, coverPath: null },
];
const DETAILS = {
  A1: { tags: ["走路", "跑步"], descriptions: [{ title: "动作", content: "基础动作" }] },
  B2: { tags: ["待机"], descriptions: [] },
  R1: { tags: ["废案"], descriptions: [] },
};
window.__cbId = 0; window.__cbs = {};
// contextmenu 派发记录（capture 全量，2026-09-09 S9-7 偶发不开菜单诊断：连 capture 都没有 = 事件被吞）
window.__ctxLog = [];
window.addEventListener('contextmenu', (e) => {
  const t = e.target;
  window.__ctxLog.push(((t.className && t.className.toString()) || t.tagName || '?').slice(0, 90));
  if (window.__ctxLog.length > 30) window.__ctxLog.shift();
}, true);
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
    if (cmd === "set_asset_props") {
      window.__calls.set_asset_props.push(JSON.parse(JSON.stringify(args ?? {})));
      if (DETAILS[args.assetId]) DETAILS[args.assetId].tags = args.props;
      return Promise.resolve(null);
    }
    if (cmd === "set_archive_props") {
      window.__calls.set_archive_props.push(JSON.parse(JSON.stringify(args ?? {})));
      const a = ARCHIVES.find((x) => x.folderId === args.folderId); if (a) a.props = args.props;
      return Promise.resolve(null);
    }
    if (cmd === "set_auto_import") {
      // 探针环境启动链路不填 currentSpace（autoImport 恒 false）——点「自动导入」先弹确认窗；
      // 桩直接批准开启（S6-5 自动导入视口分流断言前提）
      return Promise.resolve({ name: "S", path: "E:/T/S", autoImport: args?.enabled ?? true });
    }
    if (cmd === "get_asset_detail") {
      const d = DETAILS[args.assetId] ?? { tags: [], descriptions: [] };
      return Promise.resolve(JSON.parse(JSON.stringify({
        assetId: args.assetId, originName: "o.png", ext: "png", size: 1, width: 1, height: 1,
        duration: null, importedAt: "", modifiedAt: "", tags: d.tags, descriptions: d.descriptions,
        colors: [], links: [], note: "", placements: [], archives: [],
      })));
    }
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 3 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: JSON.parse(JSON.stringify(RECYCLE)),
      list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [],
      list_archives: JSON.parse(JSON.stringify(ARCHIVES)),
      list_space_libraries: ["测试库"],
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = raw["params"]["args"]
            print("[CONSOLE-ERR]", str([a.get("value", a.get("description", "?")) for a in args])[:300])


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
                # buttons = 事件后的按键状态：release/move 恒 0（旧写法 release 带 1 = 左键"假按住"，
                # 后续右键 press 被 Chromium 当组合键吃不发 contextmenu——2026-09-05 S3 段随机失败根因）
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "modifiers": modifiers, "buttons": 0 if t in ("mouseReleased", "mouseMoved") else (2 if button == "right" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

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
            check("A9 无剪切时「粘贴文件夹」不出现（2026-09-05 走查改条件出现，旧 disabled 断言废除）", await ev("[...document.querySelectorAll('.app-menu .menu-item')].every((x) => x.querySelector('.label')?.textContent !== '粘贴文件夹')"))
            check("A10 名称→右件最小间距 36（更改颜色行 label→色块组）", await ev("(() => { const it = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '更改颜色'); if (!it) return false; const l = it.querySelector('.label').getBoundingClientRect(); const s = it.querySelector('.swatches').getBoundingClientRect(); return Math.abs(s.left - l.right - 36) <= 1.5; })()"))
            check("A11 容器宽=最长项（短项右件贴右缘：容器右缘-14）", await ev("(() => { const m = document.querySelector('.app-menu'); const it = [...m.querySelectorAll('.menu-item')].find((x) => x.querySelector('.label')?.textContent === '打开文件夹'); if (!it) return false; const s = it.querySelector('.shortcut').getBoundingClientRect(); return Math.abs(m.getBoundingClientRect().right - 14 - s.right) <= 1.5; })()"))
            await shot("ctx-menu")

            # —— B. hover 排序依据 → flyout ——
            sortRect = await ev("(() => { const el = [...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.querySelector('.label')?.textContent === '排序依据'); const r = el.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; })()")
            await mouse("mouseMoved", sortRect["x"], sortRect["y"])
            await asyncio.sleep(0.5)
            check("B1 flyout 出且宽内容自适应（无内联 width）", await ev("document.querySelectorAll('.app-menu').length === 2 && document.querySelectorAll('.app-menu')[1].style.width === ''"))
            check("B2 flyout 宽 < 250（废 300 定宽）", await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth < 250"), str(await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth")))
            check("B3 flyout 五个对勾项", await ev("document.querySelectorAll('.app-menu')[1].querySelectorAll('.mark-slot').length === 5"))
            check("B4 自定义选中（checked 恒显一级）", await ev("document.querySelectorAll('.app-menu')[1].querySelector('.menu-item.checked .mark-slot') != null"))
            # 对勾 2026-09-05 走查移到左侧（对勾稿本意，与快捷键可同存）：贴左缘（容器 pad 6 + item padding 8 = 14）
            check("B5 flyout 对勾贴左缘（容器左缘+14）", await ev("(() => { const m = document.querySelectorAll('.app-menu')[1]; const c = m.querySelector('.menu-item .mark-slot'); if (!c) return false; return Math.abs(c.getBoundingClientRect().left - m.getBoundingClientRect().left - 14) <= 1.5; })()"))
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

            # —— S. 2026-09-05 走查「上下文菜单」节修订 ——
            # S1 文件夹粘贴条件出现 / S2 文件卡（提示词骨架·缩略图重排·设为原始条件·导出删项·复制数据）/
            # S3 复制→粘贴标签合并追加 / S4 linkList 二级（与徽章弹窗同构）/ S5 档案删项+克隆+提示词骨架 /
            # S6 空白视口分流 / S7 回收站全量 / S8 混选删除两项分流
            async def js_click(expr):
                r = await ev(expr)
                if r is None:
                    return None
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def js_hover(expr):
                r = await ev(expr)
                if r is None:
                    return None
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def js_rclick_expr(expr):
                r = await ev(expr)
                if r is None:
                    print("DIAG rclick MISS（元素取不到坐标）:", expr[:100])
                    return None
                # 先移入目标再按（真人路径）——上一轮菜单项原位关菜单后指针悬停卡片会触发
                # 悬停大图浮层，不移动直接 press 时 contextmenu 被浮层吃掉（2026-09-09 S9-7 高发实证）
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.25)
                await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                await asyncio.sleep(0.6)
                # CDP 合成右键偶发不开菜单两道防线（2026-09-09 S9-7 实证，__ctxLog 抓到真凶）：
                # ① toast 挡命中（§25 坑①：headless 计时节流 toast leave 动画卡死永不移除，
                #    contextmenu 打在 toast 上到不了卡片）——点 .close 无效（leave-active 卡 DOM），
                #    必须直接 remove；② 合成右键偶发不派发（§25 坑②）——原位重试一次
                if not await ev("!!document.querySelector('.app-menu')"):
                    await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
                    await asyncio.sleep(0.3)
                    await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                    await asyncio.sleep(0.6)
                return r

            def js_find(label, idx=0):
                return f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item') ?? []].find((x) => x.querySelector('.label')?.textContent === '{label}')"

            def el_center(sel_expr):
                return f"(() => {{ const el = {sel_expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()"

            def item_center(label, idx=0):
                return el_center(js_find(label, idx))

            async def menu_labels(idx=0):
                return await ev(f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)") or []

            async def click_quick(label):
                await js_click(el_center(f"[...document.querySelectorAll('.block.quick .tree-item')].find((x) => x.textContent.includes('{label}'))"))
                await asyncio.sleep(0.6)

            def blank_bottom():
                # 落点避开底部居中搜索悬浮栏（底 12 + 高 42 = 底部 54 区间；25 稿悬浮栏）——取右下偏上
                return "(() => { const el = document.querySelector('.grid-viewport'); if (!el) return null; const b = el.getBoundingClientRect(); return { x: b.right - 60, y: b.bottom - 100 }; })()"

            # S1 文件夹「粘贴文件夹」条件出现（无剪切不出现 → 剪切后出现）
            await lclick("f1")  # 单选归位（G 段后选中域 f1+f2）
            await rclick("f1")
            labels = await menu_labels()
            check("S1-1 无剪切时「粘贴文件夹」不出现", "粘贴文件夹" not in labels)
            await js_click(item_center("剪切文件夹"))  # 一次性任务类：点完菜单关
            await rclick("f1")
            check("S1-2 剪切后「粘贴文件夹」出现且未禁用", await ev(f"(() => {{ const it = {js_find('粘贴文件夹')}; return it != null && !it.disabled; }})()"))
            await esc_close()

            # S2 文件卡菜单（f1 视口已开；A1 = 单位置）
            check("S2-0 f1 视口渲卡", await ev("document.querySelector('.asset-card[data-key=\"A1|f1\"]') != null"))
            await js_rclick_expr(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            labels = await menu_labels()
            check("S2-1 文件卡「提示词」一级带箭头", await ev(f"(() => {{ const it = {js_find('提示词')}; return it != null && it.querySelector('.arrow') != null; }})()"))
            check("S2-2 单位置无「将此文件设为原始文件」", "将此文件设为原始文件" not in labels)
            check("S2-3 旧「保存提示词」占位项移除", "保存提示词" not in labels)
            await js_hover(item_center("提示词"))
            fl = await menu_labels(1)
            check("S2-4 提示词二级 = 分析/指定/新建md/新建txt", fl == ["分析提示词", "指定文本文件为提示词", "新建 .md 提示词文档", "新建 .txt 提示词文档"], str(fl))
            check("S2-5 提示词二级全禁用（业务未立项骨架）", await ev("[...document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-item')].every((x) => x.disabled)"))
            await js_hover(item_center("缩略图"))
            fl = await menu_labels(1)
            check("S2-6 缩略图二级重排序", fl == ["打开 Animate 生成缩略图", "转换为缩略图链接并复制", "将「文件」设为缩略图", "自定义缩略图 - 从剪贴板", "自定义缩略图 - 选择文件", "复制缩略图", "刷新缩略图"], str(fl))
            check("S2-7 「将「文件」设为缩略图」禁用占位文案", await ev(f"(() => {{ const it = {js_find('将「文件」设为缩略图', 1)}; return it != null && it.disabled; }})()"))
            check("S2-8 缩略图二级 4 分组", await ev("document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-group').length === 4"), str(await ev("document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-group').length")))
            await js_hover(item_center("导出文件"))
            fl = await menu_labels(1)
            check("S2-9 导出二级无「导出提示词为 .MD 文档」", all("提示词" not in x for x in fl), str(fl))
            await js_hover(item_center("复制数据"))
            fl = await menu_labels(1)
            check("S2-10 复制数据：有复制标签/描述、无复制提示词", "复制标签" in fl and "复制描述" in fl and "复制提示词" not in fl, str(fl))
            # S3/S4（复制→粘贴标签合并追加 / linkList 二级）= 独立探针 cdp-menu-paste-link-probe.py——
            # 长操作链后 CDP 合成右键偶发不派发 contextmenu（2026-09-05 实证，事件连 document capture 都收不到），
            # 拆短链独立跑稳定；主探针本段仅验复制数据二级构成
            await esc_close()

            # S5 档案树节点：删项 / 克隆 / 导出档案文案 / 提示词骨架 / 复制数据
            await rclick("a1")
            labels = await menu_labels()
            check("S5-1 档案菜单有「克隆」", "克隆" in labels)
            check("S5-2 删项五条不在（置顶/副本粘贴/链接粘贴/移除链接/打开提示词编辑器）", all(x not in labels for x in ["置顶", "作为副本粘贴", "作为链接粘贴", "从文件夹中移除档案链接", "打开提示词编辑器"]), str(labels))
            check("S5-3 「导出档案」文案（无导出文件）", "导出档案" in labels and "导出文件" not in labels)
            await js_hover(item_center("提示词"))
            fl = await menu_labels(1)
            check("S5-4 档案提示词二级三项（无分析）", fl == ["指定文本文档为提示词", "新建 .md 提示词文档", "新建 .txt 提示词文档"], str(fl))
            await js_hover(item_center("复制数据"))
            fl = await menu_labels(1)
            check("S5-5 档案复制数据有复制标签/描述", "复制标签" in fl and "复制描述" in fl, str(fl))
            await js_click(item_center("复制标签", 1))  # 槽 = [主角, NPC]
            # 粘贴标签挪一级（2026-09-05 走查二轮：跟在复制项后，不放复制数据二级）
            await rclick("a1")
            labels = await menu_labels()
            check("S5-6 档案复制后一级多出「粘贴标签」（复制档案下方）", "粘贴标签" in labels, str(labels))
            check("S5-7 一级位置：跟在「复制档案」后", await ev("(() => { const ls = [...document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-item .label')].map((x) => x.textContent); return ls.indexOf('粘贴标签') === ls.indexOf('复制档案') + 1; })()"))
            await esc_close()

            # S6 空白视口分流（2026-09-09 收窄修订）：全部降只扫描（虚拟视图不是位置）；
            # 未分类 = 候车区总入口（quickFullSpec：去新建文件夹/档案，留导入/新建文档/粘贴/扫描）
            await click_quick("全部")
            await js_rclick_expr(blank_bottom())
            labels = await menu_labels()
            check("S6-1 全部空白 = 只扫描两项（09-09 收窄：全部不是位置，落点类降档）", labels == ["扫描重复文件", "扫描文件/档案链接"], str(labels))
            await esc_close()
            await click_quick("未分类")
            await js_rclick_expr(blank_bottom())
            labels = await menu_labels()
            check("S6-2 未分类空白：无新建文件夹/新建档案（快速访问无文件夹概念）", "新建文件夹" not in labels and "新建档案" not in labels, str(labels))
            check("S6-3 未分类空白：有导入与新建文档（候车区总入口放开）", "导入本地文件" in labels and "Animate 文档" in labels, str(labels))
            check("S6-4 新建文档项带图标（mark-slot svg）", await ev(f"(() => {{ const it = {js_find('Animate 文档')}; return it?.querySelector('.mark-slot svg') != null; }})()"))
            check("S6-4b 无图标项占位对齐（markPad 空盒）", await ev(f"(() => {{ const it = {js_find('导入本地文件')}; const s = it?.querySelector('.mark-slot'); return s != null && s.querySelector('svg') == null; }})()"))
            await esc_close()
            for q in ["最近使用", "自动导入", "缩略图"]:
                await click_quick(q)
                # 自动导入首次点击弹确认窗（探针环境 currentSpace.autoImport 恒 false）——点「开启」批准进视图
                if q == "自动导入":
                    if await ev("!!document.querySelector('.modal-mask')"):
                        await js_click(el_center("[...document.querySelectorAll('.modal-mask button')].find((x) => x.textContent.includes('开启'))"))
                        await asyncio.sleep(0.6)
                await js_rclick_expr(blank_bottom())
                labels = await menu_labels()
                check(f"S6-5 {q} 空白 = 只扫描两项", labels == ["扫描重复文件", "扫描文件/档案链接"], str(labels))
                await esc_close()

            # S9 虚拟视图文件卡收窄（2026-09-09 入口收窄，PRD 02-15）：全部/最近使用/缩略图
            # 不是位置——位置级（移动/剪切/副本/移除）与落点类（粘贴/导入）不出现，占位项不列；
            # 留资产级与所见即所得（打开外发/链接管理导航/复制/重命名/缩略图组/导出/丢回收站）
            # 先清累积 toast（§25 坑①：headless 下 toast leave 动画卡死堆叠盖卡片吃右键，
            # 点 .close 无效须直接 remove——S9-7 实证）
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await asyncio.sleep(0.3)
            await click_quick("全部")
            check("S9-0 全部渲卡（含未分类 C3|root）", await ev("document.querySelector('.asset-card[data-key=\"A1|f1\"]') != null && document.querySelector('.asset-card[data-key=\"C3|root\"]') != null"))
            await js_rclick_expr(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            labels = await menu_labels()
            check("S9-1 全部卡：位置级/落点类全不出现", all(x not in labels for x in ["移动文件", "剪切文件", "创建副本", "从文件夹中移除", "作为链接粘贴", "作为副本粘贴"]), str(labels))
            check("S9-2 全部卡：占位项不出现（新标签页/添加资源库/AI/提示词/置顶）", all(x not in labels for x in ["新标签页打开", "添加至其他资源库", "AI 工具集", "提示词", "置顶"]), str(labels))
            check("S9-3 全部卡：保留集在（打开外发/链接管理/复制/重命名/缩略图/导出/复制数据/丢回收站）", all(x in labels for x in ["打开预览", "新窗口打开", "在默认应用打开", "在其他应用打开", "发送至 Animate", "在资源管理器打开", "文件链接管理", "打开文件链接所在位置", "复制文件", "重命名", "缩略图", "导出到本地磁盘", "复制数据", "丢到回收站"]), str(labels))
            check("S9-4 全部卡：导出拍平一级（无「导出文件」壳）", "导出文件" not in labels, str(labels))
            check("S9-5 单位置 A1 无「将此文件设为原始文件」（条件出现口径不变）", "将此文件设为原始文件" not in labels)
            await js_hover(item_center("缩略图"))
            fl = await menu_labels(1)
            check("S9-6 缩略图二级 = 收窄六项（无 Animate 生成占位）", fl == ["转换为缩略图链接并复制", "将「文件」设为缩略图", "自定义缩略图 - 从剪贴板", "自定义缩略图 - 选择文件", "复制缩略图", "刷新缩略图"], str(fl))
            await esc_close()
            # 双位置卡 B2|f2（链接卡）→ 设为原始出现且可用（!origin；origin 卡反向 = 禁用）
            await js_rclick_expr(el_center("document.querySelector('.asset-card[data-key=\"B2|f2\"]')"))
            labels = await menu_labels()
            if not labels:
                # 偶发不开菜单现场诊断（2026-09-09）：contextmenu capture 记录 + 浮层/遮罩存在性
                print("DIAG S9-7 ctxLog:", await ev("JSON.stringify(window.__ctxLog.slice(-4))"),
                      "| zoom:", await ev("!!document.querySelector('.zoom-mask')"),
                      "| modal:", await ev("!!document.querySelector('.modal-mask')"),
                      "| toast:", await ev("document.querySelectorAll('.toast').length"))
            check("S9-7 全部卡双位置：链接卡「将此文件设为原始文件」出现且可用", await ev(f"(() => {{ const it = {js_find('将此文件设为原始文件')}; return it != null && !it.disabled; }})()"), str(labels))
            await esc_close()
            # 虚拟视图卡片禁拖出（黑箱移动同病）：按下 + 移动超阈值不启动手势（body 无 .dragging）
            r = await ev(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await mouse("mousePressed", r["x"], r["y"])
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": r["x"] + 40, "y": r["y"] + 40, "buttons": 1}, session=sess)
            await asyncio.sleep(0.5)
            check("S9-8 全部视口卡片拖不出（body 无 dragging）", await ev("!document.body.classList.contains('dragging')"))
            await mouse("mouseReleased", r["x"] + 40, r["y"] + 40)
            await asyncio.sleep(0.4)
            # 位置视口对照：未分类卡 = 全量菜单（移动/剪切/副本/移除都在；候车区粘贴两项出现）
            await click_quick("未分类")
            await js_rclick_expr(el_center("document.querySelector('.asset-card[data-key=\"C3|root\"]')"))
            labels = await menu_labels()
            check("S9-9 未分类卡 = 全量（位置视口：移动/剪切/副本/移除/丢回收站都在）", all(x in labels for x in ["移动文件", "剪切文件", "创建副本", "从文件夹中移除", "丢到回收站"]), str(labels))
            check("S9-10 未分类卡：粘贴两项出现（S1 剪切文件夹后剪贴板非空；落点 root 已放开）", "作为链接粘贴" in labels and "作为副本粘贴" in labels, str(labels))
            await esc_close()

            # S7 回收站全量
            await click_quick("回收站")
            check("S7-0 回收站渲卡", await ev("document.querySelector('.asset-card[data-key=\"R1|recycle\"]') != null"))
            await js_rclick_expr(el_center("document.querySelector('.asset-card[data-key=\"R1|recycle\"]')"))
            labels = await menu_labels()
            expect7 = ["还原文件", "打开预览", "新标签页打开", "新窗口打开", "在默认应用打开", "在其他应用打开", "发送至 Animate", "在资源管理器打开", "移动文件", "将此文件设为原始文件", "打开文件链接所在位置", "剪切", "重命名", "添加至其他资源库", "导出文件", "AI 工具集", "复制数据", "彻底删除文件", "清空回收站"]
            check("S7-1 回收站卡菜单 = 全量 19 项", labels == expect7, str(labels))
            check("S7-2 占位四项禁用（移动/设为原始/剪切/重命名）", await ev("['移动文件', '将此文件设为原始文件', '剪切', '重命名'].every((l) => { const it = [...document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-item')].find((x) => x.querySelector('.label')?.textContent === l); return it != null && it.disabled; })"))
            check("S7-3 发送至 Animate 可用（fla 单选）", await ev(f"(() => {{ const it = {js_find('发送至 Animate')}; return it != null && !it.disabled; }})()"))
            check("S7-4 链接位置二级 = linkList 面板", await ev(f"(() => {{ const it = {js_find('打开文件链接所在位置')}; return it != null && it.querySelector('.arrow') != null; }})()"))
            await esc_close()
            await js_rclick_expr(blank_bottom())
            labels = await menu_labels()
            check("S7-5 回收站空白四项", labels == ["扫描重复文件", "扫描文件/档案链接", "还原全部文件", "清空回收站"], str(labels))
            check("S7-6 清空回收站未禁用（有条目）", await ev(f"(() => {{ const it = {js_find('清空回收站')}; return it != null && !it.disabled; }})()"))
            await esc_close()

            # S8 混选删除两项分流（f2 文件夹 + a1 档案）
            await lclick("f2")
            await lclick("a1", modifiers=2)  # Ctrl 加选
            await rclick("f2")
            labels = await menu_labels()
            check("S8-1 混选右键文件夹：删除文件夹 + 将档案丢到回收站", "删除文件夹" in labels and "将档案丢到回收站" in labels, str(labels))
            await esc_close()
            await rclick("a1")
            labels = await menu_labels()
            check("S8-2 混选右键档案：同两项", "删除文件夹" in labels and "将档案丢到回收站" in labels, str(labels))
            await esc_close()

            # —— D. 设计系统预览页 menus 展区（hash 导航不重跑 main.ts，须 reload 触发预览页挂载判定）——
            await send(ws, "Page.navigate", {"url": URL + "/#design-system-menus"}, session=sess)
            await asyncio.sleep(1)
            await send(ws, "Page.reload", session=sess)
            await asyncio.sleep(5)
            await send(ws, "Runtime.evaluate", {"expression": "document.querySelector('#design-system-menus')?.scrollIntoView()"}, session=sess)
            await asyncio.sleep(0.6)
            check("D1 预览页菜单展区存在", await ev("document.querySelectorAll('#design-system-menus .pv-menu-shell').length >= 9"), str(await ev("document.querySelectorAll('#design-system-menus .pv-menu-shell').length")))
            check("D2 预览色块行 cur 外环", await ev("getComputedStyle(document.querySelector('#design-system-menus .sw.cur')).boxShadow.includes('0px 0px 0px 1.5px')"))
            # 2026-09-05 带图标稿展区：文档图标陈列 / markPad 空盒 / 对勾左侧占位
            check("D3 文档图标陈列（7 项 mark-slot svg）", await ev("[...document.querySelectorAll('#design-system-menus .menu-item .mark-slot')].filter((s) => s.querySelector('svg')).length >= 7"), str(await ev("document.querySelectorAll('#design-system-menus .menu-item .mark-slot svg').length")))
            check("D4 markPad 空盒占位（无图标项对齐）", await ev("[...document.querySelectorAll('#design-system-menus .menu-item .mark-slot')].some((s) => !s.querySelector('svg'))"))
            check("D5 对勾在左侧（checked 项 mark-check 一级恒显）", await ev("document.querySelector('#design-system-menus .menu-item.checked .mark-slot .mark-check') != null"))
            # 对勾稿：外框 18×18、svg 实际 12×8.7——check.svg viewBox=18 满盒渲染不被二次缩放（2026-09-05 走查抓出）
            check("D5b 对勾实寸 18 满盒（viewBox 18 不缩放）", await ev("(() => { const c = document.querySelector('#design-system-menus .menu-item.checked .mark-check svg'); return c != null && Math.abs(c.getBoundingClientRect().width - 18) <= 0.5; })()"), str(await ev("document.querySelector('#design-system-menus .menu-item.checked .mark-check svg')?.getBoundingClientRect().width")))
            await shot("ctx-menu-preview")
            return 1 if fails else 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
