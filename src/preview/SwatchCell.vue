<script setup lang="ts">
// 色板格：色块 + Token.md 中文名 + 变量名 + 实际值；半透明色下垫棋盘格
import TokenVar from "./TokenVar.vue";

defineProps<{ name: string; token: string }>();
</script>

<template>
  <div class="swatch">
    <div class="chip">
      <div class="fill" :style="{ background: `var(${token})` }" />
    </div>
    <div class="meta">
      <span class="nm">{{ name }}</span>
      <TokenVar :token="token" />
    </div>
  </div>
</template>

<style scoped>
.swatch {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  min-width: 0;
}
.chip {
  width: 64px;
  height: 40px;
  flex: none;
  border-radius: var(--n-radius-6);
  overflow: hidden;
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
  /* 棋盘格底（透出半透明 / 渐隐色） */
  background-color: var(--n-g2);
  background-image:
    linear-gradient(45deg, var(--n-g4) 25%, transparent 25%, transparent 75%, var(--n-g4) 75%),
    linear-gradient(45deg, var(--n-g4) 25%, transparent 25%, transparent 75%, var(--n-g4) 75%);
  background-size: 8px 8px;
  background-position:
    0 0,
    4px 4px;
}
.fill {
  width: 100%;
  height: 100%;
}
.meta {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  min-width: 0;
}
.nm {
  color: var(--n-l1);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
</style>
