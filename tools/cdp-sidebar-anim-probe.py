# CDP 逐帧实测：侧栏收展闪帧根治（任务 B）+ 档案二级侧边栏空壳（任务 A，2026-08-15）。
# B：rAF 逐帧采样宽度序列——展开 = 单调递增且无首帧全宽（旧 bug：模板布尔跳变当帧写全宽），
#    收起 = 单调递减且无首帧 0 弹回；左右栏 + 档案面板三处全过。
# A：档案视口 = 切换钮出现（面包屑返回左边）+ .archive-side 250 宽 C3 右边框；收展逐帧单调；
#    切出档案视口面板与按钮同消失。截图留档 tools/style-preview/archive-side-panel.png。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9363
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => encodeURI("/@fs/" + p),
  transformCallback: (cb) => 1,
  invoke: (cmd, args) => {
    const STILL = "__STILL__";
    const folders = [
      { id: "F-DIR", name: "素材", parent: null, kind: null },
      { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
      { id: "F-ARC", name: "设定集", parent: "F-DIR", kind: "archive" },
    ];
    const placements = [
      { assetId: "A1", folderId: "F-ARC", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ];
    const nodeOf = (f) => ({ id: f.id, name: f.name, count: placements.filter((p) => p.folderId === f.id).length, kind: f.kind, block: f.parent === null ? "dir" : null, icon: null, children: folders.filter((c) => c.parent === f.id).map(nodeOf) });
    const R = {
      get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 1, folderCount: 3 },
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_placements: () => placements,
      list_folder_tree: () => ({ blocks: [{ id: "dir", name: "目录", icon: null }], nodes: folders.filter((f) => f.parent === null).map(nodeOf) }),
      list_archives: () => [{ folderId: "F-ARC", badge: { text: "设定", color: "#B38427" }, avatar: null, props: ["奇幻"], descriptions: [{ title: "来源", content: "官方设定集" }] }],
      list_recycle: [], list_recent_assets: [], list_inbox_snapshot: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["动画资产"], list_recent_libraries: [],
    };
    if (cmd === "save_ui_settings") return Promise.resolve(null);
    if (cmd === "get_asset_detail") {
      return Promise.resolve({
        assetId: "A1", originName: "主角-立绘.webp", ext: "webp", size: 204800, width: 800, height: 600, duration: null,
        importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00",
        tags: [], links: [], note: "",
        placements: [{ folderId: "F-ARC", blockName: "目录", path: ["素材", "设定集"], name: "主角-立绘.webp", origin: true }],
        archives: [{ folderId: "F-ARC", name: "设定集" }],
      });
    }
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(typeof v === "function" ? v() : v);
  },
};
""".replace("__STILL__", STILL)

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


def monotonic_up(seq, tol=1.0):
    return all(seq[i] <= seq[i + 1] + tol for i in range(len(seq) - 1))


def monotonic_down(seq, tol=1.0):
    return all(seq[i] >= seq[i + 1] - tol for i in range(len(seq) - 1))


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

            async def rect_of(expr):
                return await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")

            async def click(expr):
                r = await rect_of(expr)
                if not r:
                    print("!! 元素不存在:", expr[:70])
                    return False
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.4)
                return True

            # 启动 rAF 逐帧采样 → 触发动作 → 等动画收尾读序列
            async def record_toggle(sel, trigger_expr):
                await ev(f"""window.__seq = [];
                  (function rec() {{ const el = {sel}; window.__seq.push(el ? Math.round(el.getBoundingClientRect().width * 10) / 10 : -1);
                    if (window.__seq.length < 90) requestAnimationFrame(rec); }})();""")
                ok = await click(trigger_expr)
                await asyncio.sleep(0.8)
                seq = await ev("window.__seq")
                return (seq or []) if ok else []

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # —— 任务 B：右栏收 / 展 ——
            seq = await record_toggle("document.querySelector('.sidebar-right')", "[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][0]")
            print("右栏收起序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 右栏收起：单调递减无弹回（首帧不跳 0）",
                  bool(seq) and seq[0] > 200 and monotonic_down(seq) and seq[-1] <= 1,
                  json.dumps(seq))
            seq = await record_toggle("document.querySelector('.sidebar-right')", "document.querySelector('.panel-toolbar .icon-btn.flip-h')")
            print("右栏展开序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 右栏展开：单调递增无回退（首帧不跳全宽）",
                  bool(seq) and seq[0] <= 1 and monotonic_up(seq) and seq[-1] >= 249,
                  json.dumps(seq))

            # —— 任务 B：左栏收 / 展 ——
            seq = await record_toggle("document.querySelector('.sidebar-left')", "[...document.querySelectorAll('.sidebar-left .bar-title .btn-group .icon-btn')][2]")
            print("左栏收起序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 左栏收起：单调递减无弹回（首帧不跳 0）",
                  bool(seq) and seq[0] > 250 and monotonic_down(seq) and seq[-1] <= 1,
                  json.dumps(seq))
            seq = await record_toggle("document.querySelector('.sidebar-left')", "[...document.querySelectorAll('.icon-btn.flip-h')].find((b) => !b.closest('.panel-toolbar'))")
            print("左栏展开序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 左栏展开：单调递增无回退（首帧不跳全宽）",
                  bool(seq) and seq[0] <= 1 and monotonic_up(seq) and seq[-1] >= 303,
                  json.dumps(seq))

            # —— 任务 A：档案二级侧边栏 ——
            # 非档案视口：按钮与面板都不在
            pre = await ev("({ btn: !!document.querySelector('.archive-panel-toggle'), panel: !!document.querySelector('.archive-side') })")
            check("A 非档案视口：切换钮与面板均不出现", pre == {"btn": False, "panel": False}, json.dumps(pre))
            # 进入档案视口（展开素材 → 点设定集树节点）
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .expand')")
            await click("document.querySelector('.tree-item[data-folder-id=\"F-ARC\"] .body')")
            await asyncio.sleep(0.8)
            ap = await ev("""(() => {
              const el = document.querySelector('.archive-side');
              const btn = document.querySelector('.archive-panel-toggle');
              if (!el || !btn) return { el: !!el, btn: !!btn };
              const cs = getComputedStyle(el);
              const inner = el.querySelector('.as-inner');
              const bcs = getComputedStyle(inner);
              return {
                el: true, btn: true,
                w: Math.round(el.getBoundingClientRect().width),
                bg: cs.backgroundColor,
                borderR: bcs.borderRightWidth + ' ' + bcs.borderRightColor,
                title: el.querySelector('.as-title')?.textContent ?? null,
                // 切换钮在面包屑返回（后退）按钮左边
                btnBeforeBack: (() => {
                  const back = [...document.querySelectorAll('.panel-toolbar .crumb-group .icon-btn')].find((b) => b !== btn);
                  return btn.compareDocumentPosition(back) & Node.DOCUMENT_POSITION_FOLLOWING ? true : false;
                })(),
              };
            })()""")
            print("档案面板:", json.dumps(ap, ensure_ascii=False))
            check("A 档案视口：切换钮（返回钮左边）+ 面板 250 宽 C3 1px 右边框",
                  bool(ap.get("el") and ap.get("btn") and ap["w"] == 250 and ap["bg"] == "rgb(24, 25, 25)" and ap["borderR"].startswith("1px") and ap["btnBeforeBack"] and ap["title"] == "设定集"),
                  json.dumps(ap, ensure_ascii=False))
            await shot("archive-side-panel")
            # 收展逐帧单调
            seq = await record_toggle("document.querySelector('.archive-side')", "document.querySelector('.archive-panel-toggle')")
            check("A 档案面板收起：单调递减无弹回",
                  bool(seq) and seq[0] > 200 and monotonic_down(seq) and seq[-1] <= 1,
                  json.dumps(seq))
            seq = await record_toggle("document.querySelector('.archive-side')", "document.querySelector('.archive-panel-toggle')")
            check("A 档案面板展开：单调递增无回退",
                  bool(seq) and seq[0] <= 1 and monotonic_up(seq) and seq[-1] >= 249,
                  json.dumps(seq))
            # 切出档案视口：面板与按钮同消失
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.8)
            post = await ev("({ btn: !!document.querySelector('.archive-panel-toggle'), panel: !!document.querySelector('.archive-side') })")
            check("A 切出档案视口：面板与按钮同消失", post == {"btn": False, "panel": False}, json.dumps(post))

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 侧栏收展闪帧根治（左右栏展开无首帧全宽 / 收起无首帧 0，逐帧单调）+ 档案二级侧边栏（档案视口才出 / 250 C3 右边框 / 收展单调 / 切出即消）全部通过")
            return 0
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


sys.exit(asyncio.run(main()))
