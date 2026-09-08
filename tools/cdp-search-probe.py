# CDP 探针：3.6 分词搜索（2026-08-30 三轮重写——多功能悬浮栏接入，旧通栏 SearchBar + tag 化 chips 废除）
#   S0 全部视图 6 卡渲染（前置）
#   S1 全部视图直输「人物 走路」→ AND 分词命中两卡（空格 = 分词字符，无 chips）
#   S2 Esc 清空并失焦 → 恢复全部 6 卡（搜索态退出）
#   S3 扩展名大小写不敏感：「GIF」命中 跑步.gif
#   S4 无结果 → 中央空态文案「无搜索结果」
#   S5 路径命中：「走路参考」命中后代文件夹 f2 内的 场景-日.png
#   S6 链接 URL decode：「档案」命中 分镜01（路径含档案名）+ 参考图（links q=%E6%A1%A3%E6%A1%88）；
#      quick 视图不出档案区
#   S7 f1 视图范围：直输「场景」命中后代（递归开）/ 直输「~场景」全局命中 / 清空退出搜索态
#   S8 草稿「~场景」即时全局（轮询断言——headless 防抖+渲染延迟敏感）
#   S9 直输「//人物」命中两卡（// 与直接输入等效）
#   S10 档案名命中：f1 视图搜「分镜」→ 档案区「分镜档案」+ 档案内素材卡
#   S11 徽标命中：搜「进行中」→ 档案区「分镜档案」
#   S12 分组平铺：f1（配扩展名分组，查看方式菜单关递归——递归恒不分组）搜索态组标题消失；清空恢复
#   S13 Ctrl+F 聚焦悬浮栏输入框
#   S14 空文件夹 f3 视图直输「~人物」→ 全局命中视口外素材（基底分流回归钉）
#   （旧 S15「/」占位 tag / S16 kbd 徽章组随 tag 化废除删除；旧 S1b 空格成 chip 删除）
#   S17 悬浮栏宽度：常态 250 → 聚焦 450 → 失焦回 250（壳 = 唯一宽度动画源 300ms，轮询等宽到位）
#   S18 筛选展开：filter 钮 → chip×10 出现；chip 点击 = .on 本地切换且不触发 search_assets；
#      collapse 钮收起
#   S19 聚焦态点筛选直接展开（2026-09-09 修复钉：真鼠标链路 mousedown→click；mousedown.prevent
#      阻失焦，展开后输入框保持聚焦，壳宽探针实测档 >450）
#   S20 挤压裁剪 + 滚轮横滚（2026-09-09 修复钉：content-area 钉窄 500 等价侧栏挤压——壳钳制在
#      面板内不跑底色外 / 收起钮钉壳内 / 筛选组溢出横滚 wheel→scrollLeft；Emulation 视口收窄对
#      dockview 无效故用 inline 宽模拟）
#   截图 = 搜索态悬浮栏（f1 视图「人物」命中 + 聚焦 450 宽，clip 栏体）
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-search-probe.py
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
PORT = 9360
URL = "http://localhost:5373"
SHOT = "tools/style-preview/search-probe.png"
SLEEP = 0.35

# 桩：f1 角色（人物-开心/伤心-走路 + 参考图）/ f2 走路参考（场景-日，f1 后代）/
# fa 分镜档案（分镜01，f1 后代，kind=archive，徽标「进行中」）/ root 跑步.gif。
# f1 配扩展名分组（png 组）供 S12 平铺断言。search_assets 桩 = 后端同口径 JS 简化实现。
# 纪律：返回列表 / 对象一律深拷贝斩断共享引用（§12）。
STUB = r"""
window.__calls = { search_assets: [] };
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 5,
      groups: [{ id: "g1", name: "图片组", rules: [{ kind: "ext", values: ["png"] }] }],
      children: [
        { id: "f2", name: "走路参考", count: 1, children: [] },
        { id: "fa", name: "分镜档案", kind: "archive", count: 1, children: [] },
      ] },
    { id: "f3", name: "空文件夹", count: 0, children: [] },
  ],
};
const mk = (assetId, folderId, name, ext) => ({
  assetId, folderId, name, w: 800, h: 600, ext, thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 100, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T10:00:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
});
const PLACEMENTS = [
  mk("AN-1", "f1", "人物-开心-走路.png", "png"),
  mk("AN-2", "f1", "人物-伤心-走路.png", "png"),
  mk("AN-3", "f2", "场景-日.png", "png"),
  mk("AN-4", "fa", "分镜01.png", "png"),
  mk("AN-5", "root", "跑步.gif", "gif"),
  mk("AN-6", "f1", "参考图.png", "png"),
];
const META = {
  "AN-1": { name: "人物-开心-走路", ext: "png", links: [] },
  "AN-2": { name: "人物-伤心-走路", ext: "png", links: [] },
  "AN-3": { name: "场景-日", ext: "png", links: [] },
  "AN-4": { name: "分镜01", ext: "png", links: [] },
  "AN-5": { name: "跑步", ext: "gif", links: [] },
  "AN-6": { name: "参考图", ext: "png", links: ["https://x.com/?q=%E6%A1%A3%E6%A1%88"] },
};
const FOLDERS = {
  f1: { name: "角色", parent: null, block: "dir" },
  f2: { name: "走路参考", parent: "f1" },
  fa: { name: "分镜档案", parent: "f1", kind: "archive" },
};
const ARCHIVES = {
  fa: { badge: { text: "进行中", color: "#fff" }, props: ["二维"], descriptions: [{ title: "用途", content: "片头" }], links: [] },
};
const pct = (s) => { try { return decodeURIComponent(s.replace(/\+/g, " ")); } catch { return s; } };
const pathText = (fid) => {
  if (fid === "root") return "";
  const names = [];
  let cur = fid, guard = 0, block = null;
  while (FOLDERS[cur] && guard++ < 64) {
    names.push(FOLDERS[cur].name);
    if (FOLDERS[cur].parent) cur = FOLDERS[cur].parent;
    else { block = FOLDERS[cur].block; break; }
  }
  if (block === "dir") names.push("目录");
  return names.join(" ");
};
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 6, folderCount: 3 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [],
      list_archives: JSON.parse(JSON.stringify([{ folderId: "fa", badge: ARCHIVES.fa.badge, avatar: null,
        props: ARCHIVES.fa.props, descriptions: ARCHIVES.fa.descriptions, links: [], createdAt: "" }])),
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "search_assets") {
      window.__calls.search_assets.push(JSON.parse(JSON.stringify(args ?? {})));
      let q = (args?.query || "").trim();
      let global = !!(args?.scope?.global);
      let go = true;
      while (go) {
        go = false;
        if (q.startsWith("//")) { q = q.slice(2).trimStart(); go = true; }
        else if (q.startsWith("~")) { global = true; q = q.slice(1).trimStart(); go = true; }
      }
      const kws = q.split(/\s+/).filter(Boolean).map((s) => s.toLowerCase());
      if (!kws.length) return Promise.resolve({ placements: [], archives: [] });
      const vk = args?.scope?.viewKey || "quick:all";
      let folderSet = null;
      if (!global && vk.startsWith("folder:")) {
        folderSet = new Set([vk.slice(7)]);
        if (args?.scope?.recursive) {
          let added = true;
          while (added) {
            added = false;
            for (const [fid, f] of Object.entries(FOLDERS)) {
              if (f.parent && folderSet.has(f.parent) && !folderSet.has(fid)) { folderSet.add(fid); added = true; }
            }
          }
        }
      }
      const inScope = (p) => {
        if (global) return true;
        if (folderSet) return folderSet.has(p.folderId);
        if (vk === "quick:uncategorized") return p.folderId === "root";
        return true;
      };
      const placements = [];
      for (const p of PLACEMENTS) {
        const m = META[p.assetId];
        if (!m) continue;
        const texts = [m.name, m.ext, ...m.links.map(pct),
          ...PLACEMENTS.filter((x) => x.assetId === p.assetId).map((x) => x.name),
          pathText(p.folderId)].map((s) => s.toLowerCase());
        if (kws.every((k) => texts.some((t) => t.includes(k))) && inScope(p)) {
          placements.push({ assetId: p.assetId, folderId: p.folderId });
        }
      }
      const archives = [];
      if (global || folderSet) {
        for (const [fid, f] of Object.entries(FOLDERS)) {
          if (f.kind !== "archive") continue;
          if (!global && f.parent !== vk.slice(7)) continue;
          const a = ARCHIVES[fid] || {};
          const texts = [f.name, a.badge?.text || "", ...(a.props || []),
            ...(a.descriptions || []).flatMap((d) => [d.title, d.content]),
            ...(a.links || []).map(pct)].map((s) => s.toLowerCase());
          if (kws.every((k) => texts.some((t) => t.includes(k)))) archives.push(fid);
        }
      }
      return Promise.resolve({ placements, archives });
    }
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


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（vite node 进程脱离 pnpm 壳进程树，端口直杀兜底）"""
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

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            # 激活 panel 悬浮栏搜索输入框（单 panel 场景 = 唯一 .fbs-input）
            async def search_input():
                return await ev("document.querySelector('.fbs-input') !== null")

            # 聚焦驱动（headless Edge focus() 只挂 activeElement 不发 focus 事件——
            # 悬浮栏壳宽 450 档依赖 Vue @focus → focused-change，须合成 FocusEvent 补发）
            async def focus_input():
                await ev("""
                  (() => { const i = document.querySelector('.fbs-input');
                    i.focus(); i.dispatchEvent(new FocusEvent('focus')); return true; })()
                """)

            async def blur_input():
                await ev("""
                  (() => { const i = document.querySelector('.fbs-input');
                    i.blur(); i.dispatchEvent(new FocusEvent('blur')); return true; })()
                """)

            async def set_input(text):
                """先清空（Esc 口径：清空 + 失焦）再设草稿文本，随后轮询等待防抖 + invoke 完成
                （headless 下 setTimeout 防抖触发延迟漂移——固定 sleep 断言不可靠，2026-08-19 复验实证）"""
                await clear_all()
                await ev(f"""
                  (() => {{ const i = document.querySelector('.fbs-input');
                    i.value = {json.dumps(text)}; i.dispatchEvent(new Event('input', {{ bubbles: true }})); return i.value; }})()
                """)
                if text.strip():
                    deadline = time.time() + 5
                    while time.time() < deadline:
                        last = await ev("window.__calls.search_assets.length ? window.__calls.search_assets.at(-1).query : null")
                        if last == text:
                            break
                        await asyncio.sleep(0.2)
                    await asyncio.sleep(1.0)  # hits 写入 + 渲染（headless 偶发慢，0.4s 不够——S8 实测）
                else:
                    await asyncio.sleep(0.5)

            async def card_names():
                return await ev("[...document.querySelectorAll('.asset-card .kc-name')].map((el) => el.textContent.trim())")

            async def archive_names():
                return await ev("[...document.querySelectorAll('.ar-card .ar-name')].map((el) => el.textContent.trim())")

            async def press_key(key, code, vk):
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": key, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": key, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await asyncio.sleep(0.3)

            # Esc 清空草稿并失焦（SearchFloatBar onEscape 口径：输入框 keydown.esc → 业务壳清空 + blur）
            async def clear_all():
                await focus_input()
                await asyncio.sleep(0.1)
                await press_key("Escape", "Escape", 27)
                await asyncio.sleep(0.4)

            # 出现类断言轮询（§18：出现/消失类不断固定 sleep）
            async def wait_cards(n, timeout=5.0):
                cards = []
                deadline = time.time() + timeout
                while time.time() < deadline:
                    cards = await card_names()
                    if cards and len(cards) == n:
                        break
                    await asyncio.sleep(0.2)
                return cards or []

            async def tree_click(fid):
                pos = await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)
                if not pos:
                    return False
                await click(pos["x"], pos["y"])
                await asyncio.sleep(0.4)
                return True

            # 准备：等卡片渲染（默认 quick:all = 6 卡）
            cards = []
            for _ in range(30):
                cards = await card_names()
                if cards and len(cards) == 6:
                    break
                await asyncio.sleep(0.5)
            print("初始卡片（quick:all）:", cards)
            check("S0 全部视图 6 卡渲染", bool(cards) and len(cards) == 6, str(cards))
            if not await search_input():
                print("悬浮栏搜索框未渲染，探针无法执行")
                return 2

            # ── S1：AND 分词（空格 = 分词字符，直输直搜无 chips）──
            await set_input("人物 走路")
            cards = await card_names()
            print("S1 「人物 走路」:", cards)
            check("S1 AND 分词命中两卡", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S2：Esc 清空恢复 ──
            await clear_all()
            cards = await wait_cards(6)
            check("S2 Esc 清空恢复 6 卡", len(cards) == 6, str(cards))

            # ── S3：扩展名大小写不敏感 ──
            await set_input("GIF")
            cards = await card_names()
            check("S3 「GIF」命中 跑步.gif", cards == ["跑步.gif"], str(cards))

            # ── S4：空态 ──
            await set_input("不存在xyz")
            cards = await card_names()
            empty_text = await ev("document.querySelector('.search-empty')?.textContent.trim() ?? null")
            check("S4a 无结果零卡", cards == [], str(cards))
            check("S4b 空态文案「无搜索结果」", empty_text == "无搜索结果", str(empty_text))

            # ── S5：路径命中（后代文件夹名）──
            await set_input("走路参考")
            cards = await card_names()
            check("S5 路径「走路参考」命中 场景-日.png", cards == ["场景-日.png"], str(cards))

            # ── S6：链接 URL decode ──
            await set_input("档案")
            cards = await card_names()
            print("S6 「档案」:", cards)
            check("S6 「档案」命中 分镜01（路径含档案名）+ 参考图（链接 decode）",
                  sorted(cards or []) == sorted(["分镜01.png", "参考图.png"]), str(cards))
            # quick 视图不出档案区（档案命中不参与）
            archs = await archive_names()
            check("S6b quick 视图不出档案区", archs == [], str(archs))

            # ── S7：f1 视图范围（直输 / ~ 手敲全局 / 清空退出）──
            await clear_all()
            await tree_click("f1")
            # 递归显示默认开（ui.showSubfolders）→ 当前目录搜索含后代（跟随递归开关口径）
            await set_input("场景")
            cards = await card_names()
            check("S7a f1 视图直输「场景」命中后代", cards == ["场景-日.png"], str(cards))
            await set_input("~场景")
            cards = await card_names()
            check("S7b f1 视图直输「~场景」全局命中", cards == ["场景-日.png"], str(cards))
            await clear_all()
            cards = await wait_cards(5)
            check("S7c 清空退出搜索态（f1 递归 5 卡）", len(cards) == 5, str(cards))

            # ── S8：草稿 ~ 前缀即时全局 ──
            await set_input("~场景")
            # 时序敏感场景（草稿前缀 + 全局基底切换）：轮询等渲染结果，不赌固定 sleep
            cards = []
            for _ in range(25):
                cards = await card_names()
                if cards == ["场景-日.png"]:
                    break
                await asyncio.sleep(0.2)
            check("S8 草稿「~场景」即时全局命中", cards == ["场景-日.png"], str(cards))

            # ── S9：// 前缀（与直接输入等效）──
            await set_input("//人物")
            cards = await card_names()
            check("S9 直输「//人物」命中两卡", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S10：档案名命中 → 档案区 + 档案内素材（递归开：fa 在范围集内）──
            await set_input("分镜")
            cards = await card_names()
            archs = await archive_names()
            print("S10 素材:", cards, "档案:", archs)
            check("S10a 档案内素材命中（路径含档案名）", cards == ["分镜01.png"], str(cards))
            check("S10b 档案区显示「分镜档案」", archs == ["分镜档案"], str(archs))

            # ── S11：徽标命中 ──
            await set_input("进行中")
            archs = await archive_names()
            cards = await card_names()
            check("S11 徽标「进行中」命中档案区", archs == ["分镜档案"] and cards == [], f"cards={cards} archs={archs}")

            # ── S12：分组平铺（查看方式菜单关递归——递归口径恒不分组，3.5 复验定案；
            #    旧工具栏 .recursive-cb 复选框已随 UI 重构移除，开关在「查看方式」菜单 AppSwitch）──
            await clear_all()
            # 查看方式钮 = 工具栏按钮组第 4 个（添加链接 / 插件 / 分组管理 / 查看方式；
            # IconButton 的 title 是 v-tip 不落 title 属性，按 DOM 序取）
            await ev("""
              (() => { const b = [...document.querySelectorAll('.panel-toolbar .btn-group .ic-btn')][3];
                if (!b) return false; b.click(); return true; })()
            """)
            menu_ok = False
            for _ in range(20):
                menu_ok = await ev("""
                  [...document.querySelectorAll('.app-menu .label')]
                    .some((el) => el.textContent.trim() === '显示子文件夹内容')
                """)
                if menu_ok:
                    break
                await asyncio.sleep(0.2)
            check("S12a 查看方式菜单打开（含「显示子文件夹内容」开关行）", menu_ok is True, str(menu_ok))
            await ev("""
              (() => { const row = [...document.querySelectorAll('.app-menu .row')]
                  .find((r) => r.querySelector('.label')?.textContent.trim() === '显示子文件夹内容');
                const sw = row?.querySelector('.app-switch'); if (!sw) return false; sw.click(); return true; })()
            """)
            sw_off = False
            for _ in range(20):
                sw_off = await ev("""
                  (() => { const row = [...document.querySelectorAll('.app-menu .row')]
                      .find((r) => r.querySelector('.label')?.textContent.trim() === '显示子文件夹内容');
                    const sw = row?.querySelector('.app-switch'); return sw ? !sw.classList.contains('on') : null; })()
                """)
                if sw_off:
                    break
                await asyncio.sleep(0.2)
            check("S12b 递归开关关闭", sw_off is True, str(sw_off))
            # 关菜单（AppMenu window capture 监听 Esc；菜单内输入框豁免不涉及——悬浮栏 input 不在菜单内）
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); true")
            grp = 0
            for _ in range(20):
                grp = await ev("document.querySelectorAll('.group-header').length")
                if grp and grp >= 1:
                    break
                await asyncio.sleep(0.2)
            check("S12c 非搜索态分组标题存在（f1 配 png 组）", bool(grp) and grp >= 1, f"group-headers={grp}")
            await set_input("人物")
            grp2 = 1
            for _ in range(20):
                grp2 = await ev("document.querySelectorAll('.group-header').length")
                if grp2 == 0:
                    break
                await asyncio.sleep(0.2)
            cards = await card_names()
            check("S12d 搜索态组标题消失（平铺）", grp2 == 0, f"group-headers={grp2}")
            check("S12e 搜索态命中保持", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))
            await clear_all()
            grp3 = 0
            for _ in range(20):
                grp3 = await ev("document.querySelectorAll('.group-header').length")
                if grp3 and grp3 >= 1:
                    break
                await asyncio.sleep(0.2)
            check("S12f 清空后分组恢复", bool(grp3) and grp3 >= 1, f"group-headers={grp3}")

            # ── S13：Ctrl+F 聚焦 ──
            await blur_input()
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "f", "code": "KeyF", "windowsVirtualKeyCode": 70, "modifiers": 2}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "f", "code": "KeyF", "windowsVirtualKeyCode": 70, "modifiers": 2}, session=sess)
            await asyncio.sleep(0.3)
            focused = await ev("document.activeElement === document.querySelector('.fbs-input')")
            check("S13 Ctrl+F 聚焦悬浮栏输入框", focused is True, str(focused))

            # ── S14：空文件夹视图 + ~ 全局搜命中视口外素材（2026-08-19 复验修复回归钉：
            #    原前端过滤基底恒取本视口，空基底把全库命中键滤光）──
            await tree_click("f3")
            cards = ["__init__"]
            for _ in range(20):
                cards = await card_names()
                if cards == []:
                    break
                await asyncio.sleep(0.2)
            check("S14a 空文件夹视口零卡", cards == [], str(cards))
            await set_input("~人物")
            cards = await card_names()
            check("S14b 空文件夹 ~ 全局命中视口外素材", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S17：悬浮栏宽度（壳 = 唯一宽度动画源：250 常态 / 450 聚焦定值档；300ms 动画轮询到位）──
            await clear_all()  # 空草稿 + 失焦回常态

            async def shell_width():
                return await ev("document.querySelector('.fb-shell')?.offsetWidth ?? null")

            w0 = None
            for _ in range(25):
                w0 = await shell_width()
                if w0 == 250:
                    break
                await asyncio.sleep(0.2)
            check("S17a 常态壳宽 250", w0 == 250, f"w={w0}")
            await focus_input()
            w1 = None
            for _ in range(25):
                w1 = await shell_width()
                if w1 == 450:
                    break
                await asyncio.sleep(0.2)
            check("S17b 聚焦壳宽 450（合成 FocusEvent 驱动）", w1 == 450, f"w={w1}")
            await blur_input()
            w2 = None
            for _ in range(25):
                w2 = await shell_width()
                if w2 == 250:
                    break
                await asyncio.sleep(0.2)
            check("S17c 失焦壳宽回 250", w2 == 250, f"w={w2}")

            # ── S18：筛选展开（点亮不接业务：.on 本地切换，不触发搜索）──
            await ev("document.querySelector('.fbs-ic.filter')?.click(); true")
            chips_n = 0
            for _ in range(20):
                chips_n = await ev("document.querySelectorAll('.fbs-chip').length")
                if chips_n == 10:
                    break
                await asyncio.sleep(0.2)
            check("S18a 点筛选钮展开 chip×10", chips_n == 10, f"n={chips_n}")
            chip_names = await ev("[...document.querySelectorAll('.fbs-chip')].map((el) => el.textContent.trim())")
            check("S18b chip 文案齐（稿 10 项）",
                  chip_names == ["颜色", "时长", "标签", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"],
                  str(chip_names))
            n0 = await ev("window.__calls.search_assets.length")
            await ev("document.querySelectorAll('.fbs-chip')[0]?.click(); true")
            on_n = 0
            for _ in range(15):
                on_n = await ev("document.querySelectorAll('.fbs-chip.on').length")
                if on_n == 1:
                    break
                await asyncio.sleep(0.2)
            n1 = await ev("window.__calls.search_assets.length")
            check("S18c chip 点击点亮 .on（本地切换）", on_n == 1, f"on={on_n}")
            check("S18d chip 点击不触发 search_assets", n1 == n0, f"before={n0} after={n1}")
            await ev("document.querySelectorAll('.fbs-chip')[0]?.click(); true")
            off_n = 1
            for _ in range(15):
                off_n = await ev("document.querySelectorAll('.fbs-chip.on').length")
                if off_n == 0:
                    break
                await asyncio.sleep(0.2)
            n2 = await ev("window.__calls.search_assets.length")
            check("S18e 再点熄灭 .on（切换）且仍不触发搜索", off_n == 0 and n2 == n0, f"on={off_n} before={n0} after={n2}")
            await ev("document.querySelector('.fbs-ic.collapse')?.click(); true")
            chips_n2 = 1
            for _ in range(20):
                chips_n2 = await ev("document.querySelectorAll('.fbs-chip').length")
                if chips_n2 == 0:
                    break
                await asyncio.sleep(0.2)
            filter_back = await ev("document.querySelector('.fbs-ic.filter') !== null")
            check("S18f 点收起钮折叠回搜索态", chips_n2 == 0 and filter_back is True, f"chips={chips_n2} filterBtn={filter_back}")

            # 截图留档：搜索态悬浮栏（f1 视图「人物」命中两卡 + 聚焦 450 宽，clip 栏体）
            await clear_all()
            await tree_click("f1")
            await set_input("人物")
            await focus_input()
            for _ in range(25):
                if await shell_width() == 450:
                    break
                await asyncio.sleep(0.2)
            clip = await ev("""
              (() => { const b = document.querySelector('.fb-shell').getBoundingClientRect();
                const m = 16;
                return { x: Math.max(0, b.left - m), y: Math.max(0, b.top - m),
                  width: b.width + m * 2, height: b.height + m * 2 }; })()
            """)
            shot = await send(ws, "Page.captureScreenshot",
                              {"format": "png", "clip": {**clip, "scale": 1}}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图:", SHOT)

            # ── S19：聚焦态点筛选直接展开（2026-09-09 修复回归钉：原 blur 抢先触发 450→250 收缩动画，
            #    筛选钮随壳边移出指针下方致 click 落空；现栏内钮一律 mousedown.prevent 阻失焦）。
            #    必须真鼠标链路（mousedown→mouseup→click）——js .click() 不经过失焦路径，复现不了原 bug ──
            frect = await ev("(() => { const b = document.querySelector('.fbs-ic.filter').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": frect["x"], "y": frect["y"], "button": "left", "clickCount": 1}, session=sess)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": frect["x"], "y": frect["y"], "button": "left", "clickCount": 1}, session=sess)
            chips19 = 0
            for _ in range(20):
                chips19 = await ev("document.querySelectorAll('.fbs-chip').length")
                if chips19 == 10:
                    break
                await asyncio.sleep(0.2)
            still_focused = await ev("document.activeElement === document.querySelector('.fbs-input')")
            check("S19a 聚焦态点筛选展开 chip×10", chips19 == 10, f"n={chips19}")
            check("S19b 展开后输入框保持聚焦（mousedown.prevent 阻失焦）", still_focused is True, str(still_focused))
            w19 = None
            for _ in range(25):
                w19 = await shell_width()
                if w19 and w19 > 450:
                    break
                await asyncio.sleep(0.2)
            check("S19c 壳宽展开实测（>450，探针实测档）", bool(w19 and w19 > 450), f"w={w19}")

            # ── S20：挤压裁剪 + 滚轮横滚（2026-09-09 回归钉：侧栏挤压中间面板时筛选组壳内裁剪、
            #    收起钮钉右端不被裁走、筛选组滚轮横滚；壳 max-width 100% 不跑底色外）。
            #    注：Emulation 收窄视口对 dockview 无效（panel 宽由 dock split 钉，panelW 不随视口变），
            #    故直接钉 .content-area inline 宽 = 侧栏挤压后的窄态等价物（验证目标 = 壳在窄容器内的行为）──
            await ev("""(() => { const ca = document.querySelector('.content-area');
              ca.style.width = '500px'; ca.style.flex = 'none'; return true; })()""")
            await asyncio.sleep(0.6)  # 等布局重排 + 壳宽收缩到位
            s20 = await ev("""(() => {
              const shell = document.querySelector('.fb-shell');
              const panel = document.querySelector('.content-area');
              const col = document.querySelector('.fbs-filters');
              const collapse = document.querySelector('.fbs-ic.collapse');
              if (!shell || !panel || !col || !collapse) return null;
              const sr = shell.getBoundingClientRect(); const pr = panel.getBoundingClientRect();
              const cr = collapse.getBoundingClientRect();
              return { shellW: shell.offsetWidth, panelW: panel.offsetWidth,
                       inPanelX: sr.left >= pr.left + 11 && sr.right <= pr.right - 11,
                       collapseIn: cr.left >= sr.left - 1 && cr.right <= sr.right + 1,
                       overflowX: col.scrollWidth > col.clientWidth };
            })()""")
            print("  [S20] 挤压态:", json.dumps(s20, ensure_ascii=False))
            check("S20a 壳钳制在面板内 + 两侧 12 呼吸（不跑底色外）", bool(s20 and s20["inPanelX"] and s20["shellW"] <= s20["panelW"] - 24), json.dumps(s20))
            check("S20b 收起钮钉在壳内（不被裁走）", bool(s20 and s20["collapseIn"]), json.dumps(s20))
            check("S20c 筛选组横向溢出（可横滚前提）", bool(s20 and s20["overflowX"]), json.dumps(s20))
            # 滚轮横滚：合成 wheel（deltaY → scrollLeft）
            await ev("""(() => { const c = document.querySelector('.fbs-filters');
              c.dispatchEvent(new WheelEvent('wheel', { deltaY: 240, bubbles: true, cancelable: true })); return true; })()""")
            await asyncio.sleep(0.2)
            sl = await ev("document.querySelector('.fbs-filters').scrollLeft")
            check("S20d 滚轮横向滚动筛选组", bool(sl and sl > 0), f"scrollLeft={sl}")
            # 还原布局 + 收起（不留脏状态）
            await ev("""(() => { const ca = document.querySelector('.content-area');
              ca.style.width = ''; ca.style.flex = ''; return true; })()""")
            await ev("document.querySelector('.fbs-ic.collapse')?.click(); true")

            print()
            if fails:
                print(f"失败 {len(fails)} 项：")
                for f_ in fails:
                    print(" -", f_)
                return 1
            print("全部 PASS")
            return 0
    finally:
        # headless Edge 整树杀（§18：proc.terminate()/kill() 只杀父进程，残留占调试端口）
        subprocess.run(["taskkill", "/PID", str(edge.pid), "/F", "/T"], capture_output=True)
        if spawned:
            spawned.kill()
            kill_listening_port(5373)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
