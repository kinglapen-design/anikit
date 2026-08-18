# CDP 探针：3.5 二轮 · 分组（33/34/12/23 稿 + 分组渲染，2026-08-19）
#   S1 进 f1 → 分组标题渲染：未分组在头 + 图片组 + 动图组（count 各对）
#   S2 归组正确：卡片序 = [文本文档（未分组）, 图一, 图2（图片组）, 动图（动图组）]
#   S3 折叠图片组 → 图一/图2 卡片消失，组标题仍在
#   S4 sticky：滚动到中部 → 悬浮置顶标题出现
#   S5 工具栏分组管理钮 → 33 稿弹窗（2 组块 + 添加分组 + 确认/取消）
#   S6 组标题 group-menu → 12 稿 7 项菜单
#   S7 菜单「上移」（动图组）→ set_folder_groups 提交 [动图组, 图片组]
#   S8 快速访问「全部」→ 分组管理钮禁用
#   S9 完全相同预检 → 23 稿错误弹窗（无法保存，存在完全相同的分组！）
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-group-probe.py
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
PORT = 9359
URL = "http://localhost:5373"
SHOT = "tools/style-preview/group-probe.png"
SLEEP = 0.35

# 桩：f1 角色（4 素材 + 2 组）/ f2 场景（空）。分组规则直接挂在 FolderNode.groups。
STUB = r"""
window.__calls = { set_folder_groups: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 4, children: [], groups: [
      { id: "g1", name: "图片组", rules: [{ kind: "ext", values: ["png"] }] },
      { id: "g2", name: "动图组", rules: [{ kind: "type", values: ["anim"] }] },
    ] },
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
  mk("AN-1", "图一.png", "png"),
  mk("AN-2", "动图.gif", "gif", { frameCount: 3, animated: true }),
  mk("AN-3", "文本文档.md", "md", { w: null, h: null }),
  mk("AN-4", "图2.png", "png"),
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
    if (cmd === "set_folder_groups") {
      window.__calls.set_folder_groups.push(JSON.parse(JSON.stringify(args ?? {})));
      const f = TREE.nodes.find((n) => n.id === args.folderId);
      if (f) f.groups = args.groups.length ? args.groups : undefined;
      return Promise.resolve(null);
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
        # 页面 console 转发（探针调试：菜单不开等问题多为 JS 异常）
        if raw.get("method") == "Runtime.consoleAPICalled":
            args = raw.get("params", {}).get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            print(f"  [console.{raw['params'].get('type')}]", text[:200])
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw.get("params", {}).get("exceptionDetails", {})
            print("  [exception]", str(d.get("text", ""))[:200], str(d.get("exception", {}).get("description", ""))[:300])


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（2026-08-19 踩坑：vite node 进程脱离 pnpm 壳的进程树，
    taskkill /T 杀壳后 5373 仍被占——端口直杀兜底）"""
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

            async def click(x, y):
                await mouse("mousePressed", x, y, 1)
                await mouse("mouseReleased", x, y, 0, clicks=1)
                await asyncio.sleep(SLEEP)

            async def key(k, code, vk):
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": k, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": k, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await asyncio.sleep(SLEEP)

            async def click_sel(sel, desc):
                pos = await ev(f"""
                  (() => {{
                    const el = document.querySelector('{sel}');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if not pos:
                    return False
                await click(pos["x"], pos["y"])
                return True

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：进 f1
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            if not await click_sel('.tree-item[data-folder-id="f1"]', "f1"):
                print("树未渲染")
                return 2
            for _ in range(20):
                cards = await ev("[...document.querySelectorAll('.asset-card .title')].map((el) => el.textContent.trim())")
                if cards and len(cards) == 4:
                    break
                await asyncio.sleep(0.5)

            # ── S1/S2：分组标题 + 归组 ──
            print("S1/S2 分组渲染")
            headers = await ev("""
              [...document.querySelectorAll('.group-header .section-header .title')]
                .map((el) => el.textContent.trim())
            """)
            counts = await ev("""
              [...document.querySelectorAll('.group-header .section-header .count')]
                .map((el) => el.textContent.trim())
            """)
            print("  组标题:", headers, "计数:", counts)
            check("S1 组标题序（未分组在头）", headers == ["未分组", "图片组", "动图组"], str(headers))
            check("S1b 组计数", counts == ["- 1", "- 2", "- 1"], str(counts))
            cards = await ev("[...document.querySelectorAll('.asset-card .title')].map((el) => el.textContent.trim())")
            print("  卡片序:", cards)
            check("S2 组拼接序（未分组 → 图片组 → 动图组）",
                  cards == ["文本文档.md", "图一.png", "图2.png", "动图.gif"], str(cards))

            # ── S3：折叠图片组 ──
            print("S3 折叠图片组")
            ok = await click_sel('.group-header:nth-of-type(2) .section-header .fold-area', "折叠图片组")
            if not ok:
                # nth-of-type 对 absolute div 不适用时的退路：按标题文本找
                pos = await ev("""
                  (() => {
                    const hs = [...document.querySelectorAll('.group-header')];
                    const h = hs.find((x) => x.textContent.includes('图片组'));
                    const fa = h?.querySelector('.fold-area');
                    if (!fa) return null;
                    const r = fa.getBoundingClientRect();
                    return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
                  })()
                """)
                await click(pos["x"], pos["y"])
            await asyncio.sleep(0.5)
            cards = await ev("[...document.querySelectorAll('.asset-card .title')].map((el) => el.textContent.trim())")
            headers = await ev("[...document.querySelectorAll('.group-header .section-header .title')].map((el) => el.textContent.trim())")
            print("  折叠后卡片:", cards, "组标题:", headers)
            check("S3 折叠后图片组卡片消失、标题仍在",
                  cards == ["文本文档.md", "动图.gif"] and headers == ["未分组", "图片组", "动图组"], str(cards))
            # 再展开还原
            pos = await ev("""
              (() => {
                const hs = [...document.querySelectorAll('.group-header')];
                const h = hs.find((x) => x.textContent.includes('图片组'));
                const fa = h?.querySelector('.fold-area');
                if (!fa) return null;
                const r = fa.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await click(pos["x"], pos["y"])

            # ── S5：工具栏分组管理钮 → 33 稿弹窗 ──
            print("S5 分组管理弹窗")
            ok = await click_sel(".panel-toolbar .btn-group .icon-btn:nth-last-child(2)", "分组管理钮")
            # 按钮组倒数第二 = 分组管理（最后是查看方式）——确认没有右栏收起钮干扰
            await asyncio.sleep(0.5)
            dlg = await ev("""
              (() => {
                const m = document.querySelector('.group-manage');
                if (!m) return null;
                return {
                  blocks: m.querySelectorAll('.group-block').length,
                  title: m.querySelector('.titlebar .title')?.textContent?.trim() ?? null,
                  addBtn: !!m.querySelector('.add-btn'),
                };
              })()
            """)
            print("  弹窗:", dlg)
            check("S5 33 稿弹窗（2 组块 + 标题 + 添加钮）",
                  bool(dlg) and dlg.get("blocks") == 2 and dlg.get("title") == "分组管理" and dlg.get("addBtn"), str(dlg))

            # ── S9：完全相同预检 → 23 稿错误（多规则组场景——单规则组会被互斥转移拦截） ──
            # 构造：组2 加规则 ext:gif → [type:anim, ext:gif]；组1 改 [type:anim, ext:gif] → 完全相同
            print("S9 完全相同 → 23 稿错误（多规则组）")

            async def group_block_pos(i, sel):
                return await ev(f"""
                  (() => {{
                    const blocks = [...document.querySelectorAll('.group-manage .group-block')];
                    const el = blocks[{i}]?.querySelector('{sel}');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)

            async def pick_kind(i, label):
                p = await group_block_pos(i, ".kind-pick")
                await click(p["x"], p["y"])
                await asyncio.sleep(0.2)
                it = await ev(f"""
                  (() => {{
                    const items = [...document.querySelectorAll('.app-menu .menu-item')];
                    const m = items.find((el) => el.textContent.trim() === '{label}');
                    if (!m) return null;
                    const r = m.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if it:
                    await click(it["x"], it["y"])

            async def pick_chip(i, label):
                p = await ev(f"""
                  (() => {{
                    const blocks = [...document.querySelectorAll('.group-manage .group-block')];
                    const chips = [...(blocks[{i}]?.querySelectorAll('.chip') ?? [])];
                    const c = chips.find((el) => el.textContent.trim() === '{label}');
                    if (!c) return null;
                    const r = c.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if p:
                    await click(p["x"], p["y"])

            # 组2 添加新规则（默认 ext）→ 选 gif
            p = await group_block_pos(1, ".add-rule")
            await click(p["x"], p["y"])
            await asyncio.sleep(0.3)
            # 组2 第二行规则选 gif（chips 按行查——第二规则行）
            p = await ev("""
              (() => {
                const blocks = [...document.querySelectorAll('.group-manage .group-block')];
                const rows = [...(blocks[1]?.querySelectorAll('.rule-row') ?? [])];
                const chips = [...(rows[1]?.querySelectorAll('.chip') ?? [])];
                const c = chips.find((el) => el.textContent.trim() === 'gif');
                if (!c) return null;
                const r = c.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if p:
                await click(p["x"], p["y"])
            # 组1 kind 改「类型」→ chip 选「动图」
            await pick_kind(0, "类型")
            await pick_chip(0, "动图")
            # 组1 添加新规则（ext）→ 选 gif
            p = await group_block_pos(0, ".add-rule")
            await click(p["x"], p["y"])
            await asyncio.sleep(0.3)
            p = await ev("""
              (() => {
                const blocks = [...document.querySelectorAll('.group-manage .group-block')];
                const rows = [...(blocks[0]?.querySelectorAll('.rule-row') ?? [])];
                const chips = [...(rows[1]?.querySelectorAll('.chip') ?? [])];
                const c = chips.find((el) => el.textContent.trim() === 'gif');
                if (!c) return null;
                const r = c.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if p:
                await click(p["x"], p["y"])
            # 草稿状态核对（调试用）
            draft_state = await ev("""
              [...document.querySelectorAll('.group-manage .group-block')].map((b) =>
                [...b.querySelectorAll('.rule-row')].map((r) => r.textContent.trim().slice(0, 30)))
            """)
            print("  草稿规则:", draft_state)
            # 确认
            pos = await ev("""
              (() => {
                const m = document.querySelector('.group-manage');
                const btns = [...(m?.querySelectorAll('.actions .app-btn') ?? [])];
                const c = btns.find((el) => el.textContent.trim() === '确认');
                if (!c) return null;
                const r = c.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if pos:
                await click(pos["x"], pos["y"])
            await asyncio.sleep(0.5)
            err = await ev("""
              (() => {
                const m = document.querySelector('.app-modal.alert');
                return m ? (m.querySelector('.title')?.textContent?.trim() ?? null) : null;
              })()
            """)
            print("  错误弹窗标题:", err)
            check("S9 完全相同拦截（23 稿）", err == "无法保存，存在完全相同的分组！", str(err))
            # 关错误窗（修改分组）+ 关管理弹窗（取消）
            pos = await ev("""
              (() => {
                const m = document.querySelector('.app-modal.alert');
                const btns = [...(m?.querySelectorAll('.actions .app-btn') ?? [])];
                const c = btns.find((el) => el.textContent.trim() === '修改分组');
                if (!c) return null;
                const r = c.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if pos:
                await click(pos["x"], pos["y"])
            pos = await ev("""
              (() => {
                const m = document.querySelector('.group-manage');
                const btns = [...(m?.querySelectorAll('.actions .app-btn') ?? [])];
                const c = btns.find((el) => el.textContent.trim() === '取消');
                if (!c) return null;
                const r = c.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if pos:
                await click(pos["x"], pos["y"])

            # ── S6/S7：组标题菜单 → 上移 ──
            print("S6/S7 分组菜单")
            pos = await ev("""
              (() => {
                const hs = [...document.querySelectorAll('.group-header')];
                const h = hs.find((x) => x.textContent.includes('动图组'));
                const btn = h?.querySelector('.group-menu-btn');
                if (!btn) return null;
                const r = btn.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if not pos:
                check("S6 group-menu 按钮存在", False, "未找到")
            else:
                # 调试：挂事件标记（按钮 capture + document capture），定位点击是否到达按钮
                await ev("""
                  window.__clicks = [];
                  document.querySelectorAll('.group-menu-btn').forEach((b) =>
                    b.addEventListener('click', () => window.__clicks.push('btn'), true));
                  document.addEventListener('click', (e) =>
                    window.__clicks.push('doc:' + ((e.target.className || e.target.tagName || '').toString().slice(0, 40))), true);
                  'ok'
                """)
                # 调试：按钮 rect + 该坐标实际命中元素
                dbg = await ev(f"""
                  (() => {{
                    const hs = [...document.querySelectorAll('.group-header')];
                    const h = hs.find((x) => x.textContent.includes('动图组'));
                    const btn = h?.querySelector('.group-menu-btn');
                    if (!btn) return null;
                    const r = btn.getBoundingClientRect();
                    const hit = document.elementFromPoint({pos["x"]}, {pos["y"]});
                    return {{ rect: {{ x: r.x, y: r.y, w: r.width, h: r.height }},
                              hit: hit ? (hit.className || hit.tagName).toString().slice(0, 60) : null }};
                  }})()
                """)
                print("  按钮 rect/命中:", dbg)
                await click(pos["x"], pos["y"])
                clicks = await ev("window.__clicks")
                menu_n = await ev("document.querySelectorAll('.app-menu').length")
                items = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((el) => el.querySelector('.label')?.textContent.trim() ?? el.textContent.trim())")
                print("  点击事件链:", clicks, "菜单数:", menu_n, "菜单项:", items)
                check("S6 12 稿 7 项菜单",
                      items == ["跳转", "新建分组", "编辑分组", "展开/折叠", "上移", "下移", "删除"], str(items))
                # 点「上移」（动图组 → 移到图片组前）
                up = await ev("""
                  (() => {
                    const items = [...document.querySelectorAll('.app-menu .menu-item')];
                    const it = items.find((el) => el.textContent.trim() === '上移');
                    if (!it) return null;
                    const r = it.getBoundingClientRect();
                    return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
                  })()
                """)
                if up:
                    await click(up["x"], up["y"])
                await asyncio.sleep(0.6)
                calls = await ev("window.__calls.set_folder_groups")
                print("  set_folder_groups 调用:", json.dumps(calls, ensure_ascii=False))
                ok = bool(calls) and [g["name"] for g in calls[-1].get("groups", [])] == ["动图组", "图片组"]
                check("S7 上移提交 [动图组, 图片组]", ok, json.dumps(calls[-1] if calls else None, ensure_ascii=False))
                # 上移后标题序
                headers = await ev("[...document.querySelectorAll('.group-header .section-header .title')].map((el) => el.textContent.trim())")
                check("S7b 上移后组序", headers == ["未分组", "动图组", "图片组"], str(headers))

            # ── S8：快速访问禁用 ──
            print("S8 快速访问分组管理钮禁用")
            # 侧栏快速访问「全部」= .block.quick 首个 tree-item（无 data-folder-id，LibraryView）
            pos = await ev("""
              (() => {
                const el = document.querySelector('.block.quick .tree-item');
                if (!el) return null;
                const r = el.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            if pos:
                await click(pos["x"], pos["y"])
            await asyncio.sleep(0.5)
            disabled = await ev("""
              (() => {
                const btns = [...document.querySelectorAll('.panel-toolbar .btn-group button.icon-btn')];
                const gm = btns[btns.length - 2]; // 分组管理 = 倒数第二（最后查看方式）
                return gm ? gm.disabled : null;
              })()
            """)
            check("S8 快速访问视图分组管理钮禁用", disabled is True, str(disabled))

            # ── S4：sticky 置顶（滚动后悬浮标题出现；transform 容器内 sticky 无效 → 悬浮层实现） ──
            print("S4 sticky 置顶")
            # 切回 f1（S8 在全部视图）
            await click_sel('.tree-item[data-folder-id="f1"]', "f1")
            await asyncio.sleep(0.5)
            # 滚轮向下（卡片区中心）→ 滚过第一组标题
            await send(ws, "Input.dispatchMouseEvent", {
                "type": "mouseWheel", "x": 700, "y": 500, "deltaX": 0, "deltaY": 800,
            }, session=sess)
            await asyncio.sleep(0.6)
            sticky = await ev("""
              (() => {
                const el = document.querySelector('.sticky-group-header .section-header .title');
                return el ? el.textContent.trim() : null;
              })()
            """)
            print("  悬浮标题:", sticky)
            check("S4 滚动后悬浮置顶标题出现", sticky is not None, str(sticky))

            # 截图留档（滚动态，含悬浮标题）
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图:", SHOT)
    finally:
        edge.terminate()
        if spawned:
            # Windows：shell=True 的 pnpm 壳 terminate 只杀壳、node/vite 子树存活（2026-08-19 踩坑）；
            # taskkill /T 杀树 + 按端口直杀兜底（vite node 会脱离 cmd 壳进程树，实测 /T 后 5373 仍占）
            subprocess.run(["taskkill", "/PID", str(spawned.pid), "/T", "/F"], capture_output=True)
            kill_listening_port(5373)

    print()
    if fails:
        print(f"{len(fails)} 项失败：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


sys.exit(asyncio.run(main()))
