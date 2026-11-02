# CDP 实证：子窗口创建（2026-11-02 Runtime 156 UDF 冲突修复验证，踩坑 121）
# 前置：WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9333 启动的 dev / release 实例
# 步骤：主窗口 __TAURI_INTERNALS__.invoke 开 plugin-mcp 与 preview 两窗 →
#       Windows 顶层窗口标题枚举断言「AniKit 插件」「AniKit 预览」可见 → 截屏 tools/_child-windows.png。
# 注意：UDF 隔离后子窗口 = 独立 browser process，主窗口 CDP /json 看不到子窗口 target
#       （不能以 target 出现作断言）；invoke 用 __TAURI_INTERNALS__（v2 无 withGlobalTauri）。
import asyncio
import ctypes
import json
import os
import time
import urllib.request

import websockets

# 端口可配：默认 9333（避开本机既有 Edge / 旧探针 9222 占用——连错目标会拿桩页面假 FAIL）
CDP = os.environ.get("CDP_URL", "http://127.0.0.1:9333/json")
# assetKey = placementKey（${assetId}|${folderId}，非纯 assetId——传纯 id 会 current null 黑屏）
ASSET_KEY = os.environ.get("PROBE_ASSET", "AN2609250223-Q5BW7|F2610080228-XJZ")


def targets():
    return json.load(urllib.request.urlopen(CDP, timeout=3))


def visible_anikit_titles():
    u = ctypes.windll.user32
    titles = []

    @ctypes.WINFUNCTYPE(ctypes.c_bool, ctypes.c_void_p, ctypes.c_void_p)
    def cb(h, _):
        if u.IsWindowVisible(h):
            buf = ctypes.create_unicode_buffer(256)
            u.GetWindowTextW(h, buf, 256)
            if "AniKit" in buf.value:
                titles.append(buf.value)
        return True

    u.EnumWindows(cb, 0)
    return titles


async def main():
    deadline = time.time() + 120
    main_target = None
    while time.time() < deadline:
        try:
            for t in targets():
                if t.get("type") == "page" and "?plugin=" not in t.get("url", "") and "?preview=" not in t.get("url", ""):
                    main_target = t
                    break
            if main_target:
                break
        except Exception:
            pass
        time.sleep(2)
    assert main_target, "找不到主窗口 target（实例未起或 CDP 未开）"
    print("主窗口:", main_target["url"][:80])

    async with websockets.connect(main_target["webSocketDebuggerUrl"], max_size=20 * 1024 * 1024) as ws:
        for i, (cmd, args) in enumerate([
            ("open_plugin_window", "{pluginId:'mcp'}"),
            ("open_preview_window", "{assetKey:'%s'}" % ASSET_KEY),
        ]):
            await ws.send(json.dumps({
                "id": i + 1, "method": "Runtime.evaluate",
                "params": {"expression": f"window.__TAURI_INTERNALS__.invoke('{cmd}',{args})", "awaitPromise": True},
            }))
            resp = json.loads(await ws.recv())
            print(f"invoke {cmd}:", json.dumps(resp.get("result", resp))[:200])

    ok = False
    for _ in range(15):
        time.sleep(1)
        titles = visible_anikit_titles()
        if "AniKit 插件" in titles and "AniKit 预览" in titles:
            print("PASS：子窗口标题枚举可见 =", titles)
            ok = True
            break
    print("当前 AniKit 可见窗口:", visible_anikit_titles())
    assert ok, "FAIL：子窗口未出现（创建仍失败）"

    from PIL import ImageGrab
    ImageGrab.grab().save("tools/_child-windows.png")
    print("截屏已存 tools/_child-windows.png")


asyncio.run(main())
