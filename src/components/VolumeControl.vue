<script setup lang="ts">
// 音量控件（5.2，音频/视频播放条共用）：图标点击切静音（volume/mute 图标随状态），
// 悬停出小滑块浮层调音量（04_滑块语言：C3 / inset 1px border-1 / 圆角 8 / --shadow-pop）。
// 浮层底部 4px 透明桥接区（padding）保证指针从按钮滑入浮层不断开。
import { computed, ref } from "vue";
import IconButton from "./IconButton.vue";
import SideSlider from "./SideSlider.vue";
import { usePreviewMediaStore } from "../stores/previewMedia";

const media = usePreviewMediaStore();
const open = ref(false);

// SideSlider 0~100 ↔ volume 0~1
const vol = computed({
  get: () => Math.round(media.volume * 100),
  set: (v: number) => media.setVolume(v / 100),
});
</script>

<template>
  <div class="volume" @pointerenter="open = true" @pointerleave="open = false">
    <IconButton
      :name="media.muted || media.volume === 0 ? 'mute' : 'volume'"
      :title="media.muted ? '取消静音' : '静音'"
      @click="media.toggleMute()"
    />
    <div v-if="open" class="vol-pop-wrap">
      <div class="vol-pop">
        <SideSlider v-model="vol" :min="0" :max="100" />
      </div>
    </div>
  </div>
</template>

<style scoped>
.volume {
  position: relative;
  display: inline-flex;
  flex: none;
}
/* 浮层定位壳：按钮正上方，padding-bottom 4 作指针桥接 */
.vol-pop-wrap {
  position: absolute;
  bottom: 100%;
  left: 50%;
  transform: translateX(-50%);
  padding-bottom: var(--space-4);
  z-index: 10;
}
/* 浮层卡片：C3 / inset 1px border-1 / 圆角 8 / --shadow-pop / 内边距 8 */
.vol-pop {
  display: flex;
  align-items: center;
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
}
</style>
