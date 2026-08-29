# CDP 探针：7.1 十九轮 · 中间面板-标签栏贴稿（2026-08-30）——dockview 标签栏结构断言
#   T1  栏容器：高 46 / padding 8·24·0·36 / 底透明（透窗口底，十七轮定）
#   T2  栏行 gap 12（标签页组 与 + 钮 的间距，稿口径）
#   T3  标签列表 gap 4（= 反弧肩径，hover 罩右下间距统一 4）
#   T4  标签页外壳：宽 180（2026-08-30 用户定：固定宽 + 过多平分收缩）/ 高 38 / 无 padding 边框 / 底透明
#   T5  选中标签：g1 底 + 圆角 8 8 0 0 + 一级字 + 关闭钮显 + 底部两侧 4×4 反弧 radial-gradient
#   T6  未选中标签（点 + 钮开第二标签后验第一枚）：透明底 + 三级字 rgb(128,128,128) + 关闭钮 opacity 0
#   T7  hover 未选中：四周 4px 内缩 6px 圆角罩 亮/半透明-3 rgba(242,242,242,0.06) + 关闭钮显
#   T8  添加钮：30×30 + 色 亮/三级
#   T9  dockview 自绘滚动条（dv-scrollbar）恒关——flex trailing gap 假溢出 4px 冒灰带截断无缝
#   截图 r19-tab-bar.png（hover 未选中态留档；headless 无云母，仅结构）
# 桩：最小主窗口（空 placements）；fx 固定 solid（本断言与窗口特效无关）。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-tab-bar-probe.py
import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19420 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-tabbar-{os.getpid()}")
SHOT = "tools/style-preview/r19-tab-bar.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 1 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [{ id: "f1", name: "角色", count: 0, children: [] }] },
      list_placements: [],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
};
window.__cbId = 0; window.__cbs = {};
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
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "--disk-cache-size=1",
        f"--user-data-dir={PROFILE}", "about:blank",
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 注入桩 → 加载主窗口
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                ok = await ev("!!document.querySelector('.window .dv-tabs-and-actions-container .dv-tab .tab')")
                if ok is True:
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            BAR = ".anikit-dock-theme .dv-tabs-and-actions-container"

            bar = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('" + BAR + "'));"
                " return { h: s.height, p: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft].join(','),"
                "        bg: s.backgroundColor, gap: s.columnGap }; })()")
            check("T1 栏容器 46 高 + padding 8·24·0·36 + 透明",
                  isinstance(bar, dict) and bar.get("h") == "46px"
                  and bar.get("p") == "8px,24px,0px,36px" and bar.get("bg") == "rgba(0, 0, 0, 0)",
                  json.dumps(bar))
            check("T2 栏行 gap 12", isinstance(bar, dict) and bar.get("gap") == "12px",
                  json.dumps(bar) if not isinstance(bar, dict) else bar.get("gap", ""))

            tabs_gap = await ev(
                "getComputedStyle(document.querySelector('" + BAR + " .dv-tabs-container')).columnGap")
            check("T3 标签列表 gap 4", tabs_gap == "4px", str(tabs_gap))

            tab_shell = await ev(
                "(() => { const t = document.querySelector('" + BAR + " .dv-tabs-container > .dv-tab');"
                " const s = getComputedStyle(t); const r = t.getBoundingClientRect();"
                " return { w: r.width, h: r.height, p: s.padding, bw: s.borderTopWidth, bg: s.backgroundColor,"
                "        br: s.borderTopLeftRadius }; })()")
            check("T4 外壳 180×38 + 无 padding/边框 + 透明",
                  isinstance(tab_shell, dict) and tab_shell.get("w") == 180 and tab_shell.get("h") == 38
                  and tab_shell.get("p") == "0px" and tab_shell.get("bw") == "0px"
                  and tab_shell.get("bg") == "rgba(0, 0, 0, 0)" and tab_shell.get("br") == "0px",
                  json.dumps(tab_shell))

            # T5 选中标签（初始唯一 tab 即 active）
            sel = await ev(
                "(() => { const t = document.querySelector('" + BAR + " .dv-tab .tab');"
                " const s = getComputedStyle(t);"
                " const lb = getComputedStyle(t.querySelector('.label'));"
                " const x = getComputedStyle(t.querySelector('.x'));"
                " const b = getComputedStyle(t, '::before'); const a = getComputedStyle(t, '::after');"
                " return { cls: t.className, bg: s.backgroundColor,"
                "        r: [s.borderTopLeftRadius, s.borderTopRightRadius, s.borderBottomRightRadius, s.borderBottomLeftRadius].join(','),"
                "        lc: lb.color, xo: x.opacity,"
                "        bw: b.width, bh: b.height, bi: b.backgroundImage.slice(0, 16), bl: b.left,"
                "        aw: a.width, ai: a.backgroundImage.slice(0, 16), ar: a.right }; })()")
            ok = (isinstance(sel, dict) and "selected" in (sel.get("cls") or "")
                  and sel.get("bg") == "rgb(23, 23, 23)" and sel.get("r") == "8px,8px,0px,0px"
                  and sel.get("lc") == "rgb(242, 242, 242)" and sel.get("xo") == "1"
                  and sel.get("bw") == "4px" and sel.get("bh") == "4px"
                  and "radial-gradient" in (sel.get("bi") or "") and sel.get("bl") == "-4px"
                  and sel.get("aw") == "4px" and "radial-gradient" in (sel.get("ai") or "")
                  and sel.get("ar") == "-4px")
            check("T5 选中 = g1 底 + 圆角 8 8 0 0 + 一级字 + 关闭钮显 + 两侧 4px 反弧", ok, json.dumps(sel))

            # 添加钮规格
            add = await ev(
                "(() => { const b = document.querySelector('" + BAR + " .dv-left-actions-container .icon-btn');"
                " const r = b.getBoundingClientRect(); const s = getComputedStyle(b);"
                " return { w: r.width, h: r.height, c: s.color, br: s.borderTopLeftRadius }; })()")
            check("T8 添加钮 30×30 + 亮/三级 + 圆角 6",
                  isinstance(add, dict) and add.get("w") == 30 and add.get("h") == 30
                  and add.get("c") == "rgb(128, 128, 128)" and add.get("br") == "6px",
                  json.dumps(add))

            # dockview 自绘滚动条恒关（flex trailing gap 假溢出 4px 会冒 4px 灰带，截图实证）
            ul = await ev(
                "(() => { const u = document.querySelector('.dv-scrollbar');"
                " return u ? getComputedStyle(u).display : 'absent'; })()")
            check("T9 自绘滚动条不渲染", ul == "none" or ul == "absent", str(ul))

            # 点 + 钮开第二标签 → 第一枚转未选中
            await ev("document.querySelector('" + BAR + " .dv-left-actions-container .icon-btn').click()")
            deadline = time.time() + 8
            while time.time() < deadline:
                n = await ev("document.querySelectorAll('" + BAR + " .dv-tabs-container > .dv-tab').length")
                if n == 2:
                    break
                await asyncio.sleep(0.3)
            await asyncio.sleep(0.6)

            unsel = await ev(
                "(() => { const t = document.querySelector('" + BAR + " .dv-tabs-container > .dv-tab .tab');"
                " const s = getComputedStyle(t); const lb = getComputedStyle(t.querySelector('.label'));"
                " const x = getComputedStyle(t.querySelector('.x'));"
                " return { cls: t.className, bg: s.backgroundColor, lc: lb.color, xo: x.opacity }; })()")
            check("T6 未选中 = 透明底 + 三级字 + 关闭钮隐",
                  isinstance(unsel, dict) and "selected" not in (unsel.get("cls") or "")
                  and unsel.get("bg") == "rgba(0, 0, 0, 0)" and unsel.get("lc") == "rgb(128, 128, 128)"
                  and unsel.get("xo") == "0",
                  json.dumps(unsel))

            # T7 hover 未选中标签（第一枚）：鼠标移到其中心
            rect = await ev(
                "(() => { const r = document.querySelector('" + BAR + " .dv-tabs-container > .dv-tab')"
                ".getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()")
            if isinstance(rect, dict):
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseMoved", "x": rect["x"], "y": rect["y"]}, session=sess)
                await asyncio.sleep(0.4)
            hov = await ev(
                "(() => { const t = document.querySelector('" + BAR + " .dv-tabs-container > .dv-tab .tab');"
                " const a = getComputedStyle(t, '::after'); const x = getComputedStyle(t.querySelector('.x'));"
                " return { bg: a.backgroundColor, br: a.borderTopLeftRadius,"
                "        i: [a.top, a.right, a.bottom, a.left].join(','), xo: x.opacity }; })()")
            check("T7 hover 未选中 = 4px 内缩 6px 圆角 半透明-3 罩 + 关闭钮显",
                  isinstance(hov, dict) and hov.get("bg") == "rgba(242, 242, 242, 0.06)"
                  and hov.get("br") == "6px" and hov.get("i") == "4px,4px,4px,4px"
                  and hov.get("xo") == "1",
                  json.dumps(hov))

            # 截图留档（hover 未选中态）
            s = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(s["result"]["data"]))
            print("saved", SHOT)
    finally:
        edge.terminate()
        try:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(edge.pid)], capture_output=True)
        except Exception:
            pass
        if spawned:
            spawned.kill()
            kill_listening_port(5373)

    print()
    if fails:
        print(f"FAILED {len(fails)} 项：")
        for f in fails:
            print(" -", f)
        return 1
    print("ALL PASS")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
