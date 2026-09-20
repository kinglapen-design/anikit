# CDP 截图留档：重构 R0/R1 预览页新展区目验（2026-09-21）
# 覆盖：组件台账 inventory / 业务弹窗 dialogs / 浮层与滚动 overlays / 独立窗口 windows。
# 纯预览页（#design-system），无后端桩。输出 tools/style-preview/r21-*.png（2x）。
# 断言：台账行数=148 且四类徽标齐；三新展区渲染且有演示内容。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 21371 + os.getpid() % 1000
BASE = "http://127.0.0.1:5373/"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-r1cov-{os.getpid()}")

SECTIONS = ["inventory", "dialogs", "overlays", "windows"]


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
            console_logs = []

            async def send(method: str, params: dict | None = None) -> dict:
                nonlocal mid
                mid += 1
                await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
                while True:
                    raw = await asyncio.wait_for(ws.recv(), 30)
                    msg = json.loads(raw)
                    if msg.get("method") == "Runtime.consoleAPICalled":
                        args = msg["params"].get("args", [])
                        console_logs.append(
                            msg["params"].get("type", "log") + ": "
                            + " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args)
                        )
                        continue
                    if msg.get("method") == "Runtime.exceptionThrown":
                        console_logs.append("EXCEPTION: " + json.dumps(msg["params"], ensure_ascii=False)[:500])
                        continue
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
            await send("Network.enable")
            await send("Network.setCacheDisabled", {"cacheDisabled": True})
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1600, "height": 1000, "deviceScaleFactor": 2, "mobile": False,
            })
            await send("Page.navigate", {"url": BASE + "#design-system"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-windows')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页 windows 展区未渲染")
                return 1
            await asyncio.sleep(2.0)

            fails = []

            # ---- 断言 1：台账行数与四类徽标 ----
            inv = await js(
                "(() => {"
                " const s = document.querySelector('#design-system-inventory');"
                " if (!s) return null;"
                " const rows = s.querySelectorAll('tbody tr').length;"
                " const badges = [...s.querySelectorAll('.pv-inv-badge')].map(b => b.className);"
                " const kinds = ['legacy','dead','business','new'].map(k =>"
                "   badges.filter(c => c.includes(k)).length);"
                " return { rows, kinds };"
                " })()"
            )
            print("台账:", inv)
            if not inv:
                fails.append("台账展区缺失")
            else:
                if inv["rows"] != 148:
                    fails.append(f"台账行数 {inv['rows']} ≠ 148")
                if inv["kinds"][0] != 2:
                    fails.append(f"旧件徽标 {inv['kinds'][0]} ≠ 2")
                if inv["kinds"][1] != 2:
                    fails.append(f"休眠徽标 {inv['kinds'][1]} ≠ 2")

            # ---- 断言 2：三新展区有内容 ----
            for sec, min_cells in [("dialogs", 12), ("overlays", 6), ("windows", 3)]:
                n = await js(
                    f"document.querySelectorAll('#design-system-{sec} .pv-demo-cell,"
                    f"#design-system-{sec} .pv-group').length"
                )
                print(f"{sec} 演示单元(含组):", n)
                if not n or n < min_cells:
                    fails.append(f"{sec} 演示内容不足: {n}")

            # ---- 断言 3：控制台无异常 ----
            errs = [l for l in console_logs if l.startswith(("EXCEPTION", "error"))]
            if errs:
                fails.append("控制台异常: " + " | ".join(errs[:5]))

            # ---- 分区截图（元素 clip） ----
            for sec in SECTIONS:
                await js(f"document.querySelector('#design-system-{sec}').scrollIntoView()")
                await asyncio.sleep(0.4)
                shot = await send("Page.captureScreenshot", {"format": "png"})
                await save(f"r21-{sec}", shot["data"])

            for l in console_logs[:10]:
                print("console:", l)
            if fails:
                print("FAILS:")
                for f in fails:
                    print(" -", f)
                return 1
            print("ALL PASS")
            return 0
    finally:
        proc.kill()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
