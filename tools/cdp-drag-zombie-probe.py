# CDP 回归探针：拖拽僵尸会话 → 侧栏树 item 点不动 / 双选中（2026-08-13 修复回归）
# 旧 bug：dragOperation 无 pointer capture，鼠标划出窗口松手丢 pointerup → sess 僵尸；
# 后续按下被 `if (sess) return` 静默复用，移动超 6px 阈值误 startDrag →
#   ① dragView.suppressClick 吞 click（FolderTree onSelect 首行拦截，item 点不动）
#   ② startDrag 内 selection.clickFolder 误入域（.domain-selected 双选中残留）
# 已修三道：begin 里 sess 存在先 endDrag(true)；onMove 首行 (e.buttons&1)===0 立即自清；
# begin 时对按下的 .tree-item 根元素 setPointerCapture。本探针逐条回归：
#   场景 1 正常点击基线（兼验 capture 不破坏纯 click）
#   场景 2 buttons=0 自清防线（mousedown 不松手 + move(buttons=0) 模拟 pointerup 丢失）
#   场景 3 begin 不复用防线（mousedown A 不发 up 直接 mousedown B）
#   场景 4 无域残留（.domain-selected == 0 且 .tree-item.selected == 1）
#   场景 5 真拖拽不误伤（超阈值拖拽执行 move_folders 留痕，拖完后 click 仍生效）
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-drag-zombie-probe.py
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9333
URL = "http://localhost:5373"
SHOT = "tools/style-preview/drag-zombie-probe.png"
SLEEP = 0.3

STUB = r"""
window.__stubTrace = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 2 },
      get_ui_settings: {},
      list_folder_tree: [
        { id: "f1", name: "项目文件夹", count: 0, children: [] },
        { id: "f2", name: "素材", count: 0, children: [] },
      ],
      list_placements: [],
      list_recycle: [],
      list_recent_assets: [],
      list_recent_libraries: [],
      list_inbox_snapshot: [],
      detect_doc_apps: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    // 落点命令留痕（场景 5 断言真拖拽执行了 move_folders / reorder_folders）
    if (cmd === "move_folders" || cmd === "reorder_folders") {
      window.__stubTrace.push(cmd + " " + JSON.stringify(args));
      return Promise.resolve(cmd === "move_folders" ? 1 : null);
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


async def main():
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

            async def center(fid):
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)

            async def state():
                return await ev("""
                  (() => ({
                    selected: document.querySelector('.tree-item.selected')?.getAttribute('data-folder-id') ?? null,
                    selectedCount: document.querySelectorAll('.tree-item.selected').length,
                    domainCount: document.querySelectorAll('.domain-selected').length,
                    dragging: document.body.classList.contains('dragging'),
                  }))()
                """)

            async def click(x, y, with_move=False):
                await mouse("mousePressed", x, y, 1)
                if with_move:
                    await mouse("mouseMoved", x + 2, y, 1)  # 2px < 6px 阈值，不起拖拽
                await mouse("mouseReleased", x, y, 0, clicks=1)
                await asyncio.sleep(SLEEP)

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：等两个树 item 渲染
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            print("树 item 数:", n)
            if not n or n < 2:
                print("树 item 未渲染，探针无法执行")
                return 2
            a = await center("f1")
            b = await center("f2")
            print("item A(f1) 中心:", a, " item B(f2) 中心:", b)

            # ── 场景 1：正常点击基线（兼验 setPointerCapture 不破坏纯 click）──
            print("场景 1 正常点击 A")
            await click(a["x"], a["y"])
            s = await state()
            print("  状态:", s)
            check("S1 click A → selected=f1", s["selected"] == "f1", str(s))

            # ── 场景 2：buttons=0 自清防线 ──
            # B 上按下不松 → move(buttons=0) 模拟"窗口外松手 pointerup 丢失、鼠标已回窗内"
            # → 旧 sess 应立即自清；随后 click A（带 2px 移动）必须生效。
            # （旧代码此处 sess 僵尸复用：startX 还是 B 中心，A 的 click 手势距离超阈值
            #   误 startDrag → suppressClick 吞 click，A 选不中）
            print("场景 2 僵尸自清（buttons=0 防线）")
            await mouse("mousePressed", b["x"], b["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", b["x"], b["y"], 0, button="none")  # pointerup 丢失，buttons 已 0
            await asyncio.sleep(SLEEP)
            # 注意：不发 mouseReleased —— 丢失的 pointerup 正是 bug 前提
            await click(a["x"], a["y"], with_move=True)
            s = await state()
            print("  状态:", s)
            check("S2 僵尸后 click A → selected=f1", s["selected"] == "f1", str(s))

            # ── 场景 3：begin 不复用防线 ──
            # A 上按下不发 up，直接 B 上再按下 → begin 应先 endDrag(true) 清旧再建新；
            # 2px 移动不过阈值，松手后 click B 生效
            print("场景 3 begin 不复用（A 按下不发 up → B 再按下）")
            await mouse("mousePressed", a["x"], a["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mousePressed", b["x"], b["y"], 1)  # 无 up 直接二次按下
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", b["x"] + 2, b["y"], 1)
            await mouse("mouseReleased", b["x"], b["y"], 0)
            await asyncio.sleep(SLEEP)
            await click(b["x"], b["y"])  # 补一次显式 click（双 press 手势的 click 合成不稳，以 DOM 断言为准）
            s = await state()
            print("  状态:", s)
            check("S3 双 press 后 click B → selected=f2", s["selected"] == "f2", str(s))

            # ── 场景 4：无域残留（双选中回归断言）──
            print("场景 4 无域残留")
            s = await state()
            print("  状态:", s)
            check("S4 .domain-selected == 0", s["domainCount"] == 0, str(s))
            check("S4 .tree-item.selected == 1", s["selectedCount"] == 1, str(s))

            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT)

            # ── 场景 5：真拖拽不误伤 ──
            # A 按下 → 移 30px 超阈值（ghost 出现，body.dragging）→ 移到 B 上 → 松手执行落点
            # （move_folders 桩留痕）；拖完那次 click 被 suppressClick 吞是设计行为——
            # 断言"再 click A 生效"
            print("场景 5 真拖拽 A → B")
            a = await center("f1")  # 场景 3/4 后 DOM 可能刷新，重取坐标
            b = await center("f2")
            await mouse("mousePressed", a["x"], a["y"], 1)
            await asyncio.sleep(SLEEP)
            await mouse("mouseMoved", a["x"] + 30, a["y"], 1)  # 30px > 6px 阈值 → startDrag
            await asyncio.sleep(SLEEP)
            s = await state()
            print("  拖拽中状态:", s)
            check("S5 拖拽中 body.dragging", s["dragging"], str(s))
            await mouse("mouseMoved", b["x"], b["y"], 1)  # 拖到 B 上（中 1/3 = 并入）
            await asyncio.sleep(SLEEP)
            await mouse("mouseReleased", b["x"], b["y"], 0)
            await asyncio.sleep(0.6)  # 等 drop 异步 invoke + refresh
            trace = await ev("window.__stubTrace")
            s = await state()
            print("  落点后状态:", s, " 桩留痕:", trace)
            check("S5 落点执行 move_folders/reorder_folders", bool(trace), str(trace))
            check("S5 拖拽结束 body.dragging 清除", not s["dragging"], str(s))
            await click(a["x"], a["y"])  # 拖完首次 click（可能被吞，设计行为）
            await click(a["x"], a["y"])  # 再 click 必须生效
            s = await state()
            print("  状态:", s)
            check("S5 拖拽后 click A → selected=f1", s["selected"] == "f1", str(s))

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


sys.exit(asyncio.run(main()))
