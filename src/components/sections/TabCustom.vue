<script setup lang="ts">
// 自定义组件 tab（2026-10-12 转正：占位期结束，装入首批两张真卡片——
// 提示词（自基础信息 tab 末位挪回，2026-09-26 二轮口径修订）+ 逐帧笔记
//（2026-08-31 摘出的 DetailNotes 接回换代为 NotesSection））：
// 卡片口径 = 绑定当前选中对象的文件级生产/批注扩展（PRD 07 扩展点第 1 条 2026-10-12 修订），
// 后续新卡片（直方图等）挂同一容器（7.3 T3 detailTabSpecs 注册表化）；
// 排序/显隐 = ui.detailBlocks.custom 注册表驱动（信息查看管理面板第三段）；
// 逐帧笔记仅库内文件形态（assetId 级），档案形态只见提示词卡片（NotesSection 内部 v-if）。
import { computed } from "vue";
import { customCardComponentFor } from "../../lib/detailTabSpecs";
import { visibleDetailBlocks, type DetailTarget } from "../../lib/detailSections";
import { useUiStore } from "../../stores/ui";

defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const ui = useUiStore();

// 卡片渲染序 = 注册表全量序过滤隐藏项（TabBasic 同口径；允许全隐藏，无空态文案）
const blocks = computed(() => visibleDetailBlocks(ui.detailBlocks.custom));
</script></script>

<template>
  <template v-for="b in blocks" :key="b">
    <component :is="customCardComponentFor(b)" v-if="customCardComponentFor(b)" :target="target" />
  </template>
</template>
