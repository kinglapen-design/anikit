# CDP 探针：3.5 一轮 · 排序（28 稿排序方式点亮 + 自然排序 + 新增区 + 拖拽物化，2026-08-19）
#   S1 查看方式菜单：排序方式行点亮（无 disabled）、picker 可点
#   S2 排序下拉 = 8 项（自定义/名称/文件大小/时长/扩展名/创建时间/修改时间/导入时间）
#   S3 选「名称」→ 自然排序 [图一, 图2, 图10, 图甲]（中文数字/天干按序）；升降 segmented 可用
#   S4 降序 → [图甲, 图10, 图2, 图一]
#   S5 「文件大小」升序 → [图10(50), 图甲(100), 图2(200), 图一(300)]
#   S6 「时长」升序 → [图一(1), 图10(5), 图2(null), 图甲(null)]（空值恒排尾，平局名称兜底）
#   S7 新增区：时长排序下 Ctrl+C/Ctrl+V 链接粘贴新项（图0，duration 9 应归第 3 位）→ 置顶第 1
#   S8 切走再切回（f2 → f1）→ 新增区归位：[图一, 图10, 图0, 图2, 图甲]
#   S9 拖拽物化：时长排序下拖 图10 到 图一 前 → reorder_placements 全层 5 项序列 + beforeItem null
#      + 排序切回自定义（查看方式菜单回「自定义」）
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-sort-probe.py
import asyncio
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
SHOT = "tools/style-preview/sort-probe.png"
SLEEP = 0.35

# 桩：f1 角色（4 素材，存储序故意乱摆）/ f2 场景（空，供切视图）。
# 名称键覆盖：阿拉伯数字串（图2/图10）+ 中文数字（图一）+ 天干（图甲）；
# duration 两个 null 验空值排尾。list_placements 每次深拷贝当前 PLACEMENTS（link 桩会追加新项）。
STUB = r"""
window.__calls = { reorder_placements: [], duplicate_placements: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 4, children: [] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mk = (assetId, name, size, duration) => ({
  assetId, folderId: "f1", name, w: 800, h: 600, ext: "png", thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration, bpm: null, importedFrom: null,
  size, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T10:00:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
});
const PLACEMENTS = [
  mk("AN-2", "图2.png", 200, null),
  mk("AN-10", "图10.png", 50, 5),
  mk("AN-1", "图一.png", 300, 1),
  mk("AN-J", "图甲.png", 100, null),
];
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 2 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "reorder_placements") {
      window.__calls.reorder_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "duplicate_placements") {
      window.__calls.duplicate_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      // 模拟副本入库：新资产「图0.png」（duration 9——按时长应归第 3 位，新增区应置顶）
      PLACEMENTS.push(mk("AN-9", "图0.png", 80, 9));
      return Promise.resolve(["AN-9"]);
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


async def main():
    fails = []

    # dev server 探测（未起则自起，结束杀）
    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(
            ["pnpm", "dev"], shell=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "about:blank",
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

            async def click(x, y):
                await mouse("mousePressed", x, y, 1)
                await mouse("mouseReleased", x, y, 0, clicks=1)
                await asyncio.sleep(SLEEP)

            async def key(k, code, vk, ctrl=False, shift=False):
                m = (2 if ctrl else 0) + (8 if shift else 0)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": k, "code": code, "windowsVirtualKeyCode": vk, "modifiers": m}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": k, "code": code, "windowsVirtualKeyCode": vk, "modifiers": m}, session=sess)
                await asyncio.sleep(SLEEP)

            # 当前内容区卡片名称序（.asset-card 按视觉阅读序 = DOM 序）
            async def card_names():
                return await ev("""
                  [...document.querySelectorAll('.asset-card .title')].map((el) => el.textContent.trim())
                """)

            async def tree_item(fid):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：等树渲染，进 f1 视口
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            print("树 item 数:", n)
            if not n or n < 2:
                print("树未渲染，探针无法执行")
                return 2
            f1 = await tree_item("f1")
            await click(f1["x"], f1["y"])
            for _ in range(20):
                cards = await card_names()
                if cards and len(cards) == 4:
                    break
                await asyncio.sleep(0.5)
            print("f1 卡片（存储序）:", cards)
            check("S0 f1 四卡渲染（存储序乱摆）", cards == ["图2.png", "图10.png", "图一.png", "图甲.png"], str(cards))

            # ── S1/S2：查看方式菜单 → 排序下拉 8 项 ──
            print("S1 查看方式菜单")
            btn = await ev("""
              (() => {
                // 工具栏按钮组：前五个占位 disabled，查看方式 = 组内唯一可用按钮
                //（限定 .panel-toolbar——侧栏"新增"等同款可用按钮在其他 .btn-group）
                const b = document.querySelector('.panel-toolbar .btn-group .icon-btn:not([disabled])');
                if (!b) return null;
                const r = b.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if not btn:
                check("S1a 查看方式按钮存在", False, "未找到按钮")
                return 2
            await click(btn["x"], btn["y"])
            await asyncio.sleep(0.6)
            menu_count = await ev("document.querySelectorAll('.app-menu').length")
            row_texts = await ev("[...document.querySelectorAll('.app-menu .row.tall')].map((r) => r.textContent.trim().slice(0, 12))")
            menu_html = await ev("document.querySelector('.app-menu')?.textContent?.slice(0, 120) ?? null")
            print("  菜单数:", menu_count, "tall 行:", row_texts, "菜单文本:", menu_html)
            s1a = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                if (!sortRow) return { row: false };
                return {
                  row: true,
                  disabled: sortRow.classList.contains('disabled'),
                  picker: !!sortRow.querySelector('.picker:not([disabled])'),
                };
              })()
            """)
            print("  排序行状态:", s1a)
            check("S1b 排序行点亮（无 disabled + picker 可用）",
                  bool(s1a) and s1a.get("row") and not s1a.get("disabled") and s1a.get("picker"), str(s1a))

            # 点排序 picker → 8 项
            picker_pos = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                const p = sortRow?.querySelector('.picker');
                if (!p) return null;
                const r = p.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await click(picker_pos["x"], picker_pos["y"])
            items = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((el) => el.textContent.trim())")
            print("  排序选项:", items)
            expect8 = ["自定义", "名称", "文件大小", "时长", "扩展名", "创建时间", "修改时间", "导入时间"]
            check("S2 排序下拉 8 项", items == expect8, str(items))

            # 选「名称」
            async def pick_menu(label):
                pos = await ev(f"""
                  (() => {{
                    const items = [...document.querySelectorAll('.app-menu .menu-item')];
                    const it = items.find((el) => el.textContent.trim() === "{label}");
                    if (!it) return null;
                    const r = it.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if pos:
                    await click(pos["x"], pos["y"])

            async def ensure_menu():
                # 查看方式菜单已开则复用（选下拉项只关 picker 不关主菜单；toggle 再点会关）
                opened = await ev("""
                  !!([...document.querySelectorAll('.app-menu .row.tall')].find((r) => r.textContent.includes('排序方式')))
                """)
                if not opened:
                    await click(btn["x"], btn["y"])

            async def open_sort_picker():
                await ensure_menu()
                p = await ev("""
                  (() => {
                    const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                    const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                    const pk = sortRow?.querySelector('.picker');
                    if (!pk) return null;
                    const r = pk.getBoundingClientRect();
                    return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
                  })()
                """)
                await click(p["x"], p["y"])  # 开排序下拉

            await pick_menu("名称")
            await asyncio.sleep(SLEEP)
            names = await card_names()
            print("S3 名称排序:", names)
            check("S3 名称自然排序（一<2<10<甲）", names == ["图一.png", "图2.png", "图10.png", "图甲.png"], str(names))
            seg_disabled = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                return sortRow ? sortRow.querySelector('.app-seg')?.classList.contains('disabled') ?? null : null;
              })()
            """)
            # 菜单已关（选择后关闭）——升降禁用态改在 S4 开菜单时查；此处先验排序结果
            # S4 降序
            print("S4 降序")
            await ensure_menu()
            seg_state = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                return sortRow ? sortRow.querySelector('.app-seg')?.classList.contains('disabled') ?? null : null;
              })()
            """)
            check("S4a 名称排序下升降切换可用", seg_state is False, str(seg_state))
            desc_pos = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                const items = [...(sortRow?.querySelectorAll('.app-seg .item') ?? [])];
                const d = items.find((el) => el.textContent.trim() === '降序');
                if (!d) return null;
                const r = d.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await click(desc_pos["x"], desc_pos["y"])
            await key("Escape", "Escape", 27)  # 关菜单
            names = await card_names()
            print("  名称降序:", names)
            check("S4b 名称降序", names == ["图甲.png", "图10.png", "图2.png", "图一.png"], str(names))

            # S5 文件大小升序（先切回升序）
            print("S5 文件大小")
            await open_sort_picker()
            await pick_menu("文件大小")
            # 切回升序
            await ensure_menu()
            asc_pos = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                const items = [...(sortRow?.querySelectorAll('.app-seg .item') ?? [])];
                const a = items.find((el) => el.textContent.trim() === '升序');
                if (!a) return null;
                const r = a.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await click(asc_pos["x"], asc_pos["y"])
            await key("Escape", "Escape", 27)
            names = await card_names()
            print("  大小升序:", names)
            check("S5 文件大小升序", names == ["图10.png", "图甲.png", "图2.png", "图一.png"], str(names))

            # S6 时长升序（空值排尾 + 平局名称兜底）
            print("S6 时长排序")
            await open_sort_picker()
            await pick_menu("时长")
            names = await card_names()
            print("  时长升序:", names)
            check("S6 时长升序空值排尾", names == ["图一.png", "图10.png", "图2.png", "图甲.png"], str(names))

            # S7 新增区：Ctrl+C 复制图2 → Ctrl+Shift+V 副本粘贴 → 新项图0 置顶
            print("S7 新增区（副本粘贴置顶）")
            await key("Escape", "Escape", 27)  # 先关可能残留的查看方式菜单（开着时首击只关菜单不选中）
            card2 = await ev("""
              (() => {
                const el = document.querySelector('.asset-card[data-key="AN-2|f1"]');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await click(card2["x"], card2["y"])  # 选中图2
            await key("c", "KeyC", 67, ctrl=True)
            await key("v", "KeyV", 86, ctrl=True, shift=True)  # 副本粘贴（3.4：Ctrl+Shift+V）
            await asyncio.sleep(0.8)  # duplicate + refresh
            names = await card_names()
            print("  粘贴后（图0 应置顶）:", names)
            check("S7 新增项置顶（不归时长位）", names == ["图0.png", "图一.png", "图10.png", "图2.png", "图甲.png"], str(names))

            # S8 切走再切回 → 归位
            print("S8 切视图归位")
            f2 = await tree_item("f2")
            await click(f2["x"], f2["y"])
            await asyncio.sleep(SLEEP)
            f1b = await tree_item("f1")
            await click(f1b["x"], f1b["y"])
            await asyncio.sleep(SLEEP)
            names = await card_names()
            print("  归位后:", names)
            check("S8 新增区归位（图0 按时长归第 3 位）",
                  names == ["图一.png", "图10.png", "图0.png", "图2.png", "图甲.png"], str(names))

            # S9 拖拽物化：时长排序下拖 图10 到 图一 前 → 全层重写 + 切回自定义
            print("S9 拖拽物化")
            src = await ev("""
              (() => {
                const el = document.querySelector('.asset-card[data-key="AN-10|f1"]');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            dst = await ev("""
              (() => {
                const el = document.querySelector('.asset-card[data-key="AN-1|f1"]');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                // 间隙判定 = 卡片左半区即"该卡前"（makeGridResolver：x < left + width/2）
                return { x: Math.round(r.left + 10), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            # 拖拽：按下 → 超阈值 → 移到目标卡片左半（前间隙判定区）→ 停顿 → 松手
            await mouse("mousePressed", src["x"], src["y"], 1)
            await asyncio.sleep(0.1)
            await mouse("mouseMoved", src["x"] + 10, src["y"], 1)
            await asyncio.sleep(0.1)
            await mouse("mouseMoved", dst["x"], dst["y"], 1)
            await asyncio.sleep(0.5)
            await mouse("mouseReleased", dst["x"], dst["y"], 0)
            await asyncio.sleep(0.8)
            calls = await ev("window.__calls.reorder_placements")
            print("  reorder 调用:", json.dumps(calls, ensure_ascii=False))
            seq_ok = False
            if calls:
                last = calls[-1]
                seq = [f'{it["assetId"]}' for it in last.get("items", [])]
                seq_ok = (seq == ["AN-10", "AN-1", "AN-9", "AN-2", "AN-J"]
                          and last.get("beforeItem") is None
                          and last.get("targetFolder") == "f1")
            check("S9a 物化 = 全层可见序 + 拖拽组落点重写（beforeItem null）", seq_ok,
                  json.dumps(calls[-1] if calls else None, ensure_ascii=False))
            # 排序切回自定义
            await ensure_menu()
            label = await ev("""
              (() => {
                const rows = [...document.querySelectorAll('.app-menu .row.tall')];
                const sortRow = rows.find((r) => r.textContent.includes('排序方式'));
                return sortRow?.querySelector('.picker .val')?.textContent.trim() ?? null;
              })()
            """)
            check("S9b 物化后排序切回自定义", label == "自定义", str(label))
            await key("Escape", "Escape", 27)

            # 截图留档
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(__import__("base64").b64decode(shot["result"]["data"]))
            print("截图:", SHOT)
    finally:
        edge.terminate()
        if spawned:
            spawned.terminate()

    print()
    if fails:
        print(f"{len(fails)} 项失败：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


sys.exit(asyncio.run(main()))
