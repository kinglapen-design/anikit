"""真实 Tauri 隔离验收：收件箱 Ctrl/Shift 多选并一次批量导入 10 个 doc。"""
import asyncio
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import traceback
import urllib.request
import uuid

import websockets

ROOT = Path(__file__).resolve().parent.parent
EXE = Path(os.environ.get("ANIKIT_INBOX_EXE", str(ROOT / "src-tauri" / "target" / "release" / "app.exe")))
BASE = ROOT / "tools" / "style-preview" / "inbox-batch-e2e"
PORT = 19349
_msg_id = 0


def log(text):
    print(text, flush=True)


def require(value, message):
    if not value:
        raise AssertionError(message)


def js(value):
    return json.dumps(value, ensure_ascii=False)


async def send(ws, method, params=None, session=None, timeout=120):
    global _msg_id
    _msg_id += 1
    mid = _msg_id
    payload = {"id": mid, "method": method, "params": params or {}}
    if session:
        payload["sessionId"] = session
    await ws.send(json.dumps(payload, ensure_ascii=False))
    while True:
        raw = json.loads(await asyncio.wait_for(ws.recv(), timeout))
        if raw.get("id") == mid:
            if "error" in raw:
                raise RuntimeError(f"{method}: {raw['error']}")
            return raw.get("result", raw)


async def evaluate(ws, session, expression):
    result = await send(ws, "Runtime.evaluate", {
        "expression": expression, "returnByValue": True, "awaitPromise": True,
    }, session)
    if "exceptionDetails" in result:
        raise RuntimeError(str(result["exceptionDetails"])[:2000])
    return result.get("result", {}).get("value")


async def invoke(ws, session, command, arguments=None):
    args = js(arguments or {})
    return await evaluate(ws, session, f"window.__TAURI_INTERNALS__.invoke({js(command)}, {args})")


async def wait_value(ws, session, expression, label, timeout=90):
    started = time.monotonic()
    last = None
    while time.monotonic() - started < timeout:
        try:
            last = await evaluate(ws, session, expression)
            if last:
                return last
        except RuntimeError:
            pass
        await asyncio.sleep(0.2)
    raise AssertionError(f"等待超时：{label}；最后状态={last}")


async def click(ws, session, expression, button="left", modifiers=0):
    rect = await evaluate(ws, session, f"""(() => {{
      const el = {expression};
      if (!el) return null;
      el.scrollIntoView({{ block: 'center' }});
      const r = el.getBoundingClientRect();
      return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }};
    }})()""")
    require(rect, f"找不到点击目标：{expression}")
    await send(ws, "Input.dispatchMouseEvent", {
        "type": "mouseMoved", "x": rect["x"], "y": rect["y"], "button": "none", "buttons": 0,
    }, session)
    buttons = 2 if button == "right" else 1
    await send(ws, "Input.dispatchMouseEvent", {
        "type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": button,
        "buttons": buttons, "clickCount": 1, "modifiers": modifiers,
    }, session)
    await send(ws, "Input.dispatchMouseEvent", {
        "type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": button,
        "buttons": 0, "clickCount": 1, "modifiers": modifiers,
    }, session)


def text_element(selector, text):
    return f"[...document.querySelectorAll({js(selector)})].find(el => el.textContent.trim() === {js(text)})"


async def main():
    require(EXE.is_file(), f"真实程序不存在：{EXE}")
    run = BASE / "runs" / uuid.uuid4().hex
    config = BASE / f"config-{uuid.uuid4().hex}"
    run.mkdir(parents=True)
    config.mkdir(parents=True)
    env = os.environ.copy()
    env.update({
        "ANIKIT_E2E": "1",
        "ANIKIT_CONFIG_DIR": str(config),
        "WEBVIEW2_USER_DATA_FOLDER": str(config / "webview"),
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS": f"--remote-debugging-port={PORT}",
    })
    proc = subprocess.Popen([str(EXE)], cwd=ROOT, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    drive_letter = None
    try:
        version = None
        for _ in range(150):
            try:
                version = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json/version", timeout=1))
                break
            except Exception:
                await asyncio.sleep(0.2)
        require(version, "隔离 WebView2 CDP 端口不可达")
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=32 * 1024 * 1024) as ws:
            page = None
            for _ in range(150):
                pages = json.load(urllib.request.urlopen(f"http://127.0.0.1:{PORT}/json", timeout=2))
                page = next((item for item in pages if item.get("type") == "page"), None)
                if page:
                    break
                await asyncio.sleep(0.2)
            require(page, "找不到真实 AniKit 页面")
            attached = await send(ws, "Target.attachToTarget", {"targetId": page["id"], "flatten": True})
            session = attached["sessionId"]
            await send(ws, "Runtime.enable", session=session)
            await send(ws, "Page.enable", session=session)
            mounted = "!!document.querySelector('#app .window') && !!window.__TAURI_INTERNALS__"
            await wait_value(ws, session, mounted, "真实应用挂载")

            space = await invoke(ws, session, "create_space", {
                "parentPath": str(run), "name": "批量验收空间", "autoImport": False,
            })
            library = await invoke(ws, session, "create_library", {"name": "批量验收库"})
            require(Path(space["path"]).resolve().is_relative_to(run), "空间越出隔离目录")
            require(Path(library["path"]).resolve().is_relative_to(run), "素材库越出隔离目录")
            inbox = Path(space["path"]) / "自动导入"
            names = [f"批量-{index:02d}.doc" for index in range(1, 11)]
            for index, name in enumerate(names, 1):
                (inbox / name).write_text(f"隔离验收文档 {index}\n", encoding="utf-8")

            await send(ws, "Page.reload", {"ignoreCache": True}, session)
            await asyncio.sleep(1)
            await wait_value(ws, session, mounted, "隔离库自动重开")
            current = await invoke(ws, session, "get_current_library")
            require(current and Path(current["path"]).resolve() == Path(library["path"]).resolve(), "重开后未停留在隔离库")
            await click(ws, session, text_element(".quick *", "自动导入"))
            enable = text_element(".kit-modal .btn.confirm", "开启")
            if await evaluate(ws, session, f"!!{enable}"):
                await click(ws, session, enable)
                await wait_value(ws, session, "!document.querySelector('.kit-modal')", "开启自动导入")
            drive_letter = await invoke(ws, session, "get_inbox_drive_letter")
            await wait_value(ws, session, "document.querySelectorAll('.disk-card').length === 10", "10 个工程文件卡片")

            await click(ws, session, "document.querySelectorAll('.disk-card')[0]")
            await click(ws, session, "document.querySelectorAll('.disk-card')[2]", modifiers=2)
            await wait_value(ws, session, "document.querySelectorAll('.disk-card .fc-root.selected').length === 2",
                             "Ctrl 逐项多选", 10)
            await click(ws, session, "document.querySelectorAll('.disk-card')[0]")
            await click(ws, session, "document.querySelectorAll('.disk-card')[9]", modifiers=8)
            await wait_value(ws, session, "document.querySelectorAll('.disk-card .fc-root.selected').length === 10",
                             "Shift 范围选择 10 项", 10)

            await click(ws, session, "document.querySelectorAll('.disk-card')[5]", button="right")
            await wait_value(ws, session, f"!!{text_element('.menu-item', '导入 10 项')}", "批量导入菜单")
            require(await evaluate(ws, session, "document.querySelectorAll('.disk-card .fc-root.selected').length") == 10,
                    "右键已选项丢失了多选")
            await click(ws, session, text_element(".menu-item", "导入 10 项"))
            await wait_value(ws, session, "!!document.querySelector('.kit-modal .btn.confirm')", "批量导入确认窗")
            body = await evaluate(ws, session, "document.querySelector('.kit-modal .body')?.textContent ?? ''")
            require("10 个条目" in body and "原位置不再保留" in body, f"移走警示文案不完整：{body}")
            require(await evaluate(ws, session, "document.querySelector('.kit-modal .btn.confirm')?.textContent.trim()") == "导入 10 项",
                    "确认钮未带批量数量")
            await click(ws, session, "document.querySelector('.kit-modal .btn.confirm')")

            placements = await wait_value(ws, session, """window.__TAURI_INTERNALS__.invoke('list_placements', {})
              .then(items => items.length === 10 ? items : null)""", "10 个 doc 一次入库", 120)
            require(all(item["ext"].lower() == "doc" and item.get("importedFrom") == "inbox" for item in placements),
                    f"入库格式或渠道标记错误：{placements}")
            require({item["name"] for item in placements} == set(names), f"入库名称不完整：{[item['name'] for item in placements]}")
            await wait_value(ws, session, "window.__TAURI_INTERNALS__.invoke('list_inbox_snapshot', {}).then(items => items.length === 0)",
                             "收件箱已移空", 60)
            require(all(not (inbox / name).exists() for name in names), "仍有源文件留在收件箱")
            await wait_value(ws, session, "document.querySelectorAll('.disk-card').length === 0", "工程文件区块清空")
            require(await evaluate(ws, session, "document.querySelectorAll('.fc-root.selected').length") == 0,
                    "导入完成后选择未清理")
            log("[PASS] Ctrl 多选、Shift 10 项、一次确认、10 doc 入库、源文件移走与选择清理全部通过")
    finally:
        if drive_letter:
            subprocess.run(["subst", drive_letter, "/D"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        subprocess.run(["taskkill", "/PID", str(proc.pid), "/T", "/F"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            pass
        for _ in range(20):
            subprocess.run(["cmd", "/c", "rd", "/s", "/q", str(BASE)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            if not BASE.exists():
                break
            time.sleep(0.25)
        require(not BASE.exists(), f"临时验收目录清理失败：{BASE}")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except Exception as error:
        log(f"[FAIL] {type(error).__name__}: {error}")
        traceback.print_exc()
        sys.exit(1)
