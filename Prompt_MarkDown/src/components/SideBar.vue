<script setup lang="ts">
import {
  store,
  openFile,
  openViaDialog,
  removeRecent,
  toggleNewMenu,
  newFromTemplate,
} from "../store";
import OutlinePane from "./OutlinePane.vue";

function baseName(path: string): string {
  return (path.split(/[\\/]/).pop() || path).replace(/\.md$/i, "");
}
</script>

<template>
  <aside class="sidebar">
    <section class="sidebar-section files">
      <div class="section-header">
        <span>最近打开</span>
        <span class="actions">
          <button class="icon-btn" title="打开文件…" @click="openViaDialog">
            <svg width="13" height="13" viewBox="0 0 14 14" fill="none">
              <path d="M1.5 3.5A1.5 1.5 0 0 1 3 2h2.6l1.4 1.6H11a1.5 1.5 0 0 1 1.5 1.5v5.4A1.5 1.5 0 0 1 11 12H3a1.5 1.5 0 0 1-1.5-1.5v-7Z"
                stroke="currentColor" stroke-width="1.2" stroke-linejoin="round" />
            </svg>
          </button>
        </span>
      </div>
      <div class="section-body">
        <div class="new-row">
          <button class="text-btn" @click.stop="toggleNewMenu">新建 ▾</button>
          <div v-if="store.newMenuOpen" class="new-menu">
            <div class="new-menu-item" @click="newFromTemplate(null)">空白文档</div>
            <div
              v-for="t in store.docTemplates"
              :key="t.path"
              class="new-menu-item"
              @click="newFromTemplate(t)"
            >
              {{ t.name }}
            </div>
          </div>
        </div>
        <div v-if="store.recent.length === 0" class="sidebar-empty">
          还没有打开过文件。<br />点「新建」从模板开始，或右上角打开文件。
        </div>
        <div
          v-for="path in store.recent"
          :key="path"
          class="tree-row"
          :class="{ active: store.doc?.path === path }"
          :title="path"
          @click="openFile(path)"
        >
          <span class="twisty"></span>
          <span class="label">{{ baseName(path) }}</span>
          <span style="flex: 1"></span>
          <button class="icon-btn row-x" title="从列表移除" @click.stop="removeRecent(path)">×</button>
        </div>
      </div>
    </section>

    <section class="sidebar-section templates">
      <div class="section-header"><span>模板配置</span></div>
      <div class="section-body">
        <div class="tpl-group">文档模板</div>
        <div
          v-for="t in store.docTemplates"
          :key="t.path"
          class="tree-row"
          :class="{ active: store.doc?.path === t.path }"
          :title="t.path"
          @click="openFile(t.path)"
        >
          <span class="twisty"></span>
          <span class="label">{{ t.name }}</span>
        </div>
        <div class="tpl-group">规则词表</div>
        <div
          v-for="t in store.menuFiles"
          :key="t.path"
          class="tree-row"
          :class="{ active: store.doc?.path === t.path }"
          :title="t.path"
          @click="openFile(t.path)"
        >
          <span class="twisty"></span>
          <span class="label">{{ t.name }}</span>
        </div>
        <div
          v-if="store.chainsFile"
          class="tree-row"
          :class="{ active: store.doc?.path === store.chainsFile.path }"
          :title="store.chainsFile.path"
          @click="openFile(store.chainsFile.path)"
        >
          <span class="twisty"></span>
          <span class="label">{{ store.chainsFile.name }}</span>
        </div>
        <div v-if="!store.docTemplates.length && !store.menuFiles.length" class="sidebar-empty">
          模板目录为空或加载失败，使用内置默认。
        </div>
      </div>
    </section>

    <section class="sidebar-section outline">
      <div class="section-header"><span>大纲</span></div>
      <div class="section-body">
        <OutlinePane />
      </div>
    </section>
  </aside>
</template>
