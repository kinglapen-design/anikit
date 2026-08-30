# CDP 探针：3.5 二轮 · 显示简介接入实证（2026-08-19；2026-08-30 适配 7.1 满铺卡新口径）
#   S1 菜单结构：显示简介 picker = 尺寸/文件大小/创建日期/修改日期/标签 5 项，无「无显示」，标签 disabled
#   S2 简介开（默认）：盒比例 = 图比例（7.1 满铺定稿——盒高 = 纯图高，名称/简介渐变罩叠图内
#      不占布局，cover 零裁；旧"卡高 = thumb + 57 简介区"排版废除）；.kc-info 可见；
#      图片卡 "1920 x 1080"；视频卡 "02:05"；无尺寸卡回退文件大小（2026-08-19 用户定：全档空值兜底统一回退 formatSize）
#   S3 简介关：卡高不变（满铺卡简介行只在渐变罩内显隐，缩卡必错位裁图——2026-08-30 用户实机
#      "卡片没按图片尺寸、都被裁剪"修订）；.kc-info 不渲染（v-if 空串）；名称行仍在
#   S4 下拉分流：文件大小 "1.5 MB" / 创建日期 YYYY/MM/DD HH:mm（null 卡回退文件大小 "512 B"，
#      2026-08-19 用户定：档内无数据回退文件大小，不显示 "—"）/ 修改日期 / 切回尺寸还原
#   S5 截图 brief-on.png / brief-off.png / brief-created-at.png（tools/style-preview/；
#      创建日期档截图验 null 卡显示文件大小）
# 桩：4 卡（1920×1080 图 / 视频 duration 125.4 / fla 无尺寸 formatIcon / createdAt null），全 origin=true
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-brief-probe.py
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9333
URL = "http://localhost:5373"
SHOT_ON = "tools/style-preview/brief-on.png"
SHOT_OFF = "tools/style-preview/brief-off.png"
SHOT_CREATED = "tools/style-preview/brief-created-at.png"
SLEEP = 0.35

# 桩：f1 角色（4 素材，全 origin=true，thumbPath null 显占位底色——不影响高度断言）。
# AN-IMG 普通图（尺寸/大小/双时间非空）；AN-VID 视频 duration 125.4（→ 02:05）；
# AN-FLA 无尺寸（w/h null + formatIcon → 尺寸档回退 formatSize(3145728) = "3.0 MB"）；
# AN-OLD createdAt null（→ 创建日期档回退 formatSize(512) = "512 B"）。
STUB = r"""
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [{ id: "f1", name: "角色", count: 4, children: [] }],
};
const mk = (assetId, name, patch) => ({
  assetId, folderId: "f1", name, w: 800, h: 600, ext: "png", thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 1024, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T12:34:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
  ...patch,
});
const PLACEMENTS = [
  mk("AN-IMG", "照片.png", { w: 1920, h: 1080, size: 1572864 }),
  mk("AN-VID", "镜头.mp4", { w: 1280, h: 720, ext: "mp4", duration: 125.4, size: 52428800 }),
  mk("AN-FLA", "场景源文件.fla", { w: null, h: null, ext: "fla", thumbKind: "formatIcon", size: 3145728 }),
  mk("AN-OLD", "旧图.png", { size: 512, createdAt: null }),
];
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 1 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
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
        m = raw.get("method", "")
        if m == "Runtime.consoleAPICalled":
            args = raw["params"].get("args", [])
            text = " ".join(str(a.get("value", a.get("description", ""))) for a in args)
            print(f"[console.{raw['params']['type']}]", text[:300])
        elif m == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（vite node 脱离 pnpm 壳进程树，端口直杀兜底）"""
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                subprocess.run(["taskkill", "/PID", line.split()[-1], "/F"], capture_output=True)
    except Exception:
        pass


async def main():
    fails = []

    # dev server 探测（未起则自起，结束杀）
    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(
            ["pnpm", "dev"], shell=True,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "about:blank",
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
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, buttons, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons,
                    "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def click(x, y):
                await mouse("mousePressed", x, y, 1)
                await mouse("mouseReleased", x, y, 0, clicks=1)
                await asyncio.sleep(SLEEP)

            async def key(k, code, vk):
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": k, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": k, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await asyncio.sleep(SLEEP)

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            async def shot(path):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", path)

            # —— 元素定位辅助 ——
            async def el_pos(expr):
                return await ev(f"""
                  (() => {{
                    const el = {expr};
                    if (!el) return null;
                    const r = el.getBoundingClientRect();
                    return {{ x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) }};
                  }})()
                """)

            # 查看方式钮 = 内容区工具栏按钮组第 4 个（7.1 工具栏重构：.ic-btn 族，同内容容器探针 TB 口径）
            VIEW_BTN = "[...document.querySelectorAll('.content-area .panel-toolbar .btn-group .ic-btn')][3]"
            BRIEF_ROW = "[...document.querySelectorAll('.app-menu .group .row')].find((r) => r.querySelector('.label')?.textContent.trim() === '显示简介')"

            async def ensure_menu():
                opened = await ev(f"!!({BRIEF_ROW})")
                if not opened:
                    p = await el_pos(VIEW_BTN)
                    await click(p["x"], p["y"])
                    await asyncio.sleep(0.3)

            async def open_brief_picker():
                await ensure_menu()
                p = await el_pos(f"({BRIEF_ROW})?.querySelector('.picker')")
                await click(p["x"], p["y"])

            async def pick_brief(label):
                pos = await el_pos(f"[...document.querySelectorAll('.app-menu .menu-item')].find((el) => el.textContent.trim() === '{label}')")
                if pos:
                    await click(pos["x"], pos["y"])

            async def card_meta(key):
                # 7.1 满铺卡：简介行 = FileCard .kc-info（旧 .meta 类随重构废除）
                return await ev(f"(document.querySelector('.asset-card[data-key=\"{key}|f1\"] .kc-info')?.textContent ?? '').trim()")

            # 准备：等树渲染，进 f1 视口
            n = 0
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 1:
                    break
                await asyncio.sleep(0.5)
            if not n:
                print("树未渲染，探针无法执行")
                return 2
            f1 = await el_pos("document.querySelector('.tree-item[data-folder-id=\"f1\"]')")
            await click(f1["x"], f1["y"])
            cards = 0
            for _ in range(20):
                cards = await ev("document.querySelectorAll('.asset-card').length")
                if cards == 4:
                    break
                await asyncio.sleep(0.5)
            print("f1 卡片数:", cards)
            check("S0 f1 四卡渲染", cards == 4, f"实际 {cards}")

            # ── S1 菜单结构：显示简介 picker 5 项 ──
            print("S1 显示简介 picker")
            await open_brief_picker()
            items = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((el) => el.textContent.trim())")
            print("  简介选项:", items)
            expect5 = ["尺寸", "文件大小", "创建日期", "修改日期", "标签"]
            check("S1a picker = 5 项且无「无显示」", items == expect5, str(items))
            tag_disabled = await ev("""
              (() => {
                const it = [...document.querySelectorAll('.app-menu .menu-item')].find((el) => el.textContent.trim() === '标签');
                return it ? it.disabled : null;
              })()
            """)
            check("S1b 「标签」项 disabled", tag_disabled is True, f"disabled={tag_disabled}")
            await pick_brief("尺寸")  # 关 picker（briefMode 本为 size，不变）；主菜单保持开

            # ── S2 简介开（默认态）──
            print("S2 简介开")
            geo = await ev("""
              (() => {
                const card = document.querySelector('.asset-card[data-key="AN-IMG|f1"]');
                if (!card) return null;
                return {
                  cardW: card.offsetWidth,
                  cardH: card.offsetHeight,
                  infoVisible: card.querySelector('.kc-info')?.offsetParent != null,
                };
              })()
            """)
            print("  几何:", geo)
            # 7.1 满铺定稿：盒高 = 纯图高，盒比例 = 图比例（cover 零裁）——本次修复核心回归断言
            ratio_ok = bool(geo) and abs(geo["cardW"] / geo["cardH"] - 1920 / 1080) < 0.02
            check("S2a 盒比例 = 图比例 16:9（盒高 = 纯图高，无简介区占位）", ratio_ok,
                  f"{geo and geo['cardW']}x{geo and geo['cardH']} 比 {geo and round(geo['cardW'] / geo['cardH'], 4)}")
            check("S2b 简介行 .kc-info 可见", bool(geo) and geo["infoVisible"] is True, str(geo))
            m_img = await card_meta("AN-IMG")
            m_vid = await card_meta("AN-VID")
            m_fla = await card_meta("AN-FLA")
            print("  meta:", m_img, "/", m_vid, "/", m_fla)
            check("S2c 图片卡 meta = 1920 x 1080", m_img == "1920 x 1080", repr(m_img))
            check("S2d 视频卡 meta = 02:05（duration 125.4）", m_vid == "02:05", repr(m_vid))
            check("S2e 无尺寸卡回退文件大小 3.0 MB", m_fla == "3.0 MB", repr(m_fla))

            # 简介开截图（关菜单后拍）
            await key("Escape", "Escape", 27)
            await asyncio.sleep(0.3)
            await shot(SHOT_ON)

            # ── S3 简介关 ──
            print("S3 简介关")
            await ensure_menu()
            sw = await el_pos(f"({BRIEF_ROW})?.querySelector('.app-switch')")
            if not sw:
                check("S3a 显示简介开关存在", False, "未找到 .app-switch")
                return 2
            await click(sw["x"], sw["y"])
            await asyncio.sleep(0.6)  # 等响应式刷新（满铺卡不随开关重排，仅简介行显隐切换）
            off = await ev("""
              (() => {
                const card = document.querySelector('.asset-card[data-key="AN-IMG|f1"]');
                if (!card) return null;
                return {
                  cardH: card.offsetHeight,
                  info: !!card.querySelector('.kc-info'),
                  nameVisible: card.querySelector('.kc-name')?.offsetParent != null,
                };
              })()
            """)
            print("  关闭态:", json.dumps(off, ensure_ascii=False))
            h_on = geo["cardH"] if geo else None
            # 2026-08-30 满铺修订：关 = 仅隐简介行——盒高 = 纯图高恒定（缩卡必致盒比例 ≠ 图比例 cover 裁图）
            check("S3a 卡高不变（满铺卡简介行不占布局高度）", bool(off) and h_on is not None and off["cardH"] == h_on,
                  f"开 {h_on} → 关 {off and off['cardH']}")
            check("S3b 简介行 .kc-info 不渲染（info 空串 v-if）", bool(off) and off["info"] is False, str(off and off["info"]))
            check("S3c 名称行 .kc-name 仍可见", bool(off) and off["nameVisible"] is True, str(off and off["nameVisible"]))
            await key("Escape", "Escape", 27)
            await asyncio.sleep(0.3)
            await shot(SHOT_OFF)

            # ── S4 下拉分流（先切回简介开）──
            print("S4 下拉分流")
            await ensure_menu()
            sw = await el_pos(f"({BRIEF_ROW})?.querySelector('.app-switch')")
            await click(sw["x"], sw["y"])  # 重新打开简介
            await asyncio.sleep(0.6)

            import re
            DATE_RE = r"^\d{4}/\d{2}/\d{2} \d{2}:\d{2}$"

            await open_brief_picker()
            await pick_brief("文件大小")
            await asyncio.sleep(0.2)
            m = await card_meta("AN-IMG")
            print("  文件大小:", m)
            check("S4a 文件大小 = 1.5 MB", m == "1.5 MB" and re.search(r"\d+(\.\d+)?\s(B|KB|MB|GB)$", m) is not None, repr(m))

            await open_brief_picker()
            await pick_brief("创建日期")
            await asyncio.sleep(0.2)
            m_img_c = await card_meta("AN-IMG")
            m_old_c = await card_meta("AN-OLD")
            print("  创建日期:", m_img_c, "/", m_old_c)
            # 空值兜底新口径（2026-08-19 用户定）：档内无数据回退文件大小——
            # AN-OLD createdAt null → formatSize(512) = "512 B"，不再显示 "—"
            check("S4b 创建日期格式 + null 卡回退文件大小",
                  re.match(DATE_RE, m_img_c or "") is not None and m_old_c == "512 B",
                  f"IMG={m_img_c!r} OLD={m_old_c!r}")
            # 创建日期档截图（验 null 卡显示文件大小；关菜单后拍）
            await key("Escape", "Escape", 27)
            await asyncio.sleep(0.3)
            await shot(SHOT_CREATED)

            await open_brief_picker()
            await pick_brief("修改日期")
            await asyncio.sleep(0.2)
            m_img_m = await card_meta("AN-IMG")
            m_old_m = await card_meta("AN-OLD")
            print("  修改日期:", m_img_m, "/", m_old_m)
            check("S4c 修改日期格式（两卡均非空）",
                  re.match(DATE_RE, m_img_m or "") is not None and re.match(DATE_RE, m_old_m or "") is not None,
                  f"IMG={m_img_m!r} OLD={m_old_m!r}")

            await open_brief_picker()
            await pick_brief("尺寸")
            await asyncio.sleep(0.2)
            m = await card_meta("AN-IMG")
            check("S4d 切回尺寸 = 1920 x 1080", m == "1920 x 1080", repr(m))
            await key("Escape", "Escape", 27)
    finally:
        edge.terminate()
        if spawned:
            # Windows：pnpm 壳 terminate 只杀壳——taskkill /T 杀树 + 端口直杀兜底
            subprocess.run(["taskkill", "/PID", str(spawned.pid), "/T", "/F"], capture_output=True)
            kill_listening_port(5373)

    print()
    if fails:
        print(f"{len(fails)} 项失败：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


sys.exit(asyncio.run(main()))
