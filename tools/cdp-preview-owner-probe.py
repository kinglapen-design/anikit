# CDP 实测：预览 per-panel 会话化（2026-09-25 双预览对比立项落地）。
# 前史：预览曾是全局单例 + ownerPanelId 绑定（踩坑 089 修「跟激活飘」），单例本质不变——
# 分屏后 B 面板双击预览左右跳。本轮 = 每 panel 独立预览会话（preview#{panelId} 注册表）。
# 场景：S1 新标签不抢预览；S2 分屏后原标签预览保留；S3 右栏双击 = 左右各自预览并存（双预览）；
# S4 键盘路由（←/→ 只作用激活 panel 的会话）；S5 关右栏只收右栏会话（左栏预览保留），
# 关左栏会话回收干净。
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

            # pinia state 快照（诊断：per-panel 会话 = preview#{panelId} 键族 / activeTabId / tabs）
            PSTATE = """(() => {
              const s = document.querySelector('#app')?.__vue_app__?.config.globalProperties.$pinia?.state.value;
              if (!s) return null;
              const sessions = {};
              for (const k of Object.keys(s)) if (k.startsWith('preview#')) sessions[k] = s[k].assetKey;
              return { sessions, active: s.tabs.activeTabId, tabs: s.tabs.tabs.map((t) => t.id) };
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

            # —— S3：右栏（组1）双击 P2 → 左右各自预览并存（左 P1 / 右 P2，per-panel 会话） ——
            # 注意：按组序取 .content-area（querySelector 拿首元素必误读，踩坑 089）
            await ev("document.querySelectorAll('.content-area')[1].querySelectorAll('.asset-card')[1].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(1.2)
            s3 = await ev("[...document.querySelectorAll('.content-area')].map((a) => !!a.querySelector('.preview-root'))")
            st3 = await ev(PSTATE)
            print("[S3 右栏双击]", json.dumps({"面板预览": s3}))
            print("[S3 state]", json.dumps(st3, ensure_ascii=False))
            if s3 != [True, True]:
                fails.append(f"[S3] 双预览未并存（{s3}，期望 [左 True, 右 True]）")
            keys3 = sorted(v for v in (st3 or {}).get("sessions", {}).values() if v)
            if keys3 != ["P1|root", "P2|root"]:
                fails.append(f"[S3] 会话资产不符（{keys3}，期望左 P1|root / 右 P2|root）")

            # —— S4：键盘路由——←/→ 只作用激活 panel 的会话 ——
            # tab 索引注意（踩坑 089）：S1 的「+」tab 与属主同组——.dv-tab = [左组:属主,+tab, 右组:分屏]，
            # 右栏 = dv-tab[2]（不是 [1]）。激活左栏按 →：左 P1→P2，右栏不动；激活右栏按 ←：右 P2→P1，左栏不动
            async def press(key):
                await ev(f"window.dispatchEvent(new KeyboardEvent('keydown', {{ key: '{key}' }}))")
                await asyncio.sleep(0.9)

            await ev("""(() => { const t = document.querySelectorAll('.dv-tab')[0]; t.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); t.dispatchEvent(new PointerEvent('pointerup', { bubbles: true })); })()""")
            await asyncio.sleep(0.7)
            await press("ArrowRight")
            st4a = await ev(PSTATE)
            print("[S4 左栏→]", json.dumps(st4a, ensure_ascii=False))
            vals = sorted(v for v in (st4a or {}).get("sessions", {}).values() if v)
            if vals != ["P2|root", "P2|root"]:
                fails.append(f"[S4] 左栏步进未生效或误伤右栏（{vals}，期望双 P2|root）")
            await ev("""(() => { const t = document.querySelectorAll('.dv-tab')[2]; t.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true })); t.dispatchEvent(new PointerEvent('pointerup', { bubbles: true })); })()""")
            await asyncio.sleep(0.7)
            await press("ArrowLeft")
            st4b = await ev(PSTATE)
            print("[S4 右栏←]", json.dumps(st4b, ensure_ascii=False))
            vals = sorted(v for v in (st4b or {}).get("sessions", {}).values() if v)
            if vals != ["P1|root", "P2|root"]:
                fails.append(f"[S4] 右栏步退未生效或误伤左栏（{vals}，期望 P1|root + P2|root）")
            # 左右会话键（供 S5 断言死 panel 的 state 键被删）
            sessions4 = (st4b or {}).get("sessions", {})
            right_key = next((k for k, v in sessions4.items() if v == "P1|root"), None)
            left_key = next((k for k, v in sessions4.items() if v == "P2|root"), None)

            # —— S5：关右栏 tab（dock-tab[2]）→ 只收右栏会话（左栏预览保留、state 键删除）；
            # 再关左栏（dock-tab[0]）→ 左栏会话同收（余「+」tab 干净列表）——
            await ev("document.querySelectorAll('.dock-tab')[2].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }))")
            await asyncio.sleep(0.6)
            if not await menu_click("关闭标签页"):
                fails.append("[S5] 右键菜单「关闭标签页」未找到（右栏）")
            await asyncio.sleep(0.5)
            s5_left = await cur_preview()  # 仅剩左组（激活回左栏）：预览应在
            st5 = await ev(PSTATE)
            print("[S5 关右栏]", json.dumps({"左栏预览": s5_left}))
            print("[S5 state]", json.dumps(st5, ensure_ascii=False))
            if s5_left is not True:
                fails.append(f"[S5] 关右栏误伤左栏预览（{s5_left}）")
            sessions5 = (st5 or {}).get("sessions", {})
            if right_key and right_key in sessions5:
                fails.append(f"[S5] 右栏会话 state 键未删（{right_key} 仍在）")
            vals5 = [v for v in sessions5.values() if v]
            if vals5 != ["P2|root"]:
                fails.append(f"[S5] 右栏会话未精准回收（{vals5}，期望仅剩左栏 P2|root）")
            await ev("document.querySelectorAll('.dock-tab')[0].dispatchEvent(new MouseEvent('contextmenu', { bubbles: true }))")
            await asyncio.sleep(0.6)
            if not await menu_click("关闭标签页"):
                fails.append("[S5] 右键菜单「关闭标签页」未找到（左栏）")
            await asyncio.sleep(0.5)
            s5_none = await cur_preview()  # 余 S1 的「+」tab：干净列表
            st5b = await ev(PSTATE)
            print("[S5 关左栏]", json.dumps({"余tab预览": s5_none}))
            if s5_none is not False:
                fails.append(f"[S5] 属主关闭后余 tab 出现预览（{s5_none}）")
            sessions5b = (st5b or {}).get("sessions", {})
            if left_key and left_key in sessions5b:
                fails.append(f"[S5] 左栏会话 state 键未删（{left_key} 仍在）")
            if [v for v in sessions5b.values() if v]:
                fails.append(f"[S5] 会话回收不干净（{sessions5b}）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 预览 per-panel 会话化成立（新标签不抢 / 分屏保留 / 双预览并存 / 键盘按激活路由 / 关 panel 精准回收）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
