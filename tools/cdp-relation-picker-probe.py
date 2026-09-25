# CDP 实测：6.2 主窗口选择模式 + 资产关系（2026-08-23 一轮 / 2026-08-24 二轮修订）
# 2026-09-25 T0 验证网修复：锚点全换代重写 + 共享桩 _probe_stub
# 二轮口径：关系 = 多选累积（整组竖排复制，批量写入单撤销步）；换绑废除，点文件卡 = 跳转定位
# （面包屑返回）；发起文件自身剪切档半透明禁选；白名单收窄（汉堡禁 / 档案侧栏隐藏 / 收件箱·包内守卫）。
#
# 锚点换代（7.1 贴稿后静态盘点核实）：
# - 入口：旧 .detail-panel .section-relations .icon-btn 已废 —— 关系区迁链接关系 tab（TabLinks
#   .links-tab 五 .lb-block；添加钮 = 区块标题动作组 .acts .act，onAdd → picker.enterRelation）。
#   详情面板三 tab 化（ui.detailTab 全局记忆，默认 basic）——进模式前先切 links 页（.seg-block .seg-item）。
# - 备选区：PickRelationItem（.pr.ni-card）——选中文件框 = 首个 .box-input（只读，值即选中名）；
#   关系双框 = .box.rel .box-input ×2（左 源称谓→label / 右 目标称谓→reverseLabel）；
#   发起文件框 = .origin-name；占位组 = picked 空时唯一一组（首框空值 + placeholder「点击卡片选择」）。
# - 确认钮：ActionButton（.pick-actions .act-btn；次序口径 = 次左主右）——按文案「确认 (N)」寻钮。
# - 详情关系行：LinkRelationItem（.lr.ni-card / .chip 称谓（点击进输入态 .chip-input）/ .tgt 目标名
#   （点击跳转）/ .ni-close 删除（滑入钮常态隐藏，JS click 直触））。
# - 详情名称框迁 TabBasic（.db-block.inputs 首个 TextInput .ti-inner，只读）——读名前先切 basic 页。
# - 品牌框色：--n-brand #0f89ec → rgb(15, 137, 236)（旧 rgb(61,221,210) 随旧 token 废；桩
#   get_window_fx_mode 恒 solid，.window.picking::after 1px 品牌框为实色模式自绘，可测）。
# - 白名单：汉堡已废 —— 菜单栏「菜单」钮视觉不禁、点击无效（SidebarLeft onAppMenu picker.active
#   闸早退），断言改「点了不开菜单」；工具栏「添加文件/档案链接」钮选择模式业务禁用（linkTarget 空）。
# - 面包屑后退：.crumb-group .nav-pair .ic-btn（前进带 .flip-h，:not(.flip-h) = 后退）。
# - 树行：7.1 十八轮迁 TreeRowItem（FolderTree.vue:154）——旧 TreeItem 的 button.body / .expand 废，
#   点行根 .tree-item[data-folder-id] 导航（行内无 .body 子层），展开钮 = .exp；树定位跟随
#   （LibraryView.vue:43-73）会自动展开当前视口祖先链——展开态不可假设，探针一律条件展开（见 S8）。
# - 仍活未动：.window.picking / .banner .text / .picker-panel / .asset-card picker-self 半透明禁选。
#
# 启动链走 reopenLastSession 真路径（get_current_library 用默认桩 null → list_recent_libraries →
# open_space → open_library）——currentLibrary.name 落「动画资产」，关系 lib 匹配（relationsForTarget /
# jumpToRelationTarget 跨库守卫）才有真值可比；直接桩 get_current_library 非空会让 currentLibrary
# 恒 null、关系视图恒空（App.vue 该分支只 refresh 不回填库名）。
#
# 桩带可变 relations 表（add_relations/update/remove 真改数据，list 深拷贝斩断共享引用，踩坑 053）：
# S1 四重标识 + 进入清选择域 + 空态占位组（发起文件雪牧.fla 在组内）
# S2 多选累积：点卡追加组 → 再点第二张再追加（非切换）→ 点已选移除该组；自身半透明禁选
# S3 两组称谓 + 确认：add_relations 批量全参数断言 + 退出回触发处 + 详情 2 item
# S4 二次修改称谓：点称谓 chip → .chip-input 输入框 → Enter 提交 update_relation（参数收窄无换绑字段）
# S5 跳转：点目标名（档案）→ 档案视口 + 选中档案出详情（basic 页名称框 = 雪天）
# S6 双向显示：档案（to 侧）详情显示反向称谓；点目标名跳回文件（asset 跳转）
# S7 × 删除：remove_relation + item 2 → 1
# S8 Esc 取消：零写入 + picked 清空退出
# S9 右键仅查看类（选择模式重进 = 占位组复原，picked 清空实证）
# 截图留档 tools/style-preview/：picker-mode-multi / relation-jump。
import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9365
URL = "http://localhost:5373"

# 档案封面图（仅视觉、不参与断言）：仓库内测试资产，vite dev 经 /@fs/ 直读
STILL = os.path.abspath(os.path.join(os.path.dirname(__file__), "test-assets", "anim3.webp")).replace("\\", "/")

# 桩数据常量与命令拦截走 make_stub 的 head / handlers；启动链默认桩表（get_window_fx_mode solid /
# get_ui_settings {} / get_undo_state 等）由 _probe_stub.DEFAULT_STUBS 兜底
HEAD = (
    "const STILL = "
    + json.dumps(STILL)
    + ";\n"
    + r"""
window.__calls = { add_relations: [], update_relation: [], remove_relation: [] };
window.__relId = 0;
const LIB = "动画资产";
const mkP = (assetId, folderId, name, ext, origin) => ({
  assetId, folderId, name, w: null, h: null, thumbW: null, thumbH: null, ext,
  thumbPath: null, thumbKind: null, origin, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, size: 3145728, createdAt: "2026-08-15T11:22:33+08:00",
  modifiedAt: "2026-08-15T11:22:33+08:00", importedAt: "2026-08-15T10:20:30+08:00",
  noteCount: null, thumbFrame: null, fps: null, hidden: false, playAssetId: null,
});
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
    mkP("A1", "F-SUB", "雪牧.fla", "fla", true),
    mkP("A2", "F-SUB", "阿拉蕾.fla", "fla", true),
  ],
  archives: {
    "F-ARC": { folderId: "F-ARC", badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", totalSize: 0, coverPath: STILL },
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
  const base = rec ?? { folderId: id, badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", totalSize: 0, coverPath: null };
  return { ...base, fileCount: inFolder.length, totalSize };
};
"""
)

HANDLERS = r"""
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
        archives: [], thumbAsset: null, thumbUsedBy: [],
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
"""

STUB = make_stub(
    {
        # 启动重开链（reopenLastSession）：最近库 → open_space → 空间内 open_library，
        # currentLibrary.name 由此落「动画资产」（get_current_library 留默认 null）
        "list_recent_libraries": [
            {"name": "动画资产", "path": "E:/T/S/动画资产", "lastOpenedAt": "2026-08-15T10:20:30+08:00"}
        ],
        "open_space": {"name": "S", "path": "E:/T/S", "autoImport": True},
        "list_space_libraries": ["动画资产"],
        "open_library": {"name": "动画资产", "path": "E:/T/S/动画资产", "assetCount": 2, "folderCount": 3},
        # DB 动态读（R 表在每次 invoke 现算；relations 深拷贝斩断共享引用——踩坑 053）
        "list_placements": "JSON.parse(JSON.stringify(DB.placements))",
        "list_folder_tree": "tree()",
        "list_archives": "JSON.parse(JSON.stringify(Object.values(DB.archives)))",
        "list_relations": "JSON.parse(JSON.stringify(DB.relations))",
        "list_prop_pool": [],
    },
    head=HEAD,
    handlers=HANDLERS,
    convert_file_src='(p) => encodeURI("/@fs/" + p)',
)

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

            # —— 锚点速查（7.1 贴稿后口径，见文件头注释）——
            # 关系区块（链接关系 tab 五区块，按标题「关系」定位；排序可拖但默认稿序固定）
            REL_BLOCK = "[...document.querySelectorAll('.links-tab .lb-block')].find((b) => b.querySelector('.st .txt')?.textContent.trim() === '关系')"
            # 关系区块添加钮（SectionTitle staticActions 常显；withMore=false → 动作组唯一钮 = 添加）
            REL_ADD = "(" + REL_BLOCK + ")?.querySelector('.acts .act')"
            # 详情关系行清单（LinkRelationItem .lr.ni-card；空数据不渲染内容容器）
            REL_ITEMS = "[...((" + REL_BLOCK + ")?.querySelectorAll('.lb-body .lr.ni-card') ?? [])]"
            # 备选区已选组名清单（PickRelationItem 首个 .box-input = 选中文件只读框，值即名；
            # 占位组该框为空值（placeholder 属性「点击卡片选择」），按空值过滤即已选清单）
            PICKED = "[...document.querySelectorAll('.picker-panel .pr.ni-card')].map((c) => c.querySelector('.box-input')?.value ?? '').filter((v) => v !== '')"
            # 确认钮（按钮次序口径 = 次左主右——按文案寻钮更稳）
            CONFIRM = "[...document.querySelectorAll('.picker-panel .pick-actions .act-btn')].find((b) => b.textContent.includes('确认'))"
            # 详情名称框（TabBasic 首个 TextInput 只读框；读名前须切 basic 页）
            DETAIL_NAME = "document.querySelector('.detail-root .db-block.inputs .ti-root .ti-inner')?.value ?? null"
            # 面包屑后退钮（前进带 .flip-h）
            NAV_BACK = "document.querySelector('.crumb-group .nav-pair .ic-btn:not(.flip-h)')"
            # 菜单栏「菜单」文字钮（汉堡的继任入口）
            MENU_BTN = "[...document.querySelectorAll('.sidebar-left .menu-bar .act-btn')].find((b) => b.textContent.trim() === '菜单')"

            async def seg_tab(i):  # 详情面板分段控制器切页：0 基础信息 / 1 链接关系 / 2 提示词（tab-x 动画 350ms）
                await click(f"document.querySelectorAll('.sidebar-right .seg-block .seg-item')[{i}]")
                await asyncio.sleep(0.6)

            # —— 初始视口 quick:all（两张素材卡），点雪牧出详情，切链接关系 tab ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('雪牧'))")
            await asyncio.sleep(0.8)
            await seg_tab(1)

            # —— S1 进入选择模式（关系区块标题添加钮 = enterRelation 入口）——
            await click(REL_ADD)
            await asyncio.sleep(0.6)
            s1 = await ev("""(() => {
              const w = document.querySelector('.window');
              const bd = getComputedStyle(w, '::after');
              const holder = document.querySelector('.picker-panel .pr.ni-card');
              return {
                picking: w.classList.contains('picking'), bw: bd.borderTopWidth, bc: bd.borderTopColor,
                banner: document.querySelector('.banner .text')?.textContent ?? null,
                panel: !!document.querySelector('.picker-panel'),
                detail: !!document.querySelector('.sidebar-right .detail-root'),
                selectedCards: document.querySelectorAll('.asset-card .fc-root.selected').length,
                origin: holder?.querySelector('.origin-name')?.textContent ?? null,
                placeholder: holder?.querySelector('.box-input')?.placeholder ?? null,
              };
            })()""")
            check("S1 四重标识 + 进入清选择域 + 空态占位组（发起文件雪牧.fla 在组内）",
                  bool(s1 and s1["picking"] and s1["bw"] in ("1px", "1.5px") and s1["bc"] == "rgb(15, 137, 236)"
                       and s1["banner"] == "选择文件/档案，为 [雪牧.fla] 添加关系"
                       and s1["panel"] and not s1["detail"] and s1["selectedCards"] == 0
                       and s1["origin"] == "雪牧.fla" and s1["placeholder"] == "点击卡片选择"),
                  json.dumps(s1, ensure_ascii=False))
            # 白名单：汉堡已废 —— 菜单栏「菜单」钮视觉不禁、点击无效（picker.active 闸早退不开菜单）；
            # 工具栏「添加文件/档案链接」钮（按钮组首个）选择模式业务禁用
            s1b_pre = await ev("(() => { const b = " + MENU_BTN + "; return b ? { disabled: b.disabled } : null; })()")
            await click(MENU_BTN)
            await asyncio.sleep(0.4)
            s1b = await ev("""(() => ({
              menuOpen: !!document.querySelector('.app-menu'),
              addLink: document.querySelector('.panel-toolbar .btn-group .ic-btn')?.disabled ?? null,
            }))()""")
            check("S1 白名单：菜单钮视觉不禁、点击不开菜单；工具栏添加链接钮禁用",
                  bool(s1b_pre and s1b_pre["disabled"] is False
                       and s1b and s1b["menuOpen"] is False and s1b["addLink"] is True),
                  json.dumps({"pre": s1b_pre, "post": s1b}, ensure_ascii=False))

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
            await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"]')")
            await asyncio.sleep(0.5)
            await click("document.querySelector('.ar-card[data-folder-id=\"F-ARC\"]')")
            s2b = await ev(PICKED)
            check("S2 多选累积（阿拉蕾.fla + 雪天两组，非切换）+ 跨视口导航模式保持",
                  bool(s2b and s2b == ["阿拉蕾.fla", "雪天"]), json.dumps(s2b, ensure_ascii=False))
            await click("document.querySelector('.ar-card[data-folder-id=\"F-ARC\"]')")
            s2c = await ev(PICKED)
            check("S2 点已选 = 移除该组（剩阿拉蕾.fla）", bool(s2c and s2c == ["阿拉蕾.fla"]), json.dumps(s2c, ensure_ascii=False))
            await click("document.querySelector('.ar-card[data-folder-id=\"F-ARC\"]')")
            await asyncio.sleep(0.4)

            # —— S3 两组称谓 + 批量确认（左框 label / 右框 reverseLabel） ——
            await ev("""(() => {
              const cards = [...document.querySelectorAll('.picker-panel .pr.ni-card')];
              const vals = [['父亲', '儿子'], ['偶像', '歌迷']];
              cards.forEach((card, i) => {
                const inputs = card.querySelectorAll('.box.rel .box-input');
                inputs.forEach((el, j) => { el.value = vals[i][j]; el.dispatchEvent(new Event('input', { bubbles: true })); });
              });
              return cards.length;
            })()""")
            await asyncio.sleep(0.3)
            s3a = await ev("(" + CONFIRM + ")?.textContent.trim() ?? null")
            check("S3 确认钮带数量（确认 (2)）", s3a == "确认 (2)", str(s3a))
            await shot("picker-mode-multi")
            await click(CONFIRM)
            await asyncio.sleep(0.8)
            s3b = await ev("(() => ({ calls: window.__calls.add_relations, picking: document.querySelector('.window').classList.contains('picking'), items: "
                           + REL_ITEMS
                           + ".map((it) => ({ label: it.querySelector('.chip')?.textContent ?? null, other: it.querySelector('.tgt')?.textContent ?? null })) }))()")
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

            # —— S4 二次修改称谓（点 chip 进输入态，Enter 提交；参数收窄：无换绑字段） ——
            await click("(" + REL_ITEMS + ")[0]?.querySelector('.chip')")
            await ev("(() => { const input = ("
                     + REL_ITEMS
                     + ")[0]?.querySelector('.chip-input'); if (!input) return false;"
                       " input.value = '老爸'; input.dispatchEvent(new Event('input', { bubbles: true }));"
                       " input.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true })); return true; })()")
            s4 = None
            for _ in range(12):
                await asyncio.sleep(0.3)
                s4 = await ev("(() => ({ calls: window.__calls.update_relation, label: (("
                              + REL_ITEMS
                              + ")[0]?.querySelector('.chip')?.textContent) ?? null }))()")
                if s4 and s4["label"] == "老爸":
                    break
            c4 = s4 and s4["calls"] and s4["calls"][0]
            check("S4 称谓二次修改 → update_relation（label 老爸 / reverse 保留 / 无换绑字段）",
                  bool(c4 and c4["id"] == "RL-1" and c4["label"] == "老爸" and c4["reverseLabel"] == "儿子"
                       and "newToId" not in (c4 or {}) and s4["label"] == "老爸"),
                  json.dumps(s4, ensure_ascii=False))

            # —— S5 跳转（档案）：点目标名 → 档案视口 + 选中档案出详情（名称框在 basic 页） ——
            await click("(" + REL_ITEMS + ").find((it) => it.textContent.includes('雪天'))?.querySelector('.tgt')")
            await asyncio.sleep(0.8)
            await seg_tab(0)
            s5 = await ev("(() => ({ name: " + DETAIL_NAME + " }))()")
            print("S5 跳转后:", json.dumps(s5, ensure_ascii=False))
            check("S5 跳转档案视口（档案详情显示雪天）",
                  bool(s5 and s5["name"] == "雪天"), json.dumps(s5, ensure_ascii=False))

            # —— S6 双向显示（to 侧反向称谓）+ 点目标名跳回文件 ——
            await seg_tab(1)
            s6 = await ev("(() => { const it = ("
                          + REL_ITEMS
                          + ")[0]; return it ? { label: it.querySelector('.chip')?.textContent ?? null, other: it.querySelector('.tgt')?.textContent ?? null } : null; })()")
            check("S6 双向显示：雪天详情 = 歌迷 ← 雪牧.fla（reverseLabel）",
                  bool(s6 and s6["label"] == "歌迷" and s6["other"] == "雪牧.fla"),
                  json.dumps(s6, ensure_ascii=False))
            await shot("relation-jump")
            await click("(" + REL_ITEMS + ")[0]?.querySelector('.tgt')")
            await asyncio.sleep(0.8)
            s6b = await ev("(() => ({ items: " + REL_ITEMS + ".length }))()")
            check("S6 点目标名跳回雪牧（详情 = 雪牧的 2 条关系）",
                  bool(s6b and s6b["items"] == 2), json.dumps(s6b, ensure_ascii=False))

            # —— S6c 面包屑后退恢复选中（2026-08-24 用户定"返回默认选中卡片"）——
            # 链路：F-DIR(雪牧选中) → 跳雪天档案 → 跳雪牧(F-SUB)；后退两次应逐站恢复选中
            await click(NAV_BACK)
            await asyncio.sleep(0.7)
            await seg_tab(0)
            s6c = await ev(DETAIL_NAME)
            check("S6c 后退一站 = 雪天档案视口 + 档案选中恢复（详情雪天）", s6c == "雪天", str(s6c))
            await click(NAV_BACK)
            await asyncio.sleep(0.7)
            s6d_name = await ev(DETAIL_NAME)  # 再后退落在 basic 页（S6c 切过、全局记忆），名称框直读
            await seg_tab(1)
            s6d_items = await ev(REL_ITEMS + ".length")
            check("S6d 再后退 = 雪牧选中恢复（详情雪牧.fla + 关系 2 条）",
                  bool(s6d_name == "雪牧.fla" and s6d_items == 2),
                  json.dumps({"name": s6d_name, "items": s6d_items}, ensure_ascii=False))

            # —— S7 × 删除（2 → 1；滑入钮常态隐藏，JS click 直触） ——
            await ev("(" + REL_ITEMS + ").find((it) => it.textContent.includes('雪天'))?.querySelector('.ni-close')?.click()")
            await asyncio.sleep(0.6)
            s7 = await ev("(() => ({ calls: window.__calls.remove_relation, items: " + REL_ITEMS + ".length }))()")
            check("S7 × → remove_relation(RL-2) + item 2 → 1",
                  bool(s7 and s7["calls"] == [{"id": "RL-2"}] and s7["items"] == 1),
                  json.dumps(s7, ensure_ascii=False))

            # —— S8 Esc 取消（零写入 + picked 清空退出） ——
            # 当前视口 = F-DIR（阿拉蕾在 F-SUB）——选择模式白名单放行树导航，需进 F-SUB 点卡。
            # F-DIR 展开态不定：树定位跟随（LibraryView.vue:43-73，sidebarSelected 落 folder: 视口
            # 即自动展开祖先链）在 S6 跳进 F-SUB 后已把 F-DIR 展开——无条件点 .exp 会把「已开」
            # 误收成「关闭」（首修实跑实证：F-SUB 行消失、导航落空，靠递归显示默认开才误绿）。
            # 条件展开：F-SUB 行不在才点 .exp，在则直点
            await click(REL_ADD)
            await asyncio.sleep(0.5)
            sub_row = await ev("!!document.querySelector('.tree-item[data-folder-id=\"F-SUB\"]')")
            if not sub_row:
                await click("document.querySelector('.tree-item[data-folder-id=\"F-DIR\"] .exp')")
                await asyncio.sleep(0.4)
            await click("document.querySelector('.tree-item[data-folder-id=\"F-SUB\"]')")
            await asyncio.sleep(0.5)
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            s8a = await ev(PICKED)
            check("S8 重进后点卡入备选（阿拉蕾.fla 一组）", bool(s8a and s8a == ["阿拉蕾.fla"]), json.dumps(s8a, ensure_ascii=False))
            await key("Escape")
            s8 = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              adds: window.__calls.add_relations.length,
              panel: !!document.querySelector('.picker-panel'),
            }))()""")
            check("S8 Esc = 取消全部放弃（零写入 + 备选区退出）",
                  bool(s8 and not s8["picking"] and s8["adds"] == 1 and not s8["panel"]), json.dumps(s8))

            # —— S9 右键仅查看类（选择模式重进：占位组复原 = picked 清空实证） ——
            await click(REL_ADD)
            await asyncio.sleep(0.5)
            s9pre = await ev("""(() => {
              const cards = [...document.querySelectorAll('.picker-panel .pr.ni-card')];
              return { groups: cards.length,
                       pickedVal: cards[0]?.querySelector('.box-input')?.value ?? null,
                       origin: cards[0]?.querySelector('.origin-name')?.textContent ?? null };
            })()""")
            check("S9 重进选择模式 = 占位组复原（picked 清空，发起文件雪牧.fla）",
                  bool(s9pre and s9pre["groups"] == 1 and s9pre["pickedVal"] == "" and s9pre["origin"] == "雪牧.fla"),
                  json.dumps(s9pre, ensure_ascii=False))
            r = await rect_of("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('阿拉蕾'))")
            if r:
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
