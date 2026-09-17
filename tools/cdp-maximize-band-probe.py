# CDP 探针（一次性诊断）：实机走查「全屏后中间面板纵向没填满」——最大化态底部分带量化
# 连接真机 dev app 的 WebView2（启动须带 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=19333）。
# 测量链：最大化前后各取 —— 视口 window.innerWidth/innerHeight(CSS px)、DPR、.window 元素 rect、
# CDP Browser.getWindowForTarget 外层窗口物理矩形。band = 外窗物理高 − innerHeight×DPR（非客户区总量），
# 与常态对比：若最大化后 band 显著增大 = 客户区比可视窗口矮（底带），差值即底带物理高。
# 用法：tools/.venv/Scripts/python.exe tools/cdp-maximize-band-probe.py
import asyncio
import json
import sys
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

PORT = 19333
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


async def eval_js(ws, session, expr):
    r = await send(ws, "Runtime.evaluate", {
        "expression": expr, "returnByValue": True, "awaitPromise": True,
    }, session)
    res = r.get("result", {}).get("result", {})
    if "value" not in res:
        print("  [eval 异常]", json.dumps(r.get("result", {}), ensure_ascii=False)[:400])
    return res.get("value")


MEASURE = """
(() => {
  const w = document.querySelector('.window');
  const r = w ? w.getBoundingClientRect() : null;
  return {
    innerW: window.innerWidth, innerH: window.innerHeight, dpr: window.devicePixelRatio,
    docClientH: document.documentElement.clientHeight,
    winRect: r ? { left: r.left, top: r.top, width: r.width, height: r.height } : null,
  };
})()
"""

MAXIMIZE = """
(async () => {
  try { await window.__TAURI_INTERNALS__.invoke('plugin:window|maximize'); return 'ok'; }
  catch (e) { return 'ERR:' + e; }
})()
"""
UNMAXIMIZE = """
(async () => {
  try { await window.__TAURI_INTERNALS__.invoke('plugin:window|unmaximize'); return 'ok'; }
  catch (e) { return 'ERR:' + e; }
})()
"""


async def main():
    ver = json.loads(urllib.request.urlopen(f"http://localhost:{PORT}/json/version").read())
    targets = json.loads(urllib.request.urlopen(f"http://localhost:{PORT}/json").read())
    page = next((t for t in targets if t.get("type") == "page"), None)
    if not page:
        print("FAIL: 无 page 目标", targets)
        return
    print("目标:", page.get("title"), "|", page.get("url"))
    async with websockets.connect(ver["webSocketDebuggerUrl"], max_size=64 * 1024 * 1024) as ws:
        att = await send(ws, "Target.attachToTarget", {"targetId": page["id"], "flatten": True})
        session = att.get("result", {}).get("sessionId")
        if not session:
            print("FAIL: attach 失败", att)
            return

        async def window_bounds():
            r = await send(ws, "Browser.getWindowForTarget", {"targetId": page["id"]})
            return r.get("result", {}).get("bounds", {})

        pre = await eval_js(ws, session, MEASURE)
        pre_b = await window_bounds()
        print("最大化前:", json.dumps(pre), "外窗:", json.dumps(pre_b))

        print("maximize 调用:", await eval_js(ws, session, MAXIMIZE))
        await asyncio.sleep(1.5)  # 等最大化动画与样式守卫落定

        post = await eval_js(ws, session, MEASURE)
        post_b = await window_bounds()
        print("最大化后:", json.dumps(post), "外窗:", json.dumps(post_b))

        if pre and post and pre_b and post_b:
            band_pre = pre_b.get("height", 0) - pre["innerH"] * pre["dpr"]
            band_post = post_b.get("height", 0) - post["innerH"] * post["dpr"]
            print(f"非客户区总量(物理px)：常态={band_pre:.1f} 最大化={band_post:.1f} 差={band_post - band_pre:.1f}")
            vp = post["innerH"] * post["dpr"]
            wr = (post["winRect"] or {}).get("height", 0) * post["dpr"]
            print(f"最大化态：外窗高={post_b.get('height')} 视口物理高={vp:.0f} .window 元素物理高={wr:.0f}")

        # 收尾：还原窗口（不留在最大化态）
        print("unmaximize:", await eval_js(ws, session, UNMAXIMIZE))


asyncio.run(main())
