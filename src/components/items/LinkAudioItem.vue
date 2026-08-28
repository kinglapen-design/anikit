<script setup lang="ts">
// 链接音频 item——docs/UI 重构/Design Token/组件/item/详情面板 item/链接音频
// 卡片 240×46 / padding 12：播放钮（22 圆，品牌底 + 亮色/半透明-1 描边，hover 品牌/提亮）
// + 名称（正文 二级）+ 波形装饰 49×18（装饰图标原值）；关闭钮 hover 滑入。
import "./items.css";
import AppIcon from "../AppIcon.vue";
import { getIcon } from "../../lib/icons";

withDefaults(
  defineProps<{
    name: string;
    playing?: boolean;
    forceState?: "" | "hover";
  }>(),
  { playing: false, forceState: "" },
);

const emit = defineEmits<{ click: []; close: []; togglePlay: [] }>();
const wave = getIcon("waveform-deco"); // 装饰图标，保留原值（#F2F2F2@14%）
</script>

<template>
  <div
    class="la ni-card ni-hover"
    :class="forceState ? `force-${forceState}` : ''"
    @click="emit('click')"
  >
    <span class="lead">
      <button
        class="play"
        :class="{ playing }"
        @click.stop="emit('togglePlay')"
      >
        <AppIcon :name="playing ? 'pause' : 'play'" :size="10" />
      </button>
      <span class="nm">{{ name }}</span>
    </span>
    <span class="tail">
      <span class="wave" aria-hidden="true" v-html="wave" />
    </span>
    <span class="ni-act">
      <button class="ni-close" @click.stop="emit('close')">
        <AppIcon name="close-small" :size="14" />
      </button>
    </span>
  </div>
</template>

<style scoped>
.la {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  width: 240px;
  height: 46px;
  padding: var(--n-space-12);
  cursor: pointer;
}
.lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.play {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  flex: none;
  padding: 0 0 0 2px; /* 播放三角光学居中（稿 padding 4 4 4 6） */
  border-radius: 50%;
  background: var(--n-brand);
  border: 1px solid var(--n-l-a1);
  color: var(--n-l1);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.play.playing {
  padding-left: 0; /* 暂停双杆无需光学补偿 */
}
.play:hover {
  background: var(--n-brand-bright);
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
.wave {
  width: 49px;
  height: 18px;
  flex: none;
  display: inline-flex;
}
.wave :deep(svg) {
  width: 100%;
  height: 100%;
  display: block;
}
</style>
