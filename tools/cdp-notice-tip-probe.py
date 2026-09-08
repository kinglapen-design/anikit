# CDP 截图留档 + 断言：阶段七悬浮栏族补稿——通知栏 / 选择模式横幅 / 悬停 tip / 悬停-链接列表（2026-08-29，十六轮）
# 纯预览页（#design-system-bars 末三组），无后端桩。输出 tools/style-preview/r16-*.png（2x）。
# 断言：通知栏壳/状态圆点/文字档位、横幅品牌半透明与文案、统一锚点堆叠（横幅在上）、
#       tip 壳与双变体（横排快捷键 / 竖排卡片名称）、链接列表双行结构与行 hover。
# 坑位沿用 bars 探针：端口/profile 随 pid 唯一 + 禁缓存；收尾 taskkill /F /T 整树杀（踩坑 056）；
# 合成事件用 JS dispatch（headless 无窗口焦点）；CSS :hover 走 Input.dispatchMouseEvent 真实鼠标路径。
import asyncio
import base64
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19393 + os.getpid() % 1000
BASE = "http://127.0.0.1:5373/"
PROFILE = os.path.join(os.environ["TEMP"], f"edge-cdp-notice-{os.getpid()}")

L1 = "rgb(242, 242, 242)"  # 亮色/一级
L2 = "rgb(191, 191, 191)"  # 二级
L3 = "rgb(128, 128, 128)"  # 三级
LA3 = "rgba(242, 242, 242, 0.06)"  # 亮色/半透明-3


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

            async def shot(name: str, rect: dict, pad: int = 0) -> None:
                s = await send("Page.captureScreenshot", {
                    "format": "png",
                    "clip": {
                        "x": max(0, rect["x"] - pad), "y": max(0, rect["y"] - pad),
                        "width": min(rect["width"] + pad * 2, 1560),
                        "height": min(rect["height"] + pad * 2, 900),
                        "scale": 2,
                    },
                })
                await save(name, s["data"])

            async def group_scroll(prefix: str) -> bool:
                return await js(
                    "(() => {"
                    " const g = [...document.querySelectorAll('#design-system-bars .pv-group')]"
                    f"  .find(h => h.querySelector('.pv-group-title').textContent.startsWith({json.dumps(prefix)}));"
                    " if (!g) return false; g.scrollIntoView({ block: 'start' }); return true;"
                    " })()"
                )

            async def click_btn(label: str) -> bool:
                return await js(
                    "(() => {"
                    " const b = [...document.querySelectorAll('#design-system-bars .pv-switch')]"
                    f"  .find(h => h.textContent.includes({json.dumps(label)}));"
                    " if (!b) return false; b.click(); return true;"
                    " })()"
                )

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
            await asyncio.sleep(2.0)  # 渲染沉降

            fails = []

            # ================= 通知栏 + 横幅 =================
            if not await group_scroll("通知栏 + 选择模式横幅"):
                fails.append("缺组：通知栏 + 选择模式横幅")
            await asyncio.sleep(0.3)
            for label in ("成功通知", "失败通知", "带「撤销」动作"):
                if not await click_btn(label):
                    fails.append(f"缺触发钮: {label}")
            await asyncio.sleep(0.6)  # 250ms 入场 + 余量

            notice = await js(
                "(() => {"
                " const st = document.querySelector('#design-system-bars .pv-notice-stage');"
                " if (!st) return { err: 'no stage' };"
                " const anchor = st.querySelector('.notice-anchor');"
                " const banner = st.querySelector('.banner');"
                " const toasts = [...st.querySelectorAll('.toast')];"
                " const cs = (el, p) => el ? getComputedStyle(el)[p] : null;"
                " return {"
                "  banner: banner ? {"
                "    bg: cs(banner, 'backgroundColor'), radius: cs(banner, 'borderRadius'),"
                "    h: Math.round(banner.getBoundingClientRect().height),"
                "    blur: cs(banner, 'backdropFilter'),"
                "    bottom: banner.getBoundingClientRect().bottom,"
                "    title: banner.querySelector('.title')?.textContent,"
                "    titleFs: cs(banner.querySelector('.title'), 'fontSize'),"
                "    titleFw: cs(banner.querySelector('.title'), 'fontWeight'),"
                "    text: banner.querySelector('.text')?.textContent,"
                "    textFs: cs(banner.querySelector('.text'), 'fontSize'),"
                "    textColor: cs(banner.querySelector('.text'), 'color'),"
                "    esc: banner.querySelector('.esc')?.textContent,"
                "    escFs: cs(banner.querySelector('.esc'), 'fontSize'),"
                "    escFw: cs(banner.querySelector('.esc'), 'fontWeight'),"
                "    escColor: cs(banner.querySelector('.esc'), 'color'),"
                "  } : null,"
                "  floatLayer: (() => { const f = st.querySelector('.banner-float');"
                "    return f ? { wc: cs(f, 'willChange'), anim: cs(f, 'animationName'),"
                "      iter: cs(f, 'animationIterationCount') } : null; })(),"
                "  toasts: toasts.map(t => ({"
                "    bg: cs(t, 'backgroundColor'), radius: cs(t, 'borderRadius'),"
                "    h: Math.round(t.getBoundingClientRect().height),"
                "    blur: cs(t, 'backdropFilter'), top: t.getBoundingClientRect().top,"
                "    stateBg: cs(t.querySelector('.state'), 'backgroundColor'),"
                "    stateIconColor: cs(t.querySelector('.state'), 'color'),"
                "    msg: t.querySelector('.message')?.textContent,"
                "    msgFs: cs(t.querySelector('.message'), 'fontSize'),"
                "    msgColor: cs(t.querySelector('.message'), 'color'),"
                "    action: t.querySelector('.action')?.textContent ?? null,"
                "    actionFs: t.querySelector('.action') ? cs(t.querySelector('.action'), 'fontSize') : null,"
                "    actionFw: t.querySelector('.action') ? cs(t.querySelector('.action'), 'fontWeight') : null,"
                "    closeColor: cs(t.querySelector('.close'), 'color'),"
                "  })),"
                " };"
                " })()"
            )
            print("通知栏/横幅:", json.dumps(notice, ensure_ascii=False))
            if not notice or "err" in notice:
                fails.append(f"通知栏舞台取值异常: {notice}")
            else:
                b = notice["banner"]
                if not b:
                    fails.append("横幅未渲染")
                else:
                    if b["bg"] != "rgba(15, 137, 236, 0.5)":
                        fails.append(f"横幅底 {b['bg']} ≠ 品牌/半透明")
                    if b["radius"] != "99px" or b["h"] != 46:
                        fails.append(f"横幅形态 {b['radius']}/{b['h']} ≠ 99px/46")
                    if b["blur"] != "blur(40px)":
                        fails.append(f"横幅模糊 {b['blur']} ≠ blur(40px)")
                    if b["title"] != "选择模式" or b["titleFs"] != "16px" or b["titleFw"] != "700":
                        fails.append(f"横幅标题异常: {b['title']} {b['titleFs']} {b['titleFw']}")
                    if b["text"] != "选择文件/档案，为 [01_人物] 添加关系":
                        fails.append(f"横幅文案异常: {b['text']}")
                    if b["textFs"] != "13px" or b["textColor"] != L1:
                        fails.append(f"横幅文案档位异常: {b['textFs']} {b['textColor']}")
                    if b["esc"] != "ESC 退出" or b["escFs"] != "12px" or b["escFw"] != "700" or b["escColor"] != L2:
                        fails.append(f"横幅 ESC 标注异常: {json.dumps(b, ensure_ascii=False)}")
                fl = notice["floatLayer"]
                if not fl or fl["wc"] != "transform" or not str(fl["anim"]).startswith("banner-float") or fl["iter"] != "infinite":
                    fails.append(f"横幅循环浮动层异常（抖动修复要件）: {fl}")
                ts = notice["toasts"]
                if len(ts) != 3:
                    fails.append(f"通知条数 {len(ts)} ≠ 3")
                else:
                    t0, t1, t2 = ts
                    for i, t in enumerate(ts):
                        if t["bg"] != "rgba(41, 41, 41, 0.75)":
                            fails.append(f"通知{i} 底 {t['bg']} ≠ 通用/半透明-2")
                        if t["radius"] != "99px" or t["h"] != 46:
                            fails.append(f"通知{i} 形态 {t['radius']}/{t['h']} ≠ 99px/46")
                        if t["blur"] != "blur(40px)":
                            fails.append(f"通知{i} 模糊 {t['blur']} ≠ blur(40px)")
                        if t["msgFs"] != "13px" or t["msgColor"] != L1:
                            fails.append(f"通知{i} 行为文字档位异常: {t['msgFs']} {t['msgColor']}")
                        if t["stateIconColor"] != L1:
                            fails.append(f"通知{i} 状态图标色 {t['stateIconColor']} ≠ 一级")
                        if t["closeColor"] != L2:
                            fails.append(f"通知{i} 关闭色 {t['closeColor']} ≠ 二级")
                    if t0["stateBg"] != "rgb(7, 167, 31)":
                        fails.append(f"成功圆点底 {t0['stateBg']} ≠ 状态/成功")
                    if t1["stateBg"] != "rgb(220, 57, 57)":
                        fails.append(f"失败圆点底 {t1['stateBg']} ≠ 状态/禁止")
                    if t2["action"] != "撤销" or t2["actionFs"] != "16px" or t2["actionFw"] != "700":
                        fails.append(f"撤销动作异常: {t2['action']} {t2['actionFs']} {t2['actionFw']}")
                    # 统一锚点堆叠：横幅在上、通知在下（gap 8）
                    if b and not (b["bottom"] <= t0["top"] + 1):
                        fails.append(f"横幅未在通知之上: banner.bottom={b['bottom']} toast.top={t0['top']}")

            stage_rect = await js(
                "(() => { const r = document.querySelector('#design-system-bars .pv-notice-stage')"
                " .getBoundingClientRect(); return { x: r.x, y: r.y, width: r.width, height: r.height }; })()"
            )
            if stage_rect:
                await shot("r16-notice-2x", stage_rect, pad=12)

            # ================= 悬停 tip =================
            if not await group_scroll("悬停 tip"):
                fails.append("缺组：悬停 tip")
            await asyncio.sleep(0.3)

            async def tip_open(label: str) -> None:
                await js(
                    "(() => {"
                    " const b = [...document.querySelectorAll('#design-system-bars .pv-switch')]"
                    f"  .find(h => h.textContent.includes({json.dumps(label)}));"
                    " if (b) b.dispatchEvent(new PointerEvent('pointerenter')); return true; })()"
                )
                await asyncio.sleep(0.6)  # 400ms 延迟 + 余量

            async def tip_close(label: str) -> None:
                await js(
                    "(() => {"
                    " const b = [...document.querySelectorAll('#design-system-bars .pv-switch')]"
                    f"  .find(h => h.textContent.includes({json.dumps(label)}));"
                    " if (b) b.dispatchEvent(new PointerEvent('pointerleave')); return true; })()"
                )
                await asyncio.sleep(0.15)

            async def tip_read():
                return await js(
                    "(() => {"
                    " const t = document.querySelector('.app-tip');"
                    " if (!t || !t.classList.contains('show')) return null;"
                    " const cs = (el, p) => el ? getComputedStyle(el)[p] : null;"
                    " const meta = t.querySelector('.app-tip-meta');"
                    " const r = t.getBoundingClientRect();"
                    " return { col: t.classList.contains('col'), text: t.firstElementChild?.textContent,"
                    "   bg: cs(t, 'backgroundColor'), bw: cs(t, 'borderTopWidth'), bc: cs(t, 'borderTopColor'),"
                    "   radius: cs(t, 'borderRadius'), blur: cs(t, 'backdropFilter'),"
                    "   fs: cs(t, 'fontSize'), fw: cs(t, 'fontWeight'), color: cs(t, 'color'),"
                    "   meta: meta?.textContent ?? null, metaFf: meta ? cs(meta, 'fontFamily') : null,"
                    "   metaColor: meta ? cs(meta, 'color') : null,"
                    "   kbd: t.querySelectorAll('.app-tip-kbd, kbd').length,"
                    "   rect: { x: r.x, y: r.y, width: r.width, height: r.height } };"
                    " })()"
                )

            await tip_open("悬停 · 横排")
            tip1 = await tip_read()
            print("tip 横排:", json.dumps(tip1, ensure_ascii=False))
            if not tip1:
                fails.append("横排 tip 未出现")
            else:
                if tip1["text"] != "添加文件/档案链接" or tip1["meta"] != "Ctrl+L":
                    fails.append(f"横排 tip 文案异常: {tip1['text']} / {tip1['meta']}")
                if tip1["bg"] != "rgba(31, 31, 31, 0.93)":
                    fails.append(f"tip 底 {tip1['bg']} ≠ 通用/半透明-1")
                if tip1["bw"] != "1px" or tip1["bc"] != LA3:
                    fails.append(f"tip 描边 {tip1['bw']} {tip1['bc']} ≠ 1px 半透明-3")
                if tip1["radius"] != "6px" or tip1["blur"] != "blur(40px)":
                    fails.append(f"tip 形态 {tip1['radius']}/{tip1['blur']} ≠ 6px/blur(40px)")
                if tip1["fs"] != "12px" or tip1["fw"] != "700" or tip1["color"] != L2:
                    fails.append(f"tip 标题档位异常: {tip1['fs']} {tip1['fw']} {tip1['color']}")
                if not tip1["metaFf"] or "Fira Mono" not in tip1["metaFf"] or tip1["metaColor"] != L3:
                    fails.append(f"tip 快捷键档位异常: {tip1['metaFf']} {tip1['metaColor']}")
                if tip1["kbd"] != 0:
                    fails.append(f"tip 仍存在 kbd 徽章元素（应废除非纯文字）: {tip1['kbd']}")
                await shot("r16-tip-row-2x", tip1["rect"], pad=36)
            await tip_close("悬停 · 横排")

            await tip_open("悬停 · 竖排")
            tip2 = await tip_read()
            print("tip 竖排:", json.dumps(tip2, ensure_ascii=False))
            if not tip2:
                fails.append("竖排 tip 未出现")
            else:
                if not tip2["col"] or tip2["text"] != "雪牧_自动.fla" or tip2["meta"] != "1.2 MB":
                    fails.append(f"竖排 tip 结构异常: {tip2['col']} {tip2['text']} / {tip2['meta']}")
                else:
                    await shot("r16-tip-col-2x", tip2["rect"], pad=36)
            await tip_close("悬停 · 竖排")

            # ================= 悬停-链接列表 =================
            if not await group_scroll("悬停-链接列表"):
                fails.append("缺组：悬停-链接列表")
            await asyncio.sleep(0.3)
            if not await click_btn("显示浮层"):
                fails.append("缺「显示浮层」钮")
            await asyncio.sleep(0.5)  # 150ms 延迟 + rAF 定位 + 余量

            pop = await js(
                "(() => {"
                " const p = document.querySelector('.link-pop');"
                " if (!p) return null;"
                " const cs = (el, q) => el ? getComputedStyle(el)[q] : null;"
                " const rows = [...p.querySelectorAll('.row')].map(r => ({"
                "   h: Math.round(r.getBoundingClientRect().height),"
                "   segs: [...r.querySelectorAll('.seg')].map(s => ({"
                "     t: s.textContent, c: cs(s, 'color'), fw: cs(s, 'fontWeight') })),"
                "   sepColor: cs(r.querySelector('.sep'), 'color'),"
                "   name: r.querySelector('.name')?.textContent,"
                "   nameColor: cs(r.querySelector('.name'), 'color'),"
                "   nameFw: cs(r.querySelector('.name'), 'fontWeight'),"
                "   badge: r.querySelector('.badge svg') ? 1 : 0,"
                " }));"
                " const r0 = p.getBoundingClientRect();"
                " return { bg: cs(p, 'backgroundColor'), bw: cs(p, 'borderTopWidth'),"
                "   bc: cs(p, 'borderTopColor'), radius: cs(p, 'borderRadius'),"
                "   blur: cs(p, 'backdropFilter'), rows,"
                "   rect: { x: r0.x, y: r0.y, width: r0.width, height: r0.height } };"
                " })()"
            )
            print("链接列表:", json.dumps(pop, ensure_ascii=False))
            if not pop:
                fails.append("链接列表浮层未出现")
            else:
                if pop["bg"] != "rgba(31, 31, 31, 0.93)" or pop["radius"] != "6px" or pop["blur"] != "blur(40px)":
                    fails.append(f"链接列表壳异常: {pop['bg']} {pop['radius']} {pop['blur']}")
                if pop["bw"] != "1px" or pop["bc"] != LA3:
                    fails.append(f"链接列表描边 {pop['bw']} {pop['bc']} ≠ 1px 半透明-3")
                if len(pop["rows"]) != 2:
                    fails.append(f"链接列表行数 {len(pop['rows'])} ≠ 2")
                else:
                    r0, r1 = pop["rows"]
                    if r0["h"] != 56:
                        fails.append(f"行高 {r0['h']} ≠ 56（padding 8×2 + 18 + 4 + 18）")
                    seg_t = [s["t"] for s in r0["segs"]]
                    if seg_t != ["文件夹", "01_人物"]:
                        fails.append(f"路径段异常: {seg_t}")
                    if r0["segs"][0]["c"] != L2 or r0["segs"][0]["fw"] != "700":
                        fails.append(f"路径首段档位异常: {json.dumps(r0['segs'][0], ensure_ascii=False)}")
                    if r0["segs"][1]["c"] != L3:
                        fails.append(f"路径后续段档位异常: {json.dumps(r0['segs'][1], ensure_ascii=False)}")
                    if r0["name"] != "雪牧_自动.fla" or r0["nameColor"] != L3 or r0["nameFw"] != "400":
                        fails.append(f"显示名档位异常: {r0['name']} {r0['nameColor']} {r0['nameFw']}")
                    if r0["badge"] != 1 or r1["badge"] != 1:
                        fails.append("徽章缺失")
                await shot("r16-linkpop-2x", pop["rect"], pad=20)

                # 行 hover（CSS :hover 需 CDP 真实鼠标路径）
                row1c = await js(
                    "(() => { const r = document.querySelectorAll('.link-pop .row')[1]"
                    " .getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()"
                )
                if row1c:
                    await send("Input.dispatchMouseEvent", {
                        "type": "mouseMoved", "x": row1c["x"], "y": row1c["y"], "buttons": 0,
                    })
                    await asyncio.sleep(0.15)
                    hov = await js(
                        "(() => { const r = document.querySelectorAll('.link-pop .row')[1];"
                        " return getComputedStyle(r).backgroundColor; })()"
                    )
                    print("次行 hover 底:", hov)
                    if hov != LA3:
                        fails.append(f"行 hover 底 {hov} ≠ 半透明-3")
                    else:
                        await shot("r16-linkpop-hover-2x", pop["rect"], pad=20)

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
        subprocess.run(
            ["taskkill", "/F", "/T", "/PID", str(proc.pid)],
            capture_output=True,
        )


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
