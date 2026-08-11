<script setup lang="ts">
// 查看方式（28_展开菜单 - 查看方式，1.4 2026-08-11 全量重写；旧 03_UI 布局/05 稿 picker 行模式作废）：
// 面板式展开菜单——宽 404 / padding 12 24 / 行高 53 / 行标题 16/22 700（标题档）；
// 布局方式 = 切换按钮组（自适应/瀑布流/网格/列表，AppSegmented 点亮）；
// 排序方式 = 下拉「自定义」+ 升/降切换（**占位禁用**——排序业务 3.5 接）；
// 时间码显示方式 = 帧数/时长切换（点亮，播放条/悬停胶囊/笔记同值联动）；
// 显示简介 = 下拉选内容 + 开关（ui.showBrief，与 briefMode 独立）；
// 显示扩展名（名称词干）/ 显示徽标（格式角标）/ 显示子文件夹内容 = 开关点亮；
// 显示自定义数据（占位——6.1）；自动播放动图 / 自动播放视频预览 = 开关点亮（5.3 已接）。
// 稿无"查看范围 / 分组方式"行——旧行移除（ui.useGlobalView / groupBy 字段保留不删）。
import { computed, ref } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import AppSwitch from "./AppSwitch.vue";
import AppSegmented from "./AppSegmented.vue";
import AppIcon from "./AppIcon.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const props = defineProps<{ anchor: { x: number; y: number }; trigger?: HTMLElement | null }>();
const emit = defineEmits<{ close: [] }>();
const groupRef = ref<HTMLElement>();

const WINDOW_W = 404;
// 原级联菜单宽 175：窗口加宽后右缘与原位置对齐（不超出窗口右缘）
const winX = computed(() => props.anchor.x - (WINDOW_W - 175));

const layouts = [
  { value: "justified", label: "自适应" },
  { value: "waterfall", label: "瀑布流" },
  { value: "grid", label: "网格" },
  { value: "list", label: "列表" },
] as const;
const briefs = [
  { value: "size", label: "尺寸" },
  { value: "fileSize", label: "文件大小" },
  { value: "createdAt", label: "创建日期" },
  { value: "modifiedAt", label: "修改日期" },
  { value: "tags", label: "标签" },
  { value: "none", label: "无显示" },
] as const;
const timeDisplays = [
  { value: "frames", label: "帧数" },
  { value: "timecode", label: "时长" },
] as const;
const sortOrders = [
  { value: "asc", label: "升序" },
  { value: "desc", label: "降序" },
] as const;

const briefLabel = computed(() => briefs.find((b) => b.value === ui.briefMode)?.label ?? "尺寸");
const sortOrderModel = computed({
  get: () => (ui.sortAsc ? "asc" : "desc"),
  set: (v) => {
    ui.sortAsc = v === "asc";
  },
});

// 下拉选择器（显示简介一项；排序方式占位禁用不出 picker）
const picker = ref<{ kind: "brief"; x: number; y: number } | null>(null);
const pickerTrigger = ref<HTMLElement | null>(null);

function openPicker(kind: "brief", e: MouseEvent): void {
  if (picker.value?.kind === kind) {
    picker.value = null;
    return;
  }
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  pickerTrigger.value = el;
  picker.value = { kind, x: Math.max(8, rect.right - 175), y: rect.bottom + 4 };
}
function closeAll(): void {
  picker.value = null;
  emit("close");
}
</script>

<template>
  <div ref="groupRef" style="display: contents">
    <AppMenu
      :x="winX"
      :y="anchor.y"
      :width="WINDOW_W"
      pad="var(--space-12) var(--space-24)"
      gap="0"
      :group="groupRef ?? null"
      :trigger="trigger ?? null"
      @close="closeAll"
    >
      <!-- 组 1：布局 / 排序 / 时间码 -->
      <div class="group">
        <div class="row">
          <span class="label">布局方式</span>
          <AppSegmented v-model="ui.layoutMode" :options="layouts" small style="width: 208px" />
        </div>
        <!-- 排序方式（占位禁用——排序业务 3.5 接） -->
        <div class="row disabled">
          <span class="label">排序方式</span>
          <span class="ctrl">
            <button class="picker" disabled>
              <span class="val">{{ ui.sortBy || "自定义" }}</span>
              <AppIcon name="tree-expand-arrow" :size="12" class="down" />
            </button>
            <AppSegmented v-model="sortOrderModel" :options="sortOrders" small style="width: 104px" class="seg-disabled" />
          </span>
        </div>
        <div class="row">
          <span class="label">时间码显示方式</span>
          <AppSegmented v-model="ui.timeDisplay" :options="timeDisplays" small style="width: 112px" />
        </div>
      </div>

      <div class="group-line" />

      <!-- 组 2：显示开关 -->
      <div class="group">
        <div class="row">
          <span class="label">显示简介</span>
          <span class="ctrl">
            <button class="picker" @click="openPicker('brief', $event)">
              <span class="val">{{ briefLabel }}</span>
              <AppIcon name="tree-expand-arrow" :size="12" class="down" />
            </button>
            <AppSwitch v-model="ui.showBrief" />
          </span>
        </div>
        <div class="row">
          <span class="label">显示扩展名</span>
          <AppSwitch v-model="ui.showExt" />
        </div>
        <div class="row">
          <span class="label">显示徽标</span>
          <AppSwitch v-model="ui.showBadge" />
        </div>
        <!-- 显示自定义数据（占位——6.1 自定义数据业务） -->
        <div class="row disabled">
          <span class="label">显示自定义数据</span>
          <AppSwitch :model-value="false" disabled />
        </div>
        <div class="row">
          <span class="label">显示子文件夹内容</span>
          <AppSwitch v-model="ui.showSubfolders" />
        </div>
      </div>

      <div class="group-line" />

      <!-- 组 3：自动播放 -->
      <div class="group">
        <div class="row">
          <span class="label">自动播放动图</span>
          <AppSwitch v-model="ui.autoPlayGif" />
        </div>
        <div class="row">
          <span class="label">自动播放视频预览</span>
          <AppSwitch v-model="ui.autoPlayVideo" />
        </div>
      </div>
    </AppMenu>

    <!-- 显示简介选项菜单（与主窗口同组） -->
    <AppMenu v-if="picker?.kind === 'brief'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem
        v-for="b in briefs"
        :key="b.value"
        :label="b.label"
        checkable
        :checked="ui.briefMode === b.value"
        @select="ui.briefMode = b.value; picker = null"
      />
    </AppMenu>
  </div>
</template>

<style scoped>
/* 组（28 稿）：行高 53（padding 12 0），行间无分隔；组间 1px 暗-悬停线（内容宽满） */
.group {
  display: flex;
  flex-direction: column;
}
.group-line {
  flex: none;
  height: 1px;
  background: var(--line-hover);
}
/* 行：标题 16/22 700（标题档，28 稿）左置，控件右置 */
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-8);
  padding: var(--space-12) 0;
}
.label {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--text-highlight);
  white-space: nowrap;
}
.ctrl {
  display: inline-flex;
  align-items: center;
  gap: var(--space-8);
  flex: none;
}
/* 占位禁用行（业务未接）：标题与控件统一降档不响应 */
.row.disabled {
  pointer-events: none;
}
.row.disabled .label {
  color: var(--line-active);
}
.seg-disabled {
  opacity: 0.5;
}
/* 下拉选择器（28 稿：C4 底 / 1px 暗-悬停边框 / 圆角 6）；disabled 不降 hover */
.picker {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 96px;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
  transition: background 0.12s ease;
}
.picker:hover {
  background: var(--c5);
}
.picker:disabled {
  cursor: default;
  opacity: 0.5;
}
.picker:disabled:hover {
  background: var(--c4);
}
.picker .val {
  flex: 1;
  text-align: right;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.picker .down {
  color: var(--text-label);
}
</style>
