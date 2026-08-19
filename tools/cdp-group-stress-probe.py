# CDP 探针：3.5 二轮 · 分组管理压力复现（2026-08-19 复验反馈 3）
#   用户现象：分组/删组操作多了后 UI 卡死——目录树选择取消不掉、递归开关点不动。
#   方法：净零循环（添加分组 → 删掉新组 → 确认 = 提交内容恒等于原配置）20 轮，
#   观测每轮 wall time / longtask 累积趋势；末态点树切换 + 递归开关验证交互响应。
#   趋势若随轮次恶化 = 前端存在累积型泄漏（监听器/订阅/DOM），与配置内容无关。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-group-stress-probe.py
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9366
URL = "http://localhost:5373"
SHOT = "tools/style-preview/group-stress-probe.png"
SLEEP = 0.3
ROUNDS = 24

# 桩：f1（24 项，含子夹 f1a 12 项——递归默认开，f1 视图 36 项）/ f2 空。f1 预置 2 组。
# 量级压到 36：组标题需在首屏可视（菜单删组要点组标题钮；累积型复现与总量无关，虚拟滚动只渲染可见区）
STUB = r"""
window.__calls = { set_folder_groups: [] };
window.__longtasks = [];
try {
  new PerformanceObserver((l) => {
    for (const e of l.getEntries()) window.__longtasks.push(Math.round(e.duration));
  }).observe({ entryTypes: ["longtask"] });
} catch (e) { /* 老内核无 longtask */ }
const GROUPS = [
  { id: "g1", name: "图片组", rules: [{ kind: "ext", values: ["png"] }] },
  { id: "g2", name: "动图组", rules: [{ kind: "type", values: ["anim"] }] },
];
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 36, children: [
      { id: "f1a", name: "子夹", count: 12, children: [] },
    ], groups: JSON.parse(JSON.stringify(GROUPS)) },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mk = (assetId, folderId, name, ext, over = {}) => ({
  assetId, folderId, name, w: 800, h: 600, ext, thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 100, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T10:00:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
  ...over,
});
const PLACEMENTS = [];
for (let i = 0; i < 36; i++) {
  const fid = i < 24 ? "f1" : "f1a";
  const ext = i % 3 === 0 ? "gif" : i % 3 === 1 ? "png" : "md";
  PLACEMENTS.push(mk("AN-" + i, fid, "素材" + i + "." + ext, ext,
    ext === "gif" ? { frameCount: 3, animated: true } : {}));
}
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 36, folderCount: 3 },
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
      const walk = (ns) => ns.forEach((n) => {
        if (n.id === args.folderId) n.groups = args.groups.length ? args.groups : undefined;
        walk(n.children ?? []);
      });
      walk(TREE.nodes);
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
        if raw.get("method") == "Runtime.consoleAPICalled":
            level = raw.get("params", {}).get("type", "")
            if level in ("error", "warn"):
                args = raw.get("params", {}).get("args", [])
                text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
                print(f"  [console.{level}]", text[:400])
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw.get("params", {}).get("exceptionDetails", {})
            print("  [exception]", str(d.get("text", ""))[:200], str(d.get("exception", {}).get("description", ""))[:500])


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（vite node 脱进程树，taskkill /T 杀壳后端口仍占——兜底）"""
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

            async def mouse(t, x, y, buttons, clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": "left", "buttons": buttons,
                    "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def click(x, y):
                await mouse("mousePressed", x, y, 1)
                await mouse("mouseReleased", x, y, 0)
                await asyncio.sleep(SLEEP)

            async def click_js(expr, desc):
                """按 JS 表达式定位元素中心点击；返回是否点成"""
                pos = await ev(f"""
                  (() => {{
                    const el = {expr};
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)
                if not pos:
                    print(f"  [miss] {desc}")
                    return False
                await click(pos["x"], pos["y"])
                return True

            async def click_scrolled(expr, desc):
                """滚动直至目标进入视口安全带（150~1050，避开 sticky 悬浮标题）再点击"""
                for _ in range(8):
                    info = await ev(f"""
                      (() => {{
                        const el = {expr};
                        if (!el) return null;
                        const r = el.getBoundingClientRect();
                        return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                      }})()
                    """)
                    if not info:
                        print(f"  [miss] {desc}")
                        return False
                    if 150 <= info["y"] <= 1050:
                        await click(info["x"], info["y"])
                        return True
                    await send(ws, "Input.dispatchMouseEvent", {
                        "type": "mouseWheel", "x": 700, "y": 500,
                        "deltaX": 0, "deltaY": 500 if info["y"] > 1050 else -500,
                    }, session=sess)
                    await asyncio.sleep(0.4)
                print(f"  [miss] {desc}（滚动 8 次未入视口）")
                return False

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：进 f1（递归默认开 → 90 项 + 2 组标题）
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 3:
                    break
                await asyncio.sleep(0.5)
            if not await click_js("document.querySelector('.tree-item[data-folder-id=\"f1\"]')", "f1"):
                print("树未渲染")
                return 2
            for _ in range(20):
                c = await ev("document.querySelectorAll('.asset-card').length")
                if c and c > 0:
                    break
                await asyncio.sleep(0.5)
            base_cards = await ev("document.querySelectorAll('.asset-card').length")
            print(f"  f1 视图卡片 {base_cards} 张（递归 36 项，虚拟滚动只渲染可见区）")

            # ── 压力循环：4 种操作轮转（弹窗净零 / 菜单真删+加回 / 折叠+树切换 / 滚动+悬停轨迹） ──
            print(f"压力循环 {ROUNDS} 轮（4 种操作轮转）")
            rounds_ms = []
            lt_prev = 0

            async def open_manage():
                if not await click_js("document.querySelector('.panel-toolbar .btn-group .icon-btn:nth-last-child(2)')", "分组管理钮"):
                    return False
                for _ in range(20):
                    if await ev("!!document.querySelector('.group-manage')"):
                        return True
                    await asyncio.sleep(0.2)
                return False

            async def confirm_manage():
                ok = await click_js("[...document.querySelectorAll('.group-manage .actions .app-btn')].find((b) => b.textContent.trim() === '确认')", "确认")
                if not ok:
                    return False
                for _ in range(30):
                    if not await ev("!!document.querySelector('.group-manage')"):
                        return True
                    await asyncio.sleep(0.2)
                return False

            for i in range(ROUNDS):
                op = i % 4
                # 上轮异常残留清理（alert「取消」= 放弃并连带关管理窗；双轮兜底）
                for _ in range(2):
                    if await ev("!!document.querySelector('.app-modal.alert')"):
                        await click_js("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.trim() === '取消')", "清错误窗")
                    if await ev("!!document.querySelector('.group-manage')"):
                        await click_js("[...document.querySelectorAll('.group-manage .actions .app-btn')].find((b) => b.textContent.trim() === '取消')", "清管理窗")
                    if not await ev("!!document.querySelector('.group-manage') || !!document.querySelector('.app-modal.alert')"):
                        break
                t0 = time.perf_counter()
                ok = True
                if op == 0:
                    # 弹窗净零：添加分组 → 删新组 → 确认
                    if await open_manage():
                        await click_js("document.querySelector('.group-manage .add-btn')", "添加分组")
                        await click_js("[...document.querySelectorAll('.group-manage .group-block .icon-btn.delete-red')].pop()", "删新组")
                        ok = await confirm_manage()
                    else:
                        ok = False
                elif op == 1:
                    # 菜单真删 ext:png 组（图片组/上轮加回的未命名分组，跳过动图组防完全相同）
                    # → 弹窗加回 ext:png 组；目标组位置随轮次漂移（组序变），滚动入视口再点
                    ok = await click_scrolled("""
                      (() => {
                        const hs = [...document.querySelectorAll('.group-header')]
                          .filter((h) => !h.textContent.includes('未分组') && !h.textContent.includes('动图组'));
                        return hs[0]?.querySelector('.group-menu-btn') ?? null;
                      })()
                    """, "组菜单钮")
                    if ok:
                        for _ in range(15):
                            if await ev("document.querySelectorAll('.app-menu .menu-item').length > 0"):
                                break
                            await asyncio.sleep(0.2)
                        ok = await click_js("[...document.querySelectorAll('.app-menu .menu-item')].find((el) => el.textContent.trim() === '删除')", "菜单删除")
                        await asyncio.sleep(0.5)
                    if ok and await open_manage():
                        await click_js("document.querySelector('.group-manage .add-btn')", "添加分组")
                        await asyncio.sleep(0.3)
                        # 新组（最后组块）第一行规则选 png chip
                        await click_js("""
                          (() => {
                            const b = [...document.querySelectorAll('.group-manage .group-block')].pop();
                            return [...(b?.querySelectorAll('.chip') ?? [])].find((c) => c.textContent.trim() === 'png') ?? null;
                          })()
                        """, "选 png")
                        ok = await confirm_manage()
                    else:
                        ok = False
                elif op == 2:
                    # 折叠未分组 → 展开 → 点 f2 → 点回 f1
                    fa = """
                      (() => {
                        const h = [...document.querySelectorAll('.group-header')].find((x) => x.textContent.includes('未分组'));
                        return h?.querySelector('.fold-area') ?? null;
                      })()
                    """
                    await click_js(fa, "折叠未分组")
                    await click_js(fa, "展开未分组")
                    await click_js("document.querySelector('.tree-item[data-folder-id=\"f2\"]')", "f2")
                    await click_js("document.querySelector('.tree-item[data-folder-id=\"f1\"]')", "回 f1")
                else:
                    # 滚动 + 悬停轨迹（10 点划过网格）→ 滚回
                    await send(ws, "Input.dispatchMouseEvent", {
                        "type": "mouseWheel", "x": 700, "y": 500, "deltaX": 0, "deltaY": 800,
                    }, session=sess)
                    for k in range(10):
                        await send(ws, "Input.dispatchMouseEvent", {
                            "type": "mouseMoved", "x": 400 + k * 60, "y": 400 + (k % 3) * 120,
                        }, session=sess)
                        await asyncio.sleep(0.05)
                    await send(ws, "Input.dispatchMouseEvent", {
                        "type": "mouseWheel", "x": 700, "y": 500, "deltaX": 0, "deltaY": -800,
                    }, session=sess)
                    await asyncio.sleep(0.3)
                ms = round((time.perf_counter() - t0) * 1000)
                rounds_ms.append(ms)
                lt = await ev("window.__longtasks.length") or 0
                lt_sum = await ev("window.__longtasks.reduce((a, b) => a + b, 0)") or 0
                dom = ""
                if (i + 1) % 6 == 0:
                    dom = await ev("""
                      " DOM " + document.getElementsByTagName('*').length
                        + " body子 " + document.body.children.length
                        + " menu " + document.querySelectorAll('.app-menu').length
                        + " modal " + document.querySelectorAll('.app-modal').length
                        + " tip " + document.querySelectorAll('.app-tip.show').length
                    """)
                print(f"  R{i + 1:02d} op{op}: {ms}ms（{'ok' if ok else '失败！'}）longtask {lt} 条 / {lt_sum}ms（+{lt - lt_prev}）{dom or ''}")
                lt_prev = lt
                if not ok:
                    fails.append(f"R{i + 1} op{op}: 操作失败")
                    break

            # ── 末态交互响应：点树 f2 → selected 应 1s 内落 f2 ──
            print("末态交互验证")
            await click_js("document.querySelector('.tree-item[data-folder-id=\"f2\"]')", "f2")
            t0 = time.perf_counter()
            switched = False
            while time.perf_counter() - t0 < 2:
                if await ev("document.querySelector('.tree-item[data-folder-id=\"f2\"]')?.classList.contains('selected')"):
                    switched = True
                    break
                await asyncio.sleep(0.2)
            check("M1 点树 f2 → 1s 内选中切换", switched, f"耗时 {round((time.perf_counter() - t0) * 1000)}ms")

            # 回 f1 切递归开关：卡片数应变化（90 → 60 直项）
            await click_js("document.querySelector('.tree-item[data-folder-id=\"f1\"]')", "回 f1")
            await asyncio.sleep(0.5)
            await click_js("document.querySelector('.recursive-cb input') ?? document.querySelector('.recursive-cb')", "递归开关")
            await asyncio.sleep(0.8)
            rec_off = await ev("window.__lastCards = document.querySelectorAll('.asset-card').length")
            await click_js("document.querySelector('.recursive-cb input') ?? document.querySelector('.recursive-cb')", "递归开关还原")
            await asyncio.sleep(0.8)
            rec_on = await ev("document.querySelectorAll('.asset-card').length")
            print(f"  递归关卡片 {rec_off} / 递归开卡片 {rec_on}")
            check("M2 递归开关可点且内容区随之变化", rec_off != rec_on, f"off={rec_off} on={rec_on}")

            # 趋势判定：后 5 轮均值 vs 前 5 轮均值
            if len(rounds_ms) >= 10:
                head = sum(rounds_ms[:5]) / 5
                tail = sum(rounds_ms[-5:]) / 5
                print(f"  耗时趋势：前 5 轮均值 {head:.0f}ms / 后 5 轮均值 {tail:.0f}ms（×{tail / max(head, 1):.2f}）")
                check("M3 耗时无显著恶化（后 5 轮 < 前 5 轮 ×1.8）", tail < head * 1.8, f"{head:.0f} → {tail:.0f}ms")

            await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            import base64
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print(f"截图 → {SHOT}")
    finally:
        edge.terminate()
        if spawned:
            spawned.terminate()
        kill_listening_port(5373)

    print()
    if fails:
        print("失败项：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


sys.exit(asyncio.run(main()))
