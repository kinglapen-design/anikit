<script setup lang="ts">
// 查看方式（03_UI 布局/05_查看方式，2026-07-23 用户定：级联菜单改二级窗口模式）：
// 行 item（名称 + 下拉选择器）×5：查看范围（全局/当前目录）/ 布局方式 / 排序方式（含升降序）/
// 显示简介（尺寸已实现，其余随 7.x 接线）/ 分组方式（占位）；
// 分割线 + 开关组（自动播放动图/视频——5.3 接功能，先存状态）；
// 分割线 + 开关组（显示扩展名角标 / 显示子文件夹内容）。
import { computed, ref } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import MenuDivider from "./MenuDivider.vue";
import AppSwitch from "./AppSwitch.vue";
import AppIcon from "./AppIcon.vue";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const props = defineProps<{ anchor: { x: number; y: number }; trigger?: HTMLElement | null }>();
const emit = defineEmits<{ close: [] }>();
const groupRef = ref<HTMLElement>();

const WINDOW_W = 264;
// 原级联菜单宽 175：窗口加宽后右缘与原位置对齐（不超出窗口右缘）
const winX = computed(() => props.anchor.x - (WINDOW_W - 175));

const layouts = [
  { value: "justified", label: "自适应" },
  { value: "waterfall", label: "瀑布流" },
  { value: "grid", label: "网格" },
  { value: "list", label: "列表" },
] as const;
const sorts = ["自定义", "标题", "扩展名", "文件大小", "尺寸", "评分", "时长", "添加日期", "创建日期", "修改日期"];
const briefs = [
  { value: "size", label: "尺寸" },
  { value: "fileSize", label: "文件大小" },
  { value: "createdAt", label: "创建日期" },
  { value: "modifiedAt", label: "修改日期" },
  { value: "tags", label: "标签" },
  { value: "none", label: "无显示" },
] as const;

const layoutLabel = computed(() => layouts.find((l) => l.value === ui.layoutMode)?.label ?? "自适应");
const briefLabel = computed(() => briefs.find((b) => b.value === ui.briefMode)?.label ?? "尺寸");
const sortLabel = computed(() => ui.sortBy || "自定义");

// 下拉选择器：记录当前打开的行与锚点；选项菜单与主窗口同组（点组外一起关闭）
type PickerKind = "scope" | "layout" | "sort" | "brief" | "group";
const picker = ref<{ kind: PickerKind; x: number; y: number } | null>(null);

function openPicker(kind: PickerKind, e: MouseEvent): void {
  const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
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
      pad="var(--space-16)"
      :group="groupRef ?? null"
      :trigger="trigger ?? null"
      @close="closeAll"
    >
      <div class="rows">
        <div class="row">
          <span class="label">查看范围</span>
          <button class="picker" @click="openPicker('scope', $event)">
            <span class="val">{{ ui.useGlobalView ? "全局" : "当前目录" }}</span>
            <AppIcon name="chevron-small" :size="8" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">布局方式</span>
          <button class="picker" @click="openPicker('layout', $event)">
            <span class="val">{{ layoutLabel }}</span>
            <AppIcon name="chevron-small" :size="8" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">排序方式</span>
          <button class="picker" @click="openPicker('sort', $event)">
            <span class="val">{{ sortLabel }}</span>
            <AppIcon name="chevron-small" :size="8" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">显示简介</span>
          <button class="picker" @click="openPicker('brief', $event)">
            <span class="val">{{ briefLabel }}</span>
            <AppIcon name="chevron-small" :size="8" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">分组方式</span>
          <button class="picker" @click="openPicker('group', $event)">
            <span class="val">{{ ui.groupBy }}</span>
            <AppIcon name="chevron-small" :size="8" class="down" />
          </button>
        </div>
      </div>

      <MenuDivider />
      <div class="switch-row">
        <span class="label">自动播放动图</span>
        <AppSwitch v-model="ui.autoPlayGif" />
      </div>
      <div class="switch-row">
        <span class="label">自动播放视频</span>
        <AppSwitch v-model="ui.autoPlayVideo" />
      </div>

      <MenuDivider />
      <div class="switch-row">
        <span class="label">显示扩展名角标</span>
        <AppSwitch v-model="ui.showExtBadge" />
      </div>
      <div class="switch-row">
        <span class="label">显示子文件夹内容</span>
        <AppSwitch v-model="ui.showSubfolders" />
      </div>
    </AppMenu>

    <!-- 选项菜单（与主窗口同组） -->
    <AppMenu v-if="picker?.kind === 'scope'" :x="picker.x" :y="picker.y" :group="groupRef ?? null" :trigger="trigger ?? null" @close="picker = null">
      <MenuItem label="全局" checkable :checked="ui.useGlobalView" @select="ui.useGlobalView = true" />
      <MenuItem label="当前目录" checkable :checked="!ui.useGlobalView" @select="ui.useGlobalView = false" />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'layout'" :x="picker.x" :y="picker.y" :group="groupRef ?? null" :trigger="trigger ?? null" @close="picker = null">
      <MenuItem
        v-for="l in layouts"
        :key="l.value"
        :label="l.label"
        checkable
        :checked="ui.layoutMode === l.value"
        @select="ui.layoutMode = l.value"
      />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'sort'" :x="picker.x" :y="picker.y" :group="groupRef ?? null" :trigger="trigger ?? null" @close="picker = null">
      <MenuItem
        v-for="s in sorts"
        :key="s"
        :label="s"
        checkable
        :checked="sortLabel === s"
        @select="ui.sortBy = s === '自定义' ? '' : s"
      />
      <MenuDivider />
      <MenuItem label="升序" checkable :checked="ui.sortAsc" @select="ui.sortAsc = true" />
      <MenuItem label="降序" checkable :checked="!ui.sortAsc" @select="ui.sortAsc = false" />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'brief'" :x="picker.x" :y="picker.y" :group="groupRef ?? null" :trigger="trigger ?? null" @close="picker = null">
      <MenuItem
        v-for="b in briefs"
        :key="b.value"
        :label="b.label"
        checkable
        :checked="ui.briefMode === b.value"
        @select="ui.briefMode = b.value"
      />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'group'" :x="picker.x" :y="picker.y" :group="groupRef ?? null" :trigger="trigger ?? null" @close="picker = null">
      <MenuItem label="自定义" checkable :checked="true" @select="ui.groupBy = '自定义'" />
    </AppMenu>
  </div>
</template>

<style scoped>
/* 行 item ×5：间距 8 */
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 行：C4 底 / 1px border-1 / 圆角 8 / 内边距 4 4 4 8 */
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-8);
  padding: var(--space-4) var(--space-4) var(--space-4) var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
}
/* 下拉选择器：1px border-1 / 圆角 6 / 内边距 4 8 / 最小宽 96，值 + 下拉箭头 */
.picker {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 96px;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-2);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
  transition: background 0.12s ease;
}
.picker:hover {
  background: var(--font-icon-6);
}
.picker .val {
  flex: 1;
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.picker .down {
  color: var(--font-icon-3);
}
/* 开关行：裸文字 + 开关，间距 8（rows 复用行间距由 MenuDivider 分隔） */
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-2) 0 var(--space-2) var(--space-4);
}
</style>
