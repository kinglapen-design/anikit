# ws 链接门探针（2026-10-02 junction 方向 b：外部文件夹挂进 WS 子目录 前端轮）：
# 桩 workspace_root / ws_list_pinned（挂链门入常用）/ list_workspace_dir——
# 根 = 普通夹（普通目录）+ 挂链门（isLink+linkOk，含子夹）+ 断链门（isLink+!linkOk+hasSubDir false）+ 一.png，
# 验收：
#   A 树行徽章 —— 挂链门 = .fico-badge（AppIcon linked-file-badge）；断链门 = .fico-badge
#      （TwoToneIcon warning-small）+ 整行 dimmed；普通夹 = 无徽章无 dimmed；pin 行（常用）同挂链；
#   B 文件夹卡徽章 —— 挂链门 = .fd-badge（AppIcon）；断链门 = .fd-badge（TwoToneIcon）+ 整卡 dimmed；
#   C 菜单 —— 树行右键挂链门/断链门：含 添加链接文件夹…/重新指向…/移除链接；普通夹：含 添加链接文件夹…、
#      无重新指向…/移除链接（条件出现 ≠ disabled）；文件夹卡右键挂链门同；空白右键：含 添加链接文件夹…；
#   D 进入 —— 树行点挂链门 = 正常列出子夹（无徽章）；树行点断链门 = 空列表降级不加提示（后端口径）。
# 徽章类型判别：linked-file-badge = AppIcon（.app-icon，currentColor 单灰）/
# warning-small = TwoToneIcon（.tt-icon，双色调变量通道）。右键 buttons=2（release 恒 0）。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9372
URL = "http://localhost:5373"
SHOT = "tools/ws-link-badges.png"


def _dir(n: str, link: str = "") -> str:
    # link: "" = 普通目录 / "ok" = 挂链门 / "broken" = 断链门
    extra = {
        "": "isLink: false, linkOk: false, hasSubDir: false",
        "ok": "isLink: true, linkOk: true, hasSubDir: true",
        "broken": "isLink: true, linkOk: false, hasSubDir: false",
    }[link]
    return (
        f'{{ name: "{n}", relPath: "{n}", isDir: true, {extra}, size: 0,'
        ' mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: null, h: null, ext: "", thumbPath: null, thumbKind: null }'
    )


STUB = make_stub(
    {
        "get_current_library": '{ name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 0 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: false }',
        "workspace_root": '{ rootPath: "E:/ws-probe", exists: true }',
        "ws_list_pinned": '["挂链门"]',
    },
    handlers=f"""
    if (cmd === "ws_remove_link") {{ window.__calls.ws_remove_link.push(args?.relPath); return Promise.resolve(null); }}
    if (cmd === "list_workspace_dir") {{
      const rel = args?.relPath ?? "";
      if (rel === "") return Promise.resolve([{_dir("普通夹")}, {_dir("挂链门", "ok")}, {_dir("断链门", "broken")},
        {{ name: "一.png", relPath: "一.png", isDir: false, isLink: false, linkOk: false, hasSubDir: false, size: 2048,
           mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: 100, h: 100, ext: "png", thumbPath: null, thumbKind: null }}]);
      if (rel === "挂链门") return Promise.resolve([{{ name: "子夹", relPath: "挂链门/子夹", isDir: true,
        isLink: false, linkOk: false, hasSubDir: false, size: 0, mtimeMs: 1758000000000, ctimeMs: 1757000000000,
        w: null, h: null, ext: "", thumbPath: null, thumbKind: null }}]);
      return Promise.resolve([]);
    }}
""",
    head="window.__calls = { ws_remove_link: [] };",
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


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


# 树行快照（.ws-tree 全部行；sec = 区块序：0 快速入口 / 1 常用 / 2 文件夹——pin 行与树行同名的判别）
TREE_DUMP = """(() => [...document.querySelectorAll('.ws-tree .tree-item')].map((t) => ({
  n: t.querySelector('.nm')?.textContent ?? '',
  sec: [...document.querySelectorAll('.ws-tree > .ws-section')].indexOf(t.closest('.ws-section')),
  badge: t.querySelector('.fico-badge .app-icon') ? 'app' : (t.querySelector('.fico-badge .tt-icon') ? 'tt' : ''),
  dim: t.classList.contains('dimmed'),
})))()"""

# 文件夹卡快照（视口渲染序）
CARD_DUMP = """(() => [...document.querySelectorAll('.folder-card')].map((c) => ({
  n: c.querySelector('.fd-name')?.textContent ?? '',
  badge: c.querySelector('.fd-badge .app-icon') ? 'app' : (c.querySelector('.fd-badge .tt-icon') ? 'tt' : ''),
  dim: c.classList.contains('dimmed'),
})))()"""

TREE_BY_NAME = """((n) => [...document.querySelectorAll('.ws-tree .tree-item')].find(
  (t) => t.querySelector('.nm')?.textContent === n && t.closest('.ws-section') === [...document.querySelectorAll('.ws-tree > .ws-section')].pop()))(%s)"""

CARD_BY_NAME = """((n) => [...document.querySelectorAll('.folder-card')].find(
  (c) => c.querySelector('.fd-name')?.textContent === n))(%s)"""

MENU_LABELS = """(() => [...document.querySelectorAll('.app-menu .menu-item .label')].map((x) => x.textContent?.trim()))()"""

ESC = "window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))"


async def main() -> int:
    dev_proc = None
    if not dev_server_up():
        print("启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
        if not dev_server_up():
            print("dev server 启动超时")
            return 2
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
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

            async def click(expr, mod=0, button="left", at=None):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min({at[0] if at else 'b.left + b.width / 2'}, window.innerWidth - 8), y: Math.min({at[1] if at else 'b.top + b.height / 2'}, window.innerHeight - 8) }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(
                        ws,
                        "Input.dispatchMouseEvent",
                        {
                            "type": t,
                            "button": button,
                            "modifiers": mod,
                            "buttons": 0 if t == "mouseReleased" else (2 if button == "right" else 1),
                            "x": r["x"],
                            "y": r["y"],
                            "clickCount": 1,
                        },
                        session=sess,
                    )
                await asyncio.sleep(0.6)
                return True

            fails = []

            def check(name, cond, got=None):
                print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"（got={got!r}）"))
                if not cond:
                    fails.append(name)

            def row(dump, n, sec):
                return next((d for d in (dump or []) if d["n"] == n and d["sec"] == sec), None)

            # 切工作空间分段（资源库 / 工作空间 / AI 三段之第二段）
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][1]")
            await asyncio.sleep(1.0)
            for _ in range(20):
                n = await ev("document.querySelectorAll('.folder-card').length")
                if (n or 0) >= 3:
                    break
                await asyncio.sleep(0.5)

            tree = await ev(TREE_DUMP)
            cards = await ev(CARD_DUMP)
            print("树行:", json.dumps(tree, ensure_ascii=False))
            print("文件夹卡:", json.dumps(cards, ensure_ascii=False))

            # —— A 树行徽章（sec 2 = 文件夹区块；sec 1 = 常用 pin 行）——
            check("A1 树行·挂链门 = 挂链徽章（AppIcon）", (row(tree, "挂链门", 2) or {}).get("badge") == "app", row(tree, "挂链门", 2))
            r = row(tree, "断链门", 2)
            check("A2 树行·断链门 = 断链徽章（TwoToneIcon）+ dimmed", r is not None and r["badge"] == "tt" and r["dim"], r)
            r = row(tree, "普通夹", 2)
            check("A3 树行·普通夹 = 无徽章无 dimmed", r is not None and r["badge"] == "" and not r["dim"], r)
            check("A4 pin 行·挂链门 = 挂链徽章", (row(tree, "挂链门", 1) or {}).get("badge") == "app", row(tree, "挂链门", 1))

            # —— B 文件夹卡徽章 ——
            def card(n):
                return next((c for c in (cards or []) if c["n"] == n), None)

            check("B1 文件夹卡·挂链门 = 挂链徽章（AppIcon）", (card("挂链门") or {}).get("badge") == "app", card("挂链门"))
            c = card("断链门")
            check("B2 文件夹卡·断链门 = 断链徽章（TwoToneIcon）+ dimmed", c is not None and c["badge"] == "tt" and c["dim"], c)
            c = card("普通夹")
            check("B3 文件夹卡·普通夹 = 无徽章无 dimmed", c is not None and c["badge"] == "" and not c["dim"], c)

            # 徽章截图（侧栏树 + 根视口卡片）——目验走查用
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图:", SHOT)

            # —— C 菜单 ——
            await click(TREE_BY_NAME % '"挂链门"', button="right")
            labels = await ev(MENU_LABELS)
            check(
                "C1 树行右键·挂链门 = 添加链接文件夹…/重新指向…/移除链接 全列",
                all(x in (labels or []) for x in ("添加链接文件夹…", "重新指向…", "移除链接")),
                labels,
            )
            await ev(ESC)
            await asyncio.sleep(0.4)
            await click(TREE_BY_NAME % '"断链门"', button="right")
            labels = await ev(MENU_LABELS)
            check("C2 树行右键·断链门 = 重新指向…/移除链接 同列（断链可送修可移除）", all(x in (labels or []) for x in ("重新指向…", "移除链接")), labels)
            await ev(ESC)
            await asyncio.sleep(0.4)
            await click(TREE_BY_NAME % '"普通夹"', button="right")
            labels = await ev(MENU_LABELS)
            check(
                "C3 树行右键·普通夹 = 有添加链接文件夹…、无重新指向…/移除链接（条件出现）",
                "添加链接文件夹…" in (labels or []) and "重新指向…" not in (labels or []) and "移除链接" not in (labels or []),
                labels,
            )
            await ev(ESC)
            await asyncio.sleep(0.4)
            await click(CARD_BY_NAME % '"挂链门"', button="right")
            labels = await ev(MENU_LABELS)
            check("C4 文件夹卡右键·挂链门 = 三项同树行", all(x in (labels or []) for x in ("添加链接文件夹…", "重新指向…", "移除链接")), labels)
            await ev(ESC)
            await asyncio.sleep(0.4)
            # 空白右键（grid-viewport 右下角空白）
            await click("document.querySelector('.grid-viewport')", button="right", at=("b.right - 24", "b.bottom - 24"))
            labels = await ev(MENU_LABELS)
            check("C5 空白右键 = 添加链接文件夹…", "添加链接文件夹…" in (labels or []), labels)
            await ev(ESC)
            await asyncio.sleep(0.4)

            # —— E 移除链接确认弹窗（文案要点 + 执行链：确认 → ws_remove_link 按 relPath）——
            await click(TREE_BY_NAME % '"挂链门"', button="right")
            await click("[...document.querySelectorAll('.app-menu .menu-item')].find((x) => x.textContent?.includes('移除链接'))")
            modal = await ev(
                "(() => { const m = document.querySelector('.kit-modal'); return m ? {"
                " title: m.querySelector('.title')?.textContent ?? '',"
                " body: m.querySelector('.body')?.textContent ?? '',"
                " confirm: m.querySelector('.btn.confirm')?.textContent ?? '' } : null; })()"
            )
            check(
                "E1 移除链接确认弹窗 = 标题「移除链接」+ 文案「只删门·目标不动·回收站可找回」+ 主钮",
                modal is not None
                and modal["title"] == "移除链接"
                and "目标文件夹内容不受影响" in modal["body"]
                and "系统回收站" in modal["body"]
                and modal["confirm"] == "移除链接",
                modal,
            )
            await click("document.querySelector('.kit-modal .btn.confirm')")
            removed = await ev("window.__calls.ws_remove_link")
            check("E2 确认后 ws_remove_link 按 relPath 调用", removed == ["挂链门"], removed)

            # —— D 进入 ——
            await click(TREE_BY_NAME % '"挂链门"')
            cards = await ev(CARD_DUMP)
            check("D1 进入挂链门 = 列出子夹（普通目录无徽章）", len(cards or []) == 1 and cards[0]["n"] == "子夹" and cards[0]["badge"] == "", cards)
            await click(TREE_BY_NAME % '"断链门"')
            cards = await ev(CARD_DUMP)
            alive = await ev("document.querySelector('#app') !== null && !document.querySelector('vite-error-overlay')")
            check("D2 进入断链门 = 空列表降级（0 卡 + 页面无报错）", (cards or []) == [] and alive is True, cards)

            print("FAIL 清单:", fails if fails else "无")
            return 1 if fails else 0
    finally:
        proc.terminate()
        if dev_proc is not None:
            dev_proc.terminate()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
