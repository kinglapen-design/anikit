<script setup lang="ts">
// 基础数据 item（信息列表）——docs/UI 重构/Design Token/组件/item/基础数据 item/
// 通用二级底 + 通用三级 1px 描边 + 圆角 8 / padding 12 / 行 gap 4；
// 行 = 标签（次要文字-加粗 亮色/四级）左 · 值（数字档 Fira Mono 中 亮色/三级）右 space-between，
// 多截值（日期 + 时间）gap 4；行内 gap 用 8（figma 的 10 是自动布局占位，速查表口径）。
// 纯展示无 hover 态；宽度随父容器（稿 240 = 详情面板宽，align-self stretch 不锁宽）。
defineProps<{
  rows: { label: string; values: string[] }[]; // 值按截传（日期行 = [日期, 时间] 两截）
}>();
</script>

<template>
  <div class="base-data">
    <div v-for="r in rows" :key="r.label" class="bd-row">
      <span class="bd-label">{{ r.label }}</span>
      <span class="bd-values">
        <span v-for="v in r.values" :key="v" class="bd-value">{{ v }}</span>
      </span>
    </div>
  </div>
</template>

<style scoped>
.base-data {
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
  padding: var(--n-space-12);
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
.bd-row {
  display: flex;
  flex-direction: row;
  justify-content: space-between;
  align-items: center;
  gap: var(--n-space-8);
}
/* 标签 = 次要文字-加粗 · 亮色/四级 */
.bd-label {
  flex: none;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
  white-space: nowrap;
}
/* 值 = 数字档（Fira Mono 中）· 亮色/三级，右对齐，多截 gap 4 */
.bd-values {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-4);
  min-width: 0;
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  text-align: right;
  white-space: nowrap;
}
</style>
