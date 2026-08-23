# CDP 实测：6.1 一轮 · 自定义数据（属性 / 描述 / 档案封面设置，2026-08-23）
# 桩带可变状态（folders/placements/archives 三表 + 新写命令真改数据，refresh 后 UI 跟变）：
# S1 文件属性弹窗（32 稿）：壳 482 / 标题栏 54 / 按钮主左次右 88；拥有 chips 初值渲染；
#    输入新属性回车加入；点池 chip 加入（pop 过冲动画类）；池 × → 警告二次确认 →
#    delete_props_globally 调用 + 池与拥有同步移除；确认 → set_asset_props + 详情 chips 刷新
# S2 档案属性弹窗：档案卡选中 → 添加属性 → set_archive_props
# S3 编辑描述弹窗（31 稿）：初值行 / 添加描述追加 / 首行禁上移末行禁下移 / 红删 / 下移交换序 →
#    set_asset_descriptions 参数序断言
# S4 描述截断：内容超 3 行出「展开」标签色小字钮 → 展开全显 → 收起
# S5 封面设置（43 稿，仅档案）：常态只有封面图（cover-btn）→ 点击展开组（218×168 框 / 滑块禁用 /
#    更换封面钮 / 徽标输入框初值 / 色块回显）→ 颜色下拉 9 格 → 选色 + 改字 → 确认
#    set_archive_cover_settings（cover 键缺失 = 不动）→ 退回常态 + 徽标 chip 更新
# S6 取景态：桩 plugin:dialog|open 回测试图 → 更换封面菜单「从本地磁盘中选择」→ cs-view 取景 +
#    滑块解禁 → 拖动 20px → 确认 cover { sourcePath, crop } 断言
# S7 档案卡封面 = coverPath 优先（6.1：头像即档案卡片封面）
# S8 取消零写入
# S9 数据区块独立设置（文件 / 档案各一套）
# S10 颜色 bar 真色（6.1 颜色解析）：A1 三色块按序渲染；无颜色资产（fla）= 7 占位色块恒显
# 截图留档 tools/style-preview/：props-dialog / desc-dialog / cover-settings / cover-framing。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9361
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__calls = { set_asset_props: [], set_archive_props: [], set_asset_descriptions: [], set_archive_descriptions: [], delete_props_globally: [], set_archive_cover_settings: [], save_ui_settings: [] };
window.__cbs = {};
window.__cbId = 0;
(function () {
  const STILL = "__STILL__";
  const DB = {
    blocks: [{ id: "dir", name: "目录", icon: null }],
    folders: [
      { id: "F-DIR", name: "素材", parent: null, block: "dir", kind: null },
      { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
      { id: "F-ARC", name: "设定集", parent: "F-DIR", kind: "archive" },
    ],
    assets: {
      A1: { id: "A1", originName: "主角-立绘.webp", ext: "webp", size: 204800, width: 800, height: 600, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: ["主角"], descriptions: [{ title: "身份", content: "沙雕动画的御用主角，第一行内容第二行内容第三行内容第四行内容——超三行截断测试用的长文本，继续加长确保超过三行显示高度，再加一点。" }], colors: ["#C81E1E", "#1E1EC8", "#E8E8E8"], links: [], note: "" },
      A2: { id: "A2", originName: "场景源文件.fla", ext: "fla", size: 3145728, width: null, height: null, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], links: [], note: "" },
    },
    placements: [
      { assetId: "A1", folderId: "F-SUB", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A1", folderId: "F-ARC", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A2", folderId: "F-SUB", name: "场景源文件.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ],
    archives: {
      "F-ARC": { folderId: "F-ARC", badge: { text: "设定", color: "#B38427" }, avatar: null, props: ["奇幻"], descriptions: [{ title: "来源", content: "官方设定集" }], links: [], createdAt: "2026-08-15T09:00:00+08:00", coverPath: STILL },
    },
  };
  const byId = (id) => DB.folders.find((f) => f.id === id);
  const childrenOf = (id) => DB.folders.filter((f) => f.parent === id);
  const countOf = (id) => {
    let n = DB.placements.filter((p) => p.folderId === id).length;
    for (const c of childrenOf(id)) n += countOf(c.id);
    return n;
  };
  const nodeOf = (f) => ({
    id: f.id, name: f.name, count: countOf(f.id), kind: f.kind,
    block: f.parent === null ? (f.block ?? "dir") : null, icon: null,
    children: childrenOf(f.id).map(nodeOf),
  });
  const tree = () => ({ blocks: DB.blocks, nodes: childrenOf(null).map(nodeOf) });
  const archiveView = (id) => {
    const rec = DB.archives[id];
    const inFolder = DB.placements.filter((p) => p.folderId === id);
    const uniq = new Set(inFolder.map((p) => p.assetId));
    const totalSize = [...uniq].reduce((s, a) => s + (DB.assets[a]?.size ?? 0), 0);
    const base = rec ?? { folderId: id, badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", coverPath: null };
    return { ...base, fileCount: inFolder.length, totalSize };
  };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    convertFileSrc: (p) => encodeURI("/@fs/" + p),
    transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
    invoke: (cmd, args) => {
      const R = {
        get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 1, folderCount: 3 },
        get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
        get_ui_settings: {},
        list_placements: () => DB.placements,
        list_folder_tree: () => tree(),
        list_archives: () => Object.values(DB.archives),
        list_recycle: [],
        list_recent_assets: [], list_inbox_snapshot: [],
        get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
        list_space_libraries: ["动画资产"],
        list_recent_libraries: [],
        list_prop_pool: () => ["主角", "二维", "拔剑", "奇幻"],
      };
      if (cmd === "plugin:dialog|open") return Promise.resolve(STILL); // 从本地磁盘中选择
      if (cmd === "save_ui_settings") { window.__calls.save_ui_settings.push(JSON.parse(JSON.stringify(args.settings))); return Promise.resolve(null); }
      if (cmd === "get_archive_data") return Promise.resolve(archiveView(args.folderId));
      if (cmd === "get_asset_detail") {
        const meta = DB.assets[args.assetId];
        if (!meta) return Promise.reject("资源不存在，可能已被删除");
        return Promise.resolve({
          assetId: meta.id, originName: meta.originName, ext: meta.ext, size: meta.size,
          width: meta.width, height: meta.height, duration: meta.duration,
          importedAt: meta.importedAt, modifiedAt: meta.modifiedAt,
          tags: meta.tags, descriptions: meta.descriptions, colors: meta.colors ?? [], links: meta.links, note: meta.note,
          placements: [{ folderId: "F-SUB", blockName: "目录", path: ["素材", "子文件夹"], name: meta.originName, origin: true }],
          archives: [],
        });
      }
      if (cmd === "set_asset_props") { window.__calls.set_asset_props.push(JSON.parse(JSON.stringify(args))); DB.assets[args.assetId].tags = args.props; return Promise.resolve(null); }
      if (cmd === "set_asset_descriptions") { window.__calls.set_asset_descriptions.push(JSON.parse(JSON.stringify(args))); DB.assets[args.assetId].descriptions = args.descriptions; return Promise.resolve(null); }
      if (cmd === "set_archive_props") { window.__calls.set_archive_props.push(JSON.parse(JSON.stringify(args))); DB.archives[args.folderId].props = args.props; return Promise.resolve(null); }
      if (cmd === "set_archive_descriptions") { window.__calls.set_archive_descriptions.push(JSON.parse(JSON.stringify(args))); DB.archives[args.folderId].descriptions = args.descriptions; return Promise.resolve(null); }
      if (cmd === "delete_props_globally") {
        window.__calls.delete_props_globally.push(JSON.parse(JSON.stringify(args)));
        let n = 0;
        for (const w of args.words) {
          for (const a of Object.values(DB.assets)) if (a.tags.includes(w)) { a.tags = a.tags.filter((t) => t !== w); n++; }
          for (const r of Object.values(DB.archives)) if (r.props.includes(w)) { r.props = r.props.filter((t) => t !== w); n++; }
        }
        return Promise.resolve(n);
      }
      if (cmd === "set_archive_cover_settings") {
        window.__calls.set_archive_cover_settings.push(JSON.parse(JSON.stringify(args)));
        const rec = DB.archives[args.folderId];
        if (args.badge !== undefined) rec.badge = args.badge;
        if (args.cover) { rec.avatar = { file: "baked.webp", crop: args.cover.crop }; rec.coverPath = STILL; }
        return Promise.resolve(null);
      }
      const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
      return Promise.resolve(typeof v === "function" ? v() : v);
    },
  };
})();
""".replace("__STILL__", STILL)

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
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    dev_proc = None
    if dev_server_up():
        print("5373 已在监听，复用现有 dev server")
    else:
        print("5373 未监听，启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
        if not dev_server_up():
            print("dev server 启动超时")
            return 2
        print("dev server 就绪")

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
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "buttons": 1 if button == "left" else 2, "x": x, "y": y, "clickCount": clicks}, session=sess)

            async def rect_of(expr):
                return await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")

            async def click(expr):
                r = await rect_of(expr)
                if not r:
                    print("!! 元素不存在:", expr[:80])
                    return False
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # —— 进 F-DIR 视口（档案区出设定集卡）并展开树 ——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.6)

            # —— S7 档案卡封面 = coverPath 优先 ——
            card_cover = await ev("""(() => {
              const img = document.querySelector('.archive-card[data-folder-id="F-ARC"] .thumb img');
              return img ? img.getAttribute('src') : null;
            })()""")
            check("S7 档案卡封面 = coverPath 优先（自定义封面图）",
                  bool(card_cover and "still.webp" in card_cover), str(card_cover))

            # —— S1 文件属性弹窗（32 稿）：进 F-SUB 视口点素材卡 ——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-SUB\"] .body')")
            await asyncio.sleep(0.6)
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(0.8)
            add_btn = await ev("""(() => {
              const b = document.querySelector('.detail-panel .section-custom .chip-add');
              return b ? { disabled: b.disabled } : null;
            })()""")
            check("S1 添加属性钮已点亮（不再 disabled）", bool(add_btn and add_btn["disabled"] is False), json.dumps(add_btn))
            await click("document.querySelector('.detail-panel .section-custom .chip-add')")
            await asyncio.sleep(0.8)
            dlg = await ev("""(() => {
              const m = document.querySelector('.props-dialog .app-modal');
              if (!m) return null;
              const b = m.getBoundingClientRect();
              const tb = m.querySelector('.titlebar').getBoundingClientRect();
              const btns = [...m.querySelectorAll('.actions .app-btn')].map((x) => x.textContent.trim());
              const owned = [...m.querySelectorAll('.pd-owned .app-tag')].map((t) => t.textContent.trim());
              const pool = [...m.querySelectorAll('.pool-chips .app-tag')].map((t) => t.textContent.trim());
              return { w: Math.round(b.width), titleH: Math.round(tb.height), btns, owned, pool,
                       title: m.querySelector('.titlebar .title')?.textContent };
            })()""")
            print("属性弹窗:", json.dumps(dlg, ensure_ascii=False))
            check("S1 弹窗壳 482 宽 / 标题栏 54 / 标题常态「属性标签」/ 按钮序 确认·取消",
                  bool(dlg and dlg["w"] == 482 and dlg["titleH"] == 54 and dlg["title"] == "属性标签" and dlg["btns"] == ["确认", "取消"]),
                  json.dumps(dlg and {"w": dlg["w"], "h": dlg["titleH"], "btns": dlg["btns"]}, ensure_ascii=False))
            check("S1 拥有 chips 初值（主角）+ 池四词",
                  bool(dlg and dlg["owned"] == ["主角"] and dlg["pool"] == ["主角", "二维", "拔剑", "奇幻"]),
                  json.dumps(dlg and {"o": dlg["owned"], "p": dlg["pool"]}, ensure_ascii=False))
            # 输入新属性回车加入
            await ev("""(() => {
              const el = document.querySelector('.props-dialog .pd-new');
              el.value = '御剑';
              el.dispatchEvent(new Event('input', { bubbles: true }));
              el.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
              return true;
            })()""")
            await asyncio.sleep(0.4)
            owned2 = await ev("[...document.querySelectorAll('.props-dialog .pd-owned .app-tag')].map((t) => t.textContent.trim())")
            check("S1 输入新属性回车加入（主角 + 御剑）", owned2 == ["主角", "御剑"], json.dumps(owned2, ensure_ascii=False))
            # 点池 chip 加入（拔剑，带 pop 过冲动画类）
            await click("[...document.querySelectorAll('.props-dialog .pool-chips .app-tag')].find((t) => t.textContent.includes('拔剑'))")
            pop = await ev("""(() => {
              const t = [...document.querySelectorAll('.props-dialog .pd-owned .app-tag')].find((x) => x.textContent.includes('拔剑'));
              return t ? { cls: t.className } : null;
            })()""")
            check("S1 点池 chip 加入拥有（带过冲回弹 pop 类）", bool(pop and "pop" in pop["cls"]), json.dumps(pop))
            # 池 × 全局删除（二维）：JS click（× 平时 width 0，hover 才滑入）
            await ev("[...document.querySelectorAll('.props-dialog .pool-chips .app-tag')].find((t) => t.textContent.includes('二维'))?.querySelector('.remove')?.click()")
            await asyncio.sleep(0.5)
            confirm = await ev("""(() => {
              const c = document.querySelector('.app-confirm, .app-modal.alert');
              return c ? { text: c.textContent } : null;
            })()""")
            check("S1 池 × 弹警告二次确认（删除后所有填写了的地方都失效）",
                  bool(confirm and "二维" in confirm["text"] and "失效" in confirm["text"]), json.dumps(confirm, ensure_ascii=False)[:200])
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('删除'))")
            await asyncio.sleep(0.6)
            after_del = await ev("""(() => ({
              calls: window.__calls.delete_props_globally,
              pool: [...document.querySelectorAll('.props-dialog .pool-chips .app-tag')].map((t) => t.textContent.trim()),
            }))()""")
            check("S1 确认删除 → delete_props_globally([二维]) + 池剔除",
                  bool(after_del and after_del["calls"] == [{"words": ["二维"]}] and "二维" not in after_del["pool"]),
                  json.dumps(after_del, ensure_ascii=False))
            await shot("props-dialog")
            # 确认提交
            await click("[...document.querySelectorAll('.props-dialog .app-btn')].find((b) => b.textContent.trim() === '确认')")
            await asyncio.sleep(0.8)
            calls = await ev("window.__calls.set_asset_props")
            chips_now = await ev("[...document.querySelectorAll('.detail-panel .section-custom .chips .app-tag')].map((t) => t.textContent.trim())")
            check("S1 确认 → set_asset_props(A1, [主角,御剑,拔剑]) + 弹窗关 + 详情 chips 刷新",
                  bool(calls and len(calls) == 1 and calls[0]["assetId"] == "A1" and calls[0]["props"] == ["主角", "御剑", "拔剑"] and chips_now == ["主角", "御剑", "拔剑"]),
                  json.dumps({"c": calls, "chips": chips_now}, ensure_ascii=False))

            # —— S4 描述截断（A1 长描述超 3 行）——
            clamp = await ev("""(() => {
              const item = document.querySelector('.detail-panel .section-custom .desc-item');
              if (!item) return null;
              const btn = item.querySelector('.desc-toggle');
              const content = item.querySelector('.d-content');
              return { btn: btn?.textContent.trim() ?? null, clamped: content.classList.contains('clamp'),
                       color: btn ? getComputedStyle(btn).color : null };
            })()""")
            check("S4 描述超 3 行截断 + 展开钮（标签色小字）",
                  bool(clamp and clamp["btn"] == "展开" and clamp["clamped"]), json.dumps(clamp, ensure_ascii=False))
            await click("document.querySelector('.detail-panel .section-custom .desc-toggle')")
            expanded = await ev("""(() => {
              const item = document.querySelector('.detail-panel .section-custom .desc-item');
              const c = item.querySelector('.d-content');
              return { btn: item.querySelector('.desc-toggle')?.textContent.trim(), clamped: c.classList.contains('clamp'),
                       h: Math.round(c.getBoundingClientRect().height) };
            })()""")
            check("S4 展开后全显（去 clamp，高 > 60）+ 钮变收起",
                  bool(expanded and expanded["btn"] == "收起" and not expanded["clamped"] and expanded["h"] > 60),
                  json.dumps(expanded))

            # —— S3 编辑描述弹窗（31 稿）——
            await click("document.querySelector('.detail-panel .section-custom .edit-desc')")
            await asyncio.sleep(0.8)
            dd = await ev("""(() => {
              const m = document.querySelector('.desc-dialog .app-modal');
              if (!m) return null;
              const b = m.getBoundingClientRect();
              const items = [...m.querySelectorAll('.dd-item')];
              const first = items[0];
              const arrows = first ? [...first.querySelectorAll('.dd-line:first-child .icon-btn.sort-arrow')].map((x) => x.disabled) : [];
              return { w: Math.round(b.width), rows: items.length, arrows,
                       title: m.querySelector('.titlebar .title')?.textContent,
                       btns: [...m.querySelectorAll('.actions .app-btn')].map((x) => x.textContent.trim()),
                       firstTitle: first?.querySelector('.dd-line:first-child input')?.value };
            })()""")
            print("描述弹窗:", json.dumps(dd, ensure_ascii=False))
            check("S3 弹窗 482 宽 / 初值 1 行（身份）/ 首行禁上移（下移可用，单条禁下移）",
                  bool(dd and dd["w"] == 482 and dd["rows"] == 1 and dd["firstTitle"] == "身份" and dd["arrows"] == [True, True]),
                  json.dumps(dd, ensure_ascii=False))
            # 添加描述 → 2 行；新行标题输入「设定」
            await click("[...document.querySelectorAll('.desc-dialog .app-btn')].find((b) => b.textContent.includes('添加描述'))")
            await asyncio.sleep(0.4)
            await ev("""(() => {
              const items = document.querySelectorAll('.desc-dialog .dd-item');
              const el = items[1].querySelector('.dd-line:first-child input');
              el.value = '设定';
              el.dispatchEvent(new Event('input', { bubbles: true }));
              return true;
            })()""")
            arrows2 = await ev("""(() => {
              const items = [...document.querySelectorAll('.desc-dialog .dd-item')];
              return items.map((it) => [...it.querySelectorAll('.dd-line:first-child .icon-btn.sort-arrow')].map((x) => x.disabled));
            })()""")
            check("S3 添加描述追加行 + 箭头禁用态（首行禁上 / 末行禁下）",
                  arrows2 == [[False, True], [True, False]], json.dumps(arrows2))
            # 第一条下移 → 交换顺序
            await click("document.querySelector('.desc-dialog .dd-item .dd-line .icon-btn')")
            await asyncio.sleep(0.5)
            order = await ev("[...document.querySelectorAll('.desc-dialog .dd-item')].map((it) => it.querySelector('.dd-line:first-child input').value)")
            check("S3 下移交换（设定 上 / 身份 下）", order == ["设定", "身份"], json.dumps(order, ensure_ascii=False))
            await shot("desc-dialog")
            await click("[...document.querySelectorAll('.desc-dialog .app-btn')].find((b) => b.textContent.trim() === '确认')")
            await asyncio.sleep(0.8)
            dcalls = await ev("window.__calls.set_asset_descriptions")
            check("S3 确认 → set_asset_descriptions 参数序（设定 → 身份）",
                  bool(dcalls and len(dcalls) == 1 and [d["title"] for d in dcalls[0]["descriptions"]] == ["设定", "身份"]),
                  json.dumps(dcalls, ensure_ascii=False))

            # —— S5/S6 封面设置（43 稿）：回 F-DIR 视口选档案卡 ——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.6)
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            await asyncio.sleep(0.8)
            cover_btn = await ev("""(() => {
              const t = document.querySelector('.detail-panel .section-pinned .thumb.cover-btn');
              return t ? { cursor: getComputedStyle(t).cursor } : null;
            })()""")
            check("S5 常态只有封面图（cover-btn 可点，名称/链接框保留）",
                  bool(cover_btn and cover_btn["cursor"] == "pointer"
                       and await ev("document.querySelectorAll('.detail-panel .section-pinned .field').length") == 2),
                  json.dumps(cover_btn))
            await click("document.querySelector('.detail-panel .section-pinned .thumb.cover-btn')")
            await asyncio.sleep(0.6)
            cs = await ev("""(() => {
              const g = document.querySelector('.detail-panel .cover-settings');
              if (!g) return null;
              const frame = g.querySelector('.cs-frame').getBoundingClientRect();
              return {
                frame: { w: Math.round(frame.width), h: Math.round(frame.height) },
                sliderOff: !!g.querySelector('.cs-slider.off'),
                change: g.querySelector('.cs-change')?.textContent.trim(),
                badgeVal: g.querySelector('.cs-badge-row input')?.value,
                sw: g.querySelector('.cs-color .sw')?.style.background,
                still: !!g.querySelector('.cs-still'),
                badgeChip: g.querySelector('.cs-badge')?.textContent.trim() ?? null,
              };
            })()""")
            print("封面设置:", json.dumps(cs, ensure_ascii=False))
            check("S5 43 组展开（框 218×168 / 滑块禁用 / 更换封面钮 / 徽标初值 设定 / 色块回显 / chip 预览）",
                  bool(cs and cs["frame"] == {"w": 218, "h": 168} and cs["sliderOff"] and cs["change"] == "更换封面"
                       and cs["badgeVal"] == "设定" and cs["badgeChip"] == "设定" and cs["still"]),
                  json.dumps(cs, ensure_ascii=False))
            # 颜色下拉展开 9 格 → 选第二色（橙 #352719）
            await click("document.querySelector('.detail-panel .cs-color')")
            await asyncio.sleep(0.4)
            cells = await ev("document.querySelectorAll('.detail-panel .cs-colors .cell').length")
            check("S5 颜色下拉展开 9 色格", cells == 9, str(cells))
            await ev("document.querySelectorAll('.detail-panel .cs-colors .cell')[1].click()")
            await asyncio.sleep(0.4)
            # 徽标改字「主角」
            await ev("""(() => {
              const el = document.querySelector('.detail-panel .cs-badge-row input');
              el.value = '主角';
              el.dispatchEvent(new Event('input', { bubbles: true }));
              return true;
            })()""")
            await asyncio.sleep(0.3)
            chip_now = await ev("(() => { const c = document.querySelector('.detail-panel .cs-badge'); return c ? { t: c.textContent.trim(), bg: c.style.background } : null; })()")
            check("S5 徽标 chip 实时预览（主角 + 橙底）",
                  bool(chip_now and chip_now["t"] == "主角" and "53, 39, 25" in str(chip_now["bg"])),
                  json.dumps(chip_now))
            await shot("cover-settings")
            # 确认 → set_archive_cover_settings（cover 键缺失 = 不动）
            await click("[...document.querySelectorAll('.detail-panel .cs-actions .app-btn')].find((b) => b.textContent.trim() === '确认')")
            await asyncio.sleep(0.8)
            ccalls = await ev("window.__calls.set_archive_cover_settings")
            back = await ev("""(() => ({
              editing: !!document.querySelector('.detail-panel .cover-settings'),
              chip: document.querySelector('.detail-panel .section-pinned .fmt-badge')?.textContent.trim() ?? null,
            }))()""")
            check("S5 确认 → badge {主角, #352719}、cover 键缺失（不动）；退回常态 + 徽标 chip 更新",
                  bool(ccalls and len(ccalls) == 1 and ccalls[0]["folderId"] == "F-ARC"
                       and ccalls[0]["badge"] == {"text": "主角", "color": "#352719"} and "cover" not in ccalls[0]
                       and back and back["editing"] is False and back["chip"] == "主角"),
                  json.dumps({"c": ccalls, "back": back}, ensure_ascii=False))

            # —— S6 取景态：更换封面 → 从本地磁盘中选择（桩回测试图）——
            await click("document.querySelector('.detail-panel .section-pinned .thumb.cover-btn')")
            await asyncio.sleep(0.5)
            await click("document.querySelector('.detail-panel .cs-change')")
            await asyncio.sleep(0.5)
            menu = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => ({ t: m.textContent.trim(), d: m.disabled }))")
            check("S6 更换封面菜单（10 稿两项；从资源库中选择 disabled 占位）",
                  bool(menu and len(menu) == 2 and menu[0]["t"] == "从本地磁盘中选择" and menu[1]["d"] is True),
                  json.dumps(menu, ensure_ascii=False))
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('从本地磁盘中选择'))")
            await asyncio.sleep(1.0)
            framing = await ev("""(() => {
              const g = document.querySelector('.detail-panel .cover-settings');
              const v = g?.querySelector('.cs-view');
              return v ? { off: !!g.querySelector('.cs-slider.off'), w: Math.round(v.getBoundingClientRect().width) } : null;
            })()""")
            check("S6 选图后进取景态（cs-view 渲染 + 滑块解禁）",
                  bool(framing and framing["off"] is False and framing["w"] >= 218), json.dumps(framing))
            # 向右拖 20px（crop.x ≈ 20/218）
            r = await rect_of("document.querySelector('.detail-panel .cs-frame')")
            await mouse("mousePressed", r["x"], r["y"])
            await mouse("mouseMoved", r["x"] + 20, r["y"])
            await mouse("mouseReleased", r["x"] + 20, r["y"])
            await asyncio.sleep(0.4)
            await shot("cover-framing")
            await click("[...document.querySelectorAll('.detail-panel .cs-actions .app-btn')].find((b) => b.textContent.trim() === '确认')")
            await asyncio.sleep(0.8)
            ccalls2 = await ev("window.__calls.set_archive_cover_settings")
            cov = (ccalls2 or [{}])[-1].get("cover") if ccalls2 else None
            # 期望位移 = min(20/218, clamp 上限 mx)——4:3 测试图 cover 铺满后横向余量只有几 px
            dims = await ev("""(() => new Promise((resolve) => {
              const img = new Image();
              img.onload = () => resolve({ w: img.naturalWidth, h: img.naturalHeight });
              img.onerror = () => resolve(null);
              img.src = encodeURI('/@fs/' + '""" + STILL.replace("\\", "/") + """');
            }))()""")
            mx = 0.0
            if dims:
                z0 = max(218 / dims["w"], 168 / dims["h"])
                mx = max(0.0, (dims["w"] * z0 - 218) / 2 / 218)
            expected = min(20 / 218, mx)
            ok_crop = cov and abs(cov["crop"]["x"] - expected) < 0.005 and cov["crop"]["scale"] == 1 and cov["sourcePath"].endswith("still.webp")
            check("S6 确认 → cover { sourcePath, crop.x≈clamp 上限, scale 1 }（烘培入参）",
                  bool(ok_crop), json.dumps({"cov": cov, "expected": expected}, ensure_ascii=False))

            # —— S8 取消零写入 ——
            before = await ev("JSON.stringify(window.__calls.set_archive_cover_settings)")
            await click("document.querySelector('.detail-panel .section-pinned .thumb.cover-btn')")
            await asyncio.sleep(0.5)
            await ev("""(() => {
              const el = document.querySelector('.detail-panel .cs-badge-row input');
              el.value = '改动';
              el.dispatchEvent(new Event('input', { bubbles: true }));
              return true;
            })()""")
            await click("[...document.querySelectorAll('.detail-panel .cs-actions .app-btn')].find((b) => b.textContent.trim() === '取消')")
            await asyncio.sleep(0.5)
            after = await ev("JSON.stringify(window.__calls.set_archive_cover_settings)")
            check("S8 取消零写入（调用数不变）+ 退回常态",
                  before == after and await ev("!!document.querySelector('.detail-panel .cover-settings')") is False, "")

            # —— S2 档案属性弹窗 ——
            await click("document.querySelector('.detail-panel .section-custom .chip-add')")
            await asyncio.sleep(0.8)
            arc_owned = await ev("[...document.querySelectorAll('.props-dialog .pd-owned .app-tag')].map((t) => t.textContent.trim())")
            check("S2 档案属性弹窗初值（奇幻）", arc_owned == ["奇幻"], json.dumps(arc_owned, ensure_ascii=False))
            await click("[...document.querySelectorAll('.props-dialog .pool-chips .app-tag')].find((t) => t.textContent.includes('二维'))")
            await asyncio.sleep(0.3)
            await click("[...document.querySelectorAll('.props-dialog .app-btn')].find((b) => b.textContent.trim() === '确认')")
            await asyncio.sleep(0.8)
            acalls = await ev("window.__calls.set_archive_props")
            check("S2 档案确认 → set_archive_props(F-ARC, [奇幻,二维])",
                  bool(acalls and len(acalls) == 1 and acalls[0]["folderId"] == "F-ARC" and acalls[0]["props"] == ["奇幻", "二维"]),
                  json.dumps(acalls, ensure_ascii=False))

            # —— S9 数据区块独立设置（2026-08-23 用户定：文件 / 档案各一套，不再联动）——
            # 当前上下文 = F-DIR 视口选中档案卡 F-ARC → 右栏 22 稿面板编辑档案套
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            # 眼睛隐藏「音频」（档案套）
            await click("[...document.querySelectorAll('.section-edit .se-row')].find((r) => r.querySelector('.row-title')?.textContent === '音频').querySelector('.icon-btn.eye')")
            await asyncio.sleep(1.2)  # 等 500ms 防抖持久化
            indep = await ev("""(() => {
              const saves = window.__calls.save_ui_settings.map((s) => s);
              const last = saves[saves.length - 1] ?? {};
              const arc = last.detailSectionsArchive ?? [];
              const fil = last.detailSections ?? [];
              const titles = [...document.querySelectorAll('.detail-panel .detail-block .section-header .title')].map((t) => t.textContent);
              return {
                arcAudio: arc.find((s) => s.id === 'audio')?.visible,
                filAudio: fil.find((s) => s.id === 'audio')?.visible,
                detailTitles: titles,
              };
            })()""")
            print("独立设置:", json.dumps(indep, ensure_ascii=False))
            check("S9 档案套隐藏音频：detailSectionsArchive.audio=false 且文件套 detailSections 不动",
                  bool(indep and indep["arcAudio"] is False and indep["filAudio"] is not False),
                  json.dumps(indep, ensure_ascii=False))
            check("S9 档案详情联动（音频区块消失）",
                  bool(indep and "音频" not in indep["detailTitles"] and "关系" in indep["detailTitles"]),
                  json.dumps(indep and indep["detailTitles"], ensure_ascii=False))
            # 关面板 → 切文件卡（F-SUB 视口 A1）：文件详情音频区块仍在（文件套不受影响）
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.4)
            await click("document.querySelector('.tree-item[data-folder-id=\"F-SUB\"] .body')")
            await asyncio.sleep(0.6)
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(0.8)
            file_titles = await ev("[...document.querySelectorAll('.detail-panel .detail-block .section-header .title')].map((t) => t.textContent)")
            check("S9 文件详情不受档案套改动影响（音频区块仍在）",
                  bool(file_titles and "音频" in file_titles), json.dumps(file_titles, ensure_ascii=False))
            # 右栏再开面板（选中文件 → 文件套）：音频行眼睛为开（两套互不影响）
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            feye = await ev("""(() => {
              const row = [...document.querySelectorAll('.section-edit .se-row')].find((r) => r.querySelector('.row-title')?.textContent === '音频');
              return row ? { off: row.classList.contains('off') } : null;
            })()""")
            check("S9 文件套面板行态独立（音频未隐藏）", bool(feye and feye["off"] is False), json.dumps(feye))
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.4)

            # —— S10 颜色 bar 真色（6.1 颜色解析）：当前上下文 = F-SUB 视口 A1 已选中 ——
            sw = await ev("""(() => {
              const els = [...document.querySelectorAll('.detail-panel .swatches .swatch')];
              return els.map((el) => getComputedStyle(el).backgroundColor);
            })()""")
            print("A1 色块:", json.dumps(sw))
            check("S10 颜色 bar 真色（A1 三色，存 10 显 7 之前 7 槽按实际数渲染）",
                  bool(sw == ["rgb(200, 30, 30)", "rgb(30, 30, 200)", "rgb(232, 232, 232)"]), json.dumps(sw))
            # 无颜色资产（fla 工程文件）→ 恒显 bar 7 个占位色块（透明底）
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('场景源文件'))")
            await asyncio.sleep(0.8)
            sw2 = await ev("""(() => {
              const els = [...document.querySelectorAll('.detail-panel .swatches .swatch')];
              return els.map((el) => getComputedStyle(el).backgroundColor);
            })()""")
            check("S10 无颜色资产 = 7 占位色块（透明底恒显）",
                  bool(sw2 and len(sw2) == 7 and all(c == "rgba(0, 0, 0, 0)" for c in sw2)), json.dumps(sw2))
    finally:
        proc.kill()
        if dev_proc:
            dev_proc.kill()

    print()
    if fails:
        print(f"FAIL {len(fails)} 项：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部 PASS")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
