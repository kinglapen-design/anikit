# CDP 探针：新增区置顶期间拖拽排序失效修复（2026-10-13 用户实机报"链接与原始文件同夹时
#   拖拽排序提示成功但没用、切目录又好了"）——dropAssetsGap 自定义排序分支按存储序插入，
#   与 pinned 可见序错位；修为"目标段含 pinned 项即物化可见序 + 归位"（同显示排序口径）
#   S1 链接粘贴（Ctrl+C/Ctrl+V）→ 链接行入库 + 新增区登记 assetId|folderId
#      → 撞键两条（origin + 链接）同钉段首：[原, 原_链, 乙, 丙]
#   S2 置顶期间拖 原_链 到 乙/丙 间隙（丙前）→ reorder = 物化全层 4 项序列 + beforeItem null
#   S3 refresh 后显示序 = [原, 乙, 原_链, 丙]（修复前 = 钉住盖回 [原, 原_链, 乙, 丙]）
#   S4 归位后再拖 原_链 回 乙 前 → 自定义基线路径（items=[链接行] + beforeItem=乙）不破
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-arrivals-drag-probe.py
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9361
URL = "http://localhost:5373"
SHOT = "tools/style-preview/arrivals-drag-probe.png"
SLEEP = 0.35

# 桩：f1 提示词（3 项自定义序）/ f2 空（备用切视图）。reorder 桩按后端语义真实应用：
# items 定位行（origin + name 精确到行）剔除后插到 beforeItem 前（null = 段尾）——
# 修复前的"提示成功但没用"正是靠这个真实应用 + 前端钉住盖回来复现
STUB = r"""
window.__calls = { reorder_placements: [], link_placements: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "提示词", count: 3, children: [] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mk = (assetId, name, origin) => ({
  assetId, folderId: "f1", name, w: 800, h: 600, ext: "md", thumbPath: null, thumbKind: null,
  origin, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 1024, createdAt: "2026-10-01T10:00:00+08:00",
  modifiedAt: "2026-10-10T10:00:00+08:00", importedAt: "2026-10-11T10:00:00+08:00", noteCount: null,
});
const PLACEMENTS = [
  mk("AN-A", "原.md", true),
  mk("AN-B", "乙.md", true),
  mk("AN-C", "丙.md", true),
];
// 按 PlacementRef 定位行：origin 恒有；name 给了 = 精确到行（同 find_placement 口径）
const findRow = (rows, it, skip) => rows.findIndex((p, k) =>
  !skip.has(k) && p.assetId === it.assetId && p.origin === !!it.origin && (it.name == null || p.name === it.name));
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 2 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "link_placements") {
      window.__calls.link_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      for (const it of args?.items ?? []) {
        const src = PLACEMENTS.find((p) => p.assetId === it.assetId);
        PLACEMENTS.push({ ...(src ?? mk(it.assetId, "原_链.md", false)), folderId: args.targetFolder, name: "原_链.md", origin: false });
      }
      return Promise.resolve((args?.items ?? []).length);
    }
    if (cmd === "reorder_placements") {
      window.__calls.reorder_placements.push(JSON.parse(JSON.stringify(args ?? {})));
      const T = args?.targetFolder;
      const rows = PLACEMENTS.filter((p) => p.folderId === T);
      const skip = new Set();
      const picked = [];
      for (const it of args?.items ?? []) {
        const i = findRow(rows, it, skip);
        if (i >= 0) { skip.add(i); picked.push(rows[i]); }
      }
      const rest = rows.filter((_, k) => !skip.has(k));
      let at = rest.length;
      if (args?.beforeItem) {
        const bi = findRow(rest, args.beforeItem, new Set());
        if (bi >= 0) at = bi;
      }
      const seg = [...rest.slice(0, at), ...picked, ...rest.slice(at)];
      const others = PLACEMENTS.filter((p) => p.folderId !== T);
      PLACEMENTS.length = 0; PLACEMENTS.push(...others, ...seg);
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


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（vite node 脱离 pnpm 壳进程树，端口直杀兜底）"""
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

            async def card_names():
                return await ev("""
                  [...document.querySelectorAll('.asset-card .kc-name')].map((el) => el.textContent.trim())
                """)

            async def card_point(data_key, lead=False):
                # lead=True 取卡片左半区（前间隙判定：x < left + width/2）
                return await ev(f"""
                  (() => {{
                    const el = document.querySelector('.asset-card[data-key="{data_key}"]');
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round({"(r.left + 10)" if lead else "(r.left + r.width / 2)"}), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)

            async def drag_to_gap(src_key, dst_key):
                src = await card_point(src_key)
                dst = await card_point(dst_key, lead=True)
                if not src or not dst:
                    return False
                await mouse("mousePressed", src["x"], src["y"], 1)
                await asyncio.sleep(0.1)
                await mouse("mouseMoved", src["x"] + 10, src["y"], 1)
                await asyncio.sleep(0.1)
                await mouse("mouseMoved", dst["x"], dst["y"], 1)
                await asyncio.sleep(0.5)
                await mouse("mouseReleased", dst["x"], dst["y"], 0)
                await asyncio.sleep(0.8)
                return True

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
            n = 0
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            if not n or n < 2:
                print("树未渲染，探针无法执行")
                return 2
            f1 = await tree_item("f1")
            await click(f1["x"], f1["y"])
            cards = None
            for _ in range(20):
                cards = await card_names()
                if cards and len(cards) == 3:
                    break
                await asyncio.sleep(0.5)
            print("f1 卡片（存储序）:", cards)
            check("S0 f1 三卡渲染", cards == ["原.md", "乙.md", "丙.md"], str(cards))

            # ── S1：Ctrl+C 原 → Ctrl+V 链接粘贴 → 撞键两条同钉段首 ──
            print("S1 链接粘贴 + 新增区置顶")
            src = await card_point("AN-A|f1")
            if not src:
                check("S1a 原卡可按基底键命中", False, "未找到卡片")
                return 2
            await click(src["x"], src["y"])
            await key("c", "KeyC", 67, ctrl=True)
            await key("v", "KeyV", 86, ctrl=True)
            await asyncio.sleep(0.8)  # link + markArrivals + refresh
            names = await card_names()
            print("  粘贴后（撞键两条应同钉段首）:", names)
            check("S1 新增区钉住 origin + 链接两条", names == ["原.md", "原_链.md", "乙.md", "丙.md"], str(names))

            # ── S2：置顶期间拖 原_链 到 乙/丙 间隙（丙前）→ 物化全层重写 ──
            print("S2 置顶期间拖拽 → 物化")
            ok = await drag_to_gap("AN-A|f1|0|原_链.md", "AN-C|f1")
            check("S2a 拖拽落点成立", ok)
            calls = await ev("window.__calls.reorder_placements")
            print("  reorder 调用:", json.dumps(calls, ensure_ascii=False))
            seq_ok = False
            if calls:
                last = calls[-1]
                seq = [(it["assetId"], it.get("origin"), it.get("name")) for it in last.get("items", [])]
                seq_ok = (seq == [("AN-A", True, "原.md"), ("AN-B", True, None),
                                  ("AN-A", False, "原_链.md"), ("AN-C", True, None)]
                          and last.get("beforeItem") is None
                          and last.get("targetFolder") == "f1")
            check("S2b 物化 = 全层可见序 + 拖拽组落点重写（beforeItem null）", seq_ok,
                  json.dumps(calls[-1] if calls else None, ensure_ascii=False))

            # ── S3：refresh 后显示序 = 物化序（修复前 = 钉住盖回不动）──
            names = await card_names()
            print("  拖拽后显示序:", names)
            check("S3 显示序随拖拽生效（不再钉住盖回）", names == ["原.md", "乙.md", "原_链.md", "丙.md"], str(names))

            # ── S4：归位后自定义基线路径不破（单项 + beforeItem 锚点）──
            print("S4 归位后再拖回（基线路径）")
            ok = await drag_to_gap("AN-A|f1|0|原_链.md", "AN-B|f1")
            check("S4a 拖拽落点成立", ok)
            calls = await ev("window.__calls.reorder_placements")
            base_ok = False
            if calls and len(calls) >= 2:
                last = calls[-1]
                its = last.get("items", [])
                b = last.get("beforeItem") or {}
                base_ok = (len(its) == 1
                           and its[0].get("assetId") == "AN-A" and its[0].get("origin") is False
                           and its[0].get("name") == "原_链.md"
                           and b.get("assetId") == "AN-B")
            check("S4b 基线路径 = 单链接行 + beforeItem=乙", base_ok,
                  json.dumps(calls[-1] if calls else None, ensure_ascii=False))
            names = await card_names()
            print("  拖回后显示序:", names)
            check("S4c 显示序 = [原, 原_链, 乙, 丙]", names == ["原.md", "原_链.md", "乙.md", "丙.md"], str(names))

            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(__import__("base64").b64decode(shot["result"]["data"]))
            print("截图:", SHOT)
    finally:
        edge.terminate()
        if spawned:
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


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
