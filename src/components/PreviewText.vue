<script setup lang="ts">
// 文本预览（5.6）：txt = APP 内编辑（防抖 1s 自动保存，2026-08-23 用户定；编辑不进 undo）；
// md = markdown-it 只读渲染（源码编辑归 6.2 提示词编辑，PRD 04-04 条 6 定案 markdown-it）。
// md 图片引用 = 库内封闭路径解析不了 → 渲染占位文本（09 待办：库内资产引用解析）；
// md 链接本轮不可点（09 待办：默认浏览器打开）。
// 保存竞态纪律：保存请求绑定加载时 assetId（loadedId），切素材先 flush 在途编辑再加载新内容——
// 旧内容不会错存到新资产；响应到达时已切走则只更新 placements 不动编辑区状态。
import { computed, onBeforeUnmount, ref, watch } from "vue";
import MarkdownIt from "markdown-it";
import ScrollThumb from "./ScrollThumb.vue";
import { readTextAsset, writeTextAsset } from "../lib/textOps";
import { useContentStore } from "../stores/content";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const content = useContentStore();

const isMd = computed(() => (preview.current?.ext ?? "").toLowerCase() === "md");

const source = ref(""); // 已加载 / 已保存内容（txt：保存基准）
const draft = ref(""); // txt 编辑草稿
const loadError = ref<string | null>(null);
const saveState = ref<"" | "saving" | "saved" | "fail">("");
const mdScrollRef = ref<HTMLElement>();
const editorRef = ref<HTMLTextAreaElement>();

const md = new MarkdownIt({ html: false, linkify: true, breaks: false });
// 图片引用解析不了（库内封闭）：渲染占位文本（记 09 待办）
md.renderer.rules.image = (tokens, idx) => `<span class="md-img">[图片 ${tokens[idx].content || "未命名"}]</span>`;
const rendered = computed(() => (isMd.value && source.value ? md.render(source.value) : ""));

const stateText = computed(() => {
  if (saveState.value === "saving") return "保存中…";
  if (saveState.value === "saved") return "已保存";
  if (saveState.value === "fail") return "保存失败";
  return "";
});

let loadedId: string | null = null;
let timer = 0; // txt 防抖定时器（须在 load/watch 前声明——immediate watch 同步触发 load 会立即访问）

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  await flushSave(); // 切走前落盘在途编辑（旧资产内容保存回旧资产）
  loadedId = assetId;
  source.value = "";
  draft.value = "";
  loadError.value = null;
  saveState.value = "";
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

// —— txt 编辑：防抖 1s 自动保存（2026-08-23 用户定）——
watch(draft, (v) => {
  if (isMd.value || v === source.value) return;
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

// 切素材 / 卸载组件前落盘在途编辑（防抖窗口期不丢稿）
async function flushSave(): Promise<void> {
  window.clearTimeout(timer);
  if (!isMd.value && draft.value !== source.value) await save();
}
onBeforeUnmount(() => void flushSave());
</script>

<template>
  <div class="text-preview">
    <!-- md = 渲染态（只读；编辑归 6.2） -->
    <template v-if="isMd">
      <div ref="mdScrollRef" class="md-body kit-scroll" v-html="rendered" />
      <ScrollThumb :body="mdScrollRef" />
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
/* —— md 渲染态：排版按 token 档（无设计稿，元素色档一律取既有 token） —— */
.md-body {
  flex: 1;
  min-height: 0;
  padding: var(--space-24) 32px;
  font-size: var(--text-body-size);
  line-height: 1.8;
  color: var(--text-normal);
}
.md-body:deep(h1),
.md-body:deep(h2),
.md-body:deep(h3),
.md-body:deep(h4),
.md-body:deep(h5),
.md-body:deep(h6) {
  margin: 1em 0 0.5em;
  line-height: 1.4;
  font-weight: 700;
  color: var(--text-normal);
}
.md-body:deep(h1:first-child),
.md-body:deep(h2:first-child),
.md-body:deep(h3:first-child) {
  margin-top: 0;
}
.md-body:deep(h1) {
  font-size: 1.7em;
}
.md-body:deep(h2) {
  font-size: 1.4em;
}
.md-body:deep(h3) {
  font-size: 1.15em;
}
.md-body:deep(p) {
  margin: 0 0 0.9em;
}
.md-body:deep(a) {
  color: var(--brand-1);
  pointer-events: none; /* 本轮不可点（09 待办：默认浏览器打开） */
  text-decoration: none;
}
.md-body:deep(code) {
  font-family: Consolas, "Cascadia Mono", monospace;
  font-size: 0.9em;
  background: var(--c4);
  border-radius: var(--radius-4, 4px);
  padding: 1px 5px;
}
.md-body:deep(pre) {
  margin: 0 0 1em;
  padding: var(--space-12) var(--space-16);
  background: var(--c2);
  border-radius: var(--radius-6, 6px);
  overflow-x: auto;
}
.md-body:deep(pre code) {
  background: none;
  padding: 0;
}
.md-body:deep(blockquote) {
  margin: 0 0 1em;
  padding: 2px 0 2px var(--space-12);
  border-left: 2px solid var(--brand-1);
  color: var(--text-label);
}
.md-body:deep(ul),
.md-body:deep(ol) {
  margin: 0 0 1em;
  padding-left: 1.6em;
}
.md-body:deep(li) {
  margin: 0.2em 0;
}
.md-body:deep(hr) {
  border: none;
  border-top: 1px solid var(--line-hover);
  margin: 1.4em 0;
}
.md-body:deep(table) {
  border-collapse: collapse;
  margin: 0 0 1em;
}
.md-body:deep(th),
.md-body:deep(td) {
  border: 1px solid var(--line-hover);
  padding: 5px var(--space-12);
}
.md-body:deep(.md-img) {
  color: var(--text-label);
  background: var(--c2);
  border-radius: var(--radius-4, 4px);
  padding: 2px 8px;
}
/* —— txt 编辑态 —— */
.editor {
  flex: 1;
  min-height: 0;
  resize: none;
  border: none;
  outline: none;
  background: transparent;
  color: var(--text-normal);
  padding: var(--space-16) var(--space-24);
  font-family: Consolas, "Cascadia Mono", monospace;
  font-size: var(--text-body-size);
  line-height: 1.7;
}
.save-state {
  position: absolute;
  right: var(--space-16);
  bottom: var(--space-12);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  pointer-events: none;
}
.save-state.fail {
  color: var(--error);
}
/* 读取 / 保存失败文案（说明文字档，居中底部，同 PreviewPanel error-text 口径） */
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--space-16);
  transform: translateX(-50%);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  white-space: nowrap;
  color: var(--error);
}
</style>
