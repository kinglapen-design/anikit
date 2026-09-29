# ws 详情基础数据探针（2026-09-26_14 工作空间详情基础数据轮验收）：
# 桩 workspace_root / list_workspace_dir（图片 1920×1080 + 视频 640×360 缩略图尺寸两条目），
# 切工作空间分段 → 点卡片 → 断言右栏 WsDetail 字段：
#   图片 = 名称 / 位置（= 根路径）/ 尺寸 1920 x 1080（源像素）/ 文件大小 / 类型 PNG 文件 / 创建日期 / 修改日期；
#   视频 = 无「尺寸」行（w/h 是缩略图尺寸，不当分辨率展示）/ 类型 MP4 文件；
#   形态 = 无分段控制器 .seg-block、无信息查看管理钮 .manage-bar（不进三 tab 配置体系）。
import asyncio
import json
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9367
URL = "http://localhost:5373"

STUB = make_stub(
    {
        "get_current_library": '{ name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 0 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: false }',
        "workspace_root": '{ rootPath: "E:/ws-probe", exists: true }',
        "ws_list_pinned": "[]",
    },
    handlers="""
    if (cmd === "list_workspace_dir") return Promise.resolve([
      { name: "图片.png", relPath: "图片.png", isDir: false, size: 204800, mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: 1920, h: 1080, ext: "png", thumbPath: null, thumbKind: null },
      { name: "视频.mp4", relPath: "视频.mp4", isDir: false, size: 5242880, mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: 640, h: 360, ext: "mp4", thumbPath: null, thumbKind: null },
    ]);
""",
)

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


def dev_server_up() -> bool:
    try:
        urllib.request.urlopen(URL, timeout=1)
        return True
    except Exception:
        return False


DETAIL_DUMP = r"""(() => {
  const sb = document.querySelector('.sidebar-right');
  if (!sb) return { error: 'no sidebar-right' };
  const inputs = [...sb.querySelectorAll('.wd-inputs input')].map((i) => i.value);
  const rows = [...sb.querySelectorAll('.wd-data .bd-row')].map((r) => ({
    label: r.querySelector('.bd-label')?.textContent ?? '',
    value: [...r.querySelectorAll('.bd-value')].map((v) => v.textContent).join(' '),
  }));
  return {
    inputs, rows,
    hasSeg: !!sb.querySelector('.seg-block'),
    hasManage: !!sb.querySelector('.manage-bar'),
    hasPinned: !!sb.querySelector('.pinned-block, .sp-root'),
  };
})()"""


async def main() -> int:
    dev_proc = None
    if not dev_server_up():
        print("启动 pnpm dev ...")
        dev_proc = subprocess.Popen("pnpm dev", shell=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(60):
            if dev_server_up():
                break
            time.sleep(0.5)
        if not dev_server_up():
            print("dev server 启动超时")
            return 2
        print("dev server 就绪")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    try:
        version = None
        for _ in range(50):
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
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def click(expr):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: Math.min(b.left + b.width / 2, window.innerWidth - 8), y: Math.min(b.top + b.height / 2, window.innerHeight - 8) }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": r["x"], "y": r["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.6)
                return True

            # 切工作空间分段（资源库 / 工作空间 / AI 三段之第二段）
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][1]")
            await asyncio.sleep(1.0)
            diag = await ev(r"""(() => ({
              segOn: [...document.querySelectorAll('.sidebar-left .seg .seg-item')].findIndex((el) => el.classList.contains('on')),
              wsTree: !!document.querySelector('.ws-tree'),
              wsEmpty: document.querySelector('.ws-empty')?.textContent ?? null,
              tabs: [...document.querySelectorAll('.tab-label, .dv-tab')].map((t) => t.textContent?.trim()).slice(0, 8),
              cards: document.querySelectorAll('.asset-card').length,
              bodySnippet: document.body.innerText.replace(/\n+/g, '|').slice(0, 300),
            }))()""")
            print("诊断:", json.dumps(diag, ensure_ascii=False))
            # 等 ws 根目录卡片出现
            for _ in range(20):
                n = await ev("document.querySelectorAll('.asset-card').length")
                if (n or 0) >= 2:
                    break
                await asyncio.sleep(0.5)
            n = await ev("document.querySelectorAll('.asset-card').length")
            print(f"ws 根视口卡片数: {n}")

            fails = []

            def check(name, cond, got=None):
                print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"（got={got!r}）"))
                if not cond:
                    fails.append(name)

            # —— 图片条目详情 ——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('图片'))")
            await asyncio.sleep(0.8)
            d1 = await ev(DETAIL_DUMP)
            print("图片详情:", json.dumps(d1, ensure_ascii=False))
            labels1 = [r["label"] for r in d1.get("rows", [])]
            row1 = {r["label"]: r["value"] for r in d1.get("rows", [])}
            check("图片·名称框含文件名", "图片" in (d1.get("inputs") or [""])[0], d1.get("inputs"))
            check("图片·位置框 = 工作空间根路径", (d1.get("inputs") or ["", ""])[1] == "E:/ws-probe", d1.get("inputs"))
            check("图片·尺寸 = 1920 x 1080（源像素）", row1.get("尺寸") == "1920 x 1080", row1.get("尺寸"))
            check("图片·类型 = PNG 文件", row1.get("类型") == "PNG 文件", row1.get("类型"))
            check("图片·文件大小行在", "文件大小" in labels1, labels1)
            check("图片·创建日期行在", "创建日期" in labels1, labels1)
            check("图片·修改日期行在", "修改日期" in labels1, labels1)
            check("图片·无分段控制器", d1.get("hasSeg") is False, d1.get("hasSeg"))
            check("图片·无信息查看管理钮", d1.get("hasManage") is False, d1.get("hasManage"))

            # —— 视频条目详情（w/h = 缩略图尺寸，不得作分辨率展示）——
            await click("[...document.querySelectorAll('.asset-card')].find((c) => c.textContent.includes('视频'))")
            await asyncio.sleep(0.8)
            d2 = await ev(DETAIL_DUMP)
            print("视频详情:", json.dumps(d2, ensure_ascii=False))
            labels2 = [r["label"] for r in d2.get("rows", [])]
            row2 = {r["label"]: r["value"] for r in d2.get("rows", [])}
            check("视频·名称框含文件名", "视频" in (d2.get("inputs") or [""])[0], d2.get("inputs"))
            check("视频·无尺寸行（缩略图尺寸不冒充分辨率）", "尺寸" not in labels2, labels2)
            check("视频·类型 = MP4 文件", row2.get("类型") == "MP4 文件", row2.get("类型"))
            check("视频·修改日期行在", "修改日期" in labels2, labels2)

            print("=" * 30)
            print("RESULT:", "ALL PASS" if not fails else f"{len(fails)} FAIL: {fails}")
            return 0 if not fails else 1
    finally:
        proc.terminate()
        if dev_proc:
            subprocess.run(["taskkill", "/F", "/T", "/PID", str(dev_proc.pid)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


sys.exit(asyncio.run(main()))
