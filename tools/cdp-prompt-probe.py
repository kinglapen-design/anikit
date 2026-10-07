# CDP 实测：提示词轮 + 二/三轮改版（2026-09-26；spec = src/lib/menuSpecs/promptMenu.ts +
# src/lib/promptOps.ts + src/components/sections/PromptSection.vue + stores/picker.ts enterPrompt）
# 二轮改版（用户定案）：详情第三 tab「提示词」废除（改「自定义组件」占位 TabCustom），提示词挪
# 基础信息 tab 末位区块（PromptSection，注册表 fields/tags/descs/baseData/prompt）；五钮稿废 =
# 两态三钮——无提示词 = 扫描（仅文件且 png/jpg/jpeg/webp）/ 新建（点击弹二级 .md/.txt）/ 指定（picker）；
# 有提示词 = 编辑 / 断开提示词链接 / 删除提示词文档。
# 三轮改版（2026-09-26 用户再定）：scanned 态按钮独立成套——保存提示词（save-prompt，
# new_asset_prompt_document kind=md 存为 md 并绑定 → 转已指定，单目标自动开预览）/ 重新扫描
# （analyze-prompt = analyze_asset_prompt）/ 删除提示词（trash = clear_asset_prompt
# label「删除提示词」，清快照回未指定）；二轮 scanned「编辑先存再开 / 断开清空 / 删除禁用」整组废除。
# 图标改名：新建 add-new-prompt（旧 add-large）、断开 break-prompt-link（旧 link）。
# 桩：同一文件夹 f1 内六卡——P1 png·未指定 / P2 png·已扫描 / P3 png·已指定(→T1) / F1 fla·未指定
# （扫描格式闸负例）/ T1 txt / T2 md（picker 文本卡）；树档案 a1·未指定 / a2·已指定(→T2)。
# 详情绑定走 DETAIL_PROMPTS（get_asset_detail 输出）；read_text_asset 由 window.__readFail
# 运行时开关切「源最新内容 / 抛错回落快照」；new_asset_prompt_document 桩回真 PlacementView 并落
# PLACEMENTS（单目标新建自动打开预览——loadAsset 按 key 找 placement，缺了预览瞬关）。
# S1 文件卡三态菜单（none 四项·分析仅 png 可用 / scanned 五项 / linked 四项）；
# S2 分析动作 → analyze_asset_prompt(assetId 正确)；
# S3 档案菜单（无「分析提示词」；none 三项「指定文本文档…」/ linked 四项）；
# S4 详情提示词区块·linked：编辑框 = read_text_asset 源最新、三钮全可用、复制钮写剪贴板、
#    断开 → clear_asset_prompt label「断开提示词链接」、删除 → delete_asset_prompt_document、
#    编辑 → 源文档预览打开；S4b __readFail 后同卡回落快照 content；
# S5 详情提示词区块·none（空文案 + 三钮可用 + 复制禁用；扫描 → analyze_asset_prompt；
#    新建 → 二级两项带图标 → .md → new_asset_prompt_document kind=md + 自动开预览；
#    指定 → picker 横幅；fla 负例扫描禁用）与 scanned（三轮独立三钮：保存 →
#    new_asset_prompt_document kind=md + 开预览 / 重新扫描 → analyze_asset_prompt /
#    删除提示词 → clear_asset_prompt label「删除提示词」）；档案无扫描钮（两钮）；
# S6 指定 picker：横幅文案 + 品牌框 + 网格只渲文本卡 + 点 md 卡 assign_asset_prompt + 退模式 toast；
# S7 主菜单 整理▸提示词：无选中整组 disabled 恒列 / 单选文件按态点亮 / 单选档案三项（无分析）；
# S8 信息查看管理弹窗：三段 = 基础信息/链接关系/自定义组件，基础信息段含「提示词」行，自定义组件段空。
# 截图留档 tools/style-preview/：prompt-picker / prompt-section-linked。
import asyncio
import base64
import json
import re
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

# Windows 控制台默认 GBK：中文可控但 ▸ 等特殊字符炸 UnicodeEncodeError——统一 utf-8
sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9383
URL = "http://localhost:5373"

# 桩数据常量与自定义命令拦截走 make_stub 的 head / handlers；启动链默认桩表由 _probe_stub 兜底
STUB = make_stub(
    {
        "get_current_library": '{ name: "测试库", path: "E:/test-lib", assetCount: 6, folderCount: 3 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: true }',
        "list_folder_tree": "JSON.parse(JSON.stringify(TREE))",
        "list_placements": "JSON.parse(JSON.stringify(PLACEMENTS))",
        "list_archives": "JSON.parse(JSON.stringify(ARCHIVES))",
        "list_recent_libraries": "[]",
        "list_space_libraries": '["测试库"]',
    },
    head=r"""
window.__calls = { analyze_asset_prompt: [], assign_asset_prompt: [], assign_archive_prompt: [], clipboard: [],
  clear_asset_prompt: [], delete_asset_prompt_document: [], new_asset_prompt_document: [] };
window.__readFail = false;
window.__newId = 0;
// 剪贴板桩：copyText 走 navigator.clipboard.writeText（lib/menuOps），headless 无授权直拒绝——换记录桩
const __stubClip = () => {
  try {
    navigator.clipboard.writeText = (t) => { window.__calls.clipboard.push(String(t)); return Promise.resolve(); };
  } catch (e) {}
};
__stubClip();
document.addEventListener("DOMContentLoaded", __stubClip);
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 6, children: [] },
    { id: "a1", name: "主角档案", kind: "archive", count: 0, children: [] },
    { id: "a2", name: "配角档案", kind: "archive", count: 0, children: [] },
  ],
};
const mkP = (assetId, folderId, name, ext, promptState) => ({
  assetId, folderId, name, w: 100, h: 100, thumbW: null, thumbH: null, ext,
  thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, size: 1234, createdAt: null,
  modifiedAt: "2026-09-26T00:00:00Z", importedAt: "2026-09-26T00:00:00Z",
  noteCount: null, thumbFrame: null, fps: null, tags: [], hidden: false, playAssetId: null,
  promptState,
});
const PLACEMENTS = [
  mkP("P1", "f1", "走路.png", "png", "none"),
  mkP("P2", "f1", "跑步.png", "png", "scanned"),
  mkP("P3", "f1", "跳跃.png", "png", "linked"),
  mkP("F1", "f1", "工程.fla", "fla", "none"),
  mkP("T1", "f1", "提示词.txt", "txt", "none"),
  mkP("T2", "f1", "设定.md", "md", "none"),
];
const ARCHIVES = [
  { folderId: "a1", badge: null, avatar: null, props: [], descriptions: [], links: [],
    createdAt: "", totalSize: 0, coverPath: null, prompt: null },
  { folderId: "a2", badge: null, avatar: null, props: [], descriptions: [], links: [],
    createdAt: "", totalSize: 0, coverPath: null,
    prompt: { assetId: "T2", name: "设定.md", content: "档案提示词快照", updatedAt: "2026-09-26T00:00:00Z" } },
];
// 详情提示词绑定（get_asset_detail 输出）：P1 未指定 / P2 已扫描快照 / P3 已指定 T1
const DETAIL_PROMPTS = {
  P1: null,
  P2: { assetId: null, name: "", content: "扫描快照文案", updatedAt: "2026-09-26T00:00:00Z" },
  P3: { assetId: "T1", name: "提示词.txt", content: "快照内容", updatedAt: "2026-09-26T00:00:00Z" },
};
""",
    handlers=r"""
    if (cmd === "analyze_asset_prompt") {
      window.__calls.analyze_asset_prompt.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "assign_asset_prompt") {
      window.__calls.assign_asset_prompt.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "assign_archive_prompt") {
      window.__calls.assign_archive_prompt.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "clear_asset_prompt") {
      window.__calls.clear_asset_prompt.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "delete_asset_prompt_document") {
      window.__calls.delete_asset_prompt_document.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "new_asset_prompt_document") {
      window.__calls.new_asset_prompt_document.push(JSON.parse(JSON.stringify(args ?? {})));
      // 回真 PlacementView 并落 PLACEMENTS：单目标新建后自动打开预览（openPromptPlacement），
      // loadAsset 按 key 在 placements 找——缺了 assetKey 置空预览瞬关（preview.ts loadAsset 开头）
      const np = mkP("N" + (++window.__newId), args.folderId ?? "f1", "提示词文档." + (args.kind ?? "md"), args.kind ?? "md", "linked");
      PLACEMENTS.push(np);
      return Promise.resolve(JSON.parse(JSON.stringify(np)));
    }
    if (cmd === "read_text_asset") {
      // linked 态 TabPrompt 实时读源：__readFail 切换「源最新 / 抛错回落快照」（S4 两轮同页完成）
      if (window.__readFail) return Promise.reject("源文件不可读（桩）");
      return Promise.resolve({ content: "源最新内容" });
    }
    if (cmd === "get_asset_detail") {
      return Promise.resolve(JSON.parse(JSON.stringify({
        assetId: args.assetId, originName: "o.png", ext: "png", size: 1, width: 1, height: 1,
        duration: null, importedAt: "", modifiedAt: "", tags: [], descriptions: [],
        colors: [], links: [], note: "", placements: [], archives: [],
        thumbAsset: null, thumbUsedBy: [],
        prompt: DETAIL_PROMPTS[args.assetId] ?? null,
      })));
    }
    if (cmd === "get_archive_data") {
      const a = ARCHIVES.find((x) => x.folderId === args.folderId);
      return Promise.resolve(JSON.parse(JSON.stringify({
        folderId: args.folderId, badge: null, avatar: null, props: [], descriptions: [],
        links: [], createdAt: "", totalSize: 0, coverPath: null, prompt: null,
        ...(a ?? {}), fileCount: 0,
      })));
    }
""",
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = raw["params"]["args"]
            print("[CONSOLE-ERR]", str([a.get("value", a.get("description", "?")) for a in args])[:300])


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


async def main():
    # 源码静态断言素材（2026-09-26 三轮：scanned 独立成套三钮 + 图标改名 add-new-prompt /
    # break-prompt-link）——三段 .btn-row 的 IconButton name/title 序（同 context-menu 探针 E2b 静态断言先例）
    psrc = open("src/components/sections/PromptSection.vue", encoding="utf-8").read()

    def btn_sig(block_re):
        m = re.search(block_re, psrc, re.S)
        return re.findall(r'name="([^"]+)"[^>]*?title="([^"]+)"', m.group(1)) if m else None

    NONE_BTNS = btn_sig(r'v-if="state === \'none\'"(.*?)</div>')
    SCANNED_BTNS = btn_sig(r'v-else-if="state === \'scanned\'"(.*?)</div>')
    LINKED_BTNS = btn_sig(r'<div v-else class="btn-row">(.*?)</div>')

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
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    fails = []

    def check(name, cond, detail=""):
        print(("PASS " if cond else "FAIL ") + name + (f" | {detail}" if detail else ""))
        if not cond:
            fails.append(name)

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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def wait_ready(expr, tries=60):
                # 冷启动 vite 首服务可能依赖重优化/强制刷新，固定 sleep 不可靠——轮询就绪（2026-09-26 实证：
                # 探针自起 dev 后立即导航，页面 11s 仍未挂载 .sidebar-left，固定 6+5s 全 MISS）
                for _ in range(tries):
                    if await ev(expr):
                        return True
                    await asyncio.sleep(0.5)
                return False

            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await wait_ready("!!window.__TAURI_INTERNALS__ && document.readyState === 'complete' && !!document.querySelector('#app')")
            # 验证纪律（开发规范 §8）：连接后强制 reload 一次再断言，防页面跑旧码
            await send(ws, "Page.reload", session=sess)
            boot = await wait_ready("[...document.querySelectorAll('.sidebar-left .menu-bar .act-btn')].some((b) => b.textContent.trim() === '菜单')")
            await asyncio.sleep(0.5)

            async def mouse(t, x, y, button="left"):
                # buttons = 事件后的按键状态：release/move 恒 0（同 cdp-context-menu-probe 口径）
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "modifiers": 0, "buttons": 0 if t in ("mouseReleased", "mouseMoved") else (2 if button == "right" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            async def poll(expr, tries=10, interval=0.3):
                for _ in range(tries):
                    v = await ev(expr)
                    if v:
                        return v
                    await asyncio.sleep(interval)
                return await ev(expr)

            def el_center(sel_expr):
                return f"(() => {{ const el = {sel_expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()"

            def js_find(label, idx=0):
                return f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item') ?? []].find((x) => x.querySelector('.label')?.textContent === '{label}')"

            def item_center(label, idx=0):
                return el_center(js_find(label, idx))

            async def menu_labels(idx=0):
                return await ev(f"[...document.querySelectorAll('.app-menu')[{idx}]?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)") or []

            async def js_click(expr):
                r = await ev(expr)
                if r is None:
                    print("DIAG click MISS:", expr[:100])
                    return None
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def js_hover(expr):
                r = await ev(expr)
                if r is None:
                    print("DIAG hover MISS:", expr[:100])
                    return None
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def js_rclick_expr(expr):
                r = await ev(expr)
                if r is None:
                    print("DIAG rclick MISS:", expr[:100])
                    return None
                await mouse("mouseMoved", r["x"], r["y"])
                await asyncio.sleep(0.25)
                await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                await asyncio.sleep(0.6)
                # toast 挡命中 / 合成右键偶发不派发两道防线（2026-09-09 S9-7 实证，同 context-menu 探针）
                if not await ev("!!document.querySelector('.app-menu')"):
                    await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
                    await asyncio.sleep(0.3)
                    await mouse("mousePressed", r["x"], r["y"], "right"); await mouse("mouseReleased", r["x"], r["y"], "right")
                    await asyncio.sleep(0.6)
                return r

            async def esc_close():
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await asyncio.sleep(0.4)

            CARD = lambda key: f"document.querySelector('.asset-card[data-key=\"{key}\"]')"
            TREE_NODE = lambda fid: f"document.querySelector('[data-folder-id=\"{fid}\"]')"
            MENU_BTN = "[...document.querySelectorAll('.sidebar-left .menu-bar .act-btn')].find((b) => b.textContent.trim() === '菜单')"
            NONE4 = ["分析提示词", "指定文本文件为提示词", "新建 .md 提示词文档", "新建 .txt 提示词文档"]
            SCANNED5 = ["保存提示词为 .md 文档", "保存提示词为 .txt 文档", "复制提示词", "重新分析提示词", "清空提示词"]
            LINKED4 = ["打开文本编辑器", "复制提示词", "断开提示词文档链接", "删除提示词文档"]
            ARCHIVE3 = ["指定文本文档为提示词", "新建 .md 提示词文档", "新建 .txt 提示词文档"]

            async def open_prompt_flyout(rclick_expr, expect_labels, tag):
                """右键目标 → hover 提示词 → 断言二级清单，返回 flyout 是否开着"""
                await js_rclick_expr(rclick_expr)
                labels = await menu_labels()
                if "提示词" not in labels:
                    check(f"{tag} 一级含「提示词」", False, str(labels))
                    return False
                await js_hover(item_center("提示词"))
                fl = await menu_labels(1)
                check(f"{tag} 二级 = {expect_labels}", fl == expect_labels, str(fl))
                return True

            async def disabled_in_flyout(label):
                return await ev(f"(() => {{ const it = {js_find(label, 1)}; return it == null ? null : it.disabled; }})()")

            # —— S7a 主菜单·无选中（须最先跑：任何点击都会留下选中态）——
            check("S0 启动链渲染就绪（菜单钮存在）", boot)
            # —— S0c PromptSection 三态三钮图标/title 序（源码静态断言）——
            check("S0c-1 无态三钮序：analyze-prompt/扫描提示词 → add-new-prompt/新建提示词文档 → set-prompt-file/指定提示词文件",
                  NONE_BTNS == [("analyze-prompt", "扫描提示词"), ("add-new-prompt", "新建提示词文档"), ("set-prompt-file", "指定提示词文件")], str(NONE_BTNS))
            check("S0c-2 已扫描三钮序：save-prompt/保存提示词 → analyze-prompt/重新扫描 → trash/删除提示词",
                  SCANNED_BTNS == [("save-prompt", "保存提示词"), ("analyze-prompt", "重新扫描"), ("trash", "删除提示词")], str(SCANNED_BTNS))
            check("S0c-3 已指定三钮序：open-prompt-editor/编辑提示词 → break-prompt-link/断开提示词链接 → trash/删除提示词文档",
                  LINKED_BTNS == [("open-prompt-editor", "编辑提示词"), ("break-prompt-link", "断开提示词链接"), ("trash", "删除提示词文档")], str(LINKED_BTNS))
            await js_click(el_center(MENU_BTN))
            check("S7a-1 根菜单出", await ev("document.querySelectorAll('.app-menu').length === 1"))
            await js_hover(item_center("整理"))
            await js_hover(item_center("提示词", 1))
            fl = await menu_labels(2)
            check("S7a-2 无选中：整理▸提示词 L3 = 未指定四项恒列", fl == NONE4, str(fl))
            check("S7a-3 无选中：L3 整组 disabled", await ev("[...document.querySelectorAll('.app-menu')[2]?.querySelectorAll('.menu-item') ?? []].every((x) => x.disabled)"))
            await esc_close()

            # —— 进 f1 视口（六卡就位）——
            await js_click(el_center(TREE_NODE("f1")))
            await asyncio.sleep(0.6)
            check("S0 f1 视口渲六卡", await ev("document.querySelectorAll('.asset-card').length === 6"), str(await ev("document.querySelectorAll('.asset-card').length")))

            # —— S1 文件卡三态菜单 ——
            if await open_prompt_flyout(el_center(CARD("P1|f1")), NONE4, "S1a 文件·未指定"):
                check("S1a-2 png 卡「分析提示词」可用", await disabled_in_flyout("分析提示词") is False)
                # —— S2 分析动作 ——
                await js_click(item_center("分析提示词", 1))
                await asyncio.sleep(0.6)
                calls = await ev("JSON.stringify(window.__calls.analyze_asset_prompt)")
                check("S2 点「分析提示词」→ analyze_asset_prompt(assetId=P1)", calls == '[{"assetId":"P1"}]', str(calls))
                await esc_close()

            if await open_prompt_flyout(el_center(CARD("F1|f1")), NONE4, "S1b 文件·未指定(fla)"):
                check("S1b-2 fla 卡「分析提示词」禁用（格式闸）", await disabled_in_flyout("分析提示词") is True)
                await esc_close()

            if await open_prompt_flyout(el_center(CARD("P2|f1")), SCANNED5, "S1c 文件·已扫描"):
                check("S1c-2 已扫描二级全可用", await ev("[...document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-item')].every((x) => !x.disabled)"))
                await esc_close()

            if await open_prompt_flyout(el_center(CARD("P3|f1")), LINKED4, "S1d 文件·已指定"):
                check("S1d-2 已指定二级全可用（单选）", await ev("[...document.querySelectorAll('.app-menu')[1].querySelectorAll('.menu-item')].every((x) => !x.disabled)"))
                await esc_close()

            # —— S3 档案菜单（树节点右键同径）——
            if await open_prompt_flyout(el_center(TREE_NODE("a1")), ARCHIVE3, "S3a 档案·未指定"):
                check("S3a-2 档案二级无「分析提示词」", await disabled_in_flyout("分析提示词") is None)
                await esc_close()
            if await open_prompt_flyout(el_center(TREE_NODE("a2")), LINKED4, "S3b 档案·已指定"):
                await esc_close()

            # —— S7b/c 主菜单·按选中态切换 ——
            await js_click(el_center(CARD("P1|f1")))  # 单选文件
            await asyncio.sleep(0.5)
            await js_click(el_center(MENU_BTN))
            await js_hover(item_center("整理"))
            await js_hover(item_center("提示词", 1))
            fl = await menu_labels(2)
            check("S7b-1 选中 png：整理▸提示词 L3 = 未指定四项", fl == NONE4, str(fl))
            check("S7b-2 选中 png：L3 全可用（分析格式过闸）", await ev("[...document.querySelectorAll('.app-menu')[2]?.querySelectorAll('.menu-item') ?? []].every((x) => !x.disabled)"))
            await esc_close()
            await js_click(el_center(TREE_NODE("a1")))  # 单选档案
            await asyncio.sleep(0.5)
            await js_click(el_center(MENU_BTN))
            await js_hover(item_center("整理"))
            await js_hover(item_center("提示词", 1))
            fl = await menu_labels(2)
            check("S7c 选中档案：整理▸提示词 L3 = 档案三项（无分析）", fl == ARCHIVE3, str(fl))
            await esc_close()

            # —— S6 指定 picker ——
            await js_click(el_center(TREE_NODE("f1")))  # 回 f1 视口
            await asyncio.sleep(0.6)
            await js_rclick_expr(el_center(CARD("P1|f1")))
            await js_hover(item_center("提示词"))
            await js_click(item_center("指定文本文件为提示词", 1))
            await asyncio.sleep(0.8)
            s6 = await ev("""(() => {
              const w = document.querySelector('.window');
              const bd = getComputedStyle(w, '::after');
              const cards = [...document.querySelectorAll('.asset-card')].map((c) => c.getAttribute('data-key'));
              return {
                picking: w.classList.contains('picking'), bw: bd.borderTopWidth, bc: bd.borderTopColor,
                banner: document.querySelector('.banner .text')?.textContent ?? null,
                cards,
              };
            })()""")
            check("S6-1 picker 四重标识：picking + 品牌框 + 横幅文案",
                  bool(s6 and s6["picking"] and s6["bw"] in ("1px", "1.5px") and s6["bc"] == "rgb(15, 137, 236)"
                       and s6["banner"] == "选择文本文件，为 [走路.png] 指定提示词"),
                  json.dumps(s6, ensure_ascii=False))
            check("S6-2 网格只渲文本卡（T1/T2；png/fla 全过滤）",
                  bool(s6 and sorted(s6["cards"]) == ["T1|f1", "T2|f1"]), json.dumps(s6["cards"] if s6 else None))
            await shot("prompt-picker")
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("T2|f1")))  # 点 md 卡 = 单选直回
            await asyncio.sleep(1.0)
            calls = await ev("JSON.stringify(window.__calls.assign_asset_prompt)")
            check("S6-3 点 md 卡 → assign_asset_prompt{assetId:P1, promptAssetId:T2}",
                  calls == '[{"assetId":"P1","promptAssetId":"T2"}]', str(calls))
            check("S6-4 退模式（picking 解除）", await ev("!document.querySelector('.window').classList.contains('picking')"))
            toast_text = await poll("[...document.querySelectorAll('.toast')].map((x) => x.textContent).join('|')")
            check("S6-5 退模式 toast（已为「走路.png」指定提示词）", bool(toast_text and "已为「走路.png」指定提示词" in toast_text), str(toast_text))

            # —— S8 信息查看管理弹窗（提示词挪基础信息区块 + 第三段「自定义组件」占位）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("P1|f1")))
            await asyncio.sleep(0.8)
            await js_click(el_center("[...document.querySelectorAll('.manage-bar .act-btn')].find((b) => b.textContent.includes('信息查看管理'))"))
            await asyncio.sleep(0.6)
            s8seg = await ev("[...document.querySelectorAll('.info-manage .seg-item .seg-lbl')].map((x) => x.textContent)")
            check("S8-1 管理弹窗三段 = 基础信息/链接关系/自定义组件",
                  s8seg == ["基础信息", "链接关系", "自定义组件"], str(s8seg))
            s8rows = await ev("[...document.querySelectorAll('.info-manage .im-row .row-title')].map((x) => x.textContent)")
            check("S8-2 基础信息段含「提示词」行（注册表末位）",
                  s8rows == ["名称与链接", "标签", "详情", "基本数据", "提示词"], str(s8rows))
            await ev("document.querySelectorAll('.info-manage .seg-item')[2]?.click()")
            await asyncio.sleep(0.6)
            s8c = await ev("document.querySelectorAll('.info-manage .im-row').length")
            check("S8-3 自定义组件段 = 空行列表（占位容器）", s8c == 0, str(s8c))
            await ev("document.querySelector('.info-manage .bm-titlebar .ic-btn')?.click()")  # 关闭钮
            await asyncio.sleep(0.5)
            check("S8-4 弹窗关闭", await ev("!document.querySelector('.info-manage')"))

            # PromptSection 状态读取（基础信息 tab 末位区块）：编辑框文案 + 三钮 disabled
            #（无态 = 扫描/新建/指定，档案无扫描 = 两钮；有态 = 编辑/断开/删除）+ 复制钮 disabled；
            # 无提示词态编辑框整块隐藏（2026-10-08 定），text/placeholder/copy 均 null
            PROMPT_STATE = """(() => {
              const blk = document.querySelector('.prompt-block');
              if (!blk) return null;
              return {
                text: blk.querySelector('textarea.area')?.value ?? null,
                placeholder: blk.querySelector('textarea.area')?.placeholder ?? null,
                btns: [...blk.querySelectorAll('.btn-row button')].map((x) => x.disabled),
                copy: blk.querySelector('.copy-btn')?.disabled ?? null,
              };
            })()"""
            PB_BTN = lambda i: f"document.querySelectorAll('.prompt-block .btn-row button')[{i}]"

            async def close_preview():
                await ev("document.querySelector('.preview-root .preview-titlebar .nav-group .ic-btn')?.click()")  # 返回钮
                await asyncio.sleep(0.5)

            # 三态按钮图标 svg 采样（S5b-4 同源对照用）：none[0]=扫描 / linked[2]=删除提示词文档
            none_svgs = None
            linked_svgs = None

            # —— S5a 详情提示词区块·none（P1 png：扫描/新建/指定 三钮；复制禁用）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            st = await ev(PROMPT_STATE)
            check("S5a-1 none 态编辑框隐藏（无 textarea，2026-10-08 定）",
                  bool(st and st["text"] is None and st["placeholder"] is None), json.dumps(st, ensure_ascii=False))
            check("S5a-2 none 三钮全可用（png 过扫描格式闸）", bool(st and st["btns"] == [False, False, False]), json.dumps(st, ensure_ascii=False))
            check("S5a-3 none 态无复制钮（编辑框隐藏）", bool(st and st["copy"] is None), json.dumps(st, ensure_ascii=False))
            none_svgs = await ev("[...document.querySelectorAll('.prompt-block .btn-row button .app-icon')].map((x) => x.innerHTML)")
            await ev(f"({PB_BTN(0)})?.click()")  # 扫描提示词（详情入口，S2 后第二发）
            await asyncio.sleep(0.8)
            calls = await ev("JSON.stringify(window.__calls.analyze_asset_prompt)")
            check("S5a-4 详情「扫描提示词」→ analyze_asset_prompt(P1)",
                  calls == '[{"assetId":"P1"},{"assetId":"P1"}]', str(calls))
            # 新建提示词文档 → 弹二级（.md/.txt 带图标）→ 点 .md → invoke + 自动打开预览
            await ev(f"({PB_BTN(1)})?.click()")
            await asyncio.sleep(0.5)
            fl = await menu_labels()
            check("S5a-5 新建钮弹二级 = .md/.txt 两项", fl == ["新建 .md 提示词文档", "新建 .txt 提示词文档"], str(fl))
            check("S5a-6 二级两项带图标（mark-slot svg）",
                  await ev("[...document.querySelectorAll('.app-menu')[0]?.querySelectorAll('.menu-item') ?? []].every((x) => x.querySelector('.mark-slot svg') != null) && document.querySelectorAll('.app-menu')[0]?.querySelectorAll('.menu-item').length === 2"))
            await ev(f"({js_find('新建 .md 提示词文档')})?.click()")
            await asyncio.sleep(0.9)
            calls = await ev("JSON.stringify(window.__calls.new_asset_prompt_document)")
            check("S5a-7 点 .md 项 → new_asset_prompt_document{assetId:P1, folderId:f1, kind:md}",
                  calls == '[{"assetId":"P1","folderId":"f1","kind":"md"}]', str(calls))
            check("S5a-8 新建后自动打开预览（.preview-root）",
                  bool(await poll("!!document.querySelector('.preview-root')", tries=16)))
            await close_preview()
            # 指定提示词文件 → 进 picker（详情入口，横幅同菜单入口）
            await ev(f"({PB_BTN(2)})?.click()")
            await asyncio.sleep(0.8)
            s5pick = await ev("(() => ({ picking: document.querySelector('.window').classList.contains('picking'), banner: document.querySelector('.banner .text')?.textContent ?? null }))()")
            check("S5a-9 详情「指定提示词文件」→ picker（横幅文案同菜单入口）",
                  bool(s5pick and s5pick["picking"] and s5pick["banner"] == "选择文本文件，为 [走路.png] 指定提示词"),
                  json.dumps(s5pick, ensure_ascii=False))
            await esc_close()  # Esc 退 picker
            check("S5a-10 Esc 退 picker", await ev("!document.querySelector('.window').classList.contains('picking')"))

            # —— S5a-neg 扫描格式闸（F1 fla·none：扫描禁用，新建/指定可用）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("F1|f1")))
            await asyncio.sleep(0.9)
            st = await ev(PROMPT_STATE)
            check("S5a-11 fla 卡：扫描禁用（格式闸）、新建/指定可用",
                  bool(st and st["btns"] == [True, False, False]), json.dumps(st, ensure_ascii=False))

            # —— S4 详情提示词区块·linked（P3→T1）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("P3|f1")))
            await asyncio.sleep(0.9)
            st = await poll(f"(() => {{ const s = {PROMPT_STATE}; return s && s.text ? s : null; }})()")
            check("S4-1 linked 编辑框 = 源最新内容（read_text_asset 优先）",
                  bool(st and st["text"] == "源最新内容"), json.dumps(st, ensure_ascii=False))
            check("S4-2 linked 三钮全可用（编辑/断开/删除）", bool(st and st["btns"] == [False, False, False]), json.dumps(st, ensure_ascii=False))
            check("S4-3 linked 复制钮可用（有文本）", bool(st and st["copy"] is False), json.dumps(st, ensure_ascii=False))
            linked_svgs = await ev("[...document.querySelectorAll('.prompt-block .btn-row button .app-icon')].map((x) => x.innerHTML)")
            await ev("document.querySelector('.prompt-block')?.scrollIntoView({ block: 'end' })")
            await shot("prompt-section-linked")
            await ev("document.querySelector('.prompt-block .copy-btn')?.click()")
            await asyncio.sleep(0.5)
            check("S4-4 点复制钮 → 剪贴板写入源文本",
                  await ev("JSON.stringify(window.__calls.clipboard)") == '["源最新内容"]',
                  str(await ev("JSON.stringify(window.__calls.clipboard)")))
            await ev(f"({PB_BTN(1)})?.click()")  # 断开提示词链接
            await asyncio.sleep(0.8)
            calls = await ev("JSON.stringify(window.__calls.clear_asset_prompt)")
            check("S4-5 断开 → clear_asset_prompt{assetId:P3, label:断开提示词链接}",
                  calls == '[{"assetId":"P3","label":"断开提示词链接"}]', str(calls))
            await ev(f"({PB_BTN(2)})?.click()")  # 删除提示词文档
            await asyncio.sleep(0.8)
            calls = await ev("JSON.stringify(window.__calls.delete_asset_prompt_document)")
            check("S4-6 删除 → delete_asset_prompt_document{assetId:P3}",
                  calls == '[{"assetId":"P3"}]', str(calls))
            await ev(f"({PB_BTN(0)})?.click()")  # 编辑提示词 → 打开源文档预览
            await asyncio.sleep(0.9)
            check("S4-7 编辑 → 打开源文档预览（.preview-root）",
                  bool(await poll("!!document.querySelector('.preview-root')", tries=16)))
            await close_preview()

            # —— S5b 详情提示词区块·scanned（2026-09-26 三轮独立成套：保存提示词 / 重新扫描 / 删除提示词）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("P2|f1")))
            await asyncio.sleep(0.9)
            st = await poll(f"(() => {{ const s = {PROMPT_STATE}; return s && s.text ? s : null; }})()")
            check("S5b-1 scanned 编辑框 = 分析快照文案", bool(st and st["text"] == "扫描快照文案"), json.dumps(st, ensure_ascii=False))
            check("S5b-2 scanned 三钮全可用（保存/重新扫描/删除）", bool(st and st["btns"] == [False, False, False]), json.dumps(st, ensure_ascii=False))
            check("S5b-3 scanned 复制钮可用", bool(st and st["copy"] is False), json.dumps(st, ensure_ascii=False))
            # 图标 DOM 同源对照：重新扫描 = none 态「扫描提示词」同 icon（analyze-prompt）；
            # 删除提示词 = linked 态「删除提示词文档」同 icon（trash）；name/title 序静态断言见 S0c-2
            scanned_svgs = await ev("[...document.querySelectorAll('.prompt-block .btn-row button .app-icon')].map((x) => x.innerHTML)")
            check("S5b-4 图标同源：重新扫描=analyze-prompt / 删除提示词=trash（与 none/linked 同位 svg 一致）",
                  bool(scanned_svgs and none_svgs and linked_svgs and len(scanned_svgs) == 3
                       and scanned_svgs[1] == none_svgs[0] and scanned_svgs[2] == linked_svgs[2]
                       and scanned_svgs[0] != ""),
                  json.dumps([len(x or "") for x in (scanned_svgs or [])]))
            # 保存提示词 = 存为 .md 并绑定 → 转已指定（new_asset_prompt_document kind=md，单目标自动开预览）
            await ev(f"({PB_BTN(0)})?.click()")
            await asyncio.sleep(0.9)
            calls = await ev("JSON.stringify(window.__calls.new_asset_prompt_document)")
            check("S5b-5 保存提示词 → new_asset_prompt_document{assetId:P2, kind:md}",
                  bool(calls and json.loads(calls)[-1] == {"assetId": "P2", "folderId": "f1", "kind": "md"}), str(calls))
            check("S5b-6 保存后自动打开预览（.preview-root）", bool(await poll("!!document.querySelector('.preview-root')", tries=16)))
            await close_preview()
            # 重新扫描 = analyze_asset_prompt
            await ev(f"({PB_BTN(1)})?.click()")
            await asyncio.sleep(0.8)
            calls = await ev("JSON.stringify(window.__calls.analyze_asset_prompt)")
            check("S5b-7 重新扫描 → analyze_asset_prompt(P2)",
                  bool(calls and json.loads(calls)[-1] == {"assetId": "P2"}), str(calls))
            # 删除提示词 = 清空快照回未指定（clear_asset_prompt label「删除提示词」）
            await ev(f"({PB_BTN(2)})?.click()")
            await asyncio.sleep(0.8)
            calls = await ev("JSON.stringify(window.__calls.clear_asset_prompt)")
            check("S5b-8 删除提示词 → clear_asset_prompt{assetId:P2, label:删除提示词}",
                  calls == '[{"assetId":"P3","label":"断开提示词链接"},{"assetId":"P2","label":"删除提示词"}]', str(calls))

            # —— S4b linked 回落：read_text_asset 抛错 → 编辑框显示快照 content ——
            await ev("window.__readFail = true")
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(CARD("P3|f1")))
            await asyncio.sleep(0.9)
            st = await poll(f"(() => {{ const s = {PROMPT_STATE}; return s && s.text === '快照内容' ? s : null; }})()")
            check("S4b linked 读源失败回落快照 content", bool(st and st["text"] == "快照内容"), json.dumps(st, ensure_ascii=False))

            # —— S5c 档案详情提示词区块（a1·none）：无扫描钮（新建/指定两钮）——
            await ev("[...document.querySelectorAll('.toast')].forEach((x) => x.remove())")
            await js_click(el_center(TREE_NODE("a1")))
            await asyncio.sleep(0.9)
            st = await ev(PROMPT_STATE)
            check("S5c 档案 none：无扫描钮（新建/指定两钮可用）",
                  bool(st and st["btns"] == [False, False] and st["text"] == ""), json.dumps(st, ensure_ascii=False))

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
    raise SystemExit(asyncio.run(main()))
