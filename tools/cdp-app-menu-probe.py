# CDP 实测：菜单栏「菜单」主菜单（2026-09-05 接业务轮；稿 = 用户当场供稿截图，spec = src/lib/menuSpecs/appMenu.ts）
# M1 点菜单钮 → 根菜单出（8 项 4 组 / 按钮左对齐 + 底 +4 弹出）；M2 文件 L2 全列（新建▸ + 新建四项带快捷键 + 导入/链接/导出组）；
# M3 新建 L3 = 新建文档七项带品牌图标（05_文档图标）；M4 hover 浅层项截断深层（L3 收、L2 留）；
# M5 仓库 → 资源库 L3：库列表纯对勾行（checkable / 当前库 checked / 无 LibraryMenuItem 抓手旧形态）+ 点当前库 keepOpen 不关；
# M6 仓库 → 仓库 L3：当前空间对勾行 + 新建/删除占位 disabled；M7 同钮再点 toggle 关；M8 Esc 关；
# M9 整理无选中 = 添加关系/移动/编辑描述 disabled；M10 选中 A1 后同项 enabled + 文件 L2 链接位置 L3 = linkList tip 壳；
# M1-7 一级条目菜单 min-width 250（2026-09-05 用户实机定，flyout 不受限）；
# M11 「转换为缩略图链接并复制」条件出现（同日用户定：fla 等不能作缩略图的格式不列，主菜单整理 + 卡片右键同步）。
# D 截图 root / docs-l3 / lib-l3。
import asyncio
import base64
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9371
URL = "http://localhost:5373"

# 桩同 cdp-context-menu-probe（2026-09-05 全量启动链路）；__calls 记录全量 invoke 供断言
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
  // F3 = fla 工程卡（f1）：缩略图链接条件出现的负例（2026-09-05 用户定：不能作缩略图的格式不列「转换为缩略图链接并复制」）
  mkP("F3", "f1", "工程.fla", "fla", true),
];
const RECYCLE = [mkP("R1", "recycle", "废稿.fla", "fla", false)];
const ARCHIVES = [
  { folderId: "a1", badge: null, avatar: null, props: ["主角", "NPC"],
    descriptions: [{ title: "设定", content: "雪国主角" }], links: [], createdAt: "", totalSize: 0, coverPath: null },
];
const DETAILS = {
  A1: { tags: ["走路", "跑步"], descriptions: [{ title: "动作", content: "基础动作" }] },
  B2: { tags: ["待机"], descriptions: [] },
  F3: { tags: [], descriptions: [] },
  R1: { tags: ["废案"], descriptions: [] },
};
window.__cbId = 0; window.__cbs = {};
// 有状态开库链（2026-09-05 主菜单探针）：boot 首查 get_current_library = null → reopenLastSession
// → open_space / open_library 回填 currentSpace / currentLibrary（仓库/资源库 L3 对勾与禁用态依赖）；
// 开库后 get_current_library 返回库信息（loadLibraries 复读）
let libOpen = false;
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
    if (cmd === "set_auto_import") {
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
    if (cmd === "open_space") return Promise.resolve({ name: "S", path: "E:/T/S", autoImport: true });
    if (cmd === "open_library") {
      libOpen = true;
      return Promise.resolve({ name: "测试库", path: "E:/T/S/测试库", assetCount: 3, folderCount: 3 });
    }
    if (cmd === "get_current_library") {
      return Promise.resolve(libOpen ? { name: "测试库", path: "E:/T/S/测试库", assetCount: 3, folderCount: 3 } : null);
    }
    const R = {
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: JSON.parse(JSON.stringify(RECYCLE)),
      list_recent_assets: [],
      // 启动自动重开链路（reopenLastSession "ok"）：回填 currentSpace / currentLibrary
      list_recent_libraries: [{ name: "测试库", path: "E:/T/S/测试库", lastOpenedAt: "" }],
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
                # buttons = 事件后的按键状态：release/move 恒 0（2026-09-05 主探针实证口径）
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "modifiers": modifiers, "buttons": 0 if t in ("mouseReleased", "mouseMoved") else (2 if button == "right" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            def el_center(expr):
                return f"(() => {{ const el = {expr}; if (!el) return null; const r = el.getBoundingClientRect(); return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }}; }})()"

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

            async def esc_close():
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await asyncio.sleep(0.4)

            async def menu_labels(idx=0):
                return await ev(f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)") or []

            async def menu_count():
                return await ev("document.querySelectorAll('.app-menu').length")

            # 菜单项查找（label 精确匹配，idx = 第几级菜单）
            def js_find(label, idx=0):
                return f"[...document.querySelectorAll('.app-menu')[{idx}].querySelectorAll('.menu-item')].find((x) => x.querySelector('.label')?.textContent === '{label}')"

            MENU_BTN = "[...document.querySelectorAll('.menu-bar .act-btn')].find((b) => b.textContent.trim() === '菜单')"

            # —— M1 点菜单钮 → 根菜单 ——
            btnRect = await ev(el_center(MENU_BTN))
            check("M0 菜单钮存在", btnRect is not None)
            if not btnRect:
                return 1
            await js_click(el_center(MENU_BTN))
            check("M1-1 根菜单出（1 个 .app-menu）", await menu_count() == 1)
            labels = await menu_labels(0)
            check("M1-2 根级 8 项全列", json.dumps(labels) == json.dumps(["仓库", "文件", "查找", "整理", "偏好设置", "帮助", "关于 AniKit", "退出"]), str(labels))
            check("M1-3 四组（三组间线）", await ev("document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-group').length === 4"))
            check("M1-4 退出带 Alt+F4 原串", await ev(f"({js_find('退出')})?.querySelector('.shortcut')?.textContent === 'Alt+F4'"))
            check("M1-5 偏好设置 / 关于 占位禁用", await ev(f"({js_find('偏好设置')})?.disabled === true && ({js_find('关于 AniKit')})?.disabled === true"))
            check("M1-6 弹出位 = 钮左缘对齐 + 底 +4", await ev(el_center(MENU_BTN)) is not None and await ev(f"(() => {{ const m = document.querySelector('.app-menu').getBoundingClientRect(); const b = {MENU_BTN}.getBoundingClientRect(); return Math.abs(m.left - b.left) <= 2 && Math.abs(m.top - b.bottom - 4) <= 2; }})()"))
            # 一级条目菜单 min-width 250（2026-09-05 用户实机定：纯自适应太窄显得轻；flyout 不受限）
            check("M1-7 一级菜单 min-width 250（短内容撑到 250）", await ev("(() => { const m = document.querySelector('.app-menu'); return m.style.minWidth === '250px' && m.offsetWidth === 250; })()"), str(await ev("document.querySelector('.app-menu')?.offsetWidth")))

            # —— M2 文件 L2 ——
            await js_hover(el_center(js_find("文件")))
            check("M2-1 文件 L2 飞出（2 个菜单）", await menu_count() == 2)
            labels = await menu_labels(1)
            expect = ["新建", "新建文件夹", "新建子文件夹", "新建档案", "转换为档案", "导入文件", "导入文件夹", "导入链接", "导入素材包", "文件/档案链接管理", "添加文件/档案链接", "打开文件/档案链接所在位置", "添加至其他资源库", "导出到计算机", "导出为素材包", "导出为 CSV"]
            check("M2-2 文件 L2 16 项全列", json.dumps(labels) == json.dumps(expect), str(labels))
            check("M2-3 新建带展开箭头", await ev(f"({js_find('新建', 1)})?.querySelector('.arrow svg') != null"))
            check("M2-4 新建文件夹 Ctrl+Shift+N / 转换为档案 Alt+N", await ev(f"({js_find('新建文件夹', 1)})?.querySelector('.shortcut')?.textContent === 'Ctrl+Shift+N' && ({js_find('转换为档案', 1)})?.querySelector('.shortcut')?.textContent === 'Alt+N'"))
            check("M2-5 导入链接 / 导出为 CSV 占位禁用", await ev(f"({js_find('导入链接', 1)})?.disabled === true && ({js_find('导出为 CSV', 1)})?.disabled === true"))
            await shot("app-menu-file-l2")

            # —— M3 新建 L3 = 文档七项带图标 ——
            await js_hover(el_center(js_find("新建", 1)))
            check("M3-1 新建 L3 飞出（3 个菜单）", await menu_count() == 3)
            labels = await menu_labels(2)
            check("M3-2 新建 L3 = 文档七项", json.dumps(labels) == json.dumps(["Animate 文档", "Photoshop 文档", "Illustrator 文档", "MarkDown 文档", "文本文档", "Word 文档", "Excel 文档"]), str(labels))
            check("M3-3 L3 带品牌图标（animate svg）", await ev("document.querySelectorAll('.app-menu')[2].querySelector('.menu-item .mark-slot svg') != null"))
            check("M3-4 L3 两组（工程 / 文本·办公分组线）", await ev("document.querySelectorAll('.app-menu')[2].querySelectorAll('.menu-group').length === 2"))
            await shot("app-menu-docs-l3")

            # —— M4 hover 浅层项截断深层 ——
            await js_hover(el_center(js_find("导入文件", 1)))
            check("M4 hover L2 普通项 → L3 收（2 个菜单）", await menu_count() == 2)

            # —— M5 仓库 → 资源库 L3（库列表纯对勾行）——
            await js_hover(el_center(js_find("仓库")))
            check("M5-1 仓库 L2 飞出", await menu_count() == 2)
            labels = await menu_labels(1)
            check("M5-2 仓库 L2 四项", json.dumps(labels) == json.dumps(["仓库", "资源库", "管理工作空间", "清除缓存并重新加载"]), str(labels))
            await js_hover(el_center(js_find("资源库", 1)))
            check("M5-3 资源库 L3 飞出（3 个菜单）", await menu_count() == 3)
            labels = await menu_labels(2)
            check("M5-4 L3 = 管理三项 + 库行", json.dumps(labels) == json.dumps(["新建资源库", "删除当前资源库", "合并资源库", "测试库"]), str(labels))
            check("M5-5 库行 = MenuItem 对勾（无 LibraryMenuItem 抓手旧形态）", await ev(f"({js_find('测试库', 2)})?.classList.contains('menu-item') === true && ({js_find('测试库', 2)})?.querySelector('.grip') == null"))
            check("M5-6 当前库 checked 恒显", await ev(f"({js_find('测试库', 2)})?.classList.contains('checked')"))
            check("M5-7 删除当前资源库可点（有当前库）", await ev(f"({js_find('删除当前资源库', 2)})?.disabled === false"))
            await shot("app-menu-lib-l3")
            # 点当前库 = keepOpen 不关（动态判定跳过切换）
            await js_click(el_center(js_find("测试库", 2)))
            check("M5-8 点当前库 keepOpen 不关菜单", await menu_count() == 3)

            # —— M6 仓库 → 仓库 L3（空间占位）——
            await js_hover(el_center(js_find("仓库", 1)))
            check("M6-1 仓库 L3 飞出", await menu_count() == 3)
            labels = await menu_labels(2)
            check("M6-2 L3 = 新建/删除占位 + 当前空间对勾", json.dumps(labels) == json.dumps(["新建仓库", "删除当前仓库", "S"]), str(labels))
            check("M6-3 新建/删除占位 disabled + 空间行 checked", await ev(f"({js_find('新建仓库', 2)})?.disabled === true && ({js_find('S', 2)})?.classList.contains('checked')"))

            # —— M7 同钮再点 = toggle 关 ——
            await js_click(el_center(MENU_BTN))
            check("M7 再点菜单钮 toggle 关闭", await menu_count() == 0)

            # —— M8 再开 + Esc 关 ——
            await js_click(el_center(MENU_BTN))
            check("M8-1 再开", await menu_count() == 1)
            await esc_close()
            check("M8-2 Esc 关", await menu_count() == 0)

            # —— M9 整理无选中 = 编辑类 disabled ——
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            check("M9-1 整理 L2 飞出", await menu_count() == 2)
            labels = await menu_labels(1)
            expect = ["文件/档案链接管理", "添加文件/档案链接", "打开文件/档案链接所在位置", "添加关系", "添加音频", "添加缩略图", "转为为缩略图链接并复制", "将「文件」设置为缩略图", "自定义缩略图 - 从剪贴板", "自定义缩略图 - 从本地磁盘", "更换封面图 - 从本地磁盘", "更换封面图 - 从资源库", "提示词", "添加标签", "复制标签", "粘贴标签", "清空标签", "编辑描述", "移动", "将所选设置为原始文件/文档"]
            check("M9-2 整理 L2 20 项全列", json.dumps(labels) == json.dumps(expect), str(labels))
            check("M9-3 无选中：添加关系 / 移动 / 编辑描述 disabled", await ev(f"({js_find('添加关系', 1)})?.disabled === true && ({js_find('移动', 1)})?.disabled === true && ({js_find('编辑描述', 1)})?.disabled === true"))
            check("M9-4 移动带 Ctrl+Y", await ev(f"({js_find('移动', 1)})?.querySelector('.shortcut')?.textContent === 'Ctrl+Y'"))
            await esc_close()

            # —— M10 选中 A1：编辑类点亮 + 链接位置 L3 = linkList tip 壳 ——
            await js_click(el_center("document.querySelector('[data-folder-id=\"f1\"]')"))
            await asyncio.sleep(0.8)
            check("M10-0 f1 视口渲三卡（含 fla F3）", await ev("document.querySelectorAll('.asset-card').length === 3"))
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            check("M10-1 选中后：添加关系 / 编辑描述 / 移动 点亮", await ev(f"({js_find('添加关系', 1)})?.disabled === false && ({js_find('编辑描述', 1)})?.disabled === false && ({js_find('移动', 1)})?.disabled === false"))
            check("M10-1b png 可选：转为缩略图链接项在且点亮", await ev(f"({js_find('转为为缩略图链接并复制', 1)})?.disabled === false"))
            check("M10-2 单位置文件不设原始 disabled", await ev(f"({js_find('将所选设置为原始文件/文档', 1)})?.disabled === true"))
            await js_hover(el_center(js_find("文件")))
            check("M10-3 文件 L2 链接位置可 hover", await ev(f"({js_find('打开文件/档案链接所在位置', 1)})?.disabled === false"))
            await js_hover(el_center(js_find("打开文件/档案链接所在位置", 1)))
            check("M10-4 链接位置 L3 出（3 个菜单）", await menu_count() == 3)
            check("M10-5 L3 = linkList tip 壳（AppMenu tip + pad 0）", await ev("(() => { const m = document.querySelectorAll('.app-menu')[2]; return m.classList.contains('tip') && m.style.padding === '0px'; })()"))
            check("M10-6 linkList 一行（A1 单位置）", await ev("document.querySelectorAll('.app-menu')[2].querySelectorAll('.link-rows .row').length === 1"), str(await ev("document.querySelectorAll('.app-menu')[2].innerHTML.slice(0, 120)")))
            await shot("app-menu-linklist-l3")
            await esc_close()
            check("M10-7 Esc 关（含 L3 面板）", await menu_count() == 0)

            # —— M11 「转换为缩略图链接」条件出现（2026-09-05 用户定：不能作缩略图的格式不列，两菜单同步）——
            # F3 = fla：主菜单整理不列该项
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"F3|f1\"]')"))
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            labels = await menu_labels(1)
            check("M11-1 fla 选中：整理无「转为为缩略图链接并复制」", "转为为缩略图链接并复制" not in labels, str(labels))
            check("M11-2 相邻项仍在（添加缩略图 / 将文件设为缩略图占位）", "添加缩略图" in labels and "将「文件」设置为缩略图" in labels)
            await esc_close()
            # 右键菜单同步：fla 卡右键 → 缩略图二级无「转换为缩略图链接并复制」
            r = await ev(el_center("document.querySelector('.asset-card[data-key=\"F3|f1\"]')"))
            await mouse("mouseMoved", r["x"], r["y"]); await asyncio.sleep(0.2)
            await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
            await asyncio.sleep(0.6)
            await js_hover(el_center(js_find("缩略图")))
            fl = await menu_labels(1)
            check("M11-3 fla 右键缩略图二级无转换项", "转换为缩略图链接并复制" not in fl, str(fl))
            check("M11-4 缩略图二级余项完整（6 项）", json.dumps(fl) == json.dumps(["打开 Animate 生成缩略图", "将「文件」设为缩略图", "自定义缩略图 - 从剪贴板", "自定义缩略图 - 选择文件", "复制缩略图", "刷新缩略图"]), str(fl))
            await esc_close()
            # png 卡对照：转换项在
            r = await ev(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await mouse("mouseMoved", r["x"], r["y"]); await asyncio.sleep(0.2)
            await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
            await asyncio.sleep(0.6)
            await js_hover(el_center(js_find("缩略图")))
            fl = await menu_labels(1)
            check("M11-5 png 右键缩略图二级有转换项", "转换为缩略图链接并复制" in fl, str(fl))
            await esc_close()
            check("M11-6 收尾菜单全关", await menu_count() == 0)

            print("----")
            print("FAIL:", fails if fails else "无")
            return 1 if fails else 0
    finally:
        proc.kill()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
