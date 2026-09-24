<script setup lang="ts">
// docx 预览（R6 新功能验证首例，PRD 04-04 条 6）：后端 services/docx 段落结构提取 → 只读渲染。
// 非版式级渲染（字体 / 字号 / 图片 / 表格框线不还原，表格按段落平铺）；编辑走「默认应用打开」。
// 排版复用 md 渲染态口径（文本编辑稿）；加载竞态守卫 = loadedId 绑定（同 PreviewText 模式）。
import { inject, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { readDocxAsset, type DocxContent } from "../lib/docxOps";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const preview = inject(PREVIEW_SESSION_KEY)!;

const doc = ref<DocxContent | null>(null);
const loadError = ref<string | null>(null);
const bodyRef = ref<HTMLElement>();

let loadedId: string | null = null;

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  loadedId = assetId;
  doc.value = null;
  loadError.value = null;
  try {
    const r = await readDocxAsset(assetId);
    if (loadedId !== assetId) return; // await 期间已切走
    doc.value = r;
  } catch (e) {
    if (loadedId !== assetId) return;
    loadError.value = e instanceof Error ? e.message : String(e);
  }
}
watch(() => preview.current?.assetId, () => void load(), { immediate: true });

// 段落标签：heading 1-6 → h1-h6，正文 / 空行 → p
function paraTag(heading: number): string {
  return heading >= 1 && heading <= 6 ? `h${heading}` : "p";
}
</script>

<template>
  <div class="docx-preview">
    <template v-if="doc">
      <div ref="bodyRef" class="docx-body md-body kit-scroll">
        <!-- 空文档（零段落 / 全空行）提示说明文字档 -->
        <p v-if="doc.paragraphs.length === 0" class="empty-hint">空文档</p>
        <component
          :is="paraTag(p.heading)"
          v-for="(p, i) in doc.paragraphs"
          :key="i"
          :class="{ list: p.list, blank: p.runs.length === 0 }"
        >
          <span v-if="p.list" class="bullet">•</span><span
            v-for="(r, j) in p.runs"
            :key="j"
            :class="{ b: r.bold, i: r.italic }"
            >{{ r.text }}</span
          >
        </component>
        <p v-if="doc.truncated" class="truncated-hint">文档过大，仅显示前一部分</p>
      </div>
      <ScrollThumb :body="bodyRef" />
    </template>
    <span v-if="loadError" class="error-text">{{ loadError }}</span>
  </div>
</template>

<style scoped>
.docx-preview {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  user-select: text; /* 文本可选中复制（stage 全局 none 覆盖回） */
}
/* —— 排版 = styles/markdown.css .md-body 全局类（文本编辑稿口径，PreviewText 共用） —— */
.docx-body {
  flex: 1;
  min-height: 0;
  white-space: pre-wrap; /* run 内 \n（w:br）/ 连续空格原样呈现 */
  overflow-wrap: break-word;
}
/* 空行段落（无 run）保持行高占位 */
.docx-body .blank::after {
  content: "\00a0";
}
/* 列表项：项目符号 + 悬挂缩进（md-body flex 子级，段距由容器 gap 供给） */
.docx-body .list {
  padding-left: 1.6em;
  text-indent: -1.6em;
}
.docx-body .bullet {
  padding-right: 0.6em;
  color: var(--n-l3);
}
.docx-body .b {
  font-weight: 700;
}
.docx-body .i {
  font-style: italic;
}
/* 空文档 / 截断提示（说明文字档 三级） */
.empty-hint,
.truncated-hint {
  color: var(--n-l3);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
/* 读取失败文案（居中底部，同 PreviewPanel error-text 口径） */
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
