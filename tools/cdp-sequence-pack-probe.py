# CDP 实测：序列帧包内视图 + 菜单点亮（2026-08-21，5.2）
# 口径（PRD 04-序列帧 包内视图定案 / 05-序列帧打包 解散条）：
# ① 素材卡片右键：打开序列帧包 / 解散序列帧包导入 两项点亮（不再 disabled）；
# ② 打开序列帧包 → 视口切 seq:{assetId}，帧卡片列表渲染（3 帧 + 封面标记），标签页标题 = 包名；
# ③ 帧右键菜单 = 打开预览 / 重排序号 / 删除此张序列图；多选 → 删除 N 张序列图；
# ④ 删除调 delete_sequence_frames（多选集入参）；空白右键重排序号调 renumber_sequence_frames；
# ⑤ 解散 → alert warning 弹窗（文案含帧数 / 笔记数 / 不可撤销）→ 确认调 dissolve_sequence。
# 前端链路全真（menuSpecs / SequencePackView / placementOps / AppConfirm），invoke 桩计数。
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets
import asyncio

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9372
URL = "http://localhost:5373"

STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 2, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "S1", folderId: "root", name: "走路.png", w: 640, h: 480, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true, frameCount: 3, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: 2, thumbFrame: "走路-001.png", fps: 24,
        size: 3000, createdAt: null, modifiedAt: "2026-08-21T10:00:00+08:00", importedAt: "2026-08-21T10:00:00+08:00" },
      { assetId: "P1", folderId: "root", name: "参考.png", w: 640, h: 480, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true, frameCount: null, animated: false,
        duration: null, bpm: null, importedFrom: null, noteCount: null, thumbFrame: null, fps: null,
        size: 1000, createdAt: null, modifiedAt: "2026-08-21T10:00:00+08:00", importedAt: "2026-08-21T10:00:00+08:00" },
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.png", pending: false, animPreview: null },
      get_notes: [],
      list_sequence_frames: {
        fps: 24, thumbFrame: "走路-001.png",
        frames: [1, 2, 3].map((n) => ({
          name: `走路-00${n}.png`,
          fullPath: `E:/T/frames/走路-00${n}.png`,
          stripPath: `E:/T/cache/走路-00${n}.png.webp`,
        })),
      },
    };
    if (["dissolve_sequence", "delete_sequence_frames", "renumber_sequence_frames"].includes(cmd)) {
      window.__calls.push(JSON.parse(JSON.stringify({ cmd, args })));
      if (cmd === "dissolve_sequence") return Promise.resolve(3);
      if (cmd === "delete_sequence_frames") return Promise.resolve((args.frameNames ?? []).length);
      return Promise.resolve(2);
    }
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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = [str(a.get("value", a.get("description", "")))[:300] for a in raw["params"]["args"]]
            print("[CONSOLE]", raw["params"]["type"], " ".join(args)[:500])


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-seq-pack-")
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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def menu_items():
                return await ev(
                    "[...document.querySelectorAll('.app-menu .menu-item')]"
                    ".map((m) => ({ label: m.textContent.trim(), disabled: m.disabled }))")

            async def click_menu(text):
                return await ev(
                    "(() => { const m = [...document.querySelectorAll('.app-menu .menu-item')]"
                    f".find((x) => x.textContent.includes({json.dumps(text)}));"
                    " if (!m) return false; m.click(); return true; })()")

            async def close_menu():
                await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
                await asyncio.sleep(0.3)

            fails = []

            async def card_ctx(selector):
                await ev(f"""(() => {{
                  const c = document.querySelector({json.dumps(selector)});
                  const r = c.getBoundingClientRect();
                  c.dispatchEvent(new MouseEvent('contextmenu', {{ bubbles: true, clientX: r.left + 20, clientY: r.top + 20 }}));
                }})()""")
                await asyncio.sleep(0.4)

            # ① 序列帧卡片右键：两项点亮（不再 disabled）
            await card_ctx(".asset-card")
            items = await menu_items()
            print("素材卡菜单:", json.dumps([i for i in (items or []) if "序列帧包" in i["label"]], ensure_ascii=False))
            open_item = next((i for i in (items or []) if "打开序列帧包" in i["label"]), None)
            dissolve_item = next((i for i in (items or []) if "解散序列帧包导入" in i["label"]), None)
            if not open_item or open_item["disabled"]:
                fails.append(f"「打开序列帧包」缺失或仍禁用（{open_item}）")
            if not dissolve_item or dissolve_item["disabled"]:
                fails.append(f"「解散序列帧包导入」缺失或仍禁用（{dissolve_item}）")
            # 普通图片卡不出序列帧项（上下文规则不回归）
            await close_menu()
            await ev("[...document.querySelectorAll('.asset-card')][1].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: 300, clientY: 300 }))")
            await asyncio.sleep(0.4)
            plain_items = await menu_items()
            if any("序列帧包" in i["label"] for i in (plain_items or [])):
                fails.append("普通图片卡不应出序列帧菜单项")
            await close_menu()

            # ⑤（前置做，解散后视口仍在包外）解散 → alert warning 弹窗 → 确认调 dissolve_sequence
            await card_ctx(".asset-card")
            if not await click_menu("解散序列帧包导入"):
                fails.append("解散菜单项点击失败")
            await asyncio.sleep(0.5)
            confirm = await ev("""({
              box: !!document.querySelector('.app-confirm'),
              text: document.querySelector('.app-confirm .text')?.textContent?.trim() ?? null,
              btns: [...document.querySelectorAll('.app-confirm .app-btn')].map((b) => b.textContent.trim()),
            })""")
            print("解散确认窗:", json.dumps(confirm, ensure_ascii=False))
            if not confirm or not confirm["box"]:
                fails.append("解散未弹确认窗")
            else:
                t = confirm["text"] or ""
                for needle in ["3 张序列帧", "2 条逐帧笔记", "不可撤销"]:
                    if needle not in t:
                        fails.append(f"解散弹窗文案缺「{needle}」（{t[:80]}）")
                if not confirm["btns"] or confirm["btns"][0] != "解散导入":
                    fails.append(f"解散弹窗主钮不是「解散导入」（{confirm['btns']}）——主左次右口径")
            await ev("[...document.querySelectorAll('.app-confirm .app-btn')][0]?.click()")
            await asyncio.sleep(0.6)
            calls = await ev("window.__calls")
            if not any(c["cmd"] == "dissolve_sequence" and c["args"].get("assetId") == "S1" for c in (calls or [])):
                fails.append(f"dissolve_sequence 未调用或入参错（{calls}）")

            # ② 打开序列帧包 → 包内卡片列表
            await card_ctx(".asset-card")
            await click_menu("打开序列帧包")
            await asyncio.sleep(1.0)
            pack = await ev("""({
              cards: [...document.querySelectorAll('.frame-card .name')].map((n) => n.textContent.trim()),
              covers: document.querySelectorAll('.frame-card .cover-tag').length,
              title: document.querySelector('.tab-label.active .tab-title')?.textContent?.trim()
                     ?? document.querySelector('.tab-title')?.textContent?.trim() ?? null,
            })""")
            print("包内视图:", json.dumps(pack, ensure_ascii=False))
            if pack is None or pack["cards"] != ["走路-001.png", "走路-002.png", "走路-003.png"]:
                fails.append(f"包内帧卡片渲染不符（{pack and pack['cards']}）")
            if pack and pack["covers"] != 1:
                fails.append(f"封面标记数 {pack['covers']}（期望 1）")
            if pack and pack["title"] != "走路":
                fails.append(f"标签页标题不是包名（{pack['title']}）")

            # ③ 帧右键菜单三项
            await card_ctx(".frame-card")
            fitems = await menu_items()
            print("帧菜单:", json.dumps(fitems, ensure_ascii=False))
            labels = [i["label"] for i in (fitems or [])]
            for needle in ["打开预览", "重排序号", "删除此张序列图"]:
                hit = next((i for i in (fitems or []) if needle in i["label"]), None)
                if not hit or hit["disabled"]:
                    fails.append(f"帧菜单「{needle}」缺失或禁用（{labels}）")
            await close_menu()

            # ④ ctrl 多选第二帧 → 右键 → 删除 2 张 → delete_sequence_frames 入参
            await ev("""(() => {
              const cards = document.querySelectorAll('.frame-card');
              cards[1].dispatchEvent(new MouseEvent('click', { bubbles: true, ctrlKey: true }));
            })()""")
            await asyncio.sleep(0.3)
            await card_ctx(".frame-card:nth-of-type(2)")
            # 注：.frame-card 在 grid 内无 wrapper，nth-of-type 对各 card 生效
            multi = await menu_items()
            del_item = next((i for i in (multi or []) if "删除" in i["label"]), None)
            print("多选删除项:", json.dumps(del_item, ensure_ascii=False))
            if not del_item or "删除 2 张序列图" not in del_item["label"]:
                fails.append(f"多选删除项文案不符（{del_item}）")
            else:
                await click_menu("删除 2 张序列图")
                await asyncio.sleep(0.5)
                calls = await ev("window.__calls")
                hit = [c for c in (calls or []) if c["cmd"] == "delete_sequence_frames"]
                names = hit and hit[-1]["args"].get("frameNames")
                if not names or set(names) != {"走路-001.png", "走路-002.png"}:
                    fails.append(f"delete_sequence_frames 入参不符（{names}）")
            await close_menu()

            # 空白右键 = 重排序号 → renumber_sequence_frames
            await ev("""(() => {
              const s = document.querySelector('.pack-scroll');
              const r = s.getBoundingClientRect();
              s.dispatchEvent(new MouseEvent('contextmenu', { bubbles: true, clientX: r.right - 30, clientY: r.bottom - 8 }));
            })()""")
            await asyncio.sleep(0.4)
            bitems = await menu_items()
            print("空白菜单:", json.dumps(bitems, ensure_ascii=False))
            if [i["label"] for i in (bitems or [])] != ["重排序号"]:
                fails.append(f"空白右键菜单不符（{[i['label'] for i in (bitems or [])]}）")
            if await click_menu("重排序号"):
                await asyncio.sleep(0.5)
                calls = await ev("window.__calls")
                if not any(c["cmd"] == "renumber_sequence_frames" and c["args"].get("assetId") == "S1" for c in (calls or [])):
                    fails.append("renumber_sequence_frames 未调用")
            else:
                fails.append("空白菜单「重排序号」点击失败")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 序列帧包内视图成立（菜单点亮 / 包内卡片列表 / 帧菜单三项 / 多选删除入参 / 空白重排 / 解散警告弹窗 + 调用）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


sys.exit(asyncio.run(main()))
