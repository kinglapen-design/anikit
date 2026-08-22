# CDP 实测：5.6 字体 / Markdown / txt 预览（2026-08-23）。
# 场景：A 字体预览页（页签 + 演示文本 + 字号滑杆）/ B 字体其它页（元数据 7 行 + 安装按钮流转
# install_font 桩断言）/ C md 渲染结构（h1/strong/code/li/blockquote + 链接不可点）/
# D txt 编辑防抖 1s 自动保存（write_text_asset 桩断言 + 已保存状态字）。
# 桩环境无真字体文件：FontFace load 假路径必失败（loadError 出现属预期），元数据与结构断言不受影响。
import asyncio
import base64
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9371
URL = "http://localhost:5373"

STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 4, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "F0", folderId: "root", name: "思源黑体.ttf", w: 640, h: 480, ext: "ttf",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "T1", folderId: "root", name: "提示词.md", w: null, h: null, ext: "md",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "T2", folderId: "root", name: "笔记.txt", w: null, h: null, ext: "txt",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "A3", folderId: "root", name: "参考图.png", w: 100, h: 80, ext: "png",
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
    if (cmd === "get_font_info") {
      return Promise.resolve({ originPath: "E:/T/font.ttf", name: "TestSans-Regular", fullName: "Test Sans Regular",
        family: "Test Sans", style: "Regular", version: "Version 1.001", glyphCount: 401, installed: false });
    }
    if (cmd === "install_font") return Promise.resolve({ installed: true });
    if (cmd === "uninstall_font") return Promise.resolve({ installed: false });
    if (cmd === "read_text_asset") {
      if (args.assetId === "T1") {
        return Promise.resolve({ content: "# 标题\n\n正文 **加粗** 与 `code`。\n\n- 列表一\n- 列表二\n\n> 引用文本\n\n[链接](https://example.com)" });
      }
      return Promise.resolve({ content: "第一行提示词" });
    }
    if (cmd === "write_text_asset") return Promise.resolve({ size: (args.content ?? "").length, modifiedAt: "2026-08-23T12:00:00+08:00" });
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
    profile = tempfile.mkdtemp(prefix="cdp-font-text-")
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
            if cards < 4:
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

            # —— 场景 A：字体预览页（页签 + 演示文本 + 字号滑杆） ——
            await open_card(0, 2.0)
            a = await ev("""(() => {
              const root = document.querySelector('.font-preview');
              if (!root) return null;
              return {
                tabs: [...root.querySelectorAll('.tab-bar .item')].map((b) => b.textContent),
                title: root.querySelector('.demo-title')?.textContent ?? null,
                bodies: root.querySelectorAll('.demo-body').length,
                slider: !!root.querySelector('.size-bar input[type=range], .size-bar .side-slider'),
                titleSize: root.querySelector('.demo-title')?.style.fontSize ?? null,
              };
            })()""")
            print("[字体-预览页]", json.dumps(a, ensure_ascii=False))
            if not a:
                fails.append("[字体] 预览组件未渲染")
            else:
                if a["tabs"] != ["预览", "其它"]:
                    fails.append(f"[字体] 页签不符（{a['tabs']}）")
                if not a["title"] or a["bodies"] < 2:
                    fails.append(f"[字体] 演示文本结构不全（title={a['title']} bodies={a['bodies']}）")
                if not a["slider"]:
                    fails.append("[字体] 字号滑杆缺失")
                if a["titleSize"] != "44px":
                    fails.append(f"[字体] 标题字号 {a['titleSize']}（期望 44px = 正文 22×2）")
            await shot("font-preview")

            # —— 场景 B：字体其它页（元数据 + 安装流转） ——
            await ev("""(() => {
              [...document.querySelectorAll('.font-preview .tab-bar .item')].find((b) => b.textContent === '其它')?.click();
            })()""")
            await asyncio.sleep(0.5)
            b = await ev("""(() => {
              const rows = [...document.querySelectorAll('.font-preview .info-row')].map((r) => ({
                k: r.querySelector('.k')?.textContent, v: r.querySelector('.v')?.textContent,
              }));
              const btn = document.querySelector('.font-preview .install-bar .app-btn');
              return { rows, btn: btn?.textContent?.trim() ?? null };
            })()""")
            print("[字体-其它页]", json.dumps(b, ensure_ascii=False))
            if not b or not b["rows"]:
                fails.append("[字体] 其它页元数据未渲染")
            else:
                ks = [r["k"] for r in b["rows"]]
                if ks != ["名称", "全名", "系列", "样式", "版本", "字数", "系统状态"]:
                    fails.append(f"[字体] 元数据行不符（{ks}）")
                if not any(r["v"] == "TestSans-Regular" for r in b["rows"]):
                    fails.append("[字体] 名称值缺失")
                if b["btn"] != "安装到系统":
                    fails.append(f"[字体] 安装钮文案 {b['btn']}（期望 安装到系统）")
            # 点安装 → 桩调用 + 按钮翻转 + 系统状态行更新
            await ev("document.querySelector('.font-preview .install-bar .app-btn')?.click()")
            await asyncio.sleep(0.6)
            b2 = await ev("""(() => ({
              btn: document.querySelector('.font-preview .install-bar .app-btn')?.textContent?.trim() ?? null,
              status: [...document.querySelectorAll('.font-preview .info-row')].map((r) => r.querySelector('.v')?.textContent).pop() ?? null,
              calls: window.__calls.filter((c) => c.cmd === 'install_font').length,
            }))()""")
            print("[字体-安装后]", json.dumps(b2, ensure_ascii=False))
            if b2["calls"] != 1:
                fails.append(f"[字体] install_font 调用 {b2['calls']} 次（期望 1）")
            if b2["btn"] != "从系统卸载":
                fails.append(f"[字体] 安装后按钮 {b2['btn']}（期望 从系统卸载）")
            if b2["status"] != "已安装到系统":
                fails.append(f"[字体] 系统状态 {b2['status']}（期望 已安装到系统）")
            await shot("font-info")
            await close_preview()

            # —— 场景 C：md 渲染结构 ——
            await open_card(1, 2.0)
            c = await ev("""(() => {
              const root = document.querySelector('.text-preview .md-body');
              if (!root) return null;
              return {
                h1: root.querySelector('h1')?.textContent ?? null,
                strong: root.querySelector('strong')?.textContent ?? null,
                code: root.querySelector('code')?.textContent ?? null,
                li: root.querySelectorAll('li').length,
                quote: root.querySelector('blockquote')?.textContent?.trim() ?? null,
                linkPeek: getComputedStyle(root.querySelector('a')).pointerEvents,
              };
            })()""")
            print("[md]", json.dumps(c, ensure_ascii=False))
            if not c:
                fails.append("[md] 渲染容器未渲染")
            else:
                if c["h1"] != "标题" or c["strong"] != "加粗" or c["code"] != "code":
                    fails.append(f"[md] 行内结构不符（{c}）")
                if c["li"] != 2 or not c["quote"]:
                    fails.append(f"[md] 列表/引用不符（li={c['li']} quote={c['quote']}）")
                if c["linkPeek"] != "none":
                    fails.append(f"[md] 链接应不可点（pointerEvents={c['linkPeek']}）")
            await shot("text-md")
            await close_preview()

            # —— 场景 D：txt 编辑防抖自动保存 ——
            await open_card(2, 2.0)
            d0 = await ev("document.querySelector('.text-preview .editor')?.value ?? null")
            print("[txt] 初始值:", json.dumps(d0, ensure_ascii=False))
            if d0 != "第一行提示词":
                fails.append(f"[txt] 初始内容 {d0}（期望 第一行提示词）")
            await ev("""(() => {
              const t = document.querySelector('.text-preview .editor');
              t.value = '第一行提示词\\n追加一行';
              t.dispatchEvent(new Event('input', { bubbles: true }));
            })()""")
            await asyncio.sleep(1.6)  # 防抖 1s + 余量
            # 防抖 timer 可能受 headless 节流影响：轮询兜底（最多 6s）
            d1 = None
            for _ in range(20):
                d1 = await ev("""(() => ({
                  writes: window.__calls.filter((c) => c.cmd === 'write_text_asset').map((c) => c.args.content),
                  state: document.querySelector('.text-preview .save-state')?.textContent ?? null,
                }))()""")
                if d1["writes"]:
                    break
                await asyncio.sleep(0.3)
            if not d1["writes"]:
                cmds = await ev("window.__calls.map((c) => c.cmd)")
                print("[txt] 诊断 __calls:", json.dumps(cmds, ensure_ascii=False))
            print("[txt] 防抖保存:", json.dumps(d1, ensure_ascii=False))
            if d1["writes"] != ["第一行提示词\n追加一行"]:
                fails.append(f"[txt] write_text_asset 调用不符（{d1['writes']}）")
            if d1["state"] != "已保存":
                fails.append(f"[txt] 保存状态 {d1['state']}（期望 已保存）")
            await shot("text-txt")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 5.6 字体/Markdown/txt 预览链路成立（页签/演示文本/元数据/安装流转/md 渲染/txt 防抖保存）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
