# CDP 逐帧实测：树容器最外层父级展开时兄弟层级"回弹"BUG（2026-08-30 用户实机反馈）。
# 场景：顶层两父级 P1（3 子项）/ P2。展开 P1 → 子层 .ft-children 高度 0→h 补间，
# P2 被推下去。用户报告 P2 会"回弹一下"。
# 采样：rAF 逐帧记录 ① P1 .ft-children 高度 ② P2 .ft-node top。
# 判定：seqY 展开全程单调递增且 max - final ≤ 1px（无超调回落）；seqH 单调递增。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9377
URL = "http://localhost:5373"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => encodeURI("/@fs/" + p),
  transformCallback: (cb) => 1,
  invoke: (cmd, args) => {
    const folders = [
      { id: "P1", name: "父级甲", parent: null, kind: null },
      { id: "C1", name: "子一", parent: "P1", kind: null },
      { id: "C2", name: "子二", parent: "P1", kind: null },
      { id: "C3", name: "子三", parent: "P1", kind: null },
      { id: "P2", name: "父级乙", parent: null, kind: null },
    ];
    const nodeOf = (f) => ({ id: f.id, name: f.name, count: 0, kind: f.kind, block: f.parent === null ? "dir" : null, icon: null, children: folders.filter((c) => c.parent === f.id).map(nodeOf) });
    const R = {
      get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 5 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_placements: [],
      list_folder_tree: () => ({ blocks: [{ id: "dir", name: "目录", icon: null }], nodes: folders.filter((f) => f.parent === null).map(nodeOf) }),
      list_archives: [], list_recycle: [], list_recent_assets: [], list_inbox_snapshot: [], list_relations: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产"], list_recent_libraries: [],
    };
    if (cmd === "save_ui_settings") return Promise.resolve(null);
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(typeof v === "function" ? v() : v);
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
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:300])


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    dev_proc = None
    if not dev_server_up():
        print("启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
        if not dev_server_up():
            print("dev server 启动超时")
            return 2
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    fails = []

    def check(label, ok, info=""):
        print(("PASS " if ok else "FAIL ") + label + (f" | {info}" if info and not ok else ""))
        if not ok:
            fails.append(label + (f"（{info}）" if info else ""))

    try:
        version = None
        for _ in range(50):
            try:
                version = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json/version", timeout=1))
                break
            except Exception:
                time.sleep(0.3)
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

            async def mouse(t, x, y):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": x, "y": y, "clickCount": 1}, session=sess)

            # 树渲染就位检查
            ready = await ev("""!!document.querySelector('.tree-item[data-folder-id="P1"]') &&
                                !!document.querySelector('.tree-item[data-folder-id="P2"]')""")
            if not ready:
                print("!! 树未渲染（stub 未生效？）")
                return 2

            y0 = await ev("""(() => {
              const n = document.querySelector('.tree-item[data-folder-id="P2"]').closest('.ft-node');
              return n.getBoundingClientRect().top;
            })()""")
            print(f"P2 初始 top = {y0}")

            # 启动 rAF 逐帧采样（点击前开始）：P1 子层高 + P2 top
            await ev("""window.__seq = [];
              (function rec() {
                const ch = document.querySelector('.tree-item[data-folder-id="P1"]')?.closest('.ft-node')?.querySelector(':scope > .ft-children');
                const p2 = document.querySelector('.tree-item[data-folder-id="P2"]')?.closest('.ft-node');
                window.__seq.push({
                  h: ch ? Math.round(ch.getBoundingClientRect().height * 100) / 100 : -1,
                  ih: ch ? ch.style.height : "",
                  y: p2 ? Math.round(p2.getBoundingClientRect().top * 100) / 100 : -1,
                });
                if (window.__seq.length < 120) requestAnimationFrame(rec);
              })();""")

            # 点击 P1 展开钮
            r = await ev("""(() => {
              const el = document.querySelector('.tree-item[data-folder-id="P1"] .exp');
              if (!el) return null;
              const b = el.getBoundingClientRect();
              return { x: b.left + b.width / 2, y: b.top + b.height / 2 };
            })()""")
            await mouse("mousePressed", r["x"], r["y"])
            await mouse("mouseReleased", r["x"], r["y"])
            await asyncio.sleep(1.2)

            seq = await ev("window.__seq")
            if not seq:
                print("!! 无采样")
                return 2
            hs = [s["h"] for s in seq]
            ys = [s["y"] for s in seq]
            print("高度序列:", json.dumps(hs))
            print("P2 top 序列:", json.dumps(ys))

            h_final = hs[-1]
            y_final = ys[-1]
            y_max = max(ys)
            overshoot = round(y_max - y_final, 2)
            mono_h = all(hs[i] <= hs[i + 1] + 0.6 for i in range(len(hs) - 1))
            mono_y = all(ys[i] <= ys[i + 1] + 0.6 for i in range(len(ys) - 1))

            check("展开后子层高 ≈ 82（3×26 + 2×2 gap）", abs(h_final - 82) <= 1.5, f"h_final={h_final}")
            check("P2 被推距离 = 子层高 + 节点 gap 2（.ft-node flex gap）", abs((y_final - y0) - (h_final + 2)) <= 1.5, f"Δy={round(y_final - y0, 2)} h={h_final}")
            check("子层高度单调递增", mono_h, json.dumps(hs))
            check("P2 top 单调下移（无超调回弹）", mono_y and overshoot <= 1.0,
                  f"overshoot={overshoot}px seq={json.dumps(ys)}")
            if overshoot > 1.0:
                # 定位超调发生的帧位（动画中 or 结束切 auto 时）
                idx = ys.index(y_max)
                print(f"超调帧位: {idx}/{len(ys)}，当时 ih={seq[idx]['ih']} h={seq[idx]['h']}")
            # 诊断：scrollHeight(84) vs auto(82) 的 2px 差来源
            diag = await ev("""(() => {
              const ch = document.querySelector('.tree-item[data-folder-id="P1"]').closest('.ft-node').querySelector(':scope > .ft-children');
              const kids = [...ch.children].map((k) => ({
                cls: k.className, tag: k.tagName,
                oh: k.offsetHeight, ot: k.offsetTop,
                rh: Math.round(k.getBoundingClientRect().height * 100) / 100,
              }));
              return {
                scrollH: ch.scrollHeight, offsetH: ch.offsetHeight,
                rectH: Math.round(ch.getBoundingClientRect().height * 100) / 100,
                childCount: ch.children.length,
                gap: getComputedStyle(ch).gap,
                kids,
              };
            })()""")
            print("诊断:", json.dumps(diag, ensure_ascii=False))
    finally:
        proc.terminate()
        if dev_proc:
            dev_proc.terminate()

    print()
    if fails:
        print("失败", len(fails), "项：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部 PASS")
    return 0


sys.exit(asyncio.run(main()))
