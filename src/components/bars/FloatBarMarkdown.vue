<script setup lang="ts">
// MarkDown 工具栏：段落（H1-H6 / 正文 / 引用）/ 列表（无序 / 有序 / 任务）/ 插入（表格 / 代码块 /
// 行内代码块 / 插入链接 / 分割线）/ 格式（加粗 / 斜体 / 删除线）四组，组间 .fb-group 左分隔线（稿 border-left 方案）；
// 第五组 = 返回预览钮（2026-09-25 走查「怎么切换到预览」：chevron-large 原样朝左 = 返回，与预览工具栏返回钮同图标同语义）。
// 图标 = 按钮族 IconButton bare 复用（18 实寸 亮色/三级，hover 升二级——稿 hover-按钮组口径）；
// 悬停 tip = IconButton title prop 走 v-tip（2026-09-25 走查补，内容容器轮同口径）。
// 只出事件（action = 语义名）；实际操作 = preview.requestMdAction → PreviewText 编辑态执行（lib/mdEdit；
// action "preview" = 退出编辑态回预览，Esc 同义）。
// 根 mousedown.prevent = 点击不抢焦点（§36 搜索栏同口径）——编辑器 textarea 选区全程保持，
// 动作作用于当前选区不丢（业务接入轮落地）。
import IconButton from "../buttons/IconButton.vue";
import "./bars.css";

const emit = defineEmits<{ action: [name: string] }>();

interface MdIcon {
  name: string;
  action: string;
  tip: string;
}
const groups: readonly MdIcon[][] = [
  // 段落
  [
    { name: "heading-1", action: "h1", tip: "标题 1" },
    { name: "heading-2", action: "h2", tip: "标题 2" },
    { name: "heading-3", action: "h3", tip: "标题 3" },
    { name: "heading-4", action: "h4", tip: "标题 4" },
    { name: "heading-5", action: "h5", tip: "标题 5" },
    { name: "heading-6", action: "h6", tip: "标题 6" },
    { name: "body-text", action: "body", tip: "正文" },
    { name: "quote", action: "quote", tip: "引用" },
  ],
  // 列表
  [
    { name: "list-unordered", action: "ul", tip: "无序列表" },
    { name: "list-ordered", action: "ol", tip: "有序列表" },
    { name: "task-list", action: "task", tip: "任务列表" },
  ],
  // 插入
  [
    { name: "table", action: "table", tip: "表格" },
    { name: "code-block", action: "code-block", tip: "代码块" },
    { name: "code-inline", action: "code-inline", tip: "行内代码" },
    { name: "insert-link", action: "link", tip: "插入链接" },
    { name: "divider", action: "divider", tip: "分割线" },
  ],
  // 格式
  [
    { name: "bold", action: "bold", tip: "加粗" },
    { name: "italic", action: "italic", tip: "斜体" },
    { name: "strikethrough", action: "strike", tip: "删除线" },
  ],
  // 视图（2026-09-25 走查补：编辑态 → 预览态可见出口）
  [{ name: "chevron-large", action: "preview", tip: "返回预览（Esc）" }],
];
</script>

<template>
  <div class="fb-mode fbm-root" @mousedown.prevent>
    <div v-for="(g, gi) in groups" :key="gi" :class="gi === 0 ? 'fbm-first' : 'fb-group'">
      <IconButton
        v-for="ic in g"
        :key="ic.action"
        :name="ic.name"
        variant="bare"
        :title="ic.tip"
        @click="emit('action', ic.action)"
      />
    </div>
  </div>
</template>

<style scoped>
.fbm-root {
  flex: none;
  width: 100%; /* 恒满壳宽（壳 = 唯一宽度动画源，FloatBar 探针实测内容宽） */
}
.fbm-first {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
</style>
