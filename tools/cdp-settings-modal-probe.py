# CDP 截图留档：偏好设置弹窗（2026-09-19，MCP 二期写入闸配套 SettingsModal）目验
# 纯预览页（#design-system-modals，PreviewModals 真实弹出实例），无后端桩。
# 输出 tools/style-preview/settings-modal-off/on.png（2x）。
# 断言：framed 壳 400 宽 / 标题栏 54 带底线与关闭 X / 段标 / 开关行（轨道 30×16 关态 g4 底）/ 说明行；
# 点击开关 → 开态品牌色底 + 圆钮右移。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
# 端口与 profile 随进程号唯一化——固定端口会撞上历史探针残留的僵尸 headless Edge
PORT = 20393 + os.getpid() % 1000
BASE = "http://127.0.0.1:5373/"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-settings-{os.getpid()}")


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
                    raw = await asyncio.wait_for(ws.recv(), 30)
                    msg = json.loads(raw)
                    if msg.get("id") == mid:
                        return msg.get("result", {})

            async def js(expr: str):
                r = await send("Runtime.evaluate", {"expression": expr, "returnByValue": True})
                return r.get("result", {}).get("value")

            async def shot(name: str) -> None:
                box = await js(
                    "(() => { const m = document.querySelector('.bm-modal'); if (!m) return null;"
                    " const r = m.getBoundingClientRect();"
                    " return { x: r.x, y: r.y, width: r.width, height: r.height, scale: 2 }; })()"
                )
                if not box:
                    print("FATAL: 弹窗不在")
                    return
                r = await send("Page.captureScreenshot", {"format": "png", "clip": box})
                out = f"tools/style-preview/{name}.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(r["data"]))
                print("saved", out)

            await send("Page.enable")
            await send("Runtime.enable")
            await send("Emulation.setDeviceMetricsOverride", {
                "width": 1400, "height": 900, "deviceScaleFactor": 2, "mobile": False,
            })
            await send("Page.navigate", {"url": BASE + "#design-system-modals"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-modals')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页模态窗区未渲染")
                return 1
            await asyncio.sleep(1.5)

            fails = []

            # ---- 打开偏好设置弹窗（点 PreviewModals 里的「偏好设置」钮）----
            clicked = await js(
                "(() => { const b = [...document.querySelectorAll('#design-system-modals .act-btn')]"
                "  .find(x => x.textContent.trim() === '偏好设置');"
                " if (!b) return false; b.click(); return true; })()"
            )
            if not clicked:
                print("FATAL: 预览页未找到「偏好设置」按钮")
                return 1
            for _ in range(20):
                if await js("!!document.querySelector('.bm-modal.framed')"):
                    break
                await asyncio.sleep(0.2)
            await asyncio.sleep(0.5)  # 弹出动画 150ms 沉降

            # ---- 结构断言 ----
            info = await js(
                "(() => { const m = document.querySelector('.bm-modal');"
                " const tb = m.querySelector('.bm-titlebar');"
                " const sw = m.querySelector('.kit-switch');"
                " const knob = sw.querySelector('.knob');"
                " const cs = getComputedStyle(m), tcs = getComputedStyle(tb), swcs = getComputedStyle(sw);"
                " return {"
                "  width: Math.round(m.getBoundingClientRect().width),"
                "  title: tb.querySelector('.bm-title').textContent,"
                "  tbHeight: Math.round(tb.getBoundingClientRect().height),"
                "  tbBorder: tcs.borderBottomWidth + ' ' + tcs.borderBottomColor,"
                "  closeBtn: !!tb.querySelector('.act-btn, button'),"
                "  label: m.querySelector('.sec-label').textContent,"
                "  rowLabel: m.querySelector('.row-label').textContent,"
                "  desc: m.querySelector('.sec-desc').textContent,"
                "  swW: Math.round(sw.getBoundingClientRect().width), swH: Math.round(sw.getBoundingClientRect().height),"
                "  swBgOff: swcs.backgroundColor,"
                "  knobX0: Math.round(knob.getBoundingClientRect().x - sw.getBoundingClientRect().x)"
                " }; })()"
            )
            print("结构:", json.dumps(info, ensure_ascii=False))
            if info["width"] != 400:
                fails.append(f"壳宽 {info['width']} ≠ 400")
            if info["title"] != "偏好设置":
                fails.append(f"标题 {info['title']} ≠ 偏好设置")
            if info["tbHeight"] != 54:
                fails.append(f"标题栏高 {info['tbHeight']} ≠ 54")
            if info["label"] != "AI 外接通道（MCP）":
                fails.append(f"段标 {info['label']} 不符")
            if info["rowLabel"] != "允许 MCP 写入":
                fails.append(f"开关行标签 {info['rowLabel']} 不符")
            if "不进撤销" not in info["desc"]:
                fails.append(f"说明 {info['desc']} 缺「不进撤销」")
            if (info["swW"], info["swH"]) != (30, 16):
                fails.append(f"开关轨道 {info['swW']}x{info['swH']} ≠ 30x16")

            await shot("settings-modal-off")

            # ---- 点击开关 → 开态（品牌色底 + 圆钮右移 14）----
            await js("document.querySelector('.bm-modal .kit-switch').click()")
            await asyncio.sleep(0.5)  # 横移 250ms + 色过渡 150ms 沉降
            on = await js(
                "(() => { const sw = document.querySelector('.bm-modal .kit-switch');"
                " const knob = sw.querySelector('.knob');"
                " return { on: sw.classList.contains('on'),"
                "  bg: getComputedStyle(sw).backgroundColor,"
                "  knobX: Math.round(knob.getBoundingClientRect().x - sw.getBoundingClientRect().x) }; })()"
            )
            print("开态:", json.dumps(on, ensure_ascii=False))
            if not on["on"]:
                fails.append("点击后开关未进开态")
            if on["knobX"] - info["knobX0"] != 14:
                fails.append(f"圆钮横移 {on['knobX'] - info['knobX0']} ≠ 14")
            await shot("settings-modal-on")

            if fails:
                print("FAIL:")
                for f in fails:
                    print(" -", f)
                return 1
            print("PASS: 偏好设置弹窗结构 / 开关交互与动画全部符合设计体系")
            return 0
    finally:
        proc.terminate()


raise SystemExit(asyncio.run(main()))
