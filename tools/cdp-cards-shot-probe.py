# CDP 截图留档：阶段七卡片组件族预览页目验（2026-08-29）
# 纯预览页（#design-system-cards），无后端桩。headless Edge 1x 全展区 + 2x 分组局部截图，
# 输出 tools/style-preview/r11-cards-*.png 供逐像素对照设计稿。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9389
URL = "http://127.0.0.1:5373/#design-system-cards"
PROFILE = os.path.join(os.environ["TEMP"], "edge-cdp-cards")

# 分组标题锚点 → 截图名
GROUPS = {
    "general": "文件卡片-通用",
    "video": "文件卡片-视频",
    "audio": "文件卡片-音频",
    "font": "文件卡片-字体",
    "text": "文件卡片-txt-md",
    "list": "列表卡片 + 列表表头",
    "folder": "文件夹卡片",
    "archive": "档案卡片",
    "badge": "格式角标",
}


async def main() -> int:
    proc = subprocess.Popen(
        [
            EDGE,
            "--headless=new",
            "--disable-gpu",
            f"--remote-debugging-port={PORT}",
            f"--user-data-dir={PROFILE}",
            "about:blank",
        ]
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

            async def save(name: str, data: str) -> None:
                out = f"tools/style-preview/{name}.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(data))
                print("saved", out)

            await send("Page.enable")
            await send("Runtime.enable")

            # ===== 1x 全展区整页（captureBeyondViewport 按页面坐标截） =====
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1600, "height": 1000, "deviceScaleFactor": 1, "mobile": False,
            })
            await send("Page.navigate", {"url": URL})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-cards .fc-root')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页卡片区未渲染")
                return 1
            await asyncio.sleep(1.0)  # 字体 / data-URI 图加载

            rect = await js(
                "(() => { const r = document.querySelector('#design-system-cards').getBoundingClientRect();"
                " return { x: Math.max(0, r.x), y: Math.max(0, r.y + scrollY),"
                " width: Math.min(r.width, 1600), height: r.height }; })()"
            )
            s = await send("Page.captureScreenshot", {
                "format": "png",
                "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 1},
                "captureBeyondViewport": True,
            })
            await save("r11-cards-1x-full", s["data"])

            # ===== 2x 各分组局部（滚动入视口后按视口坐标截，对照稿逐像素） =====
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1600, "height": 1000, "deviceScaleFactor": 2, "mobile": False,
            })
            await asyncio.sleep(0.5)
            for name, title in GROUPS.items():
                ok = await js(
                    "(() => {"
                    " const hs = [...document.querySelectorAll('#design-system-cards .pv-group')];"
                    f" const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " if (!g) return false;"
                    " g.scrollIntoView({ block: 'start' });"
                    " return true;"
                    " })()"
                )
                if not ok:
                    print("MISS", name, title)
                    continue
                await asyncio.sleep(0.3)
                rect = await js(
                    "(() => {"
                    " const hs = [...document.querySelectorAll('#design-system-cards .pv-group')];"
                    f" const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " const r = g.getBoundingClientRect();"
                    " return { x: Math.max(0, r.x), y: Math.max(0, r.y), width: Math.min(r.width, 1560), height: Math.min(r.height, 940) };"
                    " })()"
                )
                s = await send("Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 2},
                })
                await save(f"r11-cards-2x-{name}", s["data"])
            return 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
