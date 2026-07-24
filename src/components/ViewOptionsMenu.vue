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

const WINDOW_W = 296;
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

// 下拉选择器：记录当前打开的行、锚点与触发按钮（trigger 用于"再点同一按钮关闭"，
// 防止"组外先关、click 又开"反复弹）；选项菜单独立于主窗口——点主窗口空白即可收起
type PickerKind = "scope" | "layout" | "sort" | "brief" | "group";
const picker = ref<{ kind: PickerKind; x: number; y: number } | null>(null);
const pickerTrigger = ref<HTMLElement | null>(null);

function openPicker(kind: PickerKind, e: MouseEvent): void {
  // 再点同一按钮 = 收起
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
            <AppIcon name="chevron-small" :size="16" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">布局方式</span>
          <button class="picker" @click="openPicker('layout', $event)">
            <span class="val">{{ layoutLabel }}</span>
            <AppIcon name="chevron-small" :size="16" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">排序方式</span>
          <button class="picker" @click="openPicker('sort', $event)">
            <span class="val">{{ sortLabel }}</span>
            <AppIcon name="chevron-small" :size="16" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">显示简介</span>
          <button class="picker" @click="openPicker('brief', $event)">
            <span class="val">{{ briefLabel }}</span>
            <AppIcon name="chevron-small" :size="16" class="down" />
          </button>
        </div>
        <div class="row">
          <span class="label">分组方式</span>
          <button class="picker" @click="openPicker('group', $event)">
            <span class="val">{{ ui.groupBy }}</span>
            <AppIcon name="chevron-small" :size="16" class="down" />
          </button>
        </div>
      </div>

      <MenuDivider />
      <div class="switch-group">
        <div class="switch-row">
          <span class="label">自动播放动图</span>
          <AppSwitch small v-model="ui.autoPlayGif" />
        </div>
        <div class="switch-row">
          <span class="label">自动播放视频</span>
          <AppSwitch small v-model="ui.autoPlayVideo" />
        </div>
      </div>

      <MenuDivider />
      <div class="switch-group last">
        <div class="switch-row">
          <span class="label">显示扩展名角标</span>
          <AppSwitch small v-model="ui.showExtBadge" />
        </div>
        <div class="switch-row">
          <span class="label">显示子文件夹内容</span>
          <AppSwitch small v-model="ui.showSubfolders" />
        </div>
      </div>
    </AppMenu>

    <!-- 选项菜单（与主窗口同组） -->
    <AppMenu v-if="picker?.kind === 'scope'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem label="全局" checkable :checked="ui.useGlobalView" @select="ui.useGlobalView = true; picker = null" />
      <MenuItem label="当前目录" checkable :checked="!ui.useGlobalView" @select="ui.useGlobalView = false; picker = null" />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'layout'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem
        v-for="l in layouts"
        :key="l.value"
        :label="l.label"
        checkable
        :checked="ui.layoutMode === l.value"
        @select="ui.layoutMode = l.value; picker = null"
      />
    </AppMenu>
    <AppMenu v-if="picker?.kind === 'sort'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem
        v-for="s in sorts"
        :key="s"
        :label="s"
        checkable
        :checked="sortLabel === s"
        @select="ui.sortBy = s === '自定义' ? '' : s; picker = null"
      />
      <MenuDivider />
      <MenuItem label="升序" checkable :checked="ui.sortAsc" @select="ui.sortAsc = true; picker = null" />
      <MenuItem label="降序" checkable :checked="!ui.sortAsc" @select="ui.sortAsc = false; picker = null" />
    </AppMenu>
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
    <AppMenu v-if="picker?.kind === 'group'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem label="自定义" checkable :checked="true" @select="ui.groupBy = '自定义'; picker = null" />
    </AppMenu>
  </div>
</template>

<style scoped>
/* 行 item 组：间距 8（二级设置窗口 padding 16，2026-07-24 用户定规则：
   标准菜单式 8 / 工具栏弹出的二级设置窗口 16 / UI 中间弹出的模态·非模态框 32） */
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
/* 行：裸（无底色无边框，2026-07-24 用户定） */
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-8);
  padding: 0 var(--space-4);
}
.label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
}
/* 下拉选择器：C4 底 / 1px border-1 / 圆角 6（2026-07-24 用户定）；hover C5 */
.picker {
  display: flex;
  align-items: center;
  gap: var(--space-4);
  min-width: 96px;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  color: var(--font-icon-2);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  flex: none;
  transition: background 0.12s ease;
}
.picker:hover {
  background: var(--c5);
}
.picker .val {
  flex: 1;
  text-align: right;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.picker .down {
  color: var(--font-icon-3);
  transition: color 0.12s ease;
}
.picker:hover .down {
  color: var(--font-icon-1);
}
/* 开关组：间距 8；分割线两侧间距一致（divider 上下 margin 4 + 自身 padding 2 + 容器 gap 2） */
.switch-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
:deep(.menu-divider) {
  margin: var(--space-4) 0;
}
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 var(--space-4);
}
</style>
