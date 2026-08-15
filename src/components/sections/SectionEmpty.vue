<script setup lang="ts">
// 区块 · 空态（3.2 一轮，只读）：副本 / 关系 / 音频 / 提示词共用——
// 业务未接，SectionHeader 标题照常 + #tail 放 disabled 的 + 钮占位，内容为空
//（2026-08-15 定案：无内容区块不隐藏）。
import { ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import IconButton from "../IconButton.vue";

defineProps<{ title: string }>();
const open = ref(true);
</script>

<template>
  <section class="section-empty detail-block">
    <SectionHeader :title="title" grip v-model:open="open">
      <template #tail>
        <!-- 添加钮占位（业务二轮接）：禁用态常显 -->
        <IconButton name="add-small" small disabled title="添加" />
      </template>
    </SectionHeader>
  </section>
</template>

<style scoped>
.section-empty {
  display: flex;
  flex-direction: column;
}
</style>
