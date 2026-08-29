<script setup lang="ts">
// 笔记弹层（播放条-笔记稿）：壳 .fb-pop 533×378 padding 0 横排——左列表（padding 12 gap 2，宽随余量）
// + 右输入框区（340 稿值，通用/二级底 + 左 1px 亮色/半透明-3 分隔线，padding 12 gap 12）。
// 打开就聚焦输入闪烁（稿 prose）→ textarea 挂载自动聚焦。
// 笔记 item = 高 24 padding 4/6 gap 6 圆角 6：帧号 数字档 亮色/二级 + 摘录 次要文字 亮色/三级
// + 关闭-小 14 亮色/四级（hover 升二级，稿恒显）；hover item 底 亮色/半透明-3；
// 选中 item 底 亮色/半透明-2 + 文字升二级（稿 prose）。
// 取消 / 确认 = 按钮族 ActionButton lg 复用（solid / primary，稿 38 高 flex 均分一致；
// 取消阴影按 token 立体感-通用——稿导出 blur 2px 为 figma 噪声，token 链统一 1px）。
// 输入区文字 = 正文档 亮色/二级（稿未写文字档，随输入框族，2026-08-29 定）。
import { onMounted, ref } from "vue";
import AppIcon from "../AppIcon.vue";
import ActionButton from "../buttons/ActionButton.vue";
import "./bars.css";

interface NoteEntry {
  frame: string; // 帧号文本（含冒号，父级格式化，如 "0123:"）
  text: string; // 摘录
}

withDefaults(
  defineProps<{ notes?: readonly NoteEntry[]; selected?: number }>(),
  {
    notes: () => [{ frame: "0123:", text: "这是第一篇笔记…" }],
    selected: -1,
  },
);
const draft = defineModel<string>("draft", { default: "" });
const emit = defineEmits<{ select: [i: number]; remove: [i: number]; confirm: []; cancel: [] }>();

const taEl = ref<HTMLTextAreaElement>();
onMounted(() => taEl.value?.focus()); // 打开就聚焦输入闪烁（稿 prose）
</script>

<template>
  <div class="fb-pop np-root">
    <div class="np-list">
      <div
        v-for="(n, i) in notes"
        :key="i"
        class="np-item"
        :class="{ on: selected === i }"
        @click="emit('select', i)"
      >
        <span class="np-frame">{{ n.frame }}</span>
        <span class="np-text">{{ n.text }}</span>
        <button class="np-close" @click.stop="emit('remove', i)">
          <AppIcon name="close-small" :size="14" />
        </button>
      </div>
    </div>
    <div class="np-editor">
      <textarea ref="taEl" v-model="draft" class="np-ta" />
      <div class="np-actions">
        <ActionButton label="取消" variant="solid" size="lg" @click="emit('cancel')" />
        <ActionButton label="确认" variant="primary" size="lg" @click="emit('confirm')" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.np-root {
  display: flex;
  width: 533px; /* 稿值 */
  height: 378px; /* 稿值 */
  padding: 0;
}

/* 左列表：宽随余量（稿 flex-grow 1），padding 12 gap 2 */
.np-list {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  padding: var(--n-space-12);
  overflow-y: auto;
}
.np-item {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  flex: none;
  height: 24px;
  padding: var(--n-space-4) var(--n-space-6);
  border-radius: var(--n-radius-6);
  cursor: pointer;
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.np-item:hover {
  background: var(--n-l-a3);
}
.np-item.on {
  background: var(--n-l-a2);
}
.np-frame {
  flex: none;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
}
.np-text {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.np-item.on .np-text {
  color: var(--n-l2);
}
.np-close {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  width: 14px;
  height: 14px;
  padding: 0;
  color: var(--n-l4);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.np-close:hover {
  color: var(--n-l2);
}

/* 右输入框区：340 稿值，通用/二级底 + 左 1px 亮色/半透明-3 分隔线 */
.np-editor {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  flex: none;
  width: 340px;
  padding: var(--n-space-12);
  background: var(--n-g2);
  border-left: 1px solid var(--n-l-a3);
}
.np-ta {
  flex: 1;
  min-height: 0;
  padding: var(--n-space-12);
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  border-radius: var(--n-radius-8);
  font-family: var(--n-font-family);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}

/* 按钮行：gap 8，两钮 flex 均分（提权压 ActionButton 自带 flex:none，速查表 §6 口径） */
.np-actions {
  display: flex;
  gap: var(--n-space-8);
}
.np-actions :deep(.act-btn) {
  flex: 1;
  min-width: 0;
}
</style>
