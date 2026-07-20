<script setup lang="ts">
// 工作空间 / 标签 / 快捷入口视图：按 PRD 04-1 摆各自快速访问项，内容为占位（后续阶段接入）。
import { computed, ref } from "vue";
import SectionHeader from "./SectionHeader.vue";
import TreeItem from "./TreeItem.vue";
import { useContentStore } from "../stores/content";
import { useUiStore } from "../stores/ui";

const ui = useUiStore();
const content = useContentStore();
const quickOpen = ref(true);
const groupOpen = ref(true);
const pinsOpen = ref(true);

interface ViewConfig {
  quick: { key: string; icon: string; label: string }[];
  hint: string;
}

const configs: Record<string, ViewConfig> = {
  workspace: {
    quick: [
      { key: "ws-all", icon: "library-all", label: "全部" },
      { key: "ws-pinned", icon: "window-pin-on", label: "置顶" },
      { key: "ws-frequent", icon: "recent", label: "常用" },
    ],
    hint: "工作空间（索引式管理）为占位视图，后续阶段接入真实文件夹树。",
  },
  tags: {
    quick: [
      { key: "tag-all", icon: "tag", label: "全部标签" },
      { key: "tag-untagged", icon: "untagged", label: "未标签文件" },
      { key: "tag-uncategorized", icon: "uncategorized", label: "未分类标签" },
    ],
    hint: "标签视图为占位，标签管理后续阶段接入。",
  },
  pins: {
    quick: [],
    hint: "快捷入口为占位：文件夹 / 标签 / 文件经右键菜单固定后在此显示。",
  },
};

const current = computed(() => configs[ui.leftView] ?? configs.workspace);
</script>

<template>
  <div class="sidebar-views">
    <section v-if="current.quick.length > 0" class="block">
      <SectionHeader title="快速访问" pad="0 8px 0 14px" v-model:open="quickOpen" />
      <div v-if="quickOpen" class="items">
        <TreeItem
          v-for="q in current.quick"
          :key="q.key"
          :icon="q.icon"
          :label="q.label"
          :selected="content.sidebarSelected === `quick:${q.key}`"
          @select="content.sidebarSelected = `quick:${q.key}`"
        />
      </div>
    </section>

    <section v-if="ui.leftView === 'tags'" class="block">
      <SectionHeader title="标签群组" :count="2" addable pad="0 8px 0 14px" v-model:open="groupOpen" />
      <div v-if="groupOpen" class="items">
        <TreeItem icon="tag-group" label="标签群组" :count="12" />
        <TreeItem icon="tag-group" label="标签群组" :count="34" />
      </div>
    </section>

    <section v-if="ui.leftView === 'pins'" class="block">
      <SectionHeader title="快捷入口" pad="0 8px 0 14px" v-model:open="pinsOpen" />
      <div v-if="pinsOpen" class="items">
        <TreeItem icon="folder-closed" label="新建文件夹" />
        <TreeItem icon="tag" label="全部标签" />
      </div>
    </section>

    <p class="hint">{{ current.hint }}</p>
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
  gap: var(--space-6);
  padding: var(--space-8) 0;
}
.items {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  padding: 0 var(--space-8) 0 4px;
}
.hint {
  margin: var(--space-8) var(--space-16) 0;
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4-40);
}
</style>
