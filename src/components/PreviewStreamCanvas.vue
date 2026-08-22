<script setup lang="ts">
// 预览窗口动图帧流画面（阶段四·预览对接）——watch 预览 store 当前帧 → putImageData。
// 铺满父容器 .img-wrap（尺寸 = 帧 natural × zoom，缩放/平移随 wrap 体系；旋转/翻转 transform 由
// PreviewPanel 经 style 绑定透传）。未出帧时透明（下层缩略图占位可见），出帧后盖住。
import { ref, watch } from "vue";
import { usePreviewStore } from "../stores/preview";

const preview = usePreviewStore();
const cv = ref<HTMLCanvasElement>();
const hasFrame = ref(false);

watch(
  () => preview.animFrame,
  (f) => {
    const c = cv.value;
    if (!f || !c) return;
    if (c.width !== f.w || c.height !== f.h) {
      c.width = f.w;
      c.height = f.h;
    }
    c.getContext("2d")?.putImageData(new ImageData(f.rgba, f.w, f.h), 0, 0);
    hasFrame.value = true;
  },
);
</script>

<template>
  <canvas v-show="hasFrame" ref="cv" class="stream-stage" />
</template>

<style scoped>
.stream-stage {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  display: block;
}
</style>
