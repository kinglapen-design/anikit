# CDP 回归探针：名称排序显示下拖拽排序 = 可见顺序物化（2026-08-15 修复回归）
# 旧 bug：树按名称排序显示后（右键 → 排序依据 → 当前层级 - 标题 A → Z），拖拽落点
# dropFolderBeside（src/lib/dragFolderOps.ts）只对存储序 reorder → refresh 后名称排序盖回去
# = "显示排序成功但实际没变"。修法：该层显示非 custom 排序时物化可见顺序——全层可见兄弟
# 剔除移动组按落点插回，整层序列 reorder_folders(seq, parentId, null)，ui.treeSort 切回 custom。
# 场景：
#   S1 加载后显示序 = 存储序 [一丙, 二甲, 三乙]（custom）
#   S2 右键"二甲"→ 排序依据 → 当前层级 - 标题 A → Z → 显示序变 [二甲, 三乙, 一丙]（zh 拼音序）
#   S3 拖"二甲"到"三乙"下 1/3（zone after）：① 桩收 reorder_folders folderIds=["f3","f2","f1"]
#      （物化全层序列）且 beforeFolderId=null、不走 move_folders；② refresh 后显示序
#      = [三乙, 二甲, 一丙]（不被名称排序盖回）；③ toast "已调整"类成功提示
#   S4 对照基线（custom 态）：拖"一丙"到"三乙"前 → reorder_folders 旧路径参数
#      folderIds=["f1"]、beforeFolderId="f3"，证明正常路径未破坏
#   S5 排序依据二级菜单首项"自定义"（2026-08-14 用户定）：断言首项文本，点它 →
#      treeSort 回 custom、显示回存储序
#   S6 名称排序下新建文件夹（createAndName 先物化再建）：A→Z 排序态右键"二甲"→ 新建文件夹。
#      断言：① 先 reorder_folders（物化全层 ["f2","f3","f1"]，静默无 toast）后 create_folder
#      （parentId null、afterFolderId="f2"）；② refresh 后显示序 = [二甲, 新建文件夹, 三乙, 一丙]
#      （落点 = 选中项下一行，custom 不再盖回）；③ 单选新文件夹（selected+domain 合计 1 且是
#      新项——用户实机双高亮问题回归）；④ 新项命名输入态（.rename-input 可见，默认名）
# 命名注记：FolderTree 排序用 localeCompare(zh) = 拼音序，"甲乙丙" 拼音序 丙<甲<乙 与存储序
# 相同无法区分排序生效与否，故用"一丙/二甲/三乙"前缀（拼音 二<三<一）让排序真实改变显示序。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-tree-sort-drag-probe.py
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
PORT = 9355
URL = "http://localhost:5373"
SHOT = "tools/style-preview/tree-sort-drag.png"
SHOT2 = "tools/style-preview/tree-sort-create.png"
SLEEP = 0.3

# 桩：单区块「目录」三顶级文件夹，存储序 f1一丙 / f2二甲 / f3三乙（zh 拼音序 = 二<三<一，
# 与存储序不同）。reorder_folders 真实模拟重排（refresh 会重拉 list_folder_tree）：
# folderIds 覆盖全层 → 直接作顶层新序（物化路径）；否则锚点语义 = 移到 beforeFolderId 前，
# null = 末尾。create_folder 真实建档：新节点 f9 插到 afterFolderId 后（null = 顶层末尾），
# 返回 Folder 结构 {id, name, parent, order}（前端只用 id）。写操作留痕 __calls + __opLog
# （调用先后序断言用：S6 物化 reorder 必须先于 create_folder）。
STUB = r"""
window.__calls = { reorder_folders: [], move_folders: [], create_folder: [] };
window.__opLog = [];
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "一丙", count: 0, children: [] },
    { id: "f2", name: "二甲", count: 0, children: [] },
    { id: "f3", name: "三乙", count: 0, children: [] },
  ],
};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 3 },
      get_ui_settings: {},
      // 深拷贝返回：真实后端 invoke 每次反序列化出新对象；原地改 TREE 直接返回同引用会让
      // content.folders 同引用赋值不触发响应式 → blockSections getter 缓存旧快照（S6 新文件夹不渲染）
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
    if (cmd === "reorder_folders") {
      window.__calls.reorder_folders.push({
        folderIds: args.folderIds, parentId: args.parentId, beforeFolderId: args.beforeFolderId,
      });
      window.__opLog.push("reorder_folders");
      if (args.parentId === null || args.parentId === "root") {
        const allIds = TREE.nodes.map((n) => n.id);
        const coversAll = args.folderIds.length === allIds.length && allIds.every((id) => args.folderIds.includes(id));
        if (coversAll) {
          const byId = {};
          TREE.nodes.forEach((n) => (byId[n.id] = n));
          TREE.nodes = args.folderIds.map((id) => byId[id]);
        } else {
          const moving = args.folderIds.map((id) => TREE.nodes.find((n) => n.id === id)).filter(Boolean);
          TREE.nodes = TREE.nodes.filter((n) => !args.folderIds.includes(n.id));
          let at = args.beforeFolderId ? TREE.nodes.findIndex((n) => n.id === args.beforeFolderId) : -1;
          if (at < 0) at = TREE.nodes.length;
          TREE.nodes.splice(at, 0, ...moving);
        }
      }
      return Promise.resolve(null);
    }
    if (cmd === "move_folders") {
      window.__calls.move_folders.push(JSON.parse(JSON.stringify(args)));
      return Promise.resolve(1);
    }
    if (cmd === "create_folder") {
      window.__calls.create_folder.push({
        parentId: args.parentId, name: args.name, afterFolderId: args.afterFolderId, blockId: args.blockId,
      });
      window.__opLog.push("create_folder");
      const node = { id: "f9", name: args.name, count: 0, children: [] };
      if (args.blockId) node.block = args.blockId;
      if (args.parentId === null || args.parentId === "root") {
        const at = args.afterFolderId ? TREE.nodes.findIndex((n) => n.id === args.afterFolderId) : -1;
        if (at < 0) TREE.nodes.push(node);
        else TREE.nodes.splice(at + 1, 0, node);
      } else {
        const parent = TREE.nodes.find((n) => n.id === args.parentId);
        if (parent) {
          const at = args.afterFolderId ? parent.children.findIndex((n) => n.id === args.afterFolderId) : -1;
          if (at < 0) parent.children.push(node);
          else parent.children.splice(at + 1, 0, node);
        }
      }
      return Promise.resolve({ id: "f9", name: args.name, parent: args.parentId ?? null, order: 0 });
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
    # 5373 已被监听则复用；否则自起 pnpm dev，结束时杀掉（taskkill /T 杀整棵进程树）
    dev_proc = None
    if dev_server_up():
        print("5373 已在监听，复用现有 dev server")
    else:
        print("5373 未监听，启动 pnpm dev ...")
        dev_proc = subprocess.Popen(
            "pnpm dev", shell=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
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
            await asyncio.sleep(6)  # 等 vite 编译 + 首屏渲染

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, buttons, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons,
                    "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def key(t, key_name, code, vk):
                await send(ws, "Input.dispatchKeyEvent", {
                    "type": t, "key": key_name, "code": code, "windowsVirtualKeyCode": vk,
                }, session=sess)

            async def item_rect(fid, frac=0.5):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height * {frac}), h: b.height }};
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

            async def labels():
                return await ev("[...document.querySelectorAll('.block-tree[data-block-id=\"dir\"] .tree-item[data-folder-id] .label')].map((l) => l.textContent)")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：等三个树 item 渲染
            n = 0
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 3:
                    break
                await asyncio.sleep(0.5)
            print("树 item 数:", n)
            if not n or n < 3:
                print("树 item 未渲染，探针无法执行")
                return 2

            # ── S1：加载后显示序 = 存储序（custom 原序）──
            print("S1 初始显示序（custom）")
            ls = await labels()
            print("  显示序:", ls)
            check("S1 显示序 = [一丙, 二甲, 三乙]", ls == ["一丙", "二甲", "三乙"], str(ls))

            # ── S2：右键"二甲"→ 排序依据 → 当前层级 - 标题 A → Z ──
            print("S2 右键 f2(二甲) 开菜单 → hover 排序依据 → 点 标题 A → Z")
            c = await item_rect("f2")
            await mouse("mousePressed", c["x"], c["y"], 2, button="right")
            await mouse("mouseReleased", c["x"], c["y"], 0, button="right")
            await asyncio.sleep(0.6)
            menu_open = await ev("!!document.querySelector('.app-menu')")
            check("S2 右键菜单出现", bool(menu_open))
            sb = await menu_item_center("排序依据")
            if not sb:
                check("S2 菜单项「排序依据」存在", False)
            else:
                await mouse("mouseMoved", sb["x"], sb["y"], 0, button="none")
                await asyncio.sleep(0.5)  # flyout hover 展开
                it = await menu_item_center("当前层级 - 标题 A → Z")
                if not it:
                    check("S2 flyout 项「当前层级 - 标题 A → Z」存在", False)
                else:
                    await mouse("mousePressed", it["x"], it["y"], 1)
                    await mouse("mouseReleased", it["x"], it["y"], 0)
                    await asyncio.sleep(0.5)
            ls = await labels()
            print("  排序后显示序:", ls)
            check("S2 显示序变 [二甲, 三乙, 一丙]（zh 拼音序）", ls == ["二甲", "三乙", "一丙"], str(ls))

            # 截图：名称排序生效后的状态
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT)

            # 关菜单（keepOpen 不关菜单）：Esc，兜底点空白
            await key("rawKeyDown", "Escape", "Escape", 27)
            await key("keyUp", "Escape", "Escape", 27)
            await asyncio.sleep(0.4)
            if await ev("!!document.querySelector('.app-menu')"):
                await mouse("mousePressed", 900, 700, 1)
                await mouse("mouseReleased", 900, 700, 0)
                await asyncio.sleep(0.4)
            menu_closed = await ev("!document.querySelector('.app-menu')")
            check("S2 菜单已关闭（Esc/点空白）", bool(menu_closed))

            # ── S3：排序态拖拽"二甲"(显示第一位) → "三乙"下 1/3（zone after）──
            print("S3 拖 f2(二甲) → f3(三乙) 下 1/3")
            calls_before = await ev("window.__calls.reorder_folders.length")
            src = await item_rect("f2")
            dst = await item_rect("f3", frac=5 / 6)  # 下 1/3 = after
            print("  源:", src, " 落点:", dst)
            await mouse("mousePressed", src["x"], src["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", src["x"] + 30, src["y"], 1)  # 30px > 6px 阈值 → 进拖拽态
            await asyncio.sleep(SLEEP)
            dragging = await ev("document.body.classList.contains('dragging')")
            check("S3 拖拽中 body.dragging", bool(dragging))
            await mouse("mouseMoved", dst["x"], dst["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseReleased", dst["x"], dst["y"], 0)
            await asyncio.sleep(1.0)  # 等 drop 异步 invoke + refresh + toast
            calls = await ev("window.__calls.reorder_folders")
            moves = await ev("window.__calls.move_folders")
            new_calls = (calls or [])[calls_before:]
            print("  reorder_folders 新增:", json.dumps(new_calls, ensure_ascii=False), " move_folders:", json.dumps(moves, ensure_ascii=False))
            ok_call = (
                len(new_calls) == 1
                and new_calls[0]["folderIds"] == ["f3", "f2", "f1"]
                and new_calls[0]["parentId"] is None
                and new_calls[0]["beforeFolderId"] is None
            )
            check("S3 物化调用 reorder_folders([f3,f2,f1], null, null)", ok_call, json.dumps(new_calls, ensure_ascii=False))
            check("S3 未走 move_folders", not moves, json.dumps(moves, ensure_ascii=False))
            ls = await labels()
            print("  落点刷新后显示序:", ls)
            check("S3 refresh 后显示序 = [三乙, 二甲, 一丙]（不被名称排序盖回）", ls == ["三乙", "二甲", "一丙"], str(ls))
            toast = await ev("document.querySelector('.toast-stack')?.textContent?.trim() ?? null")
            print("  toast:", toast)
            check("S3 toast 出现「已调整」成功提示", bool(toast) and "已调整" in toast, str(toast))

            # ── S4 对照基线：custom 态拖"一丙"(f1) → "三乙"(f3) 上 1/3（zone before）──
            print("S4 基线（custom）：拖 f1(一丙) → f3(三乙) 上 1/3")
            calls_before = await ev("window.__calls.reorder_folders.length")
            src = await item_rect("f1")
            dst = await item_rect("f3", frac=1 / 6)  # 上 1/3 = before
            print("  源:", src, " 落点:", dst)
            await mouse("mousePressed", src["x"], src["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", src["x"] + 30, src["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", dst["x"], dst["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseReleased", dst["x"], dst["y"], 0)
            await asyncio.sleep(1.0)
            calls = await ev("window.__calls.reorder_folders")
            new_calls = (calls or [])[calls_before:]
            print("  reorder_folders 新增:", json.dumps(new_calls, ensure_ascii=False))
            ok_call = (
                len(new_calls) == 1
                and new_calls[0]["folderIds"] == ["f1"]
                and new_calls[0]["parentId"] is None
                and new_calls[0]["beforeFolderId"] == "f3"
            )
            check("S4 旧路径调用 reorder_folders([f1], null, \"f3\")", ok_call, json.dumps(new_calls, ensure_ascii=False))
            ls = await labels()
            print("  落点刷新后显示序:", ls)
            check("S4 显示序 = [一丙, 三乙, 二甲]", ls == ["一丙", "三乙", "二甲"], str(ls))

            # ── 共用：右键开菜单 / hover 排序依据 / 点菜单项 / 关菜单（S5/S6 复用）──
            async def right_click(fid):
                c = await item_rect(fid)
                await mouse("mousePressed", c["x"], c["y"], 2, button="right")
                await mouse("mouseReleased", c["x"], c["y"], 0, button="right")
                await asyncio.sleep(0.6)

            async def hover_sort_by():
                sb = await menu_item_center("排序依据")
                if not sb:
                    return False
                await mouse("mouseMoved", sb["x"], sb["y"], 0, button="none")
                await asyncio.sleep(0.5)  # flyout hover 展开
                return True

            async def click_menu(text):
                it = await menu_item_center(text)
                if not it:
                    return False
                await mouse("mousePressed", it["x"], it["y"], 1)
                await mouse("mouseReleased", it["x"], it["y"], 0)
                await asyncio.sleep(0.5)
                return True

            async def close_menu():
                await key("rawKeyDown", "Escape", "Escape", 27)
                await key("keyUp", "Escape", "Escape", 27)
                await asyncio.sleep(0.4)
                if await ev("!!document.querySelector('.app-menu')"):
                    await mouse("mousePressed", 900, 700, 1)
                    await mouse("mouseReleased", 900, 700, 0)
                    await asyncio.sleep(0.4)
                return await ev("!document.querySelector('.app-menu')")

            # 命名中新项无 data-folder-id 且 label 换成 .rename-input（FolderTree 命名态口径），
            # S6 显示序断言用宽松版：label 文本 ?? 输入框值
            async def labels_all():
                return await ev("[...document.querySelectorAll('.block-tree[data-block-id=\"dir\"] .tree-item')].map((el) => el.querySelector('.label')?.textContent ?? el.querySelector('.rename-input')?.value ?? null)")

            # ── S5：排序依据二级菜单首项「自定义」──
            print("S5 排序依据 flyout 首项「自定义」→ 切回存储序")
            await right_click("f1")
            check("S5 右键菜单出现", bool(await ev("!!document.querySelector('.app-menu')")))
            if not await hover_sort_by():
                check("S5 hover「排序依据」开 flyout", False)
            else:
                fl = await ev("[...document.querySelectorAll('.flyout-root .menu-item .label')].map((l) => l.textContent)")
                print("  flyout 项:", fl)
                check("S5 flyout 首项 = 自定义", bool(fl) and fl[0] == "自定义", str(fl))
                # 先切 A→Z 进入名称排序显示（否则「自定义」无可观测量）
                if not await click_menu("当前层级 - 标题 A → Z"):
                    check("S5 点「当前层级 - 标题 A → Z」", False)
                ls = await labels()
                check("S5 切 A→Z 后显示 [二甲, 三乙, 一丙]", ls == ["二甲", "三乙", "一丙"], str(ls))
                # keepOpen 菜单未关：再 hover 开 flyout 点「自定义」
                await hover_sort_by()
                if not await click_menu("自定义"):
                    check("S5 点「自定义」", False)
                ls = await labels()
                print("  切回自定义后显示序:", ls)
                check("S5 「自定义」切回存储序 [一丙, 三乙, 二甲]", ls == ["一丙", "三乙", "二甲"], str(ls))
            check("S5 菜单已关闭", bool(await close_menu()))

            # ── S6：名称排序下新建文件夹（createAndName 先物化再建）──
            print("S6 A→Z 排序态右键 f2(二甲) → 新建文件夹")
            # 前置：切 A→Z（显示 [二甲, 三乙, 一丙]）
            await right_click("f2")
            await hover_sort_by()
            await click_menu("当前层级 - 标题 A → Z")
            ls = await labels()
            check("S6 前置 A→Z 显示 [二甲, 三乙, 一丙]", ls == ["二甲", "三乙", "一丙"], str(ls))
            await close_menu()
            # 右键"二甲"→ 新建文件夹（一次性任务类：执行后菜单自关）
            ops_before = await ev("window.__opLog.length")
            await right_click("f2")
            if not await click_menu("新建文件夹"):
                check("S6 菜单项「新建文件夹」可点", False)
            await asyncio.sleep(1.2)  # 物化 → create → refresh + 命名态
            ops = (await ev("window.__opLog") or [])[ops_before:]
            rc = await ev("window.__calls.reorder_folders.slice(-1)[0] ?? null")
            cf = await ev("window.__calls.create_folder.slice(-1)[0] ?? null")
            print("  opLog 新增:", ops, " reorder:", json.dumps(rc, ensure_ascii=False), " create:", json.dumps(cf, ensure_ascii=False))
            check("S6 先 reorder_folders 后 create_folder", ops == ["reorder_folders", "create_folder"], str(ops))
            check(
                "S6 物化 reorder_folders([f2,f3,f1], null, null)",
                bool(rc) and rc["folderIds"] == ["f2", "f3", "f1"] and rc["parentId"] is None and rc["beforeFolderId"] is None,
                json.dumps(rc, ensure_ascii=False),
            )
            check(
                "S6 create_folder(parentId=null, afterFolderId=\"f2\")",
                bool(cf) and cf["parentId"] is None and cf["afterFolderId"] == "f2",
                json.dumps(cf, ensure_ascii=False),
            )
            ls = await labels_all()
            print("  新建后显示序:", ls)
            check("S6 显示序 = [二甲, 新建文件夹, 三乙, 一丙]（custom 不再盖回）", ls == ["二甲", "新建文件夹", "三乙", "一丙"], str(ls))
            sel = await ev("""(() => {
              const sel = [...document.querySelectorAll('.block-tree .tree-item.selected, .block-tree .tree-item.domain-selected')];
              return { total: sel.length, naming: sel.length === 1 && !!sel[0].querySelector('.rename-input'), cls: sel.map((el) => el.className) };
            })()""")
            print("  选中态:", json.dumps(sel, ensure_ascii=False))
            check("S6 单选新文件夹（selected+domain 合计 1 且为命名中新项）", bool(sel) and sel["total"] == 1 and sel["naming"], json.dumps(sel, ensure_ascii=False))
            inp = await ev("(() => { const i = document.querySelector('.block-tree .tree-item.naming .rename-input'); return i ? { value: i.value, visible: i.getBoundingClientRect().width > 0 } : null; })()")
            print("  命名输入:", inp)
            check("S6 新项命名输入态（.rename-input 可见，默认名）", bool(inp) and inp["visible"] and inp["value"] == "新建文件夹", str(inp))

            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT2, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT2)

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


sys.exit(asyncio.run(main()))
