# CDP 实测：工作空间搜索前端链路（2026-10-03 双层架构落地轮）
# 桩：everything_status 可翻转（window.__evDown）、ws_search 记录调用并按桩命中、
#     ws_search_crawl_start 返 taskId=7、爬盘事件经 plugin:event|listen 捕获的 handler 手动推拍、
#     open_external_url / launch_everything / ws_search_crawl_cancel 记录调用。
# 验收：
#   S1 库视口有筛选钮 ↔ ws 视口筛选钮整颗不渲染（filters 传空）；
#   S2 ws 输入关键字 → ws_search 调用（query 剥前缀 / viewRel 正确）+ 命中卡渲染 + 标题计数；
#   S3 草稿加 ~ → 再搜（query 带 ~ 原样透传，后端换算全局）；
#   S4 Everything 翻 down → 输入不再出结果 + 空态 Enter 引导文案（ws_search 回 everythingUnavailable 自愈翻态）；
#   S5 Enter → crawl_start 调用 → 推拍：进度条「已扫 N」→ 命中卡增量 → done 进度条消失卡保留；
#   S6 ~ + down → toast「下载 Everything」，点动作 → open_external_url voidtools；
#   S7 Esc 清空 → 退出搜索态、文件夹区恢复。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

from _probe_stub import make_stub

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9375
URL = "http://localhost:5373"
SHOT = "tools/ws-search-probe.png"


def _file(rel: str, name: str) -> str:
    return (
        f'{{ name: "{name}", relPath: "{rel}", isDir: false, hasSubDir: false, isLink: false, linkOk: false,'
        f' size: 2048, mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: 100, h: 100, ext: "png",'
        ' thumbPath: null, thumbKind: null }'
    )


def _dir(n: str) -> str:
    return (
        f'{{ name: "{n}", relPath: "{n}", isDir: true, hasSubDir: false, isLink: false, linkOk: false,'
        ' size: 0, mtimeMs: 1758000000000, ctimeMs: 1757000000000, w: null, h: null, ext: "", thumbPath: null, thumbKind: null }'
    )


STUB = make_stub(
    {
        "get_current_library": '{ name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 0 }',
        "get_current_space": '{ name: "S", path: "E:/T/S", autoImport: false }',
        "workspace_root": '{ rootPath: "E:/ws-probe", exists: true }',
        "ws_list_pinned": "[]",
        "ws_dirs_exist": "[]",
    },
    handlers=f"""
    if (cmd === "plugin:event|listen") {{ (window.__evL ??= {{}})[args?.event] = args?.handler; return Promise.resolve(1); }}
    if (cmd === "everything_status") {{ return Promise.resolve(window.__evDown ? {{ status: "notInstalled", version: null }} : {{ status: "running", version: "1.4.1.1030" }}); }}
    if (cmd === "ws_search") {{ window.__calls.ws_search.push({{ query: args?.query, viewRel: args?.viewRel }});
      return Promise.resolve(window.__evDown ? {{ status: "everythingUnavailable", entries: [] }}
        : {{ status: "ok", entries: [{_file("分镜/镜头01.png", "镜头01.png")}, {_file("分镜/镜头02.png", "镜头02.png")}] }}); }}
    if (cmd === "ws_search_crawl_start") {{ window.__calls.crawl_start.push({{ query: args?.query, viewRel: args?.viewRel }}); return Promise.resolve(7); }}
    if (cmd === "ws_search_crawl_cancel") {{ window.__calls.crawl_cancel.push(args?.taskId); return Promise.resolve(null); }}
    if (cmd === "open_external_url") {{ window.__calls.open_url.push(args?.url); return Promise.resolve(null); }}
    if (cmd === "launch_everything") {{ window.__calls.launch_ev.push(1); return Promise.resolve(null); }}
    if (cmd === "list_workspace_dir") {{ return Promise.resolve([{_dir("分镜")}, {_dir("资料")},
      {_file("说明.txt", "说明.txt")}]); }}
""",
    head="window.__calls = { ws_search: [], crawl_start: [], crawl_cancel: [], open_url: [], launch_ev: [] };",
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


# 搜索框输入（v-model：赋值 + input 事件）
TYPE = """((t) => { const el = document.querySelector('.fbs-input'); if (!el) return false;
  el.focus(); el.value = t; el.dispatchEvent(new Event('input', { bubbles: true })); return true; })(%s)"""
ENTER = "document.querySelector('.fbs-input')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))"
ESC_INPUT = "document.querySelector('.fbs-input')?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))"
CTRL_F = "document.dispatchEvent(new KeyboardEvent('keydown', { key: 'f', ctrlKey: true, bubbles: true }))"
FILTER_BTN = "!!document.querySelector('.fbs-ic.filter')"
CARD_COUNT = "document.querySelectorAll('.asset-card').length"
RESULT_COUNT = "document.querySelector('.result-count')?.textContent ?? ''"
EMPTY_TEXT = "document.querySelector('.search-empty')?.textContent ?? ''"
PROGRESS = "document.querySelector('.import-progress .label')?.textContent ?? ''"
TOAST = """(() => { const t = document.querySelector('.toast'); return t ? {
  msg: t.querySelector('.message')?.textContent ?? '', act: t.querySelector('.action')?.textContent ?? '' } : null; })()"""
# 推一拍爬盘事件（handler = listen 时 transformCallback 登记的回调 id）
CRAWL_EVT = """((p) => { const h = window.__evL?.['ws-search-crawl']; if (h == null) return false;
  window.__cbs[h]({ event: 'ws-search-crawl', payload: p }); return true; })(%s)"""


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
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1600,1200", "about:blank"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
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
            await asyncio.sleep(7)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                if "exceptionDetails" in r.get("result", {}):
                    print("!! JS 异常:", json.dumps(r["result"]["exceptionDetails"], ensure_ascii=False)[:300])
                return r.get("result", {}).get("result", {}).get("value")

            async def click(expr, button="left"):
                r = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not r:
                    print("!! 元素不存在:", expr[:60])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(
                        ws,
                        "Input.dispatchMouseEvent",
                        {"type": t, "button": button, "modifiers": 0, "buttons": 0 if t == "mouseReleased" else 1, "x": r["x"], "y": r["y"], "clickCount": 1},
                        session=sess,
                    )
                await asyncio.sleep(0.6)
                return True

            fails = []

            def check(name, cond, got=None):
                print(("PASS " if cond else "FAIL ") + name + ("" if cond else f"（got={got!r}）"))
                if not cond:
                    fails.append(name)

            # —— S1 库视口对照：筛选钮在 ——
            await asyncio.sleep(1)
            check("S1a 库视口 = 筛选钮渲染", await ev(FILTER_BTN) is True)

            # 切工作空间分段
            await click("[...document.querySelectorAll('.sidebar-left .seg .seg-item')][1]")
            await asyncio.sleep(1.2)
            check("S1b ws 视口 = 筛选钮不渲染", await ev(FILTER_BTN) is False)

            # —— S2 ws 输入关键字 → Everything 实时搜 ——
            check("S2a 搜索框就绪", await ev(TYPE % json.dumps("镜头")) is True)
            await asyncio.sleep(1.0)  # 150ms 防抖 + invoke
            calls = await ev("window.__calls.ws_search")
            check("S2b ws_search 调用（query=镜头 / viewRel=''）", len(calls or []) >= 1 and calls[-1] == {"query": "镜头", "viewRel": ""}, calls)
            await asyncio.sleep(0.8)
            check("S2c 命中卡渲染 = 2", await ev(CARD_COUNT) == 2, await ev(CARD_COUNT))
            check("S2d 标题计数 = 2", await ev(RESULT_COUNT) == "2", await ev(RESULT_COUNT))

            # —— S3 ~ 全局 ——
            await ev(TYPE % json.dumps("~镜头"))
            await asyncio.sleep(1.0)
            calls = await ev("window.__calls.ws_search")
            check("S3 ~ 原样透传后端（query='~镜头'）", (calls or [])[-1:] == [{"query": "~镜头", "viewRel": ""}], calls)

            # —— S4 Everything 翻 down ——
            await ev("window.__evDown = true")
            await ev(ESC_INPUT)  # 清空退出搜索态
            await asyncio.sleep(3.4)  # 等能力缓存 TTL 过期（3s）
            await ev(TYPE % json.dumps("镜头"))
            await asyncio.sleep(1.0)
            check("S4a 空态 = Enter 引导文案", "按 Enter" in (await ev(EMPTY_TEXT) or ""), await ev(EMPTY_TEXT))
            shot = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
            with open(SHOT, "wb") as f:
                f.write(base64.b64decode(shot["result"]["data"]))
            print("截图:", SHOT)

            # —— S5 Enter 爬盘 ——
            await ev(ENTER)
            await asyncio.sleep(0.8)
            calls = await ev("window.__calls.crawl_start")
            check("S5a Enter → crawl_start（query=镜头 / viewRel=''）", (calls or [])[-1:] == [{"query": "镜头", "viewRel": ""}], calls)
            ok = await ev(CRAWL_EVT % json.dumps({"taskId": 7, "scanned": 1200, "hits": [], "done": False, "cancelled": False}))
            await asyncio.sleep(0.5)
            check("S5b 进度条 = 已扫 1,200", ok is True and "1,200" in (await ev(PROGRESS) or ""), await ev(PROGRESS))
            hit = {"name": "镜头爬01.png", "relPath": "分镜/镜头爬01.png", "isDir": False, "hasSubDir": False, "isLink": False, "linkOk": False,
                   "size": 64, "mtimeMs": 1758000000000, "ctimeMs": 0, "w": 100, "h": 100, "ext": "png", "thumbPath": None, "thumbKind": None}
            await ev(CRAWL_EVT % json.dumps({"taskId": 7, "scanned": 2400, "hits": [hit], "done": False, "cancelled": False}))
            await asyncio.sleep(0.8)
            check("S5c 命中批次流入 = 1 卡", await ev(CARD_COUNT) == 1, await ev(CARD_COUNT))
            await ev(CRAWL_EVT % json.dumps({"taskId": 7, "scanned": 3000, "hits": [], "done": True, "cancelled": False}))
            await asyncio.sleep(0.6)
            check("S5d done → 进度条消失 + 命中保留", (await ev(PROGRESS)) == "" and await ev(CARD_COUNT) == 1)

            # —— S6 ~ + down → 通知栏指路 ——
            await ev(TYPE % json.dumps("~镜头"))
            await asyncio.sleep(1.0)
            toast = await ev(TOAST)
            check("S6a toast = 全局需 Everything + 下载动作", toast is not None and "全局搜索" in toast["msg"] and toast["act"] == "下载 Everything", toast)
            await click("document.querySelector('.toast .action')")
            urls = await ev("window.__calls.open_url")
            check("S6b 动作钮 → open_external_url(voidtools)", (urls or [])[-1:] == ["https://www.voidtools.com/"], urls)

            # —— S7 Esc 清空恢复 ——
            await ev(ESC_INPUT)
            await asyncio.sleep(0.8)
            check("S7 Esc 清空 → 退出搜索态（计数消失 + 空态消失）", (await ev(RESULT_COUNT)) == "" and (await ev(EMPTY_TEXT)) == "")

            print("FAIL 清单:", fails if fails else "无")
            return 1 if fails else 0
    finally:
        proc.terminate()
        if dev_proc is not None:
            dev_proc.terminate()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
