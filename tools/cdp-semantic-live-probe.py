"""真实 Tauri WebView2 + 临时配置/素材库的语义搜索、预览定位与 MCP 验收。"""
import asyncio
import json
import os
import sys
import time
import traceback
import urllib.request
import uuid
from pathlib import Path

import websockets
import semantic_segment_live_probe

ROOT = Path(__file__).resolve().parent.parent
CDP_PORT = int(os.environ.get("ANIKIT_CDP_PORT", "19333"))
_msg_id = 0
TEST_SPACE = None
TEST_LIBRARY = None
PANEL_ID = None
ASSETS = {}
HITS = {}
BASE = ROOT / "tools" / "style-preview" / "semantic-e2e"


def log(text):
    print(text, flush=True)


def require(value, message):
    if not value:
        raise AssertionError(message)


def js_string(value):
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


async def evaluate(ws, session, expression, debug=False):
    result = await send(ws, "Runtime.evaluate", {
        "expression": expression, "returnByValue": True, "awaitPromise": True,
    }, session)
    if debug:
        log(f"[CDP] {result}")
    if "exceptionDetails" in result:
        raise RuntimeError(str(result["exceptionDetails"])[:2000])
    return result.get("result", {}).get("value")


async def invoke(ws, session, command, arguments=None):
    args = json.dumps(arguments or {}, ensure_ascii=False)
    return await evaluate(ws, session, f"window.__TAURI_INTERNALS__.invoke({js_string(command)}, {args})")


async def wait_value(ws, session, expression, label, timeout=120):
    last = None
    started = time.monotonic()
    while time.monotonic() - started < timeout:
        last = await evaluate(ws, session, expression, debug=label == "真实应用挂载")
        if last:
            return last
        await asyncio.sleep(0.2)
    raise AssertionError(f"等待超时：{label}；最后状态={last}")


async def wait_task(ws, session, task_id, wanted="done"):
    wanted_text = js_string(wanted)
    expression = f"""(async () => {{
      const s = await window.__TAURI_INTERNALS__.invoke('get_semantic_analysis_task', {{ taskId: {js_string(task_id)} }});
      if (!s) return null;
      if (s.status === 'failed') throw new Error(JSON.stringify(s));
      return s.status === {wanted_text} ? s : null;
    }})()"""
    return await wait_value(ws, session, expression, f"任务 {task_id} -> {wanted}", 180)


async def click(ws, session, selector, count=1):
    rect = await evaluate(ws, session, f"""(() => {{
      const el = document.querySelector({js_string(selector)});
      if (!el) return null;
      el.scrollIntoView({{ block: 'center' }});
      const r = el.getBoundingClientRect();
      return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }};
    }})()""")
    require(rect, f"找不到点击目标：{selector}")
    await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": rect["x"], "y": rect["y"], "button": "none", "buttons": 0}, session)
    await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": "left", "buttons": 1, "clickCount": count}, session)
    await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": "left", "buttons": 0, "clickCount": count}, session)


async def create_library(ws, session):
    global TEST_SPACE, TEST_LIBRARY
    space = await invoke(ws, session, "get_current_space")
    library = await invoke(ws, session, "get_current_library")
    if not space or not library or not Path(library["path"]).resolve().is_relative_to(BASE):
        spaces = sorted((BASE / "runs").glob("*/语义验收空间/space.json"), key=lambda path: path.stat().st_mtime, reverse=True)
        require(spaces, "隔离库不存在，拒绝使用其他库")
        TEST_SPACE = (await invoke(ws, session, "open_space", {"path": str(spaces[0].parent)}))["path"]
        TEST_LIBRARY = (await invoke(ws, session, "open_library", {"name": "语义验收库"}))["path"]
    else:
        TEST_SPACE, TEST_LIBRARY = space["path"], library["path"]
    require(Path(TEST_LIBRARY).resolve().is_relative_to(BASE), f"库越出隔离目录：{TEST_LIBRARY}")
    await wait_value(ws, session, "!document.querySelector('.wizard-mask')", "隔离库界面就绪", 30)
    if await evaluate(ws, session, "!!document.querySelector('.preview-root')"):
        await click(ws, session, ".preview-titlebar .nav-group button")
        await wait_value(ws, session, "!document.querySelector('.preview-root')", "复位预览状态", 30)
    log(f"[S0] 复用隔离库：{TEST_LIBRARY}")


async def import_media(ws, session):
    placements = await invoke(ws, session, "list_placements")
    if not placements:
        await evaluate(ws, session, "window.__importEvents = []; true")
        source = [ROOT / "spike/decode-proto/testdata/big.mp4", ROOT / "tools/test-assets/anim3.gif"]
        source += [ROOT / "tools/test-assets" / f"blank-test-{index}.png" for index in [1, 2, 3]]
        args = {
            "paths": [str(path) for path in source], "structure": "flatten", "source": "copy",
            "targetFolder": None, "sequenceMode": "pack",
        }
        await evaluate(ws, session, f"""(async () => {{
          const {{ Channel }} = await import('/node_modules/@tauri-apps/api/core.js');
          const onEvent = new Channel();
          onEvent.onmessage = event => window.__importEvents.push(event);
          await window.__TAURI_INTERNALS__.invoke('import_paths', {{ ...{js_string(args)}, onEvent }});
          return true;
        }})()""")
        finished = await wait_value(ws, session, "window.__importEvents.find(event => event.type === 'finished') || null", "真实媒体导入", 120)
        require(not finished["failed"] and not finished["conflicts"], f"导入失败或冲突：{finished}")
        placements = await wait_value(ws, session, "window.__TAURI_INTERNALS__.invoke('list_placements', {}).then(p => p.length === 3 ? p : null)", "视频 / 动图 / 序列帧包", 60)
    for placement in placements:
        ext = placement["ext"].lower()
        if ext == "mp4":
            ASSETS["video"] = placement
        elif ext == "gif":
            ASSETS["anim"] = placement
        elif ext == "png" and placement.get("frameCount"):
            ASSETS["sequence"] = placement
    require(set(ASSETS) == {"video", "anim", "sequence"}, f"媒体识别不完整：{placements}")
    log(f"[S1] 三类素材已就绪：{[(key, value['assetId']) for key, value in ASSETS.items()]}")


async def analyze(ws, session):
    runtime = await invoke(ws, session, "test_semantic_runtime")
    require(runtime["dimension"] == 512 and runtime["actualBackend"] == "cpu", f"运行时不符：{runtime}")
    require("spike" not in runtime["modelDir"], f"误用 spike 模型目录：{runtime['modelDir']}")
    expected_model_root = os.environ.get("ANIKIT_EXPECT_MODEL_ROOT")
    if expected_model_root:
        normalize = lambda value: os.path.normcase(str(Path(value).resolve())).removeprefix("\\\\?\\")
        expected_root = normalize(expected_model_root).rstrip("\\/")
        actual_model_dir = normalize(runtime["modelDir"])
        require(actual_model_dir.startswith(expected_root + os.sep), f"模型未使用发布目录内离线包：{runtime['modelDir']}")
    video_id = ASSETS["video"]["assetId"]
    task_id = await invoke(ws, session, "start_semantic_analysis", {"request": {"assetIds": [video_id], "recursive": False}})
    await invoke(ws, session, "pause_semantic_analysis", {"taskId": task_id})
    await wait_task(ws, session, task_id, "paused")
    require(await invoke(ws, session, "resume_semantic_analysis", {"taskId": task_id}) == task_id, "恢复返回了不同 taskId")
    await invoke(ws, session, "cancel_semantic_analysis", {"taskId": task_id})
    await wait_task(ws, session, task_id, "cancelled")
    retry_id = await invoke(ws, session, "retry_semantic_analysis", {"taskId": task_id})
    retried = await wait_task(ws, session, retry_id)
    require(retried["mode"] == "basic" and not retried["errors"], f"取消后重试失败：{retried}")
    ids = [item["assetId"] for item in ASSETS.values()]
    task_id = await invoke(ws, session, "start_semantic_analysis", {"request": {"assetIds": ids, "recursive": False}})
    snapshot = await wait_task(ws, session, task_id)
    require(snapshot["mode"] == "basic" and snapshot["actualBackend"] == "cpu" and not snapshot["errors"], f"基础分析失败：{snapshot}")
    log(f"[S2] 暂停 / 恢复 / 取消 / 重试及三类素材分析完成：{snapshot['completed']}/{snapshot['total']}")


def scope(view_key="quick:all"):
    return {
        "viewKey": view_key, "recursive": True, "global": view_key == "~",
        "filters": {
            "colors": [], "duration": None, "tags": [], "exts": [], "folderIds": [], "ratios": [],
            "linkStates": [], "size": None, "imported": None, "modified": None,
        },
    }


async def search_api(ws, session, query, view_key="quick:all"):
    result = await invoke(ws, session, "semantic_search", {"query": query, "scope": scope(view_key), "limit": 10})
    require(result["hits"], f"语义检索为空：{query}")
    require(result["actualBackend"] == "cpu", f"搜索后端不符：{result}")
    for hit in result["hits"]:
        assert 0 < len(hit["segments"]) <= 8 and hit["matchCount"] > 0
        require(hit["best"]["segmentId"] == hit["segments"][0]["segmentId"], f"best 未保持融合排序首位：{hit}")
    return result


async def panel_id(ws, session):
    value = await evaluate(ws, session, """(() => {
      const pinia = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia;
      const id = [...pinia._s.keys()].find(key => key.startsWith('preview#'));
      return id ? id.slice('preview#'.length) : null;
    })()""")
    require(value, "找不到面板预览会话")
    return value


async def search_ui(ws, session, query):
    global PANEL_ID
    PANEL_ID = await panel_id(ws, session)
    await evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('tabs').openView('quick:all', {js_string(PANEL_ID)})")
    await wait_value(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('tabs').viewKeyOf({js_string(PANEL_ID)}) === 'quick:all'", "切到全部视口", 10)
    view_key = await evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('tabs').viewKeyOf({js_string(PANEL_ID)})")
    await wait_value(ws, session, "!!document.querySelector('.fbs-ic.search')", "搜索栏就绪", 10)
    await click(ws, session, ".fbs-ic.search")
    await click_text(ws, session, "语义搜索")
    require(await evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search').modeFor({js_string(PANEL_ID)})") == "semantic", "菜单没有切到语义模式")
    await click(ws, session, ".fbs-input")
    await send(ws, "Input.dispatchKeyEvent", {"type": "keyDown", "key": "a", "code": "KeyA", "windowsVirtualKeyCode": 65, "modifiers": 2}, session)
    await send(ws, "Input.dispatchKeyEvent", {"type": "keyUp", "key": "a", "code": "KeyA", "windowsVirtualKeyCode": 65, "modifiers": 2}, session)
    await send(ws, "Input.insertText", {"text": query}, session)
    await wait_value(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search').draftFor({js_string(PANEL_ID)}) === {js_string(query)} && !document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search').isPending({js_string(PANEL_ID)}) && document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search').resultPlacementsFor({js_string(PANEL_ID)}, {js_string(view_key)}).length > 0", "界面语义搜索结果")
    expected = [f"{placement['assetId']}|{placement['folderId']}" for hit in (await search_api(ws, session, query, view_key))["hits"] for placement in hit["placements"]]
    ordered = await evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search').resultPlacementsFor({js_string(PANEL_ID)}, {js_string(view_key)}).map(p => `${{p.assetId}}|${{p.folderId}}`)")
    require(ordered == expected, f"界面结果与相关度顺序不一致：{ordered} != {expected}")
    expected_hits = [{"assetId": hit["best"]["assetId"], "segmentId": hit["best"]["segmentId"], "segmentCount": len(hit["segments"])} for hit in (await search_api(ws, session, query, view_key))["hits"]]
    actual_hits = await evaluate(ws, session, f"""(() => {{
      const search = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('search');
      return {js_string(expected)}.map(key => {{
        const hit = search.semanticHitFor({js_string(PANEL_ID)}, key.split('|')[0]);
        return hit && {{ assetId: hit.best.assetId, segmentId: hit.best.segmentId, segmentCount: hit.segments.length }};
      }});
    }})()""")
    require(actual_hits == expected_hits, f"界面没有收到真实语义分段：{actual_hits} != {expected_hits}")
    visible = await evaluate(ws, session, "[...document.querySelectorAll('[data-key]')].filter(el => el.offsetParent).map(el => el.dataset.key)")
    require(visible == [key for key in expected if key in visible], f"可见卡片与搜索结果源不一致：{visible}")
    count = await evaluate(ws, session, "document.querySelector('.result-count')?.textContent.trim()")
    require(count == str(len(ordered)), f"标题计数不符：{count} != {len(ordered)}")
    await screenshot(ws, session, "semantic-search.png")
    log(f"[S3] 搜索菜单、真实语义分段、相关度顺序与计数通过：{len(ordered)} 个资产")


async def click_text(ws, session, text):
    await wait_value(ws, session, f"""(() => {{
      const el = [...document.querySelectorAll('button')].find(item => item.textContent.trim() === {js_string(text)});
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return r.width > 0 && r.height > 0;
    }})()""", f"菜单项 {text}", 3)
    rect = await evaluate(ws, session, f"""(() => {{
      const el = [...document.querySelectorAll('button')].find(item => item.textContent.trim() === {js_string(text)});
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }};
    }})()""")
    if not rect:
        log(await evaluate(ws, session, "({menu:document.querySelector('.app-menu')?.innerText,buttons:[...document.querySelectorAll('button')].map(b=>b.textContent.trim()).filter(Boolean)})"))
    require(rect, f"找不到菜单项：{text}")
    await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": rect["x"], "y": rect["y"], "button": "none", "buttons": 0}, session)
    await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": rect["x"], "y": rect["y"], "button": "left", "buttons": 1, "clickCount": 1}, session)
    await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": rect["x"], "y": rect["y"], "button": "left", "buttons": 0, "clickCount": 1}, session)


def preview_value(field):
    return f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({js_string('preview#' + PANEL_ID)})?.{field}"


async def open_preview(ws, session, kind, single_click=False):
    asset = ASSETS[kind]
    hit = HITS[kind]["best"]
    selector = f"[data-key=\"{asset['assetId']}|{asset['folderId']}\"]"
    if single_click:
        await click(ws, session, selector)
        await asyncio.sleep(0.3)
        require(await evaluate(ws, session, "!document.querySelector('.preview-root')"), "单击意外打开了预览")
    await click(ws, session, selector)
    await click(ws, session, selector, count=2)
    await wait_value(ws, session, f"{preview_value('current')}?.assetId === {js_string(asset['assetId'])}", f"打开 {kind} 预览", 60)
    if kind == "video":
        expected = hit["startMs"] / 1000
        expression = f"""(() => {{
          const video = document.querySelector('video');
          return video && Math.abs(video.currentTime - {expected}) < 1 ? video.currentTime : null;
        }})()"""
        actual = await wait_value(ws, session, expression, f"视频定位到 {expected:.3f}s", 60)
        await evaluate(ws, session, "document.querySelector('video')?.pause(); true")
        actual = await evaluate(ws, session, "document.querySelector('video').currentTime")
        require(abs(actual - expected) < 1, f"视频定位错误：{actual} != {expected}")
        log(f"[S4] 单击不开预览；双击视频定位 {actual:.3f}s（目标 {expected:.3f}s）")
    elif kind == "anim":
        expected = hit["startFrame"]
        require(expected is not None, f"动图命中缺原始帧索引：{hit}")
        actual = await wait_value(ws, session, f"{preview_value('animFrame')}?.frameIndex === {expected} ? {preview_value('frame')} : null", f"动图真实帧流定位到 {expected}", 60)
        require(actual == expected, f"动图定位错误：{actual} != {expected}")
        log(f"[S5] 动图真实帧流定位原始帧 {actual}（不是按平均 fps 猜测）")
    else:
        expected = hit["startFrame"]
        require(expected is not None, f"序列帧命中缺原始帧索引：{hit}")
        actual = await wait_value(ws, session, f"{preview_value('frame')} === {expected} ? {preview_value('seq')}?.frames?.[{expected}]?.name : null", f"序列帧定位到 {expected}", 60)
        require(actual == f"blank-test_{expected + 1}.png", f"序列帧实际文件错位：{actual} != {expected}")
        log(f"[S6] 序列帧定位原始帧 {expected}：{actual}")
    await click(ws, session, ".preview-titlebar .nav-group button")
    await wait_value(ws, session, "!document.querySelector('.preview-root')", f"退出 {kind} 预览", 30)


async def screenshot(ws, session, name):
    try:
        import base64
        result = await send(ws, "Page.captureScreenshot", {"format": "png"}, session, timeout=30)
        path = BASE / name
        path.write_bytes(base64.b64decode(result["data"]))
        log(f"截图：{path}")
    except Exception as error:
        log(f"[warn] 截图失败（不影响已执行的业务断言）：{error}")


def rpc(port, method, params=None, session_id=None):
    headers = {
        "Accept": "application/json, text/event-stream", "Content-Type": "application/json",
        "MCP-Protocol-Version": "2025-03-26",
    }
    if session_id:
        headers["Mcp-Session-Id"] = session_id
    payload = {"jsonrpc": "2.0", "method": method, "params": params or {}}
    if not method.startswith("notifications/"):
        payload["id"] = 1
    body = json.dumps(payload, ensure_ascii=False).encode()
    request = urllib.request.Request(f"http://127.0.0.1:{port}/mcp", data=body, headers=headers, method="POST")
    with urllib.request.urlopen(request, timeout=180) as response:
        session_id = response.headers.get("Mcp-Session-Id", session_id)
        raw = response.read().decode()
    events = [json.loads(line[5:]) for line in raw.splitlines() if line.startswith("data:") and line[5:].strip()]
    if not events:
        return {}, session_id
    payload = events[-1]
    if "error" in payload:
        raise RuntimeError(f"MCP {method}: {payload['error']}")
    return payload.get("result", {}), session_id


def call_tool(port, name, arguments, session_id):
    result, _ = rpc(port, "tools/call", {"name": name, "arguments": arguments}, session_id)
    text = "".join(item.get("text", "") for item in result.get("content", []) if item.get("type") == "text")
    if result.get("isError"):
        return {"error": text}
    return json.loads(text) if text else None


def wait_mcp_task(port, task_id, session_id, wanted="done"):
    started = time.monotonic()
    while time.monotonic() - started < 180:
        snapshot = call_tool(port, "get_semantic_analysis_task", {"taskId": task_id}, session_id)
        if snapshot and snapshot["status"] == wanted:
            return snapshot
        if snapshot and snapshot["status"] == "failed":
            raise AssertionError(f"MCP 任务失败：{snapshot}")
        time.sleep(0.2)
    raise AssertionError(f"MCP 任务超时：{task_id} -> {wanted}")


async def verify_mcp(ws, session):
    await invoke(ws, session, "set_mcp_write_enabled", {"enabled": True})
    status = await invoke(ws, session, "get_mcp_status")
    port = status["port"]
    require(status["running"] and port, f"MCP 未启动：{status}")
    port_file = Path(os.environ["ANIKIT_CONFIG_DIR"]) / "mcp-server.port"
    require(port_file.is_file() and port_file.read_text().strip() == str(port), f"端口文件未隔离或不一致：{port_file}")
    initialized, session_id = rpc(port, "initialize", {
        "protocolVersion": "2025-03-26", "capabilities": {},
        "clientInfo": {"name": "semantic-live-probe", "version": "1.0"},
    })
    require(initialized["serverInfo"]["name"] == "anikit", f"MCP 握手异常：{initialized}")
    rpc(port, "notifications/initialized", session_id=session_id)
    tools, _ = rpc(port, "tools/list", session_id=session_id)
    require(len(tools["tools"]) == 40, f"MCP 工具数不符：{len(tools['tools'])}")
    annotations = {tool["name"]: tool.get("annotations", {}) for tool in tools["tools"]}
    require(annotations["semantic_search"].get("readOnlyHint") is True, "语义检索未标只读")
    require(annotations["start_semantic_analysis"].get("readOnlyHint") is False, "分析任务未标受控写")
    hits = call_tool(port, "semantic_search", {"query": "电脑屏幕上的软件界面", "limit": 10}, session_id)
    require(hits["hits"] and hits["hits"][0]["uri"].startswith(f"http://127.0.0.1:{port}/open/item/"), "MCP 语义命中或 uri 缺失")
    rejected = call_tool(port, "start_semantic_analysis", {"assetIds": []}, session_id)
    require("assetIds" in rejected.get("error", ""), f"空分析清单未拒绝：{rejected}")
    task_id = call_tool(port, "start_semantic_analysis", {"assetIds": [ASSETS["video"]["assetId"]]}, session_id)
    require(isinstance(task_id, str) and task_id.startswith("sem-"), f"MCP 启动任务失败：{task_id}")
    call_tool(port, "pause_semantic_analysis", {"taskId": task_id}, session_id)
    require(wait_mcp_task(port, task_id, session_id, "paused")["mode"] == "basic", "MCP 暂停模式错误")
    require(call_tool(port, "resume_semantic_analysis", {"taskId": task_id}, session_id) == task_id, "MCP 恢复 taskId 错误")
    call_tool(port, "cancel_semantic_analysis", {"taskId": task_id}, session_id)
    require(wait_mcp_task(port, task_id, session_id, "cancelled")["status"] == "cancelled", "MCP 取消失败")
    retry_id = call_tool(port, "retry_semantic_analysis", {"taskId": task_id}, session_id)
    require(wait_mcp_task(port, retry_id, session_id)["mode"] == "basic", "MCP 重试没有保持基础模式")
    detailed_error = call_tool(port, "start_detailed_semantic_analysis", {"assetIds": [ASSETS["video"]["assetId"]]}, session_id)
    require("provider" in detailed_error.get("error", "") or "配置" in detailed_error.get("error", ""), f"未配置详细分析未拒绝：{detailed_error}")
    tasks = call_tool(port, "list_semantic_analysis_tasks", {}, session_id)
    require(all(task["libraryRoot"] == TEST_LIBRARY for task in tasks), "MCP 混入其他库任务")
    await invoke(ws, session, "set_mcp_write_enabled", {"enabled": False})
    gated = call_tool(port, "start_semantic_analysis", {"assetIds": [ASSETS["video"]["assetId"]]}, session_id)
    require("MCP 写入未开启" in gated.get("error", ""), f"写入闸未生效：{gated}")
    log("[S7] MCP 握手、40 工具、语义检索、任务控制、库隔离与写入闸全通过")


async def main():
    require(os.environ.get("ANIKIT_E2E") == "1", "缺少 ANIKIT_E2E=1，拒绝在普通实例上验收")
    config = Path(os.environ["ANIKIT_CONFIG_DIR"]).resolve()
    require(config.parent == BASE, f"隔离配置目录不符：{config}")
    version = json.loads(urllib.request.urlopen(f"http://127.0.0.1:{CDP_PORT}/json/version", timeout=10).read())
    pages = json.loads(urllib.request.urlopen(f"http://127.0.0.1:{CDP_PORT}/json", timeout=10).read())
    url_part = os.environ.get("ANIKIT_E2E_URL_PART", "5373")
    page = next((item for item in pages if item.get("type") == "page" and url_part in item.get("url", "")), None)
    require(page, f"找不到真实应用页面：{pages}")
    async with websockets.connect(version["webSocketDebuggerUrl"], max_size=64 * 1024 * 1024) as ws:
        attached = await send(ws, "Target.attachToTarget", {"targetId": page["id"], "flatten": True})
        session = attached["sessionId"]
        await wait_value(ws, session, "!!document.querySelector('#app .window') && !!window.__TAURI_INTERNALS__", "真实应用挂载", 90)
        await create_library(ws, session)
        await import_media(ws, session)
        await analyze(ws, session)
        result = await search_api(ws, session, "电脑屏幕上的软件界面")
        for hit in result["hits"]:
            for kind, asset in ASSETS.items():
                if hit["best"]["assetId"] == asset["assetId"]:
                    HITS[kind] = hit
        require(set(HITS) == set(ASSETS), f"命中与素材不完整：{result}")
        await search_ui(ws, session, "电脑屏幕上的软件界面")
        await open_preview(ws, session, "video", single_click=True)
        await open_preview(ws, session, "anim")
        await open_preview(ws, session, "sequence")
        await verify_mcp(ws, session)
        await semantic_segment_live_probe.verify_segments(sys.modules[__name__], ws, session)
        log("[PASS] 真实语义搜索、预览定位、片段闭环与 MCP 全链验收完成")


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except Exception as error:
        log(f"[FAIL] {type(error).__name__}: {error}")
        traceback.print_exc()
        sys.exit(1)
