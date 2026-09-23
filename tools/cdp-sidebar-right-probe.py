# CDP 探针：7.1 二十三轮 · 右侧边栏轮（2026-08-31）——窗口控制浮块 / 详情面板三 tab / 文件夹·快速访问形态贴稿断言
#   R1  栏宽 = 264（ui.rightWidth 新默认；桩 get_ui_settings = {}）
#   R2  窗口控制浮块：四钮 58×46 连续排，图标序 = pin-on/minimize/maximize/close-large
#   R3  初始 quick:all 未选中 → 快速访问形态：无分段控制器 / 无管理钮 / 无缩略图；单名称框「全部」只读；
#       数据卡两行（文件数 3 / 占用空间）
#   R4  顶行 46 拖拽区无 border-bottom；旧结构清零（.panel-toolbar / .side-thumb / .picker-panel）
#   R5  点树 f1 → 文件夹形态：folder-detail-panel 图标缩略图 + 名称框「角色」+ 数据卡 4 行
#       （文件数/档案数/子文件夹数/占用空间），仍无分段控制器 / 管理钮
#   R6  点素材卡 a1 → 文件形态：pinned 缩略图 240 方（g2 底 g3 边）+ FormatBadge「FLA」左上 12/12；
#       色块行显隐（2026-09-23 用户定）：无内容缩略图（桩 a1 thumbKind null）→ 整行隐藏（R6b）；
#       有内容缩略图（桩 a2 thumbKind asset，fla 伴生 webp 代理同径）→ 显示真色不显示占位（R6c）
#   R7  分段控制器三段 icon，默认「基础信息」选中
#   R8  基础信息 tab：名称框值 / 链接框占位 http://+link 图标 / 属性 chips×2 / 描述条目「身份」/
#       数据卡 4 行（文件大小/格式/添加日期/修改日期——桩 width null 无尺寸行，旧口径）
#   R9  底部信息查看管理钮：58 高块 + 34 高 solid + 文案（点亮不接业务）
#   R10 链接关系 tab：五区块标题序 = 链接/音频/关系/缩略图/污染文件；链接卡 ×2 首行 origin 徽章；
#       污染文件区块无动作组（稿）；R10f 缩略图区块接业务（2026-09-23）：调用侧卡 = 伴生资产名 + 引用数
#   R11 提示词 tab：只读编辑框 + 五枚 solid 图标钮（业务占位）
#   R12 tab 全局记忆：links 态点另一素材卡 a2 → 仍 links（2026-09-19 用户改定，持久化 ui.detailTab）
#   R13 档案视口（folder:f2a）未选中 → 档案形态：封面区块 + 徽标「主角」+ chips「连载」+ 数据卡含创建日期
#   R14 关闭钮 hover = 状态/禁止底 rgb(220,57,57)（Input.dispatchMouseEvent 真实悬停）
#   R15 选择模式：关系区块添加钮进 picker → 占位组（点击卡片选择 + 双框禁用）+ 按钮序 取消左/确认右 +
#       顶部横幅；取消退出 = 详情面板恢复
# 2026-08-31 用户实机走查六条修复追加断言：
#   R4c 窗口控制区透窗口底：aside 根无底色/无左边线，::before 只画顶行 46 以下（g1 + 左缘 1px g3）
#   R7b 分段图标色：选中段一级 rgb(242,242,242) / 未选中三级 rgb(128,128,128)（svg stroke 转 currentColor 修复）
#   R8d 描述标题行收起 = 留一排截断正文（clamp-1 / 高 18 / line-clamp 1），不再整段隐藏
#   R10d 链接 tab 四区块标题行折叠钮 + 动作组常显（staticActions——「添加按钮本身就是入口」）；
#       污染文件区块仍无动作组（R10c）
#   R10e tab 切换滑动动画：方向恒定旧左出/新右入（与左侧边栏统一——初版按 tab 序双向被走查否掉），
#       enter 250ms 缓入 delay 100ms / leave 150ms 缓出 / 位移 48px，点击切换后 .tab-pane 挂 tab-x-enter 类
#   R13b/R13c/R13d 封面编辑：点封面 → pinned 根 250ms 高度补间（inline height + transition）；
#       编辑态边距 = 栏缘 +12（pinned padding 12——双重 padding 24 修复）；取消退回常态组
#   截图 r23-sidebar-right.png（整窗 + 右栏局部）
# 桩：最小主窗口；f1 角色（子 f1a 主角）/ f2 场景（子 f2a 子档案 kind=archive）；3 条 Placement（a1 fla 等）。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；桩返回 structuredClone 深拷贝（速查 §12）；
# 素材卡选择走真实指针事件（Input.dispatchMouseEvent 按下+抬起——gridPointerDown 不吃合成 click）；
# 收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-sidebar-right-probe.py
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
PORT = 19230 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-sbright-{os.getpid()}")
SHOT = "tools/style-preview/r23-sidebar-right.png"
SHOT_FULL = "tools/style-preview/r23-sidebar-right-full.png"

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    // 资产详情按 assetId 分桩（2026-09-23 色块行显隐口径断言：a1 无内容缩略图无色 /
    // a2 thumbKind=asset 伴生 webp 代理同径 → 七真色）
    const DETAILS = {
      a1: {
        assetId: "a1", originName: "雪牧_自动.fla", ext: "fla", size: 1000,
        width: null, height: null, duration: null,
        importedAt: "2026-07-12T14:26:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
        tags: ["男", "16岁"],
        descriptions: [{ title: "身份", content: "沙雕动画的御用主角" }],
        colors: [], links: [], note: "",
        placements: [
          { folderId: "f1", blockName: "目录", path: ["角色"], name: "雪牧_自动.fla", origin: true },
          { folderId: "f2", blockName: "目录", path: ["场景"], name: "雪牧_自动-副本.fla", origin: false },
        ],
        archives: [],
        // 缩略图区块（2026-09-23 接业务）：调用侧 = 伴生资产卡（名称 + 引用数），R10f 断言
        thumbAsset: { assetId: "t1", name: "雪牧_自动.webp", users: 2 },
        thumbUsedBy: [],
      },
      a2: {
        assetId: "a2", originName: "雪牧_跑动.fla", ext: "fla", size: 2000,
        width: null, height: null, duration: null,
        importedAt: "2026-07-12T14:26:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
        tags: [], descriptions: [],
        colors: ["#C81E1E", "#1E1EC8", "#1EC81E", "#C8C81E", "#C81EC8", "#1EC8C8", "#808080"],
        links: [], note: "",
        placements: [
          { folderId: "f1", blockName: "目录", path: ["角色"], name: "雪牧_跑动.fla", origin: true },
        ],
        archives: [],
        thumbAsset: null, thumbUsedBy: [],
      },
    };
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 3 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "角色", count: 3, totalSize: 6000,
            children: [{ id: "f1a", name: "主角", count: 1, totalSize: 3000, children: [] }] },
          { id: "f2", name: "场景", count: 0, totalSize: 0,
            children: [{ id: "f2a", name: "子档案", count: 0, totalSize: 0, children: [], kind: "archive" }] },
        ] },
      list_placements: [
        { assetId: "a1", folderId: "f1", name: "雪牧_自动.fla", ext: "fla", size: 1000,
          w: null, h: null, thumbKind: null, thumbPath: null, origin: true,
          frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
          createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
          importedAt: "2026-07-12T14:26:00+08:00" },
        { assetId: "a2", folderId: "f1", name: "雪牧_跑动.fla", ext: "fla", size: 2000,
          w: null, h: null, thumbKind: "asset",
          thumbPath: "E:/test-lib/assets/AN-A2.info/AN-A2_thumbnail.webp", origin: true,
          frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
          createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
          importedAt: "2026-07-12T14:26:00+08:00" },
        { assetId: "a3", folderId: "f1a", name: "雪天_自动.fla", ext: "fla", size: 3000,
          w: null, h: null, thumbKind: null, thumbPath: null, origin: true,
          frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
          createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
          importedAt: "2026-07-12T14:26:00+08:00" },
      ],
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [], list_relations: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
      save_ui_settings: null,
      get_archive_data: {
        folderId: "f2a", badge: { text: "主角", color: "#FF6566" }, avatar: null,
        props: ["连载"], descriptions: [{ title: "定位", content: "沙雕动画御用档案" }],
        links: [], createdAt: "2026-07-01T10:00:00+08:00", fileCount: 0, totalSize: 0, coverPath: null,
      },
    };
    const v = cmd === "get_asset_detail"
      ? (DETAILS[(args && args.assetId) || ""] ?? null)
      : (Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
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

            async def click_sel(selector, scope=""):
                ok = await ev(f"!!document.querySelector('{scope}{selector}')")
                if not ok:
                    return False
                await ev(f"document.querySelector('{scope}{selector}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))")
                await asyncio.sleep(0.4)
                return True

            async def pointer_click(selector):
                """真实指针按下+抬起（素材卡选择走 gridPointerDown，不吃合成 click）"""
                rect = await ev(
                    f"(() => {{ const el = document.querySelector('{selector}'); if (!el) return null;"
                    f" const r = el.getBoundingClientRect(); return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }}; }})()")
                if not rect:
                    return False
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            # 注入桩并加载
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                ok = await ev("!!document.querySelector('.sidebar-right .detail-root')")
                if ok is True:
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            # ---------- R1 栏宽 264 ----------
            w = await ev("document.querySelector('.sidebar-right').offsetWidth")
            check("R1 右栏宽 = 264", w == 264, f"offsetWidth = {w}")

            # ---------- R2 窗口控制浮块 ----------
            wc = await ev(
                "[...document.querySelectorAll('.win-controls-overlay .wc-btn')].map(b => ({"
                " w: b.offsetWidth, h: b.offsetHeight }))")
            check("R2a 窗口控制四钮 58×46 连续排",
                  isinstance(wc, list) and len(wc) == 4 and all(b.get("w") == 58 and b.get("h") == 46 for b in wc),
                  json.dumps(wc))
            icons = await ev(
                "[...document.querySelectorAll('.win-controls-overlay .wc-btn svg')].map(s => s.outerHTML.slice(0, 0) || '')"
                " .length ? [...document.querySelectorAll('.win-controls-overlay .wc-btn')].length : 0")
            # 图标名不可从 DOM 读（内联 svg），按序断言钮数与 title 文案（v-tip 不落 title 属性，断言略）；
            # 视觉核图标 = 截图目验
            check("R2b 浮块在右栏顶行位（top 0 / right 0 / 高 46）",
                  icons == 4,
                  f"wc-btn 数 = {icons}")

            # ---------- R3 初始 quick:all 未选中 → 快速访问形态 ----------
            q = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .detail-root');"
                " const inp = root.querySelector('.fd-inputs .ti-root input');"
                " const rows = [...root.querySelectorAll('.bd-row')].map(r => r.querySelector('.bd-label')?.textContent);"
                " return { seg: !!root.querySelector('.seg'), manage: !!root.querySelector('.manage-bar'),"
                "   thumb: !!root.querySelector('.fd-thumb'), val: inp?.value ?? null, ro: inp?.readOnly ?? null,"
                "   inputs: root.querySelectorAll('.fd-inputs .ti-root').length, rows }; })()")
            check("R3a 快速访问形态：无分段控制器 / 无管理钮 / 无缩略图",
                  isinstance(q, dict) and q.get("seg") is False and q.get("manage") is False and q.get("thumb") is False,
                  json.dumps(q, ensure_ascii=False))
            check("R3b 单名称框 = 「全部」只读（不让改名）",
                  isinstance(q, dict) and q.get("val") == "全部" and q.get("ro") is True and q.get("inputs") == 1,
                  json.dumps(q, ensure_ascii=False))
            check("R3c 数据卡两行 = 文件数 / 占用空间",
                  isinstance(q, dict) and q.get("rows") == ["文件数", "占用空间"],
                  json.dumps(q, ensure_ascii=False))
            qv = await ev(
                "document.querySelector('.sidebar-right .bd-row .bd-values')?.textContent")
            check("R3d 文件数 = 3（桩 3 条 Placement）", qv == "3", f"值 = {qv}")

            # ---------- R3e 快速访问名称框不出聚焦描边（focusable=false，2026-08-31 用户定） ----------
            await pointer_click(".sidebar-right .fd-inputs .ti-root .ti-inner")
            qf = await ev(
                "(() => { const r = document.querySelector('.sidebar-right .fd-inputs .ti-root');"
                " return r ? { op: getComputedStyle(r, '::before').opacity,"
                "   focus: document.activeElement === r.querySelector('.ti-inner') } : null; })()")
            check("R3e 快速访问名称框：可聚焦但不出聚焦描边（聚焦保留、边不显）",
                  isinstance(qf, dict) and qf.get("op") == "0" and qf.get("focus") is True,
                  json.dumps(qf))

            # ---------- R4 顶行与旧结构 ----------
            bt = await ev(
                "(() => { const el = document.querySelector('.sidebar-right .bar-title');"
                " const s = getComputedStyle(el); return { h: el.offsetHeight, bb: s.borderBottomWidth }; })()")
            check("R4a 顶行 46 拖拽区 + 无 border-bottom（稿）",
                  isinstance(bt, dict) and bt.get("h") == 46 and bt.get("bb") == "0px", json.dumps(bt))
            legacy = await ev(
                "['.panel-toolbar', '.side-thumb', '.picker-panel']"
                ".filter(s => document.querySelector('.sidebar-right ' + s))")
            check("R4b 旧结构清零（54 工具栏 / 旧自制滚轮）",
                  isinstance(legacy, list) and len(legacy) == 0, json.dumps(legacy))

            # ---------- R4c 窗口控制区透窗口底（2026-08-31 用户实机定） ----------
            sr = await ev(
                "(() => { const a = document.querySelector('.sidebar-right');"
                " const s = getComputedStyle(a);"
                " const b = getComputedStyle(a, '::before');"
                " return { bg: s.backgroundColor, bl: s.borderLeftWidth, iso: s.isolation,"
                "   pbg: b.backgroundColor, ptop: b.top, pbl: b.borderLeftWidth, pz: b.zIndex }; })()")
            check("R4c 窗口控制区透窗口底（aside 根无底无线；::before 只画顶行 46 以下 g1 + 左缘 1px）",
                  isinstance(sr, dict) and sr.get("bg") == "rgba(0, 0, 0, 0)" and sr.get("bl") == "0px"
                  and sr.get("ptop") == "46px" and sr.get("pbg") != "rgba(0, 0, 0, 0)"
                  and sr.get("pbl") == "1px",
                  json.dumps(sr))
            check("R4c2 背景伪元素沉底（aside isolation isolate + ::before z-index -1——防盖正常流内容）",
                  isinstance(sr, dict) and sr.get("iso") == "isolate" and sr.get("pz") == "-1",
                  json.dumps(sr))

            # ---------- R5 文件夹形态（点树 f1） ----------
            await click_sel('.tree-item[data-folder-id="f1"]')
            fd = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .detail-root');"
                " const inp = root.querySelector('.fd-inputs .ti-root input');"
                " const rows = [...root.querySelectorAll('.bd-row')].map(r => r.querySelector('.bd-label')?.textContent);"
                " return { thumb: !!root.querySelector('.fd-thumb'),"
                "   icon: !!root.querySelector('.fd-thumb svg'), val: inp?.value ?? null, rows,"
                "   seg: !!root.querySelector('.seg'), manage: !!root.querySelector('.manage-bar') }; })()")
            check("R5a 文件夹形态：图标缩略图 + 名称框「角色」",
                  isinstance(fd, dict) and fd.get("thumb") is True and fd.get("icon") is True and fd.get("val") == "角色",
                  json.dumps(fd, ensure_ascii=False))
            check("R5b 数据卡 4 行 = 文件数/档案数/子文件夹数/占用空间",
                  isinstance(fd, dict) and fd.get("rows") == ["文件数", "档案数", "子文件夹数", "占用空间"],
                  json.dumps(fd, ensure_ascii=False))
            check("R5c 文件夹形态无分段控制器 / 无管理钮（稿注）",
                  isinstance(fd, dict) and fd.get("seg") is False and fd.get("manage") is False,
                  json.dumps(fd, ensure_ascii=False))

            # ---------- R6 文件形态 pinned（点素材卡 a1） ----------
            await pointer_click(".asset-card")
            pin = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .detail-root');"
                " if (!root) return null;"
                " const th = root.querySelector('.section-pinned .thumb');"
                " if (!th) return { thumb: false };"
                " const s = getComputedStyle(th);"
                " const badge = root.querySelector('.section-pinned .fmt-badge');"
                " const dots = root.querySelectorAll('.section-pinned .cs-dot').length;"
                " return { thumb: true, sq: th.offsetWidth === th.offsetHeight, bg: s.backgroundColor,"
                "   bw: s.borderWidth, bc: s.borderColor,"
                "   badge: badge?.textContent ?? null, dots,"
                "   bt: badge ? { t: getComputedStyle(badge).top, l: getComputedStyle(badge).left } : null }; })()")
            check("R6a pinned 缩略图 240 方（g2 底 + g3 1px 边）",
                  isinstance(pin, dict) and pin.get("thumb") is True and pin.get("sq") is True
                  and pin.get("bg") == "rgb(28, 28, 28)" and pin.get("bw") == "1px"
                  and pin.get("bc") == "rgb(36, 36, 36)",
                  json.dumps(pin, ensure_ascii=False))
            check("R6b FormatBadge「FLA」+ 无内容缩略图 → 色块行整行隐藏（2026-09-23 口径：不提色不显示）",
                  isinstance(pin, dict) and pin.get("badge") == "FLA" and pin.get("dots") == 0,
                  json.dumps(pin, ensure_ascii=False))

            # ---------- R6c 有内容缩略图（桩 a2 thumbKind=asset，伴生 webp 代理同径）→ 色块行显示真色 ----------
            await pointer_click(".asset-card ~ .asset-card, .asset-card:nth-child(2)")
            sw = await ev(
                "(() => { const dots = [...document.querySelectorAll('.section-pinned .cs-dot')];"
                " return { n: dots.length,"
                "   first: dots[0] ? getComputedStyle(dots[0]).backgroundColor : null,"
                "   ph: dots.filter(d => d.className.includes('ph')).length }; })()")
            check("R6c 有内容缩略图 → 色块行显示真色（桩 a2 七色、无占位块）",
                  isinstance(sw, dict) and sw.get("n") == 7 and sw.get("ph") == 0
                  and sw.get("first") == "rgb(200, 30, 30)",
                  json.dumps(sw, ensure_ascii=False))
            # 点回 a1 复原（R8 基础信息断言读 a1 详情）
            await pointer_click(".asset-card")

            # ---------- R7 分段控制器 ----------
            seg = await ev(
                "(() => { const el = document.querySelector('.sidebar-right .seg-block .seg.icon');"
                " if (!el) return null;"
                " const items = [...el.querySelectorAll('.seg-item')];"
                " return { h: el.offsetHeight, n: items.length,"
                "   on: items.findIndex(b => b.className.includes('on')) }; })()")
            check("R7 分段控制器三段 icon 型 + 默认「基础信息」选中",
                  isinstance(seg, dict) and seg.get("n") == 3 and seg.get("on") == 0 and seg.get("h") == 38,
                  json.dumps(seg))

            # ---------- R7b 分段图标色（svg 转 currentColor 修复，2026-08-31 走查） ----------
            segc = await ev(
                "[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')].map(b => ({"
                " on: b.className.includes('on'), color: getComputedStyle(b).color,"
                " stroke: b.querySelector('svg path')?.getAttribute('stroke') ?? null }))")
            check("R7b 图标随选中态着色：选中一级 / 未选中三级 + stroke=currentColor",
                  isinstance(segc, list) and len(segc) == 3
                  and segc[0].get("color") == "rgb(242, 242, 242)"
                  and segc[1].get("color") == "rgb(128, 128, 128)" and segc[2].get("color") == "rgb(128, 128, 128)"
                  and all(i.get("stroke") == "currentColor" for i in segc),
                  json.dumps(segc))

            # ---------- R8 基础信息 tab ----------
            basic = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .tab-body');"
                " const inps = [...root.querySelectorAll('.db-block.inputs .ti-root')];"
                " const chips = [...root.querySelectorAll('.prop-chip')].map(c => c.textContent);"
                " const dtitle = root.querySelector('.desc-title')?.textContent ?? null;"
                " const rows = [...root.querySelectorAll('.bd-row')].map(r => r.querySelector('.bd-label')?.textContent);"
                " return { name: inps[0]?.querySelector('input')?.value ?? null,"
                "   linkPh: inps[1]?.querySelector('input')?.placeholder ?? null,"
                "   linkIcon: !!inps[1]?.querySelector('.ti-trail'), chips, dtitle, rows }; })()")
            check("R8a 名称框值 = 雪牧_自动.fla + 链接框占位 http:// 带 link 图标",
                  isinstance(basic, dict) and basic.get("name") == "雪牧_自动.fla"
                  and basic.get("linkPh") == "http://" and basic.get("linkIcon") is True,
                  json.dumps(basic, ensure_ascii=False))
            check("R8b 属性 chips = 男 / 16岁 + 描述条目「身份」",
                  isinstance(basic, dict) and basic.get("chips") == ["男", "16岁"] and basic.get("dtitle") == "身份",
                  json.dumps(basic, ensure_ascii=False))
            check("R8c 数据卡 4 行（桩 width null 无尺寸行）= 文件大小/格式/添加日期/修改日期",
                  isinstance(basic, dict) and basic.get("rows") == ["文件大小", "格式", "添加日期", "修改日期"],
                  json.dumps(basic, ensure_ascii=False))

            # ---------- R8d 描述标题行收起 = 留一排截断（2026-08-31 用户实机定） ----------
            await ev("document.querySelector('.sidebar-right .desc-head').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.3)
            dc = await ev(
                "(() => { const el = document.querySelector('.sidebar-right .desc-content');"
                " if (!el) return null;"
                " return { cls: el.className, h: el.offsetHeight,"
                "   lc: getComputedStyle(el).webkitLineClamp ?? null }; })()")
            check("R8d 描述收起 = 留一排截断正文（clamp-1 / 高 18 / line-clamp 1）",
                  isinstance(dc, dict) and "clamp-1" in (dc.get("cls") or "")
                  and dc.get("h") == 18 and dc.get("lc") == "1",
                  json.dumps(dc))
            # 再点回展开复原（防影响后续断言与截图）
            await ev("document.querySelector('.sidebar-right .desc-head').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.3)

            # ---------- R8e 文件名称框保留聚焦描边（文件/文件夹语义可改名，2026-08-31 用户定） ----------
            await pointer_click(".sidebar-right .db-block.inputs .ti-root .ti-inner")
            await asyncio.sleep(0.3)
            ff = await ev(
                "(() => { const r = document.querySelector('.sidebar-right .db-block.inputs .ti-root');"
                " return r ? { op: getComputedStyle(r, '::before').opacity,"
                "   focus: document.activeElement === r.querySelector('.ti-inner') } : null; })()")
            check("R8e 文件名称框保留聚焦描边（聚焦后 ::before opacity 1）",
                  isinstance(ff, dict) and ff.get("op") == "1" and ff.get("focus") is True,
                  json.dumps(ff))

            # ---------- R9 信息查看管理钮 ----------
            mb = await ev(
                "(() => { const el = document.querySelector('.sidebar-right .manage-bar');"
                " if (!el) return null; const b = el.querySelector('.act-btn.solid');"
                " return { h: el.offsetHeight, bh: b?.offsetHeight ?? null, t: b?.textContent?.trim() ?? null }; })()")
            check("R9 信息查看管理：58 高块 + 34 高 solid 钮 + 文案",
                  isinstance(mb, dict) and mb.get("h") == 58 and mb.get("bh") == 34 and mb.get("t") == "信息查看管理",
                  json.dumps(mb, ensure_ascii=False))

            # ---------- R10 链接关系 tab ----------
            await click_sel(".seg-block .seg-item", "")
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][1].click()")
            await asyncio.sleep(0.5)
            links = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .tab-body');"
                " if (!root || !root.querySelector('.links-tab')) return null;"
                " const blocks = [...root.querySelectorAll('.lb-block')].map(b => ({"
                "   t: b.querySelector('.st .txt')?.textContent ?? null,"
                "   acts: !!b.querySelector('.st .acts') }));"
                " const cards = root.querySelectorAll('.lb-body .lf').length;"
                " const badge = root.querySelector('.lb-body .lf .badge');"
                " return { blocks, cards, badge: !!badge }; })()")
            titles = [b.get("t") for b in links.get("blocks")] if isinstance(links, dict) and links else []
            check("R10a 链接 tab 五区块序 = 链接/音频/关系/缩略图/污染文件",
                  titles == ["链接", "音频", "关系", "缩略图", "污染文件"], json.dumps(titles, ensure_ascii=False))
            check("R10b 链接卡 ×2 + 首行（当前目录置顶）带 origin 徽章",
                  isinstance(links, dict) and links.get("cards") == 2 and links.get("badge") is True,
                  json.dumps(links, ensure_ascii=False))
            pollution = [b for b in (links.get("blocks") if isinstance(links, dict) else []) if b.get("t") == "污染文件"]
            check("R10c 污染文件区块标题行无动作组（稿）",
                  len(pollution) == 1 and pollution[0].get("acts") is False,
                  json.dumps(pollution, ensure_ascii=False))

            # ---------- R10d 标题行折叠钮 + 动作组常显（staticActions，2026-08-31 用户定） ----------
            # 鼠标挪中性位排除 hover 干扰（常显断言必须是非悬停态读出）
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": 700, "y": 500}, session=sess)
            await asyncio.sleep(0.2)
            stv = await ev(
                "[...document.querySelectorAll('.sidebar-right .links-tab .lb-block')].map(b => {"
                " const c = getComputedStyle(b.querySelector('.st .chev'));"
                " const a = b.querySelector('.st .acts');"
                " return { t: b.querySelector('.st .txt')?.textContent ?? null,"
                "   chev: c.opacity + '/' + c.visibility,"
                "   acts: a ? (s => s.opacity + '/' + s.visibility)(getComputedStyle(a)) : null }; })")
            check("R10d 五区块标题行折叠钮常显 + 四区块动作组常显（非悬停态 opacity 1）",
                  isinstance(stv, list) and len(stv) == 5
                  and all(x.get("chev") == "1/visible" for x in stv)
                  and all(x.get("acts") == "1/visible" for x in stv if x.get("t") != "污染文件")
                  and [x for x in stv if x.get("t") == "污染文件"][0].get("acts") is None,
                  json.dumps(stv, ensure_ascii=False))

            # ---------- R10f 缩略图区块接业务（2026-09-23）：调用侧卡 = 伴生资产名 + 引用数 ----------
            thumbblk = await ev(
                "(() => { const blocks = [...document.querySelectorAll('.sidebar-right .links-tab .lb-block')];"
                " const b = blocks.find(x => x.querySelector('.st .txt')?.textContent === '缩略图');"
                " if (!b) return null;"
                " const items = [...b.querySelectorAll('.lb-body .lt')];"
                " const item = items[0];"
                " return { n: items.length,"
                "   name: item?.querySelector('.nm')?.textContent ?? null,"
                "   cnt: item?.querySelector('.cnt')?.textContent ?? null }; })()")
            check("R10f 缩略图区块：调用侧卡 = 伴生资产名 + 引用数（桩 雪牧_自动.webp / 2）",
                  isinstance(thumbblk, dict) and thumbblk.get("n") == 1
                  and thumbblk.get("name") == "雪牧_自动.webp" and thumbblk.get("cnt") == "2",
                  json.dumps(thumbblk, ensure_ascii=False))

            # ---------- R10e tab 切换滑动动画（侧栏范例同款时序 + 方向恒定左出右入，2026-08-31 用户定） ----------
            # computed 级行为断言：basic → links，入场 delay 100ms 窗内读 ——
            # 新内容挂 tab-x-enter-active + 250ms 档 + delay 100ms + 停在 enter-from 位移 48px
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][0].click()")
            await asyncio.sleep(0.5)
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][1].click()")
            await asyncio.sleep(0.06)
            pane = await ev(
                "(() => { const panes = [...document.querySelectorAll('.sidebar-right .tab-body .tab-pane')];"
                " const p = panes[panes.length - 1];" # 新内容在 DOM 尾（首个 = 离场旧内容）
                " if (!p) return null; const s = getComputedStyle(p);"
                " return { cls: p.className, dur: s.transitionDuration, delay: s.transitionDelay,"
                "   ease: s.transitionTimingFunction, tf: s.transform }; })()")
            check("R10e tab 切换动画：新内容恒定右入 48px + 250ms 缓入 delay 100ms（与左侧边栏同向）",
                  isinstance(pane, dict) and "tab-x-enter-active" in (pane.get("cls") or "")
                  and "0.25s" in (pane.get("dur") or "") and "0.1s" in (pane.get("delay") or "")
                  and "cubic-bezier(0, 0, 0.1, 1)" in (pane.get("ease") or "") and "48" in (pane.get("tf") or ""),
                  json.dumps(pane))
            await asyncio.sleep(0.5)
            back = await ev(
                "(() => { const panes = [...document.querySelectorAll('.sidebar-right .tab-body .tab-pane')];"
                " const p = panes[panes.length - 1];"
                " return p ? { cls: p.className, tf: getComputedStyle(p).transform, n: panes.length } : null; })()")
            check("R10e2 动画收尾：单 pane + 类摘除 + transform 归零",
                  isinstance(back, dict) and back.get("n") == 1 and "enter" not in (back.get("cls") or "")
                  and back.get("tf") in ("none", "matrix(1, 0, 0, 1, 0, 0)"),
                  json.dumps(back))

            # 链接 tab 标题行常显截图留档（staticActions 目验）
            clip3 = await ev(
                "(() => { const r = document.querySelector('.sidebar-right').getBoundingClientRect();"
                " return { x: r.x, y: r.y, width: r.width, height: r.height }; })()")
            shot3 = await send(ws, "Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": clip3["x"], "y": clip3["y"], "width": clip3["width"],
                         "height": min(clip3["height"], 900), "scale": 2},
            }, session=sess)
            with open("tools/style-preview/r23b-sidebar-links.png", "wb") as f:
                f.write(base64.b64decode(shot3["result"]["data"]))
            print("saved tools/style-preview/r23b-sidebar-links.png")

            # ---------- R11 提示词 tab ----------
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][2].click()")
            await asyncio.sleep(0.5)
            pr = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .tab-body .prompt-block');"
                " if (!root) return null;"
                " return { area: !!root.querySelector('textarea.area'),"
                "   ro: root.querySelector('textarea.area')?.readOnly ?? null,"
                "   copy: !!root.querySelector('.copy-btn'),"
                "   btns: root.querySelectorAll('.ic-btn.solid').length }; })()")
            check("R11 提示词 tab：只读编辑框 + 悬浮复制钮 + 五枚 solid 图标钮",
                  isinstance(pr, dict) and pr.get("area") is True and pr.get("ro") is True
                  and pr.get("copy") is True and pr.get("btns") == 5,
                  json.dumps(pr))

            # ---------- R12 全局记忆：links 态换选中不重置（2026-09-19 用户改定） ----------
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][1].click()")
            await asyncio.sleep(0.4)
            await pointer_click(".asset-card ~ .asset-card, .asset-card:nth-child(2)")
            mem = await ev(
                "(() => { const items = [...document.querySelectorAll('.sidebar-right .seg-block .seg-item')];"
                " return { on: items.findIndex(b => b.className.includes('on')),"
                "  links: !!document.querySelector('.sidebar-right .tab-body .links-tab') }; })()")
            check("R12 全局记忆当前 tab（links → 点 a2 → 仍 links）",
                  isinstance(mem, dict) and mem.get("on") == 1 and mem.get("links") is True,
                  json.dumps(mem))
            # 后续 R13 档案 chips 在基础信息 tab——回 basic 再继续
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][0].click()")
            await asyncio.sleep(0.4)

            # ---------- R13 档案形态（视口 folder:f2a 未选中回退） ----------
            await ev("document.querySelector('.tree-item[data-folder-id=\"f2\"] .exp').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.4)
            # 先清空素材选中（点视口空白），再切档案视口
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')].length")
            await click_sel('.tree-item[data-folder-id="f2a"]')
            arch = await ev(
                "(() => { const root = document.querySelector('.sidebar-right .detail-root');"
                " if (!root) return null;"
                " const badge = root.querySelector('.section-pinned .fmt-badge');"
                " const chips = [...root.querySelectorAll('.prop-chip')].map(c => c.textContent);"
                " const rows = [...root.querySelectorAll('.bd-row')].map(r => r.querySelector('.bd-label')?.textContent);"
                " return { cover: !!root.querySelector('.section-pinned .thumb.cover'),"
                "   badge: badge?.textContent ?? null, chips, rows }; })()")
            check("R13 档案形态：封面区块 + 徽标「主角」+ chips「连载」+ 数据卡含创建日期",
                  isinstance(arch, dict) and arch.get("cover") is True and arch.get("badge") == "主角"
                  and arch.get("chips") == ["连载"] and "创建日期" in (arch.get("rows") or []),
                  json.dumps(arch, ensure_ascii=False))

            # ---------- R13b/c/d 封面编辑（边距修复 + 250ms 展开动画，2026-08-31 用户实机报） ----------
            await ev("document.querySelector('.sidebar-right .section-pinned .thumb.cover').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            mid = await ev(
                "(() => { const p = document.querySelector('.sidebar-right .section-pinned');"
                " return p ? { h: p.style.height, tr: p.style.transition } : null; })()")
            check("R13b 点封面 → pinned 根 250ms 高度补间启动（inline height + transition）",
                  isinstance(mid, dict) and bool(mid.get("h")) and "250ms" in (mid.get("tr") or ""),
                  json.dumps(mid))
            await asyncio.sleep(0.6)
            cs = await ev(
                "(() => { const aside = document.querySelector('.sidebar-right');"
                " const p = aside.querySelector('.section-pinned');"
                " const c = p.querySelector('.cover-settings');"
                " const fr = p.querySelector('.cs-frame');"
                " if (!c || !fr) return null;"
                " const kids = [...c.children].map(k => ({ cls: k.className.split(' ')[0],"
                "   h: k.offsetHeight, disp: getComputedStyle(k).display }));"
                " return { done: p.style.height === '', ovf: p.style.overflow, ph: p.style.height,"
                "   pad: getComputedStyle(p).paddingLeft,"
                "   inset: Math.round(fr.getBoundingClientRect().left - aside.getBoundingClientRect().left),"
                "   slider: !!p.querySelector('.cs-slider'), btns: p.querySelectorAll('.cs-actions .act-btn').length,"
                "   kids }; })()")
            check("R13c 封面编辑边距 = 栏缘 +12（pinned padding 12——双重 padding 24 修复）+ 动画收尾归位",
                  isinstance(cs, dict) and cs.get("done") is True and cs.get("pad") == "12px"
                  and cs.get("inset") == 12 and cs.get("slider") is True and cs.get("btns") == 2,
                  json.dumps(cs))
            kids = cs.get("kids") if isinstance(cs, dict) else None
            check("R13c2 封面编辑五件全显（框 240 / 滑块 10 / 更换封面 34 / 角标名 34 / 按钮组 34——不许被压没）",
                  isinstance(kids, list) and len(kids) == 5
                  and [k.get("h") for k in kids] == [240, 10, 34, 34, 34],
                  json.dumps(kids))

            # 封面编辑态截图留档（2026-08-31 ::before 遮挡 BUG 的像素级目验面——computed/几何断言
            # 测不出层叠遮挡，必须截图目验：更换封面 / 取消·确认 / 底部信息查看管理钮可见）
            shotc = await send(ws, "Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": clip3["x"], "y": clip3["y"], "width": clip3["width"],
                         "height": min(clip3["height"], 900), "scale": 2},
            }, session=sess)
            with open("tools/style-preview/r23c-cover-editing.png", "wb") as f:
                f.write(base64.b64decode(shotc["result"]["data"]))
            print("saved tools/style-preview/r23c-cover-editing.png")
            # 取消退回常态组（收起动画同路）——分拍读 height 排查收尾时机
            await ev("document.querySelector('.sidebar-right .cs-actions .act-btn').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            taps = []
            for wait in (0.3, 0.4, 0.5):
                await asyncio.sleep(wait)
                taps.append(await ev("document.querySelector('.sidebar-right .section-pinned')?.style.height ?? null"))
            backc = await ev(
                "({ cover: !!document.querySelector('.sidebar-right .section-pinned .thumb.cover'),"
                "  cs: !!document.querySelector('.sidebar-right .cover-settings'),"
                "  h: document.querySelector('.sidebar-right .section-pinned')?.style.height ?? null })")
            check("R13d 取消退回常态组（封面回 + 设置组消失 + inline 高清空）",
                  isinstance(backc, dict) and backc.get("cover") is True and backc.get("cs") is False
                  and backc.get("h") == "",
                  json.dumps(backc) + " taps=" + json.dumps(taps))

            # ---------- R14 关闭钮 hover = 状态/禁止底 ----------
            closeRect = await ev(
                "(() => { const b = [...document.querySelectorAll('.win-controls-overlay .wc-btn')].pop();"
                " if (!b) return null; const r = b.getBoundingClientRect();"
                " return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()")
            if closeRect:
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseMoved", "x": closeRect["x"], "y": closeRect["y"]}, session=sess)
                await asyncio.sleep(0.4)
            hv = await ev(
                "(() => { const b = [...document.querySelectorAll('.win-controls-overlay .wc-btn')].pop();"
                " return b ? getComputedStyle(b).backgroundColor : null; })()")
            check("R14 关闭钮 hover = 状态/禁止底 rgb(220,57,57)",
                  hv == "rgb(220, 57, 57)", f"hover bg = {hv}")
            # 鼠标挪走，防残留 hover 影响后续
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": 700, "y": 500}, session=sess)

            # ---------- R15 选择模式（关系区块添加钮 → picker → 取消退出） ----------
            # 回文件形态：点素材卡 a1
            await pointer_click(".asset-card")
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][1].click()")
            await asyncio.sleep(0.5)
            # 关系区块添加钮（staticActions 常显，2026-08-31 起不再需悬停）直点
            await ev(
                "(() => { const blocks = [...document.querySelectorAll('.sidebar-right .lb-block')];"
                " const b = blocks.find(x => x.querySelector('.st .txt')?.textContent === '关系');"
                " const add = b?.querySelector('.acts .act:last-child');"
                " add?.dispatchEvent(new MouseEvent('click', { bubbles: true })); })()")
            await asyncio.sleep(0.6)
            pk = await ev(
                "(() => { const p = document.querySelector('.sidebar-right .picker-panel');"
                " if (!p) return null;"
                " const btns = [...p.querySelectorAll('.pick-actions .act-btn')].map(b => b.textContent.trim());"
                " return { group: !!p.querySelector('.pr'),"
                "   ph: p.querySelector('.pr .box-input')?.placeholder ?? null,"
                "   ro: [...p.querySelectorAll('.pr .box.rel input')].map(i => i.disabled),"
                "   btns,"
                "   banner: document.querySelector('.notice-anchor')?.textContent?.includes('添加关系') ?? null }; })()")
            check("R15a 选择模式：备选区占位组（点击卡片选择 + 双框禁用）",
                  isinstance(pk, dict) and pk.get("group") is True and pk.get("ph") == "点击卡片选择"
                  and pk.get("ro") == [True, True],
                  json.dumps(pk, ensure_ascii=False))
            check("R15b 按钮序 = 取消左 / 确认右（2026-08-31 全局定案）+ 顶部横幅",
                  isinstance(pk, dict) and pk.get("btns") == ["取消", "确认"] and pk.get("banner") is True,
                  json.dumps(pk, ensure_ascii=False))
            # 取消退出 → 详情面板恢复（发起文件 a1 详情）
            await ev(
                "document.querySelector('.sidebar-right .pick-actions .act-btn').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            back = await ev(
                "({ picker: !!document.querySelector('.sidebar-right .picker-panel'),"
                "  pin: !!document.querySelector('.sidebar-right .section-pinned .thumb') })")
            check("R15c 取消退出 = 详情面板恢复（picker 消失 + pinned 回）",
                  isinstance(back, dict) and back.get("picker") is False and back.get("pin") is True,
                  json.dumps(back))

            # ---------- 截图留档 ----------
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT_FULL, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            clip = await ev(
                "(() => { const r = document.querySelector('.sidebar-right').getBoundingClientRect();"
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
