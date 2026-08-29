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
# 端口与 profile 随进程号唯一化——固定端口会撞上历史探针残留的僵尸 headless Edge（CDP 连上旧实例 = 拿到旧页面旧模块）
PORT = 19393 + os.getpid() % 1000
BASE = "http://127.0.0.1:5373/"
# 每次独立临时 profile + 禁 HTTP 缓存——复用 profile 会拿旧模块（混合新旧 = 既非旧版亦非新版的鬼状态）
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-bars-{os.getpid()}")

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
            await send("Page.navigate", {"url": BASE + "#design-system-bars"})
            for _ in range(40):
                if await js("!!document.querySelector('#design-system-bars .fb-shell')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页悬浮栏区未渲染")
                return 1
            await asyncio.sleep(2.0)  # 渲染沉降（懒加载 + 字体 + 挂载测宽链）

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

            # ---- 断言 5（十五轮修复实证）：聚焦/失焦宽度动画中段，筛选图标随壳边平滑移动 ----
            # 旧机制（模式根自持宽度类瞬时切换）下：根瞬跳 450、壳还在 250 附近 → 图标右缘超出壳右缘 ~170px。
            # 新机制（壳宽唯一动画源 + 模式根 100% 随壳）：中段任意时刻 图标右缘 ≈ 壳右缘 - 16 padding。
            # 注：headless 无窗口焦点，原生 focus() 只挂 activeElement 不发事件（实机正常）——探针改用合成 FocusEvent。
            async def bars_mid(tag: str):
                v = await js(
                    "(() => { try {"
                    " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                    "  .find(h => h.querySelector('.pv-group-title').textContent.startsWith('多功能悬浮栏'));"
                    " if (!g) return { err: 'no group' };"
                    " const s = g.querySelector('.fb-shell').getBoundingClientRect();"
                    " const ic = g.querySelector('.fbs-ic.filter');"
                    " if (!ic) return { err: 'no filter icon', w: Math.round(s.width),"
                    "   html: g.querySelector('.fb-shell').innerHTML.slice(0, 260) };"
                    " const i = ic.getBoundingClientRect();"
                    " return { w: Math.round(s.width), d: Math.round(s.right - i.right) };"
                    " } catch (e) { return { err: e.message }; } })()"
                )
                print(tag, v)
                return v

            await js(
                "(() => { const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                "  .find(h => h.querySelector('.pv-group-title').textContent.startsWith('多功能悬浮栏'));"
                " g.querySelector('.fbs-input').dispatchEvent(new FocusEvent('focus')); return true; })()"
            )
            await asyncio.sleep(0.12)  # 300ms 过渡中段
            mid_focus = await bars_mid("聚焦中段:")
            if not mid_focus or "err" in mid_focus:
                fails.append(f"聚焦中段取值异常: {mid_focus}")
            elif not (251 < mid_focus["w"] < 449):
                fails.append(f"聚焦中段壳宽 {mid_focus['w']} 不在 251~448 动画区间（动画未发生？）")
            elif abs(mid_focus["d"] - 16) > 24:
                fails.append(f"聚焦中段筛选图标未随壳边（右缘差 {mid_focus['d']} ≠ 16）")
            await asyncio.sleep(0.6)
            end_focus = await bars_mid("聚焦末态:")
            if not end_focus or "err" in end_focus or end_focus["w"] != 450 or end_focus["d"] != 16:
                fails.append(f"聚焦末态异常: {end_focus}（应 w=450 d=16）")
            await js(
                "(() => { const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                "  .find(h => h.querySelector('.pv-group-title').textContent.startsWith('多功能悬浮栏'));"
                " g.querySelector('.fbs-input').dispatchEvent(new FocusEvent('blur')); return true; })()"
            )
            await asyncio.sleep(0.12)
            mid_blur = await bars_mid("失焦中段:")
            if not mid_blur or "err" in mid_blur:
                fails.append(f"失焦中段取值异常: {mid_blur}")
            elif not (251 < mid_blur["w"] < 449):
                fails.append(f"失焦中段壳宽 {mid_blur['w']} 不在 251~448 动画区间")
            elif abs(mid_blur["d"] - 16) > 24:
                fails.append(f"失焦中段筛选图标未随壳边（右缘差 {mid_blur['d']} ≠ 16）")
            await asyncio.sleep(0.6)
            end_blur = await bars_mid("失焦末态:")
            if not end_blur or "err" in end_blur or end_blur["w"] != 250 or end_blur["d"] != 16:
                fails.append(f"失焦末态异常: {end_blur}（应 w=250 d=16）")

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
                await save(f"r15-bars-2x-{name}", s["data"])

            if fails:
                print("FAIL:", *fails, sep="\n  ")
                if console_logs:
                    print("页面 console:", *console_logs[-20:], sep="\n  ")
                return 1
            print("PASS: 全部断言通过")
            if console_logs:
                print("页面 console:", *console_logs[-10:], sep="\n  ")
            return 0
    finally:
        # Windows 上 terminate 只杀父进程——headless Edge 子进程树会整体残留成僵尸
        # （占调试端口导致后续探针连上旧实例），必须 taskkill /T 整树
        subprocess.run(
            ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
            capture_output=True,
        )


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
