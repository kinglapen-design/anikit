<script setup lang="ts">
// MarkDown 工具栏：段落（H1-H6 / 正文 / 引用）/ 列表（无序 / 有序 / 任务）/ 插入（表格 / 代码块 /
// 行内代码块 / 插入链接 / 分割线）/ 格式（加粗 / 斜体 / 删除线）四组，组间 .fb-group 左分隔线（稿 border-left 方案）。
// 图标 = 按钮族 IconButton bare 复用（18 实寸 亮色/三级，hover 升二级——稿 hover-按钮组口径）。
// 只出事件（action = 语义名）， markdown 实际操作归业务侧接入。
import IconButton from "../buttons/IconButton.vue";
import "./bars.css";

const emit = defineEmits<{ action: [name: string] }>();

interface MdIcon {
  name: string;
  action: string;
}
const groups: readonly MdIcon[][] = [
  // 段落
  [
    { name: "heading-1", action: "h1" },
    { name: "heading-2", action: "h2" },
    { name: "heading-3", action: "h3" },
    { name: "heading-4", action: "h4" },
    { name: "heading-5", action: "h5" },
    { name: "heading-6", action: "h6" },
    { name: "body-text", action: "body" },
    { name: "quote", action: "quote" },
  ],
  // 列表
  [
    { name: "list-unordered", action: "ul" },
    { name: "list-ordered", action: "ol" },
    { name: "task-list", action: "task" },
  ],
  // 插入
  [
    { name: "table", action: "table" },
    { name: "code-block", action: "code-block" },
    { name: "code-inline", action: "code-inline" },
    { name: "insert-link", action: "link" },
    { name: "divider", action: "divider" },
  ],
  // 格式
  [
    { name: "bold", action: "bold" },
    { name: "italic", action: "italic" },
    { name: "strikethrough", action: "strike" },
  ],
];
</script>

<template>
  <div class="fb-mode fbm-root">
    <div v-for="(g, gi) in groups" :key="gi" :class="gi === 0 ? 'fbm-first' : 'fb-group'">
      <IconButton
        v-for="ic in g"
        :key="ic.action"
        :name="ic.name"
        variant="bare"
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
