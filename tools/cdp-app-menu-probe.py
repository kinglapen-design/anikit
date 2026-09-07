# CDP 实测：菜单栏「菜单」主菜单（2026-09-05 接业务轮；稿 = 用户当场供稿截图，spec = src/lib/menuSpecs/appMenu.ts）
# M1 点菜单钮 → 根菜单出（8 项 4 组 / 按钮左对齐 + 底 +4 弹出）；M2 文件 L2 全列（新建▸ + 新建四项带快捷键 + 导入/链接/导出组）；
# M3 新建 L3 = 新建文档七项带品牌图标（05_文档图标）；M4 hover 浅层项截断深层（L3 收、L2 留）；
# M5 仓库 → 资源库 L3：库列表纯对勾行（checkable / 当前库 checked / 无 LibraryMenuItem 抓手旧形态）+ 点当前库 keepOpen 不关；
# M6 仓库 → 仓库 L3：当前空间对勾行 + 新建/删除占位 disabled；M7 同钮再点 toggle 关；M8 Esc 关；
# M9 整理无选中 = 添加关系/移动/编辑描述 disabled；M10 选中 A1 后同项 enabled + 文件 L2 链接位置 L3 = linkList tip 壳；
# M1-7 一级条目菜单 min-width 250（2026-09-05 用户实机定，flyout 不受限）；
# M11 「转换为缩略图链接并复制」格式门槛（同日用户定：fla 等不能作缩略图的格式——右键菜单不出现防误导；
#   主菜单恒列置灰，三轮用户定：主菜单是入口汇总需完整展示，和右键菜单不一样）。
# M12 全局搜索 = 聚焦搜索框 + 预填 `~`（同日用户定「就这么简单」）——续输关键字走 search_assets 全局范围，Esc 清空恢复。
# M13 添加标签 = 32 稿属性标签弹窗（单目标，多选禁用）/ 清空标签 = 所选项（文件+档案多选批量）标签全删，
#   空标签目标幂等跳过不发命令 + 零目标 toast。
# E 编辑菜单（2026-09-06 接业务轮；稿 = 用户当场供稿截图，spec = src/lib/menuSpecs/editMenu.ts）：
#   E1 空选态 12 项 6 组全列（克隆不在——副本/克隆同键 Ctrl+D 按选中域分流显示）+ 禁用态 + 弹出位 + min-width 250；
#   E2 素材选中点亮集；E3 复制 → 两粘贴点亮 → 作为副本粘贴发 duplicate_placements；
#   E4 文件夹域分流（克隆在 / 创建副本不在）+ 克隆发 clone_folders；E5 撤销点亮链（桩 __undo 态 + 操作后 refreshState 拉取）；
#   E6 同钮 toggle；E7 回收站视口只读——编辑类全 disabled。
# V 视图菜单（2026-09-06 接业务轮；稿 = 用户当场供稿截图，spec = src/lib/menuSpecs/viewMenu.ts）：
#   V1 根级 9 项 4 组 + 禁用态（信息查看管理占位）+ 弹出位 + 三显隐开关默认勾；V2 查看方式 L2 全列 + Alt+1 标注
#   + 点布局 keepOpen 对勾实时换（checked 函数化实证）；V3 排序 L3（自定义档升/降禁用 → 点键实时解禁 disabled 函数化）；
#   V4 时间/简介 L3（标签档占位禁用）；V5 开关组；V6 大纲管理 = 区块管理弹窗；V7 分组管理（f1 点亮 / 快速访问禁用）；
#   V8 标签页 L2（submenu 函数实时清单：单页禁用 → 分屏两页 → 关闭所有回单页）；V9 侧栏/信息面板收展 + 对勾实时；
#   V10 始终显示搜索框（关 = 隐藏 + toast 告知 Ctrl+F；Ctrl+F 唤出聚焦 / Esc 收回）；V11 同钮 toggle；
#   V12 重置窗口（2026-09-07 用户定：窗口 1400×1100 + 左 254 / 中 880 / 右 266 回设计基准——桩环境只验
#   三栏宽与 set_size IPC 参数，真机尺寸归实机走查）。
# D 截图 root / docs-l3 / lib-l3 / edit-empty / edit-asset / view-l2 / view-sort。
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
window.__calls = { set_folder_colors: [], set_asset_props: [], set_archive_props: [], search_assets: [], delete_placements: [], duplicate_placements: [], clone_folders: [], set_size: [] };
// 撤销栈有状态桩（E5 编辑菜单「撤销」点亮链）：操作 runOp 收尾 refreshState 拉取本对象
window.__undo = { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" };
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
    if (cmd === "search_assets") {
      window.__calls.search_assets.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve({ placements: [], archives: [] });
    }
    if (cmd === "get_asset_detail") {
      const d = DETAILS[args.assetId] ?? { tags: [], descriptions: [] };
      return Promise.resolve(JSON.parse(JSON.stringify({
        assetId: args.assetId, originName: "o.png", ext: "png", size: 1, width: 1, height: 1,
        duration: null, importedAt: "", modifiedAt: "", tags: d.tags, descriptions: d.descriptions,
        colors: [], links: [], note: "", placements: [], archives: [],
      })));
    }
    if (cmd === "get_undo_state") return Promise.resolve(JSON.parse(JSON.stringify(window.__undo)));
    // 视图菜单 V12「重置窗口」（2026-09-07）：窗口插件 IPC 只记录参数（桩环境无真窗口可缩放）
    if (cmd === "plugin:window|set_size") {
      window.__calls.set_size.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    // 编辑菜单 E 段三件（2026-09-06）：只记录参数不回写数据（断言 = 调用本身）
    if (cmd === "delete_placements") {
      window.__calls.delete_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "duplicate_placements") {
      window.__calls.duplicate_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve([]);
    }
    if (cmd === "clone_folders") {
      window.__calls.clone_folders.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
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
      list_prop_pool: ["FLA", "雪牧", "预设"],
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
        # 60s 无回应 = 卡死判负（2026-09-06 V 段实证：Page.captureScreenshot 可挂起——
        # 无超时则整探针陪葬且管道缓冲输出全丢；超时抛异常走 finally 收 Edge，已跑部分留档）
        raw = json.loads(await asyncio.wait_for(ws.recv(), timeout=60))
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

            # —— M11 「转换为缩略图链接」格式门槛（2026-09-05 用户定：fla 等不能作缩略图的格式——
            # 右键菜单不出现（防误导）；主菜单恒列置灰（三轮用户定：入口汇总需完整展示，与右键不同））——
            # F3 = fla：主菜单整理该项在且 disabled
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"F3|f1\"]')"))
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            labels = await menu_labels(1)
            check("M11-1 fla 选中：整理恒列「转为为缩略图链接并复制」且置灰", "转为为缩略图链接并复制" in labels and await ev(f"({js_find('转为为缩略图链接并复制', 1)})?.disabled === true"))
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

            # —— M12 全局搜索 = 聚焦 + 预填 `~`（2026-09-05 用户定，后端前缀协议全局范围）——
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("查找")))
            check("M12-0 查找 L2 全局搜索已点亮", await ev(f"({js_find('全局搜索', 1)})?.disabled === false"))
            await js_click(el_center(js_find("全局搜索", 1)))
            check("M12-1 点击后菜单关", await menu_count() == 0)
            check("M12-2 搜索框预填 `~` 且聚焦", await ev("(() => { const inp = document.querySelector('.sf-anchor input'); return !!inp && inp.value === '~' && document.activeElement === inp; })()"))
            # 续输关键字 → 全局范围搜索（search_assets scope.global + query 带前缀）
            await ev("(() => { const inp = document.querySelector('.sf-anchor input'); inp.value = '~雪牧'; inp.dispatchEvent(new Event('input', { bubbles: true })); })()")
            await asyncio.sleep(0.7)
            check("M12-3 search_assets 全局范围（scope.global = true，query 带 `~`）", await ev("(() => { const c = window.__calls.search_assets; return c.length > 0 && c[c.length - 1].scope.global === true && c[c.length - 1].query === '~雪牧'; })()"), str(await ev("JSON.stringify(window.__calls.search_assets)")))
            # Esc 清空并失焦 → 退出搜索态，网格恢复
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.6)
            check("M12-4 Esc 清空恢复（输入框空 + 网格回 3 卡）", await ev("(() => { const inp = document.querySelector('.sf-anchor input'); return !!inp && inp.value === '' && document.querySelectorAll('.asset-card').length === 3; })()"))

            # —— M13 添加标签 = 32 稿属性标签弹窗 / 清空标签 = 所选项全删（2026-09-05 用户定）——
            # M13-1 单选 A1（tags 走路/跑步）→ 添加标签弹面板
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            check("M13-1a 单选：添加标签点亮", await ev(f"({js_find('添加标签', 1)})?.disabled === false"))
            await js_click(el_center(js_find("添加标签", 1)))
            await asyncio.sleep(0.6)
            check("M13-1b 属性标签弹窗出（菜单已关）", await ev("document.querySelector('.props-dialog') != null && document.querySelectorAll('.app-menu').length === 0"))
            check("M13-1c 拥有 chips = 走路/跑步", json.dumps(await ev("[...document.querySelectorAll('.props-dialog .pd-owned .app-tag')].map((x) => x.textContent.trim())")) == json.dumps(["走路", "跑步"]), str(await ev("[...document.querySelectorAll('.props-dialog .pd-owned .app-tag')].map((x) => x.textContent.trim())")))
            check("M13-1d 属性池三项（桩）", await ev("document.querySelectorAll('.props-dialog .pool-chips .app-tag').length === 3"))
            # 取消关闭（草稿式放弃）
            await js_click(el_center("[...document.querySelectorAll('.props-dialog .pd-bottom .app-btn')].find((b) => b.textContent.trim() === '取消')"))
            check("M13-1e 取消关弹窗", await ev("document.querySelector('.props-dialog') == null"))

            # M13-2 多选 A1+B2 → 添加标签禁用（单目标面板）；清空标签批量写空集
            # Ctrl 追加 B2（modifiers=2；选中类名不猜，多选态由菜单禁用态实证）
            rB2 = await ev(el_center("document.querySelector('.asset-card[data-key=\"B2|f1\"]')"))
            await mouse("mousePressed", rB2["x"], rB2["y"], "left", 2); await mouse("mouseReleased", rB2["x"], rB2["y"], "left", 2)
            await asyncio.sleep(0.4)
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            check("M13-2b 多选：添加标签禁用 / 清空标签点亮", await ev(f"({js_find('添加标签', 1)})?.disabled === true && ({js_find('清空标签', 1)})?.disabled === false"))
            await js_click(el_center(js_find("清空标签", 1)))
            await asyncio.sleep(0.8)
            check("M13-2c 清空批量写空集（A1 + B2 各一次）", await ev("(() => { const c = window.__calls.set_asset_props.filter((x) => Array.isArray(x.props) && x.props.length === 0); return c.length === 2 && c[0].assetId === 'A1' && c[1].assetId === 'B2'; })()"), str(await ev("JSON.stringify(window.__calls.set_asset_props)")))
            # 清 toast 防挡命中（2026-09-05 主探针踩坑口径）
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
            await asyncio.sleep(0.4)

            # M13-3 F3 无标签 → 清空 = 幂等零目标（不发命令 + toast 提示）
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"F3|f1\"]')"))
            await js_click(el_center(MENU_BTN))
            await js_hover(el_center(js_find("整理")))
            await js_click(el_center(js_find("清空标签", 1)))
            await asyncio.sleep(0.8)
            check("M13-3a 空标签目标不发 set_asset_props", await ev("window.__calls.set_asset_props.filter((x) => x.assetId === 'F3').length === 0"))
            check("M13-3b 零目标 toast「所选项都没有标签」", await ev("[...document.querySelectorAll('.toast')].some((t) => t.textContent.includes('所选项都没有标签'))"), str(await ev("[...document.querySelectorAll('.toast')].map((t) => t.textContent.trim())")))
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")

            # —— E 编辑菜单（2026-09-06 接业务；稿 = 用户当场供稿截图，spec = menuSpecs/editMenu）——
            EDIT_BTN = "[...document.querySelectorAll('.menu-bar .act-btn')].find((b) => b.textContent.trim() === '编辑')"

            def e_disabled(labels_list, want):
                return " && ".join([f"({js_find(l)})?.disabled === {str(want).lower()}" for l in labels_list])

            # E1 空选态（Esc 清掉 M13 的 F3 选中）
            await esc_close()
            await js_click(el_center(EDIT_BTN))
            check("E1-1 编辑菜单出（1 个 .app-menu）", await menu_count() == 1)
            labels = await menu_labels(0)
            expect_e = ["撤销", "重做", "剪切", "复制", "作为文件/档案链接粘贴", "作为副本粘贴", "重命名", "创建副本", "全选", "清除选择", "从文件夹中移除", "丢到回收站"]
            check("E1-2 12 项全列（克隆不在——同键 Ctrl+D 分流显示）", json.dumps(labels) == json.dumps(expect_e), str(labels))
            check("E1-3 六组", await ev("document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-group').length === 6"))
            check("E1-4 空选禁用态（撤销/重做/剪切/复制/两粘贴/重命名/副本/清除/移除/回收站）", await ev(e_disabled(["撤销", "重做", "剪切", "复制", "作为文件/档案链接粘贴", "作为副本粘贴", "重命名", "创建副本", "清除选择", "从文件夹中移除", "丢到回收站"], True)))
            check("E1-5 全选恒可点", await ev(f"({js_find('全选')})?.disabled === false"))
            check("E1-6 快捷键原串（撤销 Ctrl+Z / 清除选择 Esc / 移除 Shift+Delete / 丢到回收站 Delete）", await ev(f"({js_find('撤销')})?.querySelector('.shortcut')?.textContent === 'Ctrl+Z' && ({js_find('清除选择')})?.querySelector('.shortcut')?.textContent === 'Esc' && ({js_find('从文件夹中移除')})?.querySelector('.shortcut')?.textContent === 'Shift+Delete' && ({js_find('丢到回收站')})?.querySelector('.shortcut')?.textContent === 'Delete'"))
            check("E1-7 弹出位 = 编辑钮左缘对齐 + 底 +4 / min-width 250", await ev(f"(() => {{ const m = document.querySelector('.app-menu').getBoundingClientRect(); const b = {EDIT_BTN}.getBoundingClientRect(); return Math.abs(m.left - b.left) <= 2 && Math.abs(m.top - b.bottom - 4) <= 2 && document.querySelector('.app-menu').style.minWidth === '250px'; }})()"))
            await shot("app-menu-edit-empty")
            await esc_close()

            # E2 素材选中（A1）点亮集
            await js_click(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_click(el_center(EDIT_BTN))
            check("E2-1 素材选中：剪切/复制/重命名/创建副本/清除选择/移除/丢到回收站 点亮", await ev(e_disabled(["剪切", "复制", "重命名", "创建副本", "清除选择", "从文件夹中移除", "丢到回收站"], False)))
            check("E2-2 无剪贴板：两粘贴 + 撤销/重做 disabled", await ev(e_disabled(["作为文件/档案链接粘贴", "作为副本粘贴", "撤销", "重做"], True)))
            await shot("app-menu-edit-asset")

            # E3 复制 → 两粘贴点亮；作为副本粘贴发 duplicate_placements
            await js_click(el_center(js_find("复制")))
            check("E3-1 复制后菜单关", await menu_count() == 0)
            await js_click(el_center(EDIT_BTN))
            check("E3-2 有剪贴板：两粘贴点亮", await ev(e_disabled(["作为文件/档案链接粘贴", "作为副本粘贴"], False)))
            await js_click(el_center(js_find("作为副本粘贴")))
            await asyncio.sleep(0.8)
            check("E3-3 副本粘贴发 duplicate_placements（A1 → f1）", await ev("(() => { const c = window.__calls.duplicate_placements; return c.length === 1 && c[0].targetFolder === 'f1' && c[0].items.length === 1 && c[0].items[0].assetId === 'A1'; })()"), str(await ev("JSON.stringify(window.__calls.duplicate_placements)")))
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
            await asyncio.sleep(0.4)

            # E4 文件夹域分流（树 f2 plain click = 导航 + 入选）：克隆在 / 创建副本不在
            await ev("window.__undo.canUndo = true")  # 供 E5：克隆收尾 refreshState 拉取（桩纪律：操作驱动拉取）
            await js_click(el_center("document.querySelector('[data-folder-id=\"f2\"]')"))
            await asyncio.sleep(0.6)
            await js_click(el_center(EDIT_BTN))
            labels = await menu_labels(0)
            check("E4-1 文件夹域：克隆在 / 创建副本不在", "克隆" in labels and "创建副本" not in labels, str(labels))
            check("E4-2 文件夹域：复制/移除 disabled；剪切/重命名/克隆/丢到回收站 enabled", await ev(e_disabled(["复制", "从文件夹中移除"], True) + " && " + e_disabled(["剪切", "重命名", "克隆", "丢到回收站"], False)))
            await js_click(el_center(js_find("克隆")))
            await asyncio.sleep(0.8)
            check("E4-3 克隆发 clone_folders（f2）", await ev("(() => { const c = window.__calls.clone_folders; return c.length === 1 && c[0].folderIds.length === 1 && c[0].folderIds[0] === 'f2'; })()"), str(await ev("JSON.stringify(window.__calls.clone_folders)")))
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
            await asyncio.sleep(0.4)

            # E5 撤销点亮链（克隆 runOp 收尾 refreshState 已拉 __undo.canUndo=true）
            await js_click(el_center(EDIT_BTN))
            check("E5-1 撤销点亮 / 重做仍 disabled", await ev(f"({js_find('撤销')})?.disabled === false && ({js_find('重做')})?.disabled === true"))
            await esc_close()

            # E6 同钮再点 = toggle 关
            await js_click(el_center(EDIT_BTN))
            check("E6-1 编辑钮开", await menu_count() == 1)
            await js_click(el_center(EDIT_BTN))
            check("E6-2 同钮再点 toggle 关", await menu_count() == 0)

            # E7 回收站视口只读（编辑类全 disabled；Esc 先清选择回素材域形态）
            await js_click(el_center("[...document.querySelectorAll('.block.quick .tree-item')].find((x) => x.textContent.includes('回收站'))"))
            await asyncio.sleep(0.6)
            await esc_close()
            await js_click(el_center(EDIT_BTN))
            labels = await menu_labels(0)
            check("E7-1 回收站空选：创建副本在（回素材域形态）/ 克隆不在", "创建副本" in labels and "克隆" not in labels, str(labels))
            check("E7-2 回收站只读：剪切/复制/重命名/创建副本/移除/丢到回收站 全 disabled", await ev(e_disabled(["剪切", "复制", "重命名", "创建副本", "从文件夹中移除", "丢到回收站"], True)))
            check("E7-3 全选 / 撤销可用", await ev(e_disabled(["全选", "撤销"], False)))
            await esc_close()
            check("E7-4 收尾菜单全关", await menu_count() == 0)

            # —— V 视图菜单（2026-09-06 接业务；稿 = 用户当场供稿截图，spec = menuSpecs/viewMenu）——
            # checked/disabled 函数化（keepOpen 期间对勾实时刷新）与 submenu 函数（标签页清单实时生成）同轮落地
            VIEW_BTN = "[...document.querySelectorAll('.menu-bar .act-btn')].find((b) => b.textContent.trim() === '视图')"

            # V0 导航到文件夹 f1（分组管理目标 = 当前文件夹视口；E7 收尾停在回收站）
            await js_click(el_center("document.querySelector('[data-folder-id=\"f1\"]')"))
            await asyncio.sleep(0.6)

            # V1 根级全列 + 禁用态 + 弹出位 + 三显隐开关默认勾
            await js_click(el_center(VIEW_BTN))
            check("V1-1 视图菜单出（1 个 .app-menu）", await menu_count() == 1)
            labels = await menu_labels(0)
            expect_v = ["大纲管理", "分组管理", "信息查看管理", "查看方式", "标签页", "显示侧边栏", "显示信息面板", "始终显示搜索框", "重置窗口"]
            check("V1-2 根级 9 项全列", json.dumps(labels) == json.dumps(expect_v), str(labels))
            check("V1-3 四组", await ev("document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-group').length === 4"))
            check("V1-4 信息查看管理占位禁用；大纲/分组管理点亮（f1 视口）", await ev(f"({js_find('信息查看管理')})?.disabled === true && ({js_find('大纲管理')})?.disabled === false && ({js_find('分组管理')})?.disabled === false"))
            check("V1-5 弹出位 = 视图钮左缘对齐 + 底 +4 / min-width 250", await ev(f"(() => {{ const m = document.querySelector('.app-menu').getBoundingClientRect(); const b = {VIEW_BTN}.getBoundingClientRect(); return Math.abs(m.left - b.left) <= 2 && Math.abs(m.top - b.bottom - 4) <= 2 && document.querySelector('.app-menu').style.minWidth === '250px'; }})()"))
            check("V1-6 显示侧边栏/信息面板/搜索框 默认全勾", await ev(f"({js_find('显示侧边栏')})?.classList.contains('checked') && ({js_find('显示信息面板')})?.classList.contains('checked') && ({js_find('始终显示搜索框')})?.classList.contains('checked')"))

            # V2 查看方式 L2：全列 + 布局对勾/快捷键 + 点「网格」keepOpen 对勾实时换（checked 函数化实证）
            await js_hover(el_center(js_find("查看方式")))
            check("V2-1 查看方式 L2 飞出（2 个菜单）", await menu_count() == 2)
            labels = await menu_labels(1)
            expect_v2 = ["自适应", "瀑布流", "网格", "列表", "排序方式", "时间显示格式", "简介显示方式", "显示扩展名", "显示角标", "显示链接标识", "显示子文件夹内容", "自动播放动图", "自动播放视频预览"]
            check("V2-2 L2 13 项 4 组全列", json.dumps(labels) == json.dumps(expect_v2) and await ev("document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-group').length === 4"), str(labels))
            check("V2-3 自适应默认勾 + Alt+1 快捷键标注", await ev(f"({js_find('自适应', 1)})?.classList.contains('checked') && ({js_find('自适应', 1)})?.querySelector('.shortcut')?.textContent === 'Alt+1'"))
            check("V2-4 三个子菜单项带展开箭头", await ev("[...document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-item')].filter((x) => ['排序方式', '时间显示格式', '简介显示方式'].includes(x.querySelector('.label')?.textContent)).every((x) => x.querySelector('.arrow svg') != null)"))
            await shot("app-menu-view-l2")
            await js_click(el_center(js_find("网格", 1)))
            check("V2-5 点布局项 keepOpen 不关菜单", await menu_count() == 2)
            check("V2-6 对勾实时换：网格勾 / 自适应灭（checked 函数化）", await ev(f"({js_find('网格', 1)})?.classList.contains('checked') && !({js_find('自适应', 1)})?.classList.contains('checked')"))

            # V3 排序方式 L3：自定义档升/降禁用 → 点排序键实时解禁（disabled 函数化实证）→ 降序切换
            await js_hover(el_center(js_find("排序方式", 1)))
            check("V3-1 排序方式 L3 飞出（3 个菜单）", await menu_count() == 3)
            labels = await menu_labels(2)
            check("V3-2 L3 = 8 键 + 升/降序", json.dumps(labels) == json.dumps(["自定义", "名称", "文件大小", "时长", "扩展名", "创建时间", "修改时间", "导入时间", "升序", "降序"]), str(labels))
            check("V3-3 默认自定义勾 + 升/降序禁用（存储序无升降语义）", await ev(f"({js_find('自定义', 2)})?.classList.contains('checked') && ({js_find('升序', 2)})?.disabled === true && ({js_find('降序', 2)})?.disabled === true"))
            await js_click(el_center(js_find("文件大小", 2)))
            check("V3-4 点排序键 keepOpen + 升/降序实时解禁（disabled 函数化）", await ev(f"({js_find('文件大小', 2)})?.classList.contains('checked') && ({js_find('升序', 2)})?.disabled === false"))
            check("V3-5 升序默认勾（sortAsc=true）", await ev(f"({js_find('升序', 2)})?.classList.contains('checked')"))
            await js_click(el_center(js_find("降序", 2)))
            check("V3-6 点降序：降序勾 / 升序灭（对勾实时）", await ev(f"({js_find('降序', 2)})?.classList.contains('checked') && !({js_find('升序', 2)})?.classList.contains('checked')"))

            # V4 时间显示格式 / 简介显示方式 L3
            await js_hover(el_center(js_find("时间显示格式", 1)))
            check("V4-1 时间显示 L3 = 帧数勾 / 时间码", json.dumps(await menu_labels(2)) == json.dumps(["帧数", "时间码"]) and await ev(f"({js_find('帧数', 2)})?.classList.contains('checked')"))
            await js_hover(el_center(js_find("简介显示方式", 1)))
            labels = await menu_labels(2)
            check("V4-2 简介 L3 = 显示简介 + 内容 5 档", json.dumps(labels) == json.dumps(["显示简介", "尺寸", "文件大小", "创建日期", "修改日期", "标签"]), str(labels))
            check("V4-3 显示简介勾 / 尺寸勾 / 标签档禁用（卡片数据无 tags）", await ev(f"({js_find('显示简介', 2)})?.classList.contains('checked') && ({js_find('尺寸', 2)})?.classList.contains('checked') && ({js_find('标签', 2)})?.disabled === true"))
            await js_click(el_center(js_find("显示简介", 2)))
            check("V4-4 关显示简介 keepOpen + 对勾实时灭", await menu_count() == 3 and await ev(f"({js_find('显示简介', 2)})?.classList.contains('checked') === false"))
            await js_click(el_center(js_find("显示简介", 2)))  # 恢复默认态收尾
            await asyncio.sleep(0.3)

            # V5 L2 开关组：默认态 + 自动播放动图点亮
            check("V5-1 显示扩展名/角标/链接标识/子文件夹 默认全勾", await ev("['显示扩展名', '显示角标', '显示链接标识', '显示子文件夹内容'].every((l) => [...document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-item')].find((x) => x.querySelector('.label')?.textContent === l)?.classList.contains('checked'))"))
            await js_click(el_center(js_find("自动播放动图", 1)))
            # keepOpen = 根 + L2 不关；L3 被点击项 hover 截断收掉（标准行为，非 keepOpen 失效）
            check("V5-2 自动播放动图默认关 → 点击点亮（keepOpen：根+L2 不关，L3 hover 截断收）", await menu_count() == 2 and await ev(f"({js_find('自动播放动图', 1)})?.classList.contains('checked')"))

            # V6 大纲管理 = 侧栏区块管理弹窗（BlockManageDialog）
            await esc_close()
            await js_click(el_center(VIEW_BTN))
            await js_click(el_center(js_find("大纲管理")))
            await asyncio.sleep(0.6)
            check("V6-1 大纲管理 → 区块管理弹窗出（菜单关）", await ev("document.querySelector('.block-manage') != null && document.querySelectorAll('.app-menu').length === 0"))
            check("V6-2 弹窗含桩区块行（目录）", await ev("[...document.querySelectorAll('.block-manage')].some((d) => d.textContent.includes('目录'))"))
            await js_click(el_center("[...document.querySelectorAll('.block-manage button')].find((b) => b.textContent.trim() === '取消')"))
            check("V6-3 取消关弹窗", await ev("document.querySelector('.block-manage') == null"))

            # V7 分组管理 = GroupManageDialog（groupDialogFolder = f1）；非文件夹视口禁用
            await js_click(el_center(VIEW_BTN))
            await js_click(el_center(js_find("分组管理")))
            await asyncio.sleep(0.6)
            check("V7-1 分组管理弹窗出（目标 = f1 视口）", await ev("document.querySelector('.group-manage') != null"))
            await js_click(el_center("[...document.querySelectorAll('.group-manage button')].find((b) => b.textContent.trim() === '取消')"))
            check("V7-2 取消关弹窗", await ev("document.querySelector('.group-manage') == null"))
            await js_click(el_center("[...document.querySelectorAll('.block.quick .tree-item')].find((x) => x.textContent.includes('全部'))"))
            await asyncio.sleep(0.6)
            await js_click(el_center(VIEW_BTN))
            check("V7-3 快速访问视口：分组管理禁用（无目标文件夹）", await ev(f"({js_find('分组管理')})?.disabled === true"))

            # V8 标签页 ▸（submenu 函数实时生成）：单页禁用态 → 左右分屏 → 两页清单 → 关闭所有回单页
            await js_hover(el_center(js_find("标签页")))
            labels = await menu_labels(1)
            check("V8-1 标签页 L2 = 清单 1 行 + 管理 6 项", json.dumps(labels) == json.dumps(["全部", "锁定当前标签页", "关闭当前标签页", "关闭所有标签页", "关闭其他标签页", "左右分屏", "上下分屏"]), str(labels))
            check("V8-2 当前页勾 / 关闭所有·其他 单页禁用 / 关闭当前点亮", await ev(f"({js_find('全部', 1)})?.classList.contains('checked') && ({js_find('关闭所有标签页', 1)})?.disabled === true && ({js_find('关闭其他标签页', 1)})?.disabled === true && ({js_find('关闭当前标签页', 1)})?.disabled === false"))
            await js_click(el_center(js_find("左右分屏", 1)))
            await asyncio.sleep(0.8)
            check("V8-3 左右分屏 = 2 标签页（菜单关）", await ev("document.querySelectorAll('.dv-tab').length === 2 && document.querySelectorAll('.app-menu').length === 0"))
            await js_click(el_center(VIEW_BTN))
            await js_hover(el_center(js_find("标签页")))
            labels = await menu_labels(1)
            check("V8-4 分屏后清单 2 行 + 关闭所有解禁（清单实时生成）", len(labels) == 9 and await ev(f"({js_find('关闭所有标签页', 1)})?.disabled === false"), str(labels))
            await js_click(el_center(js_find("关闭所有标签页", 1)))
            await asyncio.sleep(0.8)
            check("V8-5 关闭所有 → 回单页（自动补默认「全部」）", await ev("document.querySelectorAll('.dv-tab').length === 1"))

            # V9 显示侧边栏 / 显示信息面板（收展 + 对勾实时；收展动画 400ms 后断言宽）
            await js_click(el_center(VIEW_BTN))
            await js_click(el_center(js_find("显示侧边栏")))
            await asyncio.sleep(0.9)
            check("V9-1 点显示侧边栏 keepOpen + 对勾灭 + 左栏收起", await menu_count() == 1 and await ev(f"({js_find('显示侧边栏')})?.classList.contains('checked') === false && document.querySelector('.sidebar-left').offsetWidth === 0"))
            await js_click(el_center(js_find("显示侧边栏")))
            await asyncio.sleep(0.9)
            check("V9-2 再点恢复（对勾亮 + 左栏回 254）", await ev(f"({js_find('显示侧边栏')})?.classList.contains('checked') && document.querySelector('.sidebar-left').offsetWidth === 254"))
            await js_click(el_center(js_find("显示信息面板")))
            await asyncio.sleep(0.9)
            check("V9-3 显示信息面板：右栏收起 + 对勾灭", await ev(f"({js_find('显示信息面板')})?.classList.contains('checked') === false && document.querySelector('.sidebar-right').offsetWidth === 0"))
            await js_click(el_center(js_find("显示信息面板")))
            await asyncio.sleep(0.9)
            check("V9-4 右栏恢复", await ev(f"({js_find('显示信息面板')})?.classList.contains('checked') && document.querySelector('.sidebar-right').offsetWidth > 0"))

            # V10 始终显示搜索框（2026-09-06 用户定：关 = 下沉隐藏 + toast 告知 Ctrl+F；Ctrl+F 唤出 / Esc 收回）
            check("V10-1 默认悬浮栏在（.sf-anchor）", await ev("document.querySelector('.sf-anchor') != null"))
            await js_click(el_center(js_find("始终显示搜索框")))
            await asyncio.sleep(0.6)
            check("V10-2 关闭后 keepOpen + 对勾灭 + 悬浮栏收走（下沉动画后 DOM 移除）", await menu_count() == 1 and await ev(f"({js_find('始终显示搜索框')})?.classList.contains('checked') === false && document.querySelector('.sf-anchor') == null"))
            check("V10-3 toast 告知 Ctrl+F 通道", await ev("[...document.querySelectorAll('.toast')].some((t) => t.textContent.includes('Ctrl+F'))"), str(await ev("[...document.querySelectorAll('.toast')].map((t) => t.textContent.trim())")))
            # Ctrl+F 唤出（菜单开着不挡——onSearchFocus 无菜单让位）；modifiers 2 = Ctrl
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "f", "code": "KeyF", "modifiers": 2, "windowsVirtualKeyCode": 70}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "f", "code": "KeyF", "modifiers": 2, "windowsVirtualKeyCode": 70}, session=sess)
            await asyncio.sleep(0.6)
            check("V10-4 Ctrl+F 唤出悬浮栏（下浮入）", await ev("document.querySelector('.sf-anchor') != null"))
            check("V10-5 唤出即聚焦输入框", await ev("document.activeElement === document.querySelector('.sf-anchor input')"))
            # Esc = 清空失焦收回悬浮栏（菜单同被 Esc 关——escHadOverlay 口径两吃）
            await esc_close()
            await asyncio.sleep(0.4)
            check("V10-6 Esc 收回悬浮栏 + 菜单关", await ev("document.querySelector('.sf-anchor') == null && document.querySelectorAll('.app-menu').length === 0"))
            await js_click(el_center(VIEW_BTN))
            await js_click(el_center(js_find("始终显示搜索框")))
            await asyncio.sleep(0.6)
            check("V10-7 恢复勾选：悬浮栏回 + 对勾亮", await ev(f"document.querySelector('.sf-anchor') != null && ({js_find('始终显示搜索框')})?.classList.contains('checked')"))
            await esc_close()
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
            await asyncio.sleep(0.4)

            # V11 同钮 toggle
            await js_click(el_center(VIEW_BTN))
            check("V11-1 视图钮开", await menu_count() == 1)
            await js_click(el_center(VIEW_BTN))
            check("V11-2 同钮再点 toggle 关", await menu_count() == 0)

            # V12 重置窗口（2026-09-07 用户定：窗口 1400×1100 + 三栏 254/880/266 回设计基准）——
            # 桩环境无真窗口：验三栏宽 + set_size IPC 参数；真机尺寸 / 最大化还原归实机走查
            await ev("window.__calls.set_size = []")
            await js_click(el_center(VIEW_BTN))
            check("V12-1 视图菜单开", await menu_count() == 1)
            await js_click(el_center(js_find("重置窗口")))
            await asyncio.sleep(0.6)
            check("V12-2 点重置窗口 = 菜单关（非 keepOpen）", await menu_count() == 0)
            check("V12-3 左栏 254 / 右栏 266（侧栏收展动画后）", await ev("document.querySelector('.sidebar-left').offsetWidth === 254 && document.querySelector('.sidebar-right').offsetWidth === 266"))
            check("V12-4 set_size IPC = 逻辑 1400×1100", await ev("window.__calls.set_size.length === 1 && window.__calls.set_size[0].value?.Logical?.width === 1400 && window.__calls.set_size[0].value?.Logical?.height === 1100"), str(await ev("window.__calls.set_size")))

            print("----")
            print("FAIL:", fails if fails else "无")
            return 1 if fails else 0
    finally:
        proc.kill()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
