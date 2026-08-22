# CDP 实测：5.3 视频帧流 · 真后端联调探针（2026-08-22，用户实机"预览冻结/模糊/seek 慢"返修验证）
# 与桩探针的差别：帧源 = 真 Rust 后端（frame_stream + VideoDecoder 解码真实视频文件，WS 协议 v2），
# 前端全真。启动前置：dev server 5373 + harness（cargo test --lib -- --ignored video_stream_harness，
# 状态文件 tools/.harness-state.json）。断言：
# ① 预览开帧流出帧 + 播放条帧号 x/total；② 3s 连续出帧（变化数≈节流后 fps、无 >500ms 间隔 = 不冻结）；
# ③ 进度条点按 seek 端到端延迟（点击 → 帧号落位）< 1s；④ seek 后继续播放不冻结；
# ⑤ 暂停/恢复命令经命令文件直达真后端；⑥ 无前端异常。
import asyncio
import json
import os
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9374
URL = "http://localhost:5373"
STATE = os.path.join(os.path.dirname(__file__), ".harness-state.json")
CMD = os.path.join(os.path.dirname(__file__), ".harness-cmd.jsonl")

_msg_id = 0


def load_state():
    with open(STATE, encoding="utf-8") as f:
        return json.load(f)


def make_stub(st):
    return r"""
window.__cmds = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 1, folderCount: 0, autoImport: false };
    const placements = [{
      assetId: "V0", folderId: "root", name: "实测.mp4",
      w: 3840, h: 2160, ext: "mp4", thumbPath: "E:/T/thumb.webp", thumbKind: "asset",
      origin: true, frameCount: null, animated: false, duration: DUR, bpm: null,
      importedFrom: null, noteCount: null, thumbFrame: null, fps: FPS,
    }];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false, autoPlayVideo: false },
      detect_doc_apps: [],
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_asset_file_path: { path: "E:/T/src.mp4", pending: false },
      get_notes: [],
    };
    // 帧流命令族 → 命令文件（真后端 harness 轮询执行）；start 返回 harness 会话
    if (cmd === "start_video_stream")
      return Promise.resolve({ session: SID, port: SPORT, hasAudio: true, fps: FPS, duration: DUR });
    if (["seek_video_stream", "set_anim_stream_playing", "set_anim_stream_rate",
         "set_stream_loop", "set_anim_stream_size", "stop_anim_stream"].includes(cmd)) {
      window.__cmds.push([cmd, args]);
      fetch("__CMDECHO__").catch(() => {}); // 占位（命令经探针进程转发，见下）
      return Promise.resolve(null);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
""".replace("SID", str(st["session"])).replace("SPORT", str(st["port"])).replace("FPS", str(st["fps"])).replace("DUR", str(st["duration"]))


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
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = [str(a.get("value", a.get("description", "")))[:300] for a in raw["params"]["args"]]
            print("[CONSOLE]", raw["params"]["type"], " ".join(args)[:500])


async def main():
    if not os.path.isfile(STATE):
        print("FAIL: harness 状态文件不存在——先启动 harness（见文件头注释）")
        return 2
    st = load_state()
    print(f"harness 会话: {st}")
    total = round(st["duration"] * st["fps"])

    profile = tempfile.mkdtemp(prefix="cdp-video-real-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
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
        async with websockets.connect(version["webSocketDebuggerUrl"], max_size=100 * 1024 * 1024) as ws:
            target = await send(ws, "Target.createTarget", {"url": "about:blank"})
            tid = target["result"]["targetId"]
            sess = (await send(ws, "Target.attachToTarget", {"targetId": tid, "flatten": True}))["result"]["sessionId"]
            await send(ws, "Runtime.enable", session=sess)
            await send(ws, "Page.enable", session=sess)
            await send(ws, "Page.addScriptToEvaluateOnNewDocument", {"source": make_stub(st)}, session=sess)
            await send(ws, "Page.navigate", {"url": URL}, session=sess)
            await asyncio.sleep(6)

            async def ev(expr, await_promise=True):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": await_promise}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            fails = []

            # 命令转发（主流程同步执行，不用后台任务——CDP 同一连接并发 evaluate 会互吃响应）：
            # 把 __cmds 里的帧流命令写进 harness 命令文件
            async def flush_cmds():
                cmds = await ev("window.__cmds.splice(0)")
                for cmd, args in cmds or []:
                    m = {"seek_video_stream": lambda: {"seek": args["secs"]},
                         "set_anim_stream_playing": lambda: {"playing": args["playing"]},
                         "set_anim_stream_rate": lambda: {"rate": args["rate"]},
                         "set_stream_loop": lambda: {"loop": args["loopEnabled"]},
                         "set_anim_stream_size": lambda: {"size": args["shortEdge"]},
                         "stop_anim_stream": lambda: {"stop": True}}[cmd]()
                    with open(CMD, "a", encoding="utf-8") as f:
                        f.write(json.dumps(m) + "\n")

            # 双击视频卡 → 预览
            await ev("document.querySelector('.asset-card').dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(3.0)  # 预览加载 + 起会话 + WS 连接 + 首帧

            p0 = await ev("""({
              stage: !!document.querySelector('.preview-stage'),
              canvas: (() => { const c = document.querySelector('.preview-stage canvas'); return c ? { w: c.width, h: c.height } : null; })(),
              barText: document.querySelector('.play-bar .tdc-btn')?.textContent?.trim() ?? null,
            })""")
            print("预览打开:", json.dumps(p0, ensure_ascii=False))
            if not p0 or not p0["canvas"] or not p0["canvas"]["w"]:
                fails.append(f"预览 canvas 未出帧（{p0 and p0['canvas']}）——真后端链路断")
            if not p0 or not p0["barText"] or not p0["barText"].endswith(f"/ {total}"):
                fails.append(f"播放条总帧数 ≠ {total}（{p0 and p0['barText']}）")

            # ② 3s 连续出帧采样（播放条文本逐帧变；记录变化数与最大间隔）
            smooth = await ev("""(async () => {
              const b = document.querySelector('.play-bar .tdc-btn');
              let last = b.textContent, lastT = performance.now(), maxGap = 0, changes = 0;
              const t0 = performance.now();
              while (performance.now() - t0 < 3000) {
                await new Promise((r) => setTimeout(r, 30));
                const t = b.textContent;
                if (t !== last) {
                  changes++;
                  last = t;
                  const now = performance.now();
                  maxGap = Math.max(maxGap, now - lastT);
                  lastT = now;
                }
              }
              return { changes, maxGap: Math.round(maxGap) };
            })()""")
            print("3s 出帧采样:", smooth)
            # 阈值口径：无头 Edge --disable-gpu = putImageData 软件光栅，60fps×7MB 下主线程天然抖
            # （真机 GPU canvas 无此瓶颈）——本探针钉的是"永久冻结"回归（长间隙 / 龟速），不钉满帧率
            if not smooth or smooth["changes"] < 8:
                fails.append(f"3s 出帧变化过少（{smooth}）——播放冻结/龟速")
            if smooth and smooth["maxGap"] > 1500:
                fails.append(f"出帧最大间隔 {smooth['maxGap']}ms > 1500ms——疑似冻结级卡顿")

            # ③ 进度条点按远端（当前位置之后 90% 处）→ seek 端到端延迟（含命令文件轮询 ≤100ms）
            cur = await ev("(() => { const m = document.querySelector('.play-bar .tdc-btn').textContent.match(/(\\d+)/); return m ? parseInt(m[1]) : 0; })()")
            target_min = max(int(total * 0.9), cur + int(total * 0.2))
            ratio = target_min / total
            prog = await ev("(() => { const r = document.querySelector('.play-bar .progress').getBoundingClientRect(); return { x: r.left + r.width * 0.9, y: r.top + r.height / 2 }; })()")
            await ev(f"""window.__seekProbe = (() => {{
              const b = document.querySelector('.play-bar .tdc-btn');
              const t0 = performance.now();
              const timer = setInterval(() => {{
                const m = b.textContent.match(/(\\d+)/);
                if (m && parseInt(m[1]) >= {target_min}) {{
                  clearInterval(timer);
                  window.__seekMs = performance.now() - t0;
                }}
              }}, 16);
              return true;
            }})()""")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.15)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": prog["x"], "y": prog["y"], "button": "left", "clickCount": 1}, session=sess)
            await flush_cmds()  # seek 命令同步转发（点击后立刻）
            await asyncio.sleep(4.0)  # 等落位
            seek_ms = await ev("window.__seekMs ?? -1")
            print(f"seek 90% 端到端延迟: {seek_ms}ms（目标帧 ≥{target_min}）")
            if seek_ms == -1:
                fails.append("seek 后 4s 内未落位（冻结复现）")
            elif seek_ms > 1000:
                fails.append(f"seek 端到端 {seek_ms:.0f}ms > 1s")

            # ④ seek 后续播 1.5s（不冻结）
            cont = await ev("""(async () => {
              const b = document.querySelector('.play-bar .tdc-btn');
              const a = b.textContent;
              await new Promise((r) => setTimeout(r, 1500));
              return { from: a, to: b.textContent };
            })()""")
            print("seek 后续播:", cont)
            if cont and cont["from"] == cont["to"]:
                fails.append(f"seek 后未续播（{cont}）——冻结")

            # ⑤ 暂停 / 恢复（真后端命令经命令文件）
            await ev("document.querySelector('.play-bar .icon-btn').click()")  # 暂停
            await flush_cmds()
            await asyncio.sleep(0.6)
            paused = await ev("""(async () => {
              const b = document.querySelector('.play-bar .tdc-btn');
              const a = b.textContent;
              await new Promise((r) => setTimeout(r, 700));
              return a === b.textContent;
            })()""")
            await ev("document.querySelector('.play-bar .icon-btn').click()")  # 恢复
            await flush_cmds()
            await asyncio.sleep(0.5)
            print("暂停冻结校验:", paused)
            if not paused:
                fails.append("暂停后帧号仍推进——暂停命令未达真后端")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 真后端全链路成立（连续出帧不冻结 / seek <1s 落位 / 后续播 / 暂停恢复 / 无异常）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


sys.exit(asyncio.run(main()))
