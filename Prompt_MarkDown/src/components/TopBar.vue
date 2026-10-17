<script setup lang="ts">
import { store, isDirty, toggleTheme } from "../store";
</script>

<template>
  <header class="topbar">
    <span class="brand">PromptMD</span>
    <span class="doc-name">
      {{ store.doc ? store.doc.name : "" }}
      <span v-if="isDirty()" class="dirty-dot">●</span>
    </span>
    <button class="theme-toggle" :title="store.theme === 'light' ? '切换为暗色' : '切换为亮色'" @click="toggleTheme">
      <svg v-if="store.theme === 'light'" width="16" height="16" viewBox="0 0 16 16" fill="none">
        <circle cx="8" cy="8" r="3.2" stroke="currentColor" stroke-width="1.4" />
        <path d="M8 1.5v1.6M8 12.9v1.6M1.5 8h1.6M12.9 8h1.6M3.4 3.4l1.1 1.1M11.5 11.5l1.1 1.1M12.6 3.4l-1.1 1.1M4.5 11.5l-1.1 1.1"
          stroke="currentColor" stroke-width="1.4" stroke-linecap="round" />
      </svg>
      <svg v-else width="16" height="16" viewBox="0 0 16 16" fill="none">
        <path d="M13.5 9.8A5.5 5.5 0 1 1 6.2 2.5a4.4 4.4 0 0 0 7.3 7.3Z" stroke="currentColor"
          stroke-width="1.4" stroke-linejoin="round" />
      </svg>
    </button>
  </header>
</template>
