# CDP 截图留档：阶段七悬浮栏组件族预览页目验（2026-08-29，十四轮）
# 纯预览页（#design-system-bars），无后端桩。输出 tools/style-preview/r14-bars-*.png（2x）。
# 断言：搜索栏三态宽度（250/450/展开>450）、播放条结构（图标数/时间码/进度条）、弹层尺寸。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9394
BASE = "http://127.0.0.1:5373/"
PROFILE = os.path.join(os.environ["TEMP"], "edge-cdp-bars")

# 展区组标题 → 截图名（组级截屏）
GROUPS = {
    "morph": "多功能悬浮栏 · 模式切换",
    "search": "搜索栏（常态 250",
    "markdown": "MarkDown 工具栏",
    "play": "播放条（全量陈列",
    "rate": "倍率播放弹层",
    "volume": "音量滑块弹层",
    "frames": "帧条 + 播放条",
    "notes": "笔记弹层",
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
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1600, "height": 1000, "deviceScaleFactor": 2, "mobile": False,
            })
            await send("Page.navigate", {"url": BASE + "#design-system-bars"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-bars .fb-shell')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页悬浮栏区未渲染")
                return 1
            await asyncio.sleep(1.0)

            fails = []

            # ---- 断言 1：搜索栏三态宽度（250 / 450 / 展开 >450 且含 10 筛选钮）----
            ws_search = await js(
                "[...document.querySelectorAll('#design-system-bars .pv-group')]"
                ".find(g => g.querySelector('.pv-group-title').textContent.startsWith('搜索栏'))"
                " && (() => {"
                " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                "   .find(g => g.querySelector('.pv-group-title').textContent.startsWith('搜索栏'));"
                " const bars = [...g.querySelectorAll('.fb-shell')];"
                " return bars.map(b => Math.round(b.getBoundingClientRect().width));"                " })()"
            )
            print("搜索栏三态宽度(逻辑px):", ws_search)
            if not ws_search or len(ws_search) != 3:
                fails.append(f"搜索栏实例数≠3: {ws_search}")
            else:
                if ws_search[0] != 250:
                    fails.append(f"搜索栏常态宽 {ws_search[0]} ≠ 250")
                if ws_search[1] != 450:
                    fails.append(f"搜索栏聚焦宽 {ws_search[1]} ≠ 450")
                if ws_search[2] <= 450:
                    fails.append(f"搜索栏展开宽 {ws_search[2]} 未超过 450")
            chips = await js(
                "(() => {"
                " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                "   .find(g => g.querySelector('.pv-group-title').textContent.startsWith('搜索栏'));"
                " return g.querySelectorAll('.fbs-chip').length;"
                " })()"
            )
            if chips != 10:
                fails.append(f"筛选钮数 {chips} ≠ 10")

            # ---- 断言 2：播放条结构（10 图标钮 = 播放/暂停 + 快退/快进 + 上/下帧 + 声音静音 + 循环/笔记/封面/全屏；倍速 + 时间码 + 进度条）----
            play = await js(
                "(() => {"
                " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                "   .find(g => g.querySelector('.pv-group-title').textContent.startsWith('播放条'));"
                " const first = g.querySelector('.fbp-root');"
                " return { icons: first.querySelectorAll('.ic-btn').length,"
                "   rate: !!first.querySelector('.fbp-rate'), tc: !!first.querySelector('.fbp-tc'),"
                "   slider: !!first.querySelector('.bs-root') };"
                " })()"
            )
            print("播放条结构:", play)
            if not play or play.get("icons") != 10 or not play.get("rate") or not play.get("tc") or not play.get("slider"):
                fails.append(f"播放条结构异常: {play}")

            # ---- 断言 3：倍率 11 档 + 音量弹宽 124 + 笔记 533×378 ----
            rc = await js("document.querySelectorAll('#design-system-bars .rp-item').length / 2")  # 静态+交互两实例
            if int(rc) != 11:
                fails.append(f"倍率档数 {rc} ≠ 11/实例")
            vp = await js(
                "(() => { const el = document.querySelector('#design-system-bars .vp-root');"
                " const r = el.getBoundingClientRect(); return Math.round(r.width); })()"
            )
            if vp != 124:
                fails.append(f"音量弹层宽 {vp} ≠ 124")
            np = await js(
                "(() => { const el = document.querySelector('#design-system-bars .np-root');"
                " const r = el.getBoundingClientRect();"
                " return [Math.round(r.width), Math.round(r.height)]; })()"
            )
            if np != [533, 378]:
                fails.append(f"笔记弹层尺寸 {np} ≠ [533, 378]")

            # ---- 断言 4：帧条与播放条同宽（组合区）----
            combo = await js(
                "(() => { const c = document.querySelector('#design-system-bars .pv-combo');"
                " const fs = c.querySelector('.fs-root').getBoundingClientRect().width;"
                " const bar = c.querySelector('.fb-shell').getBoundingClientRect().width;"
                " return [Math.round(fs), Math.round(bar)]; })()"
            )
            print("帧条/播放条组合宽:", combo)
            if not combo or combo[0] != combo[1]:
                fails.append(f"帧条与播放条不同宽: {combo}")

            # ---- 组级截图（2x）----
            for name, title in GROUPS.items():
                ok = await js(
                    "(() => {"
                    " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                    f"  .find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " if (!g) return false; g.scrollIntoView({ block: 'start' }); return true;"
                    " })()"
                )
                if not ok:
                    print("MISS", name)
                    fails.append(f"展区缺组: {title}")
                    continue
                await asyncio.sleep(0.3)
                rect = await js(
                    "(() => {"
                    " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                    f"  .find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(title)}));"
                    " const r = g.getBoundingClientRect();"
                    " return { x: Math.max(0, r.x), y: Math.max(0, r.y),"
                    "   width: Math.min(r.width, 1560), height: Math.min(r.height, 460) };"
                    " })()"
                )
                s = await send("Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 2},
                })
                await save(f"r14-bars-2x-{name}", s["data"])

            if fails:
                print("FAIL:", *fails, sep="\n  ")
                return 1
            print("PASS: 全部断言通过")
            return 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
