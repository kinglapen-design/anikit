<script setup lang="ts">
// 列表表头——docs/UI 重构/Design Token/组件/卡片/列表表头.md：
// 通用三级底 + 立体感-通用 + 圆角 8，padding 8/12 gap 12 高 34；
// 列 = 名称（flex 1）/ 尺寸 / 扩展名 / 文件大小 / 修改日期（各 123 宽，与列表卡片列对齐）；
// 列标签 正文-加粗 亮色/三级，hover 升二级并出现箭头；箭头 = chevron-small（朝上 = 升序原样，
// 降序 rotate 180°，旋转 300ms 形态切换——稿注「上下箭头不同时出现，旋转改变方向」）；
// 箭头常态 亮色/四级、自身 hover 升三级；当前排序列箭头恒显。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    sortKey?: string; // 当前排序列 key（其箭头恒显）
    sortDir?: "asc" | "desc";
  }>(),
  { sortKey: "", sortDir: "asc" },
);
const emit = defineEmits<{ sort: [key: string] }>();

// 列定义：sortable = 可作排序依据（sort emit）；尺寸列无对应 SortKey（2026-08-30 内容容器轮：
// lib/sortPlacements 无尺寸档）——纯展示（无箭头、不响应点击）
const columns = [
  { key: "name", label: "名称", grow: true, sortable: true },
  { key: "dims", label: "尺寸", grow: false, sortable: false },
  { key: "ext", label: "扩展名", grow: false, sortable: true },
  { key: "size", label: "文件大小", grow: false, sortable: true },
  { key: "mtime", label: "修改日期", grow: false, sortable: true },
];

function onColClick(c: { key: string; sortable: boolean }): void {
  if (!c.sortable) return;
  emit("sort", c.key);
}
</script>

<template>
  <div class="lh-root">
    <span
      v-for="c in columns"
      :key="c.key"
      class="lh-col"
      :class="{ grow: c.grow, active: sortKey === c.key, sortable: c.sortable }"
      @click="onColClick(c)"
    >
      <span class="lh-label">{{ c.label }}</span>
      <AppIcon
        v-if="c.sortable"
        name="chevron-small"
        :size="14"
        class="lh-arrow"
        :class="{ desc: sortKey === c.key && sortDir === 'desc' }"
      />
    </span>
  </div>
</template>

<style scoped>
.lh-root {
  box-sizing: border-box;
  width: 100%;
  display: flex;
  flex-direction: row;
  align-items: center;
  padding: var(--n-space-8) var(--n-space-12);
  gap: var(--n-space-12);
  background: var(--n-g3);
  box-shadow: var(--n-effect-raised);
  border-radius: var(--n-radius-8);
}
.lh-col {
  flex: none;
  width: 123px;
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-6);
}
/* 可排序列 = 指针 + hover 文案升二级 + 箭头现身；不可排序列（尺寸）纯展示 */
.lh-col.sortable {
  cursor: pointer;
}
.lh-col.grow {
  flex: 1;
  min-width: 0;
  width: auto;
}
.lh-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  white-space: nowrap;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.lh-col.sortable:hover .lh-label {
  color: var(--n-l2);
}
/* 箭头：hover 列 / 当前排序列才现身；desc = rotate 180°（300ms 形态切换） */
.lh-arrow {
  color: var(--n-l4);
  opacity: 0;
  transition:
    opacity var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out),
    transform var(--n-dur-300) var(--n-ease-in-out);
}
.lh-col.sortable:hover .lh-arrow,
.lh-col.active .lh-arrow {
  opacity: 1;
}
.lh-arrow:hover {
  color: var(--n-l3);
}
.lh-arrow.desc {
  transform: rotate(180deg);
}
</style>
