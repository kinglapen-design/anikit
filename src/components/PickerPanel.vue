<script setup lang="ts">
// 备选区（6.2 选择模式一轮，figma-css 02_布局/02_主窗口选择模式 + 2026-08-23 用户贴稿）：
// 选择模式下临时替换右侧详情栏（SidebarRight 内分流）。结构 = 标题栏 54（「备选区」）+
// 文件列表（C3，两模板）+ 按钮栏 62（确认主左 / 取消次右，flex 均分）。
// 模板一 relation（关系模式，本轮用途）：选中文件 item + 关系输入框×2（左 = 选中文件对发起文件
// 的称谓、右 = 反向称谓，均手填）+ 发起文件 item（固定不可删），红删 = 移出备选；
// 模板二 file（文件选择模式，框架就位、用途入口随后续轮）：名称 + 链接状态徽章 + 红删，多选累积。
import { computed } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppButton from "./AppButton.vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { usePickerStore } from "../stores/picker";
import type { PickerTarget } from "../lib/relationOps";
import { ref } from "vue";

const picker = usePickerStore();
const bodyRef = ref<HTMLElement>();

// 确认文案：多选带数量（PRD：确认 (3)）；单选恒「确认」
const confirmLabel = computed(() =>
  picker.multiple && picker.picked.length > 0 ? `确认 (${picker.picked.length})` : "确认",
);
// item 左标小图（15px）：缩略图 / 档案封面路径 → 协议 URL；无图回退由模板分（扩展名文字 / archive 图标）
function targetThumb(t: PickerTarget): string | null {
  return t.thumb ? convertFileSrc(t.thumb) : null;
}
</script>

<template>
  <div class="picker-panel">
    <!-- 标题栏 54（稿：C3 / padding 12 16 / 下边框 暗-悬停；标题档 16/22 700 常态色） -->
    <header class="pick-title">备选区</header>
    <!-- 文件列表（C3 通栏；悬浮滚轮 ScrollThumb 同右栏口径） -->
    <div class="pick-scroll-wrap">
      <div ref="bodyRef" class="pick-body kit-scroll">
        <!-- 模板一：关系模式（选中文件 + 关系输入框×2 + 发起文件 + 红删） -->
        <div v-if="picker.template === 'relation'" class="rel-row">
          <div class="rel-box">
            <!-- 选中文件 item（36 高 C4；未选 = 占位提示） -->
            <div class="rel-item" :class="{ empty: !picker.picked[0] }">
              <template v-if="picker.picked[0]">
                <img v-if="targetThumb(picker.picked[0])" class="mini" :src="targetThumb(picker.picked[0])!" alt="" />
                <AppIcon v-else-if="picker.picked[0].kind === 'archive'" name="archive" :size="15" class="deco" />
                <span v-else class="ext">{{ (picker.picked[0].ext ?? "").toUpperCase() }}</span>
                <span class="name">{{ picker.picked[0].name }}</span>
              </template>
              <span v-else class="name placeholder">点击卡片选择</span>
            </div>
            <!-- 关系输入框（C2 底 36 高：左 = 选中文件→发起文件 ↓，右 = 发起文件→选中文件 ↑） -->
            <div class="rel-inputs">
              <div class="rel-input">
                <input v-model="picker.label" class="rel-field" placeholder="关系" maxlength="20" />
                <AppIcon name="chevron-small" :size="13" class="arrow down" />
              </div>
              <div class="rel-input">
                <input v-model="picker.reverseLabel" class="rel-field" placeholder="关系" maxlength="20" />
                <AppIcon name="chevron-small" :size="13" class="arrow" />
              </div>
            </div>
            <!-- 发起文件 item（固定，不可删） -->
            <div v-if="picker.origin" class="rel-item">
              <img v-if="targetThumb(picker.origin)" class="mini" :src="targetThumb(picker.origin)!" alt="" />
              <AppIcon v-else-if="picker.origin.kind === 'archive'" name="archive" :size="15" class="deco" />
              <span v-else class="ext">{{ (picker.origin.ext ?? "").toUpperCase() }}</span>
              <span class="name">{{ picker.origin.name }}</span>
            </div>
          </div>
          <!-- 红色删除 = 移出备选（清选中，两框保留；稿：右侧整行垂直居中） -->
          <div class="del-slot">
            <IconButton v-if="picker.picked.length > 0" name="delete-red" deleteRed title="移出备选" @click="picker.picked = []" />
          </div>
        </div>
        <!-- 模板二：文件选择模式（名称 + 链接状态徽章 + 红删，多选累积） -->
        <div v-for="p in picker.template === 'file' ? picker.picked : []" :key="`${p.kind}|${p.id}`" class="file-row">
          <div class="file-item">
            <span class="name">{{ p.name }}</span>
            <AppIcon v-if="p.origin === false" name="linked-file-badge" :size="13" class="link-badge" />
          </div>
          <IconButton name="delete-red" deleteRed title="移出备选" @click="picker.togglePick(p)" />
        </div>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <!-- 按钮栏 62（稿：padding 8 16 16 / gap 12 / C3；确认主左品牌色、取消次右 C4，flex 均分） -->
    <footer class="pick-actions">
      <AppButton :label="confirmLabel" variant="primary" :disabled="!picker.canConfirm" @click="picker.confirm()" />
      <AppButton label="取消" @click="picker.exit()" />
    </footer>
  </div>
</template>

<style scoped>
.picker-panel {
  display: flex;
  flex-direction: column;
  width: var(--sb-w); /* 跟随右栏宽（SidebarRight 定宽子块口径） */
  flex: 1;
  min-height: 0;
  border-left: 1px solid var(--line-hover);
  background: var(--c3);
}
/* 标题栏 54（稿：C3 / 下边框 暗-悬停 / padding 12 16；标题档常态色） */
.pick-title {
  flex: none;
  display: flex;
  align-items: center;
  gap: var(--space-4);
  height: 54px;
  box-sizing: border-box;
  padding: var(--space-12) var(--space-16);
  border-bottom: 1px solid var(--line-hover);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-normal);
}
/* 文件列表区（C3，可滚；包裹层 relative 供悬浮滚轮贴右） */
.pick-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.pick-body {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-12) var(--space-16);
  overflow-y: auto;
  overflow-x: hidden;
}
/* —— 关系模式（稿：行 = 关系容器 186 + 红删 20，gap 12） —— */
.rel-row {
  display: flex;
  align-items: flex-start;
  gap: var(--space-12);
}
.rel-box {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
/* 选中 / 发起文件 item（稿：36 高 C4 / padding 8 12 / gap 6 / 圆角 6 / 1px 暗-悬停） */
.rel-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.deco {
  flex: none;
  color: var(--text-normal);
}
/* item 左标 15px 小图（文件缩略图 / 档案封面）与扩展名回退文字（标签色） */
.mini {
  flex: none;
  width: 15px;
  height: 15px;
  border-radius: var(--radius-4);
  object-fit: cover;
}
.ext {
  flex: none;
  max-width: 40px;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 未选占位（稿无空态，占位文案标签色——2026-08-23 实现定） */
.rel-item.empty {
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.placeholder {
  color: var(--text-label);
}
/* 关系输入框行（稿：两框均分，gap 8） */
.rel-inputs {
  display: flex;
  gap: var(--space-8);
}
/* 关系输入框（稿：36 高 C2 / padding 8 8 8 12 / space-between / 圆角 6 / 1px 暗-悬停） */
.rel-input {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  height: 36px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.rel-field {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  padding: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--text-normal);
}
.rel-field::placeholder {
  color: var(--text-faint);
}
/* 箭头 13px 标签档（稿 svg 文字/标签）：chevron-small 源图形朝上（5.1 踩坑实证）——
   左框 ↓ = 旋转 180，右框 ↑ = 原样 */
.arrow {
  flex: none;
  color: var(--text-label);
}
.arrow.down {
  transform: rotate(180deg);
}
/* 红删槽位（稿：宽 20，高同容器，垂直居中） */
.del-slot {
  flex: none;
  width: 20px;
  align-self: stretch;
  display: flex;
  align-items: center;
}
/* —— 文件选择模式（稿：item 行 32 高 = 名称框 186 + 红删 20，gap 12） —— */
.file-row {
  display: flex;
  align-items: center;
  gap: var(--space-12);
  height: 32px;
}
/* 名称 + 链接状态徽章（稿：C4 / padding 6 8 / gap 6 / 圆角 6 / 1px 暗-悬停） */
.file-item {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: 1;
  min-width: 0;
  height: 32px;
  box-sizing: border-box;
  padding: var(--space-6) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
.link-badge {
  flex: none;
  color: var(--text-label);
}
/* 按钮栏（稿：62 高 / padding 8 16 16 / gap 12 / C3；两钮 flex 均分） */
.pick-actions {
  flex: none;
  display: flex;
  justify-content: center;
  align-items: center;
  gap: var(--space-12);
  height: 62px;
  box-sizing: border-box;
  padding: var(--space-8) var(--space-16) var(--space-16);
}
.pick-actions :deep(.app-btn) {
  flex: 1;
}
</style>
