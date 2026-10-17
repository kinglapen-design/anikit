<script setup lang="ts">
import { onMounted, watchEffect } from "vue";
import { store, loadTemplateConfig } from "./store";
import TopBar from "./components/TopBar.vue";
import SideBar from "./components/SideBar.vue";
import EditorPane from "./components/EditorPane.vue";

watchEffect(() => {
  document.documentElement.dataset.theme = store.theme;
});

onMounted(() => void loadTemplateConfig());
</script>

<template>
  <div class="app-shell">
    <TopBar />
    <main class="app-main">
      <SideBar />
      <EditorPane />
    </main>
  </div>
</template>
