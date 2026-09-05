<script setup lang="ts">
// 多功能悬浮栏·搜索模式 业务壳（7.1 二十二轮，2026-08-31 悬浮栏接入轮）：替代旧通栏 SearchBar
//（tag 化 chips 同日用户定案废除——筛选与搜索分离，正常空格分词；`~` 手敲全局保留，后端前缀协议不变）。
// 定位 = 中间面板居中、距底 12（中间面板-多功能悬浮栏稿），浮于列表上方（遮挡补偿 54 在列表容器）。
// 接线：draft ↔ search store（实时分词，防抖 150 在 store）；Ctrl+F 聚焦（requestFocus）；
// Esc = 清空并失焦（沿用旧栏口径）；视图 / 递归开关变化按新范围重搜（2026-08-19 定案）；
// 筛选按钮组点亮不接业务（本地选中态切换不触发搜索——筛选弹窗 9 稿 + 后端筛选条件另轮）。
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import FloatBar from "./bars/FloatBar.vue";
import { useSearchStore } from "../stores/search";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ panelId: string; viewKey: string }>();
// 「始终显示搜索框」关（ui.showSearchBar=false，2026-09-06 视图菜单轮）时的收回请求：
// Esc 清空失焦后通知 ContentArea 隐藏（v-if 第二条件）；开关恒开时该 emit 无害
const emit = defineEmits<{ requestHide: [] }>();
const search = useSearchStore();
const ui = useUiStore();

// 初始回读（面板热重载 / 布局恢复残留不丢字）
const draft = ref(search.draftFor(props.panelId));
const floatBarRef = ref<InstanceType<typeof FloatBar>>();
// 筛选按钮组选中态（点亮不接业务：本地切换，不触发搜索；筛选弹窗另轮）
const activeFilters = ref<string[]>([]);

watch(draft, (v) => search.setDraft(props.panelId, props.viewKey, v));

// store → 本地回填（主菜单「全局搜索」prefillGlobal 预填 `~` 等外部写入通道；等值不回流防环）
watch(
  () => search.draftFor(props.panelId),
  (v) => {
    if (v !== draft.value) draft.value = v;
  },
);

// 视图 / 递归开关变化：搜索保持并按新范围重搜
watch([() => props.viewKey, () => ui.showSubfolders], () => {
  search.rerun(props.panelId, props.viewKey);
});

// Ctrl+F 聚焦请求（requestFocus 记 panelId，仅本悬浮栏响应）
watch(
  () => search.focusSeq,
  () => {
    if (search.focusPanelId === props.panelId) floatBarRef.value?.focusSearch();
  },
);

function toggleFilter(key: string): void {
  const i = activeFilters.value.indexOf(key);
  if (i >= 0) activeFilters.value.splice(i, 1);
  else activeFilters.value.push(key);
}

// Esc = 清空并失焦（draft 置空经 watch 同步 store）；「始终显示搜索框」关时顺带收回悬浮栏
function onEscape(): void {
  draft.value = "";
  floatBarRef.value?.blurSearch();
  emit("requestHide");
}

onMounted(() => {
  // 热重载 / 布局恢复：残留内容命中可能已失效，补搜一次恢复结果态
  search.rerun(props.panelId, props.viewKey);
});

onBeforeUnmount(() => search.removePanel(props.panelId));
</script>

<template>
  <div class="sf-anchor">
    <FloatBar
      ref="floatBarRef"
      mode="search"
      v-model:query="draft"
      :active="activeFilters"
      @filter-toggle="toggleFilter"
      @escape="onEscape"
    />
  </div>
</template>

<style scoped>
/* 定位 = 中间面板居中、距底 12（稿）；浮于列表上方、穿透空隙（仅栏体可交互） */
.sf-anchor {
  position: absolute;
  left: 0;
  right: 0;
  bottom: var(--n-space-12);
  display: flex;
  justify-content: center;
  pointer-events: none;
}
.sf-anchor > * {
  pointer-events: auto;
}
</style>
