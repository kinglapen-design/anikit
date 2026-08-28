# CDP 实测：选择模式 item hover 过渡是否与详情面板一致（2026-08-28 走查第 5 条）
# 用户报「选择模式 item 悬停变色没有过渡效果」——两族同挂 .ni-card（items.css 同一条
# transition: background 100ms），本探针实证：① 两族 computed 的 transition 声明一致；
# ② CSS.forcePseudoState 强制 hover 后按时间采样 backgroundColor，验证过渡真实执行（中间值 ≠ 两端）。
# 纯预览页（#design-system-items），无后端桩。留档探针。
import asyncio
import json
import os
import subprocess
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9377
URL = "http://127.0.0.1:5373/#design-system-items"
PROFILE = os.path.join(os.environ["TEMP"], "edge-cdp-items")

# 选择器：常态（非强制态）实例
TARGETS = {
    "detail_thumb": ".lt.ni-card:not(.force-hover)",  # 详情面板 · 链接缩略图
    "pick_file": ".pk.ni-card:not(.force-hover)",  # 选择模式 · 选择文件
    "pick_relation": ".pr.ni-card:not(.force-hover)",  # 选择模式 · 选择关系
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
                r = await send(
                    "Runtime.evaluate", {"expression": expr, "returnByValue": True}
                )
                return r.get("result", {}).get("value")

            await send("Page.enable")
            await send("Runtime.enable")
            await send("DOM.enable")
            await send("CSS.enable")
            await send("Page.navigate", {"url": URL})
            # 等动态 import + 渲染
            for _ in range(40):
                if await js("!!document.querySelector('.pk.ni-card')"):
                    break
                await asyncio.sleep(0.25)
            else:
                print("FATAL: 预览页 item 未渲染")
                return 1

            print("== ① transition 声明（computed）==")
            wiring = await js(
                """
                (() => {
                  const out = {};
                  for (const [k, sel] of Object.entries(%s)) {
                    const el = document.querySelector(sel);
                    if (!el) { out[k] = null; continue; }
                    const cs = getComputedStyle(el);
                    out[k] = { transition: cs.transitionProperty, dur: cs.transitionDuration, bg: cs.backgroundColor };
                  }
                  return out;
                })()
                """
                % json.dumps(TARGETS)
            )
            print(json.dumps(wiring, ensure_ascii=False, indent=2))

            print("== ② forcePseudoState hover 采样 backgroundColor ==")
            doc = await send("DOM.getDocument")
            for name, sel in TARGETS.items():
                node = await send("DOM.querySelector", {"nodeId": doc["root"]["nodeId"], "selector": sel})
                nid = node["nodeId"]
                before = await js(f"getComputedStyle(document.querySelector({json.dumps(sel)})).backgroundColor")
                await send("CSS.forcePseudoState", {"nodeId": nid, "forcedPseudoClasses": ["hover"]})
                samples = []
                t0 = time.monotonic()
                for wait in (0.05, 0.06, 0.1, 0.15):
                    await asyncio.sleep(wait)
                    v = await js(f"getComputedStyle(document.querySelector({json.dumps(sel)})).backgroundColor")
                    samples.append((round((time.monotonic() - t0) * 1000), v))
                await send("CSS.forcePseudoState", {"nodeId": nid, "forcedPseudoClasses": []})
                after = await js(f"getComputedStyle(document.querySelector({json.dumps(sel)})).backgroundColor")
                transiting = any(v not in (before, samples[-1][1]) for _, v in samples)
                print(
                    f"{name}: 常态 {before} → hover {after}；采样 {samples}；"
                    f"过渡执行中 = {'YES' if transiting else 'NO（直跳）'}"
                )
        return 0
    finally:
        proc.terminate()


if __name__ == "__main__":
    raise SystemExit(asyncio.run(main()))
