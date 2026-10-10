<script setup lang="ts">
// 文本预览（5.6 + 文本编辑稿轮）：txt = APP 内编辑（防抖 1s 自动保存，2026-08-23 用户定；编辑不进 undo）；
// md = MarkDown 编辑器（本稿轮落地 + 二轮走查改版 + 2026-10-10 CM6 换代）——预览态 = markdown-it 渲染
//（排版 = styles/markdown.css 全局类；任务列表 = 复选框通用类可点 toggle、代码块右上角复制钮；**纯阅读不主动进编辑**——
// 点击文字 = 选中复制，进编辑唯一入口 = 悬浮栏铅笔钮 / Ctrl+E，2026-09-25 用户「只想复制文字」定）。
// 编辑态 = CodeMirror 6 Live Preview（所见所得，PRD 04-04 条 6 新口径推翻旧「不引入所见即所得」）：
// 光标外行内联渲染 / 光标行回落源码（lib/cmMd.ts 装配 + styles/cm-md.css 排版，数值同文本编辑稿 token）；
// 撤销 = CM6 原生事务栈（自实现 mdHistory 快照栈退役，「编辑不进 undo」口径不变）；
// 分屏 = 工具栏分屏钮开：左 CM6 编辑右 markdown-it 实时渲染（同 draft 驱动），中间分割条左右拖（splitRatio 会话级）。
// md 图片引用 = anikit 双形态渲染真图、点击内部跳转（2026-10-08 S3；渲染态 lib/markdown.ts 管道，
// 编辑态 cmMd 图片 widget，缩略图 URL 同源 lib/anikitLink.anikitThumbUrl）；
// md 链接可点（渲染态）：anikit:// 内部跳转，http(s) 默认浏览器（onMdClick 代理）。
// 保存竞态纪律：保存请求绑定加载时 assetId（loadedId），切素材先 flush 在途编辑再加载新内容——
// 旧内容不会错存到新资产；响应到达时已切走则只更新 placements 不动编辑区状态。
import { computed, inject, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { createMd } from "../lib/markdown";
import { applyMdAction, toggleTaskLine } from "../lib/mdEdit";
import { createMdEditor, type MdEditorHandle } from "../lib/cmMd";
import { readTextAsset, writeTextAsset } from "../lib/textOps";
import { jumpToAnikitRef, parseAnikitRef } from "../lib/anikitLink";
import { invoke } from "@tauri-apps/api/core";
import { useContentStore } from "../stores/content";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";
import { useToastStore } from "../stores/toast";

const preview = inject(PREVIEW_SESSION_KEY)!;
const content = useContentStore();
const toast = useToastStore();

const isMd = computed(() => (preview.current?.ext ?? "").toLowerCase() === "md");

const source = ref(""); // 已加载 / 已保存内容（保存基准）
const draft = ref(""); // 编辑草稿（md 渲染源 = 草稿，复选框 toggle / 编辑实时反映）
const loadError = ref<string | null>(null);
const saveState = ref<"" | "saving" | "saved" | "fail">("");
const editing = ref(false); // md 编辑态（false = 渲染预览态）
const split = ref(false); // 分屏（左源码右实时渲染，会话级；切素材重置）
const splitRatio = ref(0.5); // 分屏左栏宽度占比（分割条拖动，clamp 0.25-0.75）
const mdScrollRef = ref<HTMLElement>();
const editorRef = ref<HTMLTextAreaElement>(); // txt 编辑区（md 已换 CM6，见 cmHostRef）
const mdSplitBodyRef = ref<HTMLElement>(); // 分屏右栏渲染体（ScrollThumb 第二轨）
const cmHostRef = ref<HTMLElement>(); // md CM6 挂载点
const cmScrollEl = ref<HTMLElement>(); // CM6 滚动容器（ScrollThumb / 滚动保活取元用）

const md = createMd();
const rendered = computed(() => (isMd.value && draft.value ? md.render(draft.value) : ""));

const stateText = computed(() => {
  if (saveState.value === "saving") return "保存中…";
  if (saveState.value === "saved") return "已保存";
  if (saveState.value === "fail") return "保存失败";
  return "";
});

let loadedId: string | null = null;
let timer = 0; // 防抖定时器（须在 load/watch 前声明——immediate watch 同步触发 load 会立即访问）

// —— 滚动保活（2026-10-02 用户实机报「重新聚焦回第一行」根治，声明须在 immediate watch 前）——
// 根因：dockview 对已激活 panel 重复 openPanel 无守卫（pane 重新聚焦 panelApi.setActive /
// 同 group 切标签页均触发）→ panel 内容 DOM detach→reattach，同元素 scrollTop 被浏览器归零
//（探针 cdp-md-scroll-repro 实锤：全程无 load 重跑、元素同一、display 正常）；
// v-show 进出编辑态的 display:none 同款归零（display 恢复后滚动位置不还原）。
// 对策：滚动位置记进组件闭包（DOM 操作够不着），dockview 重挂 / 显隐回显时恢复；
// 切素材 load() 清零从新文档顶读。挂载与显隐的恢复接线在 load() 后（keepMo / watch）。
const keepEls = new Map<string, () => HTMLElement | undefined>();
const keepPos = new Map<string, number>();
function onKeepScroll(key: string, el: HTMLElement | undefined): void {
  if (el) keepPos.set(key, el.scrollTop);
}
function restoreKeeps(): void {
  for (const [key, getEl] of keepEls) {
    const el = getEl();
    const saved = keepPos.get(key) ?? 0;
    if (!el || saved <= 0 || el.scrollTop >= saved - 1) continue;
    el.scrollTop = saved; // 首拍即改；布局未稳被钳时 nextTick 补一拍
    void nextTick(() => {
      if (el.scrollTop < saved - 1) el.scrollTop = saved;
    });
  }
}
keepEls.set("md", () => mdScrollRef.value);
keepEls.set("editor", () => (isMd.value ? cmScrollEl.value : editorRef.value));
keepEls.set("split", () => mdSplitBodyRef.value);

// —— md CM6 编辑器实例（编辑态存活，出编辑态 / 切素材销毁；撤销 = 内核原生事务栈） ——
let mdEditor: MdEditorHandle | null = null;
function destroyMdEditor(): void {
  mdEditor?.destroy();
  mdEditor = null;
  cmScrollEl.value = undefined;
}

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  await flushSave(); // 切走前落盘在途编辑（旧资产内容保存回旧资产）
  loadedId = assetId;
  keepPos.clear(); // 切素材：滚动记忆清零（新文档从顶读，防保活把旧位置拉回）
  source.value = "";
  draft.value = "";
  loadError.value = null;
  saveState.value = "";
  destroyMdEditor();
  editing.value = false;
  split.value = false;
  preview.setMdEditing(false);
  try {
    const r = await readTextAsset(assetId);
    if (loadedId !== assetId) return;
    source.value = r.content;
    draft.value = r.content;
    consumePendingMdEdit(); // 「打开编辑器」md 直进编辑态：内容就绪后才允许消费（防挂载竞态）
  } catch (e) {
    if (loadedId !== assetId) return;
    loadError.value = e instanceof Error ? e.message : String(e);
    preview.pendingMdEdit = false; // 内容加载失败：直进编辑态信号一并作废（无内容可编辑）
  }
}
watch(() => preview.current?.assetId, () => void load(), { immediate: true });

// 保活的挂载与显隐恢复（声明在 load() 前，见上）：
// 宿主 .dv-content-container 的 childList 变化 = dockview 重挂本 panel → 恢复；
// 显隐切换（display:none 同款归零，MO 感知不到 DOM 未变的切换）走 watch——
// 回预览态 md-body 恢复；分屏开关右栏新挂载恢复（进编辑态不恢复——enterEdit 设计即「光标落末尾」）
let keepMo: MutationObserver | null = null;
onMounted(() => {
  const host = (mdScrollRef.value ?? editorRef.value)?.closest(".dv-content-container");
  if (!host) return;
  keepMo = new MutationObserver(restoreKeeps);
  keepMo.observe(host, { childList: true });
});
onBeforeUnmount(() => {
  keepMo?.disconnect();
  keepMo = null;
});
watch(editing, (v) => {
  if (!v) void nextTick(() => restoreKeeps());
});
watch(split, () => void nextTick(() => restoreKeeps()));

// 「打开编辑器」md 直进编辑态（提示词轮，preview.pendingMdEdit 一次性信号）：
// open() 为异步 loadAsset——置信号时文本尚未加载，消费点必须在内容就绪后（load() 成功尾）；
// 另挂信号 watch 兜底「预览已停在该文本资产上再点打开编辑器」（assetId 不变 load 不重跑）的场景
function consumePendingMdEdit(): void {
  if (!preview.pendingMdEdit) return;
  if (!loadedId || loadedId !== preview.current?.assetId) return; // 内容未就绪——等 load() 完成点消费
  preview.pendingMdEdit = false;
  void enterEdit(); // 内部守卫 isMd（txt 恒编辑态无需处理）
}
watch(() => preview.pendingMdEdit, consumePendingMdEdit);

// —— 编辑：防抖 1s 自动保存（2026-08-23 用户定）——
watch(draft, (v) => {
  mdEditor?.setDoc(v); // 外部 draft 变更回灌 CM6（分屏右栏复选框 toggle 等；内核内变更 setDoc 自守恒等）
  if (v === source.value) return;
  window.clearTimeout(timer);
  saveState.value = "saving";
  timer = window.setTimeout(() => void save(), 1000);
});

async function save(): Promise<void> {
  const assetId = loadedId; // 绑定加载时资产（防切素材后旧草稿错存新资产）
  if (!assetId || draft.value === source.value) return;
  saveState.value = "saving";
  try {
    const r = await writeTextAsset(assetId, draft.value);
    // 后端已落盘：PlacementView 同步恒做（简介"文件大小"档 / 排序"修改时间"键即时正确；
    // 资产级字段，同资产全部 placement 同刷）
    for (const p of content.placements) {
      if (p.assetId === assetId) {
        p.size = r.size;
        p.modifiedAt = r.modifiedAt;
      }
    }
    if (loadedId !== assetId) return; // 已切走：编辑区状态不动
    source.value = draft.value;
    saveState.value = "saved";
  } catch (e) {
    if (loadedId !== assetId) return;
    saveState.value = "fail";
    loadError.value = e instanceof Error ? e.message : String(e);
  }
}

// 切素材 / 退出编辑 / 卸载组件前落盘在途编辑（防抖窗口期不丢稿）
async function flushSave(): Promise<void> {
  window.clearTimeout(timer);
  if (draft.value !== source.value) await save();
}
onBeforeUnmount(() => {
  preview.setMdEditing(false);
  destroyMdEditor();
  void flushSave();
});

// —— md 编辑态进出（进编辑唯一入口 = 铅笔钮 / Ctrl+E / 打开编辑器信号；Esc / Ctrl+Enter / Ctrl+E 退出——
//    键位由 cmMd Prec.high 键图回调 onExitEdit，组件不再挂 DOM keydown）——
async function enterEdit(): Promise<void> {
  if (!isMd.value || editing.value) return;
  editing.value = true;
  preview.setMdEditing(true);
  await nextTick();
  const host = cmHostRef.value;
  if (!host) return;
  destroyMdEditor(); // 防御：编辑态重进 / 重挂不叠实例
  mdEditor = createMdEditor({
    parent: host,
    doc: draft.value,
    onDocChanged: (text) => {
      draft.value = text; // → 防抖保存管线（watch draft）
    },
    onCursorActivity: () => scheduleSplitScroll(),
    onScroll: () => onEditScroll(),
    onExitEdit: () => void exitEdit(),
  });
  cmScrollEl.value = mdEditor.view.scrollDOM;
  mdEditor.focusEnd(); // 光标落末尾（接着写）
}

async function exitEdit(): Promise<void> {
  if (!editing.value) return;
  await flushSave();
  destroyMdEditor();
  editing.value = false;
  preview.setMdEditing(false);
}

// 退出键上浮截停（旧 textarea stopPropagation 口径平移）：CM6 键图在 contentDOM 先处理退出，
// 冒泡到宿主时拦停——Esc / Ctrl+Enter 不再上浮到面板级 / 全局监听（否则 Esc 会连带关预览，探针 F 实锤）
function onCmKeydownGuard(e: KeyboardEvent): void {
  if (e.key === "Escape" || (e.key === "Enter" && (e.ctrlKey || e.metaKey))) e.stopPropagation();
}

// —— 分屏滚动（2026-09-25 用户定「两个滚动条合并成一个直接联动」）——
// 单滚轮（右栏渲染体主体，贴右缘）；双向按比例联动（±1 阈值防往返震荡）；
// 输入 / 光标移动 = typewriter 跟进：光标行对应渲染块滚到视口中间（用户「书写常在最下面，
// 预览最后一行固定中间」），16ms 定时器合一（rAF 弃用——headless 按需出帧模式下帧生产停摆时回调永不达，
// 2026-10-10 踩坑），精调期间 linkLock 锁反向回写不拽左栏。
// linkLock = 墙钟截止时刻（不用计数 + 定时器解锁——headless 定时器节流会让解锁回调迟数秒，
// 锁长期卡死联动全瘫，同日实锤；墙钟到点自然失效，零回调依赖）
let linkLockUntil = 0;
let splitScrollRaf = 0; // setTimeout 句柄（历史名留 raf；16ms 合一节流）

function linkScroll(src: HTMLElement, dst: HTMLElement): void {
  if (performance.now() < linkLockUntil) return;
  const sMax = src.scrollHeight - src.clientHeight;
  const dMax = dst.scrollHeight - dst.clientHeight;
  if (sMax <= 0 || dMax <= 0) return;
  const target = Math.round((src.scrollTop / sMax) * dMax);
  if (Math.abs(dst.scrollTop - target) > 1) dst.scrollTop = target;
}

function onEditScroll(): void {
  onKeepScroll("editor", cmScrollEl.value);
  const src = cmScrollEl.value;
  const body = mdSplitBodyRef.value;
  if (src && body) linkScroll(src, body);
}

function onSideScroll(): void {
  onKeepScroll("split", mdSplitBodyRef.value);
  const src = cmScrollEl.value;
  const body = mdSplitBodyRef.value;
  if (src && body) linkScroll(body, src);
}

function scheduleSplitScroll(): void {
  if (!split.value) return;
  // 定时器合一（2026-10-10 踩坑：headless 按需出帧模式 rAF 会整段停摆成僵尸回调——帧生产依赖视觉损伤，
  // 空损伤窗口期永不回调、守卫 id 永锁；定时器无此依赖，16ms ≈ 一帧，连续键入合一后者赢）
  if (splitScrollRaf) window.clearTimeout(splitScrollRaf);
  splitScrollRaf = window.setTimeout(() => {
    splitScrollRaf = 0;
    const view = mdEditor?.view;
    const body = mdSplitBodyRef.value;
    if (!view || !body) return;
    const line = view.state.doc.lineAt(view.state.selection.main.head).number - 1; // 0 起，同 data-line-start 口径
    let target: HTMLElement | null = null;
    for (const el of body.querySelectorAll<HTMLElement>("[data-line-start]")) {
      if (Number(el.dataset.lineStart) <= line) target = el;
      else break;
    }
    if (!target) return;
    linkLockUntil = performance.now() + 100; // 墙钟锁 100ms：盖住本写入引发的异步 scroll 事件回环
    const bRect = body.getBoundingClientRect();
    const tRect = target.getBoundingClientRect();
    body.scrollTop += tRect.top - bRect.top - (bRect.height - tRect.height) / 2;
  }, 16);
}

// —— 渲染态点击代理：复制钮 / anikit 图与链接 / 任务复选框 分流；其余 = 无操作（纯阅读可复制，进编辑走铅笔钮 / Ctrl+E）——
function onMdClick(e: MouseEvent): void {
  const t = e.target as HTMLElement;
  const copy = t.closest(".md-copy");
  if (copy) {
    const text = copy.closest(".md-code")?.querySelector("code")?.textContent ?? "";
    void navigator.clipboard.writeText(text).then(
      () => toast.notify({ message: "已复制代码", icon: "success" }),
      () => toast.notify({ message: "复制失败", icon: "fail" }),
    );
    return;
  }
  // anikit 引用图片（2026-10-08 S3）：点击 = 内部跳转选中该资产（渲染层行为，源码保持裸 ![]() 语法）；
  // preventDefault 必须——带跳转回链形态 [![..](..)](..open/item/x) 的 img 被 <a> 包裹，
  // 不拦默认行为会让 webview 整窗导航到 open 端点回页（2026-10-08 实机事故：APP 被回页替换关不掉）
  const img = t.closest<HTMLElement>("img.md-asset[data-anikit-id]");
  if (img?.dataset.anikitId) {
    e.preventDefault();
    jumpToAnikitRef({ kind: "item", id: img.dataset.anikitId });
    return;
  }
  // 链接可点（同轮勾销 09 待办）：anikit:// = 内部跳转；http(s) = 默认浏览器打开
  const a = t.closest("a");
  if (a) {
    e.preventDefault();
    const href = a.getAttribute("href") ?? "";
    const ref = parseAnikitRef(href);
    if (ref) jumpToAnikitRef(ref);
    else if (/^https?:\/\//.test(href)) void invoke("open_external_url", { url: href }).catch((err) => toast.notify({ message: String(err), icon: "fail" }));
    return;
  }
  const box = t.closest<HTMLElement>(".kit-check-box[data-task-line]");
  if (box) {
    const line = Number(box.dataset.taskLine);
    if (Number.isInteger(line) && line >= 0) {
      box.classList.toggle("on"); // 先原位播对勾动画；draft 触发重渲染后视觉态一致
      draft.value = toggleTaskLine(draft.value, line); // 分屏态经 watch(draft) 回灌 CM6（mdEditor.setDoc）
    }
  }
}

// —— 编辑态粘贴补全（2026-10-08 S3）已随 CM6 内核平移进 lib/cmMd.ts domEventHandlers.paste（同口径：
//  剪贴板整串为 anikit item 引用 → 插入 ![名](anikit://item/{id}) 图片语法；独立撤销步 userEvent input.paste）——

// —— 分屏分割条拖动（§9 ③ 窗口监听口径）：splitRatio clamp 0.25-0.75 ——
function onSplitDrag(e: PointerEvent): void {
  e.preventDefault();
  const bar = e.currentTarget as HTMLElement;
  const wrap = bar.parentElement;
  if (!wrap) return;
  bar.classList.add("drag");
  const rect = wrap.getBoundingClientRect();
  const move = (ev: PointerEvent): void => {
    splitRatio.value = Math.min(0.75, Math.max(0.25, (ev.clientX - rect.left) / rect.width));
  };
  const up = (): void => {
    bar.classList.remove("drag");
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
}

// —— 悬浮栏 MarkDown 工具栏动作（preview.requestMdAction → seq 推进；栏内 mousedown.prevent 保选区）——
watch(
  () => preview.mdActionSeq,
  () => {
    if (!isMd.value) return;
    const name = preview.mdActionName;
    if (name === "edit") {
      void enterEdit(); // 铅笔钮 / Ctrl+E（预览态）
      return;
    }
    if (name === "preview") {
      void exitEdit(); // 眼睛钮（Esc / Ctrl+E 同义）
      return;
    }
    if (name === "split") {
      if (editing.value) split.value = !split.value;
      else {
        split.value = true; // 预览态点分屏 = 直接进分屏编辑
        void enterEdit();
      }
      return;
    }
    if (!editing.value) return;
    if (!mdEditor) return;
    const sel = mdEditor.view.state.selection.main;
    const r = applyMdAction(draft.value, sel.from, sel.to, name);
    mdEditor.applySource(r.text, r.start, r.end, preview.mdActionSeq); // 独立撤销步（userEvent 带 seq）
  },
);
</script>

<template>
  <div class="text-preview">
    <!-- md = 编辑器：预览态渲染（v-show 不重建，滚动由 keepPos 保活恢复）↔ 编辑态（单栏 / 分屏左码右渲） -->
    <template v-if="isMd">
      <div
        v-show="!editing"
        ref="mdScrollRef"
        class="md-body kit-scroll"
        v-html="rendered"
        @click="onMdClick"
        @scroll.passive="onKeepScroll('md', mdScrollRef)"
      />
      <div v-if="editing" class="md-edit-wrap" :class="{ split }">
        <div class="pane" :style="split ? { flexGrow: splitRatio } : undefined">
          <!-- md 编辑 = CM6 Live Preview 内核（lib/cmMd.ts 装配；排版 styles/cm-md.css 全局类） -->
          <div ref="cmHostRef" class="cm-host" @keydown="onCmKeydownGuard" />
          <ScrollThumb v-if="!split" :body="cmScrollEl" />
        </div>
        <template v-if="split">
          <div class="md-split-bar" @pointerdown="onSplitDrag" />
          <div class="pane" :style="{ flexGrow: 1 - splitRatio }">
            <div
              ref="mdSplitBodyRef"
              class="md-body split-side kit-scroll"
              v-html="rendered"
              @click="onMdClick"
              @scroll.passive="onSideScroll"
            />
            <ScrollThumb :body="mdSplitBodyRef" />
          </div>
        </template>
      </div>
      <ScrollThumb v-if="!editing" :body="mdScrollRef" />
      <span v-if="saveState" class="save-state" :class="{ fail: saveState === 'fail' }">{{ stateText }}</span>
    </template>
    <!-- txt = 编辑态（textarea 即预览，防抖自动保存） -->
    <template v-else>
      <textarea
        ref="editorRef"
        v-model="draft"
        class="editor kit-scroll"
        spellcheck="false"
        @scroll.passive="onKeepScroll('editor', editorRef)"
      />
      <ScrollThumb :body="editorRef" />
      <span v-if="saveState" class="save-state" :class="{ fail: saveState === 'fail' }">{{ stateText }}</span>
    </template>
    <span v-if="loadError" class="error-text">{{ loadError }}</span>
  </div>
</template>

<style scoped>
.text-preview {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  user-select: text; /* 文本可选中复制（stage 全局 none 覆盖回） */
}
/* —— md 渲染态排版 = styles/markdown.css 全局类（PreviewDocx 共用；文本编辑稿口径） —— */
.md-body {
  flex: 1;
  min-height: 0;
}
/* 底部 66 遮挡补偿（§18 口径：距底 12 + 栏高 42 + 呼吸 12）——md 语境悬浮栏工具栏恒显，
   预览态单栏渲染体同补（工具栏会挡最后一行，2026-09-25 三轮走查实机） */
.text-preview > .md-body {
  padding-bottom: 66px;
}
/* —— 编辑态（txt 基础档；md = CM6 内核，排版 styles/cm-md.css 全局类——版心 84 / 底 66 让位悬浮栏
   工具栏同 §18 遮挡补偿口径；分屏两侧版心收窄 24 见 .md-edit-wrap.split 下覆写） —— */
.editor {
  flex: 1;
  min-height: 0;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: var(--n-l2);
  padding: var(--n-space-16) var(--n-space-24);
  font-family: Consolas, "Cascadia Mono", monospace;
  font-size: var(--n-fs-body);
  line-height: 1.7;
}
/* CM6 宿主与内核布局（外观数值全在 cm-md.css；此处只管 flex 填充与滚动容器） */
.cm-host {
  flex: 1;
  min-height: 0;
  display: flex;
}
.cm-host :deep(.cm-editor) {
  flex: 1;
  min-width: 0;
  min-height: 0;
}
.cm-host :deep(.cm-scroller) {
  overflow: auto;
}
/* —— 编辑态容器（单栏 / 分屏双列）；pane = 滚动容器包裹层（ScrollThumb 定位语境） —— */
.md-edit-wrap {
  flex: 1;
  min-height: 0;
  display: flex;
}
.md-edit-wrap .pane {
  position: relative;
  flex: 1 1 0;
  min-width: 0;
  display: flex;
}
.md-edit-wrap .pane .cm-host,
.md-edit-wrap .pane .md-body {
  flex: 1;
  min-width: 0;
}
/* 分屏 = 左 CM6 编辑右实时渲染（用户 2026-09-25 定）：两侧版心收窄 24（全宽 84 稿值为单栏档，
   半宽沿用太奢）；分割条 = 1px 通用/三级线 + 左右 2px 命中扩边，hover / 拖拽 = 品牌色
   （全 app 拖拽线同色系口径：插入提示 / 框选 §22） */
.md-edit-wrap.split .cm-host :deep(.cm-content) {
  padding: var(--n-space-12) var(--n-space-24) calc(66px + 50vh);
}
.md-split-bar {
  flex: none;
  width: 5px;
  cursor: col-resize;
  position: relative;
}
.md-split-bar::before {
  content: "";
  position: absolute;
  left: 2px;
  top: 0;
  bottom: 0;
  width: 1px;
  background: var(--n-g3);
}
.md-split-bar:hover::before,
.md-split-bar.drag::before {
  background: var(--n-brand);
}
.md-body.split-side {
  min-width: 0;
  overflow-y: auto;
  padding: var(--n-space-12) var(--n-space-24) 66px;
}
.save-state {
  position: absolute;
  right: var(--n-space-16);
  bottom: var(--n-space-12);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  pointer-events: none;
}
.save-state.fail {
  color: var(--n-danger);
}
/* 读取 / 保存失败文案（说明文字档，居中底部，同 PreviewPanel error-text 口径） */
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--n-space-16);
  transform: translateX(-50%);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  white-space: nowrap;
  color: var(--n-danger);
}
</style>
