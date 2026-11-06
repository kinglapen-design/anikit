"""真实 Tauri 隔离验收：静态 PNG 分析、搜索、卡片、菜单门禁与预览。"""
import asyncio
import importlib
import json
import os
from pathlib import Path
import sys
import urllib.request

import websockets

sys.path.insert(0, str(Path(__file__).resolve().parent))
probe = importlib.import_module("cdp-semantic-live-probe")


async def main():
    assert os.environ.get("ANIKIT_E2E") == "1"
    config = Path(os.environ["ANIKIT_CONFIG_DIR"]).resolve()
    assert config.parent == probe.BASE
    version = json.loads(urllib.request.urlopen("http://127.0.0.1:19333/json/version", timeout=10).read())
    pages = json.loads(urllib.request.urlopen("http://127.0.0.1:19333/json", timeout=10).read())
    page = next(item for item in pages if item.get("type") == "page" and "5373" in item.get("url", ""))
    async with websockets.connect(version["webSocketDebuggerUrl"], max_size=64 * 1024 * 1024) as ws:
        session = (await probe.send(ws, "Target.attachToTarget", {"targetId": page["id"], "flatten": True}))["sessionId"]
        await probe.wait_value(ws, session, "!!document.querySelector('#app .window') && !!window.__TAURI_INTERNALS__", "应用挂载", 90)
        await probe.create_library(ws, session)
        placements = await probe.invoke(ws, session, "list_placements")
        static = find_static(placements)
        if static is None:
            args = {
                "paths": [str((probe.ROOT / "tools/test-assets/blank-test-1.png").resolve())],
                "structure": "flatten", "source": "copy", "targetFolder": None, "sequenceMode": "scatter",
            }
            await probe.evaluate(ws, session, f"""(async () => {{
              const {{ Channel }} = await import('/node_modules/@tauri-apps/api/core.js');
              const events = new Channel();
              await window.__TAURI_INTERNALS__.invoke('import_paths', {{ ...{json.dumps(args)}, onEvent: events }});
              return true;
            }})()""")
            static = await probe.wait_value(ws, session, "window.__TAURI_INTERNALS__.invoke('list_placements', {}).then(items => items.find(p => p.ext === 'png' && p.frameCount == null && !p.animated && !p.segment) || null)", "静态图片导入", 60)
        assert static is not None
        asset_id = static["assetId"]
        await probe.evaluate(ws, session, "document.querySelector('#app').__vue_app__.config.globalProperties.$pinia._s.get('content').refresh()")
        await probe.wait_value(ws, session, f"""(() => {{
          const pinia = document.querySelector('#app').__vue_app__.config.globalProperties.$pinia;
          return pinia._s.get('content').placements.some(item => item.assetId === {probe.js_string(asset_id)});
        }})()""", "内容存储收到静态图", 30)
        status = await probe.invoke(ws, session, "get_semantic_asset_status", {"assetId": asset_id})
        if not status["segmentCount"]:
            task_id = await probe.invoke(ws, session, "start_semantic_analysis", {"request": {"assetIds": [asset_id], "recursive": False}})
            await probe.wait_task(ws, session, task_id)
        status = await probe.invoke(ws, session, "get_semantic_asset_status", {"assetId": asset_id})
        assert status["analyzable"] and status["segmentCount"] == 1, status

        query = "电脑屏幕上的软件界面"
        result = await probe.invoke(ws, session, "semantic_search", {"query": query, "scope": probe.scope("~"), "limit": 30})
        assert any(hit["best"]["assetId"] == asset_id for hit in result["hits"])
        await probe.search_ui(ws, session, query)
        selector = f"[data-key=\"{asset_id}|{static['folderId']}\"]"
        await probe.wait_value(ws, session, f"!!document.querySelector({probe.js_string(selector)})", "静态图卡片", 30)
        has_badge = await probe.evaluate(ws, session, f"!!document.querySelector({probe.js_string(selector + ' .semantic-badge')})")
        assert not has_badge, "静态图出现时间徽标"

        labels = ["分析文件向量", "详细语义分析（外部服务）", "创建虚拟片段（当前命中）", "物理截取当前命中"]
        menu = await probe.evaluate(ws, session, f"""(async () => {{
          const {{ assetCardSpec }} = await import('/src/lib/menuSpecs/assetCard.ts');
          const entries = assetCardSpec({json.dumps(static, ensure_ascii=False)}, {probe.js_string(probe.PANEL_ID)});
          const submenu = entries.find(entry => entry.label === 'AI 工具集').submenu;
          return Object.fromEntries({json.dumps(labels, ensure_ascii=False)}.map(text => {{
            const entry = submenu.find(item => item.label === text);
            return [text, entry ? {{ disabled: !!entry.disabled }} : null];
          }}));
        }})()""")
        assert menu["分析文件向量"] and not menu["分析文件向量"]["disabled"], menu
        for label in labels[1:]:
            assert menu[label] and menu[label]["disabled"], menu
        await probe.click(ws, session, selector, count=2)
        await probe.wait_value(ws, session, f"{probe.preview_value('current')}?.assetId === {probe.js_string(asset_id)}", "静态图预览", 30)
        print("PASS static PNG analysis/search/card/menu/preview: " + json.dumps({"assetId": asset_id, "status": status}, ensure_ascii=True), flush=True)


def find_static(placements):
    return next((item for item in placements if item["ext"].lower() == "png" and item.get("frameCount") is None and not item.get("animated") and not item.get("segment")), None)


if __name__ == "__main__":
    asyncio.run(main())
