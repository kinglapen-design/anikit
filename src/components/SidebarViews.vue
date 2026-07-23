<script setup lang="ts">
// 工作空间 / 标签 / 快捷入口视图（03_UI 布局/02_左侧边栏 06 其他视图布局）：
// 按设计结构摆区块与 item，数据为占位（后续阶段接入）。
import { ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import FolderTree from "./FolderTree.vue";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const content = useContentStore();

const rootOpen = ref(true);
const pinnedOpen = ref(true);
const frequentOpen = ref(true);
const wsTreeOpen = ref(true);
const tagQuickOpen = ref(true);
const tagGroupOpen = ref(true);
const pinAssetOpen = ref(true);
const pinFolderOpen = ref(true);
const pinTagOpen = ref(true);

const tagQuickItems = [
  { key: "tag-all", icon: "tag", label: "全部标签" },
  { key: "tag-uncategorized", icon: "uncategorized", label: "未分类标签" },
  { key: "tag-untagged", icon: "untagged", label: "未标签素材" },
];
</script>

<template>
  <!-- 单根包裹（Transition 要求单根子组件，多根 Fragment 会导致过渡失效、内容透明） -->
  <div>
    <!-- 工作空间：根目录 / 置顶 / 常用 / 文件夹 -->
    <div v-if="ui.leftView === 'workspace'" class="sidebar-views">
    <section class="block">
      <SectionHeader title="根目录" pad="0 8px 0 16px" v-model:open="rootOpen" />
      <div v-if="rootOpen" class="items">
        <TreeItem
          icon="workspace-all"
          label="根目录"
          :count="1235"
          :selected="content.sidebarSelected === 'quick:ws-root'"
          @select="content.sidebarSelected = 'quick:ws-root'"
        />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="置顶" pad="0 8px 0 16px" v-model:open="pinnedOpen" />
      <div v-if="pinnedOpen" class="items">
        <TreeItem icon="folder-closed" label="新建文件夹" :count="132" />
        <TreeItem icon="folder-closed" label="新建文件夹" :count="45" />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="常用" pad="0 8px 0 16px" v-model:open="frequentOpen" />
      <div v-if="frequentOpen" class="items">
        <TreeItem icon="folder-closed" label="新建文件夹" :count="321" />
        <TreeItem icon="folder-closed" label="新建文件夹" :count="87" />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="文件夹" :count="1231" addable pad="0 8px 0 16px" v-model:open="wsTreeOpen" />
      <div v-if="wsTreeOpen" class="items">
        <FolderTree :nodes="content.folders" />
      </div>
    </section>
  </div>

  <!-- 标签视图：快速访问 / 标签群组 -->
  <div v-else-if="ui.leftView === 'tags'" class="sidebar-views">
    <section class="block">
      <SectionHeader title="快速访问" pad="0 8px 0 16px" v-model:open="tagQuickOpen" />
      <div v-if="tagQuickOpen" class="items">
        <TreeItem
          v-for="q in tagQuickItems"
          :key="q.key"
          :icon="q.icon"
          :label="q.label"
          :selected="content.sidebarSelected === `quick:${q.key}`"
          @select="content.sidebarSelected = `quick:${q.key}`"
        />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="标签群组" :count="2" addable pad="0 8px 0 16px" v-model:open="tagGroupOpen" />
      <div v-if="tagGroupOpen" class="items">
        <TreeItem icon="tag-group" label="标签群组" :count="12" />
        <TreeItem icon="tag-group" label="标签群组" :count="34" />
      </div>
    </section>
  </div>

  <!-- 快捷入口：素材 / 文件夹 / 标签 -->
  <div v-else class="sidebar-views">
    <section class="block">
      <SectionHeader title="素材" :count="2" pad="0 8px 0 16px" v-model:open="pinAssetOpen" />
      <div v-if="pinAssetOpen" class="items">
        <TreeItem icon="pin-asset" label="素材" />
        <TreeItem icon="pin-asset" label="素材" />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="文件夹" :count="2" addable pad="0 8px 0 16px" v-model:open="pinFolderOpen" />
      <div v-if="pinFolderOpen" class="items">
        <TreeItem icon="folder-closed" label="新建文件夹" />
        <TreeItem icon="folder-closed" label="新建文件夹" />
      </div>
    </section>
    <section class="block">
      <SectionHeader title="标签" :count="2" addable pad="0 8px 0 16px" v-model:open="pinTagOpen" />
      <div v-if="pinTagOpen" class="items">
        <TreeItem icon="tag" label="全部标签" />
        <TreeItem icon="tag" label="场景" />
      </div>
    </section>
  </div>
  </div>
</template>

<style scoped>
.sidebar-views {
  display: flex;
  flex-direction: column;
}
.block {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-8) 0;
}
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: 0 var(--space-8) 0 0;
}
</style>
