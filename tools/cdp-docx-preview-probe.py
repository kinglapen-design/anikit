# CDP 实测：R6 docx 预览（2026-09-22，拓展性验证首例）。
# 场景：A docx 卡片双击 → kind=docx 分派 + previewSpecs 注册命中（.docx-preview 渲染，
# 标题/粗斜体/列表/空行/截断提示结构断言）；B showImg 白名单生效（thumbPath 桩非空但
# .preview-stage 下不渲染 img——排除式清单漏加新 kind 的回归点）；C 标题栏「默认应用打开」
# 判定式覆盖 docx；D 回归：png 卡片预览仍走 img（白名单不误伤 image kind）。
# 桩模式不验后端 zip 解析（cargo test services::docx 8 例已覆盖真实 zip 回路）。
import asyncio
import base64
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")  # 控制台 GBK 装不下 • 等字符

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9372
URL = "http://localhost:5373"

STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 2, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "D1", folderId: "root", name: "分镜脚本.docx", w: null, h: null, ext: "docx",
        thumbPath: "E:/T/thumbs/docx.webp", thumbKind: "formatIcon", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "P1", folderId: "root", name: "参考图.png", w: 100, h: 80, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_notes: [],
      prepare_anim_strips: { items: [] },
    };
    if (cmd === "read_docx_asset") {
      return Promise.resolve({
        paragraphs: [
          { heading: 1, list: false, runs: [{ text: "分镜脚本 & 备注", bold: false, italic: false }] },
          { heading: 2, list: false, runs: [{ text: "第一幕 登场", bold: false, italic: false }] },
          { heading: 0, list: false, runs: [
            { text: "主角从左侧入画，", bold: false, italic: false },
            { text: "快步", bold: true, italic: false },
            { text: "走向讲台。", bold: false, italic: false },
          ] },
          { heading: 0, list: true, runs: [{ text: "全景，2 秒", bold: false, italic: false }] },
          { heading: 0, list: false, runs: [] },
          { heading: 0, list: true, runs: [{ text: "斜体备注：注意光影", bold: false, italic: true }] },
        ],
        truncated: true,
      });
    }
    if (cmd === "get_asset_file_path") return Promise.resolve({ path: "E:/T/src.png", pending: false, animPreview: null });
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = [str(a.get("value", a.get("description", "")))[:200] for a in raw["params"]["args"]]
            print("[CONSOLE-ERR]", " ".join(args)[:400])


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-docx-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            cards = 0
            for _ in range(40):
                await asyncio.sleep(0.5)
                cards = await ev("document.querySelectorAll('.asset-card').length") or 0
                if cards > 0:
                    break
            if cards < 2:
                print("FAIL: 卡片未渲染（", cards, "）")
                return 3

            fails = []

            async def open_card(i, wait=2.0):
                await ev(f"document.querySelectorAll('.asset-card')[{i}].dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}))")
                await asyncio.sleep(wait)

            async def close_preview():
                await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
                await asyncio.sleep(0.8)

            async def shot(name):
                rect = await ev("(() => { const r = document.querySelector('.preview-stage')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width, height: r.height } : null; })()")
                if not rect:
                    return
                s = await send(ws, "Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 1},
                }, session=sess)
                out = f"tools/style-preview/{name}.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(s["result"]["data"]))
                print("截图:", out)

            # —— 场景 A/B/C：docx 预览结构 + showImg 白名单 + 打开按钮 ——
            await open_card(0, 2.0)
            a = await ev("""(() => {
              const stage = document.querySelector('.preview-stage');
              const root = document.querySelector('.docx-preview');
              if (!stage || !root) return null;
              const body = root.querySelector('.docx-body');
              return {
                imgInStage: stage.querySelectorAll('img').length,
                h1: body.querySelector('h1')?.textContent ?? null,
                h2: body.querySelector('h2')?.textContent ?? null,
                bold: [...body.querySelectorAll('.b')].map((s) => s.textContent),
                italic: [...body.querySelectorAll('.i')].map((s) => s.textContent),
                lists: body.querySelectorAll('.list').length,
                bullets: [...body.querySelectorAll('.bullet')].map((s) => s.textContent),
                blanks: body.querySelectorAll('.blank').length,
                paras: [...body.children].filter((c) => !c.classList.contains('truncated-hint') && !c.classList.contains('empty-hint')).length,
                truncated: root.querySelector('.truncated-hint')?.textContent ?? null,
                readCalls: window.__calls.filter((c) => c.cmd === 'read_docx_asset').map((c) => c.args.assetId),
              };
            })()""")
            print("[docx]", json.dumps(a, ensure_ascii=False))
            if not a:
                fails.append("[docx] 预览组件未渲染（kind 分派 / spec 注册未命中）")
            else:
                if a["imgInStage"] != 0:
                    fails.append(f"[docx] showImg 白名单失效：stage 内出现 img ×{a['imgInStage']}")
                if a["h1"] != "分镜脚本 & 备注":
                    fails.append(f"[docx] h1 不符（{a['h1']}）")
                if a["h2"] != "第一幕 登场":
                    fails.append(f"[docx] h2 不符（{a['h2']}）")
                if a["bold"] != ["快步"]:
                    fails.append(f"[docx] 粗体不符（{a['bold']}）")
                if a["italic"] != ["斜体备注：注意光影"]:
                    fails.append(f"[docx] 斜体不符（{a['italic']}）")
                if a["lists"] != 2 or a["bullets"] != ["•", "•"]:
                    fails.append(f"[docx] 列表不符（lists={a['lists']} bullets={a['bullets']}）")
                if a["blanks"] != 1:
                    fails.append(f"[docx] 空行段落 {a['blanks']}（期望 1）")
                if a["paras"] != 6:
                    fails.append(f"[docx] 段落总数 {a['paras']}（期望 6）")
                if a["truncated"] != "文档过大，仅显示前一部分":
                    fails.append(f"[docx] 截断提示不符（{a['truncated']}）")
                if a["readCalls"] != ["D1"]:
                    fails.append(f"[docx] read_docx_asset 调用不符（{a['readCalls']}）")
            # 「默认应用打开」按钮：IconButton title 走 v-tip 不落 DOM 属性——按按钮组位序点击
            # （插件 / 默认应用打开 / 新窗口 / 上 / 下一个，docx 非 viewable 无缩放组 = 索引 1），
            # 以 open_with_default_app 调用反证按钮存在且接线活
            await ev("document.querySelectorAll('.preview-titlebar .btn-group .ic-btn')[1]?.click()")
            await asyncio.sleep(0.5)
            od = await ev("window.__calls.filter((c) => c.cmd === 'open_with_default_app').map((c) => c.args.assetId)")
            print("[docx] 默认应用打开调用:", json.dumps(od, ensure_ascii=False))
            if od != ["D1"]:
                fails.append(f"[docx] 默认应用打开按钮缺失 / 接线不符（{od}）")
            await shot("docx-preview")
            await close_preview()

            # —— 场景 D：回归——png 卡片预览仍走 img（白名单不误伤 image kind） ——
            await open_card(1, 2.0)
            d = await ev("""(() => ({
              imgInStage: document.querySelectorAll('.preview-stage .img-wrap img').length,
              docxPane: !!document.querySelector('.docx-preview'),
            }))()""")
            print("[回归-png]", json.dumps(d, ensure_ascii=False))
            if not d or d["imgInStage"] != 1:
                fails.append(f"[回归] png 预览 img 缺失（{d}）")
            if d and d["docxPane"]:
                fails.append("[回归] png 预览挂出 docx 组件（spec 查表串线）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: R6 docx 预览链路成立（kind 分派 / spec 注册 / 结构渲染 / showImg 白名单 / 打开按钮判定式 / png 回归）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
