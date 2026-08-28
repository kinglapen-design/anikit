<script setup lang="ts">
// Token.md 6_动画：时长五档 + 缓动三条；hover 行演示位移
import TokenVar from "../TokenVar.vue";

const durations = [
  { name: "快速反馈", token: "--n-dur-100" },
  { name: "通用", token: "--n-dur-150" },
  { name: "小位移", token: "--n-dur-250" },
  { name: "形态切换", token: "--n-dur-300" },
  { name: "大幅度位移", token: "--n-dur-400" },
];

const easings = [
  { name: "缓入缓出", token: "--n-ease-in-out" },
  { name: "缓入", token: "--n-ease-in" },
  { name: "缓出", token: "--n-ease-out" },
];
</script>

<template>
  <section id="design-system-motion" class="pv-section">
    <h2 class="pv-sec-title">动画</h2>
    <p class="pv-sec-desc">Token.md 第 6 节；hover 行内轨道演示（方块从左到右）。</p>
    <div class="pv-group">
      <h3 class="pv-group-title">6.1 时长（缓动统一 --n-ease-in-out）</h3>
      <div class="rows">
        <div v-for="d in durations" :key="d.token" class="row">
          <div class="meta">
            <span class="nm">{{ d.name }}</span>
            <TokenVar :token="d.token" />
          </div>
          <div class="track">
            <div
              class="block"
              :style="{ transition: `transform var(${d.token}) var(--n-ease-in-out)` }"
            />
          </div>
        </div>
      </div>
    </div>
    <div class="pv-group">
      <h3 class="pv-group-title">6.2 缓动（时长统一 --n-dur-300）</h3>
      <div class="rows">
        <div v-for="e in easings" :key="e.token" class="row">
          <div class="meta">
            <span class="nm">{{ e.name }}</span>
            <TokenVar :token="e.token" />
          </div>
          <div class="track">
            <div
              class="block"
              :style="{ transition: `transform var(--n-dur-300) var(${e.token})` }"
            />
          </div>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.rows {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  margin-top: var(--n-space-12);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--n-space-24);
  padding: var(--n-space-8) var(--n-space-16);
  border-radius: var(--n-radius-6);
  background: var(--n-g2);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.meta {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  width: 200px;
  flex: none;
}
.nm {
  color: var(--n-l1);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.track {
  flex: 1;
  height: 24px;
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  overflow: hidden;
  display: flex;
  align-items: center;
  padding: 0 var(--n-space-2);
}
.block {
  width: 20px;
  height: 20px;
  border-radius: var(--n-radius-4);
  background: var(--n-brand);
  will-change: transform;
}
.row:hover .block {
  transform: translateX(240px);
}
</style>
