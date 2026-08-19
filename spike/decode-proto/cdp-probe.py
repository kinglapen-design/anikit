# 阶段四 spike：decode-shell 三通道实测探针（2026-08-20）
# 起真实 Tauri 壳（WebView2 CDP 端口 9333），逐通道切 ?ch=ws|invoke|proto 采样 fps，
# 同时用 psutil 采样 decode-shell + 全部 msedgewebview2 子进程 CPU 占比。
# 用法：tools/.venv/Scripts/python.exe spike/decode-proto/cdp-probe.py [lanes] [媒体文件]
import asyncio
import json
import os
import subprocess
import sys
import time
import urllib.request

import psutil
import websockets

PORT = 9333
ROOT = os.path.dirname(os.path.abspath(__file__))
EXE = os.path.join(ROOT, "shell", "target", "debug", "decode-shell.exe")
LANES = int(sys.argv[1]) if len(sys.argv) > 1 else 16
MEDIA = sys.argv[2] if len(sys.argv) > 2 else os.path.join(ROOT, "testdata", "big.gif")
CHANNELS = (sys.argv[3].split(",") if len(sys.argv) > 3 else ["ws", "invoke", "proto"])
PACE = sys.argv[4] if len(sys.argv) > 4 else None
SETTLE_SECS = 3
SAMPLE_SECS = 6

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
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:400])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = [str(a.get("value", a.get("description", "")))[:200] for a in raw["params"]["args"]]
            print("[CONSOLE]", raw["params"]["type"], " ".join(args)[:400])


async def evaljs(ws, session, expr):
    r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True}, session)
    return r.get("result", {}).get("result", {}).get("value")


def target_procs():
    out = []
    for p in psutil.process_iter(["name"]):
        n = (p.info["name"] or "").lower()
        if n.startswith("decode-shell") or n.startswith("msedgewebview2"):
            out.append(p)
    return out


def cpu_percent(seconds):
    procs = target_procs()
    for p in procs:
        try:
            p.cpu_percent(None)
        except psutil.Error:
            pass
    time.sleep(seconds)
    total = 0.0
    for p in procs:
        try:
            total += p.cpu_percent(None)
        except psutil.Error:
            pass
    return total  # 百分比（psutil 已按单核归一，>100 = 多核）


async def main():
    env = dict(os.environ)
    env["WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS"] = f"--remote-debugging-port={PORT}"
    args = [EXE, f"--lanes={LANES}", f"--media={MEDIA}"]
    if PACE:
        args.append(f"--pace={PACE}")
    proc = subprocess.Popen(args,
                            env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        version = None
        for _ in range(60):
            try:
                version = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json/version", timeout=1))
                break
            except Exception:
                time.sleep(0.5)
        if not version:
            print("CDP 端口未起来")
            return
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=None) as ws:
            targets = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json", timeout=2))
            page = next(t for t in targets if t.get("type") == "page")
            session = (await send(ws, "Target.attachToTarget",
                                  {"targetId": page["id"], "flatten": True}))["result"]["sessionId"]
            await send(ws, "Runtime.enable", session=session)
            origin = None
            for _ in range(40):
                origin = await evaljs(ws, session, "location.origin")
                if origin and origin != "null":
                    break
                await asyncio.sleep(0.5)
            if not origin or origin == "null":
                origin = "http://tauri.localhost"
            print(f"页面 origin: {origin}")

            for ch in CHANNELS:
                await evaljs(ws, session, f"location.href = '{origin}/?lanes={LANES}&ch={ch}'")
                await asyncio.sleep(SETTLE_SECS)
                # CPU 采样与 fps 采样同窗口
                import threading
                cpu_box = {}
                t = threading.Thread(target=lambda: cpu_box.setdefault("v", cpu_percent(SAMPLE_SECS)), daemon=True)
                t.start()
                fps_reads = []
                for _ in range(SAMPLE_SECS):
                    await asyncio.sleep(1)
                    v = await evaljs(ws, session, "document.getElementById('stats').textContent")
                    if v:
                        fps_reads.append(v)
                t.join()
                print(f"\n== ch={ch} lanes={LANES} pace={PACE or '0(全速)'}  CPU={cpu_box.get('v', -1):.0f}% ==")
                for s in fps_reads:
                    print("   ", s)
                shot = os.path.join(ROOT, "out", f"shot-{ch}-{LANES}.png")
                os.makedirs(os.path.dirname(shot), exist_ok=True)
                png = (await send(ws, "Page.captureScreenshot", {"format": "png"}, session))
                data = png.get("result", {}).get("data")
                if data:
                    import base64
                    with open(shot, "wb") as f:
                        f.write(base64.b64decode(data))
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


asyncio.run(main())
