# CDP 探针：7.1 十八轮 · 左侧边栏轮（2026-08-30）——菜单栏 / 分段控制器 / 树容器贴稿断言
#   S1  侧栏宽 = 254（ui.leftWidth 新默认；桩 get_ui_settings = {}）
#   S2  菜单栏 46 高 / padding 8 12 / gap 2；收起钮 30×30；四文字钮 ghost sm（28 高 / 15px / 三级色）
#   S3  分段控制器区块 54 / 容器 38 半透明-3 底圆角 8；三段均分；thumb 半透明-1 在资源库位
#   S4  切换：点工作空间 → 空态「功能正在开发中」+ 树消失；点回资源库恢复（2026-08-30 用户定）
#   S5  快速访问六项 26 高圆角 6；「全部」数量 = 3（桩 3 条 Placement）；数量 Fira Mono
#   S6  文件夹区块标题 SectionTitle 16 高：「目录 / 2」；常态动作组 visibility hidden（hover 才现身）
#   S7  树 item 26 高；f2 自定义色 blue → .tt-icon[data-tint="blue"]（旧色名映射新九色）
#   S8  选中态：点 f1 → 半透明-2 底 + 立体感-通用 + 文字升一级
#   S9  展开 f1：子层出现 + 缩进线 1px 半透明-2（二轮改定：半透明-3 太淡）；f1 数量 3 → 2（展开 = 直接子项，双口径）
#   S9d 展开 f2（蓝色）：tint 缩进线 = 颜色-2 实色 rgb(37,80,118)（三轮改定：-1 亮档 14% 看不清）；
#       档案子项提示装饰圆点 = 颜色-2（tint 变色后口径；tint+选中 = 颜色-1 走预览页陈列）
#   S10 旧结构清零：.bar-title / .rail / .side-content / .library-switch / .side-thumb 均不存在
#   截图 r18-sidebar-left.png（整窗 + 侧栏局部）
# 桩：最小主窗口；2 区块顶级文件夹（f1 带子层、f2 蓝色）+ 3 条 Placement（f1×2 / f1a×1）。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；桩返回 structuredClone 深拷贝（速查 §12）；
# 收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-sidebar-left-probe.py
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
PORT = 19770 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-sbleft-{os.getpid()}")
SHOT = "tools/style-preview/r18-sidebar-left.png"
SHOT_FULL = "tools/style-preview/r18-sidebar-left-full.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 3 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "角色", count: 3, children: [{ id: "f1a", name: "主角", count: 1, children: [] }] },
          { id: "f2", name: "场景", count: 0, children: [{ id: "f2a", name: "子档案", count: 0, children: [], kind: "archive" }], color: "blue" },
        ] },
      list_placements: [
        { assetId: "a1", folderId: "f1", ext: "png", thumbKind: null, thumbPath: null },
        { assetId: "a2", folderId: "f1", ext: "png", thumbKind: null, thumbPath: null },
        { assetId: "a3", folderId: "f1a", ext: "fla", thumbKind: null, thumbPath: null },
      ],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [], list_relations: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(v === null ? null : structuredClone(v));
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

            # 注入桩并加载
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                ok = await ev("!!document.querySelector('.sidebar-left .library-view')")
                if ok is True:
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            # ---------- S1 侧栏宽 254 ----------
            w = await ev("document.querySelector('.sidebar-left').offsetWidth")
            check("S1 侧栏宽 = 254", w == 254, f"offsetWidth = {w}")

            # ---------- S2 菜单栏 ----------
            mb = await ev(
                "(() => { const el = document.querySelector('.menu-bar'); const s = getComputedStyle(el);"
                " return { h: el.offsetHeight, p: s.padding, gap: s.gap }; })()")
            check("S2a 菜单栏 46 高 / padding 8 12 / gap 2",
                  isinstance(mb, dict) and mb.get("h") == 46 and mb.get("p") == "8px 12px" and mb.get("gap") == "2px",
                  json.dumps(mb))
            collapse = await ev(
                "(() => { const el = document.querySelector('.menu-bar .ic-btn.plain');"
                " return el ? { w: el.offsetWidth, h: el.offsetHeight } : null; })()")
            check("S2b 收起侧边栏钮 30×30",
                  isinstance(collapse, dict) and collapse.get("w") == 30 and collapse.get("h") == 30,
                  json.dumps(collapse))
            btns = await ev(
                "[...document.querySelectorAll('.menu-bar .act-btn.ghost.sm')].map(b => {"
                " const s = getComputedStyle(b); return { t: b.textContent.trim(), h: b.offsetHeight,"
                " fs: s.fontSize, c: s.color }; })")
            labels = [b.get("t") for b in btns] if isinstance(btns, list) else []
            check("S2c 四文字钮 = 菜单/编辑/插件/视图",
                  labels == ["菜单", "编辑", "插件", "视图"], json.dumps(labels))
            check("S2d 文字钮 28 高 / 15px / 三级色",
                  isinstance(btns, list) and len(btns) == 4 and all(
                      b.get("h") == 28 and b.get("fs") == "15px" and b.get("c") == "rgb(128, 128, 128)"
                      for b in btns),
                  json.dumps(btns))

            # ---------- S3 分段控制器 ----------
            seg = await ev(
                "(() => { const w = document.querySelector('.seg-wrap');"
                " const el = w.querySelector('.seg.icon'); const s = getComputedStyle(el);"
                " return { wh: w.offsetHeight, h: el.offsetHeight, bg: s.backgroundColor, r: s.borderRadius }; })()")
            check("S3a 分段区块 54 / 容器 38 半透明-3 底圆角 8",
                  isinstance(seg, dict) and seg.get("wh") == 54 and seg.get("h") == 38
                  and seg.get("bg") == "rgba(242, 242, 242, 0.06)" and seg.get("r") == "8px",
                  json.dumps(seg))
            items = await ev(
                "[...document.querySelectorAll('.seg-item')].map(b => ({ w: b.offsetWidth, on: b.className.includes('on') }))")
            check("S3b 三段均分 + 资源库选中",
                  isinstance(items, list) and len(items) == 3
                  and abs(items[0]["w"] - items[1]["w"]) <= 1 and abs(items[1]["w"] - items[2]["w"]) <= 1
                  and items[0]["on"] and not items[1]["on"] and not items[2]["on"],
                  json.dumps(items))
            thumb = await ev(
                "(() => { const el = document.querySelector('.seg .thumb'); const s = getComputedStyle(el);"
                " return { bg: s.backgroundColor, sh: s.boxShadow }; })()")
            check("S3c 滑块 = 半透明-1 + 立体感-通用",
                  isinstance(thumb, dict) and thumb.get("bg") == "rgba(242, 242, 242, 0.18)"
                  and "inset" in (thumb.get("sh") or ""),
                  json.dumps(thumb))

            # ---------- S5 快速访问（切换前查） ----------
            quick = await ev(
                "[...document.querySelectorAll('.block.quick .tree-item')].map(el => {"
                " const s = getComputedStyle(el);"
                " return { n: el.querySelector('.nm')?.textContent, c: el.querySelector('.cnt')?.textContent ?? null,"
                " h: el.offsetHeight, r: s.borderRadius,"
                " ff: el.querySelector('.cnt') ? getComputedStyle(el.querySelector('.cnt')).fontFamily : null }; })")
            names = [q.get("n") for q in quick] if isinstance(quick, list) else []
            check("S5a 快速访问六项", names == ["全部", "未分类", "最近使用", "自动导入", "缩略图", "回收站"],
                  json.dumps(names))
            check("S5b item 26 高 / 圆角 6",
                  isinstance(quick, list) and len(quick) == 6
                  and all(q.get("h") == 26 and q.get("r") == "6px" for q in quick),
                  json.dumps([{ "h": q.get("h"), "r": q.get("r") } for q in quick] if isinstance(quick, list) else quick))
            check("S5c 「全部」数量 = 3（桩 3 条 Placement）+ Fira Mono",
                  isinstance(quick, list) and quick[0].get("c") == "3"
                  and "Fira Mono" in (quick[0].get("ff") or ""),
                  json.dumps(quick[0] if isinstance(quick, list) else quick))

            # ---------- S6 文件夹区块标题 ----------
            st = await ev(
                "(() => { const el = document.querySelector('.block.tree .st.block');"
                " if (!el) return null; const acts = el.querySelector('.acts');"
                " return { h: el.offsetHeight, txt: el.querySelector('.tt')?.textContent?.replace(/\\s+/g, ''),"
                " vis: acts ? getComputedStyle(acts).visibility : null }; })()")
            check("S6a 区块标题 16 高 + 「目录/2」",
                  isinstance(st, dict) and st.get("h") == 16 and st.get("txt") == "目录/2",
                  json.dumps(st))
            check("S6b 常态动作组 hidden（hover 才现身，稿 prose）",
                  isinstance(st, dict) and st.get("vis") == "hidden", json.dumps(st))

            # ---------- S7 树 item + tint ----------
            rows = await ev(
                "[...document.querySelectorAll('.block-tree .tree-item')].map(el => ({"
                " n: el.querySelector('.nm')?.textContent, h: el.offsetHeight,"
                " tint: el.querySelector('.tt-icon')?.getAttribute('data-tint') ?? null }))")
            check("S7a 树 item 26 高（两顶级）",
                  isinstance(rows, list) and len(rows) == 2 and all(r.get("h") == 26 for r in rows),
                  json.dumps(rows))
            check("S7b f2 自定义色 blue → data-tint=blue（旧色名映射）",
                  isinstance(rows, list) and len(rows) == 2 and rows[1].get("tint") == "blue",
                  json.dumps(rows))

            # ---------- S8 选中态（点 f1） ----------
            await ev("document.querySelector('.block-tree .tree-item[data-folder-id=\"f1\"]').dispatchEvent("
                     "new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.4)
            sel = await ev(
                "(() => { const el = document.querySelector('.tree-item[data-folder-id=\"f1\"]');"
                " const s = getComputedStyle(el);"
                " return { cls: el.className, bg: s.backgroundColor, sh: s.boxShadow, c: s.color }; })()")
            check("S8 选中 = 半透明-2 底 + 立体感-通用 + 提亮一级",
                  isinstance(sel, dict) and "selected" in (sel.get("cls") or "")
                  and sel.get("bg") == "rgba(242, 242, 242, 0.14)"
                  and "inset" in (sel.get("sh") or "")
                  and sel.get("c") == "rgb(242, 242, 242)",
                  json.dumps(sel))

            # ---------- S9 展开 f1：子层 + 缩进线 + 数量双口径 ----------
            await ev("document.querySelector('.tree-item[data-folder-id=\"f1\"] .exp').dispatchEvent("
                     "new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.4)
            exp = await ev(
                "(() => { const ch = document.querySelector('.ft-children');"
                " if (!ch) return { children: false };"
                " const line = getComputedStyle(ch, '::before');"
                " const cnt = document.querySelector('.tree-item[data-folder-id=\"f1\"] .cnt');"
                " return { children: true, lw: line.width, lc: line.backgroundColor,"
                " cnt: cnt?.textContent ?? null, sub: !!document.querySelector('.tree-item[data-folder-id=\"f1a\"]') }; })()")
            check("S9a 展开出子层 + 子项 f1a 渲染",
                  isinstance(exp, dict) and exp.get("children") is True and exp.get("sub") is True,
                  json.dumps(exp))
            check("S9b 缩进线 1px 常态-2（四轮改定：--n-custom-2 实色）",
                  isinstance(exp, dict) and exp.get("lw") == "1px"
                  and exp.get("lc") == "rgb(102, 102, 102)",
                  json.dumps(exp))
            check("S9c 展开后 f1 数量 = 直接子项 2（双口径）",
                  isinstance(exp, dict) and exp.get("cnt") == "2", json.dumps(exp))

            # ---------- S9d tint 缩进线（颜色-2 实色）+ 档案圆点（颜色-2） ----------
            await ev("document.querySelector('.tree-item[data-folder-id=\"f2\"] .exp').dispatchEvent("
                     "new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.4)
            tinted = await ev(
                "(() => { const ch = [...document.querySelectorAll('.ft-children')]"
                "   .find(c => c.querySelector('[data-folder-id=\"f2a\"]'));"
                " if (!ch) return { children: false };"
                " const dot = document.querySelector('.tree-item[data-folder-id=\"f2a\"] .dot');"
                " return { children: true, line: getComputedStyle(ch, '::before').backgroundColor,"
                "   dc: dot ? getComputedStyle(dot, '::before').backgroundColor : null }; })()")
            check("S9d tint 缩进线 = 颜色-2 实色；档案圆点 = 颜色-2",
                  isinstance(tinted, dict) and tinted.get("line") == "rgb(37, 80, 118)"
                  and tinted.get("dc") == "rgb(37, 80, 118)",
                  json.dumps(tinted))

            # ---------- S4 分段切换 → 空态 ----------
            await ev("[...document.querySelectorAll('.seg-item')][1].click()")
            await asyncio.sleep(0.6)
            sw = await ev(
                "({ empty: document.querySelector('.empty-state')?.textContent ?? null,"
                "  tree: !!document.querySelector('.library-view') })")
            check("S4a 切工作空间 → 空态「功能正在开发中」+ 树消失",
                  isinstance(sw, dict) and sw.get("empty") == "功能正在开发中" and sw.get("tree") is False,
                  json.dumps(sw))
            await ev("[...document.querySelectorAll('.seg-item')][0].click()")
            await asyncio.sleep(0.6)
            back = await ev("!!document.querySelector('.library-view')")
            check("S4b 切回资源库 → 树恢复", back is True, str(back))

            # ---------- S10 旧结构清零（限左栏内；.bar-title/.side-content 等类名右栏 SidebarRight 仍在用） ----------
            legacy = await ev(
                "['.bar-title', '.rail', '.side-content', '.library-switch', '.side-thumb']"
                ".filter(s => document.querySelector('.sidebar-left ' + s))")
            check("S10 旧结构类名清零", isinstance(legacy, list) and len(legacy) == 0, json.dumps(legacy))

            # ---------- 截图留档 ----------
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT_FULL, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            clip = await ev(
                "(() => { const r = document.querySelector('.sidebar-left').getBoundingClientRect();"
                " return { x: r.x, y: r.y, width: r.width, height: r.height }; })()")
            shot2 = await send(ws, "Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": clip["x"], "y": clip["y"], "width": clip["width"],
                         "height": min(clip["height"], 900), "scale": 2},
            }, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot2["result"]["data"]))
            print("saved", SHOT, "/", SHOT_FULL)
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
