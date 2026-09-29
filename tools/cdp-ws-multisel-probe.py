# ws 多选探针（2026-09-26_18 工作空间树多选与文件夹Shift补齐轮）：
# 桩 workspace_root / list_workspace_dir（根 = 甲乙丙丁四目录 + 一二两文件，子目录空），
# 切工作空间分段后验收：
#   缺口 1 —— 文件夹卡片 Shift 范围选（实证 _17 接线生效）：点甲 → Shift 点丙 = 甲乙丙整段
#             （条带序）；Ctrl 点甲减选；Shift 点丁 = 锚点（甲）到丁整段；
#             边界记录：锚点为文件键时 Shift 点文件夹 = 退化单选（素材范围选同规则），
#             退化后锚点自愈、下一次 Shift 即成段；
#   缺口 2 —— 树行 Ctrl/Shift（本轮落地）：Ctrl 点树行 = 加选/减选不导航，同域联动
#             （树行灰底 domain-selected ↔ 网格卡 .selected 蓝环同键）；Shift 点树行 =
#             同级范围选；右键已选树行 = 复制/剪切/删除 N 项批量分流；plain 点树行 =
#             导航 + 单选入域（回归既有行为）。
# 修饰键经 CDP Input.dispatchMouseEvent modifiers 位掩码（Alt=1 Ctrl=2 Meta=4 Shift=8）真实派发；
# 右键 buttons=2（release 恒 0——cdp-context-menu-probe 2026-09-05 实证口径）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9371
URL = "http://localhost:5373"

_dir = (
    lambda n: f'{{ name: "{n}", relPath: "{n}", isDir: true, hasSubDir: false, size: 0,'
    ' mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: null, h: null, ext: "", thumbPath: null, thumbKind: null }'
)
_file = (
    lambda n: f'{{ name: "{n}.png", relPath: "{n}.png", isDir: false, hasSubDir: false, size: 2048,'
    ' mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: 100, h: 100, ext: "png", thumbPath: null, thumbKind: null }'
)

STUB = make_stub(
    {
        "get_current_library": '{ name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 0 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: false }',
        "workspace_root": '{ rootPath: "E:/ws-probe", exists: true }',
        "ws_list_pinned": "[]",
    },
    handlers=f"""
    if (cmd === "list_workspace_dir") {{
      const rel = args?.relPath ?? "";
      if (rel === "") return Promise.resolve([{_dir("甲")}, {_dir("乙")}, {_dir("丙")}, {_dir("丁")}, {_file("一")}, {_file("二")}]);
      return Promise.resolve([]);
    }}
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


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


# 文件夹卡片选中快照（渲染序）：[{ n = 名称, sel = .selected 蓝环 }]
CARD_DUMP = """(() => [...document.querySelectorAll('.folder-card')].map((c) => ({
  n: c.querySelector('.fd-name')?.textContent ?? '',
  sel: c.classList.contains('selected'),
})))()"""

# 素材卡选中快照（渲染序；选中环在内层 FileCard .kc-sel.selected，asset-card 根不带类）
ASSET_DUMP = """(() => [...document.querySelectorAll('.asset-card')].map((c) => ({
  n: c.textContent ?? '',
  sel: c.querySelector('.selected') !== null,
})))()"""

# ws 树行快照（DOM 序 = 同级行序）：sel = 当前目录高亮 / dom = 选择域灰底
TREE_DUMP = """(() => [...document.querySelectorAll('.ws-tree .tree-item')].map((t) => ({
  n: t.querySelector('.nm')?.textContent ?? '',
  sel: t.classList.contains('selected'),
  dom: t.classList.contains('domain-selected'),
})))()"""

CARD_BY_NAME = """((n) => [...document.querySelectorAll('.folder-card')].find(
  (c) => c.querySelector('.fd-name')?.textContent === n))(%s)"""

ASSET_BY_NAME = """((n) => [...document.querySelectorAll('.asset-card')].find(
  (c) => c.textContent.includes(n)))(%s)"""

TREE_BY_NAME = """((n) => [...document.querySelectorAll('.ws-tree .tree-item')].find(
  (t) => t.querySelector('.nm')?.textContent === n))(%s)"""

# 菜单项标签清单（ContextMenu = .app-menu .menu-item .label，cdp-context-menu-probe 同径）
MENU_LABELS = """(() => [...document.querySelectorAll('.app-menu .menu-item .label')].map((x) => x.textContent?.trim()))()"""


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
        print("dev server 就绪")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
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

            async def click(expr, mod=0, button="left"):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {
                        "type": t, "button": button, "modifiers": mod,
                        "buttons": 0 if t == "mouseReleased" else (2 if button == "right" else 1),
                        "x": r["x"], "y": r["y"], "clickCount": 1,
                    }, session=sess)
                await asyncio.sleep(0.6)
                return True

            # 切工作空间分段（资源库 / 工作空间 / AI 三段之第二段）
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][1]")
            await asyncio.sleep(1.0)
            # 等 ws 根视口文件夹卡片出现
            for _ in range(20):
                n = await ev("document.querySelectorAll('.folder-card').length")
                if (n or 0) >= 4:
                    break
                await asyncio.sleep(0.5)
            print("ws 根视口文件夹卡片:", json.dumps(await ev(CARD_DUMP), ensure_ascii=False))
            print("ws 树行:", json.dumps(await ev(TREE_DUMP), ensure_ascii=False))

            fails = []

            def check(name, cond, got=None):
                print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"（got={got!r}）"))
                if not cond:
                    fails.append(name)

            def sel_names(dump):
                return [d["n"] for d in (dump or []) if d["sel"]]

            def dom_names(dump):
                return [d["n"] for d in (dump or []) if d["dom"]]

            # —— 缺口 1：文件夹卡片 Shift 范围选（实证 _17 接线）——
            await click(CARD_BY_NAME % '"甲"')
            d = await ev(CARD_DUMP)
            check("卡片·点甲 = 单选甲", sel_names(d) == ["甲"], sel_names(d))
            await click(CARD_BY_NAME % '"丙"', mod=8)  # Shift
            d = await ev(CARD_DUMP)
            check("卡片·Shift点丙 = 甲乙丙整段（条带序）", sel_names(d) == ["甲", "乙", "丙"], sel_names(d))
            await click(CARD_BY_NAME % '"甲"', mod=2)  # Ctrl 减选甲（anchor = 甲）
            d = await ev(CARD_DUMP)
            check("卡片·Ctrl点甲 = 减选（剩乙丙）", sel_names(d) == ["乙", "丙"], sel_names(d))
            await click(CARD_BY_NAME % '"丁"', mod=8)  # Shift：锚点甲 → 丁
            d = await ev(CARD_DUMP)
            check("卡片·Shift点丁 = 锚点甲到丁整段", sel_names(d) == ["甲", "乙", "丙", "丁"], sel_names(d))
            # 边界（记档用，非缺口）：锚点 = 文件键时 Shift 点文件夹 → 锚不在条带序退化单选；
            # 退化置 anchor = 该项，下一次 Shift 即成段
            await click(ASSET_BY_NAME % '"一.png"')
            a = await ev(ASSET_DUMP)
            check("卡片·点文件一 = 单选文件", sum(1 for x in (a or []) if x["sel"]) == 1, a)
            await click(CARD_BY_NAME % '"丙"', mod=8)
            d = await ev(CARD_DUMP)
            check("边界·文件锚Shift点文件夹丙 = 退化单选丙", sel_names(d) == ["丙"], sel_names(d))
            await click(CARD_BY_NAME % '"丁"', mod=8)
            d = await ev(CARD_DUMP)
            check("边界·锚自愈后再Shift点丁 = 丙丁成段", sel_names(d) == ["丙", "丁"], sel_names(d))

            # —— 缺口 2：树行 Ctrl / Shift（同域联动网格卡）——
            await click(CARD_BY_NAME % '"甲"')  # 复位：单选甲（anchor = 甲；树行甲同键亦灰底）
            t = await ev(TREE_DUMP)
            check("树·卡甲选中同域联动树行甲灰底", dom_names(t) == ["甲"], dom_names(t))
            await click(TREE_BY_NAME % '"丙"', mod=2)  # Ctrl 树行丙 = 加选，不导航
            t = await ev(TREE_DUMP)
            d = await ev(CARD_DUMP)
            check("树·Ctrl点丙 = 甲丙同域（树灰底）", dom_names(t) == ["甲", "丙"], dom_names(t))
            check("树·Ctrl点丙不导航（根视口四卡仍在）", len(d or []) >= 4, len(d or []))
            check("树·Ctrl点丙同域联动网格卡甲丙蓝环", sel_names(d) == ["甲", "丙"], sel_names(d))
            await click(TREE_BY_NAME % '"丙"', mod=2)  # Ctrl 再点丙 = 减选（anchor 仍丙）
            t = await ev(TREE_DUMP)
            check("树·Ctrl再点丙 = 减选（剩甲）", dom_names(t) == ["甲"], dom_names(t))
            await click(TREE_BY_NAME % '"乙"', mod=2)  # Ctrl 点乙（anchor = 乙）
            await click(TREE_BY_NAME % '"丁"', mod=8)  # Shift 树行丁：锚点乙 → 乙丙丁
            t = await ev(TREE_DUMP)
            d = await ev(CARD_DUMP)
            check("树·Shift点丁 = 乙丙丁同级范围（树灰底）", dom_names(t) == ["乙", "丙", "丁"], dom_names(t))
            check("树·Shift点丁同域：网格卡乙丙丁全选", sel_names(d) == ["乙", "丙", "丁"], sel_names(d))

            # —— 右键批量分流（已选树行右键 = 复制/剪切/删除 3 项）——
            await click(TREE_BY_NAME % '"丙"', button="right")
            labels = await ev(MENU_LABELS)
            print("树行丙右键菜单:", json.dumps(labels, ensure_ascii=False))
            check("树·右键已选丙 = 复制 3 项", "复制 3 项" in (labels or []), labels)
            check("树·右键已选丙 = 剪切 3 项", "剪切 3 项" in (labels or []), labels)
            check("树·右键已选丙 = 删除 3 项", "删除 3 项" in (labels or []), labels)
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await ev("window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(0.4)

            # —— plain 点树行 = 导航 + 单选入域（回归：导航行为不变）——
            await click(TREE_BY_NAME % '"乙"')
            await asyncio.sleep(0.8)
            t = await ev(TREE_DUMP)
            d = await ev(CARD_DUMP)
            check("树·plain点乙 = 导航（乙为当前目录高亮）", any((r["n"] == "乙" and r["sel"]) for r in (t or [])), t)
            check("树·plain点乙后视口 = 乙（空目录无文件夹卡）", len(d or []) == 0, len(d or []))
            check("树·plain点乙单选入域（乙为当前目录不占灰底，余行无灰底）", dom_names(t) == [], dom_names(t))

            print("=" * 30)
            print("RESULT:", "ALL PASS" if not fails else f"{len(fails)} FAIL: {fails}")
            return 0 if not fails else 1
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


sys.exit(asyncio.run(main()))
