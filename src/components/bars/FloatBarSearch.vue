<script setup lang="ts">
// 搜索栏 / 搜索栏展开成筛选（同一组件两态：点筛选图标横向展开筛选按钮组，收起钮折叠回——
// 搜索图标与搜索区域两态不变，稿 prose）。根宽恒 100% 随 FloatBar 壳（壳 = 唯一宽度动画源，
// 300ms 形态切换，2026-08-29 用户定）；定值档 250 常态 / 450 聚焦与展开实测都归壳（FloatBar）。
// 搜索图标 亮色/三级、筛选图标与收起钮 亮色/半透明-2，hover 均升 亮色/二级（稿 hover 口径）；
// 占位符 亮色/半透明-2；已输入文字 亮色/二级（稿无已填态，随输入框族文字档，2026-08-29 定）。
// 筛选钮 = 高 24 padding 4/8 圆角 6 + 亮色/半透明-3 底 + 立体感-通用 + 次要文字-加粗 亮色/二级；
// hover 底升 半透明-2；选中 底 半透明-1 + 文字 一级（稿 prose）。
// 自动实时搜索（input 即 v-model）；搜索图标点击 = 预留（未来展开向量/AI 搜索选项，稿 prose）。
// 业务接入（2026-08-31 悬浮栏接入轮）：expose focus/blur + escape emit 供业务壳 FloatBarHost
//（Ctrl+F 聚焦 / Esc 清空失焦口径）。
import { onMounted, ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";
import "./bars.css";

const props = withDefaults(
  defineProps<{
    filters?: readonly string[]; // 筛选按钮组（稿 10 项）
    active?: readonly string[]; // 选中筛选项（父级持有，filter-toggle 翻转）
    initialExpanded?: boolean; // 预览页静态陈列
    forceState?: "" | "focus"; // 预览页静态验收：聚焦宽 450
    forceHoverFilter?: string; // 预览页静态验收：某筛选钮 hover
  }>(),
  {
    filters: () => ["颜色", "时长", "标签", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"],
    active: () => [],
    initialExpanded: false,
    forceState: "",
    forceHoverFilter: undefined,
  },
);
const emit = defineEmits<{
  "search-click": []; // 预留：未来向量 / AI 搜索选项
  "filter-toggle": [key: string];
  "expanded-change": [v: boolean]; // FloatBar 宽度重测挂钩
  "focused-change": [v: boolean];
  escape: []; // Esc（业务壳口径：清空并失焦，FloatBarHost）
}>();
const query = defineModel<string>("query", { default: "" });

const inputRef = ref<HTMLInputElement>();
defineExpose({
  focus: () => inputRef.value?.focus(),
  blur: () => inputRef.value?.blur(),
});

const expanded = ref(props.initialExpanded);
const focused = ref(false);

function setExpanded(v: boolean): void {
  expanded.value = v;
  emit("expanded-change", v);
}
function setFocused(v: boolean): void {
  focused.value = v;
  emit("focused-change", v);
}

// FloatBar 壳定值宽档依赖 focused/expanded 镜像——force 静态态（预览验收）与初始展开挂载即上报
onMounted(() => {
  if (expanded.value) emit("expanded-change", true);
  if (props.forceState === "focus") emit("focused-change", true);
});
watch(
  () => props.forceState,
  (s) => emit("focused-change", s === "focus" || focused.value),
);

// 筛选组滚轮横滚（2026-09-09 挤压裁剪轮）：壳被侧栏挤压收窄时筛选组壳内裁剪、横向滚轮翻看；
// 无需横滚时不拦默认行为（穿透给下层列表纵向滚动）
function onFiltersWheel(e: WheelEvent): void {
  const el = e.currentTarget as HTMLElement;
  if (el.scrollWidth <= el.clientWidth) return;
  e.preventDefault();
  el.scrollLeft += e.deltaY || e.deltaX;
}
</script>

<template>
  <div
    class="fb-mode fbs-root"
    :class="[{ expanded, focused: focused || forceState === 'focus' }]"
  >
    <button class="fbs-ic search" @click="emit('search-click')">
      <AppIcon name="search" :size="18" />
    </button>
    <input
      ref="inputRef"
      v-model="query"
      class="fbs-input"
      placeholder="搜索..."
      @focus="setFocused(true)"
      @blur="setFocused(false)"
      @keydown.esc="emit('escape')"
    />
    <!-- 栏内按钮一律 @mousedown.prevent：阻止 input 失焦——否则聚焦态点筛选时 blur 抢先触发
         壳宽 450→250 收缩动画，按钮随壳边移出指针下方，mouseup 落空 click 不成立（2026-09-09 走查修复）；
         顺带保持输入聚焦，点筛选/收起后可继续敲字 -->
    <button v-if="!expanded" class="fbs-ic filter" @mousedown.prevent @click="setExpanded(true)">
      <AppIcon name="filter" :size="18" />
    </button>
    <template v-else>
      <div class="fb-group fbs-filters" @wheel="onFiltersWheel">
        <button
          v-for="f in filters"
          :key="f"
          class="fbs-chip"
          :class="[{ on: active.includes(f), 'force-hover': forceHoverFilter === f }]"
          @mousedown.prevent
          @click="emit('filter-toggle', f)"
        >
          {{ f }}
        </button>
      </div>
      <button class="fbs-ic collapse" @mousedown.prevent @click="setExpanded(false)">
        <AppIcon name="expand" :size="14" />
      </button>
    </template>
  </div>
</template>

<style scoped>
/* 根宽恒 100% 随壳（壳 = 唯一宽度动画源）；内容态差异 = 输入区伸缩与筛选组挂载 */
.fbs-root {
  flex: none;
  width: 100%;
}

/* 图标钮：搜索 = 亮色/三级；筛选 / 收起 = 亮色/半透明-2；hover 统一升 亮色/二级 */
.fbs-ic {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  padding: 0;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fbs-ic.search,
.fbs-ic.filter {
  width: 18px;
  height: 18px;
}
.fbs-ic.search {
  color: var(--n-l3);
}
.fbs-ic.filter,
.fbs-ic.collapse {
  color: var(--n-l-a2);
}
.fbs-ic.collapse {
  position: relative;
  width: 14px;
  height: 14px;
}
/* 点击热区伪元素外扩（2026-09-13 实机走查：热区太小——树容器 item 展开钮同款 ::after 手法，
   §14 口径；上右下 -4 吃壳 padding，左 -6 吃筛选组间距） */
.fbs-ic.collapse::after {
  content: "";
  position: absolute;
  inset: -4px -4px -4px -6px;
}
.fbs-ic:hover {
  color: var(--n-l2);
}
/* 收起钮 = expand（∨ 朝下）旋转 90° 得朝左 ‹（同 DropButton 旋转口径） */
.fbs-ic.collapse :deep(.app-icon) {
  transform: rotate(90deg);
}

/* 输入区：透明去框（底色 / 圆角在壳）；文字 正文档 二级，占位符 半透明-2（稿值） */
.fbs-input {
  flex: 1;
  min-width: 0;
  padding: 0;
  border: none;
  outline: none;
  background: transparent;
  font-family: var(--n-font-family);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.fbs-input::placeholder {
  color: var(--n-l-a2);
}
/* 展开态输入区稿值：图标+占位符 组 198 = icon 18 + gap 6 + 文字区 174 */
.fbs-root.expanded .fbs-input {
  flex: none;
  width: 174px;
}

/* 筛选组 = 展开态唯一可缩件（2026-09-09 挤压裁剪轮）：壳被左右侧栏挤压收窄时本组壳内裁剪，
   输入区（174 稿值）与收起钮 flex:none 钉住两端不被裁走；组内滚轮横滚翻看，不出滚动条
   （同旧帧条 .frames 口径——菜单/列表形态隐藏滚动条） */
.fbs-filters {
  flex: 0 1 auto;
  min-width: 0;
  overflow-x: auto;
  scrollbar-width: none;
}
.fbs-filters::-webkit-scrollbar {
  display: none;
}

/* 筛选钮（稿：颜色/时长/标签/扩展名/文件夹/比例/链接/大小/添加日期/修改日期） */
.fbs-chip {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  height: 24px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  background: var(--n-l-a3);
  box-shadow: var(--n-effect-raised);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.fbs-chip:hover,
.fbs-chip.force-hover {
  background: var(--n-l-a2);
}
.fbs-chip.on {
  background: var(--n-l-a1);
  color: var(--n-l1);
}
</style>
