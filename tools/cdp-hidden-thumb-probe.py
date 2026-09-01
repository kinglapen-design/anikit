# CDP 探针：8.1 hidden 伴生缩略图资产（PRD 02-15）
#   H1 全部视图：hidden 资产全部位置消失（普通图 + 工程.fla 两卡）；计数同步
#   H2 未分类视图：hidden 的 root 位置同样消失
#   H3 文件夹视图：hidden 的链接位置（f1）消失
#   H4 缩略图视图：hidden 资产按资产显示一次（双位置去重）；计数 1
#   H5 工程卡片缩略图 = 代理 hidden 资产的图（thumbPath 指向 AN-T）
#   H6 搜索「封面」不命中 hidden 资产（空态）
# 动态绑定 / 还原 / 护栏由后端 cargo test 覆盖（asset_menu::set_thumb_asset / restore_thumb_asset），
# 本探针只验前端视图过滤口径。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-hidden-thumb-probe.py
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9361
URL = "http://localhost:5373"
SLEEP = 0.4

# 桩：AN-P 普通图（root）/ AN-F 工程.fla（root，缩略图已代理 AN-T）/
# AN-T 封面.webp hidden 双位置（root + f1 链接）。hidden 过滤与去重是前端口径（content.placementsFor）。
STUB = r"""
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => cb,
  invoke: (cmd, args) => {
    const mk = (assetId, folderId, name, ext, extra) => Object.assign({
      assetId, folderId, name, w: 800, h: 600, ext, thumbPath: null, thumbKind: null,
      origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
      size: 100, createdAt: "2026-09-01T10:00:00+08:00",
      modifiedAt: "2026-09-01T10:00:00+08:00", importedAt: "2026-09-01T10:00:00+08:00",
      noteCount: null, hidden: false,
    }, extra || {});
    const PLACEMENTS = [
      mk("AN-P", "root", "普通图.png", "png", { thumbPath: "E:/t/p_thumb.webp", thumbKind: "asset" }),
      mk("AN-F", "root", "工程.fla", "fla", { w: null, h: null,
        thumbPath: "E:/t/ANT_thumb.webp", thumbKind: "asset", thumbW: 320, thumbH: 180 }),
      mk("AN-T", "root", "封面.webp", "webp",
        { hidden: true, thumbPath: "E:/t/ANT_thumb.webp", thumbKind: "asset", thumbW: 320, thumbH: 180 }),
      mk("AN-T", "f1", "封面链接.webp", "webp",
        { hidden: true, origin: false, thumbPath: "E:/t/ANT_thumb.webp", thumbKind: "asset" }),
    ];
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 3, folderCount: 1 },
      get_ui_settings: {},
      list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [{ id: "f1", name: "角色", count: 1, children: [] }] },
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [], list_inbox_snapshot: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    if (cmd === "search_assets") {
      // 后端同口径简化：hidden 资产不命中（services/search.rs 8.1 过滤）
      const kws = (args?.query || "").trim().toLowerCase().split(/\s+/).filter(Boolean);
      const placements = [];
      if (kws.length) {
        for (const p of PLACEMENTS) {
          if (p.hidden) continue;
          if (kws.every((k) => p.name.toLowerCase().includes(k))) {
            placements.push({ assetId: p.assetId, folderId: p.folderId });
          }
        }
      }
      return Promise.resolve({ placements, archives: [] });
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
};
"""

_msg_id = 0


async def send(ws, method, params=None, session=None):
    global _msg_id
    _msg_id += 1
    payload = {"id": _msg_id, "method": method, "params": params or {}}
    if session:
        payload["sessionId"] = session
    await ws.send(json.dumps(payload))
    while True:
        raw = json.loads(await ws.recv())
        if raw.get("id") == _msg_id:
            return raw


def kill_listening_port(port):
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                subprocess.run(["taskkill", "/PID", line.split()[-1], "/F"], capture_output=True)
    except Exception:
        pass


async def main():
    fails = []
    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    kill_listening_port(PORT)
    edge = subprocess.Popen([
        EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu",
        "--window-size=1440,900", "--no-first-run", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            async def card_names():
                return await ev("[...document.querySelectorAll('.asset-card .kc-name')].map((el) => el.textContent.trim())")

            async def click_quick(label):
                return await ev(f"""
                  (() => {{ const items = [...document.querySelectorAll('.tree-item')];
                    const it = items.find((el) => el.textContent.includes({json.dumps(label)}));
                    if (!it) return false; it.click(); return true; }})()
                """)

            async def quick_count(label):
                return await ev(f"""
                  (() => {{ const items = [...document.querySelectorAll('.tree-item')];
                    const it = items.find((el) => el.textContent.includes({json.dumps(label)}));
                    if (!it) return null;
                    const m = it.textContent.match(/(\\d[\\d,]*)\\s*$/);
                    return m ? m[1].replace(/,/g, "") : null; }})()
                """)

            print("== 8.1 hidden 伴生缩略图探针 ==")

            # H1 全部视图（默认 quick:all）
            names = await card_names()
            check("H1a 全部视图 hidden 双位置消失", names == ["普通图.png", "工程.fla"] or sorted(names or []) == sorted(["普通图.png", "工程.fla"]), str(names))
            c = await quick_count("全部")
            check("H1b 全部计数 = 2（不含 hidden）", c == "2", str(c))

            # H2 未分类视图
            await click_quick("未分类")
            await asyncio.sleep(SLEEP)
            names = await card_names()
            check("H2 未分类视图 hidden 的 root 位置消失", sorted(names or []) == sorted(["普通图.png", "工程.fla"]), str(names))

            # H3 f1 文件夹视图（hidden 的链接位置消失）
            await click_quick("角色")
            await asyncio.sleep(SLEEP)
            names = await card_names()
            check("H3 文件夹视图 hidden 链接位置消失", names == [], str(names))

            # H4 缩略图视图（按资产去重）
            await click_quick("缩略图")
            await asyncio.sleep(SLEEP)
            names = await card_names()
            check("H4a 缩略图视图按资产显示一次", names == ["封面.webp"], str(names))
            c = await quick_count("缩略图")
            check("H4b 缩略图计数 = 1（去重）", c == "1", str(c))

            # H5 fla 卡片代理图（回全部视图断言 img src 指向 AN-T 代理图）
            await click_quick("全部")
            await asyncio.sleep(SLEEP)
            src = await ev("""
              (() => { const cards = [...document.querySelectorAll('.asset-card')];
                const fla = cards.find((c) => c.textContent.includes('工程.fla'));
                const img = fla && fla.querySelector('img');
                return img ? decodeURIComponent(img.src) : null; })()
            """)
            check("H5 工程卡片显示代理缩略图", bool(src) and "ANT_thumb" in src, str(src))

            # H6 搜索不命中 hidden（搜「封面」= 空态）
            await ev("""
              (() => { const i = document.querySelector('.fbs-input');
                if (!i) return false;
                i.value = '封面'; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()
            """)
            await asyncio.sleep(1.2)
            names = await card_names()
            empty = await ev("document.body.textContent.includes('无搜索结果')")
            check("H6 搜索不命中 hidden 资产", names == [] and bool(empty), f"cards={names} empty={empty}")

    finally:
        edge.terminate()
        if spawned:
            spawned.kill()
            # vite node 进程脱离 pnpm 壳进程树（踩坑 045/054）——壳杀不死端口直杀兜底
            kill_listening_port(5373)
    print("=" * 40)
    if fails:
        print(f"失败 {len(fails)} 项：")
        for f in fails:
            print(f"  - {f}")
        return 1
    print("全部 PASS")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
