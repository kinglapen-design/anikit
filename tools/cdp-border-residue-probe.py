# CDP 探针：品牌色描边残留复现（2026-08-19 用户报：卡片/输入框 聚焦/选中/右键选中取消后
# 残留品牌色像素点，从开发到现在反复出现）
# 方法：稳定态基线截图 → 循环「hover → 选中 → Esc 取消 → 鼠标移空白」→ 每次取消后截屏
# 与基线逐像素 diff；diff 中接近品牌边框色 #3DDDD2 的像素 = 残留。
# 用法：tools/.venv/Scripts/python.exe tools/cdp-border-residue-probe.py [--window]
#   --window = 非 headless（真窗口 GPU 渲染路径——残留是合成器 artifact，headless 软渲染复现不了，
#   实机回归用它；会在屏幕上开窗跑约 2 分钟）
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request
from io import BytesIO

import websockets
from PIL import Image

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9360
URL = "http://localhost:5373"
ROUNDS = 30
BRAND = (0x3D, 0xDD, 0xD2)  # --brand-border
TOL = 40  # 品牌色判定容差（抗锯齿像素偏色）
WINDOWED = "--window" in sys.argv  # 非 headless = 真 GPU 合成路径

STUB = r"""
const TREE = {
  blocks: [{ id: "dir", name: "目录", icon: null }],
  nodes: [
    { id: "f1", name: "角色", count: 4, children: [] },
    { id: "f2", name: "场景", count: 0, children: [] },
  ],
};
const mk = (assetId, name) => ({
  assetId, folderId: "f1", name, w: 800, h: 600, ext: "png", thumbPath: null, thumbKind: null,
  origin: true, frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
  size: 100, createdAt: "2026-08-01T10:00:00+08:00",
  modifiedAt: "2026-08-10T10:00:00+08:00", importedAt: "2026-08-11T10:00:00+08:00", noteCount: null,
});
const PLACEMENTS = [mk("AN-1", "图1.png"), mk("AN-2", "图2.png"), mk("AN-3", "图3.png"), mk("AN-4", "图4.png")];
window.__cbId = 0; window.__cbs = {};
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const R = {
      get_current_library: { name: "测试库", path: "E:/test-lib", assetCount: 4, folderCount: 2 },
      get_ui_settings: {},
      list_folder_tree: JSON.parse(JSON.stringify(TREE)),
      list_placements: JSON.parse(JSON.stringify(PLACEMENTS)),
      list_recycle: [], list_recent_assets: [], list_recent_libraries: [],
      list_inbox_snapshot: [], list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      undo: null, redo: null,
    };
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
};
"""

_msg_id = 0


async def send(ws, method, params=None, session=None):
    global _msg_id
    _msg_id += 1
    payload = {"id": _msg_id, "method": method, "params": params or {}}
    if session:
        payload["sessionId"] = session
    await ws.send(json.dumps(payload))
    while True:
        raw = json.loads(await ws.recv())
        if raw.get("id") == _msg_id:
            return raw


def brand_pixels(img: Image.Image):
    """接近品牌色的像素坐标集"""
    px = img.convert("RGB").load()
    out = set()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b = px[x, y]
            if abs(r - BRAND[0]) <= TOL and abs(g - BRAND[1]) <= TOL and abs(b - BRAND[2]) <= TOL:
                out.add((x, y))
    return out


def kill_listening_port(port):
    """按端口找 LISTENING PID 强杀（2026-08-19 踩坑：vite node 进程脱离 pnpm 壳的进程树，
    taskkill /T 杀壳后 5373 仍被占——端口直杀兜底）"""
    try:
        out = subprocess.run(["netstat", "-ano"], capture_output=True, text=True).stdout
        for line in out.splitlines():
            if f":{port}" in line and "LISTENING" in line:
                subprocess.run(["taskkill", "/PID", line.split()[-1], "/F"], capture_output=True)
    except Exception:
        pass


async def main():
    spawned = None
    try:
        urllib.request.urlopen(URL, timeout=1)
    except Exception:
        spawned = subprocess.Popen(["pnpm", "dev"], shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            try:
                urllib.request.urlopen(URL, timeout=1)
                break
            except Exception:
                time.sleep(0.5)

    import tempfile
    prof = tempfile.mkdtemp(prefix="anikit-probe-edge-")  # 隔离用户 Edge profile（否则复用已开实例）
    edge_args = [
        EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={prof}",
        "--window-size=1440,900", "--no-first-run", "about:blank",
    ]
    if not WINDOWED:
        edge_args[2:2] = ["--headless=new", "--disable-gpu"]
    edge = subprocess.Popen(edge_args, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    try:
        version = None
        for _ in range(30):
            try:
                version = json.load(urllib.request.urlopen(f"http://localhost:{PORT}/json/version", timeout=1))
                break
            except Exception:
                time.sleep(0.3)
        if not version:
            print("CDP 端口不可达")
            return 2
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=50 * 1024 * 1024) as ws:
            target = await send(ws, "Target.createTarget", {"url": "about:blank"})
            tid = target["result"]["targetId"]
            sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
            await send(ws, "Runtime.enable", session=sess)
            await send(ws, "Page.enable", session=sess)
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": STUB}, session=sess)
            # 分数物理像素复现残留：125% 缩放下 1.5px CSS = 1.875 物理像素（用户实机常见缩放档）
            await send(ws, "Emulation.setDeviceMetricsOverride", {
                "width": 1440, "height": 900, "deviceScaleFactor": 1.25, "mobile": False,
            }, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def mouse(t, x, y, buttons, button="left", clicks=1):
                await send(ws, "Input.dispatchMouseEvent", {
                    "type": t, "button": button, "buttons": buttons,
                    "x": x, "y": y, "clickCount": clicks,
                }, session=sess)

            async def shot() -> Image.Image:
                s = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                return Image.open(BytesIO(base64.b64decode(s["result"]["data"])))

            # 进 f1
            for _ in range(30):
                n = await ev("document.querySelectorAll('.tree-item[data-folder-id]').length")
                if n and n >= 2:
                    break
                await asyncio.sleep(0.5)
            f1 = await ev("""
              (() => {
                const el = document.querySelector('.tree-item[data-folder-id="f1"]');
                const r = el.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            await mouse("mousePressed", f1["x"], f1["y"], 1)
            await mouse("mouseReleased", f1["x"], f1["y"], 0)
            await asyncio.sleep(0.8)

            card = await ev("""
              (() => {
                const el = document.querySelector('.asset-card[data-key="AN-1|f1"]');
                const r = el.getBoundingClientRect();
                return { x: Math.round(r.left + r.width / 2), y: Math.round(r.top + r.height / 2) };
              })()
            """)
            # 空白点（卡片下方内容区）
            blank = {"x": card["x"], "y": 850}

            # 鼠标移空白，等稳定，取基线
            await mouse("mouseMoved", blank["x"], blank["y"], 0)
            await asyncio.sleep(0.8)
            base = await shot()
            base_brand = brand_pixels(base)
            print(f"基线品牌色像素数：{len(base_brand)}（含原始文件徽章等恒在元素）")

            worst = 0
            worst_round = -1
            for i in range(ROUNDS):
                # hover 卡片
                await mouse("mouseMoved", card["x"], card["y"], 0)
                await asyncio.sleep(0.15)
                # 选中
                await mouse("mousePressed", card["x"], card["y"], 1)
                await mouse("mouseReleased", card["x"], card["y"], 0)
                await asyncio.sleep(0.15)
                # Esc 取消选中
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "Escape", "code": "Escape", "windowsVirtualKeyCode": 27}, session=sess)
                # 鼠标移空白（去 hover）
                await mouse("mouseMoved", blank["x"], blank["y"], 0)
                await asyncio.sleep(0.25)  # 等 transition 结束 + 一帧
                img = await shot()
                cur = brand_pixels(img)
                new_px = cur - base_brand
                if len(new_px) > worst:
                    worst = len(new_px)
                    worst_round = i
                    img.save("tools/style-preview/residue-worst.png")
                if new_px:
                    sample = sorted(new_px)[:6]
                    print(f"第 {i+1} 轮：新增品牌色像素 {len(new_px)} 个，样本 {sample}")

            print()
            if worst > 0:
                print(f"复现：第 {worst_round+1} 轮残留 {worst} 个品牌色像素（residue-worst.png 留档）")
                return 1
            print(f"{ROUNDS} 轮未见残留（基线对比法）")
            return 0
    finally:
        edge.terminate()
        if spawned:
            # Windows：shell=True 的 pnpm 壳 terminate 只杀壳、node/vite 子树存活（2026-08-19 踩坑）；
            # taskkill /T 杀树 + 按端口直杀兜底（vite node 会脱离 cmd 壳进程树，实测 /T 后 5373 仍占）
            subprocess.run(["taskkill", "/PID", str(spawned.pid), "/T", "/F"], capture_output=True)
            kill_listening_port(5373)


sys.exit(asyncio.run(main()))
