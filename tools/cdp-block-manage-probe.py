# CDP 实测：区块子系统（2026-08-14，3.1 一轮）
# ① 侧栏多区块：桩 list_folder_tree 返回两区块 + 分属区块的顶级文件夹 → LibraryView 两个
#    SectionHeader（标题/计数）+ 各自树；FolderNode.icon 有值时树 item 渲染自定义图标
#    （assets/icons/05_custom-icon）；区块标题加号 → create_folder 带 blockId。
# ② 区块管理弹窗（30 稿）：侧栏 section-manage 点亮 → AppModal standard 559 宽 / 标题常态色 /
#    行 = 排序箭头（箭头-小 chevron-small：下移在前、上移旋转 180；可用高亮档 / 禁用 --line-active）
#    + 名称输入 28 + 图标下拉 58×28 + 红删除；「目录」行名称/图标锁死（禁用去框）、删除 opacity 0；
#    底部 确认 primary/取消 panel 88×38 主左次右；添加区块按钮 104×36。
# ③ 弹窗行为：添加区块加行 → 改名 → 箭头上移 → 删除「场景库」→ 确认 → apply_block_changes
#    （断言 blocks 数组序 / deleted / confirmed=false）→ 桩回 needConfirm → 26 稿 alert warning
#    → 点"删除"→ 桩收到 confirmed=true 重提；重开点取消 = 零 invoke。
# 截图留档 tools/style-preview/block-manage.png / sidebar-blocks.png / block-delete-warning.png。
import asyncio
import base64
import json
import subprocess
import sys
import time
import urllib.request

import websockets

EDGE = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
PORT = 9352
URL = "http://localhost:5373"

# 桩：两区块（dir 目录 + bk1 场景库）三顶级文件夹（人物属 dir 带 character 图标；场景属 bk1 带
# scene 图标；镜头属 bk1 无图标=默认文件夹图标）。apply_block_changes：deleted 非空且未 confirmed
# → needConfirm（场景库 2 文件夹）；confirmed=true → 成功。全部写操作记录参数供断言。
STUB = r"""
window.__calls = { apply_block_changes: [], reorder_blocks: [], create_folder: [] };
window.__cbs = {};
window.__cbId = 0;
window.__TAURI_INTERNALS__ = {
  metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
  convertFileSrc: (p) => "asset://localhost/" + encodeURIComponent(p),
  transformCallback: (cb) => { const id = ++window.__cbId; window.__cbs[id] = cb; return id; },
  invoke: (cmd, args) => {
    const LIB = { name: "动画资产", path: "E:/T/S/动画资产", assetCount: 0, folderCount: 3 };
    const TREE = {
      blocks: [
        { id: "dir", name: "目录", icon: null },
        { id: "bk1", name: "场景库", icon: "scene" },
      ],
      nodes: [
        { id: "F01", name: "人物", count: 3, block: "dir", icon: "character", children: [] },
        { id: "F02", name: "场景", count: 5, block: "bk1", icon: "scene", children: [] },
        { id: "F03", name: "镜头", count: 0, block: "bk1", icon: null, children: [] },
      ],
    };
    const R = {
      get_current_library: LIB,
      get_current_space: { name: "S", path: "E:/T/S", autoImport: true },
      get_ui_settings: {}, detect_doc_apps: [],
      list_placements: [], list_recycle: [], list_recent_assets: [], list_inbox_snapshot: [],
      get_undo_state: { canUndo: false, canRedo: false, undoLabel: "", redoLabel: "" },
      list_folder_tree: TREE,
      list_space_libraries: ["动画资产"],
      list_recent_libraries: [],
    };
    if (cmd === "create_folder") {
      window.__calls.create_folder.push(args);
      return Promise.resolve({ id: "F99" });
    }
    if (cmd === "apply_block_changes") {
      window.__calls.apply_block_changes.push(args);
      if (args.deleted.length > 0 && !args.confirmed) {
        return Promise.resolve({ needConfirm: true, blocks: [{ name: "场景库", folderCount: 2 }] });
      }
      return Promise.resolve({ needConfirm: false, blocks: [] });
    }
    if (cmd === "reorder_blocks") {
      window.__calls.reorder_blocks.push(args.ids);
      return Promise.resolve(null);
    }
    return Promise.resolve(Object.prototype.hasOwnProperty.call(R, cmd) ? R[cmd] : null);
  },
};
"""

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
        if raw.get("method") == "Runtime.consoleAPICalled" and raw["params"]["type"] in ("error", "warning"):
            args = raw["params"]["args"]
            print("[CONSOLE]", raw["params"]["type"], " ".join(str(a.get("value", a.get("description", "")))[:300] for a in args))


async def main():
    proc = subprocess.Popen(
        [EDGE, f"--remote-debugging-port={PORT}", "--headless=new", "--disable-gpu", "--window-size=1400,900", "about:blank"],
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
            await asyncio.sleep(6)

            async def ev(expr):
                r = await send(ws, "Runtime.evaluate", {"expression": expr, "returnByValue": True, "awaitPromise": True}, session=sess)
                return r.get("result", {}).get("result", {}).get("value")

            async def click(expr):
                rect = await ev(f"(() => {{ const el = {expr}; if (!el) return null; const b = el.getBoundingClientRect(); return {{ x: b.left + b.width / 2, y: b.top + b.height / 2 }}; }})()")
                if not rect:
                    print("!! 元素不存在:", expr[:80])
                    return False
                for t in ("mousePressed", "mouseReleased"):
                    await send(ws, "Input.dispatchMouseEvent", {"type": t, "button": "left", "buttons": 1, "x": rect["x"], "y": rect["y"], "clickCount": 1}, session=sess)
                await asyncio.sleep(0.5)
                return True

            async def shot(name):
                r = await send(ws, "Page.captureScreenshot", {"format": "png"}, session=sess)
                with open(f"tools/style-preview/{name}.png", "wb") as f:
                    f.write(base64.b64decode(r["result"]["data"]))
                print("截图:", name)

            fails = []

            # —— ① 侧栏多区块 ——
            heads = await ev("[...document.querySelectorAll('.library-view .block.tree .block-head')].map((h) => ({ id: h.dataset.blockId, title: h.querySelector('.title')?.textContent ?? null, count: h.querySelector('.count')?.textContent?.trim() ?? null }))")
            print("区块头:", json.dumps(heads, ensure_ascii=False))
            if heads != [
                {"id": "dir", "title": "目录", "count": "- 1"},
                {"id": "bk1", "title": "场景库", "count": "- 2"},
            ]:
                fails.append(f"区块头 {json.dumps(heads, ensure_ascii=False)}")

            trees = await ev("""({
              dir: [...document.querySelectorAll('.block-tree[data-block-id="dir"] .tree-item .label')].map((l) => l.textContent),
              bk1: [...document.querySelectorAll('.block-tree[data-block-id="bk1"] .tree-item .label')].map((l) => l.textContent),
            })""")
            print("区块树:", json.dumps(trees, ensure_ascii=False))
            if trees != {"dir": ["人物"], "bk1": ["场景", "镜头"]}:
                fails.append(f"区块树 {json.dumps(trees, ensure_ascii=False)}")

            # TreeItem 自定义图标：人物=character / 场景=scene / 镜头=默认 folder-closed——三者 deco 两两不同
            icons = await ev("[...document.querySelectorAll('.block-tree .tree-item')].map((it) => ({ label: it.querySelector('.label')?.textContent, svg: it.querySelector('.body .deco')?.innerHTML?.length ?? 0, hash: [...(it.querySelector('.body .deco')?.innerHTML ?? '')].reduce((a, c) => (a * 31 + c.charCodeAt(0)) % 1000003, 7) }))")
            print("树图标:", json.dumps(icons, ensure_ascii=False))
            by_label = {i["label"]: i["hash"] for i in icons} if icons else {}
            if not (by_label.get("人物") and by_label.get("场景") and by_label.get("镜头")):
                fails.append("树 item 图标缺失")
            elif by_label["人物"] == by_label["镜头"] or by_label["场景"] == by_label["镜头"] or by_label["人物"] == by_label["场景"]:
                fails.append(f"自定义图标未生效 {json.dumps(by_label)}")

            await shot("sidebar-blocks")

            # 区块标题加号 → create_folder 带 blockId
            await click("document.querySelector('.block-head[data-block-id=\"bk1\"] .add-btn')")
            await asyncio.sleep(0.8)
            cf = await ev("window.__calls.create_folder")
            print("create_folder 调用:", json.dumps(cf, ensure_ascii=False))
            if len(cf or []) != 1 or cf[0].get("blockId") != "bk1" or cf[0].get("parentId") is not None:
                fails.append(f"create_folder 参数 {json.dumps(cf, ensure_ascii=False)}")

            # —— ② 区块管理弹窗（30 稿布局）——
            manage_btn = await ev("(() => { const b = document.querySelector('.btn-group > .icon-btn'); return { exists: !!b, disabled: b?.disabled ?? null }; })()")
            print("section-manage 按钮:", manage_btn)
            if not manage_btn.get("exists") or manage_btn.get("disabled"):
                fails.append(f"section-manage 未点亮 {manage_btn}")
            await click("document.querySelector('.btn-group > .icon-btn')")
            await asyncio.sleep(0.8)

            info = await ev("""(() => {
              const modal = document.querySelector('.block-manage .app-modal');
              if (!modal) return null;
              const probeColor = (v) => { const p = document.createElement('div'); p.style.color = v; document.body.appendChild(p); const c = getComputedStyle(p).color; p.remove(); return c; };
              const normalColor = probeColor('var(--text-normal)');
              const highlightColor = probeColor('var(--text-highlight)');
              const disabledColor = probeColor('var(--line-active)');
              const rows = [...document.querySelectorAll('.block-manage .bm-row')];
              const dirRow = rows[0];
              const libRow = rows[1];
              const del = dirRow?.querySelector('.icon-btn.delete-red');
              const arrows = libRow ? [...libRow.querySelectorAll('.sort-arrow')] : [];
              const dirInput = dirRow?.querySelector('input');
              const pick = dirRow?.querySelector('.icon-pick');
              return {
                title: modal.querySelector('.title')?.textContent ?? null,
                width: modal.getBoundingClientRect().width,
                titleColor: getComputedStyle(modal.querySelector('.title')).color,
                normalColor, highlightColor, disabledColor,
                hasCloseX: !!modal.querySelector('.titlebar .icon-btn'),
                rows: rows.length,
                rowHeight: dirRow?.getBoundingClientRect().height ?? null,
                rowNames: rows.map((r) => r.querySelector('input')?.value ?? null),
                inputHeight: dirInput?.getBoundingClientRect().height ?? null,
                pickW: pick?.getBoundingClientRect().width ?? null,
                pickH: pick?.getBoundingClientRect().height ?? null,
                dirDelOpacity: del ? getComputedStyle(del).opacity : null,
                dirDelPE: del ? getComputedStyle(del).pointerEvents : null,
                dirInputDisabled: dirInput?.disabled ?? null,
                dirInputBg: dirInput ? getComputedStyle(dirInput).backgroundColor : null,
                dirPickDisabled: pick?.disabled ?? null,
                arrowOrder: arrows.map((a) => (a.classList.contains('arrow-up') ? 'up' : 'down')),
                arrowEnabledColor: arrows[1] ? getComputedStyle(arrows[1]).color : null, // 场景库为末行：下移禁用、上移可用
                dirUpDisabled: (() => { const up = dirRow?.querySelectorAll('.sort-arrow')?.[1]; return up ? { disabled: up.disabled, color: getComputedStyle(up).color, opacity: getComputedStyle(up).opacity } : null; })(),
                addBtn: (() => { const b = document.querySelector('.block-manage .add-wrap .app-btn'); if (!b) return null; const r = b.getBoundingClientRect(); return { label: b.textContent.trim(), w: r.width, h: r.height }; })(),
                actions: [...modal.querySelectorAll('.actions .app-btn')].map((b) => ({ label: b.textContent.trim(), w: b.getBoundingClientRect().width, h: b.getBoundingClientRect().height, primary: b.classList.contains('primary'), panel: b.classList.contains('panel') })),
              };
            })()""")
            print("弹窗:", json.dumps(info, ensure_ascii=False))
            if info is None:
                fails.append("区块管理弹窗未出现")
                print("FAIL:", "; ".join(fails))
                return 3
            if info["title"] != "区块管理":
                fails.append(f"标题 {info['title']}")
            if info["width"] != 559:
                fails.append(f"宽度 {info['width']}（应 559）")
            if info["titleColor"] != info["normalColor"]:
                fails.append(f"标题色 {info['titleColor']} ≠ 常态 {info['normalColor']}")
            if not info["hasCloseX"]:
                fails.append("标题栏无关闭 X")
            if info["rows"] != 2 or info["rowNames"] != ["目录", "场景库"]:
                fails.append(f"初始行 {info['rowNames']}")
            if info["rowHeight"] != 52:
                fails.append(f"行高 {info['rowHeight']}（应 52）")
            if info["inputHeight"] != 28:
                fails.append(f"输入框高 {info['inputHeight']}（应 28）")
            if info["pickW"] != 58 or info["pickH"] != 28:
                fails.append(f"图标下拉 {info['pickW']}x{info['pickH']}（应 58x28）")
            if info["dirDelOpacity"] != "0" or info["dirDelPE"] != "none":
                fails.append(f"「目录」行删除未禁 {info['dirDelOpacity']}/{info['dirDelPE']}")
            # 「目录」行锁死（2026-08-14 用户定）：名称输入 / 图标下拉禁用、输入框去框透明
            if info["dirInputDisabled"] is not True or info["dirPickDisabled"] is not True:
                fails.append(f"「目录」行名称/图标未锁 {info['dirInputDisabled']}/{info['dirPickDisabled']}")
            if info["dirInputBg"] not in ("rgba(0, 0, 0, 0)", "transparent"):
                fails.append(f"「目录」行输入框未去框 {info['dirInputBg']}")
            # 排序箭头：下移在前、上移在后；可用态 = 高亮档
            if info["arrowOrder"] != ["down", "up"]:
                fails.append(f"箭头顺序 {info['arrowOrder']}（应 下移/上移）")
            if info["arrowEnabledColor"] != info["highlightColor"]:
                fails.append(f"箭头可用态色 {info['arrowEnabledColor']} ≠ 高亮 {info['highlightColor']}")
            # 目录行上移（isFirst）= 禁用态：disabled + 禁用色且不叠半透明
            up = info["dirUpDisabled"]
            if not up or up["disabled"] is not True or up["color"] != info["disabledColor"] or up["opacity"] != "1":
                fails.append(f"「目录」行上箭头禁用态 {json.dumps(up)}")
            if info["addBtn"] != {"label": "添加区块", "w": 104, "h": 36}:
                fails.append(f"添加区块按钮 {info['addBtn']}")
            acts = info["actions"]
            if [a["label"] for a in acts] != ["确认", "取消"]:
                fails.append(f"按钮序 {[a['label'] for a in acts]}")
            elif not (acts[0]["primary"] and acts[1]["panel"] and all(a["w"] == 88 and a["h"] == 38 for a in acts)):
                fails.append(f"按钮样式/尺寸 {json.dumps(acts)}")

            await shot("block-manage")

            # —— ③ 弹窗行为：添加 → 改名 → 上移 → 删除 → 确认（两段式）——
            await click("document.querySelector('.block-manage .add-wrap .app-btn')")
            rows3 = await ev("document.querySelectorAll('.block-manage .bm-row').length")
            print("添加区块后行数:", rows3)
            if rows3 != 3:
                fails.append(f"添加区块未加行（{rows3}）")
            # 新行改名「参考库」
            await ev("(() => { const inp = document.querySelectorAll('.block-manage .bm-row')[2].querySelector('input'); inp.value = '参考库'; inp.dispatchEvent(new Event('input', { bubbles: true })); return inp.value; })()")
            # 新行（index 2）上移一位 → [目录, 参考库, 场景库]
            await click("document.querySelectorAll('.block-manage .bm-row')[2].querySelector('.icon-btn.arrow-up')")
            order = await ev("[...document.querySelectorAll('.block-manage .bm-row input')].map((i) => i.value)")
            print("上移后草稿序:", order)
            if order != ["目录", "参考库", "场景库"]:
                fails.append(f"箭头排序 {order}")
            # 删除「场景库」（现在 index 2）
            await click("document.querySelectorAll('.block-manage .bm-row')[2].querySelector('.icon-btn.delete-red')")
            order2 = await ev("[...document.querySelectorAll('.block-manage .bm-row input')].map((i) => i.value)")
            print("删除后草稿序:", order2)
            if order2 != ["目录", "参考库"]:
                fails.append(f"删除标记 {order2}")
            # 确认 → apply_block_changes(confirmed=false) → 桩回 needConfirm → alert warning
            await click("[...document.querySelectorAll('.block-manage .actions .app-btn')].find((b) => b.textContent.includes('确认'))")
            await asyncio.sleep(0.8)
            alert = await ev("""(() => {
              const m = document.querySelector('.app-modal.alert');
              if (!m) return null;
              return {
                title: m.querySelector('.title')?.textContent ?? null,
                lines: [...m.querySelectorAll('.warn-line')].map((p) => p.textContent),
                buttons: [...m.querySelectorAll('.actions .app-btn')].map((b) => b.textContent.trim()),
              };
            })()""")
            print("删除警告:", json.dumps(alert, ensure_ascii=False))
            calls1 = await ev("window.__calls.apply_block_changes")
            print("apply 调用(1):", json.dumps(calls1, ensure_ascii=False))
            if alert is None or alert["title"] != "删除区块":
                fails.append(f"needConfirm 警告未弹 {json.dumps(alert, ensure_ascii=False)}")
            elif alert["buttons"] != ["删除", "取消"] or "区块「场景库」内还有 2 个文件夹" not in alert["lines"]:
                fails.append(f"警告内容 {json.dumps(alert, ensure_ascii=False)}")
            if len(calls1 or []) != 1:
                fails.append(f"apply 调用数 {len(calls1 or [])}")
            else:
                c = calls1[0]
                names = [(b.get("id"), b.get("name")) for b in c.get("blocks", [])]
                ok = (
                    names == [("dir", "目录"), (None, "参考库")]
                    and c.get("deleted") == ["bk1"]
                    and c.get("confirmed") is False
                )
                if not ok:
                    fails.append(f"apply 参数 {json.dumps(c, ensure_ascii=False)}")
            await shot("block-delete-warning")

            # 警告窗点"删除" → confirmed=true 重提 → 成功关窗
            await click("[...document.querySelectorAll('.app-modal.alert .actions .app-btn')].find((b) => b.textContent.includes('删除'))")
            await asyncio.sleep(1.0)
            calls2 = await ev("window.__calls.apply_block_changes")
            closed = await ev("!document.querySelector('.block-manage .app-modal') && !document.querySelector('.app-modal.alert')")
            toast = await ev("document.querySelector('.toast-stack')?.textContent?.trim() ?? null")
            print("apply 调用(2):", json.dumps(calls2, ensure_ascii=False), "关窗:", closed, "toast:", toast)
            if len(calls2 or []) != 2 or calls2[1].get("confirmed") is not True:
                fails.append(f"confirmed 重提 {json.dumps(calls2, ensure_ascii=False)}")
            if not closed:
                fails.append("确认后弹窗未关闭")
            if not toast or "已更新区块" not in toast:
                fails.append(f"成功 toast 缺失 {toast}")

            # —— 取消路径：重开 → 取消 = 零新增 invoke ——
            await click("document.querySelector('.btn-group > .icon-btn')")
            await asyncio.sleep(0.8)
            reopened = await ev("!!document.querySelector('.block-manage .app-modal')")
            before = await ev("window.__calls.apply_block_changes.length")
            await click("[...document.querySelectorAll('.block-manage .actions .app-btn')].find((b) => b.textContent.includes('取消'))")
            await asyncio.sleep(0.6)
            after = await ev("window.__calls.apply_block_changes.length")
            closed2 = await ev("!document.querySelector('.block-manage .app-modal')")
            print("取消路径: 重开", reopened, "apply 数", before, "→", after, "关闭", closed2)
            if not reopened or before != after or not closed2:
                fails.append(f"取消零 invoke {reopened} {before}->{after} {closed2}")

            if fails:
                print("FAIL:", "; ".join(fails))
                return 3
            print("PASS: 侧栏双区块/自定义图标/create_folder blockId/弹窗布局/两段式删除/取消零 invoke 全部通过")
            return 0
    finally:
        proc.terminate()


sys.exit(asyncio.run(main()))
