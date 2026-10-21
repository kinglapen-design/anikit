# CDP 探针：工作空间树缩进线对齐库树（2026-10-21）——.tree-guide 共享类（base.css）落地断言
#   W1 ws 文件夹区块：展开 甲夹（depth0）/ 甲一（depth1）→ 各自 .ws-children.tree-guide 存在；
#      ::before 横坐标 = depth×20+15（15px / 35px）、宽 1px、色 rgba(102,102,102,0.5)
#      （--n-custom-2-50 缺省档，ws 无自定义色）、高度 = 子层容器高（top0 bottom0 贯穿）
#   W2 常用 pin 行：展开 固定夹 → 子层线 15px（pin 行 depth 0）
#   W3 库树回归：共享类化后 .ft-children.tree-guide 线位/色不变（15px / rgba(102,102,102,0.5)）；
#      自定义色蓝节点 = rgba(37,80,118,0.5)（--guide-color color-mix 档仍生效）
#   截图 tools/style-preview/ws-tree-guide.png（ws 段）/ ws-tree-guide-lib.png（库段）侧栏局部
# 桩：库树 f1 角色（带子层）+ f2 场景 蓝色（带子层）；ws = 甲夹/甲一/甲一内 三级 + 乙夹（无子）+ pin 固定夹/固定子。
# 用法：先起 pnpm dev（5373；未监听会自起并在结束时杀掉），再
#   tools/.venv/Scripts/python.exe tools/cdp-ws-tree-guide-probe.py
import asyncio
import base64
import json
import os
import re
import subprocess
import sys
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 18770 + os.getpid() % 500
URL = "http://localhost:5373"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-wsguide-{os.getpid()}")
SHOT_WS = "tools/style-preview/ws-tree-guide.png"
SHOT_LIB = "tools/style-preview/ws-tree-guide-lib.png"

from _probe_stub import make_stub


def _dir(n: str, rel: str, sub: bool) -> str:
    return (
        f'{{ name: "{n}", relPath: "{rel}", isDir: true, isLink: false, linkOk: false, hasSubDir: {str(sub).lower()},'
        ' size: 0, mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: null, h: null, ext: "", thumbPath: null, thumbKind: null }'
    )


STUB = make_stub(
    {
        "get_current_library": '{ name: "测试库", path: "E:/test-lib", assetCount: 0, folderCount: 2 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: false }',
        "workspace_root": '{ rootPath: "E:/ws-probe", exists: true }',
        "ws_list_pinned": '["固定夹"]',
        "list_folder_tree": """{ blocks: [{ id: "dir", name: "目录", icon: null }],
        nodes: [
          { id: "f1", name: "角色", count: 0, children: [{ id: "f1a", name: "主角", count: 0, children: [] }] },
          { id: "f2", name: "场景", count: 0, children: [{ id: "f2a", name: "内景", count: 0, children: [] }], color: "blue" },
        ] }""",
    },
    handlers=f"""
    if (cmd === "list_workspace_dir") {{
      const rel = args?.relPath ?? "";
      if (rel === "") return Promise.resolve([{_dir("甲夹", "甲夹", True)}, {_dir("乙夹", "乙夹", False)}]);
      if (rel === "甲夹") return Promise.resolve([{_dir("甲一", "甲夹/甲一", True)}]);
      if (rel === "甲夹/甲一") return Promise.resolve([{_dir("甲一内", "甲夹/甲一/甲一内", False)}]);
      if (rel === "固定夹") return Promise.resolve([{_dir("固定子", "固定夹/固定子", False)}]);
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


def row_expr(scope: str, name: str) -> str:
    return (
        f"(() => [...document.querySelectorAll('{scope} .tree-item')].find("
        f"(t) => t.querySelector('.nm')?.textContent === '{name}'))()"
    )


def exp_expr(scope: str, name: str) -> str:
    return f"(() => {{ const t = {row_expr(scope, name)}; return t ? t.querySelector('.exp') : null; }})()"


def guide_expr(scope: str, node_cls: str, name: str) -> str:
    kids = ".ft-children" if scope == ".block-tree" else ".ws-children"
    return (
        f"(() => {{ const t = {row_expr(scope, name)}; const node = t ? t.closest('{node_cls}') : null;"
        f" const ch = node ? node.querySelector(':scope > {kids}') : null; if (!ch) return null;"
        " const cs = getComputedStyle(ch, '::before');"
        " return { cls: ch.className, left: cs.left, width: cs.width, bg: cs.backgroundColor,"
        " content: cs.content, h: parseFloat(cs.height), chH: ch.offsetHeight }; })()"
    )


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
        [
            EDGE,
            f"--remote-debugging-port={PORT}",
            f"--user-data-dir={PROFILE}",
            "--headless=new",
            "--disable-gpu",
            "--window-size=1600,1200",
            "about:blank",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    fails = []

    def check(name, cond, got=None):
        print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"（got={got!r}）"))
        if not cond:
            fails.append(name)

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
                if "exceptionDetails" in r.get("result", {}):
                    print("!! JS 异常:", str(r["result"]["exceptionDetails"])[:200])
                return r.get("result", {}).get("result", {}).get("value")

            async def click(expr):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(
                        ws,
                        "Input.dispatchMouseEvent",
                        {"type": t, "button": "left", "modifiers": 0, "buttons": 0 if t == "mouseReleased" else 1, "x": r["x"], "y": r["y"], "clickCount": 1},
                        session=sess,
                    )
                await asyncio.sleep(0.5)
                return True

            async def shot(path):
                clip = await ev("(() => { const b = document.querySelector('.sidebar-left').getBoundingClientRect(); return { x: b.left, y: b.top, width: b.width, height: b.height, scale: 1 }; })()")
                r = await send(ws, "Page.captureScreenshot", {"format": "png", "clip": clip}, session=sess)
                with open(path, "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图", path)

            def norm_color(s):
                # Chrome 对 color-mix 结果序列化为 color(srgb r g b / a)，归一回 rgba() 比对
                s = (s or "").strip()
                m = re.fullmatch(r"color\(srgb\s+([\d.]+)\s+([\d.]+)\s+([\d.]+)\s*/\s*([\d.]+)\)", s)
                if m:
                    r, g, b, a = (float(m.group(i)) for i in range(1, 5))
                    return f"rgba({round(r * 255)},{round(g * 255)},{round(b * 255)},{a:g})"
                return s.replace(" ", "")

            def expect_guide(name, got, left, bg="rgba(102, 102, 102, 0.5)"):
                ok = (
                    got is not None
                    and "tree-guide" in got["cls"]
                    and got["left"] == f"{left}px"
                    and got["width"] == "1px"
                    and norm_color(got["bg"]) == bg.replace(" ", "")
                    and got["content"] != "none"
                    and abs(got["h"] - got["chH"]) < 1
                )
                check(name, ok, got)

            # —— W1/W2：ws 段（分段控制器第二段 = 工作空间） ——
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][1]")
            await asyncio.sleep(1.0)
            for _ in range(20):
                if await ev("document.querySelectorAll('.ws-tree .tree-item').length"):
                    break
                await asyncio.sleep(0.5)
            await click(exp_expr(".ws-tree", "甲夹"))
            await asyncio.sleep(0.6)
            await click(exp_expr(".ws-tree", "甲一"))
            await asyncio.sleep(0.6)
            await click(exp_expr(".ws-tree", "固定夹"))
            await asyncio.sleep(0.6)

            n = await ev("document.querySelectorAll('.ws-tree .ws-children.tree-guide').length")
            check("W0 ws 子层 tree-guide 容器数 = 3（甲夹/甲一/固定夹）", n == 3, n)
            expect_guide("W1a 甲夹（depth0）子层线 15px", await ev(guide_expr(".ws-tree", ".ws-node", "甲夹")), 15)
            expect_guide("W1b 甲一（depth1）子层线 35px", await ev(guide_expr(".ws-tree", ".ws-node", "甲一")), 35)
            expect_guide("W2 固定夹 pin（depth0）子层线 15px", await ev(guide_expr(".ws-tree", ".ws-node", "固定夹")), 15)
            await shot(SHOT_WS)

            # —— W3：库段回归（分段控制器第一段 = 资源库） ——
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][0]")
            await asyncio.sleep(1.0)
            await click(exp_expr(".block-tree", "角色"))
            await asyncio.sleep(0.6)
            await click(exp_expr(".block-tree", "场景"))
            await asyncio.sleep(0.6)
            expect_guide("W3a 库树 角色（depth0）线 15px 缺省档", await ev(guide_expr(".block-tree", ".ft-node", "角色")), 15)
            expect_guide("W3b 库树 场景 蓝线 color-mix 档", await ev(guide_expr(".block-tree", ".ft-node", "场景")), 15, "rgba(37, 80, 118, 0.5)")
            await shot(SHOT_LIB)
    finally:
        try:
            proc.terminate()
        except Exception:
            pass
        subprocess.run(f"taskkill /F /T /PID {proc.pid}", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(f'rd /s /q "{PROFILE}"', shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if dev_proc:
            subprocess.run(f"taskkill /F /T /PID {dev_proc.pid}", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print("FAILS:", fails)
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
