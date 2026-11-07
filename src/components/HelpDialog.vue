<script setup lang="ts">
import { ref } from "vue";
import BaseModal from "./modals/BaseModal.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { createMd } from "../lib/markdown";
import { useDialogEscape } from "../lib/dialogKeys";
import tutorialSource from "../../docs/使用教程.md?raw";

const emit = defineEmits<{ close: [] }>();
const contentEl = ref<HTMLElement>();
const rendered = createMd().render(tutorialSource);
useDialogEscape(() => emit("close"));
</script>

<template>
  <BaseModal title="使用教程" :width="840" framed class="help-dialog" @close="emit('close')">
    <div class="help-body">
      <div ref="contentEl" class="help-scroll kit-scroll">
        <article class="md-body help-document" v-html="rendered" />
      </div>
      <ScrollThumb :body="contentEl" />
    </div>
  </BaseModal>
</template>

<style scoped>
.help-dialog :deep(.bm-modal.framed) {
  gap: 0;
  height: min(640px, calc(100vh - var(--n-space-72)));
  max-width: calc(100vw - var(--n-space-72));
  padding: 0;
  overflow: hidden;
}
.help-body {
  position: relative;
  display: flex;
  flex: 1;
  min-height: 0;
  background: var(--n-g1);
  overflow: hidden;
}
.help-scroll {
  flex: 1;
  min-width: 0;
  padding: var(--n-space-24);
  overflow-x: hidden;
  overflow-y: auto;
}
.help-document {
  padding: 0;
  user-select: text;
}
.help-document :deep(h1) {
  font-size: var(--n-fs-md-h2);
  line-height: var(--n-lh-md-h2);
}
.help-document :deep(h2) {
  font-size: var(--n-fs-md-h4);
  line-height: var(--n-lh-md-h4);
  padding-top: var(--n-space-12);
}
.help-document :deep(h3) {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.help-document :deep(p),
.help-document :deep(li) {
  color: var(--n-l2);
}
.help-document :deep(strong) {
  color: var(--n-l1);
}
</style>
