# CDP 实测：MarkDown 文本编辑器（文本编辑稿轮，2026-09 排版 + 编辑器；2026-10-10 CM6 Live Preview 换代适配）。
# 场景：A 预览态排版结构 + 贴稿 computed 数值（H1 30/40 + 2px g3 分割线 / H2 26/36 / 正文 15/25 /
# 引用 2px g3 竖线 / 代码块 g2 底 + 复制钮）；B 任务复选框点击 toggle → 防抖回写 write_text_asset；
# C 代码块复制钮 → 剪贴板（桩 stub）；D 铅笔钮进编辑态（CM6 .cm-editor + 悬浮栏换 MarkDown 工具栏）；
# E 工具栏 bold / h1 动作作用源码选区；F Esc 退出回渲染态 + 悬浮栏退场；G png 预览回归。
# I 撤销 = CM6 原生事务栈（userEvent 分步：input.type 键入合并 / md.toolbar.N 独立步）；
# P Live Preview 装饰层（光标外隐标记 + 行级排版 + 任务框 widget）。
# 桩模式不验真后端读写（cargo test 覆盖 text 命令回路）。
# CM6 驱动口径：实例经宿主钩子拿（.cm-host.__cmView，lib/cmMd.ts 探针钩子）；
# 选区 / 键入 = view.dispatch（updateListener 自动同步 draft）；快捷键 = 合成 keydown 派发 .cm-content。
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
PORT = 9374
URL = "http://localhost:5373"

MD_SRC = """# 方舟前史

## 1._创世纪

### 1.1_亚当与夏娃

夏娃摘下一颗苹果，开启了人类的时代。

> 引用一行

```
代码块内容
```

- 无序一项
- [ ] 未完成任务
- [x] 已完成任务

1. 有序一项

- 父级一项
  - 嵌套一项

| 名称 | 数值 |
| --- | --- |
| 正文 | 15 |

```
```
"""

STUB = r"""
window.__calls = [];
window.__clip = [];
Object.defineProperty(navigator, "clipboard", { value: { writeText: (t) => { window.__clip.push(t); return Promise.resolve(); } } });
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  invoke: (cmd, args) => {
    window.__calls.push({ cmd, args });
    const LIB = { name: "旧库", path: "E:/T/S/旧库", assetCount: 2, folderCount: 0, autoImport: false };
    const placements = [
      { assetId: "M1", folderId: "root", name: "01_人物.md", w: null, h: null, ext: "md",
        thumbPath: "E:/T/thumbs/md.webp", thumbKind: "formatIcon", origin: true,
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
    if (cmd === "read_text_asset") return Promise.resolve({ content: window.__mdSrc });
    if (cmd === "write_text_asset") return Promise.resolve({ size: 100, modifiedAt: "2026-09-26 00:00:00" });
    if (cmd === "get_asset_file_path") return Promise.resolve({ path: "E:/T/src.png", pending: false, animPreview: null });
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
  transformCallback: () => 0,
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
};
window.__mdSrc = __MD_SRC__;
""".replace(
    "__MD_SRC__", json.dumps(MD_SRC)
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
        if raw.get("method") == "Runtime.exceptionThrown":
            d = raw["params"]["exceptionDetails"]
            print("[EXCEPTION]", str(d.get("exception", {}).get("description", d.get("text")))[:500])
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] == "error":
            args = [str(a.get("value", a.get("description", "")))[:200] for a in raw["params"]["args"]]
            print("[CONSOLE-ERR]", " ".join(args)[:400])


async def main():
    profile = tempfile.mkdtemp(prefix="cdp-md-")
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

            # —— 场景 A：预览态排版结构 + 贴稿 computed 数值 ——
            await ev("document.querySelectorAll('.asset-card')[0].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(2.0)
            a = await ev("""(() => {
              const body = document.querySelector('.text-preview .md-body');
              if (!body) return null;
              const cs = (sel, prop) => { const el = body.querySelector(sel); return el ? getComputedStyle(el)[prop] : null; };
              const h1 = body.querySelector('h1');
              const h1cs = h1 ? getComputedStyle(h1) : null;
              const pre = body.querySelector('.md-code pre');
              const precs = pre ? getComputedStyle(pre) : null;
              return {
                h1text: h1?.textContent ?? null,
                h1fs: h1cs?.fontSize, h1lh: h1cs?.lineHeight, h1bb: h1cs?.borderBottomWidth, h1bbc: h1cs?.borderBottomColor, h1c: h1cs?.color,
                h2fs: cs('h2', 'fontSize'), h2lh: cs('h2', 'lineHeight'),
                h3fs: cs('h3', 'fontSize'), h3lh: cs('h3', 'lineHeight'),
                pfs: cs('p', 'fontSize'), plh: cs('p', 'lineHeight'), pc: cs('p', 'color'),
                bqbl: cs('blockquote', 'borderLeftWidth'), bqblc: cs('blockquote', 'borderLeftColor'), bqc: cs('blockquote', 'color'),
                prebg: precs?.backgroundColor, prebc: precs?.borderColor, prebr: precs?.borderRadius, prews: precs?.whiteSpace,
                codefs: cs('.md-code code', 'fontSize'), codelh: cs('.md-code code', 'lineHeight'), codec: cs('.md-code code', 'color'),
                copyBtn: !!body.querySelector('.md-code .md-copy'),
                tasks: body.querySelectorAll('li.md-task').length,
                taskBoxes: body.querySelectorAll('li.md-task > .kit-check-box').length,
                taskOn: body.querySelectorAll('li.md-task > .kit-check-box.on').length,
                taskLines: [...body.querySelectorAll('.kit-check-box')].map((b) => b.dataset.taskLine),
                taskText0: body.querySelectorAll('li.md-task')[0]?.textContent.trim() ?? null,
                gap: getComputedStyle(body).rowGap,
                padL: getComputedStyle(body).paddingLeft,
                padB: getComputedStyle(body).paddingBottom,
              };
            })()""")
            print("[A 预览态]", json.dumps(a, ensure_ascii=False))
            if not a:
                fails.append("[A] md 渲染体未渲染")
            else:
                exp = {
                    "h1fs": "30px", "h1lh": "40px", "h1bb": "2px",
                    "h1bbc": "rgb(36, 36, 36)", "h1c": "rgb(204, 204, 204)",
                    "h2fs": "26px", "h2lh": "36px", "h3fs": "22px", "h3lh": "32px",
                    "pfs": "15px", "plh": "25px", "pc": "rgb(166, 166, 166)",
                    "bqbl": "2px", "bqblc": "rgb(36, 36, 36)", "bqc": "rgb(115, 115, 115)",
                    "prebg": "rgb(28, 28, 28)", "prebr": "8px", "prews": "pre-wrap",
                    "codefs": "14px", "codelh": "24px", "codec": "rgb(115, 115, 115)",
                    "gap": "12px", "padL": "84px", "padB": "66px",
                }
                for k, want in exp.items():
                    if a.get(k) != want:
                        fails.append(f"[A] {k} = {a.get(k)}（期望 {want}）")
                if a["tasks"] != 2 or a["taskBoxes"] != 2 or a["taskOn"] != 1:
                    fails.append(f"[A] 任务列表结构不符（tasks={a['tasks']} boxes={a['taskBoxes']} on={a['taskOn']}）")
                if a["taskLines"] != ["15", "16"]:
                    fails.append(f"[A] 任务行号不符（{a['taskLines']}，期望 ['15','16']）")
                if a["taskText0"] != "未完成任务":
                    fails.append(f"[A] 任务文本前缀剥离失败（{a['taskText0']}）")
                if not a["copyBtn"]:
                    fails.append("[A] 代码块复制钮缺失")
            await shot("md-preview")

            # —— 场景 A2：空代码块最小高度（2026-10-11 用户实机报：空块装不下复制钮溢出）——
            # 46 = 钮距顶 8 + 钮 30 + 下 8；断言文末空块 pre 达 46 且钮底缘不出容器
            a2 = await ev("""(() => {
              const pres = [...document.querySelectorAll('.text-preview .md-body .md-code pre')];
              const empty = pres[pres.length - 1];
              if (!empty) return null;
              const btn = empty.closest('.md-code').querySelector('.md-copy');
              const pr = empty.getBoundingClientRect(), br = btn.getBoundingClientRect();
              return { minH: getComputedStyle(empty).minHeight, h: Math.round(pr.height),
                       btnIn: br.bottom <= pr.bottom + 0.5 };
            })()""")
            print("[A2 空代码块]", json.dumps(a2, ensure_ascii=False))
            if not a2 or a2["minH"] != "46px" or a2["h"] < 46 or a2["btnIn"] is not True:
                fails.append(f"[A2] 空代码块复制钮溢出未根治（{a2}，期望 min-height 46px 且钮在容器内）")

            # —— 场景 J：三列表对齐（2026-09-25 走查）——任务文本列 = 无序/有序文本列；复选框挂 marker 列（文本列 -20） ——
            j = await ev("""(() => {
              const body = document.querySelector('.text-preview .md-body');
              if (!body) return null;
              const textLeft = (li) => li.getBoundingClientRect().left + parseFloat(getComputedStyle(li).paddingLeft);
              const plain = body.querySelector('ul li:not(.md-task)');
              const task = body.querySelector('li.md-task');
              const olLi = body.querySelector('ol li');
              const box = task.querySelector('.kit-check-box');
              return { plain: textLeft(plain), task: textLeft(task), ol: textLeft(olLi),
                       box: box.getBoundingClientRect().left };
            })()""")
            print("[J 对齐]", json.dumps(j, ensure_ascii=False))
            if not j:
                fails.append("[J] 列表元素缺失")
            else:
                if abs(j["plain"] - j["task"]) > 0.6 or abs(j["plain"] - j["ol"]) > 0.6:
                    fails.append(f"[J] 文本列不齐（无序 {j['plain']} / 任务 {j['task']} / 有序 {j['ol']}）")
                if abs(j["box"] - (j["plain"] - 20)) > 1.0:
                    fails.append(f"[J] 复选框未挂 marker 列（框 {j['box']}，期望 ≈{j['plain'] - 20}）")

            # —— 场景 M：预览态点击文字不进编辑（2026-09-25 走查「只想复制文字」）——
            await ev("document.querySelector('.text-preview .md-body p').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.5)
            m = await ev("""(() => ({
              cm: !!document.querySelector('.text-preview .cm-editor'),
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              groups: document.querySelectorAll('.fbm-root > div').length,
              viewBtns: document.querySelectorAll('.fbm-root > div')[0]?.querySelectorAll('button').length ?? -1,
            }))()""")
            print("[M 预览不主动编辑]", json.dumps(m, ensure_ascii=False))
            if not m or m["cm"]:
                fails.append(f"[M] 预览态点击文字误进编辑（{m}）")
            if m and m["mdBar"] is not True:
                fails.append(f"[M] md 语境悬浮栏未恒显（{m}）")
            if m and (m["groups"] != 1 or m["viewBtns"] != 2):
                fails.append(f"[M] 预览态工具栏应为仅视图组 2 钮（groups={m['groups']} btns={m['viewBtns']}）")

            # —— 场景 B：任务复选框 toggle → 防抖回写 ——
            await ev("document.querySelector('.kit-check-box[data-task-line=\\'15\\']').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(1.8)  # 防抖 1s + 余量
            b = await ev("""(() => ({
              writes: window.__calls.filter((c) => c.cmd === 'write_text_asset').map((c) => c.args.content),
              onNow: document.querySelector('.kit-check-box[data-task-line=\\'15\\']')?.classList.contains('on') ?? null,
            }))()""")
            print("[B toggle]", json.dumps({**b, "writes": [w[:60] for w in (b or {}).get("writes", [])]}, ensure_ascii=False))
            if not b or not b["writes"]:
                fails.append("[B] write_text_asset 未触发（防抖回写失败）")
            elif "- [x] 未完成任务" not in b["writes"][-1]:
                fails.append("[B] 回写内容未翻转复选框（" + b["writes"][-1][-60:] + "）")
            if b and b["onNow"] is not True:
                fails.append(f"[B] toggle 后复选框 on 态丢失（{b and b['onNow']}）")

            # —— 场景 C：代码块复制钮 ——
            await ev("document.querySelector('.md-code .md-copy').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.4)
            c = await ev("window.__clip")
            print("[C 复制]", json.dumps(c, ensure_ascii=False))
            if not c or "代码块内容" not in c[-1]:
                fails.append(f"[C] 剪贴板内容不符（{c}）")

            # —— 场景 D：铅笔钮进编辑态（进编辑唯一入口 = 铅笔 / Ctrl+E）+ 壳宽自动展开（RO 根治） ——
            d0 = await ev("document.querySelector('.fb-shell')?.offsetWidth ?? 0")  # 预览态壳宽（视图组 2 钮）
            await ev("document.querySelectorAll('.fbm-root > div')[0].querySelectorAll('button')[0].click()")
            await asyncio.sleep(1.2)  # 组渲染 + RO 重测 + 300ms 壳宽动画
            d = await ev("""(() => ({
              cm: !!document.querySelector('.text-preview .cm-editor'),
              lp: document.querySelectorAll('.text-preview .cm-line.cm-lp-h1').length > 0,
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              shellGone: document.querySelector('.fb-shell')?.classList.contains('gone') ?? null,
              groups: document.querySelectorAll('.fbm-root > div').length,
              shellW: document.querySelector('.fb-shell')?.offsetWidth ?? 0,
              clipOk: (() => { const r = document.querySelector('.fbm-root'); return r ? r.scrollWidth <= document.querySelector('.fb-shell').offsetWidth + 2 : null; })(),
              mdBodyVisible: !!document.querySelector('.text-preview .md-body')?.offsetParent,
            }))()""")
            print("[D 铅笔进编辑]", json.dumps({"预览壳宽": d0, **(d or {})}, ensure_ascii=False))
            if not d or not d["cm"]:
                fails.append("[D] 铅笔钮未进编辑态（CM6 未出现）")
            if d and d["lp"] is not True:
                fails.append("[D] Live Preview 行装饰缺失（cm-lp-h1 未挂）")
            if not d or not d["mdBar"]:
                fails.append("[D] 悬浮栏未切 MarkDown 工具栏")
            if d and d["groups"] != 5:
                fails.append(f"[D] 编辑态工具栏应为 4 格式组 + 视图组 = 5 组（{d['groups']}）")
            if d and d0 and d["shellW"] <= d0:
                fails.append(f"[D] 工具栏没展开——壳宽未随组数增大（预览 {d0} → 编辑 {d['shellW']}）")
            if d and d["clipOk"] is not True:
                fails.append(f"[D] 工具栏内容被壳裁剪（scrollWidth 超壳宽，壳宽 {d['shellW']}）")

            # —— 场景 E：工具栏动作（bold 包裹选区 / h1 行首；选区 = __cmView.dispatch 驱动） ——
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              const i = v.state.doc.toString().indexOf('方舟前史');
              v.dispatch({ selection: { anchor: i, head: i + 4 } });
              v.focus();
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].click()")  # 格式组·bold
            await asyncio.sleep(0.3)
            e1 = await ev("document.querySelector('.text-preview .cm-host').__cmView.state.doc.toString().includes('**方舟前史**')")
            print("[E1 bold]", e1)
            if e1 is not True:
                fails.append(f"[E1] bold 包裹失败（{e1}）")
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              const i = v.state.doc.toString().indexOf('夏娃摘下一颗苹果');
              v.dispatch({ selection: { anchor: i, head: i + 2 } });
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[0].querySelectorAll('button')[0].click()")  # 段落组·h1
            await asyncio.sleep(0.3)
            e2 = await ev("document.querySelector('.text-preview .cm-host').__cmView.state.doc.toString().includes('# 夏娃摘下一颗苹果')")
            print("[E2 h1]", e2)
            if e2 is not True:
                fails.append(f"[E2] h1 行首失败（{e2}）")
            await shot("md-editing")

            # —— 场景 P：Live Preview 装饰层（2026-10-10 CM6 换代）——光标外隐标记 + 行级排版 + widget；光标行回落源码 ——
            # 此刻文档 = 「# **方舟前史**」H1 + 「# 夏娃…」H1（E2 选区停该行 = active）、任务 ×2（1 on）、代码块 3 行
            p1 = await ev("""(() => {
              const host = document.querySelector('.text-preview .cm-host');
              const lines = [...host.querySelectorAll('.cm-line')];
              return {
                h1: lines.filter((l) => l.classList.contains('cm-lp-h1')).map((l) => l.textContent),
                tasks: host.querySelectorAll('.cm-lp-task').length,
                taskOn: host.querySelectorAll('.cm-lp-task.on').length,
                codeLines: host.querySelectorAll('.cm-lp-code-line').length,
                codeFirst: host.querySelectorAll('.cm-lp-code-first').length,
                codeLast: host.querySelectorAll('.cm-lp-code-last').length,
                quote: host.querySelectorAll('.cm-lp-quote').length,
              };
            })()""")
            print("[P1 光标外装饰]", json.dumps(p1, ensure_ascii=False))
            if not p1 or len(p1["h1"]) != 2:
                fails.append(f"[P] H1 行装饰不符（{p1 and p1['h1']}，期望 2 行）")
            else:
                if p1["h1"][0] != "方舟前史":
                    fails.append(f"[P] 光标外 H1 未隐标记（{p1['h1'][0]}，期望 方舟前史）")
                if not p1["h1"][1].startswith("# "):
                    fails.append(f"[P] 光标行未回落源码（{p1['h1'][1]}，期望 # 前缀可见）")
            if not p1 or p1["tasks"] != 2 or p1["taskOn"] != 2:
                fails.append(f"[P] 任务框 widget 不符（{p1}，期望 2 框 2 on——场景 B 已把首任务勾上）")
            if p1 and (p1["codeLines"] != 5 or p1["codeFirst"] != 2 or p1["codeLast"] != 2):
                fails.append(f"[P] 代码块卡片行装饰不符（{p1}，期望 5 行 2 首 2 末——含文末空代码块两栅线行）")
            if p1 and p1["quote"] < 1:
                fails.append(f"[P] 引用行装饰缺失（{p1}）")
            # 光标移进首行 → 该行回落源码（# 与 ** 标记现身）
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              v.dispatch({ selection: { anchor: 2 } });
            })()""")
            await asyncio.sleep(0.4)
            p2 = await ev("document.querySelector('.text-preview .cm-host .cm-line')?.textContent ?? null")
            print("[P2 光标行回落]", json.dumps(p2, ensure_ascii=False))
            if p2 is None or not p2.startswith("# **"):
                fails.append(f"[P] 光标行源码回落失败（{p2}，期望 '# **方舟前史**'）")

            # —— 场景 Q：编辑态方向键归光标（2026-10-10 用户实机报「左右无法移动指针」）——
            # PreviewPanel 窗级 ←/→ = 上/下一个素材，守卫原为 tagName INPUT/TEXTAREA 漏 contenteditable——
            # 断言：cm-content 上 ArrowLeft/ArrowRight 后光标移动且编辑器/素材不切换（.cm-editor 存活）
            await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }))")
            await asyncio.sleep(0.3)
            q = await ev("""(() => {
              const host = document.querySelector('.text-preview .cm-host');
              if (!host || !host.__cmView) return { dead: true };
              return { head: host.__cmView.state.selection.main.head, alive: !!document.querySelector('.text-preview .cm-editor') };
            })()""")
            print("[Q 方向键归光标]", json.dumps(q, ensure_ascii=False))
            if not q or q.get("dead") or not q["alive"]:
                fails.append(f"[Q] ArrowLeft 后编辑器被切走（{q}，预览快捷键抢键未让位）")
            elif q["head"] != 1:
                fails.append(f"[Q] ArrowLeft 光标未左移（head={q['head']}，期望 1——P2 停在 2）")
            await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }))")
            await asyncio.sleep(0.3)
            q2 = await ev("document.querySelector('.text-preview .cm-host').__cmView.state.selection.main.head")
            if q2 != 2:
                fails.append(f"[Q] ArrowRight 光标未右移（head={q2}，期望 2）")
            await shot("md-live-preview")

            # —— 场景 R：表格 / 列表渲染（2026-10-10 二轮补——用户实机盘点未渲染项）——
            # 表格 = 整段 widget（点击回落源码）；无序标记 = •；任务行 `-` 整藏；嵌套行 padding-left 24/级
            r = await ev("""(() => {
              const host = document.querySelector('.text-preview .cm-host');
              const lines = [...host.querySelectorAll('.cm-line')];
              const nested = lines.find((l) => l.textContent.includes('嵌套一项'));
              const taskLine = lines.find((l) => l.textContent.includes('未完成任务'));
              return {
                table: !!host.querySelector('.cm-lp-table table'),
                th: host.querySelector('.cm-lp-table th')?.textContent ?? null,
                td: host.querySelector('.cm-lp-table td')?.textContent ?? null,
                bullets: host.querySelectorAll('.cm-lp-bullet').length,
                nestedPad: nested ? getComputedStyle(nested).paddingLeft : null,
                taskMarkGone: taskLine ? !taskLine.textContent.trim().startsWith('-') : null,
                taskBox: taskLine ? !!taskLine.querySelector('.cm-lp-task') : null,
              };
            })()""")
            print("[R1 表格与列表]", json.dumps(r, ensure_ascii=False))
            if not r or not r["table"] or r["th"] != "名称" or r["td"] != "正文":
                fails.append(f"[R] 表格 widget 不符（{r}，期望 表头 名称 / 首格 正文）")
            if r and r["bullets"] != 3:
                fails.append(f"[R] 无序圆点数量不符（{r['bullets']}，期望 3 = 无序一项 / 父级 / 嵌套）")
            if r and r["nestedPad"] != "24px":
                fails.append(f"[R] 嵌套缩进不符（{r and r['nestedPad']}，期望 24px）")
            if r and (r["taskMarkGone"] is not True or r["taskBox"] is not True):
                fails.append(f"[R] 任务行 `-` 未整藏或复选框缺失（{r}）")
            r_bullet_leak = await ev("""(() => {
              const host = document.querySelector('.text-preview .cm-host');
              const taskLine = [...host.querySelectorAll('.cm-line')].find((l) => l.textContent.includes('未完成任务'));
              return taskLine ? !!taskLine.querySelector('.cm-lp-bullet') : null;
            })()""")
            if r_bullet_leak is not False:
                fails.append(f"[R] 任务行漏出圆点（应整藏，{r_bullet_leak}）")
            # 点击表格 → 光标入表 → 回落源码；光标移出 → 回渲染
            await ev("document.querySelector('.text-preview .cm-lp-table').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.5)
            r2 = await ev("""(() => ({
              widgetGone: !document.querySelector('.text-preview .cm-lp-table'),
              src: [...document.querySelectorAll('.text-preview .cm-line')].some((l) => l.textContent.includes('| 名称 |')),
            }))()""")
            print("[R2 点击回落源码]", json.dumps(r2, ensure_ascii=False))
            if not r2 or not r2["widgetGone"] or not r2["src"]:
                fails.append(f"[R] 点击表格未回落源码（{r2}）")
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              v.dispatch({ selection: { anchor: 2 } });
            })()""")
            await asyncio.sleep(0.5)
            r3 = await ev("!!document.querySelector('.text-preview .cm-lp-table table')")
            if r3 is not True:
                fails.append(f"[R] 光标移出后表格未回渲染（{r3}）")

            # —— 场景 L：工具栏悬停 tip（2026-09-25 走查补）——hover bold 钮出 v-tip「加粗」
            # （400ms 延迟 + headless 定时器节流，轮询至 1.6s 收敛）
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].dispatchEvent(new PointerEvent('pointerenter'))")
            l = None
            for _ in range(16):
                await asyncio.sleep(0.1)
                l = await ev("document.querySelector('.app-tip')?.textContent?.trim() ?? null")
                if l:
                    break
            print("[L tip]", l)
            if l != "加粗":
                fails.append(f"[L] 悬停 tip 不符（{l}，期望 加粗）")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].dispatchEvent(new PointerEvent('pointerleave'))")
            await asyncio.sleep(0.3)

            # —— 场景 F：Esc 退出编辑态 → 渲染态恢复 + 悬浮栏退场（keydown 派发 .cm-content 走 CM6 键图） ——
            await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(1.0)
            f = await ev("""(() => ({
              cm: !!document.querySelector('.text-preview .cm-editor'),
              mdBodyVisible: !!document.querySelector('.text-preview .md-body')?.offsetParent,
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              groups: document.querySelectorAll('.fbm-root > div').length,
              boldRendered: !!document.querySelector('.text-preview .md-body strong'),
              h1Count: document.querySelectorAll('.text-preview .md-body h1').length,
            }))()""")
            print("[F 退出]", json.dumps(f, ensure_ascii=False))
            if not f or f["cm"]:
                fails.append("[F] Esc 后编辑态 CM6 仍在")
            if f and f["mdBar"] is not True:
                fails.append("[F] 退出编辑态后预览工具栏未保持（md 语境应恒显视图组）")
            if f and f["groups"] != 1:
                fails.append(f"[F] 预览态工具栏应为仅视图组（groups={f['groups']}）")
            if f and f["boldRendered"] is not True:
                fails.append("[F] bold 未渲染回预览态（strong 缺失）")
            if f and f["h1Count"] != 2:
                fails.append(f"[F] h1 计数不符（{f['h1Count']}，期望 2 = 原标题 + 工具栏新设）")

            # —— 场景 I：编辑撤销 = CM6 原生事务栈（2026-10-10 换代；自实现 mdHistory 快照栈退役）——
            # 键入 = userEvent input.type（500ms 新组窗合并）/ 工具栏 = md.toolbar.N 恒独立步 / Ctrl+Z / Ctrl+Shift+Z ——
            await ev("document.querySelectorAll('.fbm-root > div')[0].querySelectorAll('button')[0].click()")  # 铅笔进编辑
            await asyncio.sleep(0.8)
            async def type_text(t):  # CM6 语境键入 = dispatch（updateListener 自动同步 draft → 防抖保存）
                await ev(f"""(() => {{
                  const v = document.querySelector('.text-preview .cm-host').__cmView;
                  const tt = {json.dumps(t)};
                  const len = v.state.doc.length;
                  v.dispatch({{ changes: {{ from: len, insert: tt }}, selection: {{ anchor: len + tt.length }}, userEvent: 'input.type' }});
                }})()""")
            await type_text("AAA")
            await asyncio.sleep(1.0)  # 超 500ms 新组窗 → 独立第二步
            await type_text("BBB")
            await asyncio.sleep(0.3)
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              v.dispatch({ selection: { anchor: 0, head: 2 } });
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].click()")  # bold 独立步
            await asyncio.sleep(0.3)
            async def ctrl_z(shift=False):
                # 重做走 Ctrl+Y——合成事件 Shift 持有时真实浏览器 event.key='Z'，CM6 keyName 归一化对不上
                # Mod-Shift-z（真机无此问题，探针语境绕行）；撤销仍 Ctrl+Z
                if shift:
                    await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'y', ctrlKey: true, bubbles: true }))")
                else:
                    await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true, bubbles: true }))")
                await asyncio.sleep(0.25)
            await ctrl_z()  # 撤 bold
            i1 = await ev("""(() => { const v = document.querySelector('.text-preview .cm-host').__cmView.state.doc.toString(); return { bold: v.startsWith('**'), bbb: v.endsWith('BBB') }; })()""")
            await ctrl_z()  # 撤 BBB（独立步）
            await ctrl_z()  # 撤 AAA（独立步）
            i2 = await ev("""(() => { const v = document.querySelector('.text-preview .cm-host').__cmView.state.doc.toString(); return { aaa: v.includes('AAA'), tail: v.slice(-20) }; })()""")
            await ctrl_z(True)
            await ctrl_z(True)
            await ctrl_z(True)  # 重做三步回 bold 后
            i3 = await ev("""(() => { const v = document.querySelector('.text-preview .cm-host').__cmView.state.doc.toString(); return { bold: v.startsWith('**'), bbb: v.endsWith('BBB') }; })()""")
            print("[I 撤销]", json.dumps({"撤bold": i1, "撤到初态": i2, "重做满": i3}, ensure_ascii=False))
            if not i1 or i1["bold"] is not False or i1["bbb"] is not True:
                fails.append(f"[I] 撤销工具栏步失败（{i1}，期望 无 bold 留 BBB）")
            if not i2 or i2["aaa"] is not False:
                fails.append(f"[I] 撤销两步键入失败（{i2}，期望 无 AAA/BBB 回初态）")
            if not i3 or i3["bold"] is not True or i3["bbb"] is not True:
                fails.append(f"[I] 重做失败（{i3}，期望 bold + BBB 全回）")

            # —— 场景 N：Ctrl+E 编辑↔预览互切（编辑态 CM6 键图自拦退出；预览态全局追加进编辑） ——
            await ev("document.querySelector('.text-preview .cm-content').dispatchEvent(new KeyboardEvent('keydown', { key: 'e', ctrlKey: true, bubbles: true }))")
            await asyncio.sleep(0.6)
            n1 = await ev("!!document.querySelector('.text-preview .cm-editor')")
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'e', ctrlKey: true, bubbles: true }))")
            await asyncio.sleep(0.8)
            n2 = await ev("!!document.querySelector('.text-preview .cm-editor')")
            print("[N Ctrl+E]", json.dumps({"退出": n1, "进入": n2}, ensure_ascii=False))
            if n1 is not False:
                fails.append(f"[N] 编辑态 Ctrl+E 未退出（{n1}）")
            if n2 is not True:
                fails.append(f"[N] 预览态 Ctrl+E 未进编辑（{n2}）")

            # —— 场景 O：分屏实时预览（双列 + 分割条拖动 + 单滚轮联动 + typewriter 跟滚）+ 眼睛钮回预览 ——
            await ev("document.querySelectorAll('.fbm-root > div')[4].querySelectorAll('button')[1].click()")  # 视图组·分屏
            await asyncio.sleep(0.6)
            o1 = await ev("""(() => ({
              bar: !!document.querySelector('.md-split-bar'),
              side: !!document.querySelector('.md-body.split-side'),
              taGrow: document.querySelector('.md-edit-wrap .pane')?.style.flexGrow,
              viewBtns: document.querySelectorAll('.fbm-root > div')[4]?.querySelectorAll('button').length ?? -1,
              thumbs: document.querySelectorAll('.md-edit-wrap .scroll-thumb').length,
              sidePadB: getComputedStyle(document.querySelector('.md-body.split-side')).paddingBottom,
              lineMarks: document.querySelectorAll('.md-body.split-side [data-line-start]').length,
            }))()""")
            print("[O1 分屏开]", json.dumps(o1, ensure_ascii=False))
            if not o1 or not o1["bar"] or not o1["side"]:
                fails.append(f"[O] 分屏结构缺失（{o1}）")
            if o1 and o1["viewBtns"] != 2:
                fails.append(f"[O] 视图组应为 2 钮（眼睛+分屏）（{o1['viewBtns']}）")
            if o1 and o1["thumbs"] != 1:
                fails.append(f"[O] 分屏应单滚轮（合并联动），实际 {o1['thumbs']} 个 ScrollThumb")
            if o1 and o1["sidePadB"] != "66px":
                fails.append(f"[O] 右栏底部遮挡补偿应为 66（{o1['sidePadB']}）")
            if o1 and o1["lineMarks"] < 5:
                fails.append(f"[O] 渲染块 data-line-start 注入不足（{o1['lineMarks']}）")
            # 分割条拖动：pointerdown → window move +200px → up；flexGrow 随 ratio 变
            await ev("""(() => {
              const bar = document.querySelector('.md-split-bar');
              const r = bar.getBoundingClientRect();
              bar.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, clientX: r.left + 2 }));
              window.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: r.left + 202 }));
              window.dispatchEvent(new PointerEvent('pointerup', { bubbles: true }));
            })()""")
            await asyncio.sleep(0.3)
            o2 = await ev("document.querySelector('.md-edit-wrap .pane')?.style.flexGrow")
            print("[O2 拖动]", o2)
            o2n = float(o2) if o2 else 0
            if not (0.25 <= o2n <= 0.75) or abs(o2n - 0.5) < 0.01:
                fails.append(f"[O] 分割条拖动未改占比（{o2}）")
            # 滚动联动（单滚轮语义）：先填充 30 个独立段落撑出两侧滚动空间（桩文档 19 行不足以滚；
            # 单换行会被 md 折叠成一段）——填充后**轮询右栏滚到底确认跟滚定时器已跑完**
            #（光标恒在文末 = 跟滚目标即底部，到底只有定时器能写、观测无歧义；归 0 轮询撞「本来就 0」、
            # 稳定窗轮询撞「节流冻结超窗」两版假收敛均已实锤，2026-10-10 三连），再左栏滚到 1/2 → 右栏按比例跟；右栏回 0 → 左栏跟回
            await type_text("\n\n" + "填充滚动行\n\n" * 30)
            for _ in range(40):  # 最长 10s（headless 定时器节流 ~1s 粒度）
                await asyncio.sleep(0.25)
                settled = await ev("""(() => {
                  const body = document.querySelector('.md-body.split-side');
                  const max = body.scrollHeight - body.clientHeight;
                  return max > 0 && body.scrollTop >= max - 2;
                })()""")
                if settled:
                    break
            await asyncio.sleep(0.3)  # 余量：linkLock 60ms 解锁落定
            await ev("""(() => {
              const sc = document.querySelector('.md-edit-wrap .cm-scroller');
              sc.scrollTop = (sc.scrollHeight - sc.clientHeight) / 2;
              sc.dispatchEvent(new Event('scroll'));
            })()""")
            await asyncio.sleep(0.3)
            o2b = await ev("""(() => {
              const sc = document.querySelector('.md-edit-wrap .cm-scroller');
              const body = document.querySelector('.md-body.split-side');
              const sMax = sc.scrollHeight - sc.clientHeight, bMax = body.scrollHeight - body.clientHeight;
              return { taRatio: sMax > 0 ? sc.scrollTop / sMax : 0, bRatio: bMax > 0 ? body.scrollTop / bMax : 0 };
            })()""")
            await ev("""(() => {
              const body = document.querySelector('.md-body.split-side');
              body.scrollTop = 0;
              body.dispatchEvent(new Event('scroll'));
            })()""")
            await asyncio.sleep(0.3)
            o2c = await ev("document.querySelector('.md-edit-wrap .cm-scroller').scrollTop")
            print("[O2b 联动]", json.dumps({"正向": o2b, "反向回滚": o2c}, ensure_ascii=False))
            if not o2b or abs(o2b["taRatio"] - o2b["bRatio"]) > 0.06:
                fails.append(f"[O] 正向联动不符（左 {o2b and o2b['taRatio']} vs 右 {o2b and o2b['bRatio']}）")
            if o2c is None or abs(o2c) > 2:
                fails.append(f"[O] 反向联动不符（右栏回 0 后左栏 {o2c}）")
            # 实时渲染：键入 **RRR** → 右栏 strong 即时出现
            await type_text("**RRR**")
            await asyncio.sleep(0.6)
            o3 = await ev("[...document.querySelectorAll('.md-body.split-side strong')].some((s) => s.textContent === 'RRR')")
            print("[O3 实时渲染]", o3)
            if o3 is not True:
                fails.append(f"[O] 分屏右栏未实时渲染（{o3}，期望存在 RRR strong）")
            # typewriter 跟滚：光标移到第 49 行（填充块群中部，居中可达——文档头尾块受滚动钳制
            # 物理上居不了中）→ 右栏滚到对应块并固定视口中间（CM6 选区 dispatch → updateListener 自动触发跟滚；
            # 16ms 定时器 + headless 后台定时器节流窗口不定——轮询 centerOff 收敛代替固定 sleep）
            o3b0 = await ev("document.querySelector('.md-body.split-side').scrollTop")
            await ev("""(() => {
              const v = document.querySelector('.text-preview .cm-host').__cmView;
              const pos = v.state.doc.line(50).from + 1;
              v.dispatch({ selection: { anchor: pos } });
            })()""")
            o3b = None
            for _ in range(20):
                await asyncio.sleep(0.1)
                o3b = await ev("""(() => {
                  const body = document.querySelector('.md-body.split-side');
                  const blocks = [...body.querySelectorAll('[data-line-start]')];
                  const v = document.querySelector('.text-preview .cm-host').__cmView;
                  const line = v.state.doc.lineAt(v.state.selection.main.head).number - 1;
                  let t = null;
                  for (const el of blocks) { if (Number(el.dataset.lineStart) <= line) t = el; else break; }
                  if (!t) return { scroll: body.scrollTop, hit: false };
                  const bR = body.getBoundingClientRect(), tR = t.getBoundingClientRect();
                  return { scroll: body.scrollTop, hit: true,
                           centerOff: Math.abs(tR.top + tR.height / 2 - (bR.top + bR.height / 2)) };
                })()""")
                if o3b and o3b.get("hit") and o3b["centerOff"] <= 40:
                    break
            print("[O3b typewriter]", json.dumps({"滚前": o3b0, **(o3b or {})}, ensure_ascii=False))
            if not o3b or not o3b["hit"]:
                fails.append(f"[O] typewriter 未命中光标行块（{o3b}）")
            elif o3b["centerOff"] > 40:
                fails.append(f"[O] 光标块未固定视口中间（偏移 {o3b['centerOff']}px，阈 40）")
            await shot("md-split")
            # 眼睛钮（视图组首钮）回预览
            await ev("document.querySelectorAll('.fbm-root > div')[4].querySelectorAll('button')[0].click()")
            await asyncio.sleep(0.8)
            k1 = await ev("""(() => ({
              cm: !!document.querySelector('.text-preview .cm-editor'),
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              rrr: !!document.querySelector('.text-preview .md-body strong'),
            }))()""")
            print("[O4 眼睛回预览]", json.dumps(k1, ensure_ascii=False))
            if not k1 or k1["cm"] or k1["mdBar"] is not True or k1["rrr"] is not True:
                fails.append(f"[O] 眼睛钮未回预览态（{k1}）")

            # —— 场景 G：回归——png 卡片预览仍走 img ——
            await ev("document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(0.8)
            await ev("document.querySelectorAll('.asset-card')[1].dispatchEvent(new MouseEvent('dblclick', { bubbles: true }))")
            await asyncio.sleep(1.5)
            g = await ev("""(() => ({
              imgInStage: document.querySelectorAll('.preview-stage .img-wrap img').length,
              textPane: !!document.querySelector('.text-preview'),
            }))()""")
            print("[G 回归-png]", json.dumps(g, ensure_ascii=False))
            if not g or g["imgInStage"] != 1:
                fails.append(f"[G] png 预览 img 缺失（{g}）")
            if g and g["textPane"]:
                fails.append("[G] png 预览挂出文本组件（spec 查表串线）")

            # —— 场景 H：Checkbox 组件样式抽象回归（预览页展区；scoped → 全局类 .kit-check-box 等价搬家） ——
            # 同 URL 换 hash 不触发重载（main.ts bootstrap 仅启动时判一次）——先走 about:blank 再进预览页
            await send(ws, "Page.navigate", {"url": "about:blank"}, session=sess)
            await asyncio.sleep(0.5)
            await send(ws, "Page.navigate", {"url": URL + "/#design-system"}, session=sess)
            await asyncio.sleep(3.0)
            h0 = await ev("""(() => {
              const cb = document.querySelector('.kit-checkbox');
              if (!cb) return null;
              const box = cb.querySelector('.kit-check-box');
              const mark = cb.querySelector('.kit-check-mark');
              if (!box || !mark) return { missing: true };
              const cs = getComputedStyle(box);
              cb.click();
              return { w: cs.width, h: cs.height, bg: cs.backgroundColor, radius: cs.borderTopLeftRadius,
                       markTransform: getComputedStyle(mark).transform };
            })()""")
            await asyncio.sleep(0.5)  # 底/描边 150ms 过渡 + 余量
            # headless 后台定时器 / 渲染节流下 Vue 更新与过渡收敛可能迟到——轮询至类翻转后再读终态（2s 窗）
            h1 = None
            for _ in range(20):
                h1 = await ev("""(() => {
                  const box = document.querySelector('.kit-checkbox .kit-check-box');
                  const mark = document.querySelector('.kit-checkbox .kit-check-mark');
                  return box ? { onCls: box.classList.contains('on'), onBg: getComputedStyle(box).backgroundColor,
                                 onMark: getComputedStyle(mark).transform } : null;
                })()""")
                if h1 and h1["onCls"] is False:
                    break
                await asyncio.sleep(0.1)
            print("[H Checkbox]", json.dumps({"常态": h0, "选中": h1}, ensure_ascii=False))
            if not h0 or h0.get("missing"):
                fails.append(f"[H] Checkbox 展区结构缺失（{h0}）")
            else:
                if h0["w"] != "14px" or h0["h"] != "14px":
                    fails.append(f"[H] 框尺寸不符（{h0['w']}×{h0['h']}，期望 14px）")
                if h0["radius"] != "4px":
                    fails.append(f"[H] 框圆角不符（{h0['radius']}）")
                # 展区首枚 = 选中态（cbOn 初始 true）：初态 brand 底 + 对勾 scale(1)
                if h0["bg"] != "rgb(15, 137, 236)":
                    fails.append(f"[H] 选中底色不符（{h0['bg']}，期望品牌 rgb(15,137,236)）")
                if h0["markTransform"] != "matrix(1, 0, 0, 1, 0, 0)":
                    fails.append(f"[H] 选中对勾未弹出（{h0['markTransform']}）")
            # 点击后翻转 = 未选中：on 类卸（样式断言只看类——headless 按需出帧模式过渡帧冻结，
            # computed 背景/对勾停在过渡起点属环境病，2026-10-10 同 rAF 停摆族；真机过渡有帧源无碍）
            if not h1 or h1["onCls"] is not False:
                fails.append(f"[H] 点击翻转后未选中态不符（{h1}，期望 无 on 类）")
            print("[H 翻转终态样式（环境参考）]", json.dumps(h1, ensure_ascii=False))

            if fails:
                print("FAIL:", "; ".join(dict.fromkeys(fails)))
                return 3
            print("PASS: md 文本编辑器链路成立（贴稿排版 / 任务复选框 toggle 回写 / 代码块复制 / 编辑态切换 / 工具栏动作 / Esc 退出 / png 回归）")
            return 0
    finally:
        proc.terminate()
        try:
            proc.wait(timeout=8)
        except subprocess.TimeoutExpired:
            proc.kill()


if __name__ == "__main__":
    sys.exit(asyncio.run(main()))
