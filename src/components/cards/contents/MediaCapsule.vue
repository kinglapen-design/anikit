<script setup lang="ts">
// 附加件 · 悬停媒体胶囊（video / audio）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/（video.ts 带音量钮 / audio.ts 不带）；样式块原样随迁、值未动。
// 悬停才现（2026-08-29 用户定）：显隐走祖先态选择器 .fc-root:hover / .force-hover，过渡 150ms。
import AppIcon from "../../AppIcon.vue";
import "../cards.css"; // kc-float / kc-info 共享铬（层叠一次导入，与 FileCard 同源）

defineOptions({ inheritAttrs: false });
withDefaults(
  defineProps<{
    capsuleText?: string; // 胶囊文本（video = 帧数 / audio = 时长）
    playing?: boolean; // 播放 ↔ 暂停
    muted?: boolean; // 音量 ↔ 静音（muteIcon 档才出）
    muteIcon?: boolean; // video 档：音量/静音钮（audio 胶囊无此钮）
  }>(),
  { capsuleText: "", playing: false, muted: false, muteIcon: false },
);
</script>

<template>
  <div class="kc-float fc-capsule">
    <span v-if="capsuleText" class="kc-info fc-capsule-text">{{ capsuleText }}</span>
    <AppIcon
      :name="playing ? 'card-hover-pause' : 'card-hover-play'"
      :size="14"
      class="fc-capsule-icon"
    />
    <AppIcon
      v-if="muteIcon"
      :name="muted ? 'card-hover-mute' : 'card-hover-volume'"
      :size="14"
      class="fc-capsule-icon"
    />
  </div>
</template>

<style scoped>
/* 悬停胶囊（右上 12/12，gap 6）：常态隐，hover 现（150ms） */
.fc-capsule {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  z-index: 4;
  gap: var(--n-space-6);
  opacity: 0;
  pointer-events: none;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
}
.fc-root:hover .fc-capsule,
.fc-root.force-hover .fc-capsule {
  opacity: 1;
  pointer-events: auto;
}
.fc-capsule-text {
  overflow: visible;
}
/* 胶囊图标：常态 亮色/二级，自身 hover 提亮 亮色/一级（稿「hover-播放/音量按钮」节） */
.fc-capsule-icon {
  color: var(--n-l2);
  cursor: pointer;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fc-capsule-icon:hover {
  color: var(--n-l1);
}
</style>
