<script setup lang="ts">
// TokenVar：内联显示某个 CSS 变量在 :root 的实际值（验收 tokens.css 与 Token.md 一致性）
import { onMounted, ref } from "vue";

const props = defineProps<{ token: string }>();

const text = ref("…");
onMounted(() => {
  const v = getComputedStyle(document.documentElement).getPropertyValue(props.token).trim();
  text.value = v || "（未定义）";
});
</script>

<template>
  <span class="pv-var">{{ props.token }} = {{ text }}</span>
</template>
