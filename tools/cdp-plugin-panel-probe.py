# CDP 实测：插件菜单 + 插件列表面板（2026-09-06 插件窗轮；同日二轮走查改定——菜单栏「插件」钮
# = 普通上下文菜单（menuSpecs/pluginMenu 四项两组），400×450 浮层面板只留工具栏 plugin 图标钮）
# P1 菜单栏「插件」钮出菜单（位置 = 钮左缘 + 底 +4 / 四项两组 / 三项 disabled / Animate 发
# open_plugin_window）；P1b 菜单三关闭（同钮 toggle / 点外 / Esc）；
# P2 工具栏 plugin 钮开浮层（位置 / 400×450 / 两项 / batch 禁用 / 默认桥接）+ 浮层三关闭；
# P3 分类分段切换过滤；P4 更多菜单（打开 / 安装动态文案 / 配置快捷键 disabled + 安装链路 toast 翻转）；
# P5 选择模式（picker.active）菜单栏钮禁开。
# 截图 plugin-menu / plugin-panel / plugin-more-menu → tools/style-preview/。
import asyncio
import base64
import json
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9373
URL = "http://localhost:5373"

# 桩同 cdp-app-menu-probe（2026-09-05 全量启动链路）+ 插件命令族（open_plugin_window /
# get_cep_install_status / install_cep_plugin / uninstall_cep_plugin；cepInstalled 有状态翻安装文案）
STUB = r"""
window.__calls = { open_plugin_window: [], install_cep_plugin: [], uninstall_cep_plugin: [] };
let cepInstalled = false;
const cepStatus = () => ({
  sourceAvailable: true, installed: cepInstalled, installedVersion: cepInstalled ? "1.0.0" : null,
  bundledVersion: "1.0.0", debugModeReady: true,
  installDir: "C:/Users/T/AppData/Roaming/Adobe/CEP/extensions",
});
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 1, children: [] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mkP = (assetId, folderId, name, ext, origin) => ({
  assetId, folderId, name, w: 100, h: 100, thumbW: null, thumbH: null, ext,
  thumbPath: null, thumbKind: null, origin, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, size: 1234, createdAt: null,
  modifiedAt: "2026-09-06T00:00:00Z", importedAt: "2026-09-06T00:00:00Z",
  noteCount: null, thumbFrame: null, fps: null, hidden: false, playAssetId: null,
});
const PLACEMENTS = [mkP("A1", "f1", "走路.png", "png", true)];
window.__cbId = 0; window.__cbs = {};
let libOpen = false;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    if (cmd === "open_plugin_window") {
      window.__calls.open_plugin_window.push(JSON.parse(JSON.stringify(args ?? {})));
      return Promise.resolve(null);
    }
    if (cmd === "get_cep_install_status") return Promise.resolve(cepStatus());
    if (cmd === "install_cep_plugin") {
      window.__calls.install_cep_plugin.push(JSON.parse(JSON.stringify(args ?? {})));
      cepInstalled = true;
      return Promise.resolve(cepStatus());
    }
    if (cmd === "uninstall_cep_plugin") {
      window.__calls.uninstall_cep_plugin.push(JSON.parse(JSON.stringify(args ?? {})));
      cepInstalled = false;
      return Promise.resolve(cepStatus());
    }
    if (cmd === "get_undo_state") return Promise.resolve({ canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" });
    if (cmd === "open_space") return Promise.resolve({ name: "S", path: "E:/T/S", autoImport: true });
    if (cmd === "open_library") {
      libOpen = true;
      return Promise.resolve({ name: "测试库", path: "E:/T/S/测试库", assetCount: 1, folderCount: 2 });
    }
    if (cmd === "get_current_library") {
      return Promise.resolve(libOpen ? { name: "测试库", path: "E:/T/S/测试库", assetCount: 1, folderCount: 2 } : null);
    }
    const R = {
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [],
      list_recent_assets: [],
      list_recent_libraries: [{ name: "测试库", path: "E:/T/S/测试库", lastOpenedAt: "" }],
      list_inbox_snapshot: [],
      list_archives: [],
      list_space_libraries: ["测试库"],
      list_prop_pool: [],
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
        # 60s 无回应 = 卡死判负（2026-09-06 视图菜单轮实证：captureScreenshot 可挂起——
        # 超时抛异常走 finally 收 Edge，已跑部分留档）
        raw = json.loads(await asyncio.wait_for(ws.recv(), timeout=60))
        if raw.get("id") == mid:
            return raw
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = raw["params"]["args"]
            print("[CONSOLE-ERR]", str([a.get("value", a.get("description", "?")) for a in args])[:300])


async def main():
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
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, button="left", modifiers=0):
                # buttons = 事件后的按键状态：release/move 恒 0（2026-09-05 主探针实证口径）
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": button, "modifiers": modifiers, "buttons": 0 if t in ("mouseReleased", "mouseMoved") else (2 if button == "right" else 1), "x": x, "y": y, "clickCount": 1}, session=sess)

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))

            def el_center(expr):
                return f"(() => {{ const el = {expr}; if (!el) return null; const r = el.getBoundingClientRect(); return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }}; }})()"

            async def js_click(expr):
                r = await ev(expr)
                if r is None:
                    return None
                await mouse("mousePressed", r["x"], r["y"]); await mouse("mouseReleased", r["x"], r["y"])
                await asyncio.sleep(0.5)
                return r

            async def esc_close():
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await asyncio.sleep(0.4)

            # 菜单栏「插件」文字钮 / 工具栏 plugin 图标钮（btn-group 第 2 个：add-link 之后）
            PLUGIN_BTN = "[...document.querySelectorAll('.sidebar-left .menu-bar .act-btn')].find((b) => b.textContent.trim() === '插件')"
            TOOLBAR_PLUGIN_BTN = "document.querySelectorAll('.panel-toolbar .btn-group .ic-btn')[1]"
            PANEL = "document.querySelector('.plugin-panel')"
            MENU = "[...document.querySelectorAll('.app-menu')].find((m) => m.querySelector('.menu-item'))"

            async def panel_open():
                return await ev("document.querySelector('.plugin-panel') != null")

            async def menu_open():
                return await ev(f"({MENU}) != null")

            async def open_panel():
                # 浮层入口 = 工具栏 plugin 图标钮（二轮改定：菜单栏钮改出菜单，浮层只留工具栏）
                await js_click(el_center(TOOLBAR_PLUGIN_BTN))
                return await panel_open()

            async def open_menu():
                await js_click(el_center(PLUGIN_BTN))
                return await menu_open()

            # 启动就绪闸：菜单栏四钮齐 + 内容工具栏在（库重开链完成）
            for _ in range(20):
                if await ev(f"({PLUGIN_BTN}) != null && ({TOOLBAR_PLUGIN_BTN}) != null"):
                    break
                await asyncio.sleep(0.4)

            # P1 菜单栏「插件」钮出上下文菜单（二轮改定）：位置 / 四项两组 / 三项 disabled / Animate 发命令
            check("P1-1 点「插件」钮菜单出", await open_menu(), "app-menu 未出现")
            geo = await ev(f"(() => {{ const m = {MENU}; const b = {PLUGIN_BTN}; if (!m || !b) return null; const mr = m.getBoundingClientRect(); const br = b.getBoundingClientRect(); return {{ ml: mr.left, mt: mr.top, bl: br.left, bb: br.bottom }}; }})()")
            check("P1-2 位置 = 钮左缘对齐 + 底 +4", geo is not None and abs(geo["ml"] - geo["bl"]) <= 1 and abs(geo["mt"] - geo["bb"] - 4) <= 1, str(geo))
            labels = await ev(f"[...{MENU}?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)")
            expect = ["Animate", "批量生成缩略图", "API 接口文档", "开发者选项"]
            check("P1-3 四项两组", json.dumps(labels, ensure_ascii=False) == json.dumps(expect, ensure_ascii=False) and await ev(f"{MENU}.querySelectorAll('.menu-group').length === 2"), str(labels))
            disabled = await ev(f"[...{MENU}.querySelectorAll('.menu-item')].filter((m) => m.disabled === true).map((m) => m.querySelector('.label')?.textContent)")
            check("P1-4 三项 disabled（批量缩略图 / API / 开发者选项）", json.dumps(disabled, ensure_ascii=False) == json.dumps(expect[1:], ensure_ascii=False), str(disabled))
            check("P1-5 Animate 可用（未禁用）", await ev(f"[...{MENU}.querySelectorAll('.menu-item')].find((m) => m.querySelector('.label')?.textContent === 'Animate')?.disabled === false"))
            await shot("plugin-menu")
            await js_click(el_center(f"[...{MENU}.querySelectorAll('.menu-item')].find((m) => m.querySelector('.label')?.textContent === 'Animate')"))
            calls = await ev("window.__calls.open_plugin_window")
            check("P1-6 点 Animate = invoke open_plugin_window（pluginId animate）", json.dumps(calls) == json.dumps([{"pluginId": "animate"}]), str(calls))
            check("P1-7 点击后菜单关闭", not await menu_open())

            # P1b 菜单三关闭：同钮 toggle / 点外 / Esc
            check("P1b-0 前置：再开菜单", await open_menu())
            await js_click(el_center(PLUGIN_BTN))
            check("P1b-1 同钮再点 toggle 关", not await menu_open())
            check("P1b-2 前置：再开菜单", await open_menu())
            await mouse("mousePressed", 700, 500); await mouse("mouseReleased", 700, 500)
            await asyncio.sleep(0.4)
            check("P1b-3 点菜单外关闭", not await menu_open())
            check("P1b-4 前置：再开菜单", await open_menu())
            await esc_close()
            check("P1b-5 Esc 关闭", not await menu_open())

            # P2 工具栏 plugin 钮开浮层（浮层唯一入口）：位置 / 尺寸 / 两项 / batch 禁用 / 默认桥接
            check("P2-1 工具栏 plugin 钮开面板", await open_panel())
            geo = await ev(f"(() => {{ const p = {PANEL}; const b = {TOOLBAR_PLUGIN_BTN}; if (!p || !b) return null; const pr = p.getBoundingClientRect(); const br = b.getBoundingClientRect(); return {{ pt: pr.top, pr: pr.right, pw: p.offsetWidth, ph: p.offsetHeight, bb: br.bottom, br: br.right }}; }})()")
            check("P2-2 弹出在钮下方 +4", geo is not None and abs(geo["pt"] - geo["bb"] - 4) <= 1, str(geo))
            check("P2-2b 右缘 = 钮右缘对齐（三轮改定向左展开）", geo is not None and abs(geo["pr"] - geo["br"]) <= 1, str(geo))
            check("P2-3 尺寸 400×450", geo is not None and geo["pw"] == 400 and geo["ph"] == 450, str(geo))
            names = await ev("[...document.querySelectorAll('.plugin-panel .plugin-item .p-name')].map((x) => x.textContent)")
            check("P2-4 列表两项（Animate 插件 / 批量生成缩略图）", json.dumps(names, ensure_ascii=False) == json.dumps(["Animate 插件", "批量生成缩略图"], ensure_ascii=False), str(names))
            check("P2-5 第二项禁用态（disabled 类 + pointer-events none）", await ev("(() => { const it = document.querySelectorAll('.plugin-panel .plugin-item')[1]; return it?.classList.contains('disabled') && getComputedStyle(it).pointerEvents === 'none'; })()"))
            check("P2-6 默认选中「桥接」分段", await ev("[...document.querySelectorAll('.plugin-panel .seg-item')].find((s) => s.textContent.trim() === '桥接')?.classList.contains('on') === true"))
            await shot("plugin-panel")
            # 浮层三关闭（工具栏钮 toggle / 点外 / Esc）
            await js_click(el_center(TOOLBAR_PLUGIN_BTN))
            check("P2-7 同钮再点 toggle 关", not await panel_open())
            check("P2-8 前置：再开", await open_panel())
            await mouse("mousePressed", 500, 500); await mouse("mouseReleased", 500, 500)
            await asyncio.sleep(0.4)
            check("P2-9 点面板外关闭", not await panel_open())
            check("P2-10 前置：再开", await open_panel())
            await esc_close()
            check("P2-11 Esc 关闭", not await panel_open())

            # P3 分类分段切换过滤
            check("P3-0 前置：再开", await open_panel())
            await js_click(el_center("[...document.querySelectorAll('.plugin-panel .seg-item')].find((s) => s.textContent.trim() === '格式')"))
            check("P3-1 切「格式」= 空态「暂无插件」", await ev("document.querySelector('.plugin-panel .list-empty')?.textContent === '暂无插件'"))
            check("P3-2 空态下无 item", await ev("document.querySelectorAll('.plugin-panel .plugin-item').length === 0"))
            await js_click(el_center("[...document.querySelectorAll('.plugin-panel .seg-item')].find((s) => s.textContent.trim() === '桥接')"))
            check("P3-3 切回「桥接」= 两项回来", await ev("document.querySelectorAll('.plugin-panel .plugin-item').length === 2"))

            # P4 更多菜单：三项 + 禁用 + 打开发命令 + 安装链路文案翻转
            await js_click(el_center("document.querySelector('.plugin-panel .plugin-item .more-btn')"))
            ctx = "[...document.querySelectorAll('.app-menu')].find((m) => m.querySelector('.menu-item'))"
            labels = await ev(f"[...{ctx}?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)")
            check("P4-1 更多菜单三项（打开 / 安装 / 配置快捷键）", json.dumps(labels, ensure_ascii=False) == json.dumps(["打开", "安装", "配置快捷键"], ensure_ascii=False), str(labels))
            check("P4-2 配置快捷键 disabled 占位", await ev(f"[...{ctx}.querySelectorAll('.menu-item')].find((m) => m.textContent.includes('配置快捷键'))?.disabled === true"))
            await shot("plugin-more-menu")
            await js_click(el_center(f"[...{ctx}.querySelectorAll('.menu-item')].find((m) => m.querySelector('.label')?.textContent === '打开')"))
            calls = await ev("window.__calls.open_plugin_window")
            check("P4-3 打开 = invoke open_plugin_window（P1 菜单一次 + 本次 = 两条）", json.dumps(calls) == json.dumps([{"pluginId": "animate"}, {"pluginId": "animate"}]), str(calls))
            check("P4-4 打开后浮层与菜单同关", not await panel_open() and await ev("document.querySelectorAll('.app-menu').length === 0"))
            # 安装链路：未安装 =「安装」→ 点击发 install_cep_plugin + toast；再开菜单文案翻「卸载」
            check("P4-5 前置：再开面板", await open_panel())
            await js_click(el_center("document.querySelector('.plugin-panel .plugin-item .more-btn')"))
            await js_click(el_center(f"[...{ctx}.querySelectorAll('.menu-item')].find((m) => m.querySelector('.label')?.textContent === '安装')"))
            await asyncio.sleep(0.5)
            check("P4-6 安装 = invoke install_cep_plugin", await ev("window.__calls.install_cep_plugin.length === 1"))
            check("P4-7 安装成功 toast", await ev("[...document.querySelectorAll('.toast')].some((t) => t.textContent.includes('CEP 插件已安装'))"), str(await ev("[...document.querySelectorAll('.toast')].map((t) => t.textContent.trim())")))
            await ev("document.querySelectorAll('.toast .close').forEach((b) => b.click())")
            check("P4-8 前置：再开面板", await open_panel())
            await js_click(el_center("document.querySelector('.plugin-panel .plugin-item .more-btn')"))
            labels = await ev(f"[...{ctx}?.querySelectorAll('.menu-item .label') ?? []].map((x) => x.textContent)")
            check("P4-9 已安装文案翻「卸载」", json.dumps(labels, ensure_ascii=False) == json.dumps(["打开", "卸载", "配置快捷键"], ensure_ascii=False), str(labels))
            await esc_close()

            # P5 选择模式禁开：进文件夹视口 → 工具栏「添加文件/档案链接」进 picker → 菜单栏插件钮不开菜单
            await js_click(el_center("document.querySelector('.block.tree .tree-item[data-folder-id=\"f1\"]')"))
            await asyncio.sleep(0.6)
            await js_click(el_center("document.querySelectorAll('.panel-toolbar .btn-group .ic-btn')[0]"))
            await asyncio.sleep(0.6)
            check("P5-0 前置：picker 已进入（.window.picking）", await ev("document.querySelector('.window')?.classList.contains('picking') === true"))
            await js_click(el_center(PLUGIN_BTN))
            check("P5-1 选择模式下点「插件」不开菜单", not await menu_open())
            await esc_close()  # 退出 picker 清场
            check("P5-2 Esc 退出 picker 清场", await ev("document.querySelector('.window')?.classList.contains('picking') === false"))

            print("----")
            print("FAIL:", fails if fails else "无")
            return 1 if fails else 0
    finally:
        proc.kill()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
