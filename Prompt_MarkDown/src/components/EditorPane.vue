<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref, watch } from "vue";
import { EditorView } from "@codemirror/view";
import { store, markContent, saveDoc, setOutline, clearJump } from "../store";
import { createEditor } from "../editor/createEditor";
import { parseOutline } from "../editor/outline";

const host = ref<HTMLElement | null>(null);
let view: EditorView | null = null;

function mountEditor() {
  if (!host.value || !store.doc) return;
  view?.destroy();
  view = createEditor(host.value, {
    doc: store.doc.content,
    onChange: (text) => {
      markContent(text);
      setOutline(parseOutline(text));
    },
    onSave: () => void saveDoc(),
  });
  setOutline(parseOutline(store.doc.content));
  view.focus();
}

onMounted(mountEditor);

watch(() => store.doc?.path, mountEditor);

watch(
  () => store.jumpPos,
  (pos) => {
    if (pos === null || !view) return;
    clearJump();
    view.dispatch({
      selection: { anchor: pos },
      effects: EditorView.scrollIntoView(pos, { y: "center" }),
    });
    view.focus();
  },
);

onBeforeUnmount(() => {
  view?.destroy();
  view = null;
});
</script>

<template>
  <div class="editor-area">
    <div v-if="!store.doc" class="editor-empty">从左侧打开一个 .md 文件，或新建一个。</div>
    <div v-show="store.doc" ref="host" class="editor-host"></div>
  </div>
</template>
