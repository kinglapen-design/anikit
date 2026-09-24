# CDP 实测：预览会话绑定发起面板（ownerPanelId，2026-09-25 用户实机报三条）。
# 根因：预览全局单例（preview.assetKey），ContentArea previewActive = assetKey && 本 panel 激活——
# 预览跟着激活面板飘：① 预览中「+」新标签 → 预览飘进新标签（应留原标签、新标签为列表）；
# ② 预览中分屏 → 原标签预览丢失；③ 两面板各双击 → 预览左右跳（本版 = 可预测：谁发起归谁；
# 真双预览对比 = per-panel 预览实例，独立立项）。
# 场景：S1 新标签不抢预览；S2 分屏后原标签预览保留；S3 B 面板双击 = 预览明确跳 B；
# S4 属主标签关闭 → 预览退出（syncMirror 清理）。
import asyncio
import base64
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9376
URL = "http://localhost:5373"

STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 2, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "P1", folderId: "root", name: "图一.png", w: 100, h: 80, ext: "png",
        thumbPath: "E:/T/t1.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "P2", folderId: "root", name: "图二.png", w: 120, h: 90, ext: "png",
        thumbPath: "E:/T/t2.webp", thumbKind: "asset", origin: true,
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
    profile = tempfile.mkdtemp(prefix="cdp-owner-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1600,900", "about:blank"],
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
                if "exceptionDetails" in r.get("result", {}):
                    print("[EV-EXC]", str(r["result"]["exceptionDetails"].get("text"))[:300])
                return r.get("result", {}).get("result", {}).get("value")

            for _ in range(40):
                await asyncio.sleep(0.5)
                if await ev("document.querySelectorAll('.asset-card').length") or 0 > 0:
                    break

            fails = []

            # pinia state 快照（诊断：assetKey / panelId / activeTabId / tabs）
            PSTATE = """(() => {
              const s = document.querySelector('#app')?.__vue_app__?.config.globalProperties.$pinia?.state.value;
              return s ? { key: s.preview.assetKey, pid: s.preview.panelId, active: s.tabs.activeTabId, tabs: s.tabs.tabs.map((t) => t.id) } : null;
            })()"""

            # dockview 只渲染激活 panel 内容（非激活卸载）；tab 激活走 pointerdown（合成 click 无效实证）
            async def tab_preview(i):
                await ev(f"""(() => {{
                  const t = document.querySelectorAll('.dv-tab')[{i}];
                  t.dispatchEvent(new PointerEvent('pointerdown', {{ bubbles: true }}));
                  t.dispatchEvent(new PointerEvent('pointerup', {{ bubbles: true }}));
                }})()""")
                await asyncio.sleep(0.7)
                return await ev("!!document.querySelector('.content-area .preview-root')")

            async def cur_preview():
                return await ev("!!document.querySelector('.content-area .preview-root')")

            async def tab_count():
                return await ev("document.querySelectorAll('.dv-tab').length")

            async def menu_click(text):
                ok = await ev(f"""(() => {{
                  const els = [...document.querySelectorAll('.app-menu *')];
                  const it = els.find((e) => e.textContent.trim() === '{text}' && e.children.length === 0);
                  if (!it) return false;
                  it.click();
                  return true;
                }})()""")
                await asyncio.sleep(0.6)
                return ok

            # —— 前置：双击 P1 开预览（属主 = 当前唯一标签） ——
            await ev("document.querySelectorAll('.asset-card')[0].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(1.5)
            s0 = await cur_preview()
            print("[前置 预览开]", s0)
            if s0 is not True:
                fails.append(f"[前置] 预览未打开（{s0}）")

            # —— S1：预览中点「+」新标签 → 新标签（即激活）应为列表；切回原标签预览应在 ——
            await ev("[...document.querySelectorAll('.dv-tabs-and-actions-container .ic-btn')].pop()?.click()")
            await asyncio.sleep(1.0)
            s1_new = await cur_preview()  # 新标签即激活：应列表
            s1_old = await tab_preview(0)  # 切回原标签：预览应在
            print("[S1 新标签]", json.dumps({"新标签": s1_new, "原标签": s1_old}))
            print("[S1 state]", json.dumps(await ev(PSTATE), ensure_ascii=False))
            if s1_new is not False:
                fails.append(f"[S1] 新标签抢走了预览（{s1_new}）——预览应留原标签")
            if s1_old is not True:
                fails.append(f"[S1] 原标签预览丢失（{s1_old}）")

            # —— S2：右键原 tab「左右分屏」→ 新分屏 panel（即激活）列表；切回属主预览应在 ——
            await ev("document.querySelectorAll('.dock-tab')[0].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }))")
            await asyncio.sleep(0.6)
            if not await menu_click("左右分屏"):
                fails.append("[S2] 右键菜单「左右分屏」未找到")
            s2 = await ev("[...document.querySelectorAll('.content-area')].map((a) => !!a.querySelector('.preview-root'))")
            print("[S2 分屏]", json.dumps(s2))
            print("[S2 state]", json.dumps(await ev(PSTATE), ensure_ascii=False))
            # 多组时各组渲染自己激活 tab：组0 = 属主（预览应在）、组1 = 新分屏（应列表）
            if s2 != [True, False]:
                fails.append(f"[S2] 分屏后预览归属不符（{s2}，期望 [属主 True, 新分屏 False]）")

            # —— S3：右栏（tab[1]）双击 P2 → 预览明确跳右（谁发起归谁）；左栏回列表 ——
            await ev("""(() => { const t = document.querySelectorAll('.dv-tab')[1]; t.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); t.dispatchEvent(new PointerEvent('pointerup', { bubbles: true })); })()""")
            await asyncio.sleep(0.7)
            await ev("document.querySelector('.content-area').querySelectorAll('.asset-card')[1].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(1.2)
            s3_right = await cur_preview()  # 右栏：预览应归此
            s3_left = await tab_preview(0)  # 左栏：应回列表
            print("[S3 右栏双击]", json.dumps({"右栏": s3_right, "左栏": s3_left}))
            if s3_right is not True or s3_left is not False:
                fails.append(f"[S3] B 面板双击后预览未明确归 B（右 {s3_right} / 左 {s3_left}）")

            # —— S4：关闭属主（右栏 tab）→ 预览退出（任何面板都无预览） ——
            await ev("""(() => { const t = document.querySelectorAll('.dv-tab')[1]; t.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); t.dispatchEvent(new PointerEvent('pointerup', { bubbles: true })); })()""")
            await asyncio.sleep(0.5)
            await ev("document.querySelectorAll('.dock-tab')[1].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }))")
            await asyncio.sleep(0.6)
            if not await menu_click("关闭标签页"):
                fails.append("[S4] 右键菜单「关闭标签页」未找到")
            s4 = await tab_preview(0)
            s4_tabs = await tab_count()
            print("[S4 关属主]", json.dumps({"余面板预览": s4, "tab数": s4_tabs}))
            if s4 is not False:
                fails.append(f"[S4] 属主标签关闭后预览未退出（{s4}）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 预览绑定发起面板成立（新标签不抢 / 分屏保留 / B 双击归 B / 属主关即退）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
