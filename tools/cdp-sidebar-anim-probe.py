# CDP 逐帧实测：侧栏收展闪帧根治（任务 B，2026-08-15）。
# 2026-09-25 T0 验证网修复：选择器换代适配 + 共享桩 _probe_stub；
#   任务 A（档案二级侧边栏 .archive-side）随功能删除整体退役——触发器/断言对象在 src/ 零匹配。
# B：rAF 逐帧采样宽度序列——展开 = 单调递增且无首帧全宽（旧 bug：模板布尔跳变当帧写全宽），
#    收起 = 单调递减且无首帧 0 弹回；左右栏两处全过。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9363
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
# 启动链默认桩表（get_window_fx_mode / get_ui_settings / list_relations / list_inbox_snapshot 等）
# 由 _probe_stub.DEFAULT_STUBS 兜底；缩略图真读盘故 convertFileSrc 用 /@fs/ 形式
STUB = make_stub(
    {
        "get_current_library": '{ name: "动画资产", path: "E:/T/S/动画资产", assetCount: 1, folderCount: 2 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: true }',
        "list_placements": "placements",
        "list_folder_tree": "tree()",
        "list_space_libraries": '["动画资产"]',
        "list_recent_libraries": "[]",
    },
    head="""
const STILL = "__STILL__";
const folders = [
  { id: "F-DIR", name: "素材", parent: null, kind: null },
  { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
];
const placements = [
  { assetId: "A1", folderId: "F-DIR", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
];
const nodeOf = (f) => ({ id: f.id, name: f.name, count: placements.filter((p) => p.folderId === f.id).length, kind: f.kind, block: f.parent === null ? "dir" : null, icon: null, children: folders.filter((c) => c.parent === f.id).map(nodeOf) });
const tree = () => ({ blocks: [{ id: "dir", name: "目录", icon: null }], nodes: folders.filter((f) => f.parent === null).map(nodeOf) });
""",
    handlers='    if (cmd === "save_ui_settings") return Promise.resolve(null);',
    convert_file_src='(p) => encodeURI("/@fs/" + p)',
).replace("__STILL__", STILL)

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

            # —— 任务 B：右栏收 / 展 ——
            # 2026-09-25 T0：右栏 54 工具栏已废，收展钮归中栏 ContentTitleBar 最右侧 collapse-sidebar
            # （.btn-group 末位；多 panel 并存按 offsetParent 可见性过滤）
            seq = await record_toggle("document.querySelector('.sidebar-right')", "(() => { const bar = [...document.querySelectorAll('.panel-toolbar')].find((el) => el.offsetParent !== null); if (!bar) return null; const btns = [...bar.querySelectorAll('.btn-group .ic-btn')]; return btns[btns.length - 1] ?? null; })()")
            print("右栏收起序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 右栏收起：单调递减无弹回（首帧不跳 0）",
                  bool(seq) and seq[0] > 200 and monotonic_down(seq) and seq[-1] <= 1,
                  json.dumps(seq))
            # 收起态同钮 flip-h（前进钮恒 flip-h 但在 crumb-group，须锚 .btn-group 内找）
            seq = await record_toggle("document.querySelector('.sidebar-right')", "(() => { const bar = [...document.querySelectorAll('.panel-toolbar')].find((el) => el.offsetParent !== null); return bar ? bar.querySelector('.btn-group .ic-btn.flip-h') : null; })()")
            print("右栏展开序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 右栏展开：单调递增无回退（首帧不跳全宽）",
                  bool(seq) and seq[0] <= 1 and monotonic_up(seq) and seq[-1] >= 249,
                  json.dumps(seq))

            # —— 任务 B：左栏收 / 展 ——
            seq = await record_toggle("document.querySelector('.sidebar-left')", "document.querySelector('.sidebar-left .menu-bar .ic-btn')")
            print("左栏收起序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 左栏收起：单调递减无弹回（首帧不跳 0）",
                  bool(seq) and seq[0] > 200 and monotonic_down(seq) and seq[-1] <= 1,
                  json.dumps(seq))
            # 展开钮 = SidebarExpandButton（标签栏行首，非 panel-toolbar 内）
            seq = await record_toggle("document.querySelector('.sidebar-left')", "[...document.querySelectorAll('.ic-btn.flip-h')].find((b) => !b.closest('.panel-toolbar'))")
            print("左栏展开序列:", seq[:8], "...", seq[-3:] if seq else "")
            check("B 左栏展开：单调递增无回退（首帧不跳全宽）",
                  bool(seq) and seq[0] <= 1 and monotonic_up(seq) and seq[-1] >= 253,
                  json.dumps(seq))

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 侧栏收展闪帧根治（左右栏展开无首帧全宽 / 收起无首帧 0，逐帧单调）全部通过")
            return 0
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


sys.exit(asyncio.run(main()))
