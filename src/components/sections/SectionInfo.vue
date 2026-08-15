<script setup lang="ts">
// 区块 · 基础信息（3.2 一轮，只读）：两列网格（标题左 / 值右，行距 2，同既有基础信息稿）。
// 文件 = 尺寸（无则不显示该行）/ 文件大小 / 格式 / 添加日期 / 修改日期；
// 档案 = 文件数 / 名称（添加日期档案无数据，有则显示）。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import { formatDetailDate, type DetailTarget } from "../../lib/detailSections";
import { formatSize } from "../../lib/formatSize";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const rows = computed<{ k: string; v: string }[]>(() => {
  if (props.target.kind === "file") {
    const d = props.target.detail;
    const out: { k: string; v: string }[] = [];
    if (d.width != null && d.height != null) out.push({ k: "尺寸", v: `${d.width} x ${d.height}` });
    out.push({ k: "文件大小", v: formatSize(d.size) });
    out.push({ k: "格式", v: d.ext.toUpperCase() });
    out.push({ k: "添加日期", v: formatDetailDate(d.importedAt) });
    out.push({ k: "修改日期", v: formatDetailDate(d.modifiedAt) });
    return out;
  }
  const n = props.target.node;
  return [
    { k: "文件数", v: n.count.toLocaleString() },
    { k: "名称", v: n.name },
  ];
});
</script>

<template>
  <section class="section-info detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <div v-if="open" class="detail-table">
      <template v-for="r in rows" :key="r.k">
        <span class="k">{{ r.k }}</span>
        <span class="v">{{ r.v }}</span>
      </template>
    </div>
  </section>
</template>

<style scoped>
.section-info {
  display: flex;
  flex-direction: column;
}
/* 详情容器：两列网格 / 列间距 16 / 行距 2；标题列居左、值列右对齐（同既有基础信息稿） */
.detail-table {
  display: grid;
  grid-template-columns: 1fr auto;
  gap: var(--space-2) var(--space-16);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
}
.k {
  color: var(--text-normal);
  white-space: nowrap;
}
.v {
  color: var(--text-normal);
  justify-self: end;
  text-align: right;
  overflow-wrap: anywhere;
}
</style>
