# CDP 探针：MD 链接点击跳转（2026-10-08 用户实机报「手型指针但不跳转」后的实证桩）
#   场景：md 文档含五种引用——anikit 链接 / anikit 图片 / http 图片 / http 带跳转组合 / 真外链
#   断言：①渲染形态（img.md-asset / a[href^=anikit:]）②anikit 与 http open 形态点击 =
#   新开标签页定位（当前页预览不打断）③重复点击激活复用不堆积 ④真外链走 open_external_url
#   ⑤整窗无导航（location 不变）
# 桩：fa 文件夹 = md1 + a1 图片；invoke 计数器盯 open_external_url。
# 用法：先起 pnpm dev（5373），再 tools/.venv/Scripts/python.exe tools/cdp-md-link-probe.py
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
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-mdlink-{os.getpid()}")

MD = """# 链接测试

[内部链接](anikit://item/a1)

![内部图片](anikit://item/a1)

![HTTP图片](http://127.0.0.1:17701/item/a1)

[![带跳转](http://127.0.0.1:17701/item/a1)](http://127.0.0.1:17701/open/item/a1)

[外部链接](https://www.example.com)
"""

STUB = r"""
window.__invokeLog = [];
window.__readCount = 0;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    window.__invokeLog.push(cmd);
    const P = (o) => Object.assign({
      w: 1000, h: 800, thumbW: null, thumbH: null, thumbPath: "E:/fake/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: null, animated: false, duration: null, bpm: null,
      importedFrom: null, createdAt: null, modifiedAt: "2026-09-23T10:00:00+08:00",
      importedAt: "2026-09-23T10:00:00+08:00", noteCount: null, thumbFrame: null, fps: null,
      playAssetId: null, hidden: false, wsPath: null, tags: [], promptState: "none",
    }, o);
    const fa = [P({ assetId: "md1", folderId: "fa", name: "测试文档.md", ext: "md", size: 2048 })];
    fa.push(P({ assetId: "a1", folderId: "fa", name: "甲图01.png", ext: "png", size: 102400 }));
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 2, folderCount: 1 },
      get_ui_settings: {},
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [ { id: "fa", name: "甲夹", count: 2, totalSize: 104448, children: [] } ] },
      list_placements: fa, list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_relations: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      take_pending_deep_link: null,
    };
    if (cmd === "read_text_asset") {
      window.__readCount++;
      return Promise.resolve({ content: "__MD__" });
    }
    const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
    return Promise.resolve(v === null ? null : structuredClone(v));
  },
};
window.__cbId = 0; window.__cbs = {};
""".replace("__MD__", MD.replace("\\", "\\\\").replace("`", "\\`").replace("\n", "\\n"))

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

            async def click_sel(sel):
                rect = await ev(f"JSON.stringify(document.querySelector('{sel}')?.getBoundingClientRect() ?? null)")
                if rect is None:
                    return False
                r = json.loads(rect)
                cx, cy = r["left"] + r["width"] / 2, r["top"] + r["height"] / 2
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "x": cx, "y": cy, "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            async def snap():
                return json.loads(await ev(f"""JSON.stringify({{
                  tabs: {PINIA}.state.value.tabs.tabs.map(t => t.viewKey),
                  active: (() => {{ const p = {PINIA}; const id = p.state.value.tabs.activeTabId; return p.state.value.tabs.tabs.find(t => t.id === id)?.viewKey ?? null; }})(),
                  previewKey: (() => {{ const p = {PINIA}; const id = p.state.value.tabs.activeTabId; return p._s.get('preview#' + id)?.assetKey ?? null; }})(),
                  openExtCount: window.__invokeLog.filter(c => c === 'open_external_url').length,
                  href: location.href,
                }})"""))

            # S0 就绪 + 打开 md 预览
            ok = await wait_for(f"!!{PINIA}")
            check("S0 app 就绪", ok)
            await ev(f"{PINIA}._s.get('tabs').openView('folder:fa'); true")
            await asyncio.sleep(0.5)
            await ev(f"""(async () => {{
              const p = {PINIA};
              const s = p._s.get('preview#' + p.state.value.tabs.activeTabId);
              await s.open('md1|fa');
              return true;
            }})()""")
            ok = await wait_for("!!document.querySelector('.md-body') && document.querySelector('.md-body').innerHTML.length > 50")
            check("S1 md 预览渲染", ok)

            # R1 渲染形态：img.md-asset ×3（anikit/http/带跳转组合）+ a[href^=anikit:] ×1（纯文字链接）
            form = await ev("""JSON.stringify({
              imgs: document.querySelectorAll('.md-body img.md-asset').length,
              anikitLinks: document.querySelectorAll('.md-body a[href^="anikit:"]').length,
              allA: [...document.querySelectorAll('.md-body a')].map(a => a.getAttribute('href')),
              extLink: !!document.querySelector('.md-body a[href="https://www.example.com"]'),
            })""")
            form = json.loads(form)
            print(f"  渲染形态: {form}")
            check("R1 图片渲染 img.md-asset ×3", form["imgs"] == 3, f"imgs={form['imgs']}")
            check("R1 外部链接 href 保留", form["extLink"])

            base = await snap()
            print(f"  基线: tabs={base['tabs']} active={base['active']} preview={base['previewKey']}")

            # R2 点 anikit 文字链接 → 新开标签页 folder:fa（a1 所在），预览页原样保留
            hit = await click_sel('.md-body a[href^="anikit:"]')
            check("R2 链接元素在", hit)
            s = await snap()
            print(f"  点内部链接后: tabs={s['tabs']} active={s['active']}")
            check("R2 新开标签页定位", "folder:fa" in s["tabs"] and s["tabs"][-1] == "folder:fa" and s["active"] == "folder:fa", f"tabs={s['tabs']} active={s['active']}")
            check("R2 无浏览器调用", s["openExtCount"] == 0, f"openExt={s['openExtCount']}")
            check("R2 整窗未导航", "localhost:5373" in s["href"], s["href"])

            # R3 回预览页再点一次同目标 → 激活复用不新增
            await ev(f"{PINIA}._s.get('tabs').activateTab({PINIA}.state.value.tabs.tabs[0].id); true")
            await asyncio.sleep(0.3)
            await ev(f"""(async () => {{
              const p = {PINIA};
              const s = p._s.get('preview#' + p.state.value.tabs.activeTabId);
              if (s && !s.assetKey) await s.open('md1|fa');
              return true;
            }})()""")
            await asyncio.sleep(0.4)
            n0 = len((await snap())["tabs"])
            await click_sel('.md-body a[href^="anikit:"]')
            s = await snap()
            check("R3 重复点击激活复用不堆积", len(s["tabs"]) == n0 and s["active"] == "folder:fa", f"tabs={s['tabs']}")

            # R4 点 http 带跳转组合图（a>img 嵌套）→ 同样内部新开页，不走浏览器
            await ev(f"{PINIA}._s.get('tabs').activateTab({PINIA}.state.value.tabs.tabs[0].id); true")
            await asyncio.sleep(0.3)
            hit = await click_sel('.md-body a[href*="open"] img.md-asset')
            if not hit:
                hit = await click_sel(".md-body img.md-asset")
            s = await snap()
            print(f"  点带跳转图后: tabs={s['tabs']} active={s['active']} openExt={s['openExtCount']}")
            check("R4 http open 形态内部跳转", hit and s["active"] == "folder:fa", f"hit={hit} active={s['active']}")
            check("R4 无浏览器调用", s["openExtCount"] == 0, f"openExt={s['openExtCount']}")
            check("R4 整窗未导航", "localhost:5373" in s["href"], s["href"])

            # R5 点真外链 → open_external_url（桩计数），页面不动
            await ev(f"{PINIA}._s.get('tabs').activateTab({PINIA}.state.value.tabs.tabs[0].id); true")
            await asyncio.sleep(0.3)
            hit = await click_sel('.md-body a[href="https://www.example.com"]')
            s = await snap()
            check("R5 真外链走 open_external_url", hit and s["openExtCount"] == 1, f"hit={hit} openExt={s['openExtCount']}")
            check("R5 整窗未导航", "localhost:5373" in s["href"], s["href"])

            print()
            if fails:
                print(f"FAIL {len(fails)}: {fails}")
                code = 1
            else:
                print("ALL PASS")
    finally:
        edge.terminate()
    return code


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
