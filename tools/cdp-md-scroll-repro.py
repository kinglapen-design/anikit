# CDP 探针：md 预览「重新聚焦回第一行」最小复现（2026-10-02 用户实机三报之①）
#   事实采集：重新聚焦类操作后 md-body.scrollTop 是否归零 + read_text_asset 调用计数
#   （计数增 = load() 重跑（watch 触发）；计数不变 = DOM 层面（重挂 / display / 滚动钳制））
#   T1 点击预览区文字（未激活 pane 重聚焦路径 onRootPointerdown）
#   T2 splitTab 分屏 → 点右侧 pane → 点回左侧预览
#   T3 悬停视频卡（用户视频里回顶均伴随 hover 播放，排除项）
# 桩：fa 文件夹 = md1（长文 240 行）+ v1（mp4）+ 38 png；read_text_asset 计数。
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-md-scroll-repro.py
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
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-mdscroll-{os.getpid()}")

MD_LINES = "\n".join(f"## 镜头 {i}\n\n- 第 {i} 段正文内容，用于撑高文档制造滚动空间。\n" for i in range(1, 241))

STUB = r"""
window.__readCount = 0;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const P = (o) => Object.assign({
      w: 1000, h: 800, thumbW: null, thumbH: null, thumbPath: null, thumbKind: null,
      origin: true, frameCount: null, animated: false, duration: null, bpm: null,
      importedFrom: null, createdAt: null, modifiedAt: "2026-09-23T10:00:00+08:00",
      importedAt: "2026-09-23T10:00:00+08:00", noteCount: null, thumbFrame: null, fps: null,
      playAssetId: null, hidden: false, wsPath: null,
    }, o);
    const fa = [P({ assetId: "md1", folderId: "fa", name: "测试文档.md", ext: "md", size: 20480 })];
    fa.push(P({ assetId: "v1", folderId: "fa", name: "测试视频.mp4", ext: "mp4", size: 1024000, duration: 12, fps: 30, frameCount: 360 }));
    for (let i = 1; i <= 38; i++) fa.push(P({ assetId: "a" + i, folderId: "fa", name: "甲图" + String(i).padStart(2, "0") + ".png", ext: "png", size: 102400 }));
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 40, folderCount: 1 },
      get_ui_settings: {},
      get_window_fx_mode: "solid",
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [ { id: "fa", name: "甲夹", count: 40, totalSize: 4096000, children: [] } ] },
      list_placements: fa,
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_relations: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
    };
    if (cmd === "read_text_asset") {
      window.__readCount++;
      return Promise.resolve({ content: "__MD__" });
    }
    if (cmd === "get_media_source") return Promise.resolve({ path: "E:/fake/v1.mp4", transcoded: false });
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(v === null ? null : structuredClone(v));
  },
};
window.__cbId = 0; window.__cbs = {};
""".replace("__MD__", MD_LINES.replace("\\", "\\\\").replace("`", "\\`").replace("\n", "\\n"))

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


PINIA = "document.querySelector('#app').__vue_app__.config.globalProperties.$pinia"


async def main():
    fails = []
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        print("pnpm dev（5373）未起")
        return 2

    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1600,1000", "--no-first-run", "--disk-cache-size=1",
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
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)

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
                    await asyncio.sleep(0.1)
                return False

            # S0 应用就绪 + 导航到 folder:fa
            ok = await wait_for(f"!!{PINIA}")
            check("S0 app 就绪", ok)
            await ev(f"{PINIA}._s.get('tabs').openView('folder:fa'); true")
            await asyncio.sleep(0.5)

            # S1 打开 md 预览（经激活 panel 的 preview session）
            opened = await ev(f"""(async () => {{
              const p = {PINIA};
              const id = p.state.value.tabs.activeTabId;
              const s = p._s.get('preview#' + id);
              if (!s) return 'no-session:' + id;
              await s.open('md1|fa');
              return id;
            }})()""")
            left_id = opened  # 左侧预览页 panel id（T2/T5 复用）
            print(f"  预览会话 panel = {opened}")
            ok = await wait_for("!!document.querySelector('.md-body') && !!document.querySelector('.md-body h1, .md-body h2')")
            check("S1 md 预览渲染", ok)

            # S2 滚到中部
            await ev("const b = document.querySelector('.md-body'); b.scrollTop = 1600; true")
            await asyncio.sleep(0.2)
            pos0 = await ev("document.querySelector('.md-body').scrollTop")
            check("S2 滚动就位 > 1000", isinstance(pos0, (int, float)) and pos0 > 1000, f"pos={pos0}")

            async def snap(tag):
                return await ev(f"""JSON.stringify({{
                  pos: Math.round(document.querySelector('.md-body')?.scrollTop ?? -1),
                  reads: window.__readCount,
                  assetKey: (() => {{ const p = {PINIA}; const id = p.state.value.tabs.activeTabId; return p._s.get('preview#' + id)?.assetKey ?? null; }})(),
                  panels: {PINIA}.state.value.tabs.tabs.length,
                }})""")

            print(f"  基线: {await snap('base')}")

            # 插桩：md-body 元素同一性 + 属性/子树变化 + scroll 轨迹 + display 轨迹
            await ev("""(() => {
              window.__mdEl = document.querySelector('.md-body');
              window.__log = [];
              const mo = new MutationObserver((recs) => {
                for (const r of recs) {
                  if (window.__log.length > 400) return;
                  if (r.type === 'attributes') {
                    const t = r.target;
                    const id = t.className && t.className.toString ? t.className.toString().slice(0, 30) : t.tagName;
                    window.__log.push('attr:' + id + '@' + r.attributeName);
                  } else {
                    const t = r.target;
                    const id = t.className && t.className.toString ? t.className.toString().slice(0, 30) : t.tagName;
                    window.__log.push('child:' + id + ' +' + r.addedNodes.length + '/-' + r.removedNodes.length);
                  }
                }
              });
              mo.observe(document.body, { subtree: true, attributes: true, childList: true, attributeFilter: ['style', 'class'] });
              window.__mdEl.addEventListener('scroll', () => { if (window.__log.length < 400) window.__log.push('scroll:' + Math.round(window.__mdEl.scrollTop)); }, { passive: true });
              return true;
            })()""")

            async def truth(tag):
                return await ev(f"""JSON.stringify({{
                  sameEl: document.querySelector('.md-body') === window.__mdEl,
                  display: getComputedStyle(window.__mdEl).display,
                  overflowY: getComputedStyle(window.__mdEl).overflowY,
                  pos: Math.round(window.__mdEl.scrollTop),
                  log: window.__log.slice(-30),
                }})""")

            # T1 点击预览区文字（真实鼠标管道）
            rect = await ev("JSON.stringify(document.querySelector('.md-body').getBoundingClientRect())")
            r = json.loads(rect)
            cx, cy = r["left"] + r["width"] / 2, r["top"] + 200
            for t in ("mousePressed", "mouseReleased"):
                await send(ws, "Input.dispatchMouseEvent", {"type": t, "x": cx, "y": cy, "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.4)
            s1 = json.loads(await snap("T1"))
            print(f"  T1 点击预览后: {s1}")
            check("T1 点击预览区滚动不丢", s1["pos"] > 1000, f"pos={s1['pos']} reads={s1['reads']}")

            # T2 分屏 → 点右侧 pane → 点回左侧预览
            await ev(f"""(() => {{ const p = {PINIA}; const id = p.state.value.tabs.activeTabId; p._s.get('tabs').splitTab(id, 'right'); return true; }})()""")
            ok = await wait_for(f"{PINIA}.state.value.tabs.tabs.length === 2")
            check("T2 分屏成两 pane", ok)
            # Bug2 验证：源在预览 → 新 pane 同样进预览（双侧 md-body）
            ok = await wait_for("document.querySelectorAll('.md-body').length === 2", timeout=5.0)
            nb = await ev("document.querySelectorAll('.md-body').length")
            check("Bug2 分屏新 pane 复制预览态", ok, f"md-body 数={nb}")
            await asyncio.sleep(0.6)
            s2a = json.loads(await snap("T2a"))
            print(f"  T2 分屏后: {s2a}")
            # 点右侧 pane 空白（网格区）
            rects = await ev("""JSON.stringify([...document.querySelectorAll('.content-area')].map(e => { const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + 120, w: r.width }; }))""")
            rl = json.loads(rects)
            print(f"  panes = {rl}")
            if len(rl) == 2:
                # 右侧 pane（预览在左，新 pane 在右）
                rx = max(rl, key=lambda q: q["x"])
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "x": rx["x"], "y": rx["y"], "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.4)
                s2b = json.loads(await snap("T2b"))
                print(f"  T2 点右侧后: {s2b}")
                # 点回左侧预览区
                lx = min(rl, key=lambda q: q["x"])
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "x": lx["x"], "y": lx["y"] + 120, "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.4)
                s2c = json.loads(await snap("T2c"))
                print(f"  T2 点回左侧预览后: {s2c}")
                t2truth = json.loads(await truth("T2"))
                print(f"  T2 真相: sameEl={t2truth['sameEl']} display={t2truth['display']} overflowY={t2truth['overflowY']} pos={t2truth['pos']}")
                print(f"  T2 事件轨迹: {t2truth['log']}")
                check("T2 分屏来回聚焦滚动不丢", s2c["pos"] > 1000, f"pos={s2c['pos']} reads={s2c['reads']}")

            # T3 悬停视频卡（pointer 移入 mp4 卡）
            vrect = await ev("""(() => { const cs = [...document.querySelectorAll('.asset-card')]; const c = cs.find(e => e.textContent.includes('测试视频')); if (!c) return null; const r = c.getBoundingClientRect(); return JSON.stringify({ x: r.left + r.width / 2, y: r.top + r.height / 2 }); })()""")
            if vrect:
                v = json.loads(vrect)
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": v["x"], "y": v["y"]}, session=sess)
                await asyncio.sleep(1.2)  # hover 进入延迟 300ms + 播放
                s3 = json.loads(await snap("T3"))
                print(f"  T3 悬停视频卡后: {s3}")
                check("T3 悬停视频滚动不丢", s3["pos"] > 1000, f"pos={s3['pos']} reads={s3['reads']}")
            else:
                print("  T3 跳过（未找到视频卡）")

            # T5（Bug3）锁定预览页 → openView 新开标签 → 切回 → 预览必须在
            await ev(f"{PINIA}._s.get('tabs').toggleLock('{left_id}'); true")
            await asyncio.sleep(0.2)
            locked = await ev(f"!!{PINIA}.state.value.tabs.tabs.find(t => t.id === '{left_id}')?.locked")
            check("T5 锁定就位", locked is True, f"locked={locked}")
            await ev(f"{PINIA}._s.get('tabs').openView('quick:all'); true")  # 锁定 → 新开标签页
            ok = await wait_for(f"{PINIA}.state.value.tabs.tabs.length === 3")
            check("T5 锁定页导航新开标签", ok)
            await ev(f"{PINIA}._s.get('tabs').activateTab('{left_id}'); true")  # 切回原锁定页
            await asyncio.sleep(0.4)
            ak = await ev(f"{PINIA}._s.get('preview#' + '{left_id}')?.assetKey ?? null")
            md_alive = await ev("document.querySelector('.md-body') !== null")
            check("T5 切回锁定页预览仍在", ak == "md1|fa" and md_alive is True, f"assetKey={ak} md-body={md_alive}")
            # 切回后滚动位置也在（reattach 保活顺带验证）
            s5 = json.loads(await snap("T5"))
            print(f"  T5 切回后: {s5}")

            # T6 进出编辑态（v-show display:none 路径）：滚动应保持
            await ev("const b = document.querySelector('.md-body'); b.scrollTop = 1500; b.dispatchEvent(new Event('scroll')); true")
            await asyncio.sleep(0.2)
            await ev(f"{PINIA}._s.get('preview#' + '{left_id}').requestMdAction('edit'); true")
            ok = await wait_for("document.querySelector('.md-editor') !== null", timeout=5.0)
            check("T6 进编辑态", ok)
            await ev(f"{PINIA}._s.get('preview#' + '{left_id}').requestMdAction('preview'); true")
            ok = await wait_for("document.querySelector('.md-editor') === null && getComputedStyle(document.querySelector('.md-body')).display !== 'none'", timeout=5.0)
            await asyncio.sleep(0.3)
            pos6 = await ev("document.querySelector('.md-body').scrollTop")
            check("T6 回预览态滚动不丢", isinstance(pos6, (int, float)) and pos6 > 1000, f"pos={pos6}")

            code = 1 if fails else 0
    finally:
        subprocess.run(["taskkill", "/F", "/PID", str(edge.pid), "/T"], capture_output=True)
    print("RESULT:", "FAIL" if fails else "PASS", fails if fails else "")
    return code


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
