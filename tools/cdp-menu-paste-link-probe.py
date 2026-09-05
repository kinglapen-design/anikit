# CDP 实测：2026-09-05 走查「上下文菜单」——复制/粘贴标签·描述 + linkList 二级（独立短链探针）。
# 自 cdp-context-menu-probe.py S3/S4 段拆出：主探针长操作链后 CDP 合成右键偶发不派发 contextmenu
#（事件连 document capture 都收不到，非业务问题——短链独立跑稳定）。
# P1 复制标签（A1 = [走路, 跑步]）→ B2 右键复制数据多出「粘贴标签」→ 粘贴 = 合并追加（set_asset_props
#    [待机, 走路, 跑步] 实证）；P2 描述同路径（按标题去重）；P3 链接状态「将此文件设为原始文件」出现
#   （origin 卡禁用）；P4「打开文件链接所在位置」二级 = linkList 双行（与徽章 hover 弹窗 LinkListPopover 同构）。
import asyncio
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9351
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { set_asset_props: [], set_asset_descriptions: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 2, children: [] },
    { id: "f2", name: "场景", count: 1, children: [] },
  ],
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
];
const DETAILS = {
  A1: { tags: ["走路", "跑步"], descriptions: [{ title: "动作", content: "基础动作" }, { title: "背景", content: "蓝调" }] },
  B2: { tags: ["待机"], descriptions: [{ title: "动作", content: "已有的" }] },
};
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    if (cmd === "set_asset_props") {
      window.__calls.set_asset_props.push(JSON.parse(JSON.stringify(args ?? {})));
      if (DETAILS[args.assetId]) DETAILS[args.assetId].tags = args.props;
      return Promise.resolve(null);
    }
    if (cmd === "set_asset_descriptions") {
      window.__calls.set_asset_descriptions.push(JSON.parse(JSON.stringify(args ?? {})));
      if (DETAILS[args.assetId]) DETAILS[args.assetId].descriptions = args.descriptions;
      return Promise.resolve(null);
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
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 2, folderCount: 2 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
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
                # buttons = 事件后的按键状态：release/move 恒 0（release 带 1 = 左键假按住，后续右键被吞）
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "buttons": 0 if t in ("mouseReleased", "mouseMoved") else (2 if button == "right" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            def js_find(label, idx=0):
                return f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item') ?? []].find((x) => x.querySelector('.label')?.textContent === '{label}')"

            def el_center(sel_expr):
                return f"(() => {{ const el = {sel_expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()"

            async def js_click(expr):
                r = await ev(expr)
                if r is None:
                    return None
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def js_rclick(expr):
                r = await ev(expr)
                if r is None:
                    return None
                # 先显式 move 同步悬停态再 press——连续「点菜单项→右键」链中 CDP 合成右键
                # 偶发不派发 contextmenu（主探针 2026-09-05 实证）；move 预热 + 菜单未出重试一次
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.2)
                await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                await asyncio.sleep(0.6)
                if await ev("document.querySelectorAll('.app-menu').length === 0"):
                    await mouse("mouseMoved", r["x"] - 4, r["y"] - 4)
                    await asyncio.sleep(0.2)
                    await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                    await asyncio.sleep(0.6)
                return r

            async def js_hover(expr):
                r = await ev(expr)
                if r is None:
                    return None
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def menu_labels(idx=0):
                return await ev(f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)") or []

            async def wait_toast_gone():
                # 复制/粘贴 toast 本体可点会挡命中（居中堆叠盖住中间卡片吃掉右键，2026-09-05 实证）——
                # headless 后台 tab 计时器节流，3s 自然消失不可靠；主动点各条关闭钮清场
                for _ in range(14):
                    if not await ev("!!document.querySelector('.toast')"):
                        return
                    await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
                    await asyncio.sleep(0.4)  # 等退场动画

            async def esc_close():
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await asyncio.sleep(0.4)

            # f1 视口（树 plain click = openView）
            await js_click(el_center("document.querySelector('[data-folder-id=\"f1\"]')"))
            await asyncio.sleep(0.8)
            check("P0 f1 视口渲两卡", await ev("document.querySelectorAll('.asset-card').length === 2"))

            # —— P1 复制标签 → 粘贴（合并追加）——
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_hover(el_center(js_find("复制数据")))
            await js_click(el_center(js_find("复制标签", 1)))  # 写槽 [走路, 跑步]；菜单关
            await wait_toast_gone()
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"B2|f1\"]')"))  # 右键未选中项先单选
            # 粘贴项 2026-09-05 走查二轮挪一级（跟在「复制文件」同组下方，不放复制数据二级）
            labels = await menu_labels(0)
            check("P1-1 复制后一级多出「粘贴标签」", "粘贴标签" in labels, str(labels))
            check("P1-1b 一级位置：在「复制文件」之后", await ev("(() => { const ls = [...document.querySelectorAll('.app-menu')[0].querySelectorAll('.menu-item .label')].map((x) => x.textContent); return ls.indexOf('粘贴标签') > ls.indexOf('复制文件'); })()"))
            check("P1-2 无「粘贴描述」（未复制过）", "粘贴描述" not in labels, str(labels))
            await js_click(el_center(js_find("粘贴标签", 0)))
            check("P1-3 粘贴合并追加（B2：待机 + 走路/跑步）", await ev("(() => { const c = window.__calls.set_asset_props; return c.length === 1 && c[0].assetId === 'B2' && JSON.stringify(c[0].props) === '[\"待机\",\"走路\",\"跑步\"]'; })()"), str(await ev("JSON.stringify(window.__calls.set_asset_props)")))
            await wait_toast_gone()

            # —— P2 复制描述 → 粘贴（同标题去重：B2 已有「动作」条，粘贴后仍 1 条不重复）——
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_hover(el_center(js_find("复制数据")))
            await js_click(el_center(js_find("复制描述", 1)))  # 写槽 [动作, 背景]
            await wait_toast_gone()
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"B2|f1\"]')"))
            labels = await menu_labels(0)
            check("P2-1 复制描述后一级多出「粘贴描述」（粘贴标签也在，双槽）", "粘贴描述" in labels and "粘贴标签" in labels, str(labels))
            await js_click(el_center(js_find("粘贴描述", 0)))
            check("P2-2 描述按标题去重合并（B2 已有「动作」不重复 + 追加「背景」）", await ev("(() => { const c = window.__calls.set_asset_descriptions; return c.length === 1 && c[0].assetId === 'B2' && c[0].descriptions.length === 2 && c[0].descriptions[0].title === '动作' && c[0].descriptions[0].content === '已有的' && c[0].descriptions[1].title === '背景'; })()"), str(await ev("JSON.stringify(window.__calls.set_asset_descriptions)")))
            await wait_toast_gone()

            # —— P3 链接状态：「将此文件设为原始文件」出现（origin 卡禁用）——
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"B2|f1\"]')"))
            check("P3 双位置出「将此文件设为原始文件」（origin 卡禁用）", await ev(f"(() => {{ const it = {js_find('将此文件设为原始文件')}; return it != null && it.disabled; }})()"))
            # —— P4 打开文件链接所在位置 = linkList 二级（tip 壳 + 最小宽 180，2026-09-05 走查定）——
            await js_hover(el_center(js_find("打开文件链接所在位置")))
            check("P4-1 二级 = linkList 双行两行（f1 + f2 位置）", await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.link-rows .row').length === 2"), str(await ev("document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.link-rows .row').length")))
            check("P4-2 行路径两段（目录 / 角色）+ origin 徽章", await ev("(() => { const r = document.querySelectorAll('.app-menu')[1]?.querySelector('.link-rows .row'); if (!r) return false; const segs = [...r.querySelectorAll('.seg')].map((x) => x.textContent); return segs[0] === '目录' && segs[1] === '角色' && r.querySelector('.badge') != null; })()"))
            check("P4-2b tip 壳 + 最小宽 180", await ev("(() => { const m = document.querySelectorAll('.app-menu')[1]; return m.classList.contains('tip') && m.offsetWidth >= 180; })()"), str(await ev("document.querySelectorAll('.app-menu')[1]?.offsetWidth")))
            check("P4-2c tip 壳无容器内距（pad 0 行贴边）", await ev("document.querySelectorAll('.app-menu')[1]?.style.padding === '0px'"), str(await ev("document.querySelectorAll('.app-menu')[1]?.style.padding")))
            check("P4-2d tip 壳 overflow hidden（行 hover 直角底裁进圆角）", await ev("getComputedStyle(document.querySelectorAll('.app-menu')[1]).overflow === 'hidden'"))
            # 点第二行（f2 位置）→ 跳转定位 folder:f2（菜单关 + 视口切）
            await js_click(el_center("document.querySelectorAll('.app-menu')[1].querySelectorAll('.link-rows .row')[1]"))
            await asyncio.sleep(0.8)
            check("P4-3 点行跳转定位（f2 视口 + 菜单关）", await ev("document.querySelectorAll('.app-menu').length === 0 && document.querySelector('.asset-card[data-key=\"B2|f2\"]') != null"))
            # 单位置文件（A1）：linkList 一行且无徽章（2026-09-05 走查：徽章 = 活位置 > 1 才出，同卡片口径）
            # 先点树 f1 回其视口（P4-3 已跳转到 f2，A1 卡不在当前视口）
            await js_click(el_center("document.querySelector('[data-folder-id=\"f1\"]')"))
            await asyncio.sleep(0.8)
            await js_rclick(el_center("document.querySelector('.asset-card[data-key=\"A1|f1\"]')"))
            await js_hover(el_center(js_find("打开文件链接所在位置")))
            check("P4-4 单位置行无原始徽章（活位置 1）", await ev("(() => { const rows = document.querySelectorAll('.app-menu')[1]?.querySelectorAll('.link-rows .row') ?? []; return rows.length === 1 && rows[0].querySelector('.badge') == null; })()"))
            await esc_close()
            return 1 if fails else 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
