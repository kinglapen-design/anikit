# CDP 实测：扫描重复文件（35_扫描重复文件稿 + Eagle 式合并前确认，2.7 二轮 2026-08-13）
# 桩有库进主界面 → 汉堡"查找"▸"扫描重复文件"（openScan({kind:"all"}) → scan_duplicates 桩返 3 组假数据）。
# 屏 1：714 宽 / 标题 / 3 组行复选默认全选 / 底部"已选择 7 个文件""4.0 MB" / 组 C 卡片横向可滚 /
# 红钮移除组 B 剩 2 组 5 文件；屏 2：合并前确认 970 宽 / 左列 2 组（A 在前 .cur）/ 右列保留者 radio
# 默认 a1 / 改选 a2 再改回 a1 → 确认合并 delete_placements items = 落选全 placement（a2 链接一并回收）。
# 重开点 X = 不调写命令。截图 tools/style-preview/scan-duplicates.png / scan-duplicates-confirm.png。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9347
URL = "http://localhost:5373"

# 缩略图：指向 assets/image 真实图片，convertFileSrc 桩转 vite /@fs/ 可加载 URL（截图好看）
THUMB = "E:/01_Projects/03_Software-dev/AniKit/assets/image/Pasted image 20260803215034.png"

STUB = r"""
window.__cmds = [];
window.__scanCalls = [];
window.__deleteCalls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => encodeURI("/@fs/" + p),
  transformCallback: () => 0,
  invoke: (cmd, args) => {
    window.__cmds.push(cmd);
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 0, folderCount: 0, autoImport: false };
    const THUMB = "__THUMB__";
    const asset = (assetId, name, folderId, folderName, w, h, ext, thumbPath, placements) => ({
      assetId, name, folderId, folderName, w, h, ext, size: 0,
      thumbPath, thumbKind: thumbPath ? "asset" : null, placements,
    });
    const GROUPS = [
      { hash: "sha256:a", size: 2097152, assets: [
        { ...asset("a1", "场景-远景.psd", "f1", "03_外景", 1920, 1080, "psd", THUMB, [{ folderId: "f1", origin: true }]), size: 2097152 },
        { ...asset("a2", "场景-远景-副本.psd", "root", "未分类", 1920, 1080, "psd", null, [{ folderId: "root", origin: true }, { folderId: "f2", origin: false }]), size: 2097152 },
      ]},
      { hash: "sha256:b", size: 1048576, assets: [
        { ...asset("b1", "走路-循环.fla", "f3", "01_动作", null, null, "fla", THUMB, [{ folderId: "f3", origin: true }]), size: 1048576 },
        { ...asset("b2", "走路-循环-副本.fla", "root", "未分类", null, null, "fla", null, [{ folderId: "root", origin: true }]), size: 1048576 },
      ]},
      { hash: "sha256:c", size: 524288, assets: [
        { ...asset("c1", "特效-火.png", "f4", "02_特效", 800, 450, "png", THUMB, [{ folderId: "f4", origin: true }]), size: 524288 },
        { ...asset("c2", "特效-火-副本.png", "root", "未分类", 800, 450, "png", null, [{ folderId: "root", origin: true }]), size: 524288 },
        { ...asset("c3", "特效-火-副本2.png", "f5", "05_旧稿", 800, 450, "png", null, [{ folderId: "f5", origin: true }]), size: 524288 },
      ]},
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_folder_tree: [], list_recycle: [], list_recent_assets: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
    };
    if (cmd === "scan_duplicates") {
      window.__scanCalls.push(args);
      return Promise.resolve(JSON.parse(JSON.stringify(GROUPS)));
    }
    if (cmd === "delete_placements") {
      window.__deleteCalls.push(args);
      return Promise.resolve({ moved: args.items.length, failed: [] });
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
};
""".replace("__THUMB__", THUMB)

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
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


async def main():
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,1200", "about:blank"],
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

            async def rect_of(expr):
                return await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")

            async def click(expr):
                rect = await rect_of(expr)
                if not rect:
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            async def hover(expr):
                rect = await rect_of(expr)
                if not rect:
                    print("!! 元素不存在:", expr[:80])
                    return False
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": rect["x"], "y": rect["y"]}, session=sess)
                await asyncio.sleep(0.6)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            # 汉堡点击链：汉堡钮 → hover"查找"出二级 → 点"扫描重复文件"
            async def open_scan_via_menu():
                await click("document.querySelector('.bar-title .icon-btn')")
                await hover("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.trim().startsWith('查找'))")
                await asyncio.sleep(0.4)
                await click("[...document.querySelectorAll('.app-menu .menu-item')].find((m) => m.textContent.trim() === '扫描重复文件')")
                await asyncio.sleep(1.0)

            fails = []

            await open_scan_via_menu()
            opened = await ev("!!document.querySelector('.scan-dialog .app-modal')")
            scan_calls = await ev("window.__scanCalls")
            print("弹窗打开:", opened, "scan_duplicates 入参:", json.dumps(scan_calls, ensure_ascii=False))
            if not opened:
                print("FAIL: 弹窗未打开")
                return 3
            if scan_calls != [{"scope": {"kind": "all"}}]:
                fails.append(f"scan_duplicates 入参 {json.dumps(scan_calls, ensure_ascii=False)}")

            # ① 屏 1：壳 714 / 标题 / 3 组全选 / 底部计数
            info = await ev("""({
              title: document.querySelector('.scan-dialog .app-modal .title')?.textContent ?? null,
              width: document.querySelector('.scan-dialog .app-modal')?.getBoundingClientRect().width ?? null,
              groups: document.querySelectorAll('.scan-group').length,
              checked: document.querySelectorAll('.scan-group .app-checkbox.on').length,
              summary: document.querySelector('.scan-bottom .summary')?.textContent.replace(/\\s+/g, ' ').trim() ?? null,
            })""")
            print("屏 1:", json.dumps(info, ensure_ascii=False))
            if info["title"] != "扫描重复文件":
                fails.append(f"屏1标题 {info['title']}")
            if info["width"] != 717:  # 内容 714 + 真 border 3（2026-08-13 二级窗口外框改真 border）
                fails.append(f"屏1宽 {info['width']}")
            if info["groups"] != 3:
                fails.append(f"组数 {info['groups']}")
            if info["checked"] != 3:
                fails.append(f"默认全选 {info['checked']}")
            if "已选择 7 个文件" not in info["summary"] or "4.0 MB" not in info["summary"]:
                fails.append(f"底部计数 {info['summary']}")

            # ② 组 C 卡片横向可滚
            scroll = await ev("(() => { const c = document.querySelectorAll('.scan-group')[2]?.querySelector('.cards'); return c ? { sw: c.scrollWidth, cw: c.clientWidth } : null; })()")
            print("组 C 滚动:", scroll)
            if not scroll or scroll["sw"] <= scroll["cw"]:
                fails.append(f"组 C 无横向滚动 {scroll}")

            # ②b 悬浮滚轮（2026-08-13 定稿，主窗口统一交互）：屏 1 可滚 + ScrollThumb 存在 +
            #     hover 包裹层后可见 + 无 gutter 挤压（两卡组内容恰 666 不出横条）
            sb = await ev("""(() => {
              const b = document.querySelector('.groups-body');
              const rowA = document.querySelectorAll('.scan-group')[0]?.querySelector('.cards');
              return b ? { sh: b.scrollHeight, ch: b.clientHeight, ow: b.offsetWidth, cw: b.clientWidth,
                           thumbs: document.querySelectorAll('.scroll-thumb').length,
                           aSw: rowA?.scrollWidth ?? 0, aCw: rowA?.clientWidth ?? 0 } : null;
            })()""")
            print("屏1滚轮:", sb)
            if not sb or sb["sh"] <= sb["ch"]:
                fails.append(f"屏 1 未溢出（桩数据应可滚）{sb}")
            if not sb or sb["thumbs"] < 1:
                fails.append(f"ScrollThumb 缺失 {sb}")
            if sb and sb["aSw"] > sb["aCw"] + 1:
                fails.append(f"两卡组误出横向条 {sb}")
            # hover 屏 1 包裹层 → 悬浮滚轮显形
            await hover("document.querySelector('.scroll-wrap')")
            await asyncio.sleep(0.4)  # 原生 mouseenter → 响应式 hover → visible 类需一个渲染周期
            tb = await ev("""(() => {
              const t = document.querySelector('.scroll-wrap .scroll-thumb');
              return t ? { visible: t.classList.contains('visible'), h: t.getBoundingClientRect().height } : null;
            })()""")
            print("悬浮滚轮:", tb)
            if not tb or not tb["visible"] or tb["h"] < 20:
                fails.append(f"悬浮滚轮 hover 未显 {tb}")

            await shot("scan-duplicates")

            # ③ 组 B 红钮移除 → 剩 2 组 5 文件
            await click("document.querySelectorAll('.scan-group')[1].querySelector('.icon-btn.delete-red')")
            info2 = await ev("""({
              groups: document.querySelectorAll('.scan-group').length,
              summary: document.querySelector('.scan-bottom .summary')?.textContent.replace(/\\s+/g, ' ').trim() ?? null,
            })""")
            print("移除组 B 后:", json.dumps(info2, ensure_ascii=False))
            if info2["groups"] != 2 or "已选择 5 个文件" not in info2["summary"]:
                fails.append(f"移除组 B {json.dumps(info2, ensure_ascii=False)}")

            # ④ 合并 → 屏 2：970 / 左列 2 组 A 在前 .cur / 右列 keeper × 2
            await click("[...document.querySelectorAll('.scan-bottom .app-btn')].find((b) => b.textContent.includes('合并'))")
            await asyncio.sleep(0.6)  # 宽度过渡 0.2s
            info3 = await ev("""({
              title: document.querySelector('.scan-dialog .app-modal .title')?.textContent ?? null,
              width: document.querySelector('.scan-dialog .app-modal')?.getBoundingClientRect().width ?? null,
              groupItems: document.querySelectorAll('.group-item').length,
              curName: document.querySelector('.group-item.cur .group-name')?.textContent ?? null,
              firstName: document.querySelector('.group-item .group-name')?.textContent ?? null,
              keepers: document.querySelectorAll('.keeper-item').length,
              keeperNames: [...document.querySelectorAll('.keeper-item .keeper-name')].map((n) => n.textContent),
            })""")
            print("屏 2:", json.dumps(info3, ensure_ascii=False))
            if info3["title"] != "合并前确认":
                fails.append(f"屏2标题 {info3['title']}")
            if info3["width"] != 973:  # 内容 970 + 真 border 3
                fails.append(f"屏2宽 {info3['width']}")
            if info3["groupItems"] != 2:
                fails.append(f"屏2组数 {info3['groupItems']}")
            if info3["firstName"] != "场景-远景.psd" or info3["curName"] != "场景-远景.psd":
                fails.append(f"组序/当前组 {info3['firstName']} {info3['curName']}（应 A 在前且 .cur）")
            if info3["keepers"] != 2:
                fails.append(f"keeper 数 {info3['keepers']}")

            # ⑤ 默认保留者 = a1（assets[0] = 导入最早）
            on0 = await ev("""({
              onIdx: [...document.querySelectorAll('.keeper-item')].findIndex((k) => k.classList.contains('on')),
              radioOnIdx: [...document.querySelectorAll('.keeper-item')].findIndex((k) => !!k.querySelector('.app-radio.on')),
            })""")
            print("默认保留者:", on0)
            if on0["onIdx"] != 0 or on0["radioOnIdx"] != 0:
                fails.append(f"默认保留者 {on0}")

            await shot("scan-duplicates-confirm")

            # ⑥ 改选 a2 → a2 变选中；改回 a1 → 确认合并：
            #    items = 落选 a2 全部 placement（root origin:true + f2 链接 origin:false）+ c2/c3，a1 不在
            await click("document.querySelectorAll('.keeper-item')[1]")
            on1 = await ev("[...document.querySelectorAll('.keeper-item')].findIndex((k) => k.classList.contains('on'))")
            print("改选 a2 后 onIdx:", on1)
            if on1 != 1:
                fails.append(f"改选保留者失败 onIdx={on1}")
            await click("document.querySelectorAll('.keeper-item')[0]")
            await click("[...document.querySelectorAll('.scan-bottom .app-btn')].find((b) => b.textContent.includes('确认合并'))")
            await asyncio.sleep(1.0)
            calls = await ev("window.__deleteCalls")
            closed = await ev("!document.querySelector('.scan-dialog .app-modal')")
            print("delete_placements:", json.dumps(calls, ensure_ascii=False), "弹窗关闭:", closed)
            if len(calls) != 1:
                fails.append(f"delete_placements 调用数 {len(calls)}")
            else:
                c = calls[0]
                items = c.get("items", [])
                a2_items = [i for i in items if i["assetId"] == "a2"]
                ok = (
                    c.get("lastAction") == "recycle"
                    and len(a2_items) == 2
                    and {"folderId": "root", "origin": True} in [{k: i[k] for k in ("folderId", "origin")} for i in a2_items]
                    and {"folderId": "f2", "origin": False} in [{k: i[k] for k in ("folderId", "origin")} for i in a2_items]
                    and not any(i["assetId"] == "a1" for i in items)
                    and {i["assetId"] for i in items} == {"a2", "c2", "c3"}
                )
                if not ok:
                    fails.append(f"合并提交项不符 {json.dumps(c, ensure_ascii=False)}")
            if not closed:
                fails.append("确认合并后弹窗未关闭")

            # ⑦ 重开 → 点 X 关闭 = 不调写命令
            await open_scan_via_menu()
            reopened = await ev("!!document.querySelector('.scan-dialog .app-modal')")
            await click("document.querySelector('.scan-dialog .app-modal .titlebar .icon-btn:not(.back-btn)')")
            closed2 = await ev("!document.querySelector('.scan-dialog .app-modal')")
            del_count = await ev("window.__deleteCalls.length")
            print("重开:", reopened, "X 关闭:", closed2, "delete_placements 次数:", del_count)
            if not reopened or not closed2 or del_count != 1:
                fails.append(f"取消路径 {reopened} {closed2} {del_count}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 屏1布局/计数/横向滚动/红钮移除/屏2双栏/默认保留者/改选/合并提交/取消均符合")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
