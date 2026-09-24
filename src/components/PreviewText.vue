<script setup lang="ts">
// 文本预览（5.6 + 文本编辑稿轮）：txt = APP 内编辑（防抖 1s 自动保存，2026-08-23 用户定；编辑不进 undo）；
// md = MarkDown 编辑器（本稿轮落地 + 二轮走查改版）——预览态 = markdown-it 渲染（排版 = styles/markdown.css
// 全局类；任务列表 = 复选框通用类可点 toggle、代码块右上角复制钮；**纯阅读不主动进编辑**——点击文字 =
// 选中复制，进编辑唯一入口 = 悬浮栏铅笔钮 / Ctrl+E，2026-09-25 用户「只想复制文字」定）。
// 编辑态 = 全文源码编辑（防抖自动保存同 txt；悬浮栏换全格式工具栏，眼睛钮 / Esc / Ctrl+Enter / Ctrl+E 回预览）；
// 分屏 = 工具栏分屏钮开：左源码右实时渲染（同 draft 驱动），中间分割条左右拖（splitRatio 会话级）。
// 编辑撤销（2026-09-25 走查补）：textarea 原生撤销遇程序化修改（工具栏动作 / toggle）即断栈——
// 自实现快照历史（lib/mdHistory，组件会话级），编辑态 Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y；
// 与库撤销无关（「编辑不进 undo」口径不变，全局 Ctrl+Z 本对输入框让位）。
// md 图片引用 = 库内封闭路径解析不了 → 渲染占位文本（09 待办：库内资产引用解析）；
// md 链接本轮不可点（09 待办：默认浏览器打开）。
// 保存竞态纪律：保存请求绑定加载时 assetId（loadedId），切素材先 flush 在途编辑再加载新内容——
// 旧内容不会错存到新资产；响应到达时已切走则只更新 placements 不动编辑区状态。
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { createMd } from "../lib/markdown";
import { applyMdAction, toggleTaskLine } from "../lib/mdEdit";
import { createMdHistory, type HistEntry } from "../lib/mdHistory";
import { readTextAsset, writeTextAsset } from "../lib/textOps";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";
import { useToastStore } from "../stores/toast";

const preview = usePreviewStore();
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
const editorRef = ref<HTMLTextAreaElement>();
const mdSplitBodyRef = ref<HTMLElement>(); // 分屏右栏渲染体（ScrollThumb 第二轨）

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

// —— 编辑撤销栈（组件会话级；prevSnap = 最近一次已知的 draft+选区快照） ——
const history = createMdHistory();
let prevSnap: HistEntry = { text: "", start: 0, end: 0 };

function snapOf(ta: HTMLTextAreaElement): HistEntry {
  return { text: ta.value, start: ta.selectionStart, end: ta.selectionEnd };
}

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  await flushSave(); // 切走前落盘在途编辑（旧资产内容保存回旧资产）
  loadedId = assetId;
  source.value = "";
  draft.value = "";
  loadError.value = null;
  saveState.value = "";
  editing.value = false;
  split.value = false;
  preview.setMdEditing(false);
  history.reset();
  prevSnap = { text: "", start: 0, end: 0 };
  try {
    const r = await readTextAsset(assetId);
    if (loadedId !== assetId) return;
    source.value = r.content;
    draft.value = r.content;
  } catch (e) {
    if (loadedId !== assetId) return;
    loadError.value = e instanceof Error ? e.message : String(e);
  }
}
watch(() => preview.current?.assetId, () => void load(), { immediate: true });

// —— 编辑：防抖 1s 自动保存（2026-08-23 用户定）——
watch(draft, (v) => {
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
  void flushSave();
});

// —— md 编辑态进出（预览态点击正文进入；Esc / Ctrl+Enter 退出）——
async function enterEdit(): Promise<void> {
  if (!isMd.value || editing.value) return;
  editing.value = true;
  preview.setMdEditing(true);
  await nextTick();
  const ta = editorRef.value;
  if (ta) {
    ta.focus();
    ta.setSelectionRange(ta.value.length, ta.value.length); // 光标落末尾（接着写）
    prevSnap = snapOf(ta); // 撤销基线 = 进入编辑态时刻
  }
}

async function exitEdit(): Promise<void> {
  if (!editing.value) return;
  await flushSave();
  editing.value = false;
  preview.setMdEditing(false);
}

function onEditorKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape" || (e.key === "Enter" && (e.ctrlKey || e.metaKey))) {
    e.stopPropagation();
    e.preventDefault();
    void exitEdit();
    return;
  }
  // Ctrl+E = 编辑↔预览互切（编辑态内 = 回预览；预览态进编辑 = useGlobalKeys 追加分支；
  // 输入框内全局快捷键让位，不冲突「发送至 Animate」fla 键）
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "e") {
    e.preventDefault();
    void exitEdit();
    return;
  }
  // 撤销 / 重做：preventDefault 禁 textarea 原生栈（程序化修改会断它，统一走自实现栈）；
  // 不 stopPropagation——全局 Ctrl+Z 本对输入框让位（useGlobalKeys isEditableTarget）
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
    e.preventDefault();
    if (e.shiftKey) doRedo();
    else doUndo();
    return;
  }
  if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
    e.preventDefault();
    doRedo();
  }
}

function doUndo(): void {
  const ta = editorRef.value;
  const entry = history.undo(ta ? snapOf(ta) : prevSnap);
  if (!entry) return;
  applySnap(entry);
}

function doRedo(): void {
  const ta = editorRef.value;
  const entry = history.redo(ta ? snapOf(ta) : prevSnap);
  if (!entry) return;
  applySnap(entry);
}

function applySnap(entry: HistEntry): void {
  draft.value = entry.text;
  prevSnap = entry;
  void nextTick(() => {
    const ta = editorRef.value;
    if (ta) {
      ta.focus();
      ta.setSelectionRange(entry.start, entry.end);
    }
  });
}

// 键入变更记录（@input 在 v-model 同步后触发：prevSnap 即变更前态；连续键入 800ms 合并一步）
function onDraftInput(): void {
  const ta = editorRef.value;
  if (!ta || !isMd.value) return;
  history.record(prevSnap, true);
  prevSnap = snapOf(ta);
  scheduleSplitScroll();
}

// —— 分屏滚动（2026-09-25 用户定「两个滚动条合并成一个直接联动」）——
// 单滚轮（右栏渲染体主体，贴右缘）；双向按比例联动（±1 阈值防往返震荡）；
// 输入 / 光标移动 = typewriter 跟进：光标行对应渲染块滚到视口中间（用户「书写常在最下面，
// 预览最后一行固定中间」），精调期间 linkLock 锁反向回写不拽左栏。
let linkLock = 0;
let splitScrollRaf = 0;

function linkScroll(src: HTMLElement, dst: HTMLElement): void {
  if (linkLock > 0) return;
  const sMax = src.scrollHeight - src.clientHeight;
  const dMax = dst.scrollHeight - dst.clientHeight;
  if (sMax <= 0 || dMax <= 0) return;
  const target = Math.round((src.scrollTop / sMax) * dMax);
  if (Math.abs(dst.scrollTop - target) > 1) dst.scrollTop = target;
}

function onEditScroll(): void {
  const ta = editorRef.value;
  const body = mdSplitBodyRef.value;
  if (ta && body) linkScroll(ta, body);
}

function onSideScroll(): void {
  const ta = editorRef.value;
  const body = mdSplitBodyRef.value;
  if (ta && body) linkScroll(body, ta);
}

function scheduleSplitScroll(): void {
  if (!split.value || splitScrollRaf) return;
  splitScrollRaf = requestAnimationFrame(() => {
    splitScrollRaf = 0;
    const ta = editorRef.value;
    const body = mdSplitBodyRef.value;
    if (!ta || !body) return;
    const line = ta.value.slice(0, ta.selectionStart).split("\n").length - 1;
    let target: HTMLElement | null = null;
    for (const el of body.querySelectorAll<HTMLElement>("[data-line-start]")) {
      if (Number(el.dataset.lineStart) <= line) target = el;
      else break;
    }
    if (!target) return;
    linkLock++;
    const bRect = body.getBoundingClientRect();
    const tRect = target.getBoundingClientRect();
    body.scrollTop += tRect.top - bRect.top - (bRect.height - tRect.height) / 2;
    // scroll 事件异步派发窗口留两帧再解锁（反向回写不拽左栏）
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        linkLock--;
      });
    });
  });
}

// —— 渲染态点击代理：复制钮 / 任务复选框 分流；其余 = 无操作（纯阅读可复制，进编辑走铅笔钮 / Ctrl+E）——
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
  const box = t.closest<HTMLElement>(".kit-check-box[data-task-line]");
  if (box) {
    const line = Number(box.dataset.taskLine);
    if (Number.isInteger(line) && line >= 0) {
      history.record({ text: draft.value, start: 0, end: 0 }, false); // toggle = 独立撤销步（编辑态可撤）
      box.classList.toggle("on"); // 先原位播对勾动画；draft 触发重渲染后视觉态一致
      draft.value = toggleTaskLine(draft.value, line);
    }
  }
}

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
    const ta = editorRef.value;
    if (!ta) return;
    history.record(snapOf(ta), false); // 工具栏动作 = 独立撤销步
    const r = applyMdAction(draft.value, ta.selectionStart, ta.selectionEnd, name);
    draft.value = r.text;
    prevSnap = { text: r.text, start: r.start, end: r.end };
    void nextTick(() => {
      ta.focus();
      ta.setSelectionRange(r.start, r.end);
    });
  },
);
</script>

<template>
  <div class="text-preview">
    <!-- md = 编辑器：预览态渲染（v-show 保滚动位）↔ 编辑态（单栏 / 分屏左码右渲） -->
    <template v-if="isMd">
      <div
        v-show="!editing"
        ref="mdScrollRef"
        class="md-body kit-scroll"
        v-html="rendered"
        @click="onMdClick"
      />
      <div v-if="editing" class="md-edit-wrap" :class="{ split }">
        <div class="pane" :style="split ? { flexGrow: splitRatio } : undefined">
          <textarea
            ref="editorRef"
            v-model="draft"
            class="editor md-editor kit-scroll"
            spellcheck="false"
            @input="onDraftInput"
            @keydown="onEditorKeydown"
            @keyup="scheduleSplitScroll"
            @click="scheduleSplitScroll"
            @scroll.passive="onEditScroll"
          />
          <ScrollThumb v-if="!split" :body="editorRef" />
        </div>
        <template v-if="split">
          <div class="md-split-bar" @pointerdown="onSplitDrag" />
          <div class="pane" :style="{ flexGrow: 1 - splitRatio }">
            <div
              ref="mdSplitBodyRef"
              class="md-body split-side kit-scroll"
              v-html="rendered"
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
      <textarea ref="editorRef" v-model="draft" class="editor kit-scroll" spellcheck="false" />
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
/* —— 编辑态（txt / md 源码共用基础档；md 版心随渲染态 84、底 66 让位悬浮栏工具栏——
   §18 遮挡补偿口径：距底 12 + 栏高 42 + 呼吸 12） —— */
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
.md-editor {
  padding: var(--n-space-12) calc(var(--n-space-12) + var(--n-space-72)) 66px;
  font-family: var(--n-font-mono);
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
.md-edit-wrap .pane .editor,
.md-edit-wrap .pane .md-body {
  flex: 1;
  min-width: 0;
}
/* 分屏 = 左源码右实时渲染（用户 2026-09-25 定）：两侧版心收窄 24（全宽 84 稿值为单栏档，
   半宽沿用太奢）；分割条 = 1px 通用/三级线 + 左右 2px 命中扩边，hover / 拖拽 = 品牌色
   （全 app 拖拽线同色系口径：插入提示 / 框选 §22） */
.md-edit-wrap.split .md-editor {
  padding: var(--n-space-12) var(--n-space-24) 66px;
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
