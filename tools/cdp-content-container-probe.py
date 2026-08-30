# CDP 探针：7.1 二十一轮 · 中间面板-内容容器轮（2026-08-30）——工具栏 / 三区列表容器 / 列表模式 / 空态贴稿断言
#   T1  工具栏容器：54 高（30 内容 + padding 12×2）/ padding 12 / 无底色 / space-between
#   T2  面包屑组 gap 12 + 返回前进组 gap 2 + 钮 30×30 圆角 6 图标 18 三级色 + 前进 flip-h
#       （svg computed transform = scaleX(-1)）+ 初始无历史双禁用 + 非搜索态无 result-group
#   T3  尺寸滑块：wrap 100 宽 + BarSlider tone=size（轨道 3px g3 #242424——稿滑轨 = 3px 线，
#       用户实机纠正 / 拖钮 10px 圆 二级 无边框）
#   T4  按钮组 gap 2 + 4 钮 30×30 + 插件业务禁用 + 右栏展开态无第五钮
#   T4b 收起右栏 → 第五钮「展开详细信息」现（collapse-sidebar flip-h）；点它 → 右栏展开 + 钮消失
#   T5  列表容器 padding：asset-section 上 12 / 左 12 / 下 0 / 右 0（稿右 12 由布局 PAD.right 承担——
#       滚动条贴右缘口径，ContentArea 注释在案）+ 网格最右卡右缘距视口右缘 12（±1）
#   T6  文件卡：壳 box 绝对定位 + 芯 FileCard g2 #1C1C1C 圆角 8 + .kc-clip 裁剪层 + 渐变罩
#       （名称 13/18 Bold 一级 + 简介 mono 12/16 二级 + padding 24/12/12）+ fla 卡 FormatBadge
#       左上 12/12（kc-float 半透明-2 底 + 圆角 4 + mono 12/16 Bold 二级）+ png 静态图无徽标
#   T7  文件夹区块标题（SectionTitle block）：16 高 / padding-left 8 /「文件夹/1」12/16 Bold 四级 /
#       actions=false 无 .acts + tail「递归显示」复选框（label 12/16）+ 展开钮 14 常态 hidden
#   T7b hover 标题行（Input.dispatchMouseEvent 真实移动）→ 展开钮现 + 抓手指针
#   T8  文件夹卡：grid auto-rows 122 + gap 12 + 卡 122 高 宽 ≥220 + g3 底圆角 8 padding 12 +
#       名称 18/26 Bold 一级 + folder-large 24 双色调无 tint + 简介两行 mono 12/16 三级 +
#       文本精确「2 个文件/500.0 MB」「1 个档案/1 个子文件夹」（桩 f1a 带 totalSize + 双类子级）
#   T9  档案区：标题「档案/1」无 .acts + 卡正方形（offsetWidth===offsetHeight ≥250）+ flow gap 12 +
#       g2 底圆角 8 + 罩名称 18/26 Bold 一级 + 属性行「立绘|彩色」/ 数量行「2 个文件/1.25 GB」
#       mono 12/16 二级 + 徽标「主角」左上 12/12 统一铬
#   T10 内容标题「内容/5」（递归显示开 = f1 子树全部 5 条）
#   T4c 文件夹视口下「添加文件/档案链接」钮解禁（落点 = 本视口；分组管理钮递归显示时禁用）
#   T11 后退/前进接线：进 f1 后后退解禁 → 点后退回 quick:all（文件夹区消失、前进解禁）→ 点前进回 f1
#   T12 关「递归显示」→ 内容变 3 + 分组标题两枚（SectionTitle group 26 高 18/26 Bold 二级：
#       未分组/2、图片/1——桩 f1 带 groups 规则 type=image）
#   T13 列表表头：34 高 g3 底圆角 8 padding 8/12 gap 12 + 名称列 flex 1 + 四数据列各 123 +
#       标签 13/18 Bold 三级
#   T14 列表行：57 高 padding 6/12/6/6 gap 12 g2 底圆角 8 + 缩略图 80×45（g3 + g5 描边圆角 6）+
#       数据列 123 mono 数字-大 三级 + 名称 13/18 Bold 一级 + 文件大小/修改日期真值；
#       T14d 真实点击行 → 选中（2026-08-30 列表选中回归，壳 list-row 标记类补回）+
#       T14e 格式图标行 icon 档 50% 居中（a2 桩 thumbKind=formatIcon）
#   T15 排序接线：点「文件大小」→ 列 active + 行序升序（首行 50.0 KB）；再点 → 箭头 desc +
#       降序（首行 5.0 MB）；点「尺寸」列 → 无反应（无 SortKey 档纯展示）
#   T15b 表头右缘 = 行右缘 = 视口右缘 -12（钉顶列头与行严格对齐）
#   T16 搜索态计数：输入「跑步」→ 桩 search_assets（2 文件 + 1 档案）→ 工具栏「搜索结果 / 3」
#       （label/sep 正文 13/18 二级 + count mono 13/18 三级）
#   T16b 清空搜索 → 计数消失
#   T17 空文件夹（f-empty 三区皆空）→ .ef-root +「暂无文件」18/26 Bold 二级 + 提示
#       「将文件拖拽至此处导入」13/18 三级 + 双钮（导入文件/导入文件夹 = ActionButton solid md
#       34 高 g4 #292929 圆角 8）+ 按钮组 282 宽 gap 8
#   截图 r21-content-container-{full,grid,list,search,empty}.png
# 桩：5 Placement（f1×3 + f1a×1 + f1a1×1）+ 树（f1[子 f1a[子 f1a1 + 档案 ar1sub] + 档案 ar1] +
#   空文件夹 f-empty）；list_archives 带 badge/props/totalSize；search_assets 固定命中；
#   全量清单补齐（list_relations 缺了 DetailPanel 会崩）+ structuredClone 深拷贝（速查 §12）。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；hover 用 Input.dispatchMouseEvent 真实移动；
#   收尾 taskkill /F /T 整树杀（踩坑 056）；文案引号一律「」防撞 Python 定界。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-content-container-probe.py
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
PORT = 19340 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-content-{os.getpid()}")
SHOT_DIR = "tools/style-preview"

# 内容区工具栏（SidebarRight 也用 .panel-toolbar，一律锚 .content-area 域内）
TB = ".content-area .panel-toolbar"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const P = (o) => Object.assign({
      w: null, h: null, thumbW: null, thumbH: null, thumbPath: null, thumbKind: null,
      origin: true, frameCount: null, animated: false, duration: null, bpm: null,
      importedFrom: null, createdAt: null, modifiedAt: "2026-08-24T16:07:00+08:00",
      importedAt: "2026-08-20T10:00:00+08:00", noteCount: null, thumbFrame: null, fps: null,
    }, o);
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 5, folderCount: 5 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "角色", count: 5, totalSize: 9066496,
            groups: [{ id: "g-img", name: "图片", rules: [{ kind: "type", values: ["image"] }] }],
            children: [
            { id: "f1a", name: "线稿", count: 2, totalSize: 524288000, children: [
              { id: "f1a1", name: "草图", count: 1, totalSize: 51200, children: [] },
              { id: "ar1sub", name: "线稿集", kind: "archive", count: 0, totalSize: 0, children: [] },
            ] },
            { id: "ar1", name: "主角", kind: "archive", count: 2, totalSize: 1342177280, children: [] },
          ] },
          { id: "f-empty", name: "空文件夹", count: 0, totalSize: 0, children: [] },
        ] },
      list_placements: [
        P({ assetId: "a1", folderId: "f1", name: "跑步循环.png", w: 1920, h: 1080, ext: "png", size: 2097152 }),
        P({ assetId: "a2", folderId: "f1", name: "主角设定.fla", ext: "fla", size: 5242880,
            thumbKind: "formatIcon", thumbPath: "E:/fake/fla-icon.png" }),
        P({ assetId: "a3", folderId: "f1", name: "参考片段.mp4", w: 1280, h: 720, ext: "mp4", duration: 62.4, size: 1572864 }),
        P({ assetId: "a4", folderId: "f1a", name: "线稿01.png", w: 800, h: 600, ext: "png", size: 102400 }),
        P({ assetId: "a5", folderId: "f1a1", name: "草图01.png", w: 640, h: 480, ext: "png", size: 51200 }),
      ],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_relations: [],
      list_archives: [
        { folderId: "ar1", badge: { text: "主角", color: "#25506e" }, avatar: null,
          props: ["立绘", "彩色"], descriptions: [], links: [], createdAt: "",
          totalSize: 1342177280, coverPath: null },
        { folderId: "ar1sub", badge: null, avatar: null, props: [], descriptions: [],
          links: [], createdAt: "", totalSize: 0, coverPath: null },
      ],
      search_assets: { placements: [{ assetId: "a1", folderId: "f1" }, { assetId: "a3", folderId: "f1" }],
        archives: ["ar1"] },
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

            async def wait_for(expr, timeout=8.0):
                deadline = time.time() + timeout
                while time.time() < deadline:
                    v = await ev(expr)
                    if v is True:
                        return True
                    await asyncio.sleep(0.25)
                return False

            async def shot(path, sel=None, scale=1):
                params = {"format": "png"}
                if sel:
                    clip = await ev(
                        "(() => { const r = document.querySelector('" + sel + "').getBoundingClientRect();"
                        " return { x: r.x, y: r.y, width: r.width, height: r.height }; })()")
                    if isinstance(clip, dict):
                        params["clip"] = {"x": clip["x"], "y": clip["y"], "width": clip["width"],
                                          "height": min(clip["height"], 900), "scale": scale}
                s = await send(ws, "Page.captureScreenshot", params, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(s["result"]["data"]))
                print("saved", path)

            # 注入桩 → 加载主窗口（默认单标签 quick:all，桩 5 条 Placement 全出卡）
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await wait_for("!!document.querySelector('" + TB + "')", 15)
            await wait_for("document.querySelectorAll('.content-area .asset-card').length >= 5", 10)
            await asyncio.sleep(1.0)

            # ================= 阶段 A：quick:all —— 工具栏 + 文件卡 =================
            bar = await ev(
                "(() => { const el = document.querySelector('" + TB + "'); const s = getComputedStyle(el);"
                " return { h: el.offsetHeight, p: [s.paddingTop, s.paddingRight, s.paddingBottom, s.paddingLeft].join(','),"
                "        bg: s.backgroundColor, jc: s.justifyContent }; })()")
            check("T1 工具栏 54 高 + padding 12 + 无底色 + space-between",
                  isinstance(bar, dict) and bar.get("h") == 54
                  and bar.get("p") == "12px,12px,12px,12px"
                  and bar.get("bg") == "rgba(0, 0, 0, 0)" and bar.get("jc") == "space-between",
                  json.dumps(bar))

            nav = await ev(
                "(() => { const g = document.querySelector('" + TB + " .crumb-group');"
                " const pair = g.querySelector('.nav-pair');"
                " const btns = [...pair.querySelectorAll('.ic-btn')];"
                " const s0 = getComputedStyle(btns[0]);"
                " const tf = (b) => getComputedStyle(b.querySelector('svg')).transform;"
                " return { gg: getComputedStyle(g).columnGap, pg: getComputedStyle(pair).columnGap,"
                "  n: btns.length, w0: btns[0].offsetWidth, h0: btns[0].offsetHeight,"
                "  r0: s0.borderTopLeftRadius, c0: s0.color, iw: btns[0].querySelector('.app-icon').offsetWidth,"
                "  t0: tf(btns[0]), t1: tf(btns[1]), f1: btns[1].className.includes('flip-h'),"
                "  d0: btns[0].disabled, d1: btns[1].disabled, rg: !!g.querySelector('.result-group') }; })()")
            check("T2 面包屑组 gap 12 + 返回前进组 gap 2 + 双钮 30×30 圆角 6 图标 18 三级",
                  isinstance(nav, dict) and nav.get("gg") == "12px" and nav.get("pg") == "2px"
                  and nav.get("n") == 2 and nav.get("w0") == 30 and nav.get("h0") == 30
                  and nav.get("r0") == "6px" and nav.get("c0") == "rgb(128, 128, 128)"
                  and nav.get("iw") == 18,
                  json.dumps(nav))
            check("T2b 前进 = angle-bracket flip-h（svg scaleX(-1)）+ 后退原样 + 无历史双禁用 + 无搜索计数",
                  isinstance(nav, dict) and nav.get("f1") is True
                  and nav.get("t1") == "matrix(-1, 0, 0, 1, 0, 0)" and nav.get("t0") == "none"
                  and nav.get("d0") is True and nav.get("d1") is True and nav.get("rg") is False,
                  json.dumps(nav))

            sl = await ev(
                "(() => { const w = document.querySelector('" + TB + " .slider-wrap');"
                " const track = w.querySelector('.bs-track'); const knob = w.querySelector('.bs-knob');"
                " const ts = getComputedStyle(track); const ks = getComputedStyle(knob);"
                " return { ww: w.offsetWidth, size: w.querySelector('.bs-root').className.includes('size'),"
                "  th: ts.height, tbg: ts.backgroundColor, kw: ks.width, kh: ks.height,"
                "  kbg: ks.backgroundColor, kbr: ks.borderRadius, kbw: ks.borderTopWidth }; })()")
            check("T3 尺寸滑块 wrap 100 + 轨道 3px g3 + 拖钮 10px 圆 二级 无边框",
                  isinstance(sl, dict) and sl.get("ww") == 100 and sl.get("size") is True
                  and sl.get("th") == "3px" and sl.get("tbg") == "rgb(36, 36, 36)"
                  and sl.get("kw") == "10px" and sl.get("kh") == "10px"
                  and sl.get("kbg") == "rgb(191, 191, 191)" and sl.get("kbr") == "50%"
                  and sl.get("kbw") == "0px",
                  json.dumps(sl))

            bg4 = await ev(
                "(() => { const g = document.querySelector('" + TB + " .btn-group');"
                " const btns = [...g.querySelectorAll('.ic-btn')];"
                " return { gap: getComputedStyle(g).columnGap, n: btns.length,"
                "  sz: btns.map((b) => b.offsetWidth + 'x' + b.offsetHeight),"
                "  dis: btns.map((b) => b.disabled) }; })()")
            check("T4 按钮组 gap 2 + 4 钮 30×30 + 插件禁用 + 查看方式可用 + 无第五钮",
                  isinstance(bg4, dict) and bg4.get("gap") == "2px" and bg4.get("n") == 4
                  and all(s == "30x30" for s in bg4.get("sz", []))
                  and bg4.get("dis", [])[1] is True and bg4.get("dis", [])[3] is False,
                  json.dumps(bg4))

            # T4b 收起右栏 → 第五钮现；点它 → 展开 + 钮消失
            await ev("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][0].click()")
            await asyncio.sleep(1.2)
            fifth = await ev(
                "(() => { const btns = [...document.querySelectorAll('" + TB + " .btn-group .ic-btn')];"
                " const b = btns[btns.length - 1];"
                " return { n: btns.length, flip: b.className.includes('flip-h'),"
                "  t: getComputedStyle(b.querySelector('svg')).transform }; })()")
            check("T4b 右栏收起 → 第五钮「展开详细信息」现（flip-h）",
                  isinstance(fifth, dict) and fifth.get("n") == 5 and fifth.get("flip") is True
                  and fifth.get("t") == "matrix(-1, 0, 0, 1, 0, 0)",
                  json.dumps(fifth))
            await ev("[...document.querySelectorAll('" + TB + " .btn-group .ic-btn')][4].click()")
            await asyncio.sleep(1.2)
            back4 = await ev("document.querySelectorAll('" + TB + " .btn-group .ic-btn').length")
            check("T4b2 点第五钮 → 右栏展开 + 钮消失", back4 == 4, str(back4))

            # T5 列表容器 padding + 网格最右卡右缘（稿右 12 = PAD.right 口径）
            sec = await ev(
                "(() => { const el = document.querySelector('.content-area .asset-section');"
                " const s = getComputedStyle(el);"
                " return { pt: s.paddingTop, pr: s.paddingRight, pb: s.paddingBottom, pl: s.paddingLeft }; })()")
            check("T5a 列表容器 padding 上12 左12 下0 右0（右 12 归布局 PAD.right，滚动条贴缘）",
                  isinstance(sec, dict) and sec.get("pt") == "12px" and sec.get("pl") == "12px"
                  and sec.get("pb") == "0px" and sec.get("pr") == "0px",
                  json.dumps(sec))
            edge5 = await ev(
                "(() => { const vp = document.querySelector('.content-area .grid-viewport').getBoundingClientRect();"
                " const rs = [...document.querySelectorAll('.content-area .asset-card')]"
                "   .map((c) => c.getBoundingClientRect().right);"
                " return { gap: vp.right - Math.max(...rs) }; })()")
            check("T5b 网格最右卡右缘距视口右缘 12（±1）",
                  isinstance(edge5, dict) and abs(edge5.get("gap", 0) - 12) <= 1,
                  json.dumps(edge5))

            card6 = await ev(
                "(() => { const shell = document.querySelector('.content-area .asset-card[data-key=\"a2|f1\"]');"
                " const root = shell.querySelector('.fc-root'); const rs = getComputedStyle(root);"
                " const clip = root.querySelector('.kc-clip'); const cs = getComputedStyle(clip);"
                " const ov = root.querySelector('.kc-overlay'); const os = getComputedStyle(ov);"
                " const nm = ov.querySelector('.kc-name'); const ns = getComputedStyle(nm);"
                " const inf = ov.querySelector('.kc-info'); const is = getComputedStyle(inf);"
                " const badge = root.querySelector('.fc-badge'); const bs = badge ? getComputedStyle(badge) : null;"
                " const sh = getComputedStyle(shell);"
                " return { pos: sh.position, absL: shell.style.left !== '',"
                "  bg: rs.backgroundColor, br: rs.borderTopLeftRadius,"
                "  cov: cs.overflow, cbr: cs.borderTopLeftRadius,"
                "  oi: os.backgroundImage.slice(0, 16), op: os.padding,"
                "  nfs: ns.fontSize, nlh: ns.lineHeight, nfw: ns.fontWeight, nc: ns.color,"
                "  ifs: is.fontSize, ilh: is.lineHeight, iff: is.fontFamily, ic: is.color,"
                "  bt: badge ? badge.textContent : null,"
                "  bl: bs ? bs.left : null, btop: bs ? bs.top : null,"
                "  bbg: bs ? bs.backgroundColor : null, bbr: bs ? bs.borderTopLeftRadius : null,"
                "  bfs: bs ? bs.fontSize : null, bfw: bs ? bs.fontWeight : null, bc: bs ? bs.color : null }; })()")
            check("T6a 文件卡 = 壳绝对定位 + 芯 g2 #1C1C1C 圆角 8 + 裁剪层 + 渐变罩 padding 24/12/12",
                  isinstance(card6, dict) and card6.get("pos") == "absolute" and card6.get("absL") is True
                  and card6.get("bg") == "rgb(28, 28, 28)" and card6.get("br") == "8px"
                  and card6.get("cov") == "hidden" and card6.get("cbr") == "8px"
                  and "linear-gradient" in (card6.get("oi") or "")
                  and card6.get("op") == "24px 12px 12px",
                  json.dumps(card6))
            check("T6b 罩名称 13/18 Bold 一级 + 简介 mono 12/16 二级",
                  isinstance(card6, dict) and card6.get("nfs") == "13px" and card6.get("nlh") == "18px"
                  and card6.get("nfw") == "700" and card6.get("nc") == "rgb(242, 242, 242)"
                  and card6.get("ifs") == "12px" and card6.get("ilh") == "16px"
                  and "Fira Mono" in (card6.get("iff") or "") and card6.get("ic") == "rgb(191, 191, 191)",
                  json.dumps(card6))
            check("T6c fla 卡徽标「FLA」左上 12/12 统一铬（半透明-2 底 + 圆角 4 + mono 12 Bold 二级）",
                  isinstance(card6, dict) and card6.get("bt") == "FLA"
                  and card6.get("bl") == "12px" and card6.get("btop") == "12px"
                  and card6.get("bbg") == "rgba(41, 41, 41, 0.5)" and card6.get("bbr") == "4px"
                  and card6.get("bfs") == "12px" and card6.get("bfw") == "700"
                  and card6.get("bc") == "rgb(191, 191, 191)",
                  json.dumps(card6))
            png_badge = await ev(
                "!!document.querySelector('.content-area .asset-card[data-key=\"a1|f1\"] .fc-badge')")
            check("T6d png 静态图不出格式徽标（formatBadgeColor null 口径）", png_badge is False, str(png_badge))

            # ================= 阶段 B：进 folder:f1 —— 三区容器 =================
            await ev("document.querySelector('.block-tree .tree-item[data-folder-id=\"f1\"]')"
                     ".dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await wait_for("!!document.querySelector('.folder-block .folder-card')", 8)
            await asyncio.sleep(1.0)

            st7 = await ev(
                "(() => { const el = document.querySelector('.folder-block .st.block');"
                " if (!el) return null; const tt = el.querySelector('.tt'); const s = getComputedStyle(tt);"
                " const chev = el.querySelector('.chev');"
                " const tail = el.querySelector('.st-tail .recursive-cb .label');"
                " const ts = tail ? getComputedStyle(tail) : null;"
                " return { h: el.offsetHeight, pl: getComputedStyle(el).paddingLeft,"
                "  txt: el.querySelector('.txt')?.textContent, num: el.querySelector('.num')?.textContent,"
                "  fs: s.fontSize, lh: s.lineHeight, fw: s.fontWeight, c: s.color,"
                "  acts: !!el.querySelector('.acts'), tail: tail ? tail.textContent : null,"
                "  tfs: ts ? ts.fontSize : null, tlh: ts ? ts.lineHeight : null,"
                "  cw: chev ? chev.offsetWidth : null, cv: chev ? getComputedStyle(chev).visibility : null }; })()")
            check("T7 文件夹区块标题 block 16 高 padding-left 8「文件夹/1」12/16 Bold 四级",
                  isinstance(st7, dict) and st7.get("h") == 16 and st7.get("pl") == "8px"
                  and st7.get("txt") == "文件夹" and st7.get("num") == "1"
                  and st7.get("fs") == "12px" and st7.get("lh") == "16px"
                  and st7.get("fw") == "700" and st7.get("c") == "rgb(77, 77, 77)",
                  json.dumps(st7, ensure_ascii=False))
            check("T7b 稿无动作钮（无 .acts）+ 无 tail（递归显示撤出归查看方式菜单，2026-08-30 用户定）+ 展开钮 14 常态隐",
                  isinstance(st7, dict) and st7.get("acts") is False
                  and st7.get("tail") is None
                  and st7.get("cw") == 14 and st7.get("cv") == "hidden",
                  json.dumps(st7, ensure_ascii=False))

            # T7c hover 标题行（真实鼠标移动）→ 展开钮现 + 抓手指针
            rect = await ev(
                "(() => { const r = document.querySelector('.folder-block .st.block').getBoundingClientRect();"
                " return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()")
            if isinstance(rect, dict):
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseMoved", "x": rect["x"], "y": rect["y"]}, session=sess)
                await asyncio.sleep(0.5)
            hv = await ev(
                "(() => { const el = document.querySelector('.folder-block .st.block');"
                " return { cv: getComputedStyle(el.querySelector('.chev')).visibility,"
                "  cur: getComputedStyle(el).cursor }; })()")
            check("T7c hover 标题行 → 展开钮现 + 抓手指针",
                  isinstance(hv, dict) and hv.get("cv") == "visible" and hv.get("cur") == "grab",
                  json.dumps(hv))
            # 鼠标挪走（后续截图不带 hover 态）
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": 30, "y": 850}, session=sess)
            await asyncio.sleep(0.3)

            fc8 = await ev(
                "(() => { const flow = document.querySelector('.folder-block .flow');"
                " const fs = getComputedStyle(flow);"
                " const card = flow.querySelector('.folder-card'); const cs = getComputedStyle(card);"
                " const nm = card.querySelector('.fd-name'); const ns = getComputedStyle(nm);"
                " const icon = card.querySelector('.tt-icon');"
                " const lines = [...card.querySelectorAll('.fd-line')].map((l) => {"
                "  const s = getComputedStyle(l);"
                "  return { segs: [...l.children].map((c) => c.textContent), ff: s.fontFamily,"
                "   fs: s.fontSize, lh: s.lineHeight, fw: s.fontWeight, c: s.color }; });"
                " return { ar: fs.gridAutoRows, cg: fs.columnGap, rg: fs.rowGap,"
                "  w: card.offsetWidth, h: card.offsetHeight, bg: cs.backgroundColor,"
                "  br: cs.borderTopLeftRadius, p: cs.padding, nm: nm.textContent,"
                "  nfs: ns.fontSize, nlh: ns.lineHeight, nfw: ns.fontWeight, nc: ns.color,"
                "  iw: icon ? icon.offsetWidth : null, tint: icon ? (icon.getAttribute('data-tint') ?? 'none') : 'absent',"
                "  lines }; })()")
            check("T8a 文件夹卡 grid auto-rows 122 + gap 12 + 卡 122 高 宽≥220 + g3 底圆角 8 padding 12",
                  isinstance(fc8, dict) and fc8.get("ar") == "122px"
                  and fc8.get("cg") == "12px" and fc8.get("rg") == "12px"
                  and fc8.get("h") == 122 and fc8.get("w", 0) >= 220
                  and fc8.get("bg") == "rgb(36, 36, 36)" and fc8.get("br") == "8px"
                  and fc8.get("p") == "12px",
                  json.dumps(fc8))
            check("T8b 名称 18/26 Bold 一级 + folder-large 24 双色调无 tint",
                  isinstance(fc8, dict) and fc8.get("nm") == "线稿"
                  and fc8.get("nfs") == "18px" and fc8.get("nlh") == "26px"
                  and fc8.get("nfw") == "700" and fc8.get("nc") == "rgb(242, 242, 242)"
                  and fc8.get("iw") == 24 and fc8.get("tint") == "none",
                  json.dumps(fc8, ensure_ascii=False))
            lines8 = fc8.get("lines") if isinstance(fc8, dict) else None
            check("T8c 简介两行 mono 12/16 三级 + 文本「2 个文件/500.0 MB」「1 个档案/1 个子文件夹」",
                  isinstance(lines8, list) and len(lines8) == 2
                  and lines8[0].get("segs") == ["2 个文件", "/", "500.0 MB"]
                  and lines8[1].get("segs") == ["1 个档案", "/", "1 个子文件夹"]
                  and all("Fira Mono" in (l.get("ff") or "") and l.get("fs") == "12px"
                          and l.get("lh") == "16px" and l.get("fw") == "500"
                          and l.get("c") == "rgb(128, 128, 128)" for l in lines8),
                  json.dumps(lines8, ensure_ascii=False))

            ar9 = await ev(
                "(() => { const sec = document.querySelector('.archive-block');"
                " if (!sec) return null; const st = sec.querySelector('.st.block');"
                " const flow = sec.querySelector('.flow'); const fs = getComputedStyle(flow);"
                " const card = flow.querySelector('.ar-card'); const cs = getComputedStyle(card);"
                " const nm = card.querySelector('.ar-name'); const ns = getComputedStyle(nm);"
                " const rows = [...card.querySelectorAll('.ar-row')].map((l) => {"
                "  const s = getComputedStyle(l);"
                "  return { segs: [...l.children].map((c) => c.textContent), ff: s.fontFamily,"
                "   fs: s.fontSize, lh: s.lineHeight, c: s.color }; });"
                " const badge = card.querySelector('.ar-badge'); const bs = badge ? getComputedStyle(badge) : null;"
                " return { txt: st.querySelector('.txt')?.textContent, num: st.querySelector('.num')?.textContent,"
                "  acts: !!st.querySelector('.acts'), cg: fs.columnGap, rg: fs.rowGap,"
                "  w: card.offsetWidth, h: card.offsetHeight, bg: cs.backgroundColor, br: cs.borderTopLeftRadius,"
                "  nm: nm.textContent, nfs: ns.fontSize, nlh: ns.lineHeight, nfw: ns.fontWeight, nc: ns.color,"
                "  rows, bt: badge ? badge.textContent : null,"
                "  bl: bs ? bs.left : null, btop: bs ? bs.top : null, bbg: bs ? bs.backgroundColor : null,"
                "  bbr: bs ? bs.borderTopLeftRadius : null }; })()")
            check("T9a 档案区块标题「档案/1」无动作组 + flow gap 12",
                  isinstance(ar9, dict) and ar9.get("txt") == "档案" and ar9.get("num") == "1"
                  and ar9.get("acts") is False
                  and ar9.get("cg") == "12px" and ar9.get("rg") == "12px",
                  json.dumps(ar9, ensure_ascii=False))
            check("T9b 档案卡正方形（宽===高 ≥250）+ g2 底圆角 8",
                  isinstance(ar9, dict) and ar9.get("w") == ar9.get("h") and ar9.get("w", 0) >= 250
                  and ar9.get("bg") == "rgb(28, 28, 28)" and ar9.get("br") == "8px",
                  json.dumps(ar9))
            rows9 = ar9.get("rows") if isinstance(ar9, dict) else None
            check("T9c 罩名称 18/26 Bold 一级 + 属性行「立绘|彩色」/ 数量行「2 个文件/1.25 GB」mono 12/16 二级",
                  isinstance(ar9, dict) and ar9.get("nm") == "主角"
                  and ar9.get("nfs") == "18px" and ar9.get("nlh") == "26px"
                  and ar9.get("nfw") == "700" and ar9.get("nc") == "rgb(242, 242, 242)"
                  and isinstance(rows9, list) and len(rows9) == 2
                  and rows9[0].get("segs") == ["立绘", "|", "彩色"]
                  and rows9[1].get("segs") == ["2 个文件", "/", "1.25 GB"]
                  and all("Fira Mono" in (r.get("ff") or "") and r.get("fs") == "12px"
                          and r.get("lh") == "16px" and r.get("c") == "rgb(191, 191, 191)" for r in rows9),
                  json.dumps(ar9, ensure_ascii=False))
            check("T9d 徽标「主角」左上 12/12 统一铬（半透明-2 底 + 圆角 4）",
                  isinstance(ar9, dict) and ar9.get("bt") == "主角"
                  and ar9.get("bl") == "12px" and ar9.get("btop") == "12px"
                  and ar9.get("bbg") == "rgba(41, 41, 41, 0.5)" and ar9.get("bbr") == "4px",
                  json.dumps(ar9, ensure_ascii=False))

            ch10 = await ev(
                "(() => { const el = document.querySelector('.grid-header > .st.block');"
                " if (!el) return null;"
                " return { txt: el.querySelector('.txt')?.textContent, num: el.querySelector('.num')?.textContent,"
                "  h: el.offsetHeight, acts: !!el.querySelector('.acts') }; })()")
            check("T10 内容标题 block「内容/5」（递归显示开 = 子树 5 条）+ 无动作组",
                  isinstance(ch10, dict) and ch10.get("txt") == "内容" and ch10.get("num") == "5"
                  and ch10.get("h") == 16 and ch10.get("acts") is False,
                  json.dumps(ch10, ensure_ascii=False))

            t4c = await ev(
                "[...document.querySelectorAll('" + TB + " .btn-group .ic-btn')].map((b) => b.disabled)")
            check("T4c 文件夹视口：添加链接钮解禁 + 分组管理递归时禁用 + 插件恒禁用",
                  isinstance(t4c, list) and t4c[0] is False and t4c[1] is True and t4c[2] is True,
                  json.dumps(t4c))

            # 截图：文件夹视口三区全景（整窗 + 内容容器 2x 局部）
            await shot(f"{SHOT_DIR}/r21-content-container-full.png")
            await shot(f"{SHOT_DIR}/r21-content-container-grid.png", ".content-area", 2)

            # T11 后退/前进接线
            dis11 = await ev("[...document.querySelectorAll('" + TB + " .nav-pair .ic-btn')].map((b) => b.disabled)")
            check("T11a 进 f1 后：后退解禁 + 前进仍禁用",
                  isinstance(dis11, list) and dis11[0] is False and dis11[1] is True, json.dumps(dis11))
            await ev("[...document.querySelectorAll('" + TB + " .nav-pair .ic-btn')][0].click()")
            await wait_for("!document.querySelector('.folder-block') && document.querySelectorAll('.content-area .asset-card').length >= 5", 8)
            await asyncio.sleep(0.6)
            back_state = await ev(
                "({ strip: !!document.querySelector('.folder-block'),"
                "  dis: [...document.querySelectorAll('" + TB + " .nav-pair .ic-btn')].map((b) => b.disabled) })")
            check("T11b 点后退 → 回 quick:all（文件夹区消失）+ 前进解禁",
                  isinstance(back_state, dict) and back_state.get("strip") is False
                  and back_state.get("dis", [])[1] is False,
                  json.dumps(back_state))
            await ev("[...document.querySelectorAll('" + TB + " .nav-pair .ic-btn')][1].click()")
            fwd_ok = await wait_for("!!document.querySelector('.folder-block .folder-card')", 8)
            await asyncio.sleep(0.6)
            check("T11c 点前进 → 回 f1（文件夹区重现）", fwd_ok is True, str(fwd_ok))

            # ================= 阶段 C：关递归显示 → 分组标题 =================
            # 递归开关已撤区块标题（2026-08-30 用户定：查看方式菜单里有就行）——走菜单路径切换
            await ev("[...document.querySelectorAll('" + TB + " .btn-group .ic-btn')][3].click()")
            await wait_for("!!document.querySelector('.app-menu')", 5)
            await ev("[...document.querySelectorAll('.app-menu .row')]"
                     ".find((r) => r.querySelector('.label')?.textContent === '显示子文件夹内容')"
                     "?.querySelector('.app-switch')?.click()")
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))")
            await wait_for("!document.querySelector('.app-menu')", 5)
            await wait_for(
                "document.querySelector('.grid-header > .st.block .num')?.textContent === '3'"
                " && document.querySelectorAll('.group-header .st.group').length === 2", 8)
            await asyncio.sleep(0.6)
            grp = await ev(
                "[...document.querySelectorAll('.group-header .st.group')].map((el) => {"
                " const tt = el.querySelector('.tt'); const s = getComputedStyle(tt);"
                " return { txt: el.querySelector('.txt')?.textContent, num: el.querySelector('.num')?.textContent,"
                "  h: el.offsetHeight, fs: s.fontSize, lh: s.lineHeight, fw: s.fontWeight, c: s.color }; })")
            check("T12 关递归 → 内容/3 + 分组标题两枚 group 26 高 18/26 Bold 二级（未分组/2、图片/1）",
                  isinstance(grp, list) and len(grp) == 2
                  and [g.get("txt") for g in grp] == ["未分组", "图片"]
                  and [g.get("num") for g in grp] == ["2", "1"]
                  and all(g.get("h") == 26 and g.get("fs") == "18px" and g.get("lh") == "26px"
                          and g.get("fw") == "700" and g.get("c") == "rgb(191, 191, 191)" for g in grp),
                  json.dumps(grp, ensure_ascii=False))

            # ================= 阶段 D：开递归 + 列表模式 =================
            await ev("[...document.querySelectorAll('" + TB + " .btn-group .ic-btn')][3].click()")
            await wait_for("!!document.querySelector('.app-menu')", 5)
            await ev("[...document.querySelectorAll('.app-menu .row')]"
                     ".find((r) => r.querySelector('.label')?.textContent === '显示子文件夹内容')"
                     "?.querySelector('.app-switch')?.click()")
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))")
            await wait_for("!document.querySelector('.app-menu')", 5)
            await wait_for(
                "document.querySelector('.grid-header > .st.block .num')?.textContent === '5'"
                " && document.querySelectorAll('.group-header').length === 0", 8)
            await asyncio.sleep(0.6)
            # 查看方式菜单 → 布局切「列表」
            await ev("[...document.querySelectorAll('" + TB + " .btn-group .ic-btn')][3].click()")
            await wait_for("!!document.querySelector('.app-menu .app-seg')", 5)
            await ev("[...document.querySelectorAll('.app-menu .app-seg .item')]"
                     ".find((b) => b.textContent.trim() === '列表').click()")
            await wait_for("!!document.querySelector('.list-header .lh-root')", 8)
            await asyncio.sleep(0.8)
            # 关菜单（Esc；capture 早序打标，不连带清选择）
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))")
            await wait_for("!document.querySelector('.app-menu')", 5)

            hd = await ev(
                "(() => { const root = document.querySelector('.list-header .lh-root');"
                " const s = getComputedStyle(root);"
                " const cols = [...root.querySelectorAll('.lh-col')].map((c) => {"
                "  const lb = c.querySelector('.lh-label'); const ls = getComputedStyle(lb);"
                "  return { t: lb.textContent, w: c.offsetWidth, grow: c.className.includes('grow'),"
                "   fs: ls.fontSize, lh: ls.lineHeight, fw: ls.fontWeight, c: ls.color }; });"
                " return { h: root.offsetHeight, bg: s.backgroundColor, br: s.borderTopLeftRadius,"
                "  p: s.padding, gap: s.columnGap, cols }; })()")
            cols13 = hd.get("cols") if isinstance(hd, dict) else []
            check("T13a 表头 34 高 g3 底圆角 8 padding 8/12 gap 12",
                  isinstance(hd, dict) and hd.get("h") == 34 and hd.get("bg") == "rgb(36, 36, 36)"
                  and hd.get("br") == "8px" and hd.get("p") == "8px 12px" and hd.get("gap") == "12px",
                  json.dumps(hd))
            check("T13b 列构 = 名称 flex 1 + 尺寸/扩展名/文件大小/修改日期 各 123 + 标签 13/18 Bold 三级",
                  isinstance(cols13, list) and len(cols13) == 5
                  and [c.get("t") for c in cols13] == ["名称", "尺寸", "扩展名", "文件大小", "修改日期"]
                  and cols13[0].get("grow") is True
                  and all(c.get("w") == 123 for c in cols13[1:])
                  and all(c.get("fs") == "13px" and c.get("lh") == "18px"
                          and c.get("fw") == "700" and c.get("c") == "rgb(128, 128, 128)" for c in cols13),
                  json.dumps(cols13, ensure_ascii=False))

            row14 = await ev(
                "(() => { const shell = document.querySelector('.list-row-shell');"
                " const card = shell.querySelector('.lr-card'); const s = getComputedStyle(card);"
                " const th = card.querySelector('.lr-thumb'); const ts = getComputedStyle(th);"
                " const nm = card.querySelector('.lr-name'); const ns = getComputedStyle(nm);"
                " const col = card.querySelector('.lr-col'); const cs = getComputedStyle(col);"
                " const sz = card.querySelector('.lr-size');"
                " const mt = [...card.querySelectorAll('.lr-mtime span')].map((x) => x.textContent);"
                " return { sh: parseFloat(shell.style.height), h: card.offsetHeight, p: s.padding,"
                "  gap: s.columnGap, bg: s.backgroundColor, br: s.borderTopLeftRadius,"
                "  tw: th.offsetWidth, th: th.offsetHeight, tbg: ts.backgroundColor,"
                "  tbc: ts.borderTopColor, tbr: ts.borderTopLeftRadius,"
                "  nfs: ns.fontSize, nlh: ns.lineHeight, nfw: ns.fontWeight, nc: ns.color,"
                "  cw: col.offsetWidth, cfs: cs.fontSize, clh: cs.lineHeight, cff: cs.fontFamily, cc: cs.color,"
                "  szt: sz.textContent, mt }; })()")
            check("T14a 列表行 57 高 padding 6/12/6/6 gap 12 + g2 底圆角 8",
                  isinstance(row14, dict) and row14.get("sh") == 57 and row14.get("h") == 57
                  and row14.get("p") == "6px 12px 6px 6px" and row14.get("gap") == "12px"
                  and row14.get("bg") == "rgb(28, 28, 28)" and row14.get("br") == "8px",
                  json.dumps(row14))
            check("T14b 缩略图 80×45 g3 底 g5 描边圆角 6 + 数据列 123 mono 13/18 三级 + 名称 13/18 Bold 一级",
                  isinstance(row14, dict) and row14.get("tw") == 80 and row14.get("th") == 45
                  and row14.get("tbg") == "rgb(36, 36, 36)" and row14.get("tbc") == "rgb(46, 46, 46)"
                  and row14.get("tbr") == "6px"
                  and row14.get("cw") == 123 and row14.get("cfs") == "13px" and row14.get("clh") == "18px"
                  and "Fira Mono" in (row14.get("cff") or "") and row14.get("cc") == "rgb(128, 128, 128)"
                  and row14.get("nfs") == "13px" and row14.get("nlh") == "18px"
                  and row14.get("nfw") == "700" and row14.get("nc") == "rgb(242, 242, 242)",
                  json.dumps(row14))
            check("T14c 文件大小 / 修改日期接真值（首行 2.0 MB = 存储序首条 a1 跑步循环.png 桩值）",
                  isinstance(row14, dict) and row14.get("szt") == "2.0 MB"
                  and isinstance(row14.get("mt"), list) and row14["mt"] == ["2026/08/24", "16:07"],
                  json.dumps(row14, ensure_ascii=False))

            # T14d 列表行真实点击 → 选中（2026-08-30 实机"列表模式无法选中"回归：壳 list-row 标记类
            # 补回后 gridPointerDown 命中 ".asset-card, .list-row" 走拖拽分发，不再落空白框选吞 click；
            # 必须 Input.dispatchMouseEvent 真实点击——JS .click() 不过 pointerdown 分发链测不到本 bug）
            rc = await ev(
                "(() => { const rows = [...document.querySelectorAll('.list-row-shell')]"
                ".sort((a, b) => parseFloat(a.style.top) - parseFloat(b.style.top));"
                " const r = rows[0].getBoundingClientRect(); return { x: r.x + 200, y: r.y + r.height / 2 }; })()")
            if isinstance(rc, dict):
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mousePressed", "x": rc["x"], "y": rc["y"],
                            "button": "left", "buttons": 1, "clickCount": 1}, session=sess)
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseReleased", "x": rc["x"], "y": rc["y"],
                            "button": "left", "buttons": 0, "clickCount": 1}, session=sess)
                await asyncio.sleep(0.4)
            sel14 = await ev(
                "(() => { const rows = [...document.querySelectorAll('.list-row-shell')]"
                ".sort((a, b) => parseFloat(a.style.top) - parseFloat(b.style.top));"
                " return { sel: !!rows[0].querySelector('.lr-card.selected'),"
                "  mq: !!document.querySelector('.grid-viewport .marquee') }; })()")
            check("T14d 列表行真实点击 → 选中（list-row 标记类命中分发，不落空白框选吞 click）",
                  isinstance(sel14, dict) and sel14.get("sel") is True and sel14.get("mq") is False,
                  json.dumps(sel14))

            # T14e 格式图标行缩略图 icon 档 50% 居中（a2 fla 桩 thumbKind=formatIcon；
            # 假 thumbPath 加载失败不影响——50% 是 CSS 尺寸与图加载无关）
            ico = await ev(
                "(() => { const rows = [...document.querySelectorAll('.list-row-shell')]"
                ".sort((a, b) => parseFloat(a.style.top) - parseFloat(b.style.top));"
                " const img = rows[1]?.querySelector('.lr-thumb-img');"
                " return img ? { cls: img.className, w: img.offsetWidth, h: img.offsetHeight } : null; })()")
            check("T14e 格式图标行缩略图 icon 档 50% 居中不撑满（2026-08-30 用户实机）",
                  isinstance(ico, dict) and "icon" in (ico.get("cls") or "")
                  and ico.get("w") == 39 and abs((ico.get("h") or 0) - 21.5) < 1,
                  json.dumps(ico))

            async def read_sizes():
                return await ev(
                    "[...document.querySelectorAll('.list-row-shell')].map((el) => ({"
                    " top: parseFloat(el.style.top), size: el.querySelector('.lr-size')?.textContent }))"
                    ".sort((a, b) => a.top - b.top).map((r) => r.size)")

            # T15 排序：点「文件大小」→ active + 升序；再点 → desc 降序；点「尺寸」→ 无反应
            await ev("[...document.querySelectorAll('.list-header .lh-col')]"
                     ".find((c) => c.querySelector('.lh-label')?.textContent === '文件大小').click()")
            await wait_for(
                "[...document.querySelectorAll('.list-row-shell')].length === 5 && "
                "[...document.querySelectorAll('.list-row-shell')].map((el) => el.querySelector('.lr-size')?.textContent).includes('50.0 KB')", 5)
            await asyncio.sleep(0.6)
            sort1 = await ev(
                "({ act: document.querySelector('.list-header .lh-col.active .lh-label')?.textContent ?? null,"
                "  desc: !!document.querySelector('.list-header .lh-col.active .lh-arrow.desc') })")
            sizes1 = await read_sizes()
            check("T15a 点文件大小 → 列 active（sortBy=size）+ 行序升序（50.0 KB → 5.0 MB）",
                  isinstance(sort1, dict) and sort1.get("act") == "文件大小" and sort1.get("desc") is False
                  and sizes1 == ["50.0 KB", "100.0 KB", "1.5 MB", "2.0 MB", "5.0 MB"],
                  json.dumps({"sort": sort1, "sizes": sizes1}, ensure_ascii=False))
            await ev("[...document.querySelectorAll('.list-header .lh-col')]"
                     ".find((c) => c.querySelector('.lh-label')?.textContent === '文件大小').click()")
            await asyncio.sleep(0.6)
            sort2 = await ev(
                "({ act: document.querySelector('.list-header .lh-col.active .lh-label')?.textContent ?? null,"
                "  desc: !!document.querySelector('.list-header .lh-col.active .lh-arrow.desc') })")
            sizes2 = await read_sizes()
            check("T15b 再点 → 箭头 desc（sortAsc 翻转）+ 行序降序（首行 5.0 MB）",
                  isinstance(sort2, dict) and sort2.get("act") == "文件大小" and sort2.get("desc") is True
                  and sizes2 == ["5.0 MB", "2.0 MB", "1.5 MB", "100.0 KB", "50.0 KB"],
                  json.dumps({"sort": sort2, "sizes": sizes2}, ensure_ascii=False))
            await ev("[...document.querySelectorAll('.list-header .lh-col')]"
                     ".find((c) => c.querySelector('.lh-label')?.textContent === '尺寸').click()")
            await asyncio.sleep(0.4)
            sort3 = await ev(
                "document.querySelector('.list-header .lh-col.active .lh-label')?.textContent ?? null")
            sizes3 = await read_sizes()
            check("T15c 尺寸列点击无反应（active 仍在文件大小 + 行序不变）",
                  sort3 == "文件大小" and sizes3 == sizes2,
                  json.dumps({"act": sort3, "sizes": sizes3}, ensure_ascii=False))

            align = await ev(
                "(() => { const vp = document.querySelector('.content-area .grid-viewport').getBoundingClientRect();"
                " const hd = document.querySelector('.list-header .lh-root').getBoundingClientRect();"
                " const rows = [...document.querySelectorAll('.list-row-shell')]"
                "   .map((el) => el.getBoundingClientRect().right);"
                " return { hd: vp.right - hd.right, row: vp.right - Math.max(...rows) }; })()")
            check("T15d 表头右缘 = 行右缘 = 视口右缘 -12（±1，钉顶列头与行严格对齐）",
                  isinstance(align, dict) and abs(align.get("hd", 0) - 12) <= 1
                  and abs(align.get("row", 0) - 12) <= 1,
                  json.dumps(align))

            # 截图：列表模式（文件大小降序 + 箭头 desc 态）
            await shot(f"{SHOT_DIR}/r21-content-container-list.png", ".content-area", 2)

            # ================= 阶段 E：搜索态计数 =================
            await ev(
                "(() => { const inp = document.querySelector('.content-area .search-bar .draft-input');"
                " inp.value = '跑步'; inp.dispatchEvent(new Event('input', { bubbles: true })); return true; })()")
            await wait_for("!!document.querySelector('" + TB + " .result-group')", 8)
            # 计数 = hits 装配后渲染（防抖 150ms + invoke）——轮询等 count 文本到位，勿固定 sleep（§12 纪律）
            await wait_for(
                "document.querySelector('" + TB + " .result-count')?.textContent === '3'", 8)
            res16 = await ev(
                "(() => { const g = document.querySelector('" + TB + " .result-group');"
                " const lb = g.querySelector('.result-label'); const sep = g.querySelector('.result-sep');"
                " const ct = g.querySelector('.result-count');"
                " const ls = getComputedStyle(lb); const ss = getComputedStyle(sep); const cs = getComputedStyle(ct);"
                " return { lt: lb.textContent, st: sep.textContent, ct: ct.textContent,"
                "  lfs: ls.fontSize, llh: ls.lineHeight, lc: ls.color, sc: ss.color,"
                "  cff: cs.fontFamily, cfs: cs.fontSize, clh: cs.lineHeight, cc: cs.color }; })()")
            check("T16 搜索「跑步」→ 工具栏「搜索结果 / 3」（2 文件 + 1 档案命中）",
                  isinstance(res16, dict) and res16.get("lt") == "搜索结果"
                  and res16.get("st") == "/" and res16.get("ct") == "3",
                  json.dumps(res16, ensure_ascii=False))
            check("T16b label/sep 正文 13/18 二级 + count mono 13/18 三级",
                  isinstance(res16, dict) and res16.get("lfs") == "13px" and res16.get("llh") == "18px"
                  and res16.get("lc") == "rgb(191, 191, 191)" and res16.get("sc") == "rgb(191, 191, 191)"
                  and "Fira Mono" in (res16.get("cff") or "")
                  and res16.get("cfs") == "13px" and res16.get("clh") == "18px"
                  and res16.get("cc") == "rgb(128, 128, 128)",
                  json.dumps(res16))

            await shot(f"{SHOT_DIR}/r21-content-container-search.png", ".content-area", 2)

            await ev(
                "(() => { const inp = document.querySelector('.content-area .search-bar .draft-input');"
                " inp.value = ''; inp.dispatchEvent(new Event('input', { bubbles: true })); return true; })()")
            gone = await wait_for("!document.querySelector('" + TB + " .result-group')", 5)
            check("T16c 清空搜索 → 计数消失", gone is True, str(gone))

            # ================= 阶段 F：空文件夹空态 =================
            await ev("document.querySelector('.block-tree .tree-item[data-folder-id=\"f-empty\"]')"
                     ".dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await wait_for("!!document.querySelector('.ef-root')", 8)
            await asyncio.sleep(0.8)
            ef = await ev(
                "(() => { const root = document.querySelector('.ef-root');"
                " const t = root.querySelector('.ef-title'); const h = root.querySelector('.ef-hint');"
                " const ts = getComputedStyle(t); const hs = getComputedStyle(h);"
                " const acts = root.querySelector('.ef-actions'); const as = getComputedStyle(acts);"
                " const rr = root.getBoundingClientRect();"
                " const bd = root.querySelector('.ef-body').getBoundingClientRect();"
                " const rc = [...acts.children].map((c) => c.getBoundingClientRect());"
                " const btns = [...acts.querySelectorAll('.act-btn')].map((b) => {"
                "  const s = getComputedStyle(b);"
                "  return { t: b.querySelector('.lbl')?.textContent, h: b.offsetHeight, w: b.offsetWidth,"
                "   bg: s.backgroundColor, br: s.borderTopLeftRadius, cls: b.className }; });"
                " return { tt: t.textContent, ht: h.textContent,"
                "  tfs: ts.fontSize, tlh: ts.lineHeight, tfw: ts.fontWeight, tc: ts.color,"
                "  hfs: hs.fontSize, hlh: hs.lineHeight, hc: hs.color,"
                "  aw: acts.offsetWidth, agap: as.columnGap, btns,"
                "  cx: (rc[0].left + rc[1].right) / 2 - (rr.left + rr.right) / 2,"
                "  cy: (bd.top + bd.bottom) / 2 - (rr.top + rr.bottom) / 2 }; })()")
            check("T17a 空态标题「暂无文件」18/26 Bold 二级 + 提示「将文件拖拽至此处导入」13/18 三级",
                  isinstance(ef, dict) and ef.get("tt") == "暂无文件"
                  and ef.get("tfs") == "18px" and ef.get("tlh") == "26px"
                  and ef.get("tfw") == "700" and ef.get("tc") == "rgb(191, 191, 191)"
                  and ef.get("ht") == "将文件拖拽至此处导入"
                  and ef.get("hfs") == "13px" and ef.get("hlh") == "18px"
                  and ef.get("hc") == "rgb(128, 128, 128)",
                  json.dumps(ef, ensure_ascii=False))
            btns17 = ef.get("btns") if isinstance(ef, dict) else None
            check("T17b 双钮（导入文件/导入文件夹）= ActionButton solid md 34 高 g4 #292929 圆角 8 + 组 282 宽 gap 8 + 钮实宽 137 均分",
                  isinstance(btns17, list) and len(btns17) == 2
                  and [b.get("t") for b in btns17] == ["导入文件", "导入文件夹"]
                  and all(b.get("h") == 34 and b.get("w") == 137 and b.get("bg") == "rgb(41, 41, 41)"
                          and b.get("br") == "8px" and "solid" in b.get("cls", "")
                          and "md" in b.get("cls", "") for b in btns17)
                  and isinstance(ef, dict) and ef.get("aw") == 282 and ef.get("agap") == "8px",
                  json.dumps(ef, ensure_ascii=False))
            # T17c 居中实证（2026-08-30 用户实机"空态没居中"回归）：按钮组视觉中心 = root 水平中心
            #（防 ActionButton width:100% + flex:none 溢出偏右——旧断言只看声明宽 aw=282 漏检溢出）；
            # 垂直 = ef-body 盒（含 padding-bottom 24，稿组 282×339.4）中心 = root 中心
            check("T17c 空态组居中（按钮组中心 = root 水平中心 ±1 + body 中心 = root 垂直中心 ±1）",
                  isinstance(ef, dict) and isinstance(ef.get("cx"), (int, float))
                  and abs(ef["cx"]) <= 1 and abs(ef["cy"]) <= 1,
                  json.dumps({"cx": ef and ef.get("cx"), "cy": ef and ef.get("cy")}))

            await shot(f"{SHOT_DIR}/r21-content-container-empty.png", ".content-area", 2)
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
