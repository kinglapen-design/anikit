<script setup lang="ts">
// 文本预览（5.6 + 文本编辑稿轮）：txt = APP 内编辑（防抖 1s 自动保存，2026-08-23 用户定；编辑不进 undo）；
// md = MarkDown 编辑器（本稿轮落地）——预览态 = markdown-it 渲染（排版 = styles/markdown.css 全局类，
// 稿 = 布局/中间面板-内容容器-文本编辑；任务列表 = 复选框通用类可点 toggle、代码块右上角复制钮），
// 点击正文进编辑态 = 全文源码编辑（防抖自动保存同 txt；悬浮栏换 MarkDown 工具栏，Esc / Ctrl+Enter 退回预览态）。
// md 图片引用 = 库内封闭路径解析不了 → 渲染占位文本（09 待办：库内资产引用解析）；
// md 链接本轮不可点（09 待办：默认浏览器打开）。
// 保存竞态纪律：保存请求绑定加载时 assetId（loadedId），切素材先 flush 在途编辑再加载新内容——
// 旧内容不会错存到新资产；响应到达时已切走则只更新 placements 不动编辑区状态。
import { computed, nextTick, onBeforeUnmount, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { createMd } from "../lib/markdown";
import { applyMdAction, toggleTaskLine } from "../lib/mdEdit";
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
const mdScrollRef = ref<HTMLElement>();
const editorRef = ref<HTMLTextAreaElement>();

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
  preview.setMdEditing(false);
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
  }
}

// —— 渲染态点击代理：复制钮 / 任务复选框 分流，其余进编辑态 ——
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
      box.classList.toggle("on"); // 先原位播对勾动画；draft 触发重渲染后视觉态一致
      draft.value = toggleTaskLine(draft.value, line);
    }
    return;
  }
  void enterEdit();
}

// —— 悬浮栏 MarkDown 工具栏动作（preview.requestMdAction → seq 推进；栏内 mousedown.prevent 保选区）——
watch(
  () => preview.mdActionSeq,
  () => {
    if (!editing.value || !isMd.value) return;
    const ta = editorRef.value;
    if (!ta) return;
    const r = applyMdAction(draft.value, ta.selectionStart, ta.selectionEnd, preview.mdActionName);
    draft.value = r.text;
    void nextTick(() => {
      ta.focus();
      ta.setSelectionRange(r.start, r.end);
    });
  },
);
</script>

<template>
  <div class="text-preview">
    <!-- md = 编辑器：预览态渲染（v-show 保滚动位）↔ 编辑态源码 textarea -->
    <template v-if="isMd">
      <div
        v-show="!editing"
        ref="mdScrollRef"
        class="md-body kit-scroll"
        v-html="rendered"
        @click="onMdClick"
      />
      <textarea
        v-if="editing"
        ref="editorRef"
        v-model="draft"
        class="editor md-editor kit-scroll"
        spellcheck="false"
        @keydown="onEditorKeydown"
      />
      <ScrollThumb :body="editing ? editorRef : mdScrollRef" />
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
