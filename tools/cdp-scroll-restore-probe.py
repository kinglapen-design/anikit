# CDP 探针：视口滚动持久化 + 链接跳转定位 最小复现（2026-09-23 用户实机两报）
#   S1  fa 视口滚轮后 pos > 1000（滚动本身工作）
#   S2  切到 fb 后 pinia scrollPositions["folder:fa"] ≈ pos（保存链路实锤）
#   S3  切回 fa 后 transform ≈ 原 pos（恢复链路实锤）
#   S4  注入 pendingReveal（folder:fa 的末排卡）→ transform 滚到该卡附近（reveal 消费实锤）
#   S5  同视口注入 pendingReveal（viewKey 不变路径）→ 同样定位
# 桩：fa 40 项 png（列表可滚）/ fb 3 项；get_ui_settings {}（会话内测，不依赖落盘）。
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-scroll-restore-probe.py
import asyncio
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19890 + os.getpid() % 100
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-scroll-{os.getpid()}")

STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd) => {
    const P = (o) => Object.assign({
      w: 1000, h: 800, thumbW: null, thumbH: null, thumbPath: null, thumbKind: null,
      origin: true, frameCount: null, animated: false, duration: null, bpm: null,
      importedFrom: null, createdAt: null, modifiedAt: "2026-09-23T10:00:00+08:00",
      importedAt: "2026-09-23T10:00:00+08:00", noteCount: null, thumbFrame: null, fps: null,
    }, o);
    const fa = [];
    for (let i = 1; i <= 40; i++) fa.push(P({ assetId: "a" + i, folderId: "fa", name: "甲图" + String(i).padStart(2, "0") + ".png", ext: "png", size: 102400 }));
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 43, folderCount: 2 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "fa", name: "甲夹", count: 40, totalSize: 4096000, children: [] },
          { id: "fb", name: "乙夹", count: 3, totalSize: 307200, children: [] },
        ] },
      list_placements: fa.concat([
        P({ assetId: "b1", folderId: "fb", name: "乙图01.png", ext: "png", size: 102400 }),
        P({ assetId: "b2", folderId: "fb", name: "乙图02.png", ext: "png", size: 102400 }),
        P({ assetId: "b3", folderId: "fb", name: "乙图03.png", ext: "png", size: 102400 }),
      ]),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_relations: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(v === null ? null : structuredClone(v));
  },
};
window.__cbId = 0; window.__cbs = {};
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


def kill_tree():
    subprocess.run(["taskkill", "/F", "/IM", "msedge.exe"], capture_output=True)


async def main():
    fails = []

    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True,
                                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "--disk-cache-size=1",
        f"--user-data-dir={PROFILE}", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    code = 0
    try:
        version = None
        for _ in range(30):
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(name)

            async def wait_for(expr, timeout=10.0):
                deadline = time.time() + timeout
                while time.time() < deadline:
                    if await ev(expr) is True:
                        return True
                    await asyncio.sleep(0.25)
                return False

            POS = "(() => { const m = /translateY\\((-?[\\d.]+)px\\)/.exec(document.querySelector('.grid-viewport .grid-canvas').style.transform || ''); return m ? -parseFloat(m[1]) : null; })()"
            # 组件实例级诊断：AssetGrid setupState.pos.value（script setup 顶层绑定 dev 可读）+
            # useVirtualScroll 的 pos ref 同一性（_uid 辅助）
            GRIDPOS = "(() => { const app = document.querySelector('#app').__vue_app__; let g = null;"
            " const walk = (i) => { if (!i || g) return; if (i.setupState && i.setupState.pos && 'gridTotal' in i.setupState) { g = i; return; }"
            "   const st = i.subTree; if (st) { if (st.component) walk(st.component); const ch = st.children; if (Array.isArray(ch)) for (const c of ch) walk(c && c.component); } };"
            " walk(app._instance);"
            " if (!g) return 'NO-GRID';"
            " const ss = g.setupState;"
            " return { uid: g.uid, pos: ss.pos.value, total: ss.gridTotal, viewKey: g.props.viewKey }; })()"
            # 注意：pinia state 是 Vue reactive Proxy，CDP returnByValue 直接序列化会得 {} 空壳——
            # 一律页面内 JSON.stringify 成字符串返回再解析
            SP = "JSON.stringify((() => { const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia; return p.state.value.ui.scrollPositions; })())"
            PEND = "JSON.stringify((() => { const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia; return p.state.value.content.pendingReveal; })())"
            SET_PEND = "(k) => { const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia; p.state.value.content.pendingReveal = { viewKey: 'folder:fa', key: k }; return true; }"
            CLICK_TREE = "(name) => { const it = [...document.querySelectorAll('.tree-item')].find((x) => x.textContent.includes(name)); if (it) { it.click(); return true; } return false; }"

            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await wait_for("document.querySelectorAll('.content-area .asset-card').length >= 40", 15)
            await asyncio.sleep(1.0)

            # 间谍：记录 setScrollPos 全部调用（key, value）+ scrollPosFor 读取结果
            spy = await ev(
                "(() => { const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia;"
                " const ui = p._s.get('ui'); window.__spCalls = [];"
                " const orig = ui.setScrollPos;"
                " ui.setScrollPos = function(k, v) { window.__spCalls.push([k, typeof v, v]); return orig.call(this, k, v); };"
                " return 'armed'; })()")
            print("  间谍武装:", spy)

            # 进 fa 视口
            await ev(f"({CLICK_TREE})('甲夹')")
            await wait_for("document.querySelectorAll('.content-area .asset-card').length >= 40 && !!document.querySelector('.grid-viewport .grid-canvas')", 10)
            await asyncio.sleep(0.8)

            # S1 滚轮滚动 fa（前后诊断 viewport 数量与各 transform）
            diag0 = await ev("(() => { const vps = [...document.querySelectorAll('.grid-viewport')]; return { n: vps.length, tf: vps.map((v) => (v.querySelector('.grid-canvas')||{}).style?.transform ?? 'NONE'), vis: vps.map((v) => v.offsetHeight) }; })()")
            for _ in range(6):
                await ev("document.querySelector('.grid-viewport').dispatchEvent(new WheelEvent('wheel', { deltaY: 600, bubbles: true, cancelable: true }))")
                await asyncio.sleep(0.1)
            await asyncio.sleep(0.4)
            pos1 = await ev(POS)
            diag1 = await ev("(() => { const vps = [...document.querySelectorAll('.grid-viewport')]; return { n: vps.length, tf: vps.map((v) => (v.querySelector('.grid-canvas')||{}).style?.transform ?? 'NONE'), vis: vps.map((v) => v.offsetHeight) }; })()")
            check("S1 fa 滚轮后 pos > 1000", isinstance(pos1, (int, float)) and pos1 > 1000, f"pos1={pos1}")
            print("  诊断 滚动前:", json.dumps(diag0, ensure_ascii=False))
            print("  诊断 滚动后:", json.dumps(diag1, ensure_ascii=False))

            # S2 切 fb：读 scrollPositions（页面内序列化，保存实锤）+ 间谍记录
            await ev(f"({CLICK_TREE})('乙夹')")
            await asyncio.sleep(0.8)
            sp_raw = await ev(SP)
            sp = json.loads(sp_raw) if isinstance(sp_raw, str) else {}
            saved = sp.get("folder:fa")
            calls = await ev("JSON.stringify(window.__spCalls)")
            check("S2 切走后 scrollPositions['folder:fa'] ≈ pos1", isinstance(saved, (int, float)) and pos1 and abs(saved - pos1) < 5, f"saved={saved} pos1={pos1} 全量={sp_raw}")
            print("  间谍 setScrollPos 调用:", calls)
            pos_fb = await ev(POS)

            # S3 切回 fa：读 transform（恢复实锤）
            await ev(f"({CLICK_TREE})('甲夹')")
            await asyncio.sleep(1.0)
            pos2 = await ev(POS)
            check("S3 切回 fa 后 pos ≈ pos1", isinstance(pos2, (int, float)) and pos1 and abs(pos2 - pos1) < 5, f"pos2={pos2} pos1={pos1}（切到 fb 时 pos={pos_fb}）")

            # S4 先滚回顶（连续向上 wheel），再注入视野外 reveal（a35）→ 应滚到该卡附近
            for _ in range(10):
                await ev("document.querySelector('.grid-viewport').dispatchEvent(new WheelEvent('wheel', { deltaY: -600, bubbles: true, cancelable: true }))")
                await asyncio.sleep(0.05)
            await asyncio.sleep(0.4)
            pos_top = await ev(POS)
            await ev(f"({SET_PEND})('a35|fa')")
            await asyncio.sleep(1.0)
            pos3 = await ev(POS)
            check("S4 视野外 reveal 注入后 pos > 300（定位滚动发生）", isinstance(pos3, (int, float)) and pos3 > 300, f"pos3={pos3}（注入前 pos_top={pos_top}）")
            pend = await ev(PEND)
            check("S4b pendingReveal 已消费（清空）", pend in (None, "null"), f"pendingReveal={pend}")

            # S5 跨视口 reveal（relationOps 真实路径形态）：在 fb 注入 reveal(fa, a20) → 切 fa →
            # relayout 应先恢复 fa 的保存位（≈1189），reveal 覆盖滚到 a20 附近（第 4 行，居中位 ~500）
            await ev(f"({CLICK_TREE})('乙夹')")
            await asyncio.sleep(0.8)
            await ev("(k => { const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia; p.state.value.content.pendingReveal = { viewKey: 'folder:fa', key: k }; return true; })('a20|fa')")
            await ev(f"({CLICK_TREE})('甲夹')")
            await asyncio.sleep(1.0)
            pos4 = await ev(POS)
            pend2 = await ev(PEND)
            sp2 = await ev(SP)
            check("S5 跨视口 reveal：pos 滚到 a20 附近（200~900，非恢复位 ≈1189）", isinstance(pos4, (int, float)) and 200 < pos4 < 900, f"pos4={pos4}（fa 恢复位应 ≈1189）")
            check("S5b 跨视口 reveal 已消费", pend2 in (None, "null"), f"pendingReveal={pend2}")
            sp2d = json.loads(sp2) if isinstance(sp2, str) else {}
            check("S5c 切回 fa 时 fb 位置已保存（fb=0）", sp2d.get("folder:fb") is not None, f"sp={sp2}")

            print("诊断汇总:", json.dumps({"pos1": pos1, "saved": saved, "pos_fb": pos_fb, "pos2": pos2, "pos_top": pos_top, "pos3": pos3, "pos4": pos4, "sp": sp}))
    finally:
        edge.terminate()
        try:
            edge.wait(timeout=5)
        except Exception:
            edge.kill()
        if spawned:
            spawned.terminate()
    print("FAIL:", fails if fails else "无")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
