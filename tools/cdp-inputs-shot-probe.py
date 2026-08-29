# CDP 截图留档：阶段七输入框组件预览页目验（2026-08-29）+ 卡片格式角标中文加粗复验
# 纯预览页（#design-system-inputs / #design-system-cards），无后端桩。
# 输出 tools/style-preview/r13-inputs-*.png / r12-badge-bold-2x.png。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9393
BASE = "http://127.0.0.1:5373/"
PROFILE = os.path.join(os.environ["TEMP"], "edge-cdp-inputs")


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
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1600, "height": 1000, "deviceScaleFactor": 2, "mobile": False,
            })
            await send("Page.navigate", {"url": BASE + "#design-system-inputs"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-inputs .ti-root')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页输入框区未渲染")
                return 1
            await asyncio.sleep(1.0)

            # 2x 输入框两组
            for name, title in {"lg": "输入框-大", "sm": "输入框-小"}.items():
                ok = await js(
                    "(() => {"
                    " const hs = [...document.querySelectorAll('#design-system-inputs .pv-group')];"
                    f" const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " if (!g) return false; g.scrollIntoView({ block: 'start' }); return true;"
                    " })()"
                )
                if not ok:
                    print("MISS", name)
                    continue
                await asyncio.sleep(0.3)
                rect = await js(
                    "(() => {"
                    " const hs = [...document.querySelectorAll('#design-system-inputs .pv-group')];"
                    f" const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " const r = g.getBoundingClientRect();"
                    " return { x: Math.max(0, r.x), y: Math.max(0, r.y), width: Math.min(r.width, 1560), height: Math.min(r.height, 400) };"
                    " })()"
                )
                s = await send("Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 2},
                })
                await save(f"r13-inputs-2x-{name}", s["data"])

            # 卡片角标加粗复验（格式角标组）
            await send("Page.navigate", {"url": BASE + "#design-system-cards"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-cards .fmt-badge')"):
                    break
                await asyncio.sleep(0.25)
            await asyncio.sleep(1.0)
            ok = await js(
                "(() => {"
                " const hs = [...document.querySelectorAll('#design-system-cards .pv-group')];"
                " const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith('格式角标'));"
                " if (!g) return false; g.scrollIntoView({ block: 'start' }); return true;"
                " })()"
            )
            if ok:
                await asyncio.sleep(0.3)
                rect = await js(
                    "(() => {"
                    " const hs = [...document.querySelectorAll('#design-system-cards .pv-group')];"
                    " const g = hs.find(h => h.querySelector('.pv-group-title').textContent.startsWith('格式角标'));"
                    " const r = g.getBoundingClientRect();"
                    " return { x: Math.max(0, r.x), y: Math.max(0, r.y), width: Math.min(r.width, 800), height: Math.min(r.height, 200) };"
                    " })()"
                )
                s = await send("Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 2},
                })
                await save("r12-badge-bold-2x", s["data"])
            return 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
