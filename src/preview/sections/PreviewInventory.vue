<script setup lang="ts">
// 组件台账展区（阶段七重构 R0，2026-09-21）——全组件清单 + 新旧状态徽标，
// 回答「哪里改了哪里没改」：旧件待烧（legacy）清零、业务件 legacy token 归零即重构收尾。
// 数据源 inventory.generated.ts 由 tools/gen-component-inventory.mjs 生成（pnpm inventory 刷新）。
import { computed, ref } from "vue";
import { inventory, inventoryDate, inventoryTotals, type InventoryStatus } from "../inventory.generated";

const STATUS_META: Record<InventoryStatus, { label: string; desc: string }> = {
  legacy: { label: "旧件待烧", desc: "有替代件的逐面板替换，消费方清零后删件" },
  dead: { label: "零消费休眠", desc: "无人引用，确认后删除或接回" },
  business: { label: "业务件", desc: "业务封装件，换代目标 = legacy token 归零" },
  new: { label: "新体系", desc: "阶段七新族 / 已换代壳件" },
};

const filter = ref<InventoryStatus | "all">("all");
const rows = computed(() =>
  filter.value === "all" ? inventory : inventory.filter((e) => e.status === filter.value),
);
const filters: { key: InventoryStatus | "all"; label: string; count: number }[] = [
  { key: "all", label: "全部", count: inventory.length },
  { key: "legacy", label: STATUS_META.legacy.label, count: inventoryTotals.legacy },
  { key: "business", label: STATUS_META.business.label, count: inventoryTotals.business },
  { key: "new", label: STATUS_META.new.label, count: inventoryTotals.new },
  { key: "dead", label: STATUS_META.dead.label, count: inventoryTotals.dead },
];
</script>

<template>
  <section id="design-system-inventory" class="pv-section">
    <h2 class="pv-sec-title">组件台账（{{ inventoryDate }} 盘点）</h2>
    <p class="pv-sec-desc">
      重构烧减验收面：旧件 {{ inventoryTotals.legacy }} 件（消费方共
      {{ inventoryTotals.legacyConsumers }} 处）、组件内 legacy token 共
      {{ inventoryTotals.legacyTokens }} 处（--c1~5 / --text-* / --brand；不含 tokens.css
      定义块与 lib/）。数据由 <code>pnpm inventory</code> 重扫生成，每烧一轮刷新一次。
    </p>

    <div class="pv-group">
      <div class="pv-inv-filters">
        <button
          v-for="f in filters"
          :key="f.key"
          class="pv-inv-filter"
          :class="{ on: filter === f.key }"
          @click="filter = f.key"
        >
          {{ f.label }} {{ f.count }}
        </button>
      </div>
      <table class="pv-inv-table">
        <thead>
          <tr>
            <th>状态</th>
            <th>组件</th>
            <th class="num">消费方</th>
            <th class="num">legacy token</th>
            <th>替代件 / 备注</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in rows" :key="e.path">
            <td>
              <span class="pv-inv-badge" :class="e.status">{{ STATUS_META[e.status].label }}</span>
            </td>
            <td class="pv-inv-path">{{ e.path }}</td>
            <td class="num">{{ e.consumers }}</td>
            <td class="num">
              <span :class="{ 'pv-inv-hot': e.legacyTokens >= 10 }">{{ e.legacyTokens }}</span>
            </td>
            <td class="pv-inv-note">
              <template v-if="e.status === 'legacy'">
                {{ e.replacement ? `→ ${e.replacement}` : "无替代件，待定案" }}
              </template>
              <template v-else-if="e.status === 'dead'">零消费，待确认删除 / 接回</template>
              <template v-else>{{ STATUS_META[e.status].desc }}</template>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.pv-inv-filters {
  display: flex;
  gap: var(--n-space-8);
  flex-wrap: wrap;
}
.pv-inv-filter {
  padding: var(--n-space-6) var(--n-space-16);
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
  color: var(--n-l2);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.pv-inv-filter:hover {
  background: var(--n-g4);
  color: var(--n-l1);
}
.pv-inv-filter.on {
  background: var(--n-g5);
  color: var(--n-l1);
  font-weight: var(--n-fw-bold);
}
.pv-inv-table {
  margin-top: var(--n-space-12);
  width: 100%;
  border-collapse: collapse;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l2);
}
.pv-inv-table th {
  text-align: left;
  padding: var(--n-space-6) var(--n-space-12);
  color: var(--n-l3);
  font-weight: var(--n-fw-bold);
  border-bottom: 1px solid var(--n-l-a1);
}
.pv-inv-table td {
  padding: var(--n-space-6) var(--n-space-12);
  border-bottom: 1px solid var(--n-l-a1);
  vertical-align: top;
}
.pv-inv-table th.num,
.pv-inv-table td.num {
  text-align: right;
  font-family: var(--n-font-mono);
}
.pv-inv-path {
  font-family: var(--n-font-mono);
  color: var(--n-l1);
}
.pv-inv-note {
  color: var(--n-l3);
}
.pv-inv-hot {
  color: var(--n-warning);
  font-weight: var(--n-fw-bold);
}
.pv-inv-badge {
  display: inline-block;
  padding: 0 var(--n-space-8);
  border-radius: var(--n-radius-4);
  font-size: var(--n-fs-secondary);
  white-space: nowrap;
}
.pv-inv-badge.legacy {
  background: var(--n-danger);
  color: var(--n-l1);
}
.pv-inv-badge.dead {
  background: var(--n-warning);
  color: var(--n-g1);
}
.pv-inv-badge.business {
  background: var(--n-g5);
  color: var(--n-l2);
}
.pv-inv-badge.new {
  background: var(--n-brand);
  color: var(--n-l1);
}
code {
  font-family: var(--n-font-mono);
}
</style>
