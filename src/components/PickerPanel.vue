<script setup lang="ts">
// 备选区（6.2 选择模式，figma-css 02_布局/02_主窗口选择模式 + 2026-08-23/24 用户定案）：
// 选择模式下临时替换右侧详情栏（SidebarRight 内分流）。结构 = 标题栏 54（「备选区」）+
// 文件列表（C3，两模板）+ 按钮栏 62（确认主左 / 取消次右，flex 均分，带数量）。
// 模板一 relation（关系模式）：**多选累积**（2026-08-24 用户定"选一个 = 追加一条而非切换"）——
// 选中文件 item + 关系输入框×2（左 = 选中文件对发起文件、右 = 反向可空）+ 发起文件 item +
// 红删为一组，整组竖排复制（用户定"把整个 item 复制就行了"）；未选 = 占位组（双框禁用）。
// 模板二 file（文件选择模式，2026-08-24 接入用途）：名称输入框（默认源名可改，确认按行写入）+
// 链接状态徽章 + 红删；用途 = link 添加文件链接（落点文件夹）/ voice 链接音频（过滤非音频）。
import { computed, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppButton from "./AppButton.vue";
import AppIcon from "./AppIcon.vue";
import IconButton from "./IconButton.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { usePickerStore } from "../stores/picker";
import type { PickerTarget } from "../lib/relationOps";

const picker = usePickerStore();
const bodyRef = ref<HTMLElement>();

// 确认文案：带数量（PRD 多选口径：确认 (3)）
const confirmLabel = computed(() => (picker.picked.length > 0 ? `确认 (${picker.picked.length})` : "确认"));
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
        <!-- 模板一：关系模式（多选整组竖排） -->
        <template v-if="picker.template === 'relation'">
          <!-- 空态占位组（未选：引导占位 + 双框禁用 + 发起文件；无红删） -->
          <div v-if="picker.picked.length === 0" class="rel-row">
            <div class="rel-box">
              <div class="rel-item empty">
                <span class="name placeholder">点击卡片选择</span>
              </div>
              <div class="rel-inputs">
                <div class="rel-input">
                  <input class="rel-field" placeholder="关系" disabled />
                  <AppIcon name="chevron-small" :size="13" class="arrow down" />
                </div>
                <div class="rel-input">
                  <input class="rel-field" placeholder="关系" disabled />
                  <AppIcon name="chevron-small" :size="13" class="arrow" />
                </div>
              </div>
              <div v-if="picker.origin" class="rel-item">
                <img v-if="targetThumb(picker.origin)" class="mini" :src="targetThumb(picker.origin)!" alt="" />
                <AppIcon v-else-if="picker.origin.kind === 'archive'" name="archive" :size="15" class="deco" />
                <span v-else class="ext">{{ (picker.origin.ext ?? "").toUpperCase() }}</span>
                <span class="name">{{ picker.origin.name }}</span>
              </div>
            </div>
          </div>
          <!-- 已选组 × N（整组竖排复制，2026-08-24 用户定；发起文件 item 每组同显） -->
          <div v-for="(p, i) in picker.picked" :key="`${p.target.kind}|${p.target.id}`" class="rel-row">
            <div class="rel-box">
              <!-- 选中文件 item -->
              <div class="rel-item">
                <img v-if="targetThumb(p.target)" class="mini" :src="targetThumb(p.target)!" alt="" />
                <AppIcon v-else-if="p.target.kind === 'archive'" name="archive" :size="15" class="deco" />
                <span v-else class="ext">{{ (p.target.ext ?? "").toUpperCase() }}</span>
                <span class="name">{{ p.target.name }}</span>
              </div>
              <!-- 关系输入框（左 = 选中文件→发起文件 ↓；右 = 发起文件→选中文件 ↑，可空） -->
              <div class="rel-inputs">
                <div class="rel-input">
                  <input v-model="p.label" class="rel-field" placeholder="关系" maxlength="20" />
                  <AppIcon name="chevron-small" :size="13" class="arrow down" />
                </div>
                <div class="rel-input">
                  <input v-model="p.reverseLabel" class="rel-field" placeholder="关系" maxlength="20" />
                  <AppIcon name="chevron-small" :size="13" class="arrow" />
                </div>
              </div>
              <!-- 发起文件 item（每组同显，整组复制口径） -->
              <div v-if="picker.origin" class="rel-item">
                <img v-if="targetThumb(picker.origin)" class="mini" :src="targetThumb(picker.origin)!" alt="" />
                <AppIcon v-else-if="picker.origin.kind === 'archive'" name="archive" :size="15" class="deco" />
                <span v-else class="ext">{{ (picker.origin.ext ?? "").toUpperCase() }}</span>
                <span class="name">{{ picker.origin.name }}</span>
              </div>
            </div>
            <!-- 红删 = 移出该组（稿：右侧整行垂直居中） -->
            <div class="del-slot">
              <IconButton name="delete-red" deleteRed title="移出备选" @click="picker.picked.splice(i, 1)" />
            </div>
          </div>
        </template>
        <!-- 模板二：文件选择模式（多选累积；行 = 名称输入框（默认源名可改，确认按行写入）+ 链接状态徽章 + 红删；
             2026-08-24 用户定"稿的名称框本身就是输入框"；用途 = link 添加文件链接 / voice 链接音频） -->
        <template v-if="picker.template === 'file'">
          <!-- 空态占位行（稿无空态，实现定：占位文案标签色 + 输入框禁用，同 relation 空态口径） -->
          <div v-if="picker.picked.length === 0" class="file-row">
            <div class="file-item">
              <input class="name-field placeholder" placeholder="点击卡片选择" disabled />
            </div>
          </div>
          <div v-for="p in picker.picked" :key="`${p.target.kind}|${p.target.id}`" class="file-row">
            <div class="file-item">
              <input v-model="p.name" class="name-field" :placeholder="p.target.name" maxlength="100" />
              <AppIcon v-if="p.target.origin === false" name="linked-file-badge" :size="13" class="link-badge" />
            </div>
            <IconButton name="delete-red" deleteRed title="移出备选" @click="picker.togglePick(p.target)" />
          </div>
        </template>
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
/* —— 关系模式（稿：行 = 关系容器 186 + 红删 20，gap 12；组间 8） —— */
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
.rel-field:disabled {
  cursor: default;
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
/* 名称输入框（2026-08-24 用户定"稿的名称框本身就是输入框"：默认源名可改，确认按行写入） */
.name-field {
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
.name-field::placeholder {
  color: var(--text-faint);
}
.name-field:disabled {
  cursor: default;
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
