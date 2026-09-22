# CDP 实测：文本卡接线（2026-09-22 R6 三变体 · text 变体生产接线，稿 = 文件卡片-txt-md）。
# 场景：A txt / md / docx 三卡内容区 = .fc-text 文本滚动预览（标题词干 + 正文，数据源
# read_text_asset / read_docx_asset 桩断言）；B 滚动容器 overflow-y auto（滚轮可滚 =
# overflow 容器原生吃滚轮）；C 回归：png 卡仍 general 壳无 .fc-text、无读文本调用。
# 桩环境懒加载由虚拟滚动可见性驱动（渲染即拉取），断言正文非空即链路成立。
import asyncio
import base64
import json
import subprocess
import sys
import tempfile
import time
import urllib.request

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9373
URL = "http://localhost:5373"

LONG_MD = "\n\n".join(f"第{i}段 金属怪人巢穴场景。" for i in range(1, 60))

STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 4, folderCount: 0, autoImport: false };
    const mk = (assetId, name, ext) => ({ assetId, folderId: "root", name, w: null, h: null, ext,
      thumbPath: null, thumbKind: "formatIcon", origin: true,
      frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
      noteCount: null, thumbFrame: null, fps: null });
    const placements = [
      mk("M1", "镜头1_提示词.md", "md"),
      mk("T1", "场记.txt", "txt"),
      mk("D1", "分镜脚本.docx", "docx"),
      { assetId: "P1", folderId: "root", name: "参考图.png", w: 100, h: 80, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      // 凑行数让网格视口可滚（滚轮交还页面断言用）
      ...Array.from({ length: 8 }, (_, i) => ({ assetId: `PX${i}`, folderId: "root", name: `垫图${i}.png`, w: 100, h: 80, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null })),
    ];
    const R = {
      get_current_library: LIB,
      get_ui_settings: { autoPlayGif: false },
      list_placements: placements, list_folder_tree: { blocks: [], nodes: [] }, list_recycle: [], list_recent_assets: [],
      list_archives: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_space_libraries: ["旧库"],
      list_recent_libraries: [{ name: "旧库", path: "E:/T/S/旧库", lastOpenedAt: "" }],
      get_notes: [],
      prepare_anim_strips: { items: [] },
    };
    if (cmd === "read_text_asset") {
      if (args.assetId === "M1") return Promise.resolve({ content: __LONG_MD__ });
      return Promise.resolve({ content: "场记正文第一行\n第二行" });
    }
    if (cmd === "read_docx_asset") {
      return Promise.resolve({ paragraphs: [
        { heading: 1, list: false, runs: [{ text: "分镜脚本", bold: false, italic: false }] },
        { heading: 0, list: false, runs: [{ text: "主角入场。", bold: true, italic: false }] },
      ], truncated: false });
    }
    if (cmd === "get_asset_file_path") return Promise.resolve({ path: "E:/T/src.png", pending: false, animPreview: null });
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
""".replace("__LONG_MD__", json.dumps(LONG_MD))

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
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = [str(a.get("value", a.get("description", "")))[:200] for a in raw["params"]["args"]]
            print("[CONSOLE-ERR]", " ".join(args)[:400])


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-text-card-")
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", f"--user-data-dir={profile}",
         "--headless=new", "--disable-gpu", "--window-size=1400,640", "about:blank"],
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

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            cards = 0
            for _ in range(40):
                await asyncio.sleep(0.5)
                cards = await ev("document.querySelectorAll('.asset-card').length") or 0
                if cards >= 4:
                    break
            if cards < 4:
                print("FAIL: 卡片未渲染（", cards, "）")
                return 3

            # 等文本懒加载落地（md 卡正文出现第 59 段 = 全文到达）
            ready = False
            for _ in range(30):
                await asyncio.sleep(0.3)
                ready = await ev("""(() => {
                  const bodies = [...document.querySelectorAll('.fc-text-body')].map((b) => b.textContent || '');
                  return bodies.length === 3 && bodies.every((t) => t.length > 0);
                })()""")
                if ready:
                    break

            fails = []

            a = await ev("""(() => {
              const cards = [...document.querySelectorAll('.asset-card')];
              const rows = cards.map((c) => {
                const name = c.querySelector('.kc-name, .fc-name, [class*="name"]')?.textContent ?? '';
                const t = c.querySelector('.fc-text');
                return {
                  name,
                  hasText: !!t,
                  title: c.querySelector('.fc-text-title')?.textContent ?? null,
                  bodyHead: (c.querySelector('.fc-text-body')?.textContent ?? '').slice(0, 20),
                  overflowY: t ? getComputedStyle(t).overflowY : null,
                  bodySize: t ? getComputedStyle(t.querySelector('.fc-text-body')).fontSize : null,
                  topInset: t ? t.style.top : null,
                  badgeText: c.querySelector('.fc-badge')?.textContent ?? null,
                };
              });
              return {
                rows,
                reads: window.__calls.filter((c) => c.cmd === 'read_text_asset').map((c) => c.args.assetId),
                docxReads: window.__calls.filter((c) => c.cmd === 'read_docx_asset').map((c) => c.args.assetId),
              };
            })()""")
            print("[卡片]", json.dumps(a, ensure_ascii=False)[:1200])

            byName = {r["name"].split(".")[0]: r for r in (a["rows"] if a else [])}
            md = byName.get("镜头1_提示词")
            txt = byName.get("场记")
            docx = byName.get("分镜脚本")
            png = byName.get("参考图")
            for label, row in (("md", md), ("txt", txt), ("docx", docx)):
                if not row or not row["hasText"]:
                    fails.append(f"[{label}] 文本卡未渲染 .fc-text")
                    continue
                if not row["title"]:
                    fails.append(f"[{label}] 词干标题缺失")
                if not row["bodyHead"]:
                    fails.append(f"[{label}] 正文为空")
                if row["overflowY"] != "hidden":
                    fails.append(f"[{label}] 滚动容器 overflowY={row['overflowY']}（期望 hidden——滚轮交还页面）")
                if row["bodySize"] != "13px":
                    fails.append(f"[{label}] 正文字号 {row['bodySize']}（期望 13px 固定稿值）")
                # 角标让位：角标在 = 36px（角标下缘 + 4）；桩环境角标缺失时降级 12px 并诊断输出
                expect_top = "36px" if row["badgeText"] else "12px"
                if row["topInset"] != expect_top:
                    fails.append(f"[{label}] 角标让位 top={row['topInset']}（期望 {expect_top}，badge={row['badgeText']}）")
            if md and "第59段" not in (md.get("bodyHead") or "") and not (await ev("([...document.querySelectorAll('.fc-text-body')].find((b) => b.textContent.includes('第59段'))?.textContent.length) ?? 0")):
                fails.append("[md] 长文未到达（截断或懒加载失败）")
            if docx and docx["bodyHead"] != "分镜脚本\n主角入场。":
                fails.append(f"[docx] 正文拼接不符（{docx}）")
            if png and png["hasText"]:
                fails.append("[回归] png 卡误挂文本变体")
            if sorted(a["reads"] if a else []) != ["M1", "T1"]:
                fails.append(f"read_text_asset 调用不符（{a['reads']}）")
            if (a["docxReads"] if a else []) != ["D1"]:
                fails.append(f"read_docx_asset 调用不符（{a['docxReads']}）")

            # 卡内程序滚动（胶囊驱动路径）：scrollTop 可写 + 拇指位置跟随（Vue 异步 patch，等一拍再读）
            scroll = await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              if (!t) return null;
              const card = t.closest('.fc-root');
              const thumbEl = card?.querySelector('.fc-text-thumb');
              const thumbRect = thumbEl?.getBoundingClientRect();
              return {
                overflow: t.scrollHeight > t.clientHeight,
                thumbVisible: thumbEl ? getComputedStyle(thumbEl).display !== 'none' : null,
                thumbRect: thumbRect ? { x: thumbRect.x, y: thumbRect.y, w: thumbRect.width, h: thumbRect.height } : null,
              };
            })()""")
            print("[胶囊状态]", json.dumps(scroll, ensure_ascii=False))
            if not scroll or not scroll["overflow"]:
                fails.append(f"[md] 内容未溢出（{scroll}）")
            if scroll and scroll["thumbVisible"] is not True:
                fails.append(f"[md] 溢出但胶囊未显（{scroll}）")
            await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              t.scrollTop = 300; t.dispatchEvent(new Event('scroll'));
            })()""")
            await asyncio.sleep(0.3)
            follow = await ev("parseFloat([...document.querySelectorAll('.fc-text-thumb')].find((x) => x.getBoundingClientRect().height > 0)?.style.top ?? '0')")
            if not follow or follow <= 12:
                fails.append(f"[md] 拇指未跟随滚动（top={follow}）")

            # ③ 胶囊可拖（先做——后续滚轮段会把网格滚走致卡出视口）：按下拇指下移 → scrollTop 推进
            # 时序纪律：scrollTop 复位后等 Vue patch 再读 rect（patch 异步，立即读 = 过期坐标打歪）
            await ev("""(() => { const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段')); t.scrollTop = 0; t.dispatchEvent(new Event('scroll')); })()""")
            await asyncio.sleep(0.3)
            tr = await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              const thumbEl = t.closest('.fc-root').querySelector('.fc-text-thumb');
              const r = thumbEl.getBoundingClientRect();
              return { x: r.left + r.width / 2, y: r.top + r.height / 2, h: r.height };
            })()""")
            print("[胶囊坐标]", json.dumps(tr, ensure_ascii=False))
            # 诊断：命中测试（坐标处最上层应为胶囊本体）
            diag = await ev(f"document.elementFromPoint({tr['x']}, {tr['y']})?.className ?? null")
            print("[命中]", json.dumps(diag, ensure_ascii=False))
            if diag != "fc-text-thumb":
                fails.append(f"[胶囊] 坐标命中 {diag} 而非胶囊（拖动必然落空）")
            await send(ws, "Input.dispatchMouseEvent", {"type": "mousePressed", "x": tr["x"], "y": tr["y"], "button": "left", "clickCount": 1}, session=sess)
            for dy in (30, 60, 90):
                await send(ws, "Input.dispatchMouseEvent", {"type": "mouseMoved", "x": tr["x"], "y": tr["y"] + dy, "buttons": 1}, session=sess)
                await asyncio.sleep(0.05)
            await send(ws, "Input.dispatchMouseEvent", {"type": "mouseReleased", "x": tr["x"], "y": tr["y"] + 90, "button": "left", "clickCount": 1}, session=sess)
            await asyncio.sleep(0.4)
            drag = await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              return { cardScroll: t.scrollTop };
            })()""")
            print("[胶囊拖动]", json.dumps(drag, ensure_ascii=False))
            if drag["cardScroll"] <= 0:
                fails.append(f"[胶囊] 拖动未驱动滚动（scrollTop={drag['cardScroll']}）")

            # —— 走查修订断言（2026-09-22） ——
            # ② 滚轮交还页面：网格 = 自定义 transform 滚动（useVirtualScroll，踩坑 005——
            # viewport wheel preventDefault + pos 平移 canvas）。mouseWheel 落 md 卡文字上：
            # fc-text 不滚（hidden 非滚轮目标 + viewport preventDefault 双保险）、canvas transform 变
            pos = await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              const r = t.getBoundingClientRect();
              const canvas = document.querySelector('.grid-canvas');
              return { x: r.left + r.width / 2, y: r.top + 60, cardScroll: t.scrollTop,
                       canvasTf: canvas.style.transform };
            })()""")
            await send(ws, "Input.dispatchMouseEvent", {
                "type": "mouseWheel", "x": pos["x"], "y": pos["y"], "deltaX": 0, "deltaY": 480,
            }, session=sess)
            await asyncio.sleep(0.6)
            wheel = await ev("""(() => {
              const t = [...document.querySelectorAll('.fc-text')].find((x) => x.textContent.includes('第59段'));
              return { cardScroll: t.scrollTop,
                       canvasTf: document.querySelector('.grid-canvas').style.transform };
            })()""")
            print("[滚轮交还页面]", json.dumps({**pos, **wheel}, ensure_ascii=False))
            if wheel["cardScroll"] != pos["cardScroll"]:
                fails.append(f"[滚轮] 卡内被滚轮驱动（{pos['cardScroll']} → {wheel['cardScroll']}）")
            if wheel["canvasTf"] == pos["canvasTf"]:
                fails.append(f"[滚轮] 网格未滚动（canvas transform 恒 {pos['canvasTf']}）")
            # 滚回顶（截图视野复位）
            await send(ws, "Input.dispatchMouseEvent", {
                "type": "mouseWheel", "x": pos["x"], "y": pos["y"], "deltaX": 0, "deltaY": -2000,
            }, session=sess)
            await asyncio.sleep(0.5)

            # 截图（首张卡区域）
            rect = await ev("(() => { const r = document.querySelector('.asset-card')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width * 2 + 24, height: r.height } : null; })()")
            if rect:
                s = await send(ws, "Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 1},
                }, session=sess)
                with open("tools/style-preview/text-card.png", "wb") as f:
                    f.write(base64.b64decode(s["result"]["data"]))
                print("截图: tools/style-preview/text-card.png")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: 文本卡接线成立（txt/md/docx 文本滚动预览 / 数据源桩断言 / 卡内滚动 / png 回归）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
