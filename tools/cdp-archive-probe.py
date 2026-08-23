# CDP 实测：档案子系统一轮 + 详情面板 8 区块体系（2026-08-15，3.2 一轮）
# 桩带可变状态（folders/placements/archives 三张表，写命令真改数据，refresh 后 UI 跟着变）：
# ① 树形态：档案节点 = variant archive（archive-dot 品牌圆点替代展开箭头，无子级）；
# ② 中间面板三区：文件夹区（摘出档案）/ 档案区（SectionHeader「档案 - N」）/ 文件区；
#    档案卡片 = 纸张 SVG×2 + 封面 + 左上徽标（badge）+ 底部信息条（名称 / N 个文件 / paperclip）；
# ③ 档案卡片右键 04 稿菜单（320 + 搜索框）→「新建档案」= 同级（parentId/afterFolderId 断言）
#    → 先建后命名（树命名态 rename_folder）；树档案节点右键同 04 稿（无「转换为档案」项）；
# ④ 转换为档案两段式：有子级 → convert_to_archive(confirmed=false) → need_confirm →
#    25 稿 AppModal alert warning（标题/按钮序主左次右/取消零转换）→ 剔除转换 confirmed=true
#    → 剔除层级（子文件夹消失、文件并入）；无子级直接转（无弹窗）；
# ⑤ 解散档案为文件夹：convert_to_folder → 摘 kind（圆点消失）+ 档案区无内容整块不渲染；
# ⑥ 详情面板区块体系（2026-08-15 定案）：pinned 缩略图区块恒第一（缩略图+颜色 bar+
#    名称/链接输入框，组 padding 16 gap 12）+ 注册表 7 区块（自定义数据带标题 gap 8
#    内容 gap 12 / 基础信息 §1 / 链接区块两段化「区块名 / 所属目录」+ .档案后缀 +
#    行2 原始文件徽章 / 副本·关系·音频·提示词空态）+ 空区块折叠零高度 +
#    分隔线通栏 + 无横向溢出 + 面板内拖拽标题排序；选中档案卡片 → 档案形态；点空白 = 空面板；
# ⑦ 22 稿编辑数据区块面板：404 宽 / 7 行 / 首行禁上移末行禁下移（禁用色 --line-active）/
#    眼睛切换 → 标题禁用色 + 详情面板区块显隐联动 / 排序箭头即时反映 / save_ui_settings 持久化 /
#    行不可拖起（2026-08-15 定案拆除）+ 箭头交换动画（TransitionGroup se-swap-move 0.18s）/
#    详情面板内拖拽标题排序落点 + 面板同步；
# ⑧ Esc 关菜单不清选择（2026-08-16 根治：右键菜单 / 22 稿面板两路径，再按 Esc 原语义清选择）；
# ⑨ 回收站视口选中详情（pinned + 基础信息实数据 + 链接区块「回收站 / 原位置」兜底条目）。
# 截图留档 tools/style-preview/：archive-sections / archive-card / archive-card-menu /
# archive-convert-warning / archive-detail-file / archive-detail-archive / archive-section-edit /
# archive-section-edit-drag。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9360
URL = "http://localhost:5373"

# 桩：可变状态三表。文件夹树——素材(F-DIR，block dir)┬子文件夹(F-SUB)└设定集(F-ARC，archive)；
# 待转换(F-CONV，顶级，带子文件夹 F-CONV-SUB)；直接转换(F-PLAIN，顶级无子级)。
# 资产：A1/A2 在设定集（静态 webp，缩略图指向 tools/test-assets 真实文件，兼作档案封面），
# A3 在 F-CONV-SUB（转换后并入 F-CONV），A4 直接在素材。archives 表：F-ARC 带徽标/属性/描述。
STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__calls = { create_archive: [], convert_to_archive: [], convert_to_folder: [], rename_folder: [], save_ui_settings: [] };
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
      { id: "F-CONV", name: "待转换", parent: null, block: "dir", kind: null },
      { id: "F-CONV-SUB", name: "内层", parent: "F-CONV", kind: null },
      { id: "F-PLAIN", name: "直接转换", parent: null, block: "dir", kind: null },
    ],
    assets: {
      A1: { id: "A1", originName: "主角-立绘.webp", ext: "webp", size: 204800, width: 800, height: 600, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: ["主角"], links: ["https://example.com/ref"], note: "" },
      A2: { id: "A2", originName: "场景-黄昏.webp", ext: "webp", size: 102400, width: 640, height: 360, duration: null, importedAt: "2026-08-14T09:00:00+08:00", modifiedAt: "2026-08-14T09:00:00+08:00", tags: [], links: [], note: "" },
      A3: { id: "A3", originName: "内层素材.webp", ext: "webp", size: 51200, width: 320, height: 240, duration: null, importedAt: "2026-08-13T08:00:00+08:00", modifiedAt: "2026-08-13T08:00:00+08:00", tags: [], links: [], note: "" },
      A4: { id: "A4", originName: "根文件.webp", ext: "webp", size: 4096, width: 100, height: 100, duration: null, importedAt: "2026-08-12T08:00:00+08:00", modifiedAt: "2026-08-12T08:00:00+08:00", tags: [], links: [], note: "" },
      AREC: { id: "AREC", originName: "废弃稿.webp", ext: "webp", size: 51200, width: 320, height: 240, duration: null, importedAt: "2026-08-10T08:00:00+08:00", modifiedAt: "2026-08-10T08:00:00+08:00", tags: [], links: [], note: "" },
    },
    placements: [
      { assetId: "A1", folderId: "F-ARC", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A1", folderId: "F-SUB", name: "主角-立绘.webp", w: 800, h: 600, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A2", folderId: "F-ARC", name: "场景-黄昏.webp", w: 640, h: 360, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A3", folderId: "F-CONV-SUB", name: "内层素材.webp", w: 320, h: 240, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A4", folderId: "F-DIR", name: "根文件.webp", w: 100, h: 100, ext: "webp", thumbPath: STILL, thumbKind: "asset", origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ],
    // 回收站（2026-08-16）：AREC 已删除（无存活 Placement），recycle.json 记录删除前位置（F-ARC）
    recycle: [
      { asset: "AREC", placements: [{ asset: "AREC", folder: "F-ARC", name: "废弃稿.webp", order: 0, origin: true }], deletedAt: "2026-08-16T08:00:00+08:00" },
    ],
    archives: {
      "F-ARC": { folderId: "F-ARC", badge: { text: "设定", color: "#B38427" }, avatar: null, props: ["奇幻"], descriptions: [{ title: "来源", content: "官方设定集" }], links: ["https://archive.ref"], createdAt: "2026-08-15T09:00:00+08:00" },
    },
    newId: 0,
  };
  const byId = (id) => DB.folders.find((f) => f.id === id);
  const childrenOf = (id) => DB.folders.filter((f) => f.parent === id);
  // 递归 Placement 数（含全部后代，后端 count 口径）
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
  // 后代文件夹 id 集（剔除层级用）
  const descendants = (id) => {
    const out = [];
    const walk = (pid) => { for (const c of childrenOf(pid)) { out.push(c.id); walk(c.id); } };
    walk(id);
    return out;
  };
  const assetDetail = (assetId) => {
    const meta = DB.assets[assetId];
    if (!meta) return null;
    const placements = [];
    const archives = [];
    for (const p of DB.placements.filter((x) => x.assetId === assetId)) {
      if (p.folderId === "root") {
        placements.push({ folderId: null, blockName: "目录", path: ["未分类"], name: p.name, origin: p.origin });
        continue;
      }
      const chain = [];
      let cur = p.folderId;
      while (cur) {
        const f = byId(cur);
        if (!f) break;
        chain.unshift(f);
        cur = f.parent;
      }
      let path = chain.map((f) => f.name);
      if (path.length > 3) path = [path[0], path[path.length - 2], path[path.length - 1]];
      placements.push({ folderId: p.folderId, blockName: "目录", path, name: p.name, origin: p.origin });
      for (const f of chain) {
        if (f.kind === "archive" && !archives.some((a) => a.folderId === f.id)) archives.push({ folderId: f.id, name: f.name });
      }
    }
    // 回收站资产兜底（2026-08-16 同后端）：无存活 Placement →「回收站（原位置：区块 / 路径）」
    if (placements.length === 0) {
      const item = DB.recycle.find((i) => i.asset === assetId);
      const rp = item?.placements[0];
      if (rp) {
        const chain = [];
        let cur = rp.folder;
        while (cur) {
          const f = byId(cur);
          if (!f) break;
          chain.unshift(f);
          cur = f.parent;
        }
        placements.push({ folderId: null, blockName: "回收站", path: [`目录 / ${chain.map((f) => f.name).join(" / ")}`], name: rp.name, origin: rp.origin });
      }
    }
    return {
      assetId: meta.id, originName: meta.originName, ext: meta.ext, size: meta.size,
      width: meta.width, height: meta.height, duration: meta.duration,
      importedAt: meta.importedAt, modifiedAt: meta.modifiedAt,
      tags: meta.tags, descriptions: meta.descriptions ?? [], links: meta.links, note: meta.note, placements, archives,
    };
  };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    convertFileSrc: (p) => encodeURI("/@fs/" + p),
    transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
    invoke: (cmd, args) => {
      const R = {
        get_current_library: { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 4, folderCount: 6 },
        get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
        get_ui_settings: {},
        list_placements: () => DB.placements,
        list_folder_tree: () => tree(),
        list_archives: () => Object.values(DB.archives),
        list_recycle: () => DB.recycle.map((i) => {
          const meta = DB.assets[i.asset];
          return {
            assetId: i.asset, folderId: "recycle", name: i.placements[0]?.name ?? meta.originName,
            w: meta.width, h: meta.height, ext: meta.ext, thumbPath: STILL, thumbKind: "asset",
            origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
            deletedAt: i.deletedAt, placementCount: i.placements.length,
          };
        }),
        list_recent_assets: [], list_inbox_snapshot: [],
        get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
        list_space_libraries: ["动画资产"],
        list_recent_libraries: [],
      };
      if (cmd === "save_ui_settings") { window.__calls.save_ui_settings.push(args.settings); return Promise.resolve(null); }
      if (cmd === "create_archive") {
        window.__calls.create_archive.push(args);
        const id = "F-NEW" + ++DB.newId;
        DB.folders.push({ id, name: args.name, parent: args.parentId ?? null, block: args.blockId ?? "dir", kind: "archive" });
        return Promise.resolve({ id });
      }
      if (cmd === "rename_folder") {
        window.__calls.rename_folder.push(args);
        const f = byId(args.folderId);
        if (f) f.name = args.newName;
        return Promise.resolve(null);
      }
      if (cmd === "convert_to_archive") {
        window.__calls.convert_to_archive.push(args);
        const f = byId(args.folderId);
        if (!f) return Promise.reject("文件夹不存在");
        if (f.kind === "archive") return Promise.resolve("done"); // 幂等
        if (childrenOf(f.id).length > 0 && !args.confirmed) return Promise.resolve("need_confirm");
        // 剔除层级：后代文件夹删除、其 Placement 并入本档案
        const desc = descendants(f.id);
        for (const p of DB.placements) if (desc.includes(p.folderId)) p.folderId = f.id;
        DB.folders = DB.folders.filter((x) => !desc.includes(x.id));
        f.kind = "archive";
        return Promise.resolve("done");
      }
      if (cmd === "convert_to_folder") {
        window.__calls.convert_to_folder.push(args);
        const f = byId(args.folderId);
        if (f) f.kind = null;
        delete DB.archives[args.folderId];
        return Promise.resolve(null);
      }
      if (cmd === "get_archive_data") {
        const rec = DB.archives[args.folderId];
        const inFolder = DB.placements.filter((p) => p.folderId === args.folderId);
        const uniq = new Set(inFolder.map((p) => p.assetId));
        const totalSize = [...uniq].reduce((s, a) => s + (DB.assets[a]?.size ?? 0), 0);
        const base = rec ?? { folderId: args.folderId, badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "" };
        return Promise.resolve({ ...base, fileCount: inFolder.length, totalSize });
      }
      if (cmd === "get_asset_detail") {
        const d = assetDetail(args.assetId);
        return d ? Promise.resolve(d) : Promise.reject("资源不存在，可能已被删除");
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
    # 5373 已被监听则复用；否则自起 pnpm dev，结束时杀掉（tree-sort-drag 探针先例）
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
            await asyncio.sleep(6)  # 等 vite 编译 + 首屏渲染

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "buttons": 1 if button == "left" else 2, "x": x, "y": y, "clickCount": clicks}, session=sess)

            async def rect_of(expr):
                # 无头窗口 innerH 比 window-size 小约 130（浏览器边框），卡片中心可能落到视口外
                # （elementFromPoint 返回 null 点击无效）——钳进视口内
                return await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")

            async def click(expr):
                r = await rect_of(expr)
                if not r:
                    print("!! 元素不存在:", expr[:80])
                    return False
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return True

            async def rclick(expr):
                r = await rect_of(expr)
                if not r:
                    print("!! 元素不存在:", expr[:80])
                    return False
                await mouse("mousePressed", r["x"], r["y"], button="right")
                await mouse("mouseReleased", r["x"], r["y"], button="right")
                await asyncio.sleep(0.6)
                return True

            async def click_menu(text):
                r = await rect_of(f"[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('{text}'))")
                if not r:
                    print("!! 菜单项不存在:", text)
                    return False
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.6)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # —— ① 树形态：档案节点 archive-dot ——
            # 先展开「素材」（其子级 F-ARC / F-SUB 才上树）
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .expand')")
            await asyncio.sleep(0.5)
            arc_item = await ev("""(() => {
              const it = document.querySelector('.tree-item[data-folder-id="F-ARC"]');
              if (!it) return null;
              return {
                variant: it.classList.contains('archive'),
                dot: !!it.querySelector('.archive-dot-slot'),
                arrowBtn: !!it.querySelector('button.expand'),
                decoLen: it.querySelector('.body .deco')?.innerHTML?.length ?? 0,
              };
            })()""")
            print("树档案节点:", json.dumps(arc_item, ensure_ascii=False))
            check("① 树档案节点 variant archive + archive-dot", bool(arc_item and arc_item["variant"] and arc_item["dot"]))
            check("① 树档案节点无展开箭头", bool(arc_item and not arc_item["arrowBtn"]))
            dir_deco = await ev("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body .deco')?.innerHTML?.length ?? 0")
            check("① 档案节点图标 ≠ 文件夹图标", bool(arc_item and dir_deco and arc_item["decoLen"] != dir_deco), f"arc={arc_item and arc_item['decoLen']} dir={dir_deco}")

            # 进入「素材」视口
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.8)

            # —— ② 中间面板三区 + 档案卡片 ——
            sections = await ev("""(() => {
              const head = (sel) => { const h = document.querySelector(sel + ' .section-header'); return h ? { title: h.querySelector('.title')?.textContent, count: h.querySelector('.count')?.textContent?.trim() ?? null } : null; };
              return { folder: head('.folder-block'), archive: head('.archive-block'), archiveSection: !!document.querySelector('.archive-block') };
            })()""")
            print("三区标题:", json.dumps(sections, ensure_ascii=False))
            check("② 文件夹区摘出档案（文件夹 - 1）", sections.get("folder") == {"title": "文件夹", "count": "- 1"}, json.dumps(sections, ensure_ascii=False))
            check("② 档案区标题计数（档案 - 1）", sections.get("archive") == {"title": "档案", "count": "- 1"}, json.dumps(sections, ensure_ascii=False))

            card = await ev("""(() => {
              const c = document.querySelector('.archive-card[data-folder-id="F-ARC"]');
              if (!c) return null;
              const b = c.getBoundingClientRect();
              const badge = c.querySelector('.badge');
              const imgs = c.querySelectorAll('.thumb img');
              return {
                ratio: Math.round((b.width / b.height) * 1000) / 1000,
                papers: c.querySelectorAll('.paper').length,
                cover: imgs.length > 0 && (imgs[0].src || '').includes('/@fs/'),
                badge: badge ? { text: badge.textContent, bg: getComputedStyle(badge).backgroundColor } : null,
                name: c.querySelector('.info .title')?.textContent ?? null,
                meta: c.querySelector('.info .meta')?.textContent ?? null,
                clip: !!c.querySelector('.clip svg'),
              };
            })()""")
            print("档案卡片:", json.dumps(card, ensure_ascii=False))
            check("② 档案卡片长宽比 260:298", bool(card and abs(card["ratio"] - 260 / 298) < 0.02), str(card and card["ratio"]))
            check("② 档案卡片纸张 SVG×2 + 封面", bool(card and card["papers"] == 2 and card["cover"]))
            check("② 档案卡片徽标（设定 / #B38427）", bool(card and card["badge"] and card["badge"]["text"] == "设定" and card["badge"]["bg"] == "rgb(179, 132, 39)"), json.dumps(card and card["badge"]))
            check("② 档案卡片信息条（设定集 / 2 个文件 / paperclip）", bool(card and card["name"] == "设定集" and card["meta"] == "2 个文件" and card["clip"]), json.dumps(card, ensure_ascii=False))
            await shot("archive-sections")
            await shot("archive-card")

            # —— ③ 档案卡片右键 04 稿菜单 → 新建档案（同级 + 先建后命名）——
            await rclick("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            menu = await ev("""(() => {
              const m = document.querySelector('.app-menu');
              if (!m) return null;
              const items = [...m.querySelectorAll('.menu-item')].map((i) => i.textContent.trim());
              return {
                width: m.style.width,
                search: !!m.querySelector('input'),
                hasNewArchive: items.some((t) => t.includes('新建档案')),
                hasDissolve: items.some((t) => t.includes('解散档案为文件夹')),
                hasConvert: items.some((t) => t.includes('转换为档案')),
              };
            })()""")
            print("04 稿菜单:", json.dumps(menu, ensure_ascii=False))
            check("③ 档案卡片菜单 04 稿（320 + 搜索框 + 新建档案 + 解散，无转换为档案）",
                  bool(menu and menu["width"] == "320px" and menu["search"] and menu["hasNewArchive"] and menu["hasDissolve"] and not menu["hasConvert"]),
                  json.dumps(menu, ensure_ascii=False))
            await shot("archive-card-menu")
            await click_menu("新建档案")
            await asyncio.sleep(0.8)
            ca = await ev("window.__calls.create_archive")
            print("create_archive 调用:", json.dumps(ca, ensure_ascii=False))
            check("③ 新建档案 = 同级（parentId=F-DIR / afterFolderId=F-ARC / blockId=null）",
                  bool(ca and len(ca) == 1 and ca[0].get("parentId") == "F-DIR" and ca[0].get("afterFolderId") == "F-ARC" and ca[0].get("blockId") is None and ca[0].get("name") == "新建档案"),
                  json.dumps(ca, ensure_ascii=False))
            naming = await ev("({ naming: !!document.querySelector('.tree-item.naming input'), toast: document.querySelector('.toast-stack')?.textContent ?? '' })")
            check("③ 先建后命名：树命名态 + toast", bool(naming and naming["naming"] and "已新建档案" in naming["toast"]), json.dumps(naming, ensure_ascii=False))
            # 命名态改名「新档案甲」→ Enter → rename_folder
            await ev("""(() => {
              const inp = document.querySelector('.tree-item.naming input');
              const set = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
              set.call(inp, '新档案甲');
              inp.dispatchEvent(new Event('input', { bubbles: true }));
              inp.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
            })()""")
            await asyncio.sleep(0.8)
            rf = await ev("window.__calls.rename_folder")
            print("rename_folder 调用:", json.dumps(rf, ensure_ascii=False))
            check("③ 命名提交 rename_folder（F-NEW1 → 新档案甲）",
                  bool(rf and len(rf) == 1 and rf[0].get("folderId") == "F-NEW1" and rf[0].get("newName") == "新档案甲"),
                  json.dumps(rf, ensure_ascii=False))
            new_node = await ev("""(() => {
              const it = document.querySelector('.tree-item[data-folder-id="F-NEW1"]');
              return it ? { label: it.querySelector('.label')?.textContent, dot: !!it.querySelector('.archive-dot-slot') } : null;
            })()""")
            check("③ 新档案节点上树（新档案甲 + archive-dot）", bool(new_node and new_node["label"] == "新档案甲" and new_node["dot"]), json.dumps(new_node, ensure_ascii=False))

            # —— ④ 转换为档案两段式（25 稿警告）——
            # 先走取消路径：警告窗点取消 = 零转换
            await rclick("document.querySelector('.tree-item[data-folder-id=\"F-CONV\"]')")
            await click_menu("转换为档案")
            await asyncio.sleep(0.6)
            warn = await ev("""(() => {
              const m = document.querySelector('.app-modal.alert');
              if (!m) return null;
              return {
                title: m.querySelector('.title')?.textContent ?? null,
                desc: m.querySelector('.desc')?.textContent?.trim() ?? null,
                badge: !!m.querySelector('.alert-badge'),
                width: Math.round(m.getBoundingClientRect().width),
                buttons: [...m.querySelectorAll('.actions .app-btn')].map((b) => ({ label: b.textContent.trim(), primary: b.classList.contains('primary'), panel: b.classList.contains('panel'), w: Math.round(b.getBoundingClientRect().width), h: Math.round(b.getBoundingClientRect().height) })),
              };
            })()""")
            print("25 稿警告:", json.dumps(warn, ensure_ascii=False))
            check("④ 25 稿警告弹窗（alert + warning 徽章 + 标题）",
                  bool(warn and warn["badge"] and warn["title"] == "该文件夹内有子文件夹存在，是否剔除层级转换！"),
                  json.dumps(warn, ensure_ascii=False))
            check("④ 25 稿警告正文", bool(warn and "可保留全部文件剔除层级转换" in (warn["desc"] or "")), str(warn and warn["desc"]))
            btns = (warn or {}).get("buttons") or []
            check("④ 25 稿按钮序主左次右（剔除转换 primary / 取消 panel，88×38）",
                  len(btns) == 2 and btns[0]["label"] == "剔除转换" and btns[0]["primary"] and btns[1]["label"] == "取消" and btns[1]["panel"] and all(b["w"] == 88 and b["h"] == 38 for b in btns),
                  json.dumps(btns, ensure_ascii=False))
            print("④ 25 稿弹窗实测宽:", warn and warn["width"], "（稿值 500）")
            await shot("archive-convert-warning")
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('取消'))")
            await asyncio.sleep(0.6)
            cv1 = await ev("window.__calls.convert_to_archive")
            still_folder = await ev("!document.querySelector('.tree-item[data-folder-id=\"F-CONV\"] .archive-dot-slot') && !document.querySelector('.app-modal')")
            check("④ 取消路径：零 confirmed 重提 + 未转换 + 关窗",
                  bool(cv1 and len(cv1) == 1 and cv1[0].get("confirmed") is False and still_folder),
                  json.dumps(cv1, ensure_ascii=False))
            # 再走确认路径：剔除转换
            await rclick("document.querySelector('.tree-item[data-folder-id=\"F-CONV\"]')")
            await click_menu("转换为档案")
            await asyncio.sleep(0.6)
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('剔除转换'))")
            await asyncio.sleep(1.0)
            cv2 = await ev("window.__calls.convert_to_archive")
            conv = await ev("""({
              dot: !!document.querySelector('.tree-item[data-folder-id="F-CONV"] .archive-dot-slot'),
              childGone: !document.querySelector('.tree-item[data-folder-id="F-CONV-SUB"]'),
              count: document.querySelector('.tree-item[data-folder-id="F-CONV"] .count')?.textContent ?? null,
              toast: document.querySelector('.toast-stack')?.textContent ?? '',
            })""")
            print("剔除转换后:", json.dumps(conv, ensure_ascii=False))
            check("④ 确认路径：confirmed=true 重提", bool(cv2 and len(cv2) == 3 and cv2[1].get("confirmed") is False and cv2[2].get("confirmed") is True), json.dumps(cv2))
            check("④ 剔除层级转换：档案圆点 + 子文件夹消失 + 文件并入（计数 1）",
                  bool(conv["dot"] and conv["childGone"] and conv["count"] == "1" and "已转换为档案" in conv["toast"]),
                  json.dumps(conv, ensure_ascii=False))
            # 无子级直接转：无弹窗
            await rclick("document.querySelector('.tree-item[data-folder-id=\"F-PLAIN\"]')")
            await click_menu("转换为档案")
            await asyncio.sleep(0.8)
            direct = await ev("""({
              modal: !!document.querySelector('.app-modal'),
              dot: !!document.querySelector('.tree-item[data-folder-id="F-PLAIN"] .archive-dot-slot'),
              calls: window.__calls.convert_to_archive.length,
            })""")
            print("直接转换:", json.dumps(direct, ensure_ascii=False))
            check("④ 无子级直接转换（无弹窗一次到位）", bool(not direct["modal"] and direct["dot"] and direct["calls"] == 4), json.dumps(direct))

            # —— ⑤ 解散档案为文件夹 + 档案区无内容不渲染 ——
            await rclick("document.querySelector('.tree-item[data-folder-id=\"F-NEW1\"]')")
            menu2 = await ev("""(() => {
              const items = [...document.querySelectorAll('.app-menu .menu-item')].map((i) => i.textContent.trim());
              return { dissolve: items.some((t) => t.includes('解散档案为文件夹')), convert: items.some((t) => t.includes('转换为档案')) };
            })()""")
            check("⑤ 树档案节点右键 = 04 稿档案菜单", bool(menu2 and menu2["dissolve"] and not menu2["convert"]), json.dumps(menu2, ensure_ascii=False))
            await click_menu("解散档案为文件夹")
            await asyncio.sleep(0.8)
            dis = await ev("""({
              calls: window.__calls.convert_to_folder,
              dotGone: !document.querySelector('.tree-item[data-folder-id="F-NEW1"] .archive-dot-slot'),
              arrow: !!document.querySelector('.tree-item[data-folder-id="F-NEW1"] button.expand'),
            })""")
            print("解散:", json.dumps(dis, ensure_ascii=False))
            check("⑤ 解散档案为文件夹（convert_to_folder + 圆点消失）",
                  bool(dis["calls"] and len(dis["calls"]) == 1 and dis["calls"][0].get("folderId") == "F-NEW1" and dis["dotGone"] and dis["arrow"]),
                  json.dumps(dis, ensure_ascii=False))
            # 回到「素材」视口：解散后 F-NEW1 落回文件夹区，档案区仍剩设定集
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.8)
            sec2 = await ev("""({
              archive: document.querySelector('.archive-block .section-header .count')?.textContent?.trim() ?? null,
              folder: document.querySelector('.folder-block .section-header .count')?.textContent?.trim() ?? null,
            })""")
            check("⑤ 解散后落回文件夹区（文件夹 - 2 / 档案 - 1）", sec2 == {"archive": "- 1", "folder": "- 2"}, json.dumps(sec2, ensure_ascii=False))

            # —— ⑥ 详情面板 8 区块（选中文件卡片实数据）——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(1.0)
            detail = await ev("""(() => {
              const p = document.querySelector('.detail-panel');
              if (!p) return null;
              const blocks = [...p.querySelectorAll('.detail-block')];
              const pinned = p.querySelector('.section-pinned');
              const custom = p.querySelector('.section-custom');
              const info = p.querySelector('.section-info .info-card');
              const ks = info ? [...info.querySelectorAll('.col:first-child .k')].map((s) => s.textContent) : [];
              const vs = info ? [...info.querySelectorAll('.col.vals .v')].map((s) => [...s.children].map((c) => c.textContent).join(' ')) : [];
              const pair = {};
              ks.forEach((k, i) => (pair[k] = vs[i]));
              const thumbImg = pinned?.querySelector('.thumb img');
              const thumbEl = pinned?.querySelector('.thumb');
              const thumbGroup = pinned?.querySelector('.thumb-group');
              const swatches = [...(pinned?.querySelectorAll('.swatches .swatch') ?? [])];
              const fieldEls = [...(pinned?.querySelectorAll('.field') ?? [])];
              const fields = [...(pinned?.querySelectorAll('.field input') ?? [])];
              const chip = custom?.querySelector('.chips .app-tag');
              const chipsRow = custom?.querySelector('.chips');
              const probeColor = (v) => { const p2 = document.createElement('div'); p2.style.color = v; document.body.appendChild(p2); const c = getComputedStyle(p2).color; p2.remove(); return c; };
              const linkItem = p.querySelector('.section-links .link-item');
              const customBody = custom?.querySelector('.custom-body');
              const editDesc = custom?.querySelector('.edit-desc');
              const body = document.querySelector('.detail-body');
              const bbr = body.getBoundingClientRect();
              const bx = { l: Math.round(bbr.left), r: Math.round(bbr.right) };
              return {
                titles: blocks.map((b) => b.querySelector('.section-header .title')?.textContent ?? '(pinned)'),
                pinned: blocks[0]?.classList.contains('pinned') ?? null,
                hasArchive: !!p.querySelector('.section-archive'),
                customTitle: custom?.querySelector('.section-header .title')?.textContent ?? null,
                customGap: custom ? getComputedStyle(custom).gap : null,
                bodyGap: customBody ? getComputedStyle(customBody).gap : null,
                thumbFit: thumbImg ? getComputedStyle(thumbImg).objectFit : null,
                thumbBg: thumbEl ? getComputedStyle(thumbEl).backgroundColor : null,
                thumbRadius: thumbEl ? getComputedStyle(thumbEl).borderTopLeftRadius : null,
                thumbGroup: thumbGroup ? { padding: getComputedStyle(thumbGroup).padding, gap: getComputedStyle(thumbGroup).gap, children: thumbGroup.children.length } : null,
                swatches: {
                  count: swatches.length,
                  w: swatches[0] ? Math.round(swatches[0].getBoundingClientRect().width) : 0,
                  radius: swatches[0] ? getComputedStyle(swatches[0]).borderTopLeftRadius : null,
                  border: swatches[0] ? getComputedStyle(swatches[0]).borderTopWidth : null,
                },
                nameVal: fields[0]?.value ?? null,
                linkVal: fields[1]?.value ?? null,
                linkPh: fields[1]?.placeholder ?? null,
                fieldH: fieldEls[0] ? Math.round(fieldEls[0].getBoundingClientRect().height) : 0,
                fieldBg: fieldEls[0] ? getComputedStyle(fieldEls[0]).backgroundColor : null,
                namePad: fieldEls[0] ? getComputedStyle(fieldEls[0]).padding : null,
                linkPad: fieldEls[1] ? getComputedStyle(fieldEls[1]).padding : null,
                linkColor: fields[1] ? getComputedStyle(fields[1]).color : null,
                faintColor: probeColor('var(--text-faint)'),
                openLinkDisabled: !!pinned?.querySelector('.field.link .icon-btn:disabled'),
                chips: [...(custom?.querySelectorAll('.chips .app-tag') ?? [])].map((t) => t.textContent.trim()),
                chipAdd: !!custom?.querySelector('.chips .icon-btn:disabled'),
                chipsGap: chipsRow ? getComputedStyle(chipsRow).gap : null,
                chipH: chip ? Math.round(chip.getBoundingClientRect().height) : 0,
                chipColor: chip ? getComputedStyle(chip).color : null,
                labelColor: probeColor('var(--text-label)'),
                editDesc: editDesc ? { disabled: editDesc.disabled, h: Math.round(editDesc.getBoundingClientRect().height), label: editDesc.textContent.trim() } : null,
                infoBg: info ? getComputedStyle(info).backgroundColor : null,
                infoCols: info ? info.children.length : 0,
                info: pair,
                link: linkItem ? {
                  rows: linkItem.children.length,
                  segs: [...linkItem.querySelectorAll('.row1 .seg')].map((s) => s.textContent),
                  segCls: [...linkItem.querySelectorAll('.row1 .seg')].map((s) => s.className),
                  row2: linkItem.querySelector('.row2')?.textContent.trim(),
                  delW: Math.round(linkItem.querySelector('.del').getBoundingClientRect().width),
                } : null,
                linkSegs: [...p.querySelectorAll('.section-links .link-item')].map((it) => [...it.querySelectorAll('.row1 .seg')].map((s) => s.textContent)),
                emptyAdds: [...p.querySelectorAll('.section-empty')].map((s) => ({ title: s.querySelector('.section-header .title')?.textContent, disabled: !!s.querySelector('.icon-btn:disabled') })),
                bodyBox: { ...bx, sw: body.scrollWidth, cw: body.clientWidth },
                blockBoxes: blocks.map((b) => { const r = b.getBoundingClientRect(); return Math.round(r.left) + ',' + Math.round(r.right); }),
                tgBox: (() => { const r = thumbGroup.getBoundingClientRect(); return { l: Math.round(r.left), r: Math.round(r.right) }; })(),
                thumbW: Math.round(thumbEl.getBoundingClientRect().width),
                rootPad: getComputedStyle(pinned).padding,
                linkBadges: [...p.querySelectorAll('.section-links .link-item')].map((it) => {
                  const badge = it.querySelector('.row2 .origin-badge');
                  const r2 = it.querySelector('.row2');
                  return { badge: !!badge, size: badge ? Math.round(badge.getBoundingClientRect().width) : 0, justify: getComputedStyle(r2).justifyContent, padR: getComputedStyle(r2).paddingRight };
                }),
              };
            })()""")
            print("详情面板(文件):", json.dumps(detail, ensure_ascii=False))
            check("⑥ 区块标题序（pinned 缩略图恒第一 + 注册表 7 项，无归属档案）",
                  bool(detail and detail["pinned"] and detail["titles"] == ["(pinned)", "自定义数据", "基础信息", "副本", "链接区块", "关系", "音频", "提示词"] and not detail["hasArchive"]),
                  json.dumps(detail and detail["titles"], ensure_ascii=False))
            check("⑥-1 缩略图 contain + C2 底 + 圆角 8",
                  bool(detail and detail["thumbFit"] == "contain" and detail["thumbBg"] == "rgb(22, 23, 23)" and detail["thumbRadius"] == "8px"),
                  json.dumps(detail and {"fit": detail["thumbFit"], "bg": detail["thumbBg"], "r": detail["thumbRadius"]}))
            tg = (detail or {}).get("thumbGroup") or {}
            check("⑥-2 缩略图组 padding 16 / gap 12 / 4 子项（缩略图+颜色 bar+名称+链接）",
                  tg.get("padding") == "16px" and tg.get("gap") == "12px" and tg.get("children") == 4,
                  json.dumps(tg))
            sw = (detail or {}).get("swatches") or {}
            check("⑥-2 颜色 bar 恒显 7 占位块（16 圆 + 1px 描边）",
                  sw.get("count") == 7 and sw.get("w") == 16 and sw.get("radius") == "50%" and sw.get("border") == "1px",
                  json.dumps(sw))
            check("⑥-3 名称/链接输入框（值 + placeholder http:// + open-link disabled）",
                  bool(detail and detail["nameVal"] == "主角-立绘.webp" and detail["linkVal"] == "https://example.com/ref" and detail["linkPh"] == "http://" and detail["openLinkDisabled"]),
                  json.dumps(detail and {"n": detail["nameVal"], "l": detail["linkVal"], "ph": detail["linkPh"]}, ensure_ascii=False))
            check("⑥-4 输入框 36 高 + C2 底 + padding（名称 8 12 / 链接 3 3 3 12）+ 链接文字极淡",
                  bool(detail and detail["fieldH"] == 36 and detail["fieldBg"] == "rgb(22, 23, 23)"
                       and detail["namePad"] == "8px 12px" and detail["linkPad"] == "3px 3px 3px 12px"
                       and detail["linkColor"] == detail["faintColor"]),
                  json.dumps(detail and {"h": detail["fieldH"], "bg": detail["fieldBg"], "np": detail["namePad"], "lp": detail["linkPad"], "lc": detail["linkColor"]}))
            check("⑥-5 自定义数据带标题 + 区块 gap 8 + 内容容器 gap 12",
                  bool(detail and detail["customTitle"] == "自定义数据" and detail["customGap"] == "8px" and detail["bodyGap"] == "12px"),
                  json.dumps(detail and {"t": detail["customTitle"], "g": detail["customGap"], "bg": detail["bodyGap"]}, ensure_ascii=False))
            check("⑥-5 chips 行恒显（主角 + 行尾添加钮（6.1 已点亮，不再 disabled）+ gap 6 + chip 25 高标签色）",
                  bool(detail and detail["chips"] == ["主角"] and not detail["chipAdd"]
                       and detail["chipsGap"] == "6px" and detail["chipH"] == 25 and detail["chipColor"] == detail["labelColor"]),
                  json.dumps(detail and {"c": detail["chips"], "gap": detail["chipsGap"], "h": detail["chipH"], "color": detail["chipColor"]}, ensure_ascii=False))
            ed = (detail or {}).get("editDesc") or {}
            check("⑥-5 编辑描述通栏钮（36 高，6.1 已点亮）",
                  ed.get("disabled") is False and ed.get("h") == 36 and ed.get("label") == "编辑描述",
                  json.dumps(ed, ensure_ascii=False))
            info = (detail or {}).get("info") or {}
            check("⑥-6 基础信息 §1 C4 容器双列",
                  bool(detail and detail["infoBg"] == "rgb(33, 35, 35)" and detail["infoCols"] == 2),
                  json.dumps(detail and {"bg": detail["infoBg"], "cols": detail["infoCols"]}))
            check("⑥-6 基础信息实数据（尺寸 800 x 600 / 200.0 KB / WEBP）",
                  info.get("尺寸") == "800 x 600" and info.get("文件大小") == "200.0 KB" and info.get("格式") == "WEBP",
                  json.dumps(info, ensure_ascii=False))
            check("⑥-6 基础信息日期（YYYY/MM/DD HH:mm）",
                  info.get("添加日期") == "2026/08/15 10:20" and info.get("修改日期") == "2026/08/15 11:22",
                  json.dumps(info, ensure_ascii=False))
            lk = (detail or {}).get("link") or {}
            check("⑥-8 链接区块两行卡（两段：目录 / 设定集.档案 + 行2 显示名 + × 初始 0 宽）",
                  lk.get("rows") == 2 and lk.get("segs") == ["目录", "设定集.档案"] and lk.get("row2") == "主角-立绘.webp" and lk.get("delW") == 0,
                  json.dumps(lk, ensure_ascii=False))
            check("⑥-8 两段色档（区块名常态 / 所属目录标签色）+ 链接目录无后缀（子文件夹）",
                  lk.get("segCls") == ["seg block", "seg dir"] and (detail or {}).get("linkSegs") == [["目录", "设定集.档案"], ["目录", "子文件夹"]],
                  json.dumps({"cls": lk.get("segCls"), "items": (detail or {}).get("linkSegs")}, ensure_ascii=False))
            lb = (detail or {}).get("linkBadges") or []
            check("⑥-8b 行2 徽章（origin=true 出 15px / 链接不出）+ justify-between + 右 padding 4",
                  len(lb) == 2 and lb[0]["badge"] and lb[0]["size"] == 15 and not lb[1]["badge"]
                  and all(x["justify"] == "space-between" and x["padR"] == "4px" for x in lb),
                  json.dumps(lb))
            bx = (detail or {}).get("bodyBox") or {}
            want = f"{bx.get('l')},{bx.get('r')}"
            check("⑥-A 分隔线通栏（各块 + 缩略图组左右 x = 面板内容区）",
                  bool(detail and all(b == want for b in detail["blockBoxes"])
                       and detail["tgBox"] == {"l": bx["l"], "r": bx["r"]}),
                  json.dumps({"body": bx, "blocks": detail and detail["blockBoxes"], "tg": detail and detail["tgBox"]}))
            check("⑥-A 缩略图区块 root padding 归零（稿为准，不残留 pinned 旧口径）",
                  bool(detail and detail["rootPad"] == "0px"), str(detail and detail["rootPad"]))
            check("⑥-A 缩略图框宽 = 面板内容区宽 - 32（组 padding 16×2）",
                  bool(detail and bx and detail["thumbW"] == bx["r"] - bx["l"] - 32),
                  json.dumps({"thumb": detail and detail["thumbW"], "body": bx}))
            check("⑥-A 详情内容区无横向溢出（scrollWidth ≤ clientWidth）",
                  bool(bx and bx.get("sw") is not None and bx["sw"] <= bx["cw"]),
                  json.dumps(bx))
            check("⑥ 空态区块标题照常 + 禁用添加钮（副本/关系/音频/提示词）",
                  bool(detail and [e["title"] for e in detail["emptyAdds"]] == ["副本", "关系", "音频", "提示词"] and all(e["disabled"] for e in detail["emptyAdds"])),
                  json.dumps(detail and detail["emptyAdds"], ensure_ascii=False))
            await shot("archive-detail-file")

            # 选中档案卡片 → 档案形态
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            await asyncio.sleep(0.8)
            adetail = await ev("""(() => {
              const p = document.querySelector('.detail-panel');
              if (!p) return null;
              const pinned = p.querySelector('.section-pinned');
              const custom = p.querySelector('.section-custom');
              const info = p.querySelector('.section-info .info-card');
              const ks = info ? [...info.querySelectorAll('.col:first-child .k')].map((s) => s.textContent) : [];
              const vs = info ? [...info.querySelectorAll('.col.vals .v')].map((s) => [...s.children].map((c) => c.textContent).join(' ')) : [];
              const pair = {};
              ks.forEach((k, i) => (pair[k] = vs[i]));
              const badge = pinned?.querySelector('.fmt-badge');
              const desc = custom?.querySelector('.desc-row');
              const editDesc = custom?.querySelector('.edit-desc');
              const fields = [...(pinned?.querySelectorAll('.field input') ?? [])];
              return {
                badge: badge ? { text: badge.textContent, top: getComputedStyle(badge).top, left: getComputedStyle(badge).left } : null,
                cover: !!pinned?.querySelector('.thumb img'),
                nameVal: fields[0]?.value ?? null,
                linkVal: fields[1]?.value ?? null,
                linkPh: fields[1]?.placeholder ?? null,
                openLinkDisabled: !!pinned?.querySelector('.field.link .icon-btn:disabled'),
                props: [...(custom?.querySelectorAll('.chips .app-tag') ?? [])].map((t) => t.textContent.trim()),
                chipAdd: !!custom?.querySelector('.chips .icon-btn:disabled'),
                swatches: pinned?.querySelectorAll('.swatches').length ?? 0,
                customTitle: custom?.querySelector('.section-header .title')?.textContent ?? null,
                desc: desc ? { title: desc.querySelector('.d-label')?.textContent, content: desc.querySelector('.d-content')?.textContent } : null,
                editDesc: editDesc ? { disabled: editDesc.disabled, h: Math.round(editDesc.getBoundingClientRect().height) } : null,
                info: pair,
              };
            })()""")
            print("详情面板(档案):", json.dumps(adetail, ensure_ascii=False))
            check("⑥ 档案形态 = 文件详情同款（封面图 + 徽标 7,7 / 名称输入框 / 链接输入框 / 属性 chips + 钮 / 描述 / 编辑描述钮 36 / 自定义数据带标题；两钮 6.1 已点亮）",
                  bool(adetail and adetail["cover"] and adetail["badge"] and adetail["badge"]["text"] == "设定" and adetail["badge"]["top"] == "7px" and adetail["badge"]["left"] == "7px"
                       and adetail["nameVal"] == "设定集" and adetail["linkVal"] == "https://archive.ref" and adetail["linkPh"] == "http://" and adetail["openLinkDisabled"]
                       and adetail["props"] == ["奇幻"] and not adetail["chipAdd"]
                       and adetail["desc"] and adetail["desc"]["title"] == "来源:" and adetail["desc"]["content"] == "官方设定集"
                       and adetail["editDesc"] and not adetail["editDesc"]["disabled"] and adetail["editDesc"]["h"] == 36
                       and adetail["customTitle"] == "自定义数据"),
                  json.dumps(adetail, ensure_ascii=False))
            check("⑥ 档案无颜色 bar（其余与文件 pinned 同款）", bool(adetail and adetail["swatches"] == 0))
            check("⑥ 档案基础信息三行（文件数 2 / 占用空间 300.0 KB / 创建日期 2026/08/15 09:00；无名称行）",
                  bool(adetail and adetail["info"].get("文件数") == "2" and adetail["info"].get("占用空间") == "300.0 KB"
                       and adetail["info"].get("创建日期") == "2026/08/15 09:00" and "名称" not in adetail["info"]),
                  json.dumps(adetail and adetail["info"], ensure_ascii=False))
            await shot("archive-detail-archive")

            # —— ⑥-9 空区块展开/折叠零高度变化（档案形态：链接区块无 item）——
            h_open = await ev("""(() => {
              const sec = document.querySelector('.detail-panel .section-links');
              return sec ? { h: sec.getBoundingClientRect().height, items: sec.querySelectorAll('.link-item').length, body: !!sec.querySelector('.body'), sh: sec.scrollHeight } : null;
            })()""")
            await click("document.querySelector('.detail-panel .section-links .section-header .fold-area')")
            await asyncio.sleep(0.4)
            h_closed = await ev("(() => { const sec = document.querySelector('.detail-panel .section-links'); return sec ? { h: sec.getBoundingClientRect().height, sh: sec.scrollHeight } : null; })()")
            await click("document.querySelector('.detail-panel .section-links .section-header .fold-area')")
            await asyncio.sleep(0.4)
            print("空区块折叠:", json.dumps({"open": h_open, "closed": h_closed}))
            check("⑥-9 空链接区块：无 item 不渲染容器 + 展开/折叠零高度变化",
                  bool(h_open and h_open["items"] == 0 and not h_open["body"] and h_closed and h_open["h"] == h_closed["h"] and h_open["sh"] == h_closed["sh"]),
                  json.dumps({"open": h_open, "closed": h_closed}))

            # —— ⑦ 22 稿编辑数据区块面板 ——
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            panel = await ev("""(() => {
              const m = document.querySelector('.section-edit.app-menu');
              if (!m) return null;
              const probeColor = (v) => { const p = document.createElement('div'); p.style.color = v; document.body.appendChild(p); const c = getComputedStyle(p).color; p.remove(); return c; };
              // 参照：同写 1.5px border 的探针 div——无头 dpr=1 下 border-width 取使用值会被对齐（1.5→1），
              // 面板与参照一致即证明声明值同为 1.5px（非实现偏差）
              const probeBorder = (() => { const p = document.createElement('div'); p.style.border = '1.5px solid red'; document.body.appendChild(p); const c = getComputedStyle(p).borderTopWidth; p.remove(); return c; })();
              const rows = [...m.querySelectorAll('.se-row')];
              const first = rows[0], last = rows[rows.length - 1];
              const arrows = (r) => [...r.querySelectorAll('.sort-arrow')];
              return {
                width: m.style.width,
                border: getComputedStyle(m).borderTopWidth,
                borderStyle: getComputedStyle(m).borderTopStyle,
                borderColor: getComputedStyle(m).borderTopColor,
                lineActiveColor: probeColor('var(--line-active)'),
                probeBorder,
                rows: rows.length,
                titles: rows.map((r) => r.querySelector('.row-title')?.textContent),
                rowH: Math.round(first.getBoundingClientRect().height),
                arrowOrder: arrows(first).map((a) => (a.classList.contains('arrow-up') ? 'up' : 'down')),
                firstUp: (() => { const up = first.querySelector('.arrow-up'); return { disabled: up.disabled, color: getComputedStyle(up).color, opacity: getComputedStyle(up).opacity }; })(),
                lastDown: (() => { const dn = last.querySelector('.arrow-down'); return { disabled: dn.disabled, color: getComputedStyle(dn).color, opacity: getComputedStyle(dn).opacity }; })(),
                enabledColor: getComputedStyle(rows[1].querySelector('.arrow-up')).color,
                disabledColor: probeColor('var(--line-active)'),
                highlightColor: probeColor('var(--text-highlight)'),
                eyes: rows.map((r) => !!r.querySelector('.icon-btn.eye')),
              };
            })()""")
            print("22 稿面板:", json.dumps(panel, ensure_ascii=False))
            check("⑦ 面板 404 宽 + 1.5px 真 border（使用值与 1.5px 参照一致 + solid + 亮-选中色）",
                  bool(panel and panel["width"] == "404px" and panel["border"] == panel["probeBorder"] and panel["borderStyle"] == "solid" and panel["borderColor"] == panel["lineActiveColor"]),
                  json.dumps(panel and {"w": panel["width"], "b": panel["border"], "ref": panel["probeBorder"], "c": panel["borderColor"]}))
            check("⑦ 7 行按注册表序（无缩略图区块 / 无归属档案）", bool(panel and panel["rows"] == 7 and panel["titles"] == ["自定义数据", "基础信息", "副本", "链接区块", "关系", "音频", "提示词"]), json.dumps(panel and panel["titles"], ensure_ascii=False))
            check("⑦ 行高 52 + 行尾眼睛", bool(panel and panel["rowH"] == 52 and all(panel["eyes"])))
            check("⑦ 箭头序下移在前", bool(panel and panel["arrowOrder"] == ["down", "up"]))
            check("⑦ 首行禁上移 / 末行禁下移（禁用色 --line-active 不叠半透明）",
                  bool(panel and panel["firstUp"]["disabled"] and panel["firstUp"]["color"] == panel["disabledColor"] and panel["firstUp"]["opacity"] == "1"
                       and panel["lastDown"]["disabled"] and panel["lastDown"]["color"] == panel["disabledColor"]),
                  json.dumps(panel and {"up": panel["firstUp"], "dn": panel["lastDown"]}))
            check("⑦ 可用箭头高亮档", bool(panel and panel["enabledColor"] == panel["highlightColor"]))
            await shot("archive-section-edit")
            # 眼睛切换「关系」→ 标题禁用色 + 详情面板区块消失
            await click("[...document.querySelectorAll('.section-edit .se-row')].find((r) => r.querySelector('.row-title')?.textContent === '关系').querySelector('.icon-btn.eye')")
            await asyncio.sleep(0.6)
            eye = await ev("""(() => {
              const row = [...document.querySelectorAll('.section-edit .se-row')].find((r) => r.querySelector('.row-title')?.textContent === '关系');
              const titles = [...document.querySelectorAll('.detail-panel .detail-block .section-header .title')].map((t) => t.textContent);
              const probeColor = (v) => { const p = document.createElement('div'); p.style.color = v; document.body.appendChild(p); const c = getComputedStyle(p).color; p.remove(); return c; };
              return {
                off: row.classList.contains('off'),
                titleColor: getComputedStyle(row.querySelector('.row-title')).color,
                disabledColor: probeColor('var(--line-active)'),
                detailTitles: titles,
              };
            })()""")
            print("眼睛切换:", json.dumps(eye, ensure_ascii=False))
            check("⑦ 眼睛切换：隐藏行标题禁用色", bool(eye and eye["off"] and eye["titleColor"] == eye["disabledColor"]), json.dumps(eye))
            check("⑦ 眼睛切换联动详情面板（关系区块消失）", bool(eye and "关系" not in eye["detailTitles"] and "副本" in eye["detailTitles"]), json.dumps(eye and eye["detailTitles"], ensure_ascii=False))
            # 排序：首行「自定义数据」下移 → 详情面板区块序跟随
            await click("document.querySelector('.section-edit .se-row .sort-arrow.arrow-down')")
            await asyncio.sleep(0.6)
            moved = await ev("""({
              panel: [...document.querySelectorAll('.section-edit .se-row .row-title')].map((t) => t.textContent),
              detail: [...document.querySelectorAll('.detail-panel .detail-block .section-header .title')].map((t) => t.textContent),
            })""")
            print("排序后:", json.dumps(moved, ensure_ascii=False))
            check("⑦ 排序即时反映详情面板（基础信息提前）",
                  moved and moved["panel"][:3] == ["基础信息", "自定义数据", "副本"] and moved["detail"][:2] == ["基础信息", "自定义数据"],
                  json.dumps(moved, ensure_ascii=False))
            # 持久化：save_ui_settings 防抖 500ms 写回——本段选中档案卡片，
            # 面板编辑 = 档案套 detailSectionsArchive（2026-08-23 文件/档案各一套独立设置）
            await asyncio.sleep(1.0)
            saves = await ev("window.__calls.save_ui_settings.map((s) => JSON.parse(JSON.stringify(s.detailSectionsArchive ?? s.detailSections)))")
            print("持久化:", json.dumps(saves, ensure_ascii=False))
            last_save = saves[-1] if saves else None
            rel = next((s for s in last_save if s.get("id") == "relations"), None) if isinstance(last_save, list) else None
            check("⑦ save_ui_settings 持久化 detailSections（排序 + 显隐）",
                  bool(isinstance(last_save, list) and [s.get("id") for s in last_save][:2] == ["info", "custom"] and rel and rel.get("visible") is False),
                  json.dumps(last_save, ensure_ascii=False))
            # 行不可拖起（2026-08-15 用户定：面板内行拖拽拆除）：pointerdown 拖动后顺序不变
            drag_rect = await rect_of("document.querySelector('.section-edit .se-row')")
            before_rows = await ev("[...document.querySelectorAll('.section-edit .se-row .row-title')].map((t) => t.textContent)")
            if drag_rect:
                await mouse("mousePressed", drag_rect["x"], drag_rect["y"])
                await asyncio.sleep(0.1)
                for dy in (30, 70, 110):
                    await mouse("mouseMoved", drag_rect["x"], drag_rect["y"] + dy)
                    await asyncio.sleep(0.1)
                await mouse("mouseReleased", drag_rect["x"], drag_rect["y"] + 110)
                await asyncio.sleep(0.5)
            after_rows = await ev("[...document.querySelectorAll('.section-edit .se-row .row-title')].map((t) => t.textContent)")
            check("⑦ 行不可拖起（pointerdown 拖动后顺序不变）",
                  before_rows == after_rows,
                  json.dumps({"before": before_rows, "after": after_rows}, ensure_ascii=False))
            # 交换动画（30 稿同款 TransitionGroup）：规则存在 + 箭头排序时行带 se-swap-move 且过渡非 0
            anim_rule = await ev("""(() => {
              for (const sh of document.styleSheets) {
                try { for (const r of sh.cssRules) { if (r.selectorText && r.selectorText.startsWith('.se-swap-move') && r.style.transitionDuration && r.style.transitionDuration !== '0s') return r.style.transitionDuration; } } catch (e) {}
              }
              return null;
            })()""")
            # 手动点首行下移箭头（不用 click()——其内置 0.5s sleep 会错过 0.18s 动画窗口）
            ar = await rect_of("document.querySelector('.section-edit .se-row .sort-arrow.arrow-down')")
            if ar:
                await mouse("mousePressed", ar["x"], ar["y"]); await mouse("mouseReleased", ar["x"], ar["y"])
                await asyncio.sleep(0.05)
            anim_live = await ev("""(() => {
              const rows = [...document.querySelectorAll('.section-edit .se-row')];
              const moving = rows.filter((r) => r.className.includes('se-swap-move'));
              return { moving: moving.length, dur: moving[0] ? getComputedStyle(moving[0]).transitionDuration : null };
            })()""")
            print("交换动画:", json.dumps({"rule": anim_rule, "live": anim_live}))
            check("⑦ 箭头交换动画（.se-swap-move 规则 0.18s + 排序时行带 move 类过渡非 0）",
                  anim_rule == "0.18s" and bool(anim_live) and anim_live["moving"] > 0 and anim_live["dur"] not in (None, "0s"),
                  json.dumps({"rule": anim_rule, "live": anim_live}))
            await asyncio.sleep(0.5)  # 动画归位
            # 关面板（Esc）
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)

            # —— ⑦-B 面板内拖拽排序（详情面板拖「提示词」区块标题上移两位，越过 音频/链接区块）——
            # 实测（cdp-detail-drag-repro）：Esc 关菜单会同拍清选择（AppMenu capture 先关、
            # useGlobalKeys bubble 时已无 .app-menu 不让位）——先重选档案卡片恢复详情面板
            await ev("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')?.click()")
            await asyncio.sleep(0.8)
            # 滚到底（提示词在视口内）→ 压标题行 → 上移 120（落 mid2~mid3 区间）→ 松手
            await ev("document.querySelector('.detail-body').scrollTop = 99999")
            await asyncio.sleep(0.3)
            ph_rect = await rect_of("[...document.querySelectorAll('.detail-panel .detail-block')].find((b) => b.querySelector('.section-header .title')?.textContent === '提示词')?.querySelector('.section-header')")
            if ph_rect:
                await mouse("mousePressed", ph_rect["x"], ph_rect["y"])
                await asyncio.sleep(0.1)
                for dy in (-40, -80, -120):
                    await mouse("mouseMoved", ph_rect["x"], ph_rect["y"] + dy)
                    await asyncio.sleep(0.1)
                await mouse("mouseReleased", ph_rect["x"], ph_rect["y"] - 120)
                await asyncio.sleep(0.6)
            inpanel = await ev("""({
              detail: [...document.querySelectorAll('.detail-panel .detail-block .section-header .title')].map((t) => t.textContent),
            })""")
            print("面板内拖拽后:", json.dumps(inpanel, ensure_ascii=False))
            check("⑦-B 面板内拖拽标题排序落点（提示词 → 链接区块之前）",
                  bool(inpanel and inpanel["detail"] == ["自定义数据", "基础信息", "副本", "提示词", "链接区块", "音频"]),
                  json.dumps(inpanel, ensure_ascii=False))
            # 22 稿面板同步：重开面板，行序 = 面板内拖拽后的全量序（含隐藏的关系）
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            sync = await ev("[...document.querySelectorAll('.section-edit .se-row .row-title')].map((t) => t.textContent)")
            check("⑦-B 22 稿面板同步面板内拖拽序",
                  sync == ["自定义数据", "基础信息", "副本", "提示词", "链接区块", "关系", "音频"],
                  json.dumps(sync, ensure_ascii=False))
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)

            # —— Esc 关菜单不清选择（2026-08-16 根治：capture 打标 / bubble 读标记）——
            # 选中文件卡片 → 右键菜单 → Esc（菜单关、选择保留）→ 再 Esc（无菜单原语义清选择）
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(0.8)
            await rclick("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(0.6)
            assert await ev("!!document.querySelector('.app-menu')"), "右键菜单未打开"
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)
            esc1 = await ev("""({
              menu: !!document.querySelector('.app-menu'),
              selected: [...document.querySelectorAll('.asset-card.selected')].length,
              panel: !!document.querySelector('.detail-panel'),
            })""")
            check("Esc 关右键菜单不清选择（菜单关 + 选择保留）",
                  esc1["menu"] is False and esc1["selected"] >= 1 and esc1["panel"] is True,
                  json.dumps(esc1))
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)
            esc2 = await ev("({ selected: [...document.querySelectorAll('.asset-card.selected')].length, panel: !!document.querySelector('.detail-panel') })")
            check("Esc 再按清选择（无菜单维持原语义）",
                  esc2["selected"] == 0 and esc2["panel"] is False,
                  json.dumps(esc2))
            # 22 稿编辑数据区块面板同路径
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主角-立绘'))")
            await asyncio.sleep(0.8)
            await click("[...document.querySelectorAll('.sidebar-right .panel-toolbar .icon-btn')][1]")
            await asyncio.sleep(0.6)
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)
            esc3 = await ev("({ menu: !!document.querySelector('.app-menu'), selected: [...document.querySelectorAll('.asset-card.selected')].length, panel: !!document.querySelector('.detail-panel') })")
            check("Esc 关 22 稿面板不清选择（菜单关 + 选择保留）",
                  esc3["menu"] is False and esc3["selected"] >= 1 and esc3["panel"] is True,
                  json.dumps(esc3))
            await send(ws, "Input.dispatchKeyEvent", {"type": "rawKeyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
            await asyncio.sleep(0.5)

            # —— 回收站视口选中详情（2026-08-16 用户拍板）——
            await click("[...document.querySelectorAll('.tree-item')].find((t) => t.textContent.includes('回收站'))?.querySelector('.body')")
            await asyncio.sleep(0.8)
            # JS click（回收站卡片矮，中心点落在 .card-info 不选中区——Eagle 口径文字区不选中）
            await ev("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('废弃稿'))?.click()")
            await asyncio.sleep(1.0)
            rec = await ev("""(() => {
              const p = document.querySelector('.detail-panel');
              if (!p) return null;
              const pinned = p.querySelector('.section-pinned');
              const info = p.querySelector('.section-info .info-card');
              const ks = info ? [...info.querySelectorAll('.col:first-child .k')].map((s) => s.textContent) : [];
              const vs = info ? [...info.querySelectorAll('.col.vals .v')].map((s) => [...s.children].map((c) => c.textContent).join(' ')) : [];
              const pair = {};
              ks.forEach((k, i) => (pair[k] = vs[i]));
              const linkItem = p.querySelector('.section-links .link-item');
              return {
                thumb: !!pinned?.querySelector('.thumb img'),
                name: pinned?.querySelector('.field input')?.value ?? null,
                info: pair,
                linkSegs: linkItem ? [...linkItem.querySelectorAll('.row1 .seg')].map((s) => s.textContent) : null,
                linkRow2: linkItem?.querySelector('.row2')?.textContent.trim() ?? null,
              };
            })()""")
            print("回收站详情:", json.dumps(rec, ensure_ascii=False))
            check("回收站选中详情（pinned 缩略图 + 名称 + 基础信息实数据）",
                  bool(rec and rec["thumb"] and rec["name"] == "废弃稿.webp"
                       and rec["info"].get("尺寸") == "320 x 240" and rec["info"].get("文件大小") == "50.0 KB" and rec["info"].get("格式") == "WEBP"),
                  json.dumps(rec, ensure_ascii=False))
            check("回收站链接区块（回收站 / 原位置 区块·路径，PRD 02-15）",
                  bool(rec and rec["linkSegs"] == ["回收站", "目录 / 素材 / 设定集"] and rec["linkRow2"] == "废弃稿.webp"),
                  json.dumps(rec and {"segs": rec["linkSegs"], "row2": rec["linkRow2"]}, ensure_ascii=False))

            # 点空白 = 空面板（grid-viewport 中下部空白区，避开右侧滚动条轨）
            grid_rect = await ev("(() => { const g = document.querySelector('.grid-viewport'); if (!g) return null; const b = g.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.bottom - 40 }; })()")
            if grid_rect:
                await mouse("mousePressed", grid_rect["x"], grid_rect["y"]); await mouse("mouseReleased", grid_rect["x"], grid_rect["y"])
                await asyncio.sleep(0.8)
            empty = await ev("!document.querySelector('.detail-panel')")
            check("⑥ 无选中 = 空面板", empty)

            # —— ⑤b 解散设定集（最后一个档案）→ 档案区无内容整块不渲染 ——
            # 先回「素材」视口（回收站详情测试停在回收站视口）
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.8)
            await rclick("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            await click_menu("解散档案为文件夹")
            await asyncio.sleep(0.8)
            gone = await ev("!document.querySelector('.archive-block')")
            check("⑤ 档案区无内容整块不渲染", gone)

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 树档案形态 / 三区 + 档案卡片 / 04 稿菜单 + 新建档案先建后命名 / 25 稿两段式转换（取消+确认+直接转）/ 解散 + 档案区空不渲染 / 详情区块体系（pinned 缩略图独立置顶 + 注册表 7 区块 + 两段化链接 + 空态折叠零高度 + 通栏）+ 档案形态 + 空面板 / 22 稿面板 7 行布局联动持久化 + 行不可拖 + 箭头交换动画 + 面板内拖拽排序 / Esc 关菜单不清选择 + 回收站选中详情 全部通过")
            return 0
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


sys.exit(asyncio.run(main()))
