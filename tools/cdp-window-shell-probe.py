# CDP 探针：7.1 十七轮 · 主窗口骨架轮（2026-08-30）——窗口壳 + 三段底色结构断言
#   W1  mica 模式：.window 挂 fx-mica 类 + 底 = 全透明（2026-08-30 十七轮同日观感定稿：
#       工程里云母上不叠色，DWM 云母直接呈现；稿 93% 半透明-1 是 figma 模拟手法）
#   W2  窗口描边（2026-08-30 同日五轮定稿：云母模式 = DWM 染色线 + 前端 ::before 同色
#       遮罩环——1px 边框线 + 1.5px 外扩阴影同为边框色，圆角 AA 过渡带整圈淹没成边框色、
#       内容边缘退入弧内，四角缺口消（纯 DWM 画边角弧 AA 咬内容色成缺口，100%/125%/150%
#       实机均见；同色重叠合一故无双层）；常态 #333、picking 换品牌色（Rust
#       set_main_border_picking 同步切染 DWM）；实色模式 DWM 无边框，前端自绘）：
#       mica = .window radius 0 + ::before 遮罩环 1px+阴影 / picking 环换品牌色 + ::after 0；
#       solid = W7c ::before 1px rgb(51,51,51) + 圆角 8 / W7d picking ::after 1.5px 品牌色
#   W3  右栏：g1 rgb(23,23,23) + 左缘 1px 通用/三级 rgb(36,36,36)
#   W4  左栏壳透明：menu-bar / seg-wrap / side-body 均 rgba(0,0,0,0)（透窗口底；7.1 十八轮左栏轮新类名）
#   W5  标签栏透明（dv-tabs-and-actions-container rgba(0,0,0,0)）
#   W6  内容容器卡片：g1 + 首 group 左上 16px 圆角 + 左缘 1px 亮/半透明-2 rgba(242,242,242,0.14)
#   W7  solid 模式（重载换桩）：无 fx-mica 类 + 底 = 实色-1 rgb(31,31,31)
#   截图 r17-window-shell.png（mica 轮整窗；headless 无 DWM 云母，仅结构留档）
# 桩：最小主窗口（空 placements）；get_window_fx_mode 由桩常量 FX 控制两轮各跑。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-window-shell-probe.py
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
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-winshell-{os.getpid()}")
SHOT = "tools/style-preview/r17-window-shell.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 1 },
      get_ui_settings: {},
      get_window_fx_mode: "__FX__",
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

            async def load_with_fx(mode):
                """换桩重载：移除旧注入 → 注入 FX=mode 桩 → navigate"""
                lst = await send(ws, "Page.removeScriptToEvaluateOnNewDocument",
                                 {"identifier": getattr(load_with_fx, "sid", "")}, session=sess) \
                    if getattr(load_with_fx, "sid", None) else None
                _ = lst
                r = await send(ws, "Page.addScriptToEvaluateOnNewDocument",
                               {"source": STUB.replace("__FX__", mode)}, session=sess)
                load_with_fx.sid = r["result"]["identifier"]
                await send(ws, "Page.navigate", {"url": URL}, session=sess)
                # 等主窗口渲染锚点（.window + dockview 内容容器）
                deadline = time.time() + 15
                while time.time() < deadline:
                    ok = await ev("!!document.querySelector('.window .dv-content-container') && !!document.querySelector('.sidebar-right')")
                    if ok is True:
                        break
                    await asyncio.sleep(0.4)
                await asyncio.sleep(1.0)

            async def bg(sel):
                return await ev(f"getComputedStyle(document.querySelector('{sel}')).backgroundColor")

            # ---------- mica 轮 ----------
            await load_with_fx("mica")
            cls = await ev("document.querySelector('.window').className")
            check("W1a fx-mica 类挂载", "fx-mica" in (cls or ""), cls or "无 .window")
            wbg = await bg(".window")
            check("W1b .window fx-mica 底 = 全透明（云母直呈）", wbg == "rgba(0, 0, 0, 0)", str(wbg))
            before = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('.window'), '::before');"
                " return { w: s.borderTopWidth, c: s.borderTopColor, r: s.borderTopLeftRadius,"
                "        sh: s.boxShadow, pe: s.pointerEvents }; })()")
            check("W2 mica 遮罩环 = 边框色 1px 线 + 1.5px 外扩阴影同色 + 圆角 7"
                  "（边框色淹没圆角 AA 带消四角缺口；与 DWM 染色线同色合一无双层，五轮）",
                  isinstance(before, dict) and before.get("w") == "1px"
                  and before.get("c") == "rgb(51, 51, 51)"
                  and before.get("r") == "7px"
                  and "rgb(51, 51, 51)" in (before.get("sh") or "")
                  and "1.5px" in (before.get("sh") or "")
                  and before.get("pe") == "none",
                  json.dumps(before))
            wr = await ev("getComputedStyle(document.querySelector('.window')).borderTopLeftRadius")
            check("W2b mica .window 圆角 0（DWM 独裁，双裁切 AA 透缝 BUG 根）", wr == "0px", str(wr))
            # picking 组：手动挂类断言 CSS 层（不进真 store；DWM 切染品牌色为 Rust 侧行为）
            await ev("document.querySelector('.window').classList.add('picking')")
            pb = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('.window'), '::before');"
                " return { w: s.borderTopWidth, c: s.borderTopColor, sh: s.boxShadow }; })()")
            check("W2c mica picking 遮罩环换品牌色（与 DWM 切染品牌线同色合一）",
                  isinstance(pb, dict) and pb.get("w") == "1px"
                  and pb.get("c") == "rgb(61, 221, 210)"
                  and "rgb(61, 221, 210)" in (pb.get("sh") or ""),
                  json.dumps(pb))
            pa = await ev("getComputedStyle(document.querySelector('.window'), '::after').borderTopWidth")
            check("W2d mica picking ::after 零画（品牌框 = DWM 染色线 + ::before 品牌遮罩环承担）",
                  pa == "0px", str(pa))
            bs = await ev("getComputedStyle(document.documentElement).getPropertyValue('--border-strong').trim()")
            check("W2e 品牌框宽档 --border-strong = 1.5px（稿值源头）", bs == "1.5px", str(bs))
            await ev("document.querySelector('.window').classList.remove('picking')")
            rbg = await bg(".sidebar-right")
            rbd = await ev("(() => { const s = getComputedStyle(document.querySelector('.sidebar-right'));"
                           " return s.borderLeftWidth + ' ' + s.borderLeftColor; })()")
            check("W3a 右栏底 = g1", rbg == "rgb(23, 23, 23)", str(rbg))
            check("W3b 右栏左缘 1px g3", rbd == "1px rgb(36, 36, 36)", str(rbd))
            for sel, tag in [(".sidebar-left .menu-bar", "菜单栏"), (".sidebar-left .seg-wrap", "分段控制器"),
                             (".sidebar-left .side-body", "侧栏树容器")]:
                v = await bg(sel)
                check(f"W4 左栏{tag}透明", v == "rgba(0, 0, 0, 0)", f"{sel} = {v}")
            tbg = await bg(".anikit-dock-theme .dv-tabs-and-actions-container")
            check("W5 标签栏透明", tbg == "rgba(0, 0, 0, 0)", str(tbg))
            cbg = await bg(".dv-content-container")
            check("W6a 内容容器底 = g1", cbg == "rgb(23, 23, 23)", str(cbg))
            cc = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('.dv-content-container'));"
                " return { r: s.borderTopLeftRadius, l: s.borderLeftWidth + ' ' + s.borderLeftColor }; })()")
            check("W6b 首 group 左上 16px 圆角 + 左缘 亮/半透明-2",
                  isinstance(cc, dict) and cc.get("r") == "16px" and cc.get("l") == "1px rgba(242, 242, 242, 0.14)",
                  json.dumps(cc))

            # 截图留档（mica 轮整窗）
            s = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(s["result"]["data"]))
            print("saved", SHOT)

            # ---------- solid 轮 ----------
            await load_with_fx("solid")
            cls2 = await ev("document.querySelector('.window').className")
            check("W7a solid 无 fx-mica 类", "fx-mica" not in (cls2 or ""), cls2 or "无 .window")
            wbg2 = await bg(".window")
            check("W7b .window 底 = 实色-1", wbg2 == "rgb(31, 31, 31)", str(wbg2))
            b7 = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('.window'), '::before');"
                " return { w: s.borderTopWidth, c: s.borderTopColor, r: s.borderTopLeftRadius }; })()")
            check("W7c solid 描边 1px 通用/六级实心 rgb(51,51,51) + 圆角 8（前端自裁）",
                  isinstance(b7, dict) and b7.get("w") == "1px"
                  and b7.get("c") == "rgb(51, 51, 51)"
                  and b7.get("r") == "8px",
                  json.dumps(b7))
            # solid 轮 picking：品牌框前端自绘（云母模式才交 DWM 染色线）
            await ev("document.querySelector('.window').classList.add('picking')")
            pa7 = await ev(
                "(() => { const s = getComputedStyle(document.querySelector('.window'), '::after');"
                " return { w: s.borderTopWidth, st: s.borderTopStyle, c: s.borderTopColor, r: s.borderTopLeftRadius }; })()")
            # 宽度容差：getComputedStyle 对 border-width 返回 used value，headless DPR=1 下
            # 1.5px 被量化成 1px（TreeRowItem 二轮同款量化坑）；稿值源头由 W2e 锁变量档
            check("W7d solid picking 品牌框自绘 1.5px 品牌色 + 圆角 8",
                  isinstance(pa7, dict) and pa7.get("st") == "solid"
                  and pa7.get("w") in ("1px", "1.5px")
                  and pa7.get("c") == "rgb(61, 221, 210)"
                  and pa7.get("r") == "8px",
                  json.dumps(pa7))
            await ev("document.querySelector('.window').classList.remove('picking')")
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
