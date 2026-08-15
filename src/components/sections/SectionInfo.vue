<script setup lang="ts">
// 区块 · 基础信息（3.2 一轮，只读；figma 数据 item §1 信息列表，2026-08-15 走查贴稿）：
// C4 容器卡（padding 12 / gap 16 / 圆角 6 / 1px 暗-悬停）双列——左列标签 12/17 极淡、
// 右列值 12/17 标签色右对齐（列内行距 4；日期值 = 日期 + 时间两截 gap 4，贴稿）。
// 文件 = 尺寸（无则不显示该行）/ 文件大小 / 格式 / 添加日期 / 修改日期（创建日期 metadata 无字段，不加）；
// 档案 = 文件数 / 名称。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import { formatDetailDate, type DetailTarget } from "../../lib/detailSections";
import { formatSize } from "../../lib/formatSize";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

// parts：值按截显示（日期 = [日期, 时间] 两截，贴稿 gap 4）
const rows = computed<{ k: string; parts: string[] }[]>(() => {
  if (props.target.kind === "file") {
    const d = props.target.detail;
    const out: { k: string; parts: string[] }[] = [];
    if (d.width != null && d.height != null) out.push({ k: "尺寸", parts: [`${d.width} x ${d.height}`] });
    out.push({ k: "文件大小", parts: [formatSize(d.size)] });
    out.push({ k: "格式", parts: [d.ext.toUpperCase()] });
    out.push({ k: "添加日期", parts: formatDetailDate(d.importedAt).split(" ") });
    out.push({ k: "修改日期", parts: formatDetailDate(d.modifiedAt).split(" ") });
    return out;
  }
  const n = props.target.node;
  return [
    { k: "文件数", parts: [n.count.toLocaleString()] },
    { k: "名称", parts: [n.name] },
  ];
});
</script>

<template>
  <section class="section-info detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <div v-if="open" class="info-card">
      <div class="col">
        <span v-for="r in rows" :key="r.k" class="k">{{ r.k }}</span>
      </div>
      <div class="col vals">
        <span v-for="r in rows" :key="r.k" class="v">
          <span v-for="p in r.parts" :key="p">{{ p }}</span>
        </span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.section-info {
  display: flex;
  flex-direction: column;
}
/* 信息列表（§1）：C4 容器卡 / padding 12 / gap 16 / 圆角 6 / 1px 暗-悬停 */
.info-card {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-16);
  padding: var(--space-12);
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
}
/* 双列：列内行距 4 */
.col {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}
.col.vals {
  align-items: flex-end;
}
/* 左列标签 12/17 极淡 */
.k {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-faint);
  white-space: nowrap;
}
/* 右列值 12/17 标签色右对齐（日期两截 gap 4） */
.v {
  display: flex;
  gap: var(--space-4);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--text-label);
  text-align: right;
  overflow-wrap: anywhere;
}
</style>
