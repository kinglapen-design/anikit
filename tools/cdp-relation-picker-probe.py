# CDP 实测：6.2 主窗口选择模式 + 关系（2026-08-23 一轮 / 2026-08-24 二轮修订）
# 二轮口径：关系 = 多选累积（整组竖排复制，批量写入单撤销步）；换绑废除，点文件卡 = 跳转定位
# （面包屑返回）；发起文件自身剪切档半透明禁选；白名单收窄（汉堡禁 / 档案侧栏隐藏 / 收件箱·包内守卫）。
# 桩带可变 relations 表（add_relations/update/remove 真改数据，list 深拷贝斩断共享引用，踩坑 053）：
# S1 四重标识 + 进入清选择域 + 发起文件 item
# S2 多选累积：点卡追加组 → 再点第二张再追加（非切换）→ 点已选移除该组；自身半透明禁选
# S3 两组称谓 + 确认：add_relations 批量全参数断言 + 退出回触发处 + 详情 2 item + toast 计数
# S4 二次修改称谓：点称谓框 → C2 输入框 → Enter 提交 update_relation（参数收窄无换绑字段）
# S5 跳转：点文件卡（档案）→ 档案视口 + 选中档案卡出详情 + 面包屑可后退；后退回原视口
# S6 双向显示：档案（to 侧）详情显示反向称谓；文件卡跳回文件（asset 跳转）
# S7 × 删除：remove_relation + item 2 → 1
# S8 Esc 取消：零写入 + picked 清空退出
# S9 右键仅查看类 + 白名单（汉堡 / 新增钮 disabled）
# 截图留档 tools/style-preview/：picker-mode-multi / relation-jump。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9365
URL = "http://localhost:5373"

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"
STUB = r"""
window.__calls = { add_relations: [], update_relation: [], remove_relation: [] };
window.__relId = 0;
window.__cbs = {};
window.__cbId = 0;
(function () {
  const STILL = "__STILL__";
  const LIB = "动画资产";
  const DB = {
    blocks: [{ id: "dir", name: "目录", icon: null }],
    folders: [
      { id: "F-DIR", name: "素材", parent: null, block: "dir", kind: null },
      { id: "F-SUB", name: "子文件夹", parent: "F-DIR", kind: null },
      { id: "F-ARC", name: "雪天", parent: "F-DIR", kind: "archive" },
    ],
    assets: {
      A1: { id: "A1", name: "雪牧.fla", originName: "雪牧.fla", ext: "fla", size: 3145728, width: null, height: null, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], colors: [], links: [], note: "" },
      A2: { id: "A2", name: "阿拉蕾.fla", originName: "阿拉蕾.fla", ext: "fla", size: 2097152, width: null, height: null, duration: null, importedAt: "2026-08-15T10:20:30+08:00", modifiedAt: "2026-08-15T11:22:33+08:00", tags: [], descriptions: [], colors: [], links: [], note: "" },
    },
    placements: [
      { assetId: "A1", folderId: "F-SUB", name: "雪牧.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
      { assetId: "A2", folderId: "F-SUB", name: "阿拉蕾.fla", w: null, h: null, ext: "fla", thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null },
    ],
    archives: {
      "F-ARC": { folderId: "F-ARC", badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", coverPath: STILL },
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
    block: f.parent === null ? (f.block ?? "dir") : null, icon: null,
    children: childrenOf(f.id).map(nodeOf),
  });
  const tree = () => ({ blocks: DB.blocks, nodes: childrenOf(null).map(nodeOf) });
  const nameOf = (kind, id) => (kind === "asset" ? DB.assets[id]?.name : byId(id)?.name) ?? id;
  const refOf = (kind, id) => ({ lib: LIB, kind, id, name: nameOf(kind, id) });
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
        get_current_library: { name: LIB, path: "E:/T/S/动画资产", assetCount: 2, folderCount: 3 },
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
        list_prop_pool: [],
        // 深拷贝斩断共享引用（踩坑 053：原样返回 = 前端 state 与桩共享，绕过响应式）
        list_relations: () => JSON.parse(JSON.stringify(DB.relations)),
      };
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
      if (cmd === "add_relations") {
        window.__calls.add_relations.push(JSON.parse(JSON.stringify(args)));
        const rels = args.items.map((it) => ({ id: "RL-" + (++window.__relId), from: refOf(args.fromKind, args.fromId), to: refOf(it.toKind, it.toId), kind: "free", label: it.label, reverseLabel: it.reverseLabel ?? null }));
        DB.relations.push(...rels);
        return Promise.resolve(JSON.parse(JSON.stringify(rels)));
      }
      if (cmd === "update_relation") {
        window.__calls.update_relation.push(JSON.parse(JSON.stringify(args)));
        const rel = DB.relations.find((r) => r.id === args.id);
        if (!rel) return Promise.reject("关系不存在，可能已被删除");
        rel.label = args.label;
        rel.reverseLabel = args.reverseLabel ?? null;
        return Promise.resolve(null);
      }
      if (cmd === "remove_relation") {
        window.__calls.remove_relation.push(JSON.parse(JSON.stringify(args)));
        DB.relations = DB.relations.filter((r) => r.id !== args.id);
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

            async def key(k):
                await ev(f"window.dispatchEvent(new KeyboardEvent('keydown', {{ key: '{k}', bubbles: true }}))")
                await asyncio.sleep(0.4)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # 备选区已选组名清单（占位组返回 []；占位文字的 .name 带 .placeholder 类须排除；
            # 发起文件"雪牧.fla"每组重复显示，滤掉只留选中文件）
            PICKED = "[...document.querySelectorAll('.picker-panel .rel-row .rel-item .name:not(.placeholder)')].map((n) => n.textContent).filter((n) => n !== '雪牧.fla')"

            # —— 初始视口 quick:all（两张素材卡），点雪牧出详情 ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            await asyncio.sleep(0.8)

            # —— S1 进入选择模式 ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.6)
            s1 = await ev("""(() => {
              const w = document.querySelector('.window');
              const bd = getComputedStyle(w, '::after');
              return {
                picking: w.classList.contains('picking'), bw: bd.borderTopWidth, bc: bd.borderTopColor,
                banner: document.querySelector('.banner .text')?.textContent ?? null,
                panel: !!document.querySelector('.picker-panel'),
                detail: !!document.querySelector('.sidebar-right .detail-panel'),
                selectedCards: document.querySelectorAll('.asset-card.selected').length,
                origin: document.querySelector('.picker-panel .rel-row .rel-box .rel-item:last-child .name')?.textContent ?? null,
                placeholder: document.querySelector('.picker-panel .placeholder')?.textContent ?? null,
              };
            })()""")
            check("S1 四重标识 + 进入清选择域 + 空态占位组（发起文件雪牧.fla 在组内）",
                  bool(s1 and s1["picking"] and s1["bw"] in ("1px", "1.5px") and s1["bc"] == "rgb(61, 221, 210)"
                       and s1["banner"] == "选择文件/档案，为 [雪牧.fla] 添加关系"
                       and s1["panel"] and not s1["detail"] and s1["selectedCards"] == 0
                       and s1["origin"] == "雪牧.fla" and s1["placeholder"] == "点击卡片选择"),
                  json.dumps(s1, ensure_ascii=False))
            # 白名单：汉堡 / 新增钮 disabled
            # 白名单：汉堡 / 新增钮 disabled（v-tip 指令不落 title 属性，按结构位置取）
            s1b = await ev("""(() => ({
              hamburger: document.querySelector('.sidebar-left .bar-title > .icon-btn')?.disabled,
              add: document.querySelector('.sidebar-left .add-btn-wrap .icon-btn')?.disabled,
            }))()""")
            check("S1 白名单：汉堡 + 新增钮 disabled", bool(s1b and s1b["hamburger"] and s1b["add"]), json.dumps(s1b))

            # —— S2 多选累积 + 自身禁选 ——
            self_state = await ev("""(() => {
              const c = [...document.querySelectorAll('.asset-card')].find((x) => x.textContent.includes('雪牧'));
              return c ? { cls: c.className, op: getComputedStyle(c).opacity, cursor: getComputedStyle(c).cursor } : null;
            })()""")
            check("S2 发起文件自身 = picker-self 半透明（0.5）+ 禁选光标",
                  bool(self_state and "picker-self" in self_state["cls"] and self_state["op"] == "0.5" and self_state["cursor"] == "not-allowed"),
                  json.dumps(self_state))
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            s2a = await ev(PICKED)
            check("S2 点自身不加组（仍空）", s2a == [], json.dumps(s2a))
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.5)
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            s2b = await ev(PICKED)
            check("S2 多选累积（阿拉蕾.fla + 雪天两组，非切换）+ 跨视口导航模式保持",
                  bool(s2b and s2b == ["阿拉蕾.fla", "雪天"]), json.dumps(s2b, ensure_ascii=False))
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            s2c = await ev(PICKED)
            check("S2 点已选 = 移除该组（剩阿拉蕾.fla）", bool(s2c and s2c == ["阿拉蕾.fla"]), json.dumps(s2c, ensure_ascii=False))
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            await asyncio.sleep(0.4)

            # —— S3 两组称谓 + 批量确认 ——
            await ev("""(() => {
              const rows = [...document.querySelectorAll('.picker-panel .rel-row')];
              const vals = [['父亲', '儿子'], ['偶像', '歌迷']];
              rows.forEach((row, i) => {
                const inputs = row.querySelectorAll('.rel-input input');
                inputs.forEach((el, j) => { el.value = vals[i][j]; el.dispatchEvent(new Event('input', { bubbles: true })); });
              });
              return rows.length;
            })()""")
            await asyncio.sleep(0.3)
            s3a = await ev("document.querySelector('.picker-panel .pick-actions .app-btn.primary')?.textContent")
            check("S3 确认钮带数量（确认 (2)）", s3a == "确认 (2)", str(s3a))
            await shot("picker-mode-multi")
            await click("document.querySelector('.picker-panel .pick-actions .app-btn.primary')")
            await asyncio.sleep(0.8)
            s3b = await ev("""(() => ({
              calls: window.__calls.add_relations,
              picking: document.querySelector('.window').classList.contains('picking'),
              items: [...document.querySelectorAll('.detail-panel .section-relations .rel-item')].map((it) => ({
                label: it.querySelector('.label-text')?.textContent, other: it.querySelector('.file-chip .name')?.textContent })),
            }))()""")
            c0 = s3b and s3b["calls"] and s3b["calls"][0]
            check("S3 确认 → add_relations 批量（from A1，items 两条全参数）",
                  bool(c0 and len(s3b["calls"]) == 1 and c0["fromKind"] == "asset" and c0["fromId"] == "A1"
                       and len(c0["items"]) == 2
                       and c0["items"][0]["toId"] == "A2" and c0["items"][0]["label"] == "父亲" and c0["items"][0]["reverseLabel"] == "儿子"
                       and c0["items"][1]["toKind"] == "archive" and c0["items"][1]["toId"] == "F-ARC" and c0["items"][1]["label"] == "偶像"),
                  json.dumps(c0, ensure_ascii=False))
            check("S3 退出回触发处（雪牧详情 2 item：父亲←阿拉蕾 / 偶像←雪天）",
                  bool(s3b and not s3b["picking"] and len(s3b["items"]) == 2
                       and s3b["items"][0]["label"] == "父亲" and s3b["items"][0]["other"] == "阿拉蕾.fla"
                       and s3b["items"][1]["label"] == "偶像" and s3b["items"][1]["other"] == "雪天"),
                  json.dumps(s3b["items"] if s3b else None, ensure_ascii=False))

            # —— S4 二次修改称谓（参数收窄：无换绑字段） ——
            await click("document.querySelector('.detail-panel .section-relations .rel-item .label-chip')")
            await ev("""(() => {
              const input = document.querySelector('.detail-panel .section-relations .rel-item .label-chip input');
              input.value = '老爸';
              input.dispatchEvent(new Event('input', { bubbles: true }));
              input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
              return true;
            })()""")
            s4 = None
            for _ in range(12):
                await asyncio.sleep(0.3)
                s4 = await ev("""(() => ({
                  calls: window.__calls.update_relation,
                  label: document.querySelector('.detail-panel .section-relations .rel-item .label-text')?.textContent ?? null,
                }))()""")
                if s4 and s4["label"] == "老爸":
                    break
            c4 = s4 and s4["calls"] and s4["calls"][0]
            check("S4 称谓二次修改 → update_relation（label 老爸 / reverse 保留 / 无换绑字段）",
                  bool(c4 and c4["id"] == "RL-1" and c4["label"] == "老爸" and c4["reverseLabel"] == "儿子"
                       and "newToId" not in (c4 or {}) and s4["label"] == "老爸"),
                  json.dumps(s4, ensure_ascii=False))

            # —— S5 跳转（档案）：点文件卡 → 档案视口 + 选中出详情 + 面包屑可后退 ——
            await click("[...document.querySelectorAll('.detail-panel .section-relations .rel-item')].find((it) => it.textContent.includes('雪天'))?.querySelector('.file-chip')")
            await asyncio.sleep(0.8)
            s5 = await ev("""(() => ({
              archivePanel: !!document.querySelector('.archive-side-panel, .archive-panel, .detail-panel .section-custom'),
              detailName: document.querySelector('.detail-panel .pinned .name-input, .detail-panel .detail-block.pinned input')?.value ?? null,
              backBtns: [...document.querySelectorAll('.content-title-bar .icon-btn, .title-bar .icon-btn')].map((b) => ({ t: b.title, d: b.disabled })),
            }))()""")
            print("S5 跳转后:", json.dumps(s5, ensure_ascii=False))
            check("S5 跳转档案视口（档案详情显示雪天）",
                  bool(s5 and s5["detailName"] == "雪天"), json.dumps(s5, ensure_ascii=False))

            # —— S6 双向显示（to 侧反向称谓）+ 文件卡跳回文件 ——
            s6 = await ev("""(() => {
              const it = document.querySelector('.detail-panel .section-relations .rel-item');
              return it ? { label: it.querySelector('.label-text')?.textContent, other: it.querySelector('.file-chip .name')?.textContent } : null;
            })()""")
            check("S6 双向显示：雪天详情 = 歌迷 ← 雪牧.fla（reverseLabel）",
                  bool(s6 and s6["label"] == "歌迷" and s6["other"] == "雪牧.fla"),
                  json.dumps(s6, ensure_ascii=False))
            await shot("relation-jump")
            await click("document.querySelector('.detail-panel .section-relations .rel-item .file-chip')")
            await asyncio.sleep(0.8)
            s6b = await ev("""(() => ({
              first: document.querySelector('.detail-panel .section-relations .rel-item .file-chip .name')?.textContent ?? null,
              items: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
            }))()""")
            check("S6 点文件卡跳回雪牧（详情 = 雪牧的 2 条关系）",
                  bool(s6b and s6b["items"] == 2), json.dumps(s6b, ensure_ascii=False))

            # —— S6c 面包屑后退恢复选中（2026-08-24 用户定"返回默认选中卡片"）——
            # 链路：F-DIR(雪牧选中) → 跳雪天档案 → 跳雪牧(F-SUB)；后退两次应逐站恢复选中
            await click("document.querySelector('.crumb-group .icon-btn:not(.rot-180):not(.archive-panel-toggle)')")
            await asyncio.sleep(0.7)
            s6c = await ev("document.querySelector('.detail-panel .pinned input')?.value ?? null")
            check("S6c 后退一站 = 雪天档案视口 + 档案选中恢复（详情雪天）", s6c == "雪天", str(s6c))
            await click("document.querySelector('.crumb-group .icon-btn:not(.rot-180):not(.archive-panel-toggle)')")
            await asyncio.sleep(0.7)
            s6d = await ev("""(() => ({
              name: document.querySelector('.detail-panel .pinned input')?.value ?? null,
              items: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
            }))()""")
            check("S6d 再后退 = 雪牧选中恢复（详情雪牧.fla + 关系 2 条）",
                  bool(s6d and s6d["name"] == "雪牧.fla" and s6d["items"] == 2),
                  json.dumps(s6d, ensure_ascii=False))

            # —— S7 × 删除（2 → 1） ——
            await ev("[...document.querySelectorAll('.detail-panel .section-relations .rel-item')].find((it) => it.textContent.includes('雪天'))?.querySelector('.del')?.click()")
            await asyncio.sleep(0.6)
            s7 = await ev("""(() => ({
              calls: window.__calls.remove_relation,
              items: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
            }))()""")
            check("S7 × → remove_relation(RL-2) + item 2 → 1",
                  bool(s7 and s7["calls"] == [{"id": "RL-2"}] and s7["items"] == 1),
                  json.dumps(s7, ensure_ascii=False))

            # —— S8 Esc 取消（零写入 + picked 清空） ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.5)
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await key("Escape")
            s8 = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              adds: window.__calls.add_relations.length,
              rows: document.querySelectorAll('.picker-panel .rel-row').length,
            }))()""")
            check("S8 Esc = 取消全部放弃（零写入）", bool(s8 and not s8["picking"] and s8["adds"] == 1), json.dumps(s8))

            # —— S9 右键仅查看类（选择模式重进） ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.5)
            r = await rect_of("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
            await asyncio.sleep(0.6)
            s9 = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => m.textContent.trim())")
            check("S9 选择模式右键仅查看类（有打开预览，无重命名/移动/删除）",
                  bool(s9 and any("打开预览" in x for x in s9) and not any("重命名" in x for x in s9)
                       and not any("移动文件" in x for x in s9) and not any("回收站" in x for x in s9)),
                  json.dumps(s9, ensure_ascii=False))
            await key("Escape")
            await key("Escape")

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
