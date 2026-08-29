# CDP 实测：6.2 三轮 · 文件链接（17 稿链接管理弹窗）+ 链接音频（voice 关系 + 音频区块）
# 2026-08-24 用户定案：弹窗 = 管理一个文件的链接分布（左列 = 侧栏文件树照搬 + 未分类特殊组；
# 右列平铺多行；√ 增删草稿式确认一次提交；移除 = 直接删记录）；链接音频 = 多选累积过滤非音频。
# 桩带可变 placements / relations 表（sync_asset_links / add_relations 真改数据，list 深拷贝，踩坑 053）：
# L1 弹窗打开（Ctrl+J）：标题 / 表头 / 区块组（含未分类特殊组）/ 有链接行（★ + 链接徽章）
# L2 淡 √ 添加草稿（无链接行 → 默认名名称框出现）
# L3 名称框点击 = 编辑态输入框 → 改名入草稿
# L4 亮 √ 取消勾选（origin 行 → 名称框消失）
# L5 确认 = sync_asset_links 三组全参数断言 + 弹窗关 + 详情链接区块同步
# L6 仅显示已选项过滤
# L7 Esc 取消零写入
# V1 链接音频：详情音频区块添加钮 → 选择模式过滤非音频（fla 卡与档案区不渲染）→ 确认 add_relations kind=voice
# V2 音频区块双向显示（使用方 = 音频 item 播放钮；音频侧 = 使用方文件卡 + 标题「使用者」无添加钮，方案 1 单向）
# V3 × 删除链接音频
# 截图留档 tools/style-preview/：link-manage / voice-picker。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9366
URL = "http://localhost:5373"

STUB = r"""
window.__calls = { sync_asset_links: [], add_relations: [], remove_relation: [], set_placement_origin: [] };
window.__relId = 0;
window.__cbs = {};
window.__cbId = 0;
(function () {
  const LIB = "动画资产";
  const DB = {
    blocks: [{ id: "dir", name: "目录", icon: null }, { id: "blk2", name: "项目", icon: null }],
    folders: [
      { id: "F-DIR", name: "素材", parent: null, block: "dir", kind: null, color: "red" },
      { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
      { id: "F-ARC", name: "雪天", parent: "F-DIR", kind: "archive" },
      { id: "F-P2", name: "工程", parent: null, block: "blk2", kind: null },
    ],
    assets: {
      A1: { id: "A1", name: "雪牧", originName: "雪牧.fla", ext: "fla", size: 3145728, width: null, height: null, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], colors: [], links: [], note: "" },
      A2: { id: "A2", name: "主题曲", originName: "主题曲.mp3", ext: "mp3", size: 2097152, width: 640, height: 272, duration: 12.5, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], colors: [], links: [], note: "" },
      A3: { id: "A3", name: "阿拉蕾", originName: "阿拉蕾.fla", ext: "fla", size: 1048576, width: null, height: null, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], colors: [], links: [], note: "" },
    },
    placements: [
      { assetId: "A1", folderId: "F-SUB", name: "雪牧.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A1", folderId: "F-ARC", name: "雪牧-链.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A2", folderId: "F-SUB", name: "主题曲.mp3", w: 640, h: 272, ext: "mp3", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: 12.5, bpm: null, importedFrom: null },
      { assetId: "A3", folderId: "F-SUB", name: "阿拉蕾.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A3", folderId: "F-DIR", name: "阿拉蕾-链.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ],
    archives: {
      "F-ARC": { folderId: "F-ARC", badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", coverPath: null },
    },
    relations: [],
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
    block: f.parent === null ? (f.block ?? "dir") : null, icon: null, color: f.color ?? null,
    children: childrenOf(f.id).map(nodeOf),
  });
  const tree = () => ({ blocks: DB.blocks, nodes: childrenOf(null).map(nodeOf) });
  const nameOf = (kind, id) => (kind === "asset" ? DB.assets[id]?.name : byId(id)?.name) ?? id;
  const refOf = (kind, id) => ({ lib: LIB, kind, id, name: nameOf(kind, id) });
  const pathOf = (id) => {
    const parts = [];
    let cur = byId(id);
    while (cur) { parts.unshift(cur.name); cur = cur.parent ? byId(cur.parent) : null; }
    return parts;
  };
  const blockNameOf = (id) => {
    let cur = byId(id);
    while (cur && cur.parent) cur = byId(cur.parent);
    return (cur && cur.block) || "dir";
  };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    convertFileSrc: (p) => encodeURI("/@fs/" + p),
    transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
    invoke: (cmd, args) => {
      const R = {
        get_current_library: { name: LIB, path: "E:/T/S/动画资产", assetCount: 3, folderCount: 4 },
        get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
        get_ui_settings: {},
        list_placements: () => JSON.parse(JSON.stringify(DB.placements)),
        list_folder_tree: () => tree(),
        list_archives: () => Object.values(DB.archives),
        list_recycle: [],
        list_recent_assets: [], list_inbox_snapshot: [],
        get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
        list_space_libraries: ["动画资产"],
        list_recent_libraries: [],
        list_prop_pool: [],
        // 深拷贝斩断共享引用（踩坑 053：原样返回 = 前端 state 与桩共享，绕过响应式）
        list_relations: () => JSON.parse(JSON.stringify(DB.relations)),
      };
      if (cmd === "get_archive_data") {
        const rec = DB.archives[args.folderId];
        const inFolder = DB.placements.filter((p) => p.folderId === args.folderId);
        const uniq = new Set(inFolder.map((p) => p.assetId));
        const totalSize = [...uniq].reduce((s, a) => s + (DB.assets[a]?.size ?? 0), 0);
        const base = rec ?? { folderId: args.folderId, badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", coverPath: null };
        return Promise.resolve({ ...base, fileCount: inFolder.length, totalSize });
      }
      if (cmd === "get_asset_detail") {
        const meta = DB.assets[args.assetId];
        if (!meta) return Promise.reject("资源不存在，可能已被删除");
        const links = DB.placements.filter((p) => p.assetId === args.assetId).map((p) => ({
          folderId: p.folderId === "root" ? null : p.folderId,
          blockName: p.folderId === "root" ? "目录" : ({ dir: "目录", blk2: "项目" })[blockNameOf(p.folderId)] ?? "目录",
          path: p.folderId === "root" ? ["未分类"] : pathOf(p.folderId),
          name: p.name, origin: p.origin,
        }));
        return Promise.resolve({
          assetId: meta.id, originName: meta.originName, ext: meta.ext, size: meta.size,
          width: meta.width, height: meta.height, duration: meta.duration,
          importedAt: meta.importedAt, modifiedAt: meta.modifiedAt,
          tags: meta.tags, descriptions: meta.descriptions, colors: [], links: meta.links, note: meta.note,
          placements: links, archives: [],
        });
      }
      if (cmd === "sync_asset_links") {
        window.__calls.sync_asset_links.push(JSON.parse(JSON.stringify(args)));
        for (const r of args.removes) {
          const i = DB.placements.findIndex((p) => p.assetId === args.assetId && p.folderId === r.folderId && p.origin === r.origin && (!r.name || p.name === r.name));
          if (i >= 0) DB.placements.splice(i, 1);
        }
        for (const r of args.renames) {
          const p = DB.placements.find((p) => p.assetId === args.assetId && p.folderId === r.folderId && p.origin === r.origin && (!r.oldName || p.name === r.oldName));
          if (p) p.name = r.newName;
        }
        for (const a of args.adds) {
          DB.placements.unshift({ assetId: args.assetId, folderId: a.folderId, name: a.name || DB.assets[args.assetId].name, w: null, h: null, ext: DB.assets[args.assetId].ext, thumbPath: null, thumbKind: null, origin: false, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null });
        }
        return Promise.resolve(args.adds.length + args.removes.length + args.renames.length);
      }
      if (cmd === "add_relations") {
        window.__calls.add_relations.push(JSON.parse(JSON.stringify(args)));
        const rels = args.items.map((it) => ({ id: "RL-" + (++window.__relId), from: refOf(args.fromKind, args.fromId), to: refOf(it.toKind, it.toId), kind: args.kind ?? "free", label: it.label, reverseLabel: it.reverseLabel ?? null }));
        DB.relations.push(...rels);
        return Promise.resolve(JSON.parse(JSON.stringify(rels)));
      }
      if (cmd === "remove_relation") {
        window.__calls.remove_relation.push(JSON.parse(JSON.stringify(args)));
        DB.relations = DB.relations.filter((r) => r.id !== args.id);
        return Promise.resolve(null);
      }
      if (cmd === "get_media_source") return Promise.resolve({ path: "E:/T/S/动画资产/assets/A2.info/主题曲.mp3", kind: "origin" });
      if (cmd === "set_placement_origin") {
        window.__calls.set_placement_origin.push(JSON.parse(JSON.stringify(args)));
        for (const p of DB.placements) if (p.assetId === args.assetId) p.origin = false;
        const t = DB.placements.find((p) => p.assetId === args.assetId && p.folderId === args.folderId && (!args.name || p.name === args.name));
        if (t) t.origin = true;
        return Promise.resolve(null);
      }
      const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
      return Promise.resolve(typeof v === "function" ? v() : v);
    },
  };
})();
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

            async def hover(x, y):
                # 纯悬停（buttons 0）：左键按住移动 = 拖拽，不触发 mouseenter（W5 实证）
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "button": "none", "buttons": 0, "x": x, "y": y}, session=sess)

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

            # —— 初始：点雪牧卡出详情 ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            await asyncio.sleep(0.8)

            # —— L1 Ctrl+J 打开链接管理弹窗 ——
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true, bubbles: true }))")
            await asyncio.sleep(0.8)
            l1 = await ev("""(() => {
              const m = document.querySelector('.app-modal');
              if (!m) return null;
              const groups = [...m.querySelectorAll('.block-group .group-title')].map((g) => g.textContent);
              const lines = [...m.querySelectorAll('.line')].map((l) => ({
                folder: l.querySelector('.folder-cell .fname')?.textContent ?? null,
                name: l.querySelector('.link-cell .lname')?.textContent ?? null,
                star: !!l.querySelector('.link-cell .badge:not(.link)'),
                linkBadge: !!l.querySelector('.link-cell .badge.link'),
                ghost: !!l.querySelector('.check.ghost'),
              }));
              return {
                title: m.querySelector('.titlebar .title')?.textContent ?? null,
                heads: [...m.querySelectorAll('.head-cell')].map((h) => h.textContent),
                onlyChecked: m.querySelector('.only-checked')?.textContent ?? null,
                groups, lines,
                tint: getComputedStyle([...m.querySelectorAll('.folder-cell')].find((c) => c.textContent.includes('素材'))).backgroundColor,
              };
            })()""")
            check("L1 弹窗结构：标题 / 表头路径|名称 / 底栏仅显示已选项",
                  bool(l1 and l1["title"] == "链接管理" and l1["heads"] == ["路径", "名称"] and "仅显示已选项" in (l1["onlyChecked"] or "")),
                  json.dumps({k: l1[k] for k in ("title", "heads", "onlyChecked")} if l1 else None, ensure_ascii=False))
            check("L1 区块组 = 目录 / 项目 / 未分类（特殊组），组头带数量",
                  bool(l1 and len(l1["groups"]) == 3 and l1["groups"][0].startswith("目录 - ") and l1["groups"][1].startswith("项目 - ") and l1["groups"][2].startswith("未分类")),
                  json.dumps(l1["groups"] if l1 else None, ensure_ascii=False))
            sub = [l for l in (l1["lines"] if l1 else []) if l["folder"] == "子文件夹"]
            arc = [l for l in (l1["lines"] if l1 else []) if l["folder"] == "雪天"]
            check("L1 有链接行：子文件夹 = 雪牧.fla + ★ 原始徽章；雪天 = 雪牧-链.fla + 链接徽章",
                  bool(sub and sub[0]["name"] == "雪牧.fla" and sub[0]["star"] and arc and arc[0]["name"] == "雪牧-链.fla" and arc[0]["linkBadge"]),
                  json.dumps({"sub": sub, "arc": arc}, ensure_ascii=False))
            check("L1 文件夹格颜色 = 自定义色（素材 red 档 --custom-red #402122）",
                  bool(l1 and l1["tint"] == "rgb(64, 33, 34)"), str(l1 and l1["tint"]))
            await shot("link-manage")

            # —— L2 淡 √ 添加草稿（工程行无链接） ——
            await click("[...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '工程').querySelector('.check.ghost')")
            l2 = await ev("""(() => {
              const l = [...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '工程');
              return { name: l?.querySelector('.link-cell .lname')?.textContent ?? null, on: !!l?.querySelector('.check.on') };
            })()""")
            check("L2 淡 √ → 添加草稿（默认名 雪牧.fla + 亮 √）", bool(l2 and l2["name"] == "雪牧.fla" and l2["on"]), json.dumps(l2, ensure_ascii=False))

            # —— L3 名称框点击 = 编辑态 → 改名（雪天行 雪牧-链.fla → 雪牧-改.fla） ——
            await click("[...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '雪天').querySelector('.link-cell .lname')")
            l3a = await ev("""(() => {
              const l = [...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '雪天');
              return { editing: !!l?.querySelector('.link-cell.editing'), hasInput: !!l?.querySelector('.name-edit') };
            })()""")
            print("L3 点击后现场:", l3a)
            l3e = await ev("""(() => {
              const input = [...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '雪天').querySelector('.name-edit');
              if (!input) return { err: 'no-input' };
              input.value = '雪牧-改.fla';
              input.dispatchEvent(new Event('input', { bubbles: true }));
              input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
              return { ok: true };
            })()""")
            print("L3 dispatch:", l3e)
            await asyncio.sleep(0.4)
            l3 = await ev("""(() => {
              const l = [...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '雪天');
              return { name: l?.querySelector('.link-cell .lname')?.textContent ?? null, editing: !!l?.querySelector('.link-cell.editing') };
            })()""")
            check("L3 名称框编辑态改名入草稿（雪牧-改.fla）", bool(l3 and l3["name"] == "雪牧-改.fla" and not l3["editing"]), json.dumps(l3, ensure_ascii=False))

            # —— L4 亮 √ 取消勾选（子文件夹 origin 行） ——
            await click("[...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '子文件夹').querySelector('.check.on')")
            l4 = await ev("""(() => {
              const l = [...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '子文件夹');
              return { name: l?.querySelector('.link-cell .lname')?.textContent ?? null, ghost: !!l?.querySelector('.check.ghost') };
            })()""")
            check("L4 亮 √ → 取消勾选（origin 行名称框消失回 ghost 态）", bool(l4 and l4["name"] is None and l4["ghost"]), json.dumps(l4, ensure_ascii=False))

            # —— L5 确认 = sync_asset_links 三组断言 ——
            await click("document.querySelector('.app-modal .actions .app-btn.primary')")
            await asyncio.sleep(0.8)
            l5 = await ev("""(() => ({
              calls: window.__calls.sync_asset_links,
              modal: !!document.querySelector('.app-modal'),
              detailLinks: [...document.querySelectorAll('.detail-panel .section-links .link-item')].map((it) => ({
                dir: it.querySelector('.seg.dir')?.textContent, name: it.querySelector('.file-name')?.textContent })),
            }))()""")
            c0 = l5 and l5["calls"] and l5["calls"][0]
            check("L5 确认 → sync_asset_links（adds 工程 / removes 子文件夹 origin / renames 雪天行带 oldName）",
                  bool(c0 and len(l5["calls"]) == 1 and c0["assetId"] == "A1"
                       and len(c0["adds"]) == 1 and c0["adds"][0]["folderId"] == "F-P2" and c0["adds"][0]["name"] == "雪牧.fla"
                       and len(c0["removes"]) == 1 and c0["removes"][0]["folderId"] == "F-SUB" and c0["removes"][0]["origin"] == True and c0["removes"][0]["name"] == "雪牧.fla"
                       and len(c0["renames"]) == 1 and c0["renames"][0]["folderId"] == "F-ARC" and c0["renames"][0]["oldName"] == "雪牧-链.fla" and c0["renames"][0]["newName"] == "雪牧-改.fla"
                       and not l5["modal"]),
                  json.dumps(c0, ensure_ascii=False))
            # origin 行被移除后选中 key 失效（与"删除当前选中位置"同口径）——改选剩余位置卡（雪天=雪牧-改.fla）再断言详情
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧-改'))")
            await asyncio.sleep(0.8)
            l5b = await ev("""(() => [...document.querySelectorAll('.detail-panel .section-links .link-item')].map((it) => ({
              dir: it.querySelector('.seg.dir')?.textContent, name: it.querySelector('.file-name')?.textContent })))()""")
            check("L5 详情链接区块同步（剩 雪天=雪牧-改.fla + 工程=雪牧.fla 两条）",
                  bool(l5b and len(l5b) == 2
                       and any(d["dir"] == "雪天.档案" and d["name"] == "雪牧-改.fla" for d in l5b)
                       and any(d["dir"] == "工程" and d["name"] == "雪牧.fla" for d in l5b)),
                  json.dumps(l5b, ensure_ascii=False))

            # —— L6 仅显示已选项（重开弹窗 → 勾选 → 只剩有链接行） ——
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true, bubbles: true }))")
            await asyncio.sleep(0.8)
            await click("document.querySelector('.app-modal .only-checked')")
            l6 = await ev("""(() => ({
              folders: [...document.querySelectorAll('.app-modal .line .folder-cell .fname')].map((f) => f.textContent),
              lines: document.querySelectorAll('.app-modal .line').length,
            }))()""")
            check("L6 仅显示已选项 = 只留有链接的文件夹行（雪天 + 工程）",
                  bool(l6 and l6["lines"] == 2 and l6["folders"] == ["雪天", "工程"]),
                  json.dumps(l6, ensure_ascii=False))

            # —— L7 Esc 取消零写入 ——
            await click("[...document.querySelectorAll('.line')].find((l) => l.querySelector('.folder-cell .fname')?.textContent === '雪天').querySelector('.check.on')")
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(0.6)
            l7 = await ev("""(() => ({
              modal: !!document.querySelector('.app-modal'),
              calls: window.__calls.sync_asset_links.length,
            }))()""")
            check("L7 Esc = 取消全部放弃（弹窗关 + 零新写入）", bool(l7 and not l7["modal"] and l7["calls"] == 1), json.dumps(l7))

            # —— V1 链接音频：详情音频区块添加钮 → 过滤非音频 → 确认 voice ——
            # Esc 关弹窗可能连带清选择（全局 Esc 守卫竞争），重选雪牧-改卡确保详情在
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧-改'))")
            await asyncio.sleep(0.8)
            await click("document.querySelector('.detail-panel .section-audio .icon-btn')")
            await asyncio.sleep(0.6)
            v1 = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              banner: document.querySelector('.banner .text')?.textContent ?? null,
              cards: [...document.querySelectorAll('.asset-card')].map((c) => c.textContent),
              archives: document.querySelectorAll('.archive-card').length,
              placeholder: document.querySelector('.picker-panel .name-field.placeholder')?.placeholder ?? null,
            }))()""")
            check("V1 选择模式文件模板：横幅 + 过滤非音频（只剩主题曲.mp3 卡，档案区不渲染）",
                  bool(v1 and v1["picking"] and v1["banner"] == "选择音频，为 [雪牧-改.fla] 链接音频"
                       and len(v1["cards"]) == 1 and "主题曲" in v1["cards"][0] and v1["archives"] == 0),
                  json.dumps({"banner": v1 and v1["banner"], "cards": v1 and v1["cards"], "archives": v1 and v1["archives"]}, ensure_ascii=False))
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主题曲'))")
            v1b = await ev("""(() => ({
              rows: document.querySelectorAll('.picker-panel .file-row').length,
              name: document.querySelector('.picker-panel .file-row .name-field:not(.placeholder)')?.value ?? null,
              confirm: document.querySelector('.picker-panel .pick-actions .app-btn.primary')?.textContent,
            }))()""")
            check("V1 备选区行 = 名称输入框（默认源名 主题曲.mp3）+ 确认带数量",
                  bool(v1b and v1b["rows"] == 1 and v1b["name"] == "主题曲.mp3" and v1b["confirm"] == "确认 (1)"),
                  json.dumps(v1b, ensure_ascii=False))
            await shot("voice-picker")
            await click("document.querySelector('.picker-panel .pick-actions .app-btn.primary')")
            await asyncio.sleep(0.8)
            v1c = await ev("""(() => ({
              calls: window.__calls.add_relations,
              picking: document.querySelector('.window').classList.contains('picking'),
              secTitle: document.querySelector('.detail-panel .section-audio .section-header .title')?.textContent ?? null,
              addBtn: !!document.querySelector('.detail-panel .section-audio .section-header .icon-btn'),
              audioItems: [...document.querySelectorAll('.detail-panel .section-audio .audio-item')].map((it) => ({
                play: !!it.querySelector('.play-btn'), name: it.querySelector('.name')?.textContent, wave: !!it.querySelector('.wave') })),
            }))()""")
            vc = v1c and v1c["calls"] and v1c["calls"][0]
            check("V1 确认 → add_relations kind=voice（label 空 / reverseLabel null / toId A2）",
                  bool(vc and vc["kind"] == "voice" and vc["fromId"] == "A1"
                       and len(vc["items"]) == 1 and vc["items"][0]["toId"] == "A2"
                       and vc["items"][0]["label"] == "" and vc["items"][0]["reverseLabel"] is None
                       and not v1c["picking"]),
                  json.dumps(vc, ensure_ascii=False))
            check("V1 详情音频区块 = 音频 item（播放钮 + 名称 主题曲 + 波形位）",
                  bool(v1c and len(v1c["audioItems"]) == 1 and v1c["audioItems"][0]["play"] and v1c["audioItems"][0]["name"] == "主题曲"),
                  json.dumps(v1c["audioItems"] if v1c else None, ensure_ascii=False))
            check("V1 使用方视角：标题「音频」+ 有添加钮",
                  bool(v1c and v1c["secTitle"] == "音频" and v1c["addBtn"]),
                  json.dumps({"title": v1c and v1c["secTitle"], "addBtn": v1c and v1c["addBtn"]}, ensure_ascii=False))

            # —— V2 双向显示：选主题曲卡 → 音频区块 = 使用方文件卡；音频视角标题「使用者」+ 无添加钮（方案 1）——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('主题曲'))")
            await asyncio.sleep(0.8)
            v2 = await ev("""(() => ({
              chip: document.querySelector('.detail-panel .section-audio .file-chip .name')?.textContent ?? null,
              audioItem: !!document.querySelector('.detail-panel .section-audio .audio-item'),
              relItems: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
              secTitle: document.querySelector('.detail-panel .section-audio .section-header .title')?.textContent ?? null,
              addBtn: !!document.querySelector('.detail-panel .section-audio .section-header .icon-btn'),
            }))()""")
            check("V2 双向显示：主题曲详情 = 使用方文件卡 雪牧（ref 名快照，非音频 item）",
                  bool(v2 and v2["chip"] == "雪牧" and not v2["audioItem"]),
                  json.dumps(v2, ensure_ascii=False))
            check("V2 音频视角：标题「使用者」+ 无添加钮（方案 1 单向，防『音频的音频』递归）",
                  bool(v2 and v2["secTitle"] == "使用者" and not v2["addBtn"]),
                  json.dumps({"title": v2 and v2["secTitle"], "addBtn": v2 and v2["addBtn"]}, ensure_ascii=False))
            check("W1 voice 不泄漏关系区块（voice 存在时关系区块 0 item）",
                  bool(v2 and v2["relItems"] == 0), json.dumps(v2, ensure_ascii=False))

            # —— V3 × 删除链接音频 ——
            await ev("document.querySelector('.detail-panel .section-audio .audio-row .del')?.click()")
            await asyncio.sleep(0.6)
            v3 = await ev("""(() => ({
              calls: window.__calls.remove_relation,
              rows: document.querySelectorAll('.detail-panel .section-audio .audio-row').length,
            }))()""")
            check("V3 × → remove_relation(RL-1) + 音频区块空",
                  bool(v3 and v3["calls"] == [{"id": "RL-1"}] and v3["rows"] == 0),
                  json.dumps(v3, ensure_ascii=False))

            # —— 走查五笔（6.2 四轮，2026-08-24 用户实机反馈）——
            # W2 链接区块：当前目录 item 恒第一行 + 点 item 跳转（L5 后 A1 = F-ARC 雪牧-改 + F-P2 雪牧.fla）
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧-改'))")
            await asyncio.sleep(0.8)
            w2 = await ev("""(() => [...document.querySelectorAll('.detail-panel .section-links .link-item')].map((it) => ({
              dir: it.querySelector('.seg.dir')?.textContent, name: it.querySelector('.file-name')?.textContent })))()""")
            check("W2 链接区块当前目录 item 恒第一行（雪天.档案 在上）",
                  bool(w2 and len(w2) == 2 and w2[0]["dir"] == "雪天.档案" and w2[0]["name"] == "雪牧-改.fla" and w2[1]["dir"] == "工程"),
                  json.dumps(w2, ensure_ascii=False))
            await click("[...document.querySelectorAll('.detail-panel .section-links .link-item')].find((it) => it.textContent.includes('工程'))")
            await asyncio.sleep(0.8)
            w2b = await ev("""(() => ({
              pinned: document.querySelector('.detail-panel .pinned input')?.value ?? null,
              selected: [...document.querySelectorAll('.asset-card.selected')].map((c) => c.textContent),
            }))()""")
            check("W2 点 item 跳转：工程视口 + 选中 雪牧.fla 卡出详情",
                  bool(w2b and w2b["pinned"] == "雪牧.fla" and len(w2b["selected"]) > 0 and "雪牧.fla" in w2b["selected"][0]),
                  json.dumps(w2b, ensure_ascii=False))

            # 面包屑后退回 quick:all（W2 跳转经 openView 入历史）
            await click("document.querySelector('.crumb-group .icon-btn:not(.rot-180):not(.archive-panel-toggle)')")
            await asyncio.sleep(0.7)

            # W3 徽章规则：多位置 origin ★ / 链接 icon；单位置无徽章
            w3 = await ev("""(() => {
              const badgeOf = (txt) => {
                const c = [...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes(txt));
                if (!c) return null;
                const b = c.querySelector('.card-badges');
                return { has: !!b, kind: b?.dataset?.badge ?? null };
              };
              return { a3origin: badgeOf('阿拉蕾.fla'), a3link: badgeOf('阿拉蕾-链.fla'), a2: badgeOf('主题曲') };
            })()""")
            check("W3 徽章规则：阿拉蕾 origin 卡 ★ / 阿拉蕾-链 链接卡 icon / 主题曲单位置无徽章",
                  bool(w3 and w3["a3origin"] and w3["a3origin"]["has"] and w3["a3origin"]["kind"] == "origin"
                       and w3["a3link"] and w3["a3link"]["has"] and w3["a3link"]["kind"] == "link"
                       and w3["a2"] and not w3["a2"]["has"]),
                  json.dumps(w3, ensure_ascii=False))

            # W4 右键菜单「打开文件链接所在位置」→ 子菜单点工程 → 跳转 + 选中
            r = await rect_of("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧-改'))")
            await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
            await asyncio.sleep(0.6)
            r2 = await rect_of("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('打开文件链接所在位置'))")
            if r2:
                await hover(r2["x"], r2["y"])
                await asyncio.sleep(0.7)
                await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.trim() === '工程')")
                await asyncio.sleep(0.8)
            w4 = await ev("""(() => ({
              pinned: document.querySelector('.detail-panel .pinned input')?.value ?? null,
              selected: [...document.querySelectorAll('.asset-card.selected')].map((c) => c.textContent),
            }))()""")
            check("W4 打开文件链接所在位置 → 工程视口 + 选中 雪牧.fla",
                  bool(w4 and w4["pinned"] == "雪牧.fla" and len(w4["selected"]) > 0 and "雪牧.fla" in w4["selected"][0]),
                  json.dumps(w4, ensure_ascii=False))
            # 回 quick:all
            await click("document.querySelector('.crumb-group .icon-btn:not(.rot-180):not(.archive-panel-toggle)')")
            await asyncio.sleep(0.7)

            # W5 徽章悬停出链接列表浮层 + 点行跳转
            # 坑：CDP Input.dispatchMouseEvent mouseMoved（含 buttons 0）不触发 Vue mouseenter（W4 子菜单
            # hover 属侥幸）——悬停类交互改 JS 直接 dispatch MouseEvent('mouseenter') 到目标元素
            w5 = None
            # flake 治理（2026-08-24 三连 FAIL 后实证）：固定 sleep 在 CDP 负载漂移下等不到浮层渲染——
            # 改轮询（重复 dispatch + 查 .link-pop，openLinkList 幂等无害，上限 8×0.4s）
            for _ in range(8):
                await ev("""(() => {
                  const b = [...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧-改'))?.querySelector('.card-badges');
                  if (b) b.dispatchEvent(new MouseEvent('mouseenter', { bubbles: false }));
                  return !!b;
                })()""")
                await asyncio.sleep(0.4)
                w5 = await ev("""(() => {
                  const pop = document.querySelector('.link-pop');
                  if (!pop) return null;
                  return { rows: [...pop.querySelectorAll('.row')].map((r) => ({
                    path: r.querySelector('.path')?.textContent, name: r.querySelector('.name')?.textContent })) };
                })()""")
                if w5: break
            check("W5 徽章悬停出浮层：全部位置 2 行（目录/…/雪天.档案 + 项目/工程，顺序无关）",
                  bool(w5 and len(w5["rows"]) == 2
                       and any("雪天.档案" in (r["path"] or "") and "雪牧-改.fla" in (r["name"] or "") for r in w5["rows"])
                       and any("工程" in (r["path"] or "") and "雪牧.fla" in (r["name"] or "") for r in w5["rows"])),
                  json.dumps(w5, ensure_ascii=False))
            await shot("link-list-popover")
            await click("[...document.querySelectorAll('.link-pop .row')].find((r) => r.textContent.includes('工程'))")
            await asyncio.sleep(0.8)
            w5b = await ev("""(() => ({
              pinned: document.querySelector('.detail-panel .pinned input')?.value ?? null,
              pop: !!document.querySelector('.link-pop'),
            }))()""")
            check("W5 点行 = 跳转工程视口 + 浮层关闭",
                  bool(w5b and w5b["pinned"] == "雪牧.fla" and not w5b["pop"]),
                  json.dumps(w5b, ensure_ascii=False))

            # W6 将此文件设为原始文件（右键链接卡 → 菜单项 → ★ 转移；origin 卡该项禁用）
            r6 = await rect_of("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧.fla'))")
            await mouse("mousePressed", r6["x"], r6["y"], "right"); await mouse("mouseReleased", r6["x"], r6["y"], "right")
            await asyncio.sleep(0.6)
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('将此文件设为原始文件'))")
            await asyncio.sleep(0.8)
            # 回 quick:all 再查两卡徽章（工程视口只有 F-P2 内容，雪牧-改 不在场）
            await click("document.querySelector('.crumb-group .icon-btn:not(.rot-180):not(.archive-panel-toggle)')")
            await asyncio.sleep(0.7)
            w6 = await ev("""(() => {
              const badgeOf = (txt) => {
                const c = [...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes(txt));
                return c?.querySelector('.card-badges')?.dataset?.badge ?? null;
              };
              return {
                calls: window.__calls.set_placement_origin,
                fp2: badgeOf('雪牧.fla'), arc: badgeOf('雪牧-改'),
              };
            })()""")
            check("W6 设为原始文件 → set_placement_origin（F-P2 雪牧.fla 带 name）+ ★ 转移（雪牧-改 回链接 icon）",
                  bool(w6 and len(w6["calls"]) == 1 and w6["calls"][0]["assetId"] == "A1"
                       and w6["calls"][0]["folderId"] == "F-P2" and w6["calls"][0]["name"] == "雪牧.fla"
                       and w6["fp2"] == "origin" and w6["arc"] == "link"),
                  json.dumps(w6, ensure_ascii=False))
            # origin 卡该项禁用
            await mouse("mousePressed", r6["x"], r6["y"], "right"); await mouse("mouseReleased", r6["x"], r6["y"], "right")
            await asyncio.sleep(0.6)
            w6b = await ev("""(() => {
              const m = [...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('将此文件设为原始文件'));
              return m ? { disabled: m.disabled === true } : null;
            })()""")
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            check("W6 origin 卡「将此文件设为原始文件」禁用", bool(w6b and w6b["disabled"]), json.dumps(w6b, ensure_ascii=False))

            await send(ws, "Target.closeTarget", {"targetId": tid})
    finally:
        proc.terminate()
        if dev_proc:
            # Windows shell=True：terminate 只杀 cmd 壳，node/vite 子进程存活占 5373——taskkill 整树
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(dev_proc.pid)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    print()
    if fails:
        print(f"失败 {len(fails)} 项：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
