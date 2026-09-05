<script setup lang="ts">
// 插件列表面板（稿 = docs/UI 重构/Design Token/布局/主窗口/工具栏展开窗口/插件.md，400×450 非模态浮层）：
// 壳 = AppMenu 机制（fixed + 视口钳制 + 点外/Esc 关 + trigger 触发钮豁免 + 弹出动画 menu-pop），
// 视觉覆盖为稿口径：g3 底 + 1px g5 真 border（速查 §1 弹窗口径）+ 大阴影 + 圆角 8；
// 打开/关闭状态在 lib/pluginPanel（三入口共用：菜单栏「插件」钮 / 工具栏 plugin 钮 / 预览标题栏 plugin 钮）。
// 顶部分类分段 64（padding 12 + border-bottom g5）：SegmentedControl text 型 tone=g2（稿容器 376×40 通用/二级），
// 五档 桥接（默认选中）/格式/属性/效率/其他——会话级状态，每次打开重置回桥接；
// 列表区 flex 1（padding 12 gap 2 + ScrollThumb 悬浮滚轮，速查 §2）：
//   item 376×32 圆角 6 = 抓手 14 盒（四级）+ 图标 24 盒 + 名称 16/22 Bold 二级 + 更多钮 18 盒（more-large 四级，
//   IconButton bare）；item hover 底 = 菜单项口径 --n-l-a3（稿未画 hover，就近套 §3——待用户走查）；
//   空分类 = 空态「暂无插件」（13/18 四级居中）；
// 抓手拖拽排序（稿 prose「抓手可拖拽排序，拖拽时要有占位动画」）= 本地 pointer 追踪 + 让位动画
// （LibraryView blockDrag 范式：拖动项跟随指针无过渡、被跨越项反向让位 0.18s、松手归位帧禁过渡防"弹回去"），
// 松手写 ui.pluginOrder 持久化（渲染序 = pluginOrder 过滤已知 id + 数据源补尾）；
// 更多菜单 = openContextMenu「打开 / 安装或卸载（按 Animate 安装态动态文案）/ 配置快捷键（业务未立项占位禁用）」；
// 单击 item（非禁用）= 打开对应详情子窗口（invoke open_plugin_window）+ 关浮层；
// 底部「开发者选项」（padding 12 24 + border-top g5；dev-options 18 盒 14 图标 + 13/18 二级）点亮不接业务（菜单栏既有惯例）；
// 「批量生成缩略图」业务未立项 = 整项禁用（名称/图标降四级、不响应点击、更多钮同禁，2026-09-06 用户定）。
// 图标（2026-09-06 用户直投源图入库，命名对照十一批）：grip（抓手，四级 currentColor）/ dev-options /
// animate-plugin（品牌彩色原值）/ batch-thumbnails（玻璃质感位图原值）。
import { computed, onBeforeUnmount, reactive, ref } from "vue";
import AppMenu from "../AppMenu.vue";
import AppIcon from "../AppIcon.vue";
import SegmentedControl from "../buttons/SegmentedControl.vue";
import IconButton from "../buttons/IconButton.vue";
import ScrollThumb from "../ScrollThumb.vue";
import { openContextMenu } from "../../lib/contextMenus";
import {
  PLUGINS,
  PLUGIN_CATEGORIES,
  getCepInstallStatus,
  installCepPlugin,
  openPluginWindow,
  uninstallCepPlugin,
  type PluginMeta,
} from "../../lib/plugins";
import { useUiStore } from "../../stores/ui";

withDefaults(defineProps<{ x: number; y: number; trigger?: HTMLElement | null }>(), { trigger: null });
const emit = defineEmits<{ close: [] }>();
const ui = useUiStore();

// 分类分段（稿：五档，桥接默认选中）
const category = ref<string>(PLUGIN_CATEGORIES[0]);
const catOptions = PLUGIN_CATEGORIES.map((c) => ({ value: c, label: c }));

// 渲染序 = ui.pluginOrder 过滤已知 id 后按数据源（PLUGINS 声明序）补尾
const orderedPlugins = computed<PluginMeta[]>(() => {
  const byId = new Map(PLUGINS.map((p) => [p.id, p]));
  const saved = ui.pluginOrder.filter((id) => byId.has(id));
  const out = saved.map((id) => byId.get(id)!);
  for (const p of PLUGINS) if (!saved.includes(p.id)) out.push(p);
  return out;
});
const visiblePlugins = computed(() => orderedPlugins.value.filter((p) => p.category === category.value));

const listRef = ref<HTMLElement>();

// —— 抓手拖拽排序（blockDrag / libDrag 范式：item 定高 32 + gap 2 = 步长 34 算术让位）——
const ITEM_STEP = 34;
const drag = reactive({ id: "", from: -1, to: -1, startY: 0, dy: 0 });
const settling = ref(false); // 松手归位帧：禁 transform 过渡
let dragMoved = false; // 超阈值视为拖动
let suppressClick = false; // 有效拖拽松手后抑制紧随 click（误打开窗口）

function onGripDown(p: PluginMeta, i: number, e: PointerEvent): void {
  if (e.button !== 0 || p.disabled) return;
  Object.assign(drag, { id: p.id, from: i, to: i, startY: e.clientY, dy: 0 });
  dragMoved = false;
  window.addEventListener("pointermove", onDragMove);
  window.addEventListener("pointerup", onDragEnd, { once: true });
  window.addEventListener("blur", onDragCancel, { once: true });
  document.addEventListener("mouseleave", onDragCancel, { once: true });
}

function onDragMove(e: PointerEvent): void {
  if (!drag.id) return;
  if ((e.buttons & 1) === 0) {
    onDragCancel(); // 僵尸自清：pointerup 丢失（窗外松手等）
    return;
  }
  drag.dy = e.clientY - drag.startY;
  if (Math.abs(drag.dy) > 3) dragMoved = true;
  const n = visiblePlugins.value.length;
  drag.to = Math.max(0, Math.min(n - 1, drag.from + Math.round(drag.dy / ITEM_STEP)));
}

function onDragEnd(): void {
  offDrag();
  const { from, to } = drag;
  const moved = dragMoved;
  drag.id = "";
  // 归位帧禁过渡（itemStyle settling 分支）——任何松手都置位：拖回原位也有残留 dy 偏移
  settling.value = true;
  setTimeout(() => (settling.value = false), 50);
  if (!moved || from < 0 || to === from) return;
  // 可见子序重排 → 回写全局序（其他分类 id 保持原位：按可见集原位替换）
  const vis = visiblePlugins.value.map((p) => p.id);
  const [m] = vis.splice(from, 1);
  vis.splice(to, 0, m);
  const visSet = new Set(vis);
  let j = 0;
  ui.pluginOrder = orderedPlugins.value.map((p) => (visSet.has(p.id) ? vis[j++] : p.id));
  suppressClick = true;
  setTimeout(() => (suppressClick = false), 0);
}

function onDragCancel(): void {
  offDrag();
  drag.id = "";
}

function offDrag(): void {
  window.removeEventListener("pointermove", onDragMove);
  window.removeEventListener("pointerup", onDragEnd);
  window.removeEventListener("blur", onDragCancel);
  document.removeEventListener("mouseleave", onDragCancel);
}

// 拖拽中面板被关（Esc / 点外）= 取消不提交（组件卸载后 window 监听不残留、静默提交不发生）
onBeforeUnmount(() => {
  onDragCancel();
});

// 让位位移：拖动项跟随指针（无过渡、置顶）；被跨越项反向让出一步（transition 在 .plugin-item 根样式）。
// 松手归位帧（settling，2026-09-06 三轮修「松手回弹」）：drag.id 清空与 pluginOrder 重排同帧发生——
// inline transform 移除若走根样式 0.18s transition，残留 dy/让位偏移会从新位置滑回（= 用户看到的回弹）；
// 归位帧恒禁过渡，DOM 重排 + transform 移除同帧瞬时完成，落位干脆（settling 判定必须先于 drag.id 判空）。
function itemStyle(i: number): string {
  if (settling.value) return "transition: none;";
  if (!drag.id) return "";
  if (i === drag.from) {
    return `transform: translateY(${drag.dy}px); transition: none; position: relative; z-index: 2;`;
  }
  const { from, to } = drag;
  if (from < to && i > from && i <= to) return `transform: translateY(${-ITEM_STEP}px)`;
  if (to < from && i >= to && i < from) return `transform: translateY(${ITEM_STEP}px)`;
  return "";
}

// 单击 item（非禁用）= 打开详情子窗口 + 关浮层；抓手上的点击不打开（拖拽起点位）
function onItemClick(p: PluginMeta, e: MouseEvent): void {
  if (p.disabled) return;
  if (suppressClick) return;
  if ((e.target as HTMLElement).closest(".grip")) return;
  emit("close");
  void openPluginWindow(p.id);
}

// 更多菜单（稿 prose：打开 / 安装或卸载 / 配置快捷键）——配置快捷键业务未立项占位禁用；
// 安装/卸载按 Animate 安装态动态文案（开菜单时实时拉状态）
async function onMore(p: PluginMeta, e: MouseEvent): Promise<void> {
  if (p.disabled) return;
  if (p.id !== "animate") return;
  const st = await getCepInstallStatus();
  const installed = st?.installed ?? false;
  openContextMenu(e, [
    {
      label: "打开",
      action: () => {
        emit("close");
        void openPluginWindow(p.id);
      },
    },
    { label: installed ? "卸载" : "安装", action: () => void (installed ? uninstallCepPlugin() : installCepPlugin()) },
    { label: "配置快捷键", disabled: true },
  ]);
}
</script>

<template>
  <AppMenu :x="x" :y="y" :width="400" pad="0" class="plugin-panel" :trigger="trigger" @close="emit('close')">
    <!-- 顶部分类分段（稿：64 高 / padding 12 / border-bottom g5；容器 376×40 通用/二级底） -->
    <div class="panel-head">
      <SegmentedControl v-model="category" :options="catOptions" variant="text" tone="g2" />
    </div>
    <!-- 插件列表区（稿：344 高 flex 1 / padding 12 / gap 2；ScrollThumb 悬浮滚轮贴右缘） -->
    <div class="list-wrap">
      <div ref="listRef" class="list-scroll kit-scroll">
        <div
          v-for="(p, i) in visiblePlugins"
          :key="p.id"
          class="plugin-item"
          :class="{ disabled: p.disabled }"
          :style="itemStyle(i)"
          @click="onItemClick(p, $event)"
        >
          <span class="item-lead">
            <!-- 抓手（拖拽排序起点）：grip 14 四级色，热区 ::after 外扩 -->
            <span class="grip" @pointerdown="onGripDown(p, i, $event)"><AppIcon name="grip" :size="14" /></span>
            <span class="p-icon"><AppIcon :name="p.icon" :size="22" /></span>
            <span class="p-name">{{ p.name }}</span>
          </span>
          <IconButton name="more-large" variant="bare" class="more-btn" @click.stop="onMore(p, $event)" />
        </div>
        <!-- 空分类档（稿未画空态，任务定案：13/18 四级居中） -->
        <div v-if="visiblePlugins.length === 0" class="list-empty">暂无插件</div>
      </div>
      <ScrollThumb :body="listRef" />
    </div>
    <!-- 开发者选项（稿：42 高 / padding 12 24 / gap 8 / border-top g5）：点亮不接业务 -->
    <div class="panel-foot">
      <span class="dev-icon"><AppIcon name="dev-options" :size="14" /></span>
      <span class="dev-label">开发者选项</span>
    </div>
  </AppMenu>
</template>

<style scoped>
/* 浮层壳覆盖为稿口径（ specificity 提权压 AppMenu 自带 .app-menu，速查 §6 口径）：
   400×450（宽走 AppMenu width prop 内联）+ g3 底（沿 AppMenu）+ 1px g5 真 border（§1 弹窗口径，
   悬浮滚轮贴边不压框）+ 大阴影 + 圆角 8（沿 AppMenu）；AppMenu 默认 inset 描边 + 小阴影被本块覆盖 */
.plugin-panel.app-menu {
  height: 450px;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  box-shadow: var(--n-shadow-lg);
}
/* 顶部分类分段（稿：64 高含 1px 底线 / padding 12） */
.panel-head {
  flex: none;
  height: 64px;
  box-sizing: border-box;
  padding: var(--n-space-12);
  border-bottom: 1px solid var(--n-g5);
}
/* 列表区包裹层：relative = ScrollThumb 锚点（§2 悬浮滚轮结构） */
.list-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.list-scroll {
  height: 100%;
  box-sizing: border-box;
  padding: var(--n-space-12);
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
/* 插件 item（稿：376×32 圆角 6 / padding 4 6 / gap 6 space-between）；
   transform 过渡 = 拖拽让位动画（blockDrag 同款 0.18s——拖动项 inline 禁过渡跟随指针） */
.plugin-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  height: 32px;
  padding: var(--n-space-4) var(--n-space-6);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  flex: none;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    background var(--n-dur-150) var(--n-ease-in-out);
}
/* hover 底 = 菜单项口径 亮色/半透明-3（稿未画 hover，就近套 §3——待用户走查） */
.plugin-item:hover {
  background: var(--n-l-a3);
}
/* 禁用项（批量生成缩略图，业务未立项）：名称/图标降四级、不响应点击（更多钮同禁）；
   位图图标（batch-thumbnails 玻璃质感图）不吃 color——降档只作用于名称与 currentColor 图标，
   位图禁用观感待用户走查 */
.plugin-item.disabled {
  pointer-events: none;
  cursor: default;
}
.plugin-item.disabled .p-name,
.plugin-item.disabled .p-icon {
  color: var(--n-l4);
}
/* 图标+名称组（稿 gap 12） */
.item-lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  flex: 1;
  min-width: 0;
}
/* 抓手 14 盒（grip 图标，四级色随 hover 不提亮——稿恒四级；cursor grab 表拖拽起点；热区外扩 §14 小钮口径） */
.grip {
  position: relative;
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--n-l4);
  cursor: grab;
}
.grip::after {
  content: "";
  position: absolute;
  inset: -4px;
}
/* 插件图标 24 盒（稿 svg 22 实寸） */
.p-icon {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
/* 名称 = 按钮文字-大 16/22 Bold 亮色/二级 */
.p-name {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 更多钮（稿：更多-大 18 盒 亮色/四级；IconButton bare hover 升二级 = 组件自带口径）——
   提权压 IconButton 自带 .ic-btn.bare 色档（§6 specificity 口径） */
.plugin-item .ic-btn.more-btn {
  color: var(--n-l4);
}
/* 空分类空态（稿未画，任务定案）：13/18 四级居中 */
.list-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  user-select: none;
}
/* 底部开发者选项（稿：42 高含 1px 顶线 / padding 12 24 / gap 8；18 图标 + 13/18 二级）点亮不接业务 */
.panel-foot {
  flex: none;
  height: 42px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding: var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
}
.dev-icon {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--n-l3);
}
.dev-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
</style>
