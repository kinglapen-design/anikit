# CDP 回归探针：跨窗口拖出（素材拖出窗口边界 → drag_assets 系统级拖放移交，2026-09-02 走查修复）
# 修复背景：drag_files  spike 命令自阶段 1 重构后无前端调用，"拖至其他软件打开"形同失效。
# 修复内容：dragOperation onMove 越界检测（capture 后窗外 pointermove 仍派发、坐标越界）→
#   取消内部会话 + invoke drag_assets（后端锁内 real_origin_file 解析 + DoDragDrop）。
# 本探针用 DOM 合成 PointerEvent（CDP Input 无法给出窗口外坐标）逐条回归：
#   场景 1 素材拖出窗口 → invoke drag_assets 留痕（assetIds 正确）+ 内部会话已取消（body.dragging 清除）
#   场景 2 窗口内落点不回归（素材拖树 item 并入 = move_placements，不误触发 drag_assets）
#   场景 3 文件夹拖出窗口 = 无操作（info 目录非用户文件，不拖出；无 drag_assets / move_folders 留痕）
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-drag-out-probe.py
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
SHOT = "tools/style-preview/drag-out-probe.png"
SLEEP = 0.3

STUB = r"""
window.__stubTrace = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 1, folderCount: 2 },
      get_ui_settings: {},
      list_folder_tree: {
        blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "项目文件夹", count: 0, totalSize: 0, children: [] },
          { id: "f2", name: "素材", count: 1, totalSize: 1234, children: [] },
        ],
      },
      list_placements: [{
        assetId: "AN-A", folderId: "f2", name: "测试图.png", w: 100, h: 80,
        thumbW: 100, thumbH: 80, ext: "png", thumbPath: null, thumbKind: null,
        origin: true, frameCount: null, animated: false, duration: null, bpm: null,
        importedFrom: null, size: 1234, createdAt: null,
        modifiedAt: "2026-09-02T00:00:00Z", importedAt: "2026-09-02T00:00:00Z",
        noteCount: null, thumbFrame: null, fps: null, hidden: false, playAssetId: null,
      }],
      list_recycle: [],
      list_recent_assets: [],
      list_recent_libraries: [],
      list_inbox_snapshot: [],
      list_archives: [],
      list_relations: [],
      detect_doc_apps: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    // 拖出 / 落点命令留痕（断言 invoke 是否抵达、入参是否正确）
    if (["drag_assets", "move_placements", "link_placements", "reorder_placements", "move_folders", "reorder_folders"].includes(cmd)) {
      window.__stubTrace.push(cmd + " " + JSON.stringify(args));
      return Promise.resolve(cmd === "move_placements" ? 1 : null);
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


# DOM 合成 pointer 手势（CDP Input.dispatchMouseEvent 给不了窗口外坐标——本探针核心场景即越界，
# 而引擎监听挂 window、begin 走卡片 pointerdown 冒泡，合成事件全链可达；setPointerCapture
# 对合成 pointerId 抛异常已被引擎 try/catch 吞掉，不影响后续分支）
DRAG_OUT = r"""
(() => {
  const card = document.querySelector('.asset-card[data-key]');
  if (!card) return "no-card";
  const r = card.getBoundingClientRect();
  const cx = Math.round(r.left + r.width / 2), cy = Math.round(r.top + r.height / 2);
  const pe = (type, x, y, buttons) => new PointerEvent(type, {
    bubbles: true, cancelable: true, clientX: x, clientY: y,
    button: 0, buttons, pointerId: 1, isPrimary: true,
  });
  card.dispatchEvent(pe("pointerdown", cx, cy, 1));
  window.dispatchEvent(pe("pointermove", cx + 30, cy, 1)); // 超 6px 阈值 → startDrag
  const midDragging = document.body.classList.contains("dragging");
  window.dispatchEvent(pe("pointermove", -50, cy, 1));     // 越出窗口左缘 → 应触发拖出
  return { midDragging, afterDragging: document.body.classList.contains("dragging") };
})()
"""

DRAG_TO_TREE = r"""
(() => {
  const card = document.querySelector('.asset-card[data-key]');
  const tree = document.querySelector('.tree-item[data-folder-id="f1"]');
  if (!card || !tree) return "no-target";
  const r = card.getBoundingClientRect();
  const t = tree.getBoundingClientRect();
  const cx = Math.round(r.left + r.width / 2), cy = Math.round(r.top + r.height / 2);
  const tx = Math.round(t.left + t.width / 2), ty = Math.round(t.top + t.height / 2);
  const pe = (type, x, y, buttons) => new PointerEvent(type, {
    bubbles: true, cancelable: true, clientX: x, clientY: y,
    button: 0, buttons, pointerId: 1, isPrimary: true,
  });
  card.dispatchEvent(pe("pointerdown", cx, cy, 1));
  window.dispatchEvent(pe("pointermove", cx + 30, cy, 1));
  window.dispatchEvent(pe("pointermove", tx, ty, 1));       // 窗口内树 item 上（并入落点）
  window.dispatchEvent(pe("pointerup", tx, ty, 0));
  return document.body.classList.contains("dragging");
})()
"""

FOLDER_DRAG_OUT = r"""
(() => {
  const tree = document.querySelector('.tree-item[data-folder-id="f1"]');
  if (!tree) return "no-tree";
  const t = tree.getBoundingClientRect();
  const tx = Math.round(t.left + t.width / 2), ty = Math.round(t.top + t.height / 2);
  const pe = (type, x, y, buttons) => new PointerEvent(type, {
    bubbles: true, cancelable: true, clientX: x, clientY: y,
    button: 0, buttons, pointerId: 1, isPrimary: true,
  });
  tree.dispatchEvent(pe("pointerdown", tx, ty, 1));
  window.dispatchEvent(pe("pointermove", tx + 30, ty, 1));
  const midDragging = document.body.classList.contains("dragging");
  window.dispatchEvent(pe("pointermove", -50, ty, 1));      // 越界——文件夹不应拖出
  window.dispatchEvent(pe("pointerup", tx, ty, 0));         // 回窗内原处松手
  return { midDragging, afterDragging: document.body.classList.contains("dragging") };
})()
"""


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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    print("[EVAL-EX]", str(r["result"]["exceptionDetails"])[:400])
                    return None
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 准备：等树渲染，点进 f2 让卡片上屏
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            if not n or n < 2:
                print("树 item 未渲染，探针无法执行")
                return 2
            await ev("""
              document.querySelector('.tree-item[data-folder-id="f2"]')
                .dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, button: 0, buttons: 1, pointerId: 9 }));
              window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, button: 0, buttons: 0, pointerId: 9 }));
            """)
            await asyncio.sleep(0.8)
            for _ in range(20):
                card_n = await ev("document.querySelectorAll('.asset-card[data-key]').length")
                if card_n:
                    break
                await asyncio.sleep(0.5)
            print("卡片数:", card_n)
            if not card_n:
                print("素材卡片未渲染，探针无法执行")
                return 2

            # ── 场景 1：素材拖出窗口 → drag_assets 移交 ──
            print("场景 1 素材拖出窗口边界")
            s1 = await ev(DRAG_OUT)
            await asyncio.sleep(SLEEP)
            trace = await ev("window.__stubTrace")
            print("  状态:", s1, " 桩留痕:", trace)
            check("S1 拖拽中 body.dragging", bool(s1 and s1.get("midDragging")), str(s1))
            drag_calls = [t for t in (trace or []) if t.startswith("drag_assets")]
            check("S1 invoke drag_assets 抵达", len(drag_calls) == 1, str(trace))
            check("S1 assetIds = [AN-A]", bool(drag_calls and '"assetIds":["AN-A"]' in drag_calls[0]), str(drag_calls))
            check("S1 移交后内部会话已取消", bool(s1 and not s1.get("afterDragging")), str(s1))

            # ── 场景 2：窗口内落点不回归（素材 → 树 f1 并入 = move_placements）──
            print("场景 2 窗口内素材 → 树并入落点")
            s2 = await ev(DRAG_TO_TREE)
            await asyncio.sleep(0.6)
            trace = await ev("window.__stubTrace")
            print("  dragging 残留:", s2, " 桩留痕:", trace)
            move_calls = [t for t in (trace or []) if t.startswith("move_placements")]
            drag_calls = [t for t in (trace or []) if t.startswith("drag_assets")]
            check("S2 并入执行 move_placements", len(move_calls) == 1, str(trace))
            check("S2 不误触发 drag_assets", len(drag_calls) == 1, str(drag_calls))  # 仅场景 1 那次
            check("S2 落点后 dragging 清除", s2 is False, str(s2))

            # ── 场景 3：文件夹拖出窗口 = 无操作 ──
            print("场景 3 文件夹拖出窗口")
            s3 = await ev(FOLDER_DRAG_OUT)
            await asyncio.sleep(SLEEP)
            trace = await ev("window.__stubTrace")
            print("  状态:", s3, " 桩留痕:", trace)
            check("S3 文件夹拖拽中 body.dragging", bool(s3 and s3.get("midDragging")), str(s3))
            check("S3 无 drag_assets / move_folders 留痕", len([t for t in (trace or []) if t.startswith(("drag_assets", "move_folders", "reorder_folders"))]) == 1, str(trace))  # 仅 S1 的 drag_assets
            check("S3 松手后 dragging 清除", bool(s3 and not s3.get("afterDragging")), str(s3))

            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图已存:", SHOT)

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
