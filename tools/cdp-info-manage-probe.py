# CDP 探针：信息查看管理面板轮（2026-09-26，速查 §61）——三 tab 化重立（旧 22 稿 SectionEditPanel 后继）
#   M1  选素材卡 a1 → 文件形态详情 + 底部「信息查看管理」钮
#   M2  开弹窗：BaseModal framed 460 / 标题「信息查看管理」/ 分段三段（基础信息/链接关系/提示词）
#   M3  默认落 basic（详情当前 tab）：4 行序 = 名称与链接/标签/详情/基本数据；首行禁上末行禁下
#   M4  切链接关系：5 行序 = 链接/音频/关系/缩略图/污染文件
#   M5  音频行上移 → 行序变 + save_ui_settings 记录 detailBlocks.links.order 音频首（即时生效持久化）
#   M6  关系行眼睛 → 标题 off 禁用色 + 眼睛 svg 切换（view-off）+ hidden 含 relations
#   M7  关弹窗切详情 links tab：区块序 = 音频/链接/缩略图/污染文件（排序+显隐双生效，hidden 整块不渲染）
#   M8  重开弹窗（详情 links tab → 弹窗落 links 段）：行序持久（音频首 / 关系行 off）
#   M9  提示词段：1 行上下箭头全禁（单区块仅显隐）
#   M10 视图菜单「信息查看管理」点亮：不 disabled，点击开同一弹窗（第二入口）
#   M11 basic 标签行上移 + 详情行眼睛 → 详情 basic 区块类序 = tags/inputs/data（v-for 配置驱动 + hidden 过滤）
#   截图 info-manage.png（M8 后弹窗 links 态含 off 行）
# 桩：_probe_stub 共享桩工厂（2026-09-25 T0 范式）；save_ui_settings 记录 / get_asset_detail 按 assetId 分桩走 handlers。
# 坑位沿用：端口/profile 随 pid 唯一 + 禁缓存；素材卡选择走真实指针事件（gridPointerDown 不吃合成 click）；
# 防抖写回 500ms——读 __saved 前 sleep ≥0.8s；收尾 taskkill /F /T 整树杀（踩坑 056）。
# 用法：先起 pnpm dev（5373；未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-info-manage-probe.py
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
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from _probe_stub import make_stub  # noqa: E402

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19230 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-infomanage-{os.getpid()}")
SHOT = "tools/style-preview/info-manage.png"

STUB = make_stub(
    {
        "get_current_library": {"name": "测试库", "path": "E:/test-lib"},
        "list_folder_tree": {
            "blocks": [{"id": "dir", "name": "目录", "icon": None}],
            "nodes": [{"id": "f1", "name": "角色", "count": 1, "totalSize": 1000, "children": []}],
        },
        "list_placements": [
            {
                "assetId": "a1", "folderId": "f1", "name": "雪牧_自动.fla", "ext": "fla", "size": 1000,
                "w": None, "h": None, "thumbKind": None, "thumbPath": None, "origin": True,
                "frameCount": None, "animated": False, "duration": None, "bpm": None, "importedFrom": None,
                "createdAt": "2026-07-10T08:00:00+08:00", "modifiedAt": "2026-07-13T09:00:00+08:00",
                "importedAt": "2026-07-12T14:26:00+08:00",
            },
        ],
    },
    head="""
window.__DETAILS = {
  a1: {
    assetId: "a1", originName: "雪牧_自动.fla", ext: "fla", size: 1000,
    width: null, height: null, duration: null,
    importedAt: "2026-07-12T14:26:00+08:00", modifiedAt: "2026-07-13T09:00:00+08:00",
    tags: ["男", "16岁"],
    descriptions: [{ title: "身份", content: "沙雕动画的御用主角" }],
    colors: [], links: [], note: "",
    placements: [{ folderId: "f1", blockName: "目录", path: ["角色"], name: "雪牧_自动.fla", origin: true }],
    archives: [], thumbAsset: null, thumbUsedBy: [],
  },
};
""",
    handlers="""
    if (cmd === "get_asset_detail") return Promise.resolve(window.__DETAILS[args.assetId] ?? null);
    if (cmd === "save_ui_settings") { (window.__saved = window.__saved || []).push(args.settings); }
""",
)

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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    return {"__error__": r["result"]["exceptionDetails"].get("text", "eval error")}
                return r.get("result", {}).get("result", {}).get("value")

            def check(name, ok, detail=""):
                print(f"  [{'PASS' if ok else 'FAIL'}] {name}{(' — ' + detail) if detail else ''}")
                if not ok:
                    fails.append(f"{name}: {detail}")

            async def click_sel(selector):
                ok = await ev(f"!!document.querySelector('{selector}')")
                if not ok:
                    return False
                await ev(f"document.querySelector('{selector}').dispatchEvent(new MouseEvent('click', {{ bubbles: true }}))")
                await asyncio.sleep(0.4)
                return True

            async def pointer_click(selector):
                """真实指针按下+抬起（素材卡选择走 gridPointerDown，不吃合成 click）"""
                rect = await ev(
                    f"(() => {{ const el = document.querySelector('{selector}'); if (!el) return null;"
                    f" const r = el.getBoundingClientRect(); return {{ x: r.x + r.width / 2, y: r.y + r.height / 2 }}; }})()")
                if not rect:
                    return False
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await send(ws, "Input.dispatchMouseEvent",
                           {"type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": "left", "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            # 弹窗行读取（标题序 / off 态 / 眼睛 svg 长度 / 箭头禁用态）
            ROWS_JS = (
                "[...document.querySelectorAll('.info-manage .im-row')].map(r => ({"
                " t: r.querySelector('.row-title').textContent,"
                " off: r.querySelector('.row-title').classList.contains('off'),"
                " eye: r.querySelector('.ic-btn:last-child svg').outerHTML.length,"
                " up: r.querySelector('.sort-arrow.arrow-up').disabled,"
                " down: r.querySelector('.sort-arrow.arrow-down').disabled }))")

            async def dlg_rows():
                return await ev(ROWS_JS)

            # 注入桩并加载
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            deadline = time.time() + 15
            while time.time() < deadline:
                ok = await ev("!!document.querySelector('.sidebar-right .detail-root')")
                if ok is True:
                    break
                await asyncio.sleep(0.4)
            await asyncio.sleep(1.0)

            # ---------- M1 选素材卡 a1 → 文件形态详情 ----------
            await pointer_click(".asset-card")
            deadline = time.time() + 8
            while time.time() < deadline:
                if await ev("!!document.querySelector('.sidebar-right .manage-bar')") is True:
                    break
                await asyncio.sleep(0.3)
            check("M1 选卡 a1 → 文件形态详情 + 信息查看管理钮",
                  await ev("!!document.querySelector('.sidebar-right .manage-bar .act-btn')") is True)
            if fails:
                print("M1 失败，后续场景无意义，提前收")
                return 1

            # ---------- M2 开弹窗 ----------
            await click_sel(".sidebar-right .manage-bar .act-btn")
            m2 = await ev(
                "(() => { const m = document.querySelector('.info-manage .bm-modal'); if (!m) return null;"
                " return { title: m.querySelector('.bm-title')?.textContent, w: m.offsetWidth,"
                "  framed: m.classList.contains('framed'), bar: m.querySelector('.bm-titlebar')?.offsetHeight,"
                "  segs: [...m.querySelectorAll('.seg .seg-item')].map(s => s.textContent.trim()) }; })()")
            check("M2a 弹窗开：标题「信息查看管理」/ framed / 宽 460 / 标题栏 54",
                  isinstance(m2, dict) and m2.get("title") == "信息查看管理" and m2.get("w") == 460
                  and m2.get("framed") is True and m2.get("bar") == 54,
                  json.dumps(m2, ensure_ascii=False))
            check("M2b 分段三段 = 基础信息/链接关系/提示词（与详情三 tab 对应）",
                  isinstance(m2, dict) and m2.get("segs") == ["基础信息", "链接关系", "提示词"],
                  json.dumps(m2, ensure_ascii=False))

            # ---------- M3 默认 basic：4 行序 + 首末行箭头禁用 ----------
            rows = await dlg_rows()
            check("M3a basic 4 行序 = 名称与链接/标签/详情/基本数据",
                  isinstance(rows, list) and [r.get("t") for r in rows] == ["名称与链接", "标签", "详情", "基本数据"],
                  json.dumps(rows, ensure_ascii=False))
            check("M3b 首行禁上 / 末行禁下",
                  isinstance(rows, list) and len(rows) == 4
                  and rows[0].get("up") is True and rows[0].get("down") is False
                  and rows[-1].get("down") is True and rows[-1].get("up") is False,
                  json.dumps(rows, ensure_ascii=False))

            # ---------- M4 切链接关系段 ----------
            await ev("[...document.querySelectorAll('.info-manage .seg .seg-item')].find(s => s.textContent.trim() === '链接关系').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            rows = await dlg_rows()
            check("M4 links 5 行序 = 链接/音频/关系/缩略图/污染文件",
                  isinstance(rows, list) and [r.get("t") for r in rows] == ["链接", "音频", "关系", "缩略图", "污染文件"],
                  json.dumps(rows, ensure_ascii=False))

            # ---------- M5 音频行上移（即时生效 + 持久化） ----------
            await ev("document.querySelectorAll('.info-manage .im-row')[1].querySelector('.sort-arrow.arrow-up').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)  # 防抖 500ms 写回
            rows = await dlg_rows()
            saved = await ev("JSON.parse(JSON.stringify((window.__saved || []).map(s => s.detailBlocks && s.detailBlocks.links && s.detailBlocks.links.order).filter(Boolean).pop() || null))")
            check("M5a 音频上移 → 行序 = 音频/链接/关系/缩略图/污染文件",
                  isinstance(rows, list) and [r.get("t") for r in rows] == ["音频", "链接", "关系", "缩略图", "污染文件"],
                  json.dumps(rows, ensure_ascii=False))
            check("M5b save_ui_settings 记录 detailBlocks.links.order 音频首",
                  isinstance(saved, list) and saved[:2] == ["audio", "links"], json.dumps(saved))

            # ---------- M6 关系行眼睛显隐 ----------
            eye0 = (await dlg_rows())[2].get("eye")
            await ev("document.querySelectorAll('.info-manage .im-row')[2].querySelector('.ic-btn:last-child').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)
            rows = await dlg_rows()
            saved_h = await ev("JSON.parse(JSON.stringify((window.__saved || []).map(s => s.detailBlocks && s.detailBlocks.links && s.detailBlocks.links.hidden).filter(Boolean).pop() || null))")
            check("M6a 关系行隐藏：标题 off 禁用色 + 眼睛 svg 切换（view-off）",
                  isinstance(rows, list) and rows[2].get("off") is True and rows[2].get("eye") != eye0,
                  json.dumps(rows, ensure_ascii=False))
            check("M6b hidden 含 relations", isinstance(saved_h, list) and "relations" in saved_h, json.dumps(saved_h))

            # ---------- M7 关弹窗 → 详情 links tab：排序+显隐双生效 ----------
            await click_sel(".info-manage .bm-titlebar .ic-btn")
            await ev("[...document.querySelectorAll('.sidebar-right .detail-root .seg .seg-item')][1].dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            blocks = await ev("[...document.querySelectorAll('.links-tab .lb-block .st .txt')].map(e => e.textContent)")
            check("M7 详情 links 区块序 = 音频/链接/缩略图/污染文件（关系 hidden 整块不渲染）",
                  isinstance(blocks, list) and blocks == ["音频", "链接", "缩略图", "污染文件"],
                  json.dumps(blocks, ensure_ascii=False))

            # ---------- M8 重开弹窗（详情 links tab → 落 links 段）行序持久 ----------
            await click_sel(".sidebar-right .manage-bar .act-btn")
            rows = await dlg_rows()
            check("M8 重开落 links 段：行序持久（音频首 / 关系行 off）",
                  isinstance(rows, list) and [r.get("t") for r in rows] == ["音频", "链接", "关系", "缩略图", "污染文件"]
                  and rows[2].get("off") is True,
                  json.dumps(rows, ensure_ascii=False))

            # 截图（弹窗 links 态含 off 行）
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            os.makedirs(os.path.dirname(SHOT), exist_ok=True)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print(f"  [SHOT] {SHOT}")

            # ---------- M9 提示词段：单区块仅显隐 ----------
            await ev("[...document.querySelectorAll('.info-manage .seg .seg-item')].find(s => s.textContent.trim() === '提示词').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            rows = await dlg_rows()
            check("M9 提示词 1 行（标题=提示词）上下箭头全禁",
                  isinstance(rows, list) and len(rows) == 1 and rows[0].get("t") == "提示词"
                  and rows[0].get("up") is True and rows[0].get("down") is True,
                  json.dumps(rows, ensure_ascii=False))

            # ---------- M10 视图菜单「信息查看管理」点亮 ----------
            await click_sel(".info-manage .bm-titlebar .ic-btn")
            await ev("[...document.querySelectorAll('.menu-bar .act-btn')].find((b) => b.textContent.trim() === '视图').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.5)
            mi = await ev(
                "(() => { const it = [...document.querySelectorAll('.app-menu *')].find(e => e.textContent.trim() === '信息查看管理');"
                " if (!it) return null; return { cls: it.className, dis: it.closest('[class*=disabled]') !== null || /disabled/.test(it.className) }; })()")
            check("M10a 视图菜单「信息查看管理」项存在且不 disabled",
                  isinstance(mi, dict) and mi.get("dis") is False, json.dumps(mi, ensure_ascii=False))
            await ev("[...document.querySelectorAll('.app-menu *')].find(e => e.textContent.trim() === '信息查看管理').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.5)
            check("M10b 菜单项点击 → 弹窗开（第二入口）",
                  await ev("!!document.querySelector('.info-manage .bm-modal')") is True)

            # ---------- M11 basic 标签上移 + 详情行隐藏 → 详情 basic 配置驱动 ----------
            await ev("[...document.querySelectorAll('.info-manage .seg .seg-item')].find(s => s.textContent.trim() === '基础信息').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            # 标签（idx1）上移 → [标签, 名称与链接, 详情, 基本数据]；再点详情行（移动后 idx2）眼睛隐藏
            await ev("document.querySelectorAll('.info-manage .im-row')[1].querySelector('.sort-arrow.arrow-up').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.3)
            await ev("document.querySelectorAll('.info-manage .im-row')[2].querySelector('.ic-btn:last-child').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.5)
            await click_sel(".info-manage .bm-titlebar .ic-btn")
            await ev("[...document.querySelectorAll('.sidebar-right .detail-root .seg .seg-item')][0].dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.6)
            cls = await ev("[...document.querySelectorAll('.sidebar-right .tab-body .db-block')].map(e => e.className.replace('db-block ', ''))")
            check("M11 详情 basic 区块类序 = tags/inputs/data（详情 hidden 不渲染）",
                  isinstance(cls, list) and cls == ["tags", "inputs", "data"], json.dumps(cls, ensure_ascii=False))

            print()
            if fails:
                print(f"FAIL {len(fails)}：")
                for f in fails:
                    print("  -", f)
                return 1
            print("ALL PASS")
            return 0
    finally:
        try:
            edge.terminate()
        except Exception:
            pass
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(edge.pid)], capture_output=True)
        kill_listening_port(PORT)
        if spawned is not None:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(spawned.pid)], capture_output=True)


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
