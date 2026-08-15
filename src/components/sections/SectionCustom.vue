<script setup lang="ts">
// 区块 · 自定义数据（2026-08-15 定案：带区块标题的普通区块——SectionHeader「自定义数据」
// 折叠开合与其他区块一致，无 + 钮；排序/显隐在 22 稿编辑面板与面板内拖拽）。
// 区块 gap 8（标题与内容间距，detail-block 公共规则）；内容容器 = 属性 chips 行 +
// 描述组 + 编辑描述按钮，容器内 gap 12。
// 一轮只读：添加 / 编辑描述一律 disabled 占位（编辑态归二轮）。
// 文件 = 属性 chips（tags）+ 描述（note 单行）+ 编辑描述；档案 = 属性 chips（props）+
// 描述列表（标题: 内容逐行）+ 编辑描述。
// 样式稿 = figma-css 02_布局/01_主窗口.md 自定义数据块内属性标签行 / 描述组 / 编辑描述按钮段。
import { computed, ref } from "vue";
import SectionHeader from "../SectionHeader.vue";
import AppTag from "../AppTag.vue";
import AppButton from "../AppButton.vue";
import IconButton from "../IconButton.vue";
import type { DetailTarget } from "../../lib/detailSections";

const props = defineProps<{ target: DetailTarget; title: string }>();
const open = ref(true);

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));
const tags = computed(() => file.value?.detail.tags ?? []);
const props_ = computed(() => archive.value?.data.props ?? []);
</script>

<template>
  <section class="section-custom detail-block">
    <SectionHeader :title="title" grip v-model:open="open" />
    <!-- 内容容器：gap 12（2026-08-15 定案） -->
    <div v-if="open" class="custom-body">
      <!-- 属性标签行（稿：gap 6；chip 高 25 / C4 / padding 4 8 / 圆角 6 / 12/17 标签色）
           + 行尾添加钮（disabled，二轮编辑业务） -->
      <div class="chips">
        <AppTag v-for="t in file ? tags : props_" :key="t" :label="t" readonly />
        <IconButton name="add-small" small disabled title="添加属性" class="chip-add" />
      </div>
      <!-- 描述组（稿：纯文本行 gap 8；行 = 标签: 标签色 + 内容 常态，行内 gap 4） -->
      <div v-if="file && file.detail.note" class="desc-group">
        <div class="desc-row">
          <span class="d-content">{{ file.detail.note }}</span>
        </div>
      </div>
      <div v-else-if="archive && archive.data.descriptions.length > 0" class="desc-group">
        <div v-for="d in archive.data.descriptions" :key="d.title" class="desc-row">
          <span class="d-label">{{ d.title }}:</span>
          <span class="d-content">{{ d.content }}</span>
        </div>
      </div>
      <!-- 编辑描述按钮（稿：通栏 36 高 / C5 / 圆角 6 / 图标 18 + 14/20 常态，disabled 占位） -->
      <AppButton class="edit-desc" variant="sidebar" icon="edit-description" label="编辑描述" disabled />
    </div>
  </section>
</template>

<style scoped>
.section-custom {
  display: flex;
  flex-direction: column;
}
/* 内容容器：chips + 描述组 + 编辑描述，gap 12（2026-08-15 定案） */
.custom-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
}
/* 属性标签行（稿）：gap 6；chip 文字 12/17 标签色（AppTag 默认常态色，本区域按稿覆盖）
   + 行尾添加钮 20×20 */
.chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-6);
}
.chips .app-tag {
  color: var(--text-label);
}
/* 描述组（稿）：纯文本行 gap 8；行 = 标签: 标签色 + 内容 常态，行内 gap 4 */
.desc-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
}
.desc-row {
  display: flex;
  gap: var(--space-4);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.d-label {
  color: var(--text-label);
  white-space: nowrap;
  flex: none;
}
.d-content {
  color: var(--text-normal);
  min-width: 0;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}
/* 编辑描述按钮（稿）：通栏 36 高 / C5（AppButton sidebar）/ 圆角 6 / 图标 18 + 14/20 常态 */
.section-custom .edit-desc {
  width: 100%;
  height: 36px;
}
</style>
