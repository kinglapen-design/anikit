<script setup lang="ts">
// 污染文件 item——docs/06_UI重构/Design Token/组件/item/详情面板 item/污染文件
// 卡片 240×42 / padding 12：警告-小图标 + 名称（正文 二级）；
// 按钮组（打开资源管理器 / 导入 / 回收站，18×18，gap 8）常态不显示，hover 右至左滑入，按钮 hover 提亮二级。
import "./items.css";
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "./TwoToneIcon.vue";

withDefaults(
  defineProps<{
    name: string;
    forceState?: "" | "hover";
  }>(),
  { forceState: "" },
);

const emit = defineEmits<{ click: []; openExplorer: []; import: []; trash: [] }>();
</script>

<template>
  <div
    class="pf ni-card ni-hover"
    :class="forceState ? `force-${forceState}` : ''"
    @click="emit('click')"
  >
    <span class="lead">
      <TwoToneIcon name="warning-small" :size="18" />
      <span class="nm">{{ name }}</span>
    </span>
    <span class="ni-slide">
      <span class="ni-slide-inner">
        <button class="ni-act" @click.stop="emit('openExplorer')">
          <AppIcon name="open-in-explorer" :size="18" />
        </button>
        <button class="ni-act" @click.stop="emit('import')">
          <AppIcon name="import" :size="18" />
        </button>
        <button class="ni-act" @click.stop="emit('trash')">
          <AppIcon name="trash" :size="18" />
        </button>
      </span>
    </span>
  </div>
</template>

<style scoped>
.pf {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 240px;
  height: 42px;
  padding: var(--n-space-12);
  cursor: pointer;
}
.lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.nm {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-width: 0;
}
/* 滑入按钮组不参与收缩（2026-10-16 实机报：三钮 70px 定宽被 flex 比例收缩裁掉尾钮）——
   全部让位由名称吸收（.lead min-width 0 + 省略），机制契约 = 按钮完整滑入、内容被推左；
   钮体样式 = items.css .ni-act 共享类（同日抽出，扫描污染文件弹窗同用） */
.pf .ni-slide {
  flex: none;
}
</style>
