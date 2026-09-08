# 一次性实验：Blink 下 flex item 各 flex 写法对父容器 max-content 宽的贡献
#（FloatBar 播放条进度条 probe 丢 327 basis 的定案实验，2026-09-09）
import asyncio, json, subprocess, sys, tempfile, time, urllib.request
import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")
EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 19371

EXP = """(() => {
  const cases = {
    'A flex 0 1 327px': 'flex:0 1 327px',
    'B flex 0 0 327px': 'flex:0 0 327px',
    'C width 327px (flex 0 1 auto)': 'width:327px',
    'D flex 0 1 327 + min-width 327': 'flex:0 1 327px;min-width:327px',
    'E flex 1 1 327px': 'flex:1 1 327px',
    'F flex 0 1 327 + 内层定宽子': 'flex:0 1 327px',
  };
  const out = {};
  for (const [k, style] of Object.entries(cases)) {
    const d = document.createElement('div');
    d.style.cssText = 'display:flex;width:max-content;position:absolute';
    const inner = k.startsWith('F') ? '<div style="width:327px;height:10px"></div>' : '';
    d.innerHTML = `<div style="flex:none;width:50px;height:10px"></div>` +
                  `<div style="${style};height:10px">${inner}</div>`;
    document.body.appendChild(d);
    out[k] = d.offsetWidth;  // 50 + 贡献
    d.remove();
  }
  return out;
})()"""


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-flex-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        version = None
        for _ in range(50):
            try:
                version = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json/version", timeout=1))
                break
            except Exception:
                time.sleep(0.3)
        # HTTP 端点直接建 tab，连其专属 ws（免 Target 会话握手）
        req = urllib.request.Request(f"http://localhost:{PORT}/json/new?about:blank", method="PUT")
        tab = json.load(urllib.request.urlopen(req, timeout=3))
        async with websockets.connect(tab["webSocketDebuggerUrl"]) as ws:
            await asyncio.sleep(0.5)
            await ws.send(json.dumps({"id": 1, "method": "Runtime.evaluate",
                                      "params": {"expression": EXP, "returnByValue": True}}))
            while True:
                msg = json.loads(await ws.recv())
                if msg.get("id") == 1:
                    print(json.dumps(msg["result"]["result"]["value"], ensure_ascii=False, indent=2))
                    break
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


asyncio.run(main())
