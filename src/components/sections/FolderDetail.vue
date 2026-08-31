<script setup lang="ts">
// 文件夹 / 快速访问详情形态（7.1 二十三轮贴稿，稿 = 右侧面板-文件夹详情 + 2026-08-31 用户定案）：
// 触发 = 选中普通文件夹 / 视口未选中文件时显示当前视口文件夹·档案信息（DetailPanel 装配分流）。
// 文件夹形态（稿）：缩略图区块（无 border-top：240×240 容器 padding 24 + folder-detail-panel 装饰图标
//   192×174 原值）→ 输入框区块（名称 + 链接——文件夹无链接数据，框留空占位只读）→
//   基础信息区块（BaseDataItem：文件数 / 档案数 / 子文件夹数 / 占用空间——添加日期 FolderNode 无字段，
//   无数据不加行沿旧口径）；**无分段控制器、无信息查看管理钮**（稿注「文件夹详情不需要信息查看按钮」）。
// 快速访问形态（用户定简化版）：无缩略图；单名称框（显示 全部/未分类等固定文案，不让改名）+
//   数据卡两行（文件数 / 占用空间 = 当前视口 Placement 清单算，占用空间按 assetId 去重合计——
//   与 FolderNode.totalSize 同口径）。
import { computed } from "vue";
import TextInput from "../inputs/TextInput.vue";
import BaseDataItem from "../items/BaseDataItem.vue";
import AppIcon from "../AppIcon.vue";
import { formatSize } from "../../lib/formatSize";
import type { DetailTarget } from "../../lib/detailSections";
import { useContentStore } from "../../stores/content";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "folder" | "quick" }> }>();
const content = useContentStore();

const folder = computed(() => (props.target.kind === "folder" ? props.target.node : null));
const quick = computed(() => (props.target.kind === "quick" ? props.target : null));

// 快速访问视口统计：文件数 = 视口 Placement 清单长；占用空间 = 按 assetId 去重合计 size
const quickStats = computed(() => {
  if (!quick.value) return null;
  const list = content.placementsFor(quick.value.viewKey);
  const seen = new Set<string>();
  let size = 0;
  for (const p of list) {
    if (seen.has(p.assetId)) continue;
    seen.add(p.assetId);
    size += p.size;
  }
  return { count: list.length, size };
});

// 基础数据行（无数据源的行不加——SectionInfo 旧口径沿）
const dataRows = computed<{ label: string; values: string[] }[]>(() => {
  if (folder.value) {
    const n = folder.value;
    const archives = n.children.filter((c) => c.kind === "archive").length;
    const subs = n.children.length - archives;
    return [
      { label: "文件数", values: [n.count.toLocaleString()] },
      { label: "档案数", values: [archives.toLocaleString()] },
      { label: "子文件夹数", values: [subs.toLocaleString()] },
      { label: "占用空间", values: [formatSize(n.totalSize)] },
    ];
  }
  const s = quickStats.value;
  if (!s) return [];
  return [
    { label: "文件数", values: [s.count.toLocaleString()] },
    { label: "占用空间", values: [formatSize(s.size)] },
  ];
});
</script>

<template>
  <!-- 缩略图区块（仅文件夹形态；稿：240×240 容器 padding 24 / 通用二级底 + 通用三级 1px / 圆角 8，
       folder-detail-panel 装饰图标 192×174 原值；快速访问形态无缩略图——用户定简化） -->
  <section v-if="folder" class="fd-thumb">
    <div class="fd-frame">
      <AppIcon name="folder-detail-panel" :size="192" />
    </div>
  </section>
  <!-- 输入框区块（稿：padding 12 / gap 8 / border-top 通用/三级）；
       快速访问 = 单名称框（不让改名——本面板输入框恒只读口径） -->
  <section class="fd-inputs">
    <TextInput :model-value="folder ? folder.name : (quick?.name ?? '')" readonly placeholder="名称" />
    <TextInput v-if="folder" model-value="" readonly placeholder="http://" trailing-icon="link" />
  </section>
  <!-- 基础信息区块（稿：border-top；BaseDataItem 信息卡） -->
  <section class="fd-data">
    <BaseDataItem :rows="dataRows" />
  </section>
</template>

<style scoped>
/* 缩略图区块（稿：padding 12，无 border-top——首块） */
.fd-thumb {
  padding: var(--n-space-12);
  flex: none;
}
/* 文件夹图标容器（稿：240×240 padding 24 / 通用二级底 + 通用三级 1px / 圆角 8） */
.fd-frame {
  width: 100%;
  aspect-ratio: 1;
  box-sizing: border-box;
  padding: var(--n-space-24);
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
/* 输入框 / 数据区块（稿：padding 12 / border-top 通用/三级；输入框 gap 8） */
.fd-inputs {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  flex: none;
}
.fd-data {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  flex: none;
}
</style>
