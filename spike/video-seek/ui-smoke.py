# 真机 UI 冒烟（5.3 回滚 <video> 直读后，2026-08-22）——连真实 Tauri WebView2（9223），
# 在真实 AniKit 界面上操作真实库视频卡：① 悬停 scrub：Input 真实鼠标进卡 → 等 video 层 →
# 横向 20 步扫过（66ms/步），逐步采样 video.currentTime 与目标比偏差 + 帧内容变化次数；
# ② 双击开预览：播放 2.5s 记 rVFC 帧率；③ 播放条拖动 10 点记 seeked 延迟；④ 逐帧按钮
# （键盘 . / ,）记步进延迟与步长精度。帧内容 = drawImage(video) 缩样 16x9 像素和。
# 前置：app.exe 以 additionalBrowserArgs=--remote-debugging-port=9223 启动且库已加载出卡片。
# 用法：tools/.venv/Scripts/python.exe spike/video-seek/ui-smoke.py
import asyncio
import json
import time
import urllib.request

import websockets

CDP = "http://localhost:9223"


def wait_page(timeout_s: float = 60) -> str:
    begin = time.time()
    while time.time() - begin < timeout_s:
        try:
            with urllib.request.urlopen(CDP + "/json", timeout=3) as r:
                for t in json.loads(r.read()):
                    if t.get("type") == "page" and "127.0.0.1:5373" in t.get("url", ""):
                        return t["webSocketDebuggerUrl"]
        except Exception:
            pass
        time.sleep(1)
    raise RuntimeError("CDP 9223 无 page（app 未起？）")


class Cdp:
    def __init__(self, ws):
        self.ws = ws
        self.mid = 0

    async def call(self, method, params=None):
        self.mid += 1
        await self.ws.send(json.dumps({"id": self.mid, "method": method, "params": params or {}}))
        while True:
            msg = json.loads(await self.ws.recv())
            if msg.get("id") == self.mid:
                return msg.get("result", {})

    async def js(self, expr):
        r = await self.call("Runtime.evaluate", {
            "expression": expr, "awaitPromise": True, "returnByValue": True,
        })
        res = r.get("result", {})
        if res.get("subtype") == "error" or "exceptionDetails" in r:
            raise RuntimeError(json.dumps(r.get("exceptionDetails", res), ensure_ascii=False)[:500])
        return res.get("value")

    async def mouse_move(self, x, y):
        await self.call("Input.dispatchMouseEvent", {"type": "mouseMoved", "x": x, "y": y})


# 找视频卡：MP4 角标向上找卡片容器，返回卡片中心坐标与数量信息
FIND_CARD_JS = r"""
(() => {
  const badges = [...document.querySelectorAll("*")].filter(
    (el) => el.children.length === 0 && el.textContent.trim() === "MP4");
  if (!badges.length) return { error: "no MP4 badge", body: document.body.innerText.slice(0, 300) };
  const card = badges[0].closest("div[class]");
  let el = badges[0];
  // 向上爬到拥有图片/视频区域的卡片根（含 img 或 .img 子树）
  while (el && !(el.querySelector && el.querySelector("img, .img"))) el = el.parentElement;
  if (!el) return { error: "no card root" };
  const r = el.getBoundingClientRect();
  return { x: r.x, y: r.y, w: r.width, h: r.height, badgeCount: badges.length,
           cls: el.className && String(el.className).slice(0, 80) };
})()
"""

# 悬停 scrub 测量：注入后在卡片内逐步移动鼠标由 python 驱动；此处只负责采样 video 状态
VIDEO_STATE_JS = r"""
(() => {
  const v = [...document.querySelectorAll("video")].find((v) => v.readyState > 0 && v.closest("div").offsetParent !== null)
         || document.querySelector("video");
  if (!v) return null;
  // 帧内容指纹：缩样像素和
  let sig = -1;
  try {
    const c = document.createElement("canvas");
    c.width = 16; c.height = 9;
    c.getContext("2d").drawImage(v, 0, 0, 16, 9);
    sig = c.toDataURL().length + c.toDataURL().slice(100, 120).split("").reduce((a, ch) => a + ch.charCodeAt(0), 0);
  } catch (e) { sig = -2; }
  return { ct: v.currentTime, dur: v.duration, rs: v.readyState, sig, paused: v.paused,
           w: v.videoWidth, h: v.videoHeight };
})()
"""


async def main():
    ws_url = wait_page()
    print("page ok")
    async with websockets.connect(ws_url, max_size=64 * 1024 * 1024) as ws:
        cdp = Cdp(ws)

        card = await cdp.js(FIND_CARD_JS)
        print("card:", json.dumps(card, ensure_ascii=False))
        if "error" in card:
            return

        # ---- ① 悬停 scrub ----
        cx = card["x"] + card["w"] / 2
        cy = card["y"] + card["h"] / 2
        await cdp.mouse_move(cx, cy)
        # 等悬停层 video 就绪（300ms 进入延迟 + 加载，最长 8s）
        t0 = time.time()
        st = None
        while time.time() - t0 < 8:
            st = await cdp.js(VIDEO_STATE_JS)
            if st and st["rs"] >= 2:
                break
            await asyncio.sleep(0.2)
        print("hover video ready:", json.dumps(st))
        if not st or st["rs"] < 2:
            print("FAIL: 悬停层未就绪")
            return

        # 横向 20 步扫过卡片（66ms/步），每步后采样
        sigs = set()
        lags = []
        prev_ct = st["ct"]
        monotonic = True
        for i in range(20):
            ratio = (i + 0.5) / 20
            x = card["x"] + card["w"] * ratio
            tset = time.time()
            await cdp.mouse_move(x, cy)
            await asyncio.sleep(0.066)
            s = await cdp.js(VIDEO_STATE_JS)
            if not s:
                continue
            sigs.add(s["sig"])
            target = ratio * s["dur"]
            lags.append(abs(s["ct"] - target))
            if s["ct"] < prev_ct - 0.05:
                monotonic = False
            prev_ct = s["ct"]
        good = sum(1 for l in lags if l < 0.35)
        print(f"hover scrub: steps={len(lags)} 跟手(<0.35s)={good} 帧变化={len(sigs)-1} 单调前进={monotonic}")
        print(f"  lag 明细(秒): {[round(l, 2) for l in lags]}")

        # ---- ② 双击开预览 ----
        await cdp.js(
            "(() => { const el = document.elementFromPoint(%f, %f); "
            "el.dispatchEvent(new MouseEvent('dblclick', {bubbles: true})); return true; })()" % (cx, cy))
        t0 = time.time()
        pv = None
        while time.time() - t0 < 10:
            # 预览 = 页面上最大的 video
            pv = await cdp.js(r"""
(() => {
  const vs = [...document.querySelectorAll("video")];
  if (!vs.length) return null;
  let best = null, area = 0;
  for (const v of vs) {
    const r = v.getBoundingClientRect();
    if (r.width * r.height > area) { area = r.width * r.height; best = v; }
  }
  if (!best || area < 20000) return null;
  window.__pv = best;
  return { area, rs: best.readyState, dur: best.duration, w: best.videoWidth, h: best.videoHeight };
})()""")
            if pv and pv["rs"] >= 2:
                break
            await asyncio.sleep(0.3)
        print("preview video:", json.dumps(pv))
        if not pv:
            print("FAIL: 预览未打开")
            return

        # ---- ③ 预览播放 2.5s 帧率 ----
        play = await cdp.js(r"""
(async () => {
  const v = window.__pv;
  let frames = 0, stopped = false;
  const tick = () => { if (stopped) return; frames++; v.requestVideoFrameCallback(tick); };
  v.requestVideoFrameCallback(tick);
  const t0 = performance.now();
  await v.play();
  await new Promise((r) => setTimeout(r, 2500));
  stopped = true;
  const fps = frames / ((performance.now() - t0) / 1000);
  const drift = v.currentTime;
  v.pause();
  return { fps: Math.round(fps * 10) / 10, playedSecs: Math.round(drift * 100) / 100 };
})()""")
        print("preview play:", json.dumps(play))

        # ---- ④ 播放条 scrub（对预览 video 直接设值 = 播放条行为的内核；延迟口径一致）----
        scrub = await cdp.js(r"""
(async () => {
  const v = window.__pv;
  const out = [];
  for (let i = 0; i < 10; i++) {
    const target = v.duration * (i + 0.5) / 10;
    const t0 = performance.now();
    await new Promise((res, rej) => {
      const to = setTimeout(() => rej(new Error("seek timeout")), 8000);
      v.addEventListener("seeked", () => { clearTimeout(to); res(); }, { once: true });
      v.currentTime = target;
    });
    out.push(Math.round(performance.now() - t0));
  }
  return out;
})()""")
        print("preview scrub seeked ms:", scrub)

        # ---- ⑤ 逐帧步进（keyboard .）----
        step = await cdp.js(r"""
(async () => {
  const v = window.__pv;
  const fpsGuess = 30;
  const out = [];
  for (let i = 0; i < 8; i++) {
    const before = v.currentTime;
    const t0 = performance.now();
    const done = new Promise((res) => {
      const to = setTimeout(res, 3000);
      v.addEventListener("seeked", () => { clearTimeout(to); res(); }, { once: true });
    });
    document.dispatchEvent(new KeyboardEvent("keydown", { key: ".", bubbles: true }));
    await done;
    out.push({ ms: Math.round(performance.now() - t0), step: Math.round((v.currentTime - before) * 1000) / 1000 });
  }
  return out;
})()""")
        print("step frame:", json.dumps(step))


if __name__ == "__main__":
    asyncio.run(main())
