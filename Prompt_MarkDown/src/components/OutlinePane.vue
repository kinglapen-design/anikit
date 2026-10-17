<script setup lang="ts">
import { store, jumpTo } from "../store";
</script>

<template>
  <div v-if="store.outline.length === 0" class="sidebar-empty">打开文件后显示标题大纲。</div>
  <div
    v-for="item in store.outline"
    :key="item.pos"
    class="outline-row"
    :class="`lv${item.level}`"
    :style="{ paddingLeft: `${(item.level - 1) * 14 + 8}px` }"
    :title="item.text"
    @click="jumpTo(item.pos)"
  >
    {{ item.text }}
  </div>
</template>
