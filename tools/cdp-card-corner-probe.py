# CDP 探针：文件卡片圆角平滑度专项 r13（2026-08-30）
# 用户实机反馈卡片四角不够平滑。r12（headless 软渲染）已截多 DSF 基线；
# 本轮 = 非 headless 真窗口 GPU 合成路径（用户实机渲染条件，踩坑 036 同款方法论），
# DSF 1.25 / 1.5（Windows 缩放档），对「文件卡片-通用」三态 × 4 实验变体截图对照：
#   base          现状（.kc-media overflow:hidden 双层裁剪 + ::after 1px / ::before 2px border）
#   no-outer-clip 去 .kc-media overflow:hidden（双层圆角裁剪 → 单层，背景走纯圆角填充路径）
#   sel-single    base + 选中态隐藏常态环 ::after（排除双层环角部错位）
#   shadow-ring   常态环 ::after 改 inset box-shadow 1px（对比绘制路径；§7 1.5px 渗线仅证 1.5 档）
# 用法：tools/.venv/Scripts/python.exe tools/cdp-card-corner-probe.py
#   会在屏幕上开窗跑约 20 秒；输出 tools/style-preview/r13-corner-*.png
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9391
URL = "http://127.0.0.1:5373/#design-system-cards"
PROFILE = os.path.join(os.environ["TEMP"], "edge-cdp-corner-r13")
DSFS = [1.25, 1.5]

# 通用组内三态（预览页前三张：常态 / force-hover / selected）
CARDS = [
    ("normal", ".fc-root:not(.force-hover):not(.selected)"),
    ("hover", ".fc-root.force-hover"),
    ("selected", ".fc-root.selected"),
]

VARIANTS = [
    ("base", ""),
    ("no-outer-clip", ".kc-media { overflow: visible !important; }"),
    ("sel-single", ".fc-root.selected::after { display: none !important; }"),
    (
        "shadow-ring",
        ".kc-bordered::after { border: none !important; box-shadow: inset 0 0 0 1px var(--n-g3) !important; }"
        ".kc-media:hover::after, .kc-media.force-hover::after { box-shadow: inset 0 0 0 1px var(--n-g6) !important; }",
    ),
]

FIND_CARD = (
    "(() => {"
    " const g = [...document.querySelectorAll('#design-system-cards .pv-group')]"
    "   .find(h => h.querySelector('.pv-group-title').textContent.startsWith('文件卡片-通用'));"
    " if (!g) return null;"
    " const c = g.querySelector(%s);"
    " if (!c) return null;"
    " c.scrollIntoView({ block: 'center' });"
    " const r = c.getBoundingClientRect();"
    " return { x: Math.max(0, r.x - 8), y: Math.max(0, r.y - 8),"
    " width: r.width + 16, height: r.height + 16 };"
    " })()"
)

SET_CSS = (
    "(() => { let st = document.getElementById('corner-exp');"
    " if (!st) { st = document.createElement('style'); st.id = 'corner-exp'; document.head.appendChild(st); }"
    " st.textContent = %s; return true; })()"
)


async def main() -> int:
    proc = subprocess.Popen(
        [
            EDGE,
            f"--remote-debugging-port={PORT}",
            f"--user-data-dir={PROFILE}",
            "--window-size=1600,1000",
            "--no-first-run",
            "about:blank",
        ],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    try:
        page = None
        for _ in range(60):
            try:
                tabs = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json"))
                page = next((t for t in tabs if t["type"] == "page"), None)
                if page:
                    break
            except Exception:
                pass
            time.sleep(0.25)
        if not page:
            print("FATAL: 拿不到 CDP 页签")
            return 1

        async with websockets.connect(page["webSocketDebuggerUrl"], max_size=None) as ws:
            mid = 0

            async def send(method: str, params: dict | None = None) -> dict:
                nonlocal mid
                mid += 1
                await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
                while True:
                    msg = json.loads(await ws.recv())
                    if msg.get("id") == mid:
                        return msg.get("result", {})

            async def js(expr: str):
                r = await send("Runtime.evaluate", {"expression": expr, "returnByValue": True})
                return r.get("result", {}).get("value")

            await send("Page.enable")
            await send("Runtime.enable")

            first = True
            for dsf in DSFS:
                await send("Emulation.setDeviceMetricsOverride", {
                    "width": 1600, "height": 1000, "deviceScaleFactor": dsf, "mobile": False,
                })
                if first:
                    await send("Page.navigate", {"url": URL})
                    for _ in range(40):
                        if await js("!!document.querySelector('#design-system-cards .fc-root')"):
                            break
                        await asyncio.sleep(0.25)
                    else:
                        print("FATAL: 预览页卡片区未渲染")
                        return 1
                    await asyncio.sleep(1.0)
                    first = False
                else:
                    await asyncio.sleep(0.5)

                tag = str(dsf).replace(".", "p")
                for variant, css in VARIANTS:
                    await js(SET_CSS % json.dumps(css))
                    await asyncio.sleep(0.2)
                    for state, sel in CARDS:
                        rect = await js(FIND_CARD % json.dumps(sel))
                        if not rect:
                            print("MISS", tag, variant, state)
                            continue
                        await asyncio.sleep(0.15)
                        s = await send("Page.captureScreenshot", {
                            "format": "png",
                            "clip": {**rect, "scale": 1},
                        })
                        out = f"tools/style-preview/r13-corner-{variant}-{state}-{tag}x.png"
                        with open(out, "wb") as f:
                            f.write(base64.b64decode(s["data"]))
                        print("saved", out)
                await js(SET_CSS % json.dumps(""))
            return 0
    finally:
        subprocess.run(["taskkill", "/F", "/T", "/PID", str(proc.pid)], capture_output=True)


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
