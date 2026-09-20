<script setup lang="ts">
// 内容件 · 缩略图（general / video 共用）——R5 扩展点注册表化（2026-09-21）自 FileCard 抽出，
// 注册见 src/lib/cardSpecs/（general.ts / video.ts）；样式块原样随迁、值未动。
// 宿主经 props 映射接线，本件按名声明子集；inheritAttrs 关闭防宿主属性透传落根。
defineOptions({ inheritAttrs: false });
withDefaults(
  defineProps<{
    thumbUrl?: string; // 空 = 不出图（壳底色兜底，同原 kind 分支）
    thumbFit?: "cover" | "contain" | "icon"; // 缩略图适配三档
    square?: boolean; // shell 缩略图 / 未导入卡 = 50% 居中（稿 square 注）
  }>(),
  { thumbUrl: "", thumbFit: "cover", square: false },
);
</script>

<template>
  <img
    v-if="thumbUrl"
    :src="thumbUrl"
    class="fc-thumb"
    :class="square ? 'shell' : thumbFit"
    alt=""
    decoding="async"
    draggable="false"
  />
</template>

<style scoped>
/* 缩略图：cover 充满（旧网格口径）；contain = 网格固定格完整显示不裁切（2026-07-24 用户定）；
   icon / shell = 50% 居中 contain（格式图标 Eagle 口径 / 稿 square 注）。
   铺满档自带圆角 8：图片走图片绘制路径画角（比容器蒙版裁剪平滑），与描边弧同心同半径——
   2026-08-30 用户实机亮图四角锯齿（.kc-clip 蒙版弧 + 描边弧双线错位），图片自圆角后裁剪弧合一 */
.fc-thumb.cover,
.fc-thumb.contain {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  border-radius: var(--n-radius-8);
}
.fc-thumb.cover {
  object-fit: cover;
}
.fc-thumb.contain {
  object-fit: contain;
}
.fc-thumb.icon,
.fc-thumb.shell {
  position: absolute;
  left: 25%;
  top: 25%;
  width: 50%;
  height: 50%;
  object-fit: contain;
}
</style>
