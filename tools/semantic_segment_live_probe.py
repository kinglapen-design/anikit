"""语义片段的真实菜单、MCP 与范围播放验收；由 cdp-semantic-live-probe.py 调用。"""
import asyncio
import json
from pathlib import Path


async def right_click(probe, ws, session, selector):
    rect = await probe.evaluate(ws, session, f"""(() => {{
      const el = document.querySelector({probe.js_string(selector)});
      if (!el) return null;
      el.scrollIntoView({{ block: 'center' }});
      const r = el.getBoundingClientRect();
      return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }};
    }})()""")
    probe.require(rect, f"找不到右键目标：{selector}")
    await probe.send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", **rect, "button": "right", "buttons": 2, "clickCount": 1}, session)
    await probe.send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", **rect, "button": "right", "buttons": 0, "clickCount": 1}, session)


async def open_created(probe, ws, session, placement, kind, expected):
    key = f"{placement['assetId']}|{placement['folderId']}"
    await probe.evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({probe.js_string('preview#' + probe.PANEL_ID)}).open({probe.js_string(key)}, null)")
    await probe.wait_value(ws, session, f"{probe.preview_value('current')}?.assetId === {probe.js_string(placement['assetId'])}", f"打开新片段 {placement['assetId']}", 60)
    segment = await probe.evaluate(ws, session, f"""(() => {{
      const range = {probe.preview_value('segment')};
      return range && {{ startMs: range.startMs, endMs: range.endMs, startFrame: range.startFrame, endFrame: range.endFrame }};
    }})()""")
    if kind == "video":
        require_range(probe, segment, expected, False)
        media_segment = await probe.evaluate(ws, session, f"""(() => {{
          const range = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({probe.js_string('previewMedia#' + probe.PANEL_ID)}).segment;
          return range && {{ startMs: range.startMs, endMs: range.endMs, startFrame: range.startFrame, endFrame: range.endFrame }};
        }})()""")
        require_range(probe, media_segment, expected, False)
        current = await probe.wait_value(ws, session, f"""(() => {{
          const video = document.querySelector('video');
          return video && video.currentTime >= {expected['startMs'] / 1000 - 0.2} ? video.currentTime : null;
        }})()""", "虚拟视频按源时间范围播放", 60)
        probe.require(current <= expected["endMs"] / 1000 + 0.5, f"虚拟视频越界：{current}")
        await probe.evaluate(ws, session, "document.querySelector('video')?.pause(); true")
    elif kind == "anim":
        require_range(probe, segment, expected, True)
        frame = await probe.wait_value(ws, session, f"""(() => {{
          const p = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({probe.js_string('preview#' + probe.PANEL_ID)});
          return p.animFrame && p.animFrame.frameIndex >= {expected['startFrame']} ? p.animFrame.frameIndex : null;
        }})()""", "虚拟动图按原始帧范围播放", 60)
        probe.require(frame <= expected["endFrame"], f"虚拟动图越出原始帧范围：{frame}")
        await probe.evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({probe.js_string('preview#' + probe.PANEL_ID)}).pause()")
    elif kind == "sequence":
        require_range(probe, segment, expected, True)
        actual = await probe.wait_value(ws, session, f"{probe.preview_value('frame')} === {expected['startFrame']} ? {probe.preview_value('seq')}.frames[{expected['startFrame']}].name : null", "虚拟序列帧按原始索引播放", 60)
        probe.require(actual == f"blank-test_{expected['startFrame'] + 1}.png", f"虚拟序列帧错位：{actual}")
        count = await probe.evaluate(ws, session, f"{probe.preview_value('seq')}.frames.length")
        probe.require(count == 3, f"虚拟序列帧错误裁剪源清单：{count}")
        await probe.evaluate(ws, session, f"document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get({probe.js_string('preview#' + probe.PANEL_ID)}).pause()")
    else:
        probe.require(segment is None, f"物理片段错误保留虚拟范围：{segment}")
        if kind == "physical-anim":
            preview_kind = await probe.evaluate(ws, session, f"{probe.preview_value('kind')}")
            probe.require(preview_kind in ("image", "anim"), f"物理帧预览类型错误：{preview_kind}")
            if preview_kind == "anim":
                frame = await probe.wait_value(ws, session, f"{probe.preview_value('animFrame')}?.frameIndex", "物理动图独立播放", 60)
                probe.require(frame in (0, 1), f"物理动图帧错位：{frame}")
            else:
                await probe.wait_value(ws, session, f"{probe.preview_value('filePath')}?.toLowerCase().endsWith('.gif')", "物理单帧图片独立显示", 60)
        else:
            current = await probe.wait_value(ws, session, "document.querySelector('video')?.currentTime", "物理视频独立播放", 60)
            probe.require(current < 2, f"物理视频从源时间而非自身起点播放：{current}")
        await probe.evaluate(ws, session, "document.querySelector('video')?.pause(); true")
    await probe.click(ws, session, ".preview-titlebar .nav-group button")
    await probe.wait_value(ws, session, "!document.querySelector('.preview-root')", f"关闭片段预览 {kind}", 30)


def require_range(probe, actual, expected, frames):
    probe.require(actual is not None, "缺少虚拟片段范围")
    probe.require(actual["startMs"] == expected["startMs"] and actual["endMs"] == expected["endMs"], f"片段时间范围不符：{actual}")
    if frames:
        probe.require(actual["startFrame"] == expected["startFrame"] and actual["endFrame"] == expected["endFrame"], f"原始帧范围不符：{actual}")


async def wait_placement(probe, ws, session, asset_id):
    expression = f"""(async () => {{
      const pinia = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia;
      if (!pinia._s.get('content').placements.some(p => p.assetId === {probe.js_string(asset_id)})) return null;
      const items = await window.__TAURI_INTERNALS__.invoke('list_placements', {{}});
      return items.find(p => p.assetId === {probe.js_string(asset_id)}) || null;
    }})()"""
    return await probe.wait_value(ws, session, expression, f"新片段入库并热刷新 {asset_id}", 60)


async def hover_text(probe, ws, session, text):
    rect = await probe.evaluate(ws, session, f"""(() => {{
      const el = [...document.querySelectorAll('button')].find(item => item.textContent.trim() === {probe.js_string(text)});
      if (!el) return null;
      const r = el.getBoundingClientRect();
      return {{ x: r.left + r.width / 2, y: r.top + r.height / 2 }};
    }})()""")
    probe.require(rect, f"找不到悬停菜单项：{text}")
    await probe.send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", **rect, "button": "none", "buttons": 0}, session)


async def verify_segments(probe, ws, session):
    source = probe.ASSETS["video"]
    before = await probe.invoke(ws, session, "list_placements")
    known = {item["assetId"] for item in before}
    await probe.evaluate(ws, session, f"window.__segmentKnown = new Set({json.dumps(list(known))}); true")
    selector = f"[data-key=\"{source['assetId']}|{source['folderId']}\"]"
    await right_click(probe, ws, session, selector)
    await hover_text(probe, ws, session, "AI 工具集")
    await probe.wait_value(ws, session, "[...document.querySelectorAll('button')].some(item => item.textContent.trim() === '创建虚拟片段（当前命中）')", "AI 子菜单展开", 3)
    await probe.evaluate(ws, session, "window.__segmentMenuClicked = false; [...document.querySelectorAll('button')].find(item => item.textContent.trim() === '创建虚拟片段（当前命中）').addEventListener('click', () => window.__segmentMenuClicked = true); true")
    await probe.click_text(ws, session, "创建虚拟片段（当前命中）")
    await asyncio.sleep(1.0)
    state = await probe.evaluate(ws, session, f"""(async () => {{
      const pinia = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia;
      const items = await window.__TAURI_INTERNALS__.invoke('list_placements', {{}});
      return {{ clicked: window.__segmentMenuClicked, count: items.length,
        hit: pinia._s.get('search').semanticHitFor({probe.js_string(probe.PANEL_ID)}, {probe.js_string(source['assetId'])})?.best?.segmentId,
        messages: pinia._s.get('toast').items.map(item => item.message) }};
    }})()""")
    probe.log(f"[segment] 点击后状态：{state}")
    created = await probe.wait_value(ws, session, "window.__TAURI_INTERNALS__.invoke('list_placements', {}).then(items => items.find(p => p.segment && !window.__segmentKnown.has(p.assetId)) || null)", "菜单创建虚拟片段", 60)
    probe.log("[S8] 右键 AI 工具集菜单创建虚拟视频片段通过")
    detail = await probe.invoke(ws, session, "get_asset_detail", {"assetId": created["assetId"]})
    probe.require(detail["type"] == "segment" and detail["source"] == source["assetId"], f"虚拟片段详情错误：{detail}")
    origin = await probe.invoke(ws, session, "get_asset_origin_path", {"assetId": created["assetId"]})
    probe.require(Path(origin).is_file(), f"虚拟片段来源路径不存在：{origin}")
    await open_created(probe, ws, session, created, "video", probe.HITS["video"]["best"])

    await probe.invoke(ws, session, "set_mcp_write_enabled", {"enabled": True})
    status = await probe.invoke(ws, session, "get_mcp_status")
    port = status["port"]
    _, session_id = probe.rpc(port, "initialize", {"protocolVersion": "2025-03-26", "capabilities": {}, "clientInfo": {"name": "segment-live-probe", "version": "1.0"}})
    probe.rpc(port, "notifications/initialized", session_id=session_id)
    tools, _ = probe.rpc(port, "tools/list", session_id=session_id)
    probe.require(len(tools["tools"]) == 32, f"片段接入后 MCP 工具数不符：{len(tools['tools'])}")
    for kind in ("anim", "sequence"):
        hit = probe.HITS[kind]["best"]
        virtual = probe.call_tool(port, "create_semantic_segment", {"assetId": hit["assetId"], "segmentId": hit["segmentId"]}, session_id)
        probe.require(virtual.get("assetId"), f"MCP 创建虚拟 {kind} 失败：{virtual}")
        placement = await wait_placement(probe, ws, session, virtual["assetId"])
        await open_created(probe, ws, session, placement, kind, hit)
    hit = probe.HITS["anim"]["best"]
    physical = probe.call_tool(port, "create_semantic_segment", {"assetId": hit["assetId"], "segmentId": hit["segmentId"], "physical": True}, session_id)
    probe.require(physical.get("assetId"), f"MCP 物理截取失败：{physical}")
    placement = await wait_placement(probe, ws, session, physical["assetId"])
    detail = await probe.invoke(ws, session, "get_asset_detail", {"assetId": physical["assetId"]})
    probe.require(detail.get("type") is None and detail["source"] == hit["assetId"], f"物理片段来源证据错误：{detail}")
    origin = await probe.invoke(ws, session, "get_asset_origin_path", {"assetId": physical["assetId"]})
    probe.require(Path(origin).is_file() and Path(origin).read_bytes()[:6] in (b"GIF87a", b"GIF89a"), f"物理动图不可独立播放：{origin}")
    await open_created(probe, ws, session, placement, "physical-anim", hit)
    await probe.invoke(ws, session, "set_mcp_write_enabled", {"enabled": False})
    probe.log("[S9] 虚拟视频 / 动图 / 序列帧范围播放、物理动图独立播放与 MCP 32 工具全通过")
