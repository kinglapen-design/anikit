<script setup lang="ts">
// docx 预览（R6 首例 → 2026-10-10 mammoth 换引擎，PRD 04-04 条 6）：后端 read_docx_binary 给原始字节
// → 前端 mammoth.js 转 HTML 只读渲染——表格带框线 / 图片内嵌 / 标题 / 列表 / 粗斜体；
// 浮动布局图片与文本框降级顺序内嵌、形状批注丢失（已知口径，真撞上再升 docx-preview）。
// 编辑走「默认应用打开」。排版 = styles/markdown.css .md-body 全局类（文本编辑稿口径，标签选择器天然命中）；
// 加载竞态守卫 = loadedId 绑定（同 PreviewText 模式）。
import { inject, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import { docxToHtml, readDocxBinary } from "../lib/docxOps";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";

const preview = inject(PREVIEW_SESSION_KEY)!;

const html = ref<string | null>(null);
const loadError = ref<string | null>(null);
const bodyRef = ref<HTMLElement>();

let loadedId: string | null = null;

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  loadedId = assetId;
  html.value = null;
  loadError.value = null;
  try {
    const buf = await readDocxBinary(assetId);
    const out = await docxToHtml(buf);
    if (loadedId !== assetId) return; // await 期间已切走
    html.value = out;
  } catch (e) {
    if (loadedId !== assetId) return;
    loadError.value = e instanceof Error ? e.message : String(e);
  }
}
watch(() => preview.current?.assetId, () => void load(), { immediate: true });
</script>

<template>
  <div class="docx-preview">
    <template v-if="html !== null">
      <div ref="bodyRef" class="docx-body md-body kit-scroll">
        <!-- 空文档提示说明文字档 -->
        <p v-if="html.trim() === ''" class="empty-hint">空文档</p>
        <!-- eslint-disable-next-line vue/no-v-html -- mammoth 自构 HTML（非源文直通） -->
        <div v-else class="docx-html" v-html="html" />
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
/* —— 排版 = styles/markdown.css .md-body 全局类（文本编辑稿口径，PreviewText 共用）——
   mammoth 输出 p/h1-h6/table/ul/img 裸标签，全局排版天然命中；本块只管图片限宽与块距 —— */
.docx-body {
  flex: 1;
  min-height: 0;
  overflow-wrap: break-word;
}
/* 内层容器吃掉 .md-body 的 flex 列 gap 影响（v-html 内容流式文档，间距由 mammoth 段落天然供给） */
.docx-html {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 截图反馈文档场景：图片限宽不爆版心（mammoth 内嵌 data URI） */
.docx-html :deep(img) {
  max-width: 100%;
  border-radius: var(--n-radius-4);
}
/* 空文档提示（说明文字档 三级） */
.empty-hint {
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
