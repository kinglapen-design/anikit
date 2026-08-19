# CDP 探针：3.6 一轮 · 分词搜索（2026-08-19）
#   S1 全部视图输入「人物 走路」→ AND 分词命中两卡（名称子串）
#   S2 清空 → 恢复全部 6 卡（搜索态退出）
#   S3 扩展名大小写不敏感：「GIF」命中 跑步.gif
#   S4 无结果 → 中央空态文案「无搜索结果」
#   S5 路径命中：「走路参考」命中后代文件夹 f2 内的 场景-日.png
#   S6 链接 URL decode：「档案」命中 参考图.png（links q=%E6%A1%A3%E6%A1%88）
#   S7 f1 视图 chip =「当前目录」；非递归搜「场景」→ 空（后代夹不命中）；
#      chip 点击切全局 → 命中（chip 文案变「全局」）
#   S8 ~ 前缀强制全局：f1 视图输「~场景」→ 命中（chip 显示「全局」）
#   S9 // 前缀显式分词：「//人物」正常命中
#   S10 档案命中：f1 视图搜「分镜」→ 档案区显示「分镜档案」卡片 + 档案内素材卡
#   S11 徽标命中：f1 视图搜「进行中」→ 档案区显示「分镜档案」
#   S12 分组平铺：f1（配扩展名分组）搜索态组标题消失；清空恢复分组
#   S13 Ctrl+F 聚焦搜索框
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-search-probe.py
import asyncio
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
      list_archives: [{ folderId: "fa", badge: ARCHIVES.fa.badge, avatar: null,
        props: ARCHIVES.fa.props, descriptions: ARCHIVES.fa.descriptions, links: [], createdAt: "" }],
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

            # 激活 panel 搜索框（单 panel 场景 = 唯一 .search-bar）
            async def search_input():
                return await ev("document.querySelector('.search-bar input') !== null")

            async def set_input(text):
                """先清空（chips + 草稿，Esc 口径）再设草稿文本，随后轮询等待防抖 + invoke 完成
                （headless 下 setTimeout 防抖触发延迟漂移——固定 sleep 断言不可靠，2026-08-19 复验实证）"""
                await clear_all()
                await ev(f"""
                  (() => {{ const i = document.querySelector('.search-bar input');
                    i.focus(); i.value = {json.dumps(text)}; i.dispatchEvent(new Event('input', {{ bubbles: true }})); return i.value; }})()
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

            async def clear_draft():
                """只清草稿（不动 chips）"""
                await ev("""
                  (() => { const i = document.querySelector('.search-bar input');
                    i.focus(); i.value = ''; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()
                """)
                await asyncio.sleep(0.5)

            async def card_names():
                return await ev("[...document.querySelectorAll('.asset-card .title')].map((el) => el.textContent.trim())")

            async def archive_names():
                return await ev("[...document.querySelectorAll('.archive-card .archive-name')].map((el) => el.textContent.trim())")

            # tag 化（2026-08-19 二轮）：chips 区读 .chip 文本；范围 = `~` tag（无按钮）
            async def chips_texts():
                return await ev("[...document.querySelectorAll('.search-bar .chip .chip-text')].map((el) => el.textContent.trim())")

            async def kbd_visible():
                return await ev("document.querySelector('.search-bar .kbd-group') !== null")

            async def press_key(key, code, vk):
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": key, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": key, "code": code, "windowsVirtualKeyCode": vk}, session=sess)
                await asyncio.sleep(0.3)

            async def press_space():
                await press_key(" ", "Space", 32)

            async def press_backspace():
                await press_key("Backspace", "Backspace", 8)

            # Esc 清空全部（chips + 草稿）并失焦（SearchBar 口径）
            async def clear_all():
                await ev("(() => { const i = document.querySelector('.search-bar input'); i.focus(); return true; })()")
                await asyncio.sleep(0.1)
                await press_key("Escape", "Escape", 27)
                await asyncio.sleep(0.4)

            # 追加草稿（不清空现有内容）
            async def append_input(text, wait_query=None):
                await ev(f"""
                  (() => {{ const i = document.querySelector('.search-bar input');
                    i.focus(); i.value += {json.dumps(text)}; i.dispatchEvent(new Event('input', {{ bubbles: true }})); return i.value; }})()
                """)
                if wait_query is not None:
                    deadline = time.time() + 5
                    while time.time() < deadline:
                        last = await ev("window.__calls.search_assets.length ? window.__calls.search_assets.at(-1).query : null")
                        if last == wait_query:
                            break
                        await asyncio.sleep(0.2)
                await asyncio.sleep(0.5)

            async def tree_click(fid):
                pos = await ev(f"""
                  (() => {{
                    const el = document.querySelector('.tree-item[data-folder-id="{fid}"]');
                    if (!el) return null;
                    const b = el.getBoundingClientRect();
                    return {{ x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }};
                  }})()
                """)
                await click(pos["x"], pos["y"])
                await asyncio.sleep(0.4)

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
                print("搜索框未渲染，探针无法执行")
                return 2

            # ── S1：AND 分词（草稿即时参与）──
            await set_input("人物 走路")
            cards = await card_names()
            print("S1 「人物 走路」:", cards)
            check("S1 AND 分词命中两卡", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S1b：空格提交成 chip（tag 化）──
            await clear_all()
            await append_input("人物", wait_query="人物")
            await press_space()
            chips = await chips_texts()
            check("S1b-1 空格提交「人物」chip", chips == ["人物"], str(chips))
            await append_input("走路")
            cards = await card_names()
            check("S1b-2 chip+草稿 AND 命中两卡", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))
            await press_space()
            chips = await chips_texts()
            check("S1b-3 第二词提交成 chip", chips == ["人物", "走路"], str(chips))
            cards = await card_names()
            check("S1b-4 两 chips 命中保持", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S2：Esc 清空恢复 ──
            await clear_all()
            cards = await card_names()
            check("S2 Esc 清空恢复 6 卡", bool(cards) and len(cards) == 6, str(cards))

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

            # ── S7：f1 视图 tag 化范围（~ tag 生成 / 生效 / 删除）──
            await clear_all()
            await tree_click("f1")
            chips = await chips_texts()
            check("S7a f1 视图无前缀 tag", chips == [], str(chips))
            # 递归显示默认开（ui.showSubfolders）→ 当前目录搜索含后代（跟随递归开关口径）
            await set_input("场景")
            cards = await card_names()
            check("S7b 递归开：当前目录搜「场景」命中后代", cards == ["场景-日.png"], str(cards))
            # 前缀剥落：改输「~场景」空格 →「~ 全局搜索」tag +「场景」chip
            await set_input("~场景")
            await press_space()
            chips = await chips_texts()
            check("S7c 「~场景」空格剥落 = 全局 tag + 关键字 chip", chips == ["~ 全局搜索", "场景"], str(chips))
            cards = await card_names()
            check("S7d ~ tag 全局搜索保持命中", cards == ["场景-日.png"], str(cards))
            # 空草稿 Backspace 删尾 chip：先删「场景」（回当前目录+无 tag），再删「~ 全局搜索」
            await press_backspace()
            chips = await chips_texts()
            check("S7e Backspace 删尾 chip（场景）", chips == ["~ 全局搜索"], str(chips))
            await press_backspace()
            chips = await chips_texts()
            check("S7f 再删「~ 全局搜索」回当前范围", chips == [], str(chips))
            cards = await card_names()
            check("S7g 无 tag 无关键字 = 退出搜索态（f1 递归 5 卡）", bool(cards) and len(cards) == 5, str(cards))

            # ── S8：草稿 ~ 前缀即时全局（未提交也生效）──
            await set_input("~场景")
            # 时序敏感场景（草稿前缀 + 全局基底切换）：轮询等渲染结果，不赌固定 sleep
            cards = []
            for _ in range(25):
                cards = await card_names()
                if cards == ["场景-日.png"]:
                    break
                await asyncio.sleep(0.2)
            check("S8 草稿「~场景」即时全局命中", cards == ["场景-日.png"], str(cards))

            # ── S9：// 前缀（草稿 + tag 两态）──
            await set_input("//人物")
            cards = await card_names()
            check("S9a 草稿「//人物」显式分词命中两卡", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))
            await press_space()
            chips = await chips_texts()
            check("S9b 「//人物」空格剥落 = 搜索 tag + 关键字 chip", chips == ["// 搜索", "人物"], str(chips))
            cards = await card_names()
            check("S9c 命中保持", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

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

            # ── S12：分组平铺（先关递归——递归口径恒不分组，3.5 复验定案）──
            await clear_all()
            cb_pos = await ev("""
              (() => { const el = document.querySelector('.recursive-cb');
                if (!el) return null;
                const b = el.getBoundingClientRect();
                return { x: Math.round(b.left + b.width / 2), y: Math.round(b.top + b.height / 2) }; })()
            """)
            if not cb_pos:
                check("S12a 递归复选框存在", False, "未找到 .recursive-cb")
            else:
                await click(cb_pos["x"], cb_pos["y"])  # 关递归
                await asyncio.sleep(0.6)
            grp = await ev("document.querySelectorAll('.group-header').length")
            check("S12a 非搜索态分组标题存在（f1 配 png 组）", bool(grp) and grp >= 1, f"group-headers={grp}")
            await set_input("人物")
            grp2 = await ev("document.querySelectorAll('.group-header').length")
            cards = await card_names()
            check("S12b 搜索态组标题消失（平铺）", grp2 == 0, f"group-headers={grp2}")
            check("S12c 搜索态命中保持", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))
            await clear_all()
            grp3 = await ev("document.querySelectorAll('.group-header').length")
            check("S12d 清空后分组恢复", bool(grp3) and grp3 >= 1, f"group-headers={grp3}")

            # ── S13：Ctrl+F 聚焦 ──
            await ev("document.activeElement?.blur(); true")
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "f", "code": "KeyF", "windowsVirtualKeyCode": 70, "modifiers": 2}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "f", "code": "KeyF", "windowsVirtualKeyCode": 70, "modifiers": 2}, session=sess)
            await asyncio.sleep(0.3)
            focused = await ev("document.activeElement === document.querySelector('.search-bar input')")
            check("S13 Ctrl+F 聚焦搜索框", focused is True, str(focused))

            # ── S14：空文件夹视图 + ~ 全局搜命中视口外素材（2026-08-19 复验修复回归钉：
            #    原前端过滤基底恒取本视口，空基底把全库命中键滤光）──
            await tree_click("f3")
            cards = await card_names()
            check("S14a 空文件夹视口零卡", cards == [], str(cards))
            await set_input("~人物")
            cards = await card_names()
            check("S14b 空文件夹 ~ 全局命中视口外素材", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))
            await press_space()
            chips = await chips_texts()
            check("S14c 空格剥落 = 全局 tag + 关键字 chip", chips == ["~ 全局搜索", "人物"], str(chips))

            # ── S15：/ 筛选占位 tag（3.6 二轮点亮；不参与搜索）──
            await clear_all()
            await tree_click("f1")
            await set_input("/")
            await press_space()
            chips = await chips_texts()
            check("S15a 「/」空格生成「/ 筛选」占位 tag", chips == ["/ 筛选"], str(chips))
            await append_input("人物", wait_query="人物")
            cards = await card_names()
            check("S15b 占位 tag 不参与搜索（人物正常命中）", sorted(cards or []) == sorted(["人物-开心-走路.png", "人物-伤心-走路.png"]), str(cards))

            # ── S16：kbd 徽章 = 未聚焦且无内容时的提示（聚焦 / 有内容隐藏）──
            await clear_all()  # 含 blur
            vis0 = await kbd_visible()
            check("S16a 未聚焦且无内容：徽章组显示", vis0 is True, str(vis0))
            await ev("document.querySelector('.search-bar input').focus(); true")
            await asyncio.sleep(0.2)
            vis1 = await kbd_visible()
            check("S16b 聚焦后徽章隐藏", vis1 is False, str(vis1))
            await append_input("人")
            await ev("document.activeElement?.blur(); true")
            await asyncio.sleep(0.5)  # blur 触发 commit（失焦提交草稿成 chip）
            vis2 = await kbd_visible()
            chips = await chips_texts()
            check("S16c 失焦提交草稿成 chip 且徽章保持隐藏（有内容）", vis2 is False and chips == ["人"], f"vis={vis2} chips={chips}")

            await clear_all()
            await tree_click("f1")  # 回 f1 供截图（递归此时已关——先开回）

            # 截图留档（chip 形态目验：「~人物」剥落两 chip + 档案区「分镜」命中——分两次截）
            await set_input("~人物")
            await press_space()
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            import base64
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图:", SHOT)

            print()
            if fails:
                print(f"失败 {len(fails)} 项：")
                for f_ in fails:
                    print(" -", f_)
                return 1
            print("全部 PASS")
            return 0
    finally:
        edge.kill()
        if spawned:
            spawned.kill()
            kill_listening_port(5373)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
