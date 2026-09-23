<script setup lang="ts">
// 查看方式（换代 2026-09-20，稿 = Design Token/布局/工具栏展开窗口/查看方式 01~06 六稿；旧 28 展开菜单稿口径作废）：
// 面板 = AppMenu 容器口径（404 宽 / padding 24 / g3 底 / 1px g5 / 圆角 8），阴影覆盖为大阴影（01 稿）；
// 行标题 16/22 按钮文字-大 亮色/三级（稿值 #808080；旧轮「标题用常态色」是旧稿口径，随新稿作废）；
// 布局方式 = 切换按钮组（自适应/瀑布流/网格/列表，SegmentedControl sm 268）；
// 排序方式 = 下拉 8 项（86×28 g6 底稿值）+ 升/降切换 sm 134（3.5 点亮：自定义 = 存储序且升降禁用 30%）；
// 时间码显示方式 = 帧数/时长切换 sm 134（点亮，播放条/悬停胶囊/笔记同值联动）；
// 显示简介 = 下拉选内容 + 开关（ui.showBrief，与 briefMode 独立）；
// 显示扩展名（名称词干）/ 显示格式角标（showBadge）/ 显示链接状态徽章（showLinkBadge）/
// 显示子文件夹内容 = 开关点亮；自动播放动图 / 自动播放视频预览 = 开关点亮（5.3 已接）。
// 开关组文案（2026-09-23 用户定）：「显示格式角标」「显示链接状态徽章」——05 稿文字图层真文案恢复
// （换代时误取容器图层名「显示徽标 / 显示自定义数据」；链接徽章行同步点亮绑 showLinkBadge——
// 业务 9-6 视图菜单轮已接，稿上行开关即开态，原 6.1「自定义数据」占位行废除）。
// 稿无"查看范围 / 分组方式"行——旧行移除（ui.useGlobalView / groupBy 字段保留不删）。
import { computed, ref } from "vue";
import AppMenu from "./AppMenu.vue";
import MenuItem from "./MenuItem.vue";
import SwitchButton from "./buttons/SwitchButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import AppIcon from "./AppIcon.vue";
import { useUiStore } from "../stores/ui";
import { isDisplaySorted, type SortKey } from "../lib/sortPlacements";

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
// 显示简介档（2026-08-19 接入）：「无显示」档删除（与右侧开关语义重复，用户定）；
// 标签档占位禁用——卡片数据无 tags（PlacementView 未带出），待后端数据通道随 3.2 二轮补
const briefs = [
  { value: "size", label: "尺寸", disabled: false },
  { value: "fileSize", label: "文件大小", disabled: false },
  { value: "createdAt", label: "创建日期", disabled: false },
  { value: "modifiedAt", label: "修改日期", disabled: false },
  { value: "tags", label: "标签", disabled: true },
] as const;
const timeDisplays = [
  { value: "frames", label: "帧数" },
  { value: "timecode", label: "时长" },
] as const;
// 排序键 8 项（3.5 用户定：含时长；04 预览 06 节口径补"自定义"首项）
const sortKeys: readonly { value: SortKey; label: string }[] = [
  { value: "", label: "自定义" },
  { value: "name", label: "名称" },
  { value: "size", label: "文件大小" },
  { value: "duration", label: "时长" },
  { value: "ext", label: "扩展名" },
  { value: "createdAt", label: "创建时间" },
  { value: "modifiedAt", label: "修改时间" },
  { value: "importedAt", label: "导入时间" },
];
const sortOrders = [
  { value: "asc", label: "升序" },
  { value: "desc", label: "降序" },
] as const;

const briefLabel = computed(() => briefs.find((b) => b.value === ui.briefMode)?.label ?? "尺寸");
const sortLabel = computed(() => sortKeys.find((s) => s.value === ui.sortBy)?.label ?? "自定义");
// 自定义 = 存储序，无升降语义——升降切换禁用（3.5 定；禁用态 30% 透明度，2026-09-20 控件族口径）
const sorted = computed(() => isDisplaySorted(ui.sortBy));
const sortOrderModel = computed({
  get: () => (ui.sortAsc ? "asc" : "desc"),
  set: (v) => {
    ui.sortAsc = v === "asc";
  },
});

// 下拉选择器（显示简介 / 排序方式两项）
const picker = ref<{ kind: "brief" | "sort"; x: number; y: number } | null>(null);
const pickerTrigger = ref<HTMLElement | null>(null);

function openPicker(kind: "brief" | "sort", e: MouseEvent): void {
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
      class="vom-panel"
      pad="var(--n-space-24)"
      gap="0"
      :group="groupRef ?? null"
      :trigger="trigger ?? null"
      @close="closeAll"
    >
      <!-- 组 1：布局 40 高（padding-bottom 12）；排序/时间码 52 高（padding 12 0 + 顶 1px g5 线）——02/03/04 稿 -->
      <div class="row lead">
        <span class="label">布局方式</span>
        <SegmentedControl v-model="ui.layoutMode" :options="layouts" variant="text" tone="g2" size="sm" style="width: 268px" />
      </div>
      <!-- 排序方式（3.5 点亮）：下拉 8 项 + 升/降切换（自定义 = 存储序，升降禁用 30%） -->
      <div class="row tall">
        <span class="label">排序方式</span>
        <span class="ctrl">
          <button class="picker" @click="openPicker('sort', $event)">
            <span class="val">{{ sortLabel }}</span>
            <AppIcon name="tree-expand-arrow" :size="14" class="down" />
          </button>
          <SegmentedControl v-model="sortOrderModel" :options="sortOrders" variant="text" tone="g2" size="sm" :disabled="!sorted" style="width: 134px" />
        </span>
      </div>
      <div class="row tall">
        <span class="label">时间码显示方式</span>
        <SegmentedControl v-model="ui.timeDisplay" :options="timeDisplays" variant="text" tone="g2" size="sm" style="width: 134px" />
      </div>

      <!-- 组 2：显示开关（05 稿：组容器 padding 12 0 / gap 12 / 行高 22，组顶 1px g5 线） -->
      <div class="group">
        <div class="row">
          <span class="label">显示简介</span>
          <span class="ctrl">
            <button class="picker" @click="openPicker('brief', $event)">
              <span class="val">{{ briefLabel }}</span>
              <AppIcon name="tree-expand-arrow" :size="14" class="down" />
            </button>
            <SwitchButton v-model="ui.showBrief" />
          </span>
        </div>
        <div class="row">
          <span class="label">显示扩展名</span>
          <SwitchButton v-model="ui.showExt" />
        </div>
        <div class="row">
          <span class="label">显示格式角标</span>
          <SwitchButton v-model="ui.showBadge" />
        </div>
        <div class="row">
          <span class="label">显示链接状态徽章</span>
          <SwitchButton v-model="ui.showLinkBadge" />
        </div>
        <div class="row">
          <span class="label">显示子文件夹内容</span>
          <SwitchButton v-model="ui.showSubfolders" />
        </div>
      </div>

      <!-- 组 3：自动播放（06 稿：组容器 padding 12 0 / gap 8 / 行高 28，组顶 1px g5 线） -->
      <div class="group play">
        <div class="row">
          <span class="label">自动播放动图</span>
          <SwitchButton v-model="ui.autoPlayGif" />
        </div>
        <div class="row">
          <span class="label">自动播放视频预览</span>
          <SwitchButton v-model="ui.autoPlayVideo" />
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
        :disabled="b.disabled"
        @select="ui.briefMode = b.value; picker = null"
      />
    </AppMenu>

    <!-- 排序方式选项菜单（8 项，当前键对勾） -->
    <AppMenu v-if="picker?.kind === 'sort'" :x="picker.x" :y="picker.y" :trigger="pickerTrigger" @close="picker = null">
      <MenuItem
        v-for="s in sortKeys"
        :key="s.value || 'custom'"
        :label="s.label"
        checkable
        :checked="ui.sortBy === s.value"
        @select="ui.sortBy = s.value; picker = null"
      />
    </AppMenu>
  </div>
</template>

<style scoped>
/* 行结构（查看方式 01~06 稿，2026-09-20 换代）：布局行 40 高（padding-bottom 12）；
   排序/时间码行 52 高（padding 12 0 + 组顶 1px g5 线）；开关组 = 组容器（padding 12 0 / gap 12 / 行高 22），
   自动播放组（gap 8 / 行高 28），组顶均 1px g5 线（稿为 border-top 系，非旧稿 border-bottom） */
.row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
}
.row.lead {
  height: 40px;
  padding-bottom: var(--n-space-12);
  box-sizing: border-box;
}
.row.tall {
  height: 52px;
  padding: var(--n-space-12) 0;
  box-sizing: border-box;
  border-top: 1px solid var(--n-g5);
}
.group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-top: 1px solid var(--n-g5);
}
.group .row {
  height: 22px;
}
.group.play {
  gap: var(--n-space-8);
}
.group.play .row {
  height: 28px;
}
/* 行标题：16/22 按钮文字-大 亮色/三级（稿值 #808080） */
.label {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  white-space: nowrap;
}
.ctrl {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-12);
  flex: none;
}
/* 下拉选择器（03 稿：86×28 / 通用/六级底 / 1px 亮色/半透明-3 边 / 圆角 6 / 次要文字-加粗 12/16 亮色/二级 / 箭头 14 三级） */
.picker {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  width: 86px;
  height: 28px;
  padding: var(--n-space-4) var(--n-space-8);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  border: 1px solid var(--n-l-a3);
  background: var(--n-g6);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  flex: none;
  transition: border-color var(--n-dur-150) var(--n-ease-in-out);
}
.picker:hover {
  border-color: var(--n-l-a2); /* hover 稿未给，就近套亮色/半透明-2 描边升档（待实机走查） */
}
.picker .val {
  flex: 1;
  min-width: 0;
  text-align: left; /* 下拉框文字一律左对齐（通用规则，2026-08-11 用户定） */
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.picker .down {
  color: var(--n-l3);
}
/* 面板阴影 = 大阴影（01 稿；AppMenu 容器默认小阴影，面板形态覆盖） */
.vom-panel :deep(.app-menu) {
  box-shadow: var(--n-shadow-lg);
}
</style>
