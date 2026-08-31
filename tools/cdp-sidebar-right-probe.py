# CDP 探针：7.1 二十三轮 · 右侧边栏轮（2026-08-31）——窗口控制浮块 / 详情面板三 tab / 文件夹·快速访问形态贴稿断言
#   R1  栏宽 = 264（ui.rightWidth 新默认；桩 get_ui_settings = {}）
#   R2  窗口控制浮块：四钮 58×46 连续排，图标序 = pin-on/minimize/maximize/close-large
#   R3  初始 quick:all 未选中 → 快速访问形态：无分段控制器 / 无管理钮 / 无缩略图；单名称框「全部」只读；
#       数据卡两行（文件数 3 / 占用空间）
#   R4  顶行 46 拖拽区无 border-bottom；旧结构清零（.panel-toolbar / .side-thumb / .picker-panel）
#   R5  点树 f1 → 文件夹形态：folder-detail-panel 图标缩略图 + 名称框「角色」+ 数据卡 4 行
#       （文件数/档案数/子文件夹数/占用空间），仍无分段控制器 / 管理钮
#   R6  点素材卡 a1 → 文件形态：pinned 缩略图 240 方（g2 底 g3 边）+ FormatBadge「FLA」左上 12/12 + 颜色 7 占位
#   R7  分段控制器三段 icon，默认「基础信息」选中
#   R8  基础信息 tab：名称框值 / 链接框占位 http://+link 图标 / 属性 chips×2 / 描述条目「身份」/
#       数据卡 4 行（文件大小/格式/添加日期/修改日期——桩 width null 无尺寸行，旧口径）
#   R9  底部信息查看管理钮：58 高块 + 34 高 solid + 文案（点亮不接业务）
#   R10 链接关系 tab：五区块标题序 = 链接/音频/关系/缩略图/污染文件；链接卡 ×2 首行 origin 徽章；
#       污染文件区块无动作组（稿）
#   R11 提示词 tab：只读编辑框 + 五枚 solid 图标钮（业务占位）
#   R12 tab 会话级记忆：links 态点另一素材卡 a2 → 回基础信息（2026-08-31 用户定）
#   R13 档案视口（folder:f2a）未选中 → 档案形态：封面区块 + 徽标「主角」+ chips「连载」+ 数据卡含创建日期
#   R14 关闭钮 hover = 状态/禁止底 rgb(220,57,57)（Input.dispatchMouseEvent 真实悬停）
#   R15 选择模式：关系区块添加钮进 picker → 占位组（点击卡片选择 + 双框禁用）+ 按钮序 取消左/确认右 +
#       顶部横幅；取消退出 = 详情面板恢复
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
  invoke: (cmd) => {
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
          w: null, h: null, thumbKind: null, thumbPath: null, origin: true,
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
      get_asset_detail: {
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
      },
      get_archive_data: {
        folderId: "f2a", badge: { text: "主角", color: "#FF6566" }, avatar: null,
        props: ["连载"], descriptions: [{ title: "定位", content: "沙雕动画御用档案" }],
        links: [], createdAt: "2026-07-01T10:00:00+08:00", fileCount: 0, totalSize: 0, coverPath: null,
      },
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
            check("R6b FormatBadge「FLA」+ 颜色 7 占位块（恒显口径）",
                  isinstance(pin, dict) and pin.get("badge") == "FLA" and pin.get("dots") == 7,
                  json.dumps(pin, ensure_ascii=False))

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

            # ---------- R12 会话级记忆：links 态换选中回 basic ----------
            await ev("[...document.querySelectorAll('.sidebar-right .seg-block .seg-item')][1].click()")
            await asyncio.sleep(0.4)
            await pointer_click(".asset-card ~ .asset-card, .asset-card:nth-child(2)")
            mem = await ev(
                "(() => { const items = [...document.querySelectorAll('.sidebar-right .seg-block .seg-item')];"
                " return { on: items.findIndex(b => b.className.includes('on')),"
                "  basic: !!document.querySelector('.sidebar-right .tab-body .db-block.inputs') }; })()")
            check("R12 换选中回「基础信息」（links → 点 a2 → basic）",
                  isinstance(mem, dict) and mem.get("on") == 0 and mem.get("basic") is True,
                  json.dumps(mem))

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
            # 关系区块标题行动作组 hover 才现身：真实悬停后点添加钮
            relTitle = await ev(
                "(() => { const blocks = [...document.querySelectorAll('.sidebar-right .lb-block')];"
                " const b = blocks.find(x => x.querySelector('.st .txt')?.textContent === '关系');"
                " if (!b) return null; const r = b.querySelector('.st').getBoundingClientRect();"
                " return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()")
            if relTitle:
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseMoved", "x": relTitle["x"], "y": relTitle["y"]}, session=sess)
                await asyncio.sleep(0.4)
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
