# 一次性目验（2026-09-25 音频音乐预览轮）：截 #design-system 预览页两个展区——
# states 的音乐预览复刻壳（AppIcon vinyl 渲染验证）+ bars 的音频波形进度格（WaveProgress 高度修复验证）。
import asyncio
import base64
import json
import subprocess
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9377
URL = "http://localhost:5373/#design-system"

SHOTS = [
    ("states", "document.querySelector('#design-system-states .pv-group:last-child .pv-demo-cell')", "tools/style-preview/audio-pane-music.png"),
    ("bars", "document.querySelector('#design-system-bars .wp-root')?.closest('.pv-demo-cell')", "tools/style-preview/audio-bar-wave.png"),
]


async def main():
    proc = subprocess.Popen([
        EDGE, "--headless=new", "--disable-gpu", "--window-size=1400,1000",
        f"--remote-debugging-port={PORT}", f"--user-data-dir={tempfile.mkdtemp(prefix='edge-pv-shot-')}", "about:blank",
    ], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        ws_url = None
        for _ in range(50):
            try:
                tabs = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json", timeout=1))
                page = next((t for t in tabs if t.get("type") == "page"), None)
                if page:
                    ws_url = page["webSocketDebuggerUrl"]
                    break
            except Exception:
                pass
            time.sleep(0.3)
        if not ws_url:
            print("FAIL: CDP 连接失败")
            return
        async with websockets.connect(ws_url, max_size=50 * 1024 * 1024) as ws:
            mid = 0

            async def cmd(method, params=None):
                nonlocal mid
                mid += 1
                await ws.send(json.dumps({"id": mid, "method": method, "params": params or {}}))
                while True:
                    m = json.loads(await ws.recv())
                    if m.get("id") == mid:
                        return m.get("result", {})

            await cmd("Page.enable")
            await cmd("Page.navigate", {"url": URL})
            await asyncio.sleep(3)
            for tag, expr, path in SHOTS:
                box = (await cmd("Runtime.evaluate", {"expression": f"""(() => {{
                  const el = {expr};
                  if (!el) return null;
                  el.scrollIntoView({{ block: 'center' }});
                  const r = el.getBoundingClientRect();
                  return {{ x: r.left, y: r.top, width: r.width, height: r.height }};
                }})()""", "returnByValue": True}))["result"].get("value")
                if not box:
                    print(f"FAIL[{tag}]: 表达式未命中 {expr}")
                    continue
                await asyncio.sleep(0.5)
                shot = await cmd("Page.captureScreenshot", {"format": "png", "clip": {**box, "scale": 2}})
                with open(path, "wb") as f:
                    f.write(base64.b64decode(shot["data"]))
                print(f"截图[{tag}]: {path} ({box['width']:.0f}x{box['height']:.0f})")
    finally:
        proc.kill()


asyncio.run(main())
