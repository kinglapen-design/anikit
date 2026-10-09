<script setup lang="ts">
// 筛选弹窗 · 属性（稿 = 中间面板-多功能悬浮栏-筛选/标签 350×350；「标签」落地名 = 属性，
// PRD 03_查找文件 §2 2026-10-09）：宽 350 壳 padding 0 两段式——头行 44 高
// （padding 8/8/8/12 gap 8：搜索框 TextInput sm leading search + 清除钮 TagButton solid md）
// + 1px 亮色/半透明-3 分隔线 + chips 流式区（padding 12 gap 8，max-h 306 滚轮，稿 prose「列表太长加滚轮」）。
// chips = TagButton solid sm 多选（选中 品牌/常态）；清除 = 清空全部选中（稿定：不清搜索输入）。
// 纯受控组件：items / selected 走 props（业务组装在 FloatBarHost），搜索过滤为组件内纯视图计算。
import { computed, ref } from "vue";
import TagButton from "../buttons/TagButton.vue";
import TextInput from "../inputs/TextInput.vue";
import "./bars.css";

const props = withDefaults(
  defineProps<{
    items?: readonly string[]; // 全库属性词集（已排序）
    selected?: readonly string[];
    forceHover?: string; // 预览页静态验收：某 chip hover
    forceFocusSearch?: boolean; // 预览页静态验收：搜索框聚焦描边
  }>(),
  { items: () => [], selected: () => [], forceHover: undefined, forceFocusSearch: false },
);
const emit = defineEmits<{ toggle: [tag: string]; clear: [] }>();

// 搜索过滤 = 子串包含不区分大小写（分词搜索同族口径）；被过滤掉的选中项仍生效（清除钮可撤）
const query = ref("");
const shown = computed(() => {
  const q = query.value.trim().toLowerCase();
  if (!q) return props.items;
  return props.items.filter((t) => t.toLowerCase().includes(q));
});
</script>

<template>
  <div class="fb-pop fp-root fp-tags">
    <div class="fp-head">
      <TextInput
        v-model="query"
        size="sm"
        leading-icon="search"
        placeholder="搜索..."
        :force-state="forceFocusSearch ? 'focus' : ''"
      />
      <TagButton label="清除" size="md" @click="emit('clear')" />
    </div>
    <div class="fp-taglist">
      <TagButton
        v-for="t in shown"
        :key="t"
        :label="t"
        :selected="props.selected.includes(t)"
        :force-state="forceHover === t ? 'hover' : ''"
        @click="emit('toggle', t)"
      />
    </div>
  </div>
</template>

<style scoped>
.fp-tags {
  display: flex;
  flex-direction: column;
  width: 350px;
  min-width: 350px; /* 稿宽下限防守（同 FilterColorPop 口径，2026-10-09 走查） */
}
.fp-head {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 44px;
  padding: var(--n-space-8) var(--n-space-8) var(--n-space-8) var(--n-space-12);
}
/* chips 流式区：横排 wrap 换行（2026-10-09 走查修复——类名避 bars.css 列表族 .fp-list
   （column 布局），旧同名时被全局 column 穿透成竖排；scoped 未声明 direction 的属性全局可渗入） */
.fp-taglist {
  display: flex;
  flex-direction: row;
  flex-wrap: wrap;
  align-content: flex-start;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  max-height: 306px;
  overflow-y: auto;
  /* 头行与列表间 1px 分隔线（稿 border-top 方案） */
  border-top: 1px solid var(--n-l-a3);
}
</style>
