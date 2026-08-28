<script setup lang="ts">
// Token.md 4_圆角 + 5_间距
import TokenVar from "../TokenVar.vue";

const radii = [
  { token: "--n-radius-4" },
  { token: "--n-radius-6" },
  { token: "--n-radius-8" },
  { token: "--n-radius-16" },
];

const spaces = [
  { token: "--n-space-2" },
  { token: "--n-space-4" },
  { token: "--n-space-6" },
  { token: "--n-space-8" },
  { token: "--n-space-12" },
  { token: "--n-space-16" },
  { token: "--n-space-24" },
  { token: "--n-space-36" },
  { token: "--n-space-72" },
];
</script>

<template>
  <section id="design-system-shape" class="pv-section">
    <h2 class="pv-sec-title">圆角 / 间距</h2>
    <p class="pv-sec-desc">Token.md 第 4、5 节。</p>
    <div class="pv-group">
      <h3 class="pv-group-title">4 圆角</h3>
      <div class="radius-row">
        <div v-for="r in radii" :key="r.token" class="radius-demo">
          <div class="box" :style="{ borderRadius: `var(${r.token})` }" />
          <TokenVar :token="r.token" />
        </div>
      </div>
    </div>
    <div class="pv-group">
      <h3 class="pv-group-title">5 间距</h3>
      <div class="space-rows">
        <div v-for="s in spaces" :key="s.token" class="space-demo">
          <div class="bar" :style="{ width: `var(${s.token})` }" />
          <TokenVar :token="s.token" />
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.radius-row {
  display: flex;
  gap: var(--n-space-24);
  margin-top: var(--n-space-12);
  flex-wrap: wrap;
}
.radius-demo {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-8);
}
.box {
  width: 72px;
  height: 72px;
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
}
.space-rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  margin-top: var(--n-space-12);
}
.space-demo {
  display: flex;
  align-items: center;
  gap: var(--n-space-16);
}
.bar {
  height: 12px;
  min-width: 2px;
  border-radius: var(--n-radius-4);
  background: var(--n-brand);
}
</style>
