<script setup lang="ts">
// 搜索框（1.2 重构，figma-css 01_主窗口 11648 搜索框段）：通栏 46 / C3 / 上边框 暗-悬停——
// 搜索按钮 + 范围 chip + 占位文案（快捷键引导）+ 快捷键徽章组（/ 筛选 · // 搜索 · ~ 全局）+ 历史下拉按钮。
// 3.6 一轮点亮分词搜索：输入即搜（防抖 150ms，search store → search_assets 命令），
// 范围 chip 点击切默认范围（kbd 徽章样式，2026-08-19 用户定）/ `~` 前缀本条强制全局 / Ctrl+F 聚焦。
// 搜索按钮与历史下拉无业务保持占位（`/` 筛选归 3.6 二轮）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { useSearchStore } from "../stores/search";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ panelId: string; viewKey: string }>();
const search = useSearchStore();
const ui = useUiStore();

// 初始回读（面板热重载 / 布局恢复残留 query 不丢字）；变更经防抖驱动后端搜索
const text = ref(search.queryFor(props.panelId));
const inputRef = ref<HTMLInputElement>();

watch(text, (v) => search.setQuery(props.panelId, props.viewKey, v));

// 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案）
watch([() => props.viewKey, () => ui.showSubfolders], () => {
  search.rerun(props.panelId, props.viewKey);
});

// Ctrl+F 聚焦请求（requestFocus 记 panelId，仅本搜索框响应）
watch(
  () => search.focusSeq,
  () => {
    if (search.focusPanelId === props.panelId) {
      inputRef.value?.focus();
      inputRef.value?.select();
    }
  },
);

// Esc = 清空并失焦（清空经 watch text → setQuery 即时退出搜索态）
function onEsc(): void {
  text.value = "";
  inputRef.value?.blur();
}

function onToggleScope(): void {
  search.toggleGlobal(props.panelId, props.viewKey);
}

const chipLabel = computed(() => search.chipLabel(props.panelId, props.viewKey));

onMounted(() => {
  // 热重载 / 布局恢复：残留 query 命中可能已失效，补搜一次恢复结果态
  search.rerun(props.panelId, props.viewKey);
});

onBeforeUnmount(() => search.removePanel(props.panelId));
</script>

<template>
  <div class="search-bar">
    <button class="search-btn" v-tip="'搜索'">
      <AppIcon name="search" :size="18" />
    </button>
    <!-- 范围 chip（PRD 03_查找文件 01：搜索框左侧，点击与 ~ 前缀等效切换）；样式 = kbd 徽章（用户定） -->
    <button class="kbd scope-chip" v-tip="'点击切换搜索范围'" @click="onToggleScope">
      {{ chipLabel }}
    </button>
    <input
      ref="inputRef"
      v-model="text"
      type="text"
      placeholder="/ 筛选看板    // 分词搜索    ~ 搜索全局"
      @keydown.esc="onEsc"
    />
    <div class="kbd-group">
      <span class="kbd">/ 筛选</span>
      <span class="kbd">// 搜索</span>
      <span class="kbd">~ 全局</span>
    </div>
    <button class="drop-btn" v-tip="'搜索历史'">
      <AppIcon name="nav-history" :size="18" class="rot" />
    </button>
  </div>
</template>

<style scoped>
/* 总容器（figma 11648）：通栏 46 / padding 8 16 / 间距 6 / C3 / 上边框 暗-悬停 */
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-16);
  flex: none;
  background: var(--c3);
  border-top: 1px solid var(--line-hover);
}
/* 搜索按钮（figma 11675）：30×30 padding 6 圆角 6，图标标签档 */
.search-btn,
.drop-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.search-btn:hover,
.drop-btn:hover {
  background: var(--line-hover);
}
/* 下拉按钮图标（figma 11752 整体 rotate 90°） */
.rot {
  transform: rotate(90deg);
}
input {
  flex: 1;
  min-width: 0;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  background: transparent;
}
input::placeholder {
  color: var(--text-faint);
}
/* 快捷键徽章组（figma 11808 等）：高 25 / padding 4 8 / C3 + 暗-悬停边框 / 圆角 6 / 极淡档 */
.kbd-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: none;
}
.kbd {
  display: inline-flex;
  align-items: center;
  padding: var(--space-4) var(--space-8);
  height: 25px;
  box-sizing: border-box;
  background: var(--c3);
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-faint);
  white-space: nowrap;
}
/* 范围 chip = kbd 徽章可点击形态（2026-08-19 用户定样式参照）；文案常态档可辨，hover 同 search-btn 罩层 */
.scope-chip {
  flex: none;
  color: var(--text-label);
  cursor: pointer;
  transition: background 0.15s ease;
}
.scope-chip:hover {
  background: var(--line-hover);
}
</style>
