<script setup lang="ts">
// 链接缩略图 item——docs/06_UI重构/Design Token/组件/item/详情面板 item/链接缩略图
// 卡片 240×42 / padding 12：图片图标 + 名称（正文 二级）+ 数量（数字档 三级）；关闭钮 hover 滑入。
import "./items.css";
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "./TwoToneIcon.vue";

withDefaults(
  defineProps<{
    name: string;
    count?: number | string; // 数量（数字档；不传则不渲染）
    forceState?: "" | "hover";
  }>(),
  { count: undefined, forceState: "" },
);

const emit = defineEmits<{ click: []; close: [] }>();
</script>

<template>
  <div
    class="lt ni-card ni-hover"
    :class="forceState ? `force-${forceState}` : ''"
    @click="emit('click')"
  >
    <span class="lead">
      <TwoToneIcon name="image" :size="18" />
      <span class="nm">{{ name }}</span>
    </span>
    <span class="tail">
      <span v-if="count !== undefined" class="cnt">{{ count }}</span>
      <span class="ni-slide">
        <span class="ni-slide-inner">
          <button class="ni-close" @click.stop="emit('close')">
            <AppIcon name="close-small" :size="14" />
          </button>
        </span>
      </span>
    </span>
  </div>
</template>

<style scoped>
.lt {
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
.tail {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  flex: none;
}
.cnt {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
}
</style>
