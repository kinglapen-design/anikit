# CDP 实测：docx 预览 mammoth 链路（2026-10-10 换引擎，PRD 04-04 条 6；初代 quick-xml 段落提取版作废——
# 用户实机「反馈文档 = 视频截图配文字」，表格平铺 / 无图片不达标）。
# 场景：A docx 卡片双击 → kind=docx 分派 + mammoth 转 HTML 渲染（标题 / 粗体 / 表格框线 / 图片内嵌
# data URI / 列表）——桩造真 docx（zip + 表格 + 内嵌 1px PNG，python zipfile 手搓 XML）；
# B showImg 白名单（.img-wrap 不出现——docx 的 img 只许长在 .docx-html 里）；C「默认应用打开」按钮；
# D 回归：png 卡片预览仍走 img。
# 桩模式不验后端（cargo test services::docx 覆盖字节回路）；文本卡懒加载仍走 read_docx_asset（桩同供）。
import asyncio
import base64
import io
import json
import subprocess
import sys
import tempfile
import time
import urllib.request
import zipfile

import websockets

sys.stdout.reconfigure(encoding="utf-8", errors="replace")

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9372
URL = "http://localhost:5373"

# 1×1 PNG（内嵌图夹具）
PNG_1PX = base64.b64decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==")

# 最小合法 docx：标题段 + 粗体段 + 2x2 表格 + 内嵌图段
DOCUMENT_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"
 xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"
 xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"
 xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"
 xmlns:pic="http://schemas.openxmlformats.org/drawingml/2006/picture"><w:body>
<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>分镜脚本 与 备注</w:t></w:r></w:p>
<w:p><w:r><w:t>主角从左侧入画，</w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>快步</w:t></w:r><w:r><w:t>走向讲台。</w:t></w:r></w:p>
<w:tbl>
<w:tr><w:tc><w:p><w:r><w:t>镜头</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>时长</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:p><w:r><w:t>全景</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2 秒</w:t></w:r></w:p></w:tc></w:tr>
</w:tbl>
<w:p><w:r><w:drawing><wp:inline><wp:extent cx="914400" cy="914400"/>
<a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/picture">
<pic:pic><pic:blipFill><a:blip r:embed="rId5"/></pic:blipFill></pic:pic>
</a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>
</w:body></w:document>"""

STYLES_XML = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:style w:type="paragraph" w:styleId="Heading1"><w:name w:val="heading 1"/></w:style>
</w:styles>"""

CONTENT_TYPES = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Default Extension="png" ContentType="image/png"/>
<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>
<Override PartName="/word/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.styles+xml"/>
</Types>"""

RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>
</Relationships>"""

DOC_RELS = """<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId5" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="media/img1.png"/>
</Relationships>"""


def build_docx_b64() -> str:
    buf = io.BytesIO()
    with zipfile.ZipFile(buf, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", CONTENT_TYPES)
        z.writestr("_rels/.rels", RELS)
        z.writestr("word/document.xml", DOCUMENT_XML)
        z.writestr("word/styles.xml", STYLES_XML)
        z.writestr("word/_rels/document.xml.rels", DOC_RELS)
        z.writestr("word/media/img1.png", PNG_1PX)
    return base64.b64encode(buf.getvalue()).decode()


STUB = r"""
window.__calls = [];
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 2, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "D1", folderId: "root", name: "分镜脚本.docx", w: null, h: null, ext: "docx",
        thumbPath: "E:/T/thumbs/docx.webp", thumbKind: "formatIcon", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
      { assetId: "P1", folderId: "root", name: "参考图.png", w: 100, h: 80, ext: "png",
        thumbPath: "E:/T/thumb.webp", thumbKind: "asset", origin: true,
        frameCount: null, animated: false, duration: null, bpm: null, importedFrom: null,
        noteCount: null, thumbFrame: null, fps: null },
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
    if (cmd === "read_docx_binary") {
      // 原始 IPC 正常路径回 ArrayBuffer；桩经 JSON 只能回 number[]——docxOps.readDocxBinary 有回退归一
      const u8 = Uint8Array.from(atob(window.__docxB64), (c) => c.charCodeAt(0));
      return Promise.resolve(Array.from(u8));
    }
    if (cmd === "read_docx_asset") {
      // 文本卡懒加载仍走段落提取（卡片不吃 mammoth）
      return Promise.resolve({ paragraphs: [{ heading: 0, list: false, runs: [{ text: "卡片正文", bold: false, italic: false }] }], truncated: false });
    }
    if (cmd === "get_asset_file_path") return Promise.resolve({ path: "E:/T/src.png", pending: false, animPreview: null });
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
window.__docxB64 = "__DOCX_B64__";
""".replace("__DOCX_B64__", build_docx_b64())

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
    profile = tempfile.mkdtemp(prefix="cdp-docx-")
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
                if cards > 0:
                    break
            if cards < 2:
                print("FAIL: 卡片未渲染（", cards, "）")
                return 3

            fails = []

            async def open_card(i, wait=2.0):
                await ev(f"document.querySelectorAll('.asset-card')[{i}].dispatchEvent(new MouseEvent('dblclick', {{ bubbles: true }}))")
                await asyncio.sleep(wait)

            async def close_preview():
                await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
                await asyncio.sleep(0.8)

            async def shot(name):
                rect = await ev("(() => { const r = document.querySelector('.preview-stage')?.getBoundingClientRect(); return r ? { x: r.left, y: r.top, width: r.width, height: r.height } : null; })()")
                if not rect:
                    return
                s = await send(ws, "Page.captureScreenshot", {
                    "format": "png",
                    "clip": {"x": rect["x"], "y": rect["y"], "width": rect["width"], "height": rect["height"], "scale": 1},
                }, session=sess)
                out = f"tools/style-preview/{name}.png"
                with open(out, "wb") as f:
                    f.write(base64.b64decode(s["result"]["data"]))
                print("截图:", out)

            # —— 场景 A/B/C：docx mammoth 渲染 + showImg 白名单 + 打开按钮 ——
            await open_card(0, 2.5)
            a = await ev("""(() => {
              const stage = document.querySelector('.preview-stage');
              const root = document.querySelector('.docx-preview');
              if (!stage || !root) return null;
              const body = root.querySelector('.docx-html');
              if (!body) return { noHtml: true };
              const imgs = body.querySelectorAll('img');
              return {
                wrapImgs: stage.querySelectorAll('.img-wrap img').length,
                h1: body.querySelector('h1')?.textContent ?? null,
                bold: [...body.querySelectorAll('strong')].map((s) => s.textContent),
                tables: body.querySelectorAll('table').length,
                cells: [...body.querySelectorAll('td')].map((c) => c.textContent),
                imgs: imgs.length,
                imgSrc: imgs[0]?.src.slice(0, 30) ?? null,
                binaryCalls: window.__calls.filter((c) => c.cmd === 'read_docx_binary').map((c) => c.args.assetId),
              };
            })()""")
            print("[docx mammoth]", json.dumps(a, ensure_ascii=False))
            if not a:
                fails.append("[docx] 预览组件未渲染（kind 分派 / spec 注册未命中）")
            elif a.get("noHtml"):
                fails.append("[docx] mammoth HTML 容器未出现（转换失败）")
            else:
                if a["wrapImgs"] != 0:
                    fails.append(f"[docx] showImg 白名单失效：.img-wrap 出现 img ×{a['wrapImgs']}")
                if a["h1"] != "分镜脚本 与 备注":
                    fails.append(f"[docx] h1 不符（{a['h1']}）——styles.xml heading 1 映射未生效")
                if a["bold"] != ["快步"]:
                    fails.append(f"[docx] 粗体不符（{a['bold']}）")
                if a["tables"] != 1 or a["cells"] != ["镜头", "时长", "全景", "2 秒"]:
                    fails.append(f"[docx] 表格不符（tables={a['tables']} cells={a['cells']}）")
                if a["imgs"] != 1 or not (a["imgSrc"] or "").startswith("data:image/png"):
                    fails.append(f"[docx] 图片内嵌不符（imgs={a['imgs']} src={a['imgSrc']}）")
                if a["binaryCalls"] != ["D1"]:
                    fails.append(f"[docx] read_docx_binary 调用不符（{a['binaryCalls']}）")
            # 「默认应用打开」按钮：IconButton title 走 v-tip 不落 DOM 属性——按按钮组位序点击
            # （插件 / 默认应用打开 / 新窗口 / 上 / 下一个，docx 非 viewable 无缩放组 = 索引 1），
            # 以 open_with_default_app 调用反证按钮存在且接线活
            await ev("document.querySelectorAll('.preview-titlebar .btn-group .ic-btn')[1]?.click()")
            await asyncio.sleep(0.5)
            od = await ev("window.__calls.filter((c) => c.cmd === 'open_with_default_app').map((c) => c.args.assetId)")
            print("[docx] 默认应用打开调用:", json.dumps(od, ensure_ascii=False))
            if od != ["D1"]:
                fails.append(f"[docx] 默认应用打开按钮缺失 / 接线不符（{od}）")
            await shot("docx-preview")
            await close_preview()

            # —— 场景 D：回归——png 卡片预览仍走 img（白名单不误伤 image kind） ——
            await open_card(1, 2.0)
            d = await ev("""(() => ({
              imgInStage: document.querySelectorAll('.preview-stage .img-wrap img').length,
              docxPane: !!document.querySelector('.docx-preview'),
            }))()""")
            print("[回归-png]", json.dumps(d, ensure_ascii=False))
            if not d or d["imgInStage"] != 1:
                fails.append(f"[回归] png 预览 img 缺失（{d}）")
            if d and d["docxPane"]:
                fails.append("[回归] png 预览挂出 docx 组件（spec 查表串线）")

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: docx mammoth 预览链路成立（kind 分派 / 标题粗体 / 表格 / 图片内嵌 / showImg 白名单 / 打开按钮 / png 回归）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
