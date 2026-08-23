# CDP 实测：6.2 一轮 · 主窗口选择模式 + 关系（2026-08-23）
# 桩带可变 relations 表（add/update/remove 真改数据，UI 跟变）：
# S1 进入选择模式四重标识：picking 类 + 1.5px 品牌边框伪元素 + 横幅文案 + 备选区替换详情面板 + 选择域清空
# S2 单选行为：点卡加入（大勾 picked 态）→ 点另一张选新换旧 → 点已选移出；发起文件自身不可选
# S3 称谓输入 + 确认：add_relation 全参数断言（from/to/label/reverseLabel）+ 退出 + 详情关系 item 出现
# S4 二次修改称谓：点称谓框 → C2 输入框 → Enter 提交 update_relation（label 字段）
# S5 换绑对方：点文件卡 → retarget 预填 → 选档案卡 → update_relation newToKind/Id 断言
# S6 双向显示：档案（to 侧）详情显示反向称谓「儿子」+ 对方快照名
# S7 × 删除：remove_relation + item 消失
# S8 Esc 取消：零调用退出
# S9 右键仅查看类：picker 下素材卡右键无编辑项（重命名/移动等），有「打开预览」
# 截图留档 tools/style-preview/：picker-mode / relation-items。
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
window.__calls = { add_relation: [], update_relation: [], remove_relation: [] };
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
        // 深拷贝斩断共享引用——原样返回 DB 数组会让前端 state 与桩共享同一数组对象：
        // 桩写绕过响应式、前端 list 赋值同引用不 trigger（2026-08-23 S4 双 item/不刷新攻关实锤）
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
      if (cmd === "add_relation") {
        window.__calls.add_relation.push(JSON.parse(JSON.stringify(args)));
        const rel = { id: "RL-" + (++window.__relId), from: refOf(args.fromKind, args.fromId), to: refOf(args.toKind, args.toId), kind: "free", label: args.label, reverseLabel: args.reverseLabel ?? null };
        DB.relations.push(rel);
        return Promise.resolve(JSON.parse(JSON.stringify(rel)));
      }
      if (cmd === "update_relation") {
        window.__calls.update_relation.push(JSON.parse(JSON.stringify(args)));
        const rel = DB.relations.find((r) => r.id === args.id);
        if (!rel) return Promise.reject("关系不存在，可能已被删除");
        rel.label = args.label;
        rel.reverseLabel = args.reverseLabel ?? null;
        if (args.newToKind && args.newToId) rel.to = refOf(args.newToKind, args.newToId);
        if (args.newFromKind && args.newFromId) rel.from = refOf(args.newFromKind, args.newFromId);
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

            # —— 进 F-SUB 视口（两张素材卡）——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-SUB\"] .body')")
            await asyncio.sleep(0.6)
            # 选中 A1（雪牧.fla）出详情
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            await asyncio.sleep(0.8)

            # —— S1 进入选择模式：详情关系区块添加钮 ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.6)
            s1 = await ev("""(() => {
              const w = document.querySelector('.window');
              const picking = w.classList.contains('picking');
              const bd = getComputedStyle(w, '::after');
              const banner = document.querySelector('.banner .text')?.textContent ?? null;
              const panel = !!document.querySelector('.picker-panel');
              const detail = !!document.querySelector('.sidebar-right .detail-panel');
              const selectedCards = document.querySelectorAll('.asset-card.selected').length;
              const origin = document.querySelector('.picker-panel .rel-box .rel-item:last-child .name')?.textContent ?? null;
              return { picking, bw: bd.borderTopWidth, bc: bd.borderTopColor, banner, panel, detail, selectedCards, origin };
            })()""")
            print("S1:", json.dumps(s1, ensure_ascii=False))
            check("S1 四重标识：picking 类 + 品牌边框 + 横幅（为 雪牧.fla 添加关系）+ 备选区替换详情",
                  bool(s1 and s1["picking"] and s1["bw"] in ("1px", "1.5px") and s1["bc"] == "rgb(61, 221, 210)"
                       and s1["banner"] == "选择文件/档案 - 为 雪牧.fla 添加关系"
                       and s1["panel"] and not s1["detail"]),
                  json.dumps(s1, ensure_ascii=False))
            check("S1 进入清选择域 + 发起文件 item（雪牧.fla）",
                  bool(s1 and s1["selectedCards"] == 0 and s1["origin"] == "雪牧.fla"),
                  json.dumps(s1, ensure_ascii=False))

            # —— S2 单选行为 ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            s2a = await ev("document.querySelector('.picker-panel .rel-box .rel-item .placeholder')?.textContent ?? '已选'")
            check("S2 发起文件自身不可选（仍为占位态）", s2a == "点击卡片选择", str(s2a))
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            s2b = await ev("""(() => ({
              name: document.querySelector('.picker-panel .rel-box .rel-item .name')?.textContent ?? null,
              pickedCards: [...document.querySelectorAll('.asset-card.picked')].length,
              check: !!document.querySelector('.asset-card.picked .pick-check'),
            }))()""")
            check("S2 点卡加入备选（阿拉蕾.fla）+ 卡片 picked 压暗大勾",
                  bool(s2b and s2b["name"] == "阿拉蕾.fla" and s2b["pickedCards"] == 1 and s2b["check"]),
                  json.dumps(s2b, ensure_ascii=False))
            # 选新换旧：点档案卡前先去 F-DIR；此处再点雪牧无效（自身），改点已选移出
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            s2c = await ev("document.querySelector('.picker-panel .rel-box .rel-item .placeholder')?.textContent ?? null")
            check("S2 点已选 = 移出（回占位态）", s2c == "点击卡片选择", str(s2c))

            # —— S3 选档案（换目标类型）+ 称谓 + 确认 ——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .body')")
            await asyncio.sleep(0.6)
            await click("document.querySelector('.archive-card[data-folder-id=\"F-ARC\"]')")
            s3a = await ev("document.querySelector('.picker-panel .rel-box .rel-item .name')?.textContent ?? null")
            check("S3 点档案卡加入备选（雪天）", s3a == "雪天", str(s3a))
            # 输入双框称谓（左 = 父亲：雪天对雪牧；右 = 儿子：雪牧对雪天）
            await ev("""(() => {
              const inputs = [...document.querySelectorAll('.picker-panel .rel-input input')];
              for (const [i, v] of [[0, '父亲'], [1, '儿子']]) {
                const el = inputs[i];
                el.value = v;
                el.dispatchEvent(new Event('input', { bubbles: true }));
              }
              return inputs.length;
            })()""")
            await asyncio.sleep(0.3)
            disabled = await ev("document.querySelector('.picker-panel .pick-actions .app-btn.primary')?.disabled")
            check("S3 已选 + 称谓非空 → 确认可用", disabled is False, str(disabled))
            await shot("picker-mode")
            await click("document.querySelector('.picker-panel .pick-actions .app-btn.primary')")
            await asyncio.sleep(0.8)
            calls = await ev("window.__calls.add_relation")
            s3b = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              item: !!document.querySelector('.detail-panel .section-relations .rel-item'),
              items: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
              label: document.querySelector('.detail-panel .section-relations .rel-item .label-text')?.textContent ?? null,
              other: document.querySelector('.detail-panel .section-relations .rel-item .file-chip .name')?.textContent ?? null,
            }))()""")
            check("S3 确认 → add_relation(asset A1 → archive F-ARC, 父亲/儿子)",
                  bool(calls and len(calls) == 1 and calls[0]["fromKind"] == "asset" and calls[0]["fromId"] == "A1"
                       and calls[0]["toKind"] == "archive" and calls[0]["toId"] == "F-ARC"
                       and calls[0]["label"] == "父亲" and calls[0]["reverseLabel"] == "儿子"),
                  json.dumps(calls, ensure_ascii=False))
            check("S3 确认后退模式 + 详情关系 item（父亲 ← 雪天）",
                  bool(s3b and not s3b["picking"] and s3b["item"] and s3b["label"] == "父亲" and s3b["other"] == "雪天"),
                  json.dumps(s3b, ensure_ascii=False))

            # —— S4 二次修改称谓（点称谓框 → C2 输入框 → 改「老爸」Enter） ——
            await click("document.querySelector('.detail-panel .section-relations .rel-item .label-chip')")
            s4a = await ev("""(() => {
              const chip = document.querySelector('.detail-panel .section-relations .rel-item .label-chip');
              const input = chip.querySelector('input');
              return input ? { editing: chip.classList.contains('editing'), bg: getComputedStyle(chip).backgroundColor, val: input.value } : null;
            })()""")
            check("S4 点称谓框进编辑态（C2 底输入框，初值 父亲）",
                  bool(s4a and s4a["editing"] and s4a["val"] == "父亲" and s4a["bg"] == "rgb(22, 23, 23)"),
                  json.dumps(s4a))
            await ev("""(() => {
              const input = document.querySelector('.detail-panel .section-relations .rel-item .label-chip input');
              input.value = '老爸';
              input.dispatchEvent(new Event('input', { bubbles: true }));
              input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
              return true;
            })()""")
            await asyncio.sleep(0.6)
            # 提交链 = update + refreshRelations 两趟 invoke + 重渲染，headless 下轮询等 UI 落定
            s4b = None
            for _ in range(12):
                s4b = await ev("""(() => ({
                  calls: window.__calls.update_relation,
                  label: document.querySelector('.detail-panel .section-relations .rel-item .label-text')?.textContent ?? null,
                }))()""")
                if s4b and s4b["label"] == "老爸":
                    break
                await asyncio.sleep(0.3)
            check("S4 提交 → update_relation(label 老爸 / reverse 儿子保留 / 不换绑)",
                  bool(s4b and len(s4b["calls"]) == 1 and s4b["calls"][0]["label"] == "老爸"
                       and s4b["calls"][0]["reverseLabel"] == "儿子" and s4b["calls"][0]["newToId"] is None
                       and s4b["label"] == "老爸"),
                  json.dumps(s4b, ensure_ascii=False))

            # —— S5 换绑对方（点文件卡 → 预填 → 改选 A2 阿拉蕾） ——
            await click("document.querySelector('.tree-item[data-folder-id=\"F-SUB\"] .body')")
            await asyncio.sleep(0.5)
            await click("document.querySelector('.detail-panel .section-relations .rel-item .file-chip')")
            await asyncio.sleep(0.5)
            s5a = await ev("""(() => ({
              banner: document.querySelector('.banner .text')?.textContent ?? null,
              picked: document.querySelector('.picker-panel .rel-box .rel-item .name')?.textContent ?? null,
              inputs: [...document.querySelectorAll('.picker-panel .rel-input input')].map((i) => i.value),
            }))()""")
            check("S5 点文件卡进换绑模式（横幅更换关系对象；预填 雪天 + 老爸/儿子）",
                  bool(s5a and "更换关系对象" in (s5a["banner"] or "") and s5a["picked"] == "雪天"
                       and s5a["inputs"] == ["老爸", "儿子"]),
                  json.dumps(s5a, ensure_ascii=False))
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await click("document.querySelector('.picker-panel .pick-actions .app-btn.primary')")
            await asyncio.sleep(0.8)
            s5b = await ev("""(() => ({
              calls: window.__calls.update_relation,
              other: document.querySelector('.detail-panel .section-relations .rel-item .file-chip .name')?.textContent ?? null,
            }))()""")
            check("S5 换绑确认 → update_relation(newTo asset A2) + item 对方 = 阿拉蕾.fla",
                  bool(s5b and len(s5b["calls"]) == 2 and s5b["calls"][1]["newToKind"] == "asset"
                       and s5b["calls"][1]["newToId"] == "A2" and s5b["other"] == "阿拉蕾.fla"),
                  json.dumps(s5b, ensure_ascii=False))

            # —— S6 双向显示（to 侧 = A2 阿拉蕾：显示反向称谓「儿子」） ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await asyncio.sleep(0.8)
            s6 = await ev("""(() => {
              const item = document.querySelector('.detail-panel .section-relations .rel-item');
              return item ? { label: item.querySelector('.label-text')?.textContent, other: item.querySelector('.file-chip .name')?.textContent } : null;
            })()""")
            check("S6 双向显示：阿拉蕾详情 = 儿子 ← 雪牧.fla（reverseLabel）",
                  bool(s6 and s6["label"] == "儿子" and s6["other"] == "雪牧.fla"),
                  json.dumps(s6, ensure_ascii=False))
            await shot("relation-items")

            # —— S7 × 删除（hover 滑入，JS click） ——
            await ev("document.querySelector('.detail-panel .section-relations .rel-item .file-chip .del')?.click()")
            await asyncio.sleep(0.6)
            s7 = await ev("""(() => ({
              calls: window.__calls.remove_relation,
              items: document.querySelectorAll('.detail-panel .section-relations .rel-item').length,
            }))()""")
            check("S7 × → remove_relation(RL-1) + item 消失",
                  bool(s7 and s7["calls"] == [{"id": "RL-1"}] and s7["items"] == 0),
                  json.dumps(s7, ensure_ascii=False))

            # —— S8 Esc 取消（零调用） ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.5)
            await key("Escape")
            s8 = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              adds: window.__calls.add_relation.length,
            }))()""")
            check("S8 Esc = 取消模式全部放弃（零写入）",
                  bool(s8 and not s8["picking"] and s8["adds"] == 1), json.dumps(s8))

            # —— S9 右键仅查看类 ——
            await click("document.querySelector('.detail-panel .section-relations .icon-btn')")
            await asyncio.sleep(0.5)
            r = await rect_of("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
            await asyncio.sleep(0.6)
            s9 = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => m.textContent.trim())")
            print("S9 菜单:", json.dumps(s9, ensure_ascii=False))
            check("S9 选择模式右键仅查看类（有打开预览，无重命名/移动/删除）",
                  bool(s9 and any("打开预览" in x for x in s9) and not any("重命名" in x for x in s9)
                       and not any("移动文件" in x for x in s9) and not any("回收站" in x for x in s9)),
                  json.dumps(s9, ensure_ascii=False))
            await key("Escape")  # 关菜单
            await key("Escape")  # 退模式
            s9b = await ev("document.querySelector('.window').classList.contains('picking')")
            check("S9 Esc 关菜单后第二个 Esc 退模式", s9b is False, str(s9b))

            await send(ws, "Target.closeTarget", {"targetId": tid})
    finally:
        proc.terminate()
        if dev_proc:
            dev_proc.terminate()

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
