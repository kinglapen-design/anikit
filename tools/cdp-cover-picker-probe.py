# CDP 探针：档案封面（2026-09-08）——默认封面图 + 封面选择模式（picker cover 用途）
# 口径：① 封面链 = 自定义封面（?v= 版本戳）→ 子项首图 → 默认封面图（档案默认封面.png）；
#       ② 封面设置「更换封面 - 从资源库中选择」点亮 = 主窗口选择模式 cover 用途——
#       过滤非封面图（静态图 + gif；序列帧包 / 工程文件排除，档案区不渲染）、单击图片
#       单选直回不进备选区、退模式自动展开封面设置组取景态；③ 确认按取景参数烘培。
# 场景：
#   S0 卡片封面链：无封面档案卡 = 默认封面图；有封面档案卡 = coverPath + ?v=coverVer；
#      档案详情 pinned 无封面 = 默认封面图
#   S1 进入 cover 选择模式：菜单两项均可用 + 四重标识 + 横幅文案 + 过滤（png/gif 在，
#      fla/序列帧包排除，档案区不渲染）
#   S2 单选直回：点图片卡即退模式 + 封面设置组自动展开取景态（cs-view + 滑块解禁）
#   S3 确认：set_archive_cover_settings（folderId + cover{sourcePath,crop} + badge null）
#      → 退回常态 + 档案卡封面换 ?v=999（桩模拟覆盖重写后版本戳变化）
#   S4 Esc 放弃：重进后 Esc = 零写入 + 不自动展开封面设置组
# 截图留档 tools/style-preview/：cover-default / cover-pick-mode / cover-framing-from-library。
# 桩备注：convertFileSrc 用 /@fs/ 形式（vite 直供真图——取景态要读图片自然尺寸，
#   asset:// 形式 headless 下加载失败会永远进不了取景态）；桩返回 structuredClone 深拷贝（速查 §12）；
#   素材卡选择走真实指针事件（gridPointerDown 不吃合成 click）；收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；探针检测到未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-cover-picker-probe.py
import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19730 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-coverpick-{os.getpid()}")

STILL = "E:/01_Projects/03_Software-dev/AniKit/tools/test-assets/still.webp"

STUB = r"""
window.__calls = { set_archive_cover_settings: [] };
window.__cbId = 0; window.__cbs = {};
(function () {
  const STILL = "__STILL__";
  const DB = {
    archives: {
      // f1a 无封面（默认封面图断言面）；f1b 有封面（?v= 版本戳断言面）
      f1b: { folderId: "f1b", badge: null, avatar: { file: "f1b.webp" }, props: [], descriptions: [], links: [], createdAt: "", coverPath: STILL, coverVer: "123456" },
    },
  };
  const archiveRec = (id) => DB.archives[id] ?? { folderId: id, badge: null, avatar: null, props: [], descriptions: [], links: [], createdAt: "", coverPath: null, coverVer: null };
  window.__TAURI_INTERNALS__ = {
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    convertFileSrc: (p) => "/@fs/" + encodeURI(p),
    transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
    invoke: (cmd, args) => {
      const R = {
        get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 3 },
        get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
        get_ui_settings: {},
        get_window_fx_mode: "solid",
        list_folder_tree: { blocks: [{ id: "dir", name: "目录", icon: null }],
          nodes: [
            { id: "f1", name: "素材", count: 4, totalSize: 10000, children: [
              { id: "f1a", name: "雪天", count: 0, totalSize: 0, children: [], kind: "archive" },
              { id: "f1b", name: "设定集", count: 0, totalSize: 0, children: [], kind: "archive" },
            ] },
          ] },
        list_placements: [
          { assetId: "a1", folderId: "f1", name: "雪牧.png", ext: "png", size: 1000,
            w: 800, h: 600, thumbW: 64, thumbH: 48, thumbKind: "asset", thumbPath: STILL, origin: true,
            frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
            createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
            importedAt: "2026-07-12T14:26:00+08:00" },
          { assetId: "a2", folderId: "f1", name: "阿拉蕾.fla", ext: "fla", size: 2000,
            w: null, h: null, thumbKind: null, thumbPath: null, origin: true,
            frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
            createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
            importedAt: "2026-07-12T14:26:00+08:00" },
          { assetId: "a3", folderId: "f1", name: "动图.gif", ext: "gif", size: 3000,
            w: 320, h: 240, thumbW: 64, thumbH: 48, thumbKind: "asset", thumbPath: STILL, origin: true,
            frameCount: 12, animated: true, duration: 1.2, bpm: null, importedFrom: null,
            createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
            importedAt: "2026-07-12T14:26:00+08:00" },
          { assetId: "a4", folderId: "f1", name: "走路序列.png", ext: "png", size: 4000,
            w: 800, h: 600, thumbW: 64, thumbH: 48, thumbKind: "asset", thumbPath: STILL, origin: true,
            frameCount: 8, animated: false, duration: null, bpm: null, importedFrom: null,
            createdAt: "2026-07-10T08:00:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
            importedAt: "2026-07-12T14:26:00+08:00" },
        ],
        list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
        list_inbox_snapshot: [], list_relations: [], list_prop_pool: [],
        list_space_libraries: ["测试库"],
        get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
        undo: null, redo: null, save_ui_settings: null, set_main_border_picking: null,
        list_archives: () => Object.values(DB.archives),
      };
      if (cmd === "get_archive_data") {
        const rec = archiveRec(args.folderId);
        return Promise.resolve(structuredClone({ ...rec, fileCount: 0, totalSize: 0 }));
      }
      if (cmd === "get_asset_file_path") {
        if (args.assetId === "a1" || args.assetId === "a3") {
          return Promise.resolve({ path: STILL, pending: false, animPreview: null });
        }
        return Promise.resolve({ path: null, pending: false, animPreview: null });
      }
      if (cmd === "set_archive_cover_settings") {
        window.__calls.set_archive_cover_settings.push(JSON.parse(JSON.stringify(args)));
        // 模拟覆盖式烘培落盘：封面换 STILL + 版本戳变化（同路径重写后 mtime 变）
        DB.archives[args.folderId] = { ...archiveRec(args.folderId), avatar: { file: args.folderId + ".webp" }, coverPath: STILL, coverVer: "999" };
        return Promise.resolve(null);
      }
      const v = Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null;
      return Promise.resolve(typeof v === "function" ? v() : (v === null ? null : structuredClone(v)));
    },
  };
})();
""".replace("__STILL__", STILL)

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

            async def click_sel(sel):
                ok = await ev(f"!!document.querySelector('{sel}')")
                if not ok:
                    return False
                await ev(f"document.querySelector('{sel}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))")
                await asyncio.sleep(0.4)
                return True

            async def pointer_click(sel):
                """真实指针按下+抬起（素材卡选择走 gridPointerDown，不吃合成 click）"""
                rect = await ev(
                    f"(() => {{ const el = document.querySelector('{sel}'); if (!el) return null;"
                    f" const r = el.getBoundingClientRect(); return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }}; }})()")
                if not rect:
                    return False
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # ---------- S0 封面链：卡片 + pinned 默认封面图；有封面 = ?v= 版本戳 ----------
            await click_sel('.tree-item[data-folder-id="f1"]')
            await asyncio.sleep(0.6)
            s0 = await ev("""(() => {
              const card = (id) => {
                const c = document.querySelector(`.ar-card[data-folder-id="${id}"] img`);
                return c ? decodeURIComponent(c.src) : null;
              };
              return { a: card("f1a"), b: card("f1b"), archives: document.querySelectorAll(".ar-card").length };
            })()""")
            check("S0 档案卡封面链：无封面 = 默认封面图；有封面 = coverPath（still.webp）",
                  isinstance(s0, dict) and "档案默认封面.png" in (s0.get("a") or "")
                  and "still.webp" in (s0.get("b") or ""),
                  json.dumps(s0, ensure_ascii=False))
            check("S0b 有封面卡 ?v= 版本戳（coverVer=123456 上 URL）",
                  isinstance(s0, dict) and "v=123456" in (s0.get("b") or ""), json.dumps(s0, ensure_ascii=False))
            # 档案详情 pinned 默认封面（视口 folder:f1a 未选中回退档案形态）；
            # 树子级默认折叠——先展开 f1 再点 f1a（R13 同款次序）
            await click_sel('.tree-item[data-folder-id="f1"] .exp')
            await asyncio.sleep(0.4)
            await click_sel('.tree-item[data-folder-id="f1a"]')
            await asyncio.sleep(0.8)
            s0c = await ev("""(() => {
              const img = document.querySelector('.sidebar-right .section-pinned .thumb.cover img');
              return img ? decodeURIComponent(img.src) : null;
            })()""")
            check("S0c 档案详情 pinned 无封面 = 默认封面图",
                  isinstance(s0c, str) and "档案默认封面.png" in s0c, str(s0c))
            await shot("cover-default")

            # 回 f1 视口并选中档案卡（真实用户路径：视口停在 f1，网格内才有图可选）——
            # 档案卡选择走真实指针事件（与素材卡同闸）
            await click_sel('.tree-item[data-folder-id="f1"]')
            await asyncio.sleep(0.5)
            await pointer_click('.ar-card[data-folder-id="f1a"]')
            await asyncio.sleep(0.6)
            s0d = await ev("""(() => {
              const img = document.querySelector('.sidebar-right .section-pinned .thumb.cover img');
              return { img: img ? decodeURIComponent(img.src) : null,
                sel: !!document.querySelector('.ar-card[data-folder-id="f1a"].selected') };
            })()""")
            check("S0d f1 视口选中档案卡 = 档案详情（pinned 默认封面图 + 卡选中）",
                  isinstance(s0d, dict) and "档案默认封面.png" in (s0d.get("img") or "") and s0d.get("sel") is True,
                  json.dumps(s0d, ensure_ascii=False))

            # ---------- S1 进入 cover 选择模式 ----------
            await click_sel('.sidebar-right .section-pinned .thumb.cover')
            await asyncio.sleep(0.6)
            cs = await ev("""(() => ({
              cs: !!document.querySelector('.sidebar-right .cover-settings'),
              change: [...document.querySelectorAll('.sidebar-right .cover-settings .act-btn')]
                .find((b) => b.textContent.includes('更换封面')) ? true : false,
            }))()""")
            check("S1a 点封面展开封面设置组（含更换封面钮）",
                  isinstance(cs, dict) and cs.get("cs") is True and cs.get("change") is True, json.dumps(cs))
            await ev("""[...document.querySelectorAll('.sidebar-right .cover-settings .act-btn')]
              .find((b) => b.textContent.includes('更换封面'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }))""")
            await asyncio.sleep(0.5)
            menu = await ev("[...document.querySelectorAll('.app-menu .menu-item')].map((m) => ({ t: m.textContent.trim(), d: m.disabled }))")
            check("S1b 更换封面菜单两项均可用（从资源库中选择不再 disabled）",
                  isinstance(menu, list) and len(menu) == 2
                  and menu[0]["t"] == "从本地磁盘中选择" and menu[0]["d"] is False
                  and menu[1]["t"] == "从资源库中选择" and menu[1]["d"] is False,
                  json.dumps(menu, ensure_ascii=False))
            await ev("""[...document.querySelectorAll('.app-menu .menu-item')]
              .find((m) => m.textContent.includes('从资源库中选择'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }))""")
            await asyncio.sleep(0.7)
            s1 = await ev("""(() => {
              const w = document.querySelector('.window');
              const names = [...document.querySelectorAll('.asset-card')].map((c) => c.textContent);
              return {
                picking: w.classList.contains('picking'),
                banner: document.querySelector('.banner .text')?.textContent ?? null,
                panel: !!document.querySelector('.sidebar-right .picker-panel'),
                detail: !!document.querySelector('.sidebar-right .detail-root'),
                archiveCards: document.querySelectorAll('.ar-card').length,
                png: names.some((n) => n.includes('雪牧')),
                gif: names.some((n) => n.includes('动图')),
                fla: names.some((n) => n.includes('阿拉蕾')),
                seq: names.some((n) => n.includes('走路')),
              };
            })()""")
            check("S1c cover 选择模式：picking + 横幅「选择图片，为 [雪天] 设置封面」+ 备选区替换详情",
                  isinstance(s1, dict) and s1.get("picking") is True
                  and s1.get("banner") == "选择图片，为 [雪天] 设置封面"
                  and s1.get("panel") is True and s1.get("detail") is False,
                  json.dumps(s1, ensure_ascii=False))
            check("S1d 过滤非封面图：png/gif 在，fla / 序列帧包排除，档案区不渲染",
                  isinstance(s1, dict) and s1.get("png") is True and s1.get("gif") is True
                  and s1.get("fla") is False and s1.get("seq") is False and s1.get("archiveCards") == 0,
                  json.dumps(s1, ensure_ascii=False))
            s1e = await ev("""(() => {
              const p = document.querySelector('.sidebar-right .picker-panel');
              return p ? { ph: p.querySelector('input')?.placeholder ?? null,
                items: p.querySelectorAll('.pick-body .ni-card').length } : null;
            })()""")
            check("S1e 备选区 = 文件模板空态（占位「点击卡片选择」，零累积）",
                  isinstance(s1e, dict) and s1e.get("ph") == "点击卡片选择" and s1e.get("items") == 1,
                  json.dumps(s1e, ensure_ascii=False))
            await shot("cover-pick-mode")

            # ---------- S2 单选直回（不进备选区，直接取景态） ----------
            await pointer_click(".asset-card")
            await asyncio.sleep(1.0)
            s2 = await ev("""(() => {
              const view = document.querySelector('.sidebar-right .cover-settings .cs-view');
              const slider = document.querySelector('.sidebar-right .cover-settings .cs-slider');
              return {
                picking: document.querySelector('.window').classList.contains('picking'),
                panel: !!document.querySelector('.sidebar-right .picker-panel'),
                cs: !!document.querySelector('.sidebar-right .cover-settings'),
                view: view ? decodeURIComponent(view.src) : null,
                sliderOff: slider ? slider.classList.contains('off') : null,
              };
            })()""")
            check("S2 点图片卡 = 单选直回：退模式 + 备选区撤下 + 封面设置组自动展开取景态（cs-view 源图 + 滑块解禁）",
                  isinstance(s2, dict) and s2.get("picking") is False and s2.get("panel") is False
                  and s2.get("cs") is True
                  and "still.webp" in (s2.get("view") or "") and s2.get("sliderOff") is False,
                  json.dumps(s2, ensure_ascii=False))
            await shot("cover-framing-from-library")

            # ---------- S3 确认烘培 ----------
            await ev("""[...document.querySelectorAll('.sidebar-right .cs-actions .act-btn')]
              .find((b) => b.textContent.includes('确认'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }))""")
            # pinned 封面换 ?v=999（桩模拟覆盖重写后版本戳变化）——bumpDetailRev 重拉为异步，轮询
            s3 = None
            for _ in range(12):
                await asyncio.sleep(0.4)
                s3 = await ev("""(() => {
                  const img = document.querySelector('.sidebar-right .section-pinned .thumb.cover img');
                  return {
                    calls: window.__calls.set_archive_cover_settings,
                    cs: !!document.querySelector('.sidebar-right .cover-settings'),
                    cover: !!document.querySelector('.sidebar-right .section-pinned .thumb.cover'),
                    pinned: img ? decodeURIComponent(img.src) : null,
                  };
                })()""")
                if isinstance(s3, dict) and "v=999" in (s3.get("pinned") or ""):
                    break
            c0 = isinstance(s3, dict) and s3.get("calls") and s3["calls"][0] or None
            check("S3 确认 → set_archive_cover_settings（folderId f1a + cover{sourcePath,crop} + badge null）",
                  bool(c0 and c0.get("folderId") == "f1a"
                       and c0.get("cover", {}).get("crop") == {"x": 0, "y": 0, "scale": 1}
                       and c0.get("cover", {}).get("sourcePath", "").endswith("still.webp")
                       and c0.get("badge") is None),
                  json.dumps(c0, ensure_ascii=False))
            check("S3b 确认退回常态组 + pinned 封面换 ?v=999（覆盖重写版本戳驱动重载）",
                  isinstance(s3, dict) and s3.get("cs") is False and s3.get("cover") is True
                  and "v=999" in (s3.get("pinned") or ""),
                  json.dumps({k: s3.get(k) for k in ("cs", "cover", "pinned")}, ensure_ascii=False) if isinstance(s3, dict) else "no s3")

            # ---------- S4 Esc 放弃（零写入 + 不自动展开） ----------
            await click_sel('.sidebar-right .section-pinned .thumb.cover')
            await asyncio.sleep(0.5)
            await ev("""[...document.querySelectorAll('.sidebar-right .cover-settings .act-btn')]
              .find((b) => b.textContent.includes('更换封面'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }))""")
            await asyncio.sleep(0.4)
            await ev("""[...document.querySelectorAll('.app-menu .menu-item')]
              .find((m) => m.textContent.includes('从资源库中选择'))?.dispatchEvent(new MouseEvent('click', { bubbles: true }))""")
            await asyncio.sleep(0.6)
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(0.6)
            s4 = await ev("""(() => ({
              picking: document.querySelector('.window').classList.contains('picking'),
              cs: !!document.querySelector('.sidebar-right .cover-settings'),
              cover: !!document.querySelector('.sidebar-right .section-pinned .thumb.cover'),
              calls: window.__calls.set_archive_cover_settings.length,
            }))()""")
            check("S4 Esc = 放弃（退模式 + 零写入 + 常态封面组不自动展开）",
                  isinstance(s4, dict) and s4.get("picking") is False and s4.get("cs") is False
                  and s4.get("cover") is True and s4.get("calls") == 1,
                  json.dumps(s4))

            await send(ws, "Target.closeTarget", {"targetId": tid})
    finally:
        edge.terminate()
        kill_listening_port(PORT)
        if spawned:
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(spawned.pid)],
                           stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    print()
    if fails:
        print(f"失败 {len(fails)} 项：")
        for f in fails:
            print(" -", f)
        return 1
    print("全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
