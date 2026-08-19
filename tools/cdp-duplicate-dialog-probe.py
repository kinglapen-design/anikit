# CDP 实测：重复文件提示弹窗 = figma 36_重复文件提示 稿（2026-08-13，2.7 第一轮）
# 桩有库进主界面 → 新增菜单"导入本地文件" → 导入弹窗确认 → import_paths 桩经 Channel 回调
# 推 finished（3 条冲突桩数据）→ DuplicateDialog 出现。
# 校验：standard 壳 970×878 / 表头双列 / 行数 / 默认 radio=作为新链接导入 / 全部跟随（开=改一行全跟随，
# 关=逐行独立）/ 跳过压暗且该项 action=skip / 内嵌重命名区已废除（2026-08-13 用户定过度设计）/
# 确认提交 items（无 newName）/ 取消 = 全部跳过不调 resolve_conflicts。
# 截图留档 tools/style-preview/duplicate-dialog.png。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9346
URL = "http://localhost:5373"

# Channel 桩：transformCallback 把回调存 window.__cbs；import_paths 捕获 onEvent 的 channel id，
# 300ms 后主动推 started + finished（带 3 条冲突）→ 前端 onImportEvent 填 importConflicts 弹窗
STUB = r"""
window.__cbs = {};
window.__cbId = 0;
window.__resolvedCalls = [];
window.__conflicts = [
  { sourcePath: "E:/T/场景A.png", sourceName: "场景A.png", sourceSize: 1258291, sourceW: 800, sourceH: 450,
    sourceThumbPath: null, targetFolder: "root", targetFolderName: "根目录",
    existingAssetId: "AN-A", existingName: "旧场景A.png", existingW: 800, existingH: 450,
    existingSize: 786432, existingExt: "png", existingThumbPath: null, existingFolderName: "人物", sequence: null },
  { sourcePath: "E:/T/场景B.png", sourceName: "场景B.png", sourceSize: 2345678, sourceW: 1920, sourceH: 1080,
    sourceThumbPath: null, targetFolder: "root", targetFolderName: "根目录",
    existingAssetId: "AN-B", existingName: "旧场景B.png", existingW: 1920, existingH: 1080,
    existingSize: 1987654, existingExt: "png", existingThumbPath: null, existingFolderName: "场景", sequence: null },
  { sourcePath: "E:/T/参考C.psd", sourceName: "参考C.psd", sourceSize: 5242880, sourceW: null, sourceH: null,
    sourceThumbPath: null, targetFolder: "root", targetFolderName: "根目录",
    existingAssetId: "AN-C", existingName: "旧参考C.psd", existingW: null, existingH: null,
    existingSize: 4194304, existingExt: "psd", existingThumbPath: null, existingFolderName: "参考", sequence: null },
];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    (window.__cmds = window.__cmds || []).push(cmd);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: false };
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      classify_paths: { kind: "files", fileCount: 2 },
      detect_sequences: [],
    };
    if (cmd === "plugin:dialog|open") return Promise.resolve(["E:/T/场景A.png", "E:/T/场景B.png", "E:/T/参考C.psd"]);
    if (cmd === "import_paths") {
      // 桩 invoke 收到的是未序列化的 Channel 实例（真实 Tauri 才会 toJSON 成 "__CHANNEL__:id"）
      const chId = typeof args.onEvent === "object" ? args.onEvent.id : Number(String(args.onEvent).split(":").pop());
      setTimeout(() => {
        const cb = window.__cbs[chId];
        if (!cb) return;
        // 落点按 __target 切换（第一轮文件夹 F01；第二轮 root——落点不设限，三选一恒出，2026-08-13）
        const t = window.__target ?? "F01";
        const conflicts = window.__conflicts.map((c) => ({ ...c, targetFolder: t, targetFolderName: t === "root" ? "根目录" : "人物" }));
        // Channel 消息包装形状：{ index, message }（@tauri-apps/api core.js Channel）
        cb({ index: 0, message: { type: "started", total: 3 } });
        cb({ index: 1, message: { type: "finished", imported: 0, conflicts, failed: [] } });
        window.__pushed = (window.__pushed || 0) + 1;
      }, 300);
      return Promise.resolve(1);
    }
    if (cmd === "resolve_conflicts") {
      window.__resolvedCalls.push(args);
      return Promise.resolve({ resolved: args.items.length, failed: [] });
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
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


async def main():
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,1200", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
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

            async def click(expr):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            fails = []

            # 进弹窗：菜单"导入本地文件" → 导入弹窗确认 → 桩推 finished（3 冲突）
            await click("document.querySelector('.add-btn-wrap .icon-btn')")
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('导入本地文件'))")
            await asyncio.sleep(0.8)
            print("导入弹窗:", await ev("!!document.querySelector('.app-modal.plain')"))
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(1.5)
            print("cbId:", await ev("window.__cbId"), "冲突弹窗:", await ev("!!document.querySelector('.dup-dialog .app-modal')"))
            print("cmds:", await ev("(window.__cmds || []).join(',')"), "pushed:", await ev("window.__pushed ?? 0"))

            INFO = """({
              title: document.querySelector('.app-modal .title')?.textContent ?? null,
              width: document.querySelector('.app-modal')?.getBoundingClientRect().width ?? null,
              height: document.querySelector('.app-modal')?.getBoundingClientRect().height ?? null,
              hasCloseX: !!document.querySelector('.app-modal .titlebar .icon-btn'),
              heads: [...document.querySelectorAll('.dup-head .col')].map((c) => c.textContent.trim()),
              rows: document.querySelectorAll('.dup-row').length,
              radiosPerRow: document.querySelectorAll('.dup-row')[0]?.querySelectorAll('.app-radio').length ?? 0,
              onLabels: [...document.querySelectorAll('.dup-row')].map((r) => r.querySelector('.app-radio.on')?.textContent.trim() ?? null),
              newNames: [...document.querySelectorAll('.dup-row .file-card.new .fname')].map((n) => n.textContent.trim()),
              chips: [...document.querySelectorAll('.dup-row')[0]?.querySelectorAll('.chip') ?? []].map((c) => c.textContent.trim()),
              noRenameBar: !document.querySelector('.rename-bar') && !document.querySelector('.dup-bottom .app-seg'),
              followOn: document.querySelector('.dup-bottom .app-switch')?.classList.contains('on') ?? null,
              buttons: [...document.querySelectorAll('.dup-bottom .app-btn')].map((b) => b.textContent.trim()),
            })"""
            info = await ev(INFO)
            print("弹窗:", json.dumps(info, ensure_ascii=False))
            if info is None or info["title"] != "重复文件提示" or info["rows"] != 3:
                fails.append("弹窗未出现或行数错误")
                print("FAIL:", "; ".join(fails))
                return 3
            if info["width"] != 970 or info["height"] != 878:
                fails.append(f"尺寸 {info['width']}x{info['height']}")
            if not info["hasCloseX"]:
                fails.append("标题栏无关闭 X")
            if info["heads"] != ["已存在文件", "新的文件"]:
                fails.append(f"表头 {info['heads']}")
            if info["radiosPerRow"] != 3 or info["onLabels"] != ["作为新链接导入"] * 3:
                fails.append(f"默认 radio {info['onLabels']}")
            if info["newNames"] != ["场景A.png", "场景B.png", "参考C.psd"]:
                fails.append(f"右卡源名 {info['newNames']}")
            if info["chips"] != ["人物", "人物"]:
                fails.append(f"位置标签 {info['chips']}")
            # 2026-08-13 用户定：内嵌批量重命名过度设计废除——底部不应再有重命名区/分段选择器
            if not info["noRenameBar"]:
                fails.append("重命名区仍在（应已废除）")
            if info["followOn"] is not True:
                fails.append("全部跟随默认应开")
            if info["buttons"] != ["确认", "取消"]:
                fails.append(f"按钮序 {info['buttons']}")

            # ① 全部跟随（开）：点第 2 行"作为新文件导入" → 全部行跟随
            await click("document.querySelectorAll('.dup-row')[1].querySelectorAll('.app-radio')[1]")
            on = await ev("[...document.querySelectorAll('.dup-row')].map((r) => r.querySelector('.app-radio.on')?.textContent.trim())")
            print("跟随后:", on)
            if on != ["作为新文件导入"] * 3:
                fails.append(f"全部跟随失败 {on}")

            # ② 关全部跟随 → 点第 1 行"用已存在文件导入" → 只第 1 行变
            await click("document.querySelector('.dup-bottom .app-switch')")
            await click("document.querySelectorAll('.dup-row')[0].querySelectorAll('.app-radio')[2]")
            on = await ev("[...document.querySelectorAll('.dup-row')].map((r) => r.querySelector('.app-radio.on')?.textContent.trim())")
            print("逐行后:", on)
            if on != ["用已存在文件导入", "作为新文件导入", "作为新文件导入"]:
                fails.append(f"逐行独立失败 {on}")

            # ③ 跳过第 3 行：行压暗（skipped class）
            await click("document.querySelectorAll('.dup-row')[2].querySelector('.skip .app-switch')")
            skipped = await ev("[...document.querySelectorAll('.dup-row')].map((r) => r.classList.contains('skipped'))")
            opacity = await ev("getComputedStyle(document.querySelectorAll('.dup-row')[2]).opacity")
            print("跳过态:", skipped, "透明度:", opacity)
            if skipped != [False, False, True] or opacity != "0.3":
                fails.append(f"跳过压暗 {skipped} {opacity}")

            # ④ 截图留档（重命名区已废除，无改名预览步骤）
            await shot("duplicate-dialog")

            # ⑤ 确认：resolve_conflicts 收到 3 项——行1 useExisting / 行2 copy / 行3 skip（newName 已废除）
            await click("[...document.querySelectorAll('.dup-bottom .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(1.0)
            calls = await ev("window.__resolvedCalls")
            closed = await ev("!document.querySelector('.dup-dialog .app-modal')")
            print("确认提交:", json.dumps(calls, ensure_ascii=False), "弹窗关闭:", closed)
            if not calls or len(calls) != 1:
                fails.append("resolve_conflicts 未调用")
            else:
                items = calls[0]["items"]
                ok = (
                    len(items) == 3
                    and items[0]["action"] == "useExisting"
                    and items[1]["action"] == "copy"
                    and items[2]["action"] == "skip"
                    and all("newName" not in it for it in items)
                    and calls[0]["source"] == "copy"
                )
                if not ok:
                    fails.append(f"提交项不符 {json.dumps(items, ensure_ascii=False)}")
            if not closed:
                fails.append("确认后弹窗未关闭")

            # ⑥ 落点不设限（2026-08-13 用户定案：落点 root 同样三选一恒出，默认新链接——
            # 候车区收窄废除）：重开（__target=root）→ 断言每行 3 radio 且默认 link → 取消 = 全部跳过
            await ev("window.__target = 'root'")
            await click("document.querySelector('.add-btn-wrap .icon-btn')")
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.includes('导入本地文件'))")
            await asyncio.sleep(0.8)
            await click("[...document.querySelectorAll('.app-modal .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(1.5)
            reopened = await ev("!!document.querySelector('.dup-dialog .app-modal')")
            wa = await ev("""({
              radiosPerRow: document.querySelectorAll('.dup-row')[0]?.querySelectorAll('.app-radio').length ?? 0,
              onLabels: [...document.querySelectorAll('.dup-row')].map((r) => r.querySelector('.app-radio.on')?.textContent.trim() ?? null),
            })""")
            print("落点不设限:", json.dumps(wa, ensure_ascii=False))
            if wa["radiosPerRow"] != 3 or wa["onLabels"] != ["作为新链接导入"] * 3:
                fails.append(f"落点不设限 {json.dumps(wa, ensure_ascii=False)}")
            await click("[...document.querySelectorAll('.dup-bottom .app-btn')].find((b) => b.textContent.includes('取消'))")
            await asyncio.sleep(0.8)
            calls2 = await ev("window.__resolvedCalls.length")
            closed2 = await ev("!document.querySelector('.dup-dialog .app-modal')")
            print("取消路径: 重开", reopened, "调用数", calls2, "关闭", closed2)
            if not reopened or calls2 != 1 or not closed2:
                fails.append(f"取消=全部跳过 {reopened} {calls2} {closed2}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 布局/默认态/跟随/跳过/落点不设限/提交/取消均贴稿")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
