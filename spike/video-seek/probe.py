# 视频播放通道真机实测（5.3 方向转换，2026-08-22）——在真实 Tauri WebView2（非仿真探针）里
# 对照两条 <video> 源通道：A = asset protocol（convertFileSrc，5.3 前架构）；B = localhost
# Range HTTP server（本脚本 8899 端口，Python 手写 Range/206）。指标：首帧 loadeddata /
# 顺序 seek×8 / 随机 seek×12（seeked 延迟）/ 66ms 连发 scrub×25（完成数 + 停手到画面到位
# 的 settle）/ 播放 3s 的 rVFC 实际帧率。素材：QQ 1080p30 动画参考 + 4K120 录屏（重载）。
# 前置：tauri dev 以 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9223 启动。
# 用法：tools/.venv/Scripts/python.exe spike/video-seek/probe.py
import asyncio
import http.server
import json
import os
import re
import threading
import urllib.parse
import urllib.request

import websockets

CDP = "http://localhost:9223"
HTTP_PORT = 8899
ROOT = r"E:\01_Projects\03_Software-dev\AniKit"

FILES = [
    {
        "name": "qq1080p30",
        "path": ROOT + r"\assets\video\QQ20260815-212153.mp4",
        "http": f"http://127.0.0.1:{HTTP_PORT}/QQ20260815-212153.mp4",
    },
    {
        "name": "desktop4k120",
        "path": ROOT + r"\assets\video\Desktop 2026.08.22 - 17.32.24.02_1.mp4",
        "http": f"http://127.0.0.1:{HTTP_PORT}/" + urllib.parse.quote("Desktop 2026.08.22 - 17.32.24.02_1.mp4"),
    },
]

MEASURE_JS = r"""
(async (src) => {
  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
  const v = document.createElement("video");
  v.style.cssText =
    "position:fixed;left:10px;top:10px;width:960px;height:540px;z-index:99999;background:#000;object-fit:contain";
  v.muted = true;
  v.preload = "auto";
  document.body.appendChild(v);
  const out = {};
  try {
    const t0 = performance.now();
    v.src = src;
    await new Promise((res, rej) => {
      const to = setTimeout(() => rej(new Error("loadeddata timeout 15s")), 15000);
      v.addEventListener("loadeddata", () => { clearTimeout(to); res(); }, { once: true });
      v.addEventListener("error", () => {
        clearTimeout(to);
        rej(new Error("video error code=" + (v.error && v.error.code)));
      }, { once: true });
    });
    out.firstFrameMs = Math.round(performance.now() - t0);
    out.duration = Math.round(v.duration * 100) / 100;
    out.videoSize = v.videoWidth + "x" + v.videoHeight;
    const dur = v.duration;

    async function seekOnce(t) {
      const s = performance.now();
      await new Promise((res, rej) => {
        const to = setTimeout(() => rej(new Error("seek timeout 8s")), 8000);
        v.addEventListener("seeked", () => { clearTimeout(to); res(); }, { once: true });
        v.currentTime = t;
      });
      return performance.now() - s;
    }

    const seq = [];
    for (let i = 0; i < 8; i++) seq.push(await seekOnce((dur * (i + 0.5)) / 8));
    out.seqSeekMs = seq.map(Math.round);

    let seed = 42;
    const rand = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
    const rnd = [];
    for (let i = 0; i < 12; i++) rnd.push(await seekOnce(rand() * dur));
    out.rndSeekMs = rnd.map(Math.round);

    // 模拟 scrub：66ms 间隔连发 25 次 currentTime（不等 seeked）
    let seekedCount = 0;
    let lastSeekedAt = 0;
    const onSeeked = () => { seekedCount++; lastSeekedAt = performance.now(); };
    v.addEventListener("seeked", onSeeked);
    for (let i = 0; i < 25; i++) {
      v.currentTime = (dur * i) / 25;
      await sleep(66);
    }
    const lastSetAt = performance.now();
    // 停手到位延迟：连发结束后再做一次正式 seek 计量（seeked 配对最后一次设置）
    v.currentTime = dur * 0.5;
    await new Promise((res, rej) => {
      const to = setTimeout(() => rej(new Error("settle timeout 3s")), 3000);
      v.addEventListener("seeked", () => { clearTimeout(to); res(); }, { once: true });
    });
    out.scrubSeekedCount = seekedCount;
    out.scrubSettleMs = Math.round(performance.now() - lastSetAt);
    v.removeEventListener("seeked", onSeeked);

    // 连续播放 3 秒：rVFC 实际渲染帧率
    await seekOnce(0);
    let frames = 0;
    let stopped = false;
    const tick = () => { if (stopped) return; frames++; v.requestVideoFrameCallback(tick); };
    v.requestVideoFrameCallback(tick);
    const p0 = performance.now();
    await v.play();
    await sleep(3000);
    v.pause();
    stopped = true;
    out.playFps = Math.round(frames / ((performance.now() - p0) / 1000));
  } finally {
    v.pause();
    v.removeAttribute("src");
    v.load();
    v.remove();
  }
  return out;
})
"""


class RangeHandler(http.server.BaseHTTPRequestHandler):
    """最小 Range/206 静态服务（Chromium 媒体 seek 依赖 Range 随机访问）"""

    def log_message(self, *args):
        pass

    def _serve(self, head_only: bool):
        rel = urllib.parse.unquote(self.path).lstrip("/")
        full = os.path.join(ROOT, "assets", "video", rel)
        if not os.path.isfile(full):
            self.send_error(404)
            return
        size = os.path.getsize(full)
        start, end, status = 0, size - 1, 200
        rng = self.headers.get("Range")
        if rng:
            m = re.match(r"bytes=(\d*)-(\d*)", rng)
            if m:
                if m.group(1):
                    start = int(m.group(1))
                if m.group(2):
                    end = min(int(m.group(2)), size - 1)
                status = 206
        length = max(0, end - start + 1)
        self.send_response(status)
        self.send_header("Content-Type", "video/mp4")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(length))
        if status == 206:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.end_headers()
        if head_only:
            return
        with open(full, "rb") as f:
            f.seek(start)
            remaining = length
            while remaining > 0:
                chunk = f.read(min(1 << 20, remaining))
                if not chunk:
                    break
                try:
                    self.wfile.write(chunk)
                except (BrokenPipeError, ConnectionResetError):
                    break
                remaining -= len(chunk)

    def do_GET(self):
        self._serve(False)

    def do_HEAD(self):
        self._serve(True)


def start_http_server():
    srv = http.server.ThreadingHTTPServer(("127.0.0.1", HTTP_PORT), RangeHandler)
    threading.Thread(target=srv.serve_forever, daemon=True).start()
    return srv


def wait_cdp_page(timeout_s: float = 90) -> str:
    import time as _time

    begin = _time.time()
    while _time.time() - begin < timeout_s:
        try:
            with urllib.request.urlopen(CDP + "/json", timeout=3) as r:
                targets = json.loads(r.read())
            for t in targets:
                if t.get("type") == "page" and "127.0.0.1:5373" in t.get("url", ""):
                    return t["webSocketDebuggerUrl"]
        except Exception:
            pass
        _time.sleep(1)
    raise RuntimeError("CDP 9223 等待超时（tauri dev 未带调试端口启动？）")


async def cdp_eval(ws, expr: str, mid: int):
    await ws.send(json.dumps({
        "id": mid,
        "method": "Runtime.evaluate",
        "params": {"expression": expr, "awaitPromise": True, "returnByValue": True},
    }))
    while True:
        msg = json.loads(await ws.recv())
        if msg.get("id") == mid:
            return msg


async def main():
    srv = start_http_server()
    ws_url = await asyncio.get_event_loop().run_in_executor(None, wait_cdp_page)
    print(f"CDP page: {ws_url}")
    async with websockets.connect(ws_url, max_size=64 * 1024 * 1024) as ws:
        mid = 0
        rows = []
        for f in FILES:
            for channel in ("asset", "http"):
                src_expr = (
                    f"window.__TAURI_INTERNALS__.convertFileSrc({json.dumps(f['path'])})"
                    if channel == "asset"
                    else json.dumps(f["http"])
                )
                expr = f"({MEASURE_JS})({src_expr})"
                mid += 1
                resp = await cdp_eval(ws, expr, mid)
                result = resp.get("result", {}).get("result", {})
                if result.get("type") == "object" and "value" in result:
                    rows.append((f["name"], channel, result["value"]))
                else:
                    rows.append((f["name"], channel, {"error": resp.get("result", {}).get("exceptionDetails", resp)}))
                print(f"[done] {f['name']} / {channel}")

    srv.shutdown()
    print("\n===== 结果 =====")
    for name, channel, r in rows:
        print(f"\n--- {name} / {channel}")
        print(json.dumps(r, ensure_ascii=False, indent=1))


if __name__ == "__main__":
    asyncio.run(main())
