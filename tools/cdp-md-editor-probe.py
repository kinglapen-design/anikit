# CDP 实测：MarkDown 文本编辑器（文本编辑稿轮，2026-09 排版 + 编辑器）。
# 场景：A 预览态排版结构 + 贴稿 computed 数值（H1 30/40 + 2px g3 分割线 / H2 26/36 / 正文 15/25 /
# 引用 2px g3 竖线 / 代码块 g2 底 + 复制钮）；B 任务复选框点击 toggle → 防抖回写 write_text_asset；
# C 代码块复制钮 → 剪贴板（桩 stub）；D 点击正文进编辑态（textarea + 悬浮栏换 MarkDown 工具栏）；
# E 工具栏 bold / h1 动作作用源码选区；F Esc 退出回渲染态 + 悬浮栏退场；G png 预览回归。
# 桩模式不验真后端读写（cargo test 覆盖 text 命令回路）。
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
                prebg: precs?.backgroundColor, prebc: precs?.borderColor, prebr: precs?.borderRadius,
                codefs: cs('.md-code code', 'fontSize'), codelh: cs('.md-code code', 'lineHeight'), codec: cs('.md-code code', 'color'),
                copyBtn: !!body.querySelector('.md-code .md-copy'),
                tasks: body.querySelectorAll('li.md-task').length,
                taskBoxes: body.querySelectorAll('li.md-task > .kit-check-box').length,
                taskOn: body.querySelectorAll('li.md-task > .kit-check-box.on').length,
                taskLines: [...body.querySelectorAll('.kit-check-box')].map((b) => b.dataset.taskLine),
                taskText0: body.querySelectorAll('li.md-task')[0]?.textContent.trim() ?? null,
                gap: getComputedStyle(body).rowGap,
                padL: getComputedStyle(body).paddingLeft,
              };
            })()""")
            print("[A 预览态]", json.dumps(a, ensure_ascii=False))
            if not a:
                fails.append("[A] md 渲染体未渲染")
            else:
                exp = {
                    "h1fs": "30px", "h1lh": "40px", "h1bb": "2px",
                    "h1bbc": "rgb(36, 36, 36)", "h1c": "rgb(191, 191, 191)",
                    "h2fs": "26px", "h2lh": "36px", "h3fs": "22px", "h3lh": "32px",
                    "pfs": "15px", "plh": "25px", "pc": "rgb(153, 153, 153)",
                    "bqbl": "2px", "bqblc": "rgb(36, 36, 36)", "bqc": "rgb(102, 102, 102)",
                    "prebg": "rgb(28, 28, 28)", "prebr": "8px",
                    "codefs": "14px", "codelh": "24px", "codec": "rgb(102, 102, 102)",
                    "gap": "12px", "padL": "84px",
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

            # —— 场景 D：点击正文进编辑态（textarea + 悬浮栏 MarkDown 工具栏） ——
            await ev("document.querySelector('.text-preview .md-body p').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(1.0)
            d = await ev("""(() => ({
              ta: !!document.querySelector('.text-preview textarea.md-editor'),
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              shellGone: document.querySelector('.fb-shell')?.classList.contains('gone') ?? null,
              mdBodyVisible: !!document.querySelector('.text-preview .md-body')?.offsetParent,
            }))()""")
            print("[D 编辑态]", json.dumps(d, ensure_ascii=False))
            if not d or not d["ta"]:
                fails.append("[D] 编辑态 textarea 未出现")
            if not d or not d["mdBar"]:
                fails.append("[D] 悬浮栏未切 MarkDown 工具栏")
            if d and d["shellGone"] is not False:
                fails.append(f"[D] 编辑态悬浮栏仍退场（gone={d and d['shellGone']}）")

            # —— 场景 E：工具栏动作（bold 包裹选区 / h1 行首） ——
            await ev("""(() => {
              const ta = document.querySelector('.text-preview textarea.md-editor');
              ta.focus();
              const i = ta.value.indexOf('方舟前史');
              ta.setSelectionRange(i, i + 4);
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].click()")  # 格式组·bold
            await asyncio.sleep(0.3)
            e1 = await ev("document.querySelector('.text-preview textarea.md-editor').value.includes('**方舟前史**')")
            print("[E1 bold]", e1)
            if e1 is not True:
                fails.append(f"[E1] bold 包裹失败（{e1}）")
            await ev("""(() => {
              const ta = document.querySelector('.text-preview textarea.md-editor');
              const i = ta.value.indexOf('夏娃摘下一颗苹果');
              ta.setSelectionRange(i, i + 2);
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[0].querySelectorAll('button')[0].click()")  # 段落组·h1
            await asyncio.sleep(0.3)
            e2 = await ev("document.querySelector('.text-preview textarea.md-editor').value.includes('# 夏娃摘下一颗苹果')")
            print("[E2 h1]", e2)
            if e2 is not True:
                fails.append(f"[E2] h1 行首失败（{e2}）")
            await shot("md-editing")

            # —— 场景 L：工具栏悬停 tip（2026-09-25 走查补）——hover bold 钮出 v-tip「加粗」 ——
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].dispatchEvent(new PointerEvent('pointerenter'))")
            await asyncio.sleep(0.7)  # v-tip 400ms 延迟 + 余量
            l = await ev("document.querySelector('.app-tip')?.textContent?.trim() ?? null")
            print("[L tip]", l)
            if l != "加粗":
                fails.append(f"[L] 悬停 tip 不符（{l}，期望 加粗）")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].dispatchEvent(new PointerEvent('pointerleave'))")
            await asyncio.sleep(0.3)

            # —— 场景 F：Esc 退出编辑态 → 渲染态恢复 + 悬浮栏退场 ——
            await ev("document.querySelector('.text-preview textarea.md-editor').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }))")
            await asyncio.sleep(1.0)
            f = await ev("""(() => ({
              ta: !!document.querySelector('.text-preview textarea.md-editor'),
              mdBodyVisible: !!document.querySelector('.text-preview .md-body')?.offsetParent,
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              shellGone: document.querySelector('.fb-shell')?.classList.contains('gone') ?? null,
              boldRendered: !!document.querySelector('.text-preview .md-body strong'),
              h1Count: document.querySelectorAll('.text-preview .md-body h1').length,
            }))()""")
            print("[F 退出]", json.dumps(f, ensure_ascii=False))
            if not f or f["ta"]:
                fails.append("[F] Esc 后编辑态 textarea 仍在")
            if f and f["mdBar"]:
                fails.append("[F] 退出编辑态后 MarkDown 工具栏未收")
            if f and f["boldRendered"] is not True:
                fails.append("[F] bold 未渲染回预览态（strong 缺失）")
            if f and f["h1Count"] != 2:
                fails.append(f"[F] h1 计数不符（{f['h1Count']}，期望 2 = 原标题 + 工具栏新设）")

            # —— 场景 I：编辑撤销栈（2026-09-25 走查「没有接入撤销」）——键入 800ms 合并 / 工具栏独立步 / Ctrl+Z / Ctrl+Shift+Z ——
            await ev("document.querySelector('.text-preview .md-body p').dispatchEvent(new MouseEvent('click', { bubbles: true }))")
            await asyncio.sleep(0.8)
            async def type_text(t):  # 编程改值不触发原生 input——手动 dispatch 驱动 v-model 与记录点
                await ev(f"""(() => {{
                  const ta = document.querySelector('.text-preview textarea.md-editor');
                  ta.focus();
                  ta.setSelectionRange(ta.value.length, ta.value.length);
                  ta.setRangeText({json.dumps(t)}, ta.value.length, ta.value.length, 'end');
                  ta.dispatchEvent(new Event('input', {{ bubbles: true }}));
                }})()""")
            await type_text("AAA")
            await asyncio.sleep(1.0)  # 超 800ms 合并窗 → 独立第二步
            await type_text("BBB")
            await asyncio.sleep(0.3)
            await ev("""(() => {
              const ta = document.querySelector('.text-preview textarea.md-editor');
              ta.setSelectionRange(0, 2);
            })()""")
            await ev("document.querySelectorAll('.fbm-root > div')[3].querySelectorAll('button')[0].click()")  # bold 独立步
            await asyncio.sleep(0.3)
            async def ctrl_z(shift=False):
                await ev(f"document.querySelector('.text-preview textarea.md-editor').dispatchEvent(new KeyboardEvent('keydown', {{ key: 'z', ctrlKey: true, shiftKey: {'true' if shift else 'false'}, bubbles: true }}))")
                await asyncio.sleep(0.25)
            await ctrl_z()  # 撤 bold
            i1 = await ev("""(() => { const v = document.querySelector('.text-preview textarea.md-editor').value; return { bold: v.startsWith('**'), bbb: v.endsWith('BBB') }; })()""")
            await ctrl_z()  # 撤 BBB（独立步）
            await ctrl_z()  # 撤 AAA（独立步）
            i2 = await ev("""(() => { const v = document.querySelector('.text-preview textarea.md-editor').value; return { aaa: v.includes('AAA'), tail: v.slice(-20) }; })()""")
            await ctrl_z(True)
            await ctrl_z(True)
            await ctrl_z(True)  # 重做三步回 bold 后
            i3 = await ev("""(() => { const v = document.querySelector('.text-preview textarea.md-editor').value; return { bold: v.startsWith('**'), bbb: v.endsWith('BBB') }; })()""")
            print("[I 撤销]", json.dumps({"撤bold": i1, "撤到初态": i2, "重做满": i3}, ensure_ascii=False))
            if not i1 or i1["bold"] is not False or i1["bbb"] is not True:
                fails.append(f"[I] 撤销工具栏步失败（{i1}，期望 无 bold 留 BBB）")
            if not i2 or i2["aaa"] is not False:
                fails.append(f"[I] 撤销两步键入失败（{i2}，期望 无 AAA/BBB 回初态）")
            if not i3 or i3["bold"] is not True or i3["bbb"] is not True:
                fails.append(f"[I] 重做失败（{i3}，期望 bold + BBB 全回）")

            # —— 场景 K：工具栏「返回预览」钮（走查「怎么切换到预览」可见出口） ——
            k0 = await ev("(() => { const g = document.querySelectorAll('.fbm-root > div')[4]; return g ? g.querySelectorAll('button').length : -1; })()")
            await ev("document.querySelectorAll('.fbm-root > div')[4].querySelector('button').click()")
            await asyncio.sleep(0.8)
            k1 = await ev("""(() => ({
              ta: !!document.querySelector('.text-preview textarea.md-editor'),
              mdBar: !!document.querySelector('.fb-shell .fbm-root'),
              mdBodyVisible: !!document.querySelector('.text-preview .md-body')?.offsetParent,
            }))()""")
            print("[K 返回预览钮]", json.dumps({"钮数": k0, **(k1 or {})}, ensure_ascii=False))
            if k0 != 1:
                fails.append(f"[K] 第五组（返回预览）钮数不符（{k0}，期望 1）")
            if not k1 or k1["ta"] or k1["mdBar"] or k1["mdBodyVisible"] is not True:
                fails.append(f"[K] 点击后未回预览态（{k1}）")

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
            h1 = await ev("""(() => {
              const box = document.querySelector('.kit-checkbox .kit-check-box');
              const mark = document.querySelector('.kit-checkbox .kit-check-mark');
              return box ? { onCls: box.classList.contains('on'), onBg: getComputedStyle(box).backgroundColor,
                             onMark: getComputedStyle(mark).transform } : null;
            })()""")
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
            # 点击后翻转 = 未选中：on 类卸 + g3 底 + 对勾 scale(0)
            if not h1 or h1["onCls"] is not False or h1["onBg"] != "rgb(36, 36, 36)" or h1["onMark"] != "matrix(0, 0, 0, 0, 0, 0)":
                fails.append(f"[H] 点击翻转后未选中态不符（{h1}，期望 无 on + g3 底 + 对勾收）")

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
