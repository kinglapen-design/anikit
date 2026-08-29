<script setup lang="ts">
// 帧条（播放条-帧条稿）：与播放条同宽、间距 12（稿 prose——同宽与间距归父容器，组件宽 100% 随父）；
// 壳 = .fb-pop + padding 8 + gap 8（高随内容：8 + 76 + 8 = 92 稿值链）。
// 两端上/下帧钮 = 尖括号 angle-bracket（下一帧 flip-h），34 宽纵贯圆角 6，图标 亮色/三级、hover 升二级
// （稿无 hover 节，随悬浮栏族按钮口径）；帧图 129×76 = 通用/二级底 + 1px 通用/三级描边 + 圆角 8，
// 无图 = 空槽；当前帧高亮稿未画 → 不做（记 09 待办，不自创）。
import AppIcon from "../AppIcon.vue";
import "./bars.css";

withDefaults(defineProps<{ frames?: readonly (string | null)[] }>(), {
  frames: () => [null, null, null, null, null], // 稿 5 格
});
defineEmits<{ "page-prev": []; "page-next": []; select: [i: number] }>();
</script>

<template>
  <div class="fb-pop fs-root">
    <button class="fs-nav" @click="$emit('page-prev')">
      <AppIcon name="angle-bracket" :size="18" />
    </button>
    <div class="fs-frames">
      <button v-for="(f, i) in frames" :key="i" class="fs-frame" @click="$emit('select', i)">
        <img v-if="f" :src="f" alt="" />
      </button>
    </div>
    <button class="fs-nav next" @click="$emit('page-next')">
      <AppIcon name="angle-bracket" :size="18" />
    </button>
  </div>
</template>

<style scoped>
.fs-root {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding: var(--n-space-8);
  width: 100%;
}

/* 上/下帧钮：34 宽纵贯（align stretch），圆角 6，图标 三级 → hover 二级 */
.fs-nav {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
  align-self: stretch;
  width: 34px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fs-nav:hover {
  color: var(--n-l2);
}
.fs-nav.next :deep(.app-icon) {
  transform: scaleX(-1);
}

/* 帧图格：129×76 通用/二级底 + 1px 通用/三级描边 + 圆角 8（1px 线 = 真 border 口径） */
.fs-frames {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-8);
  flex: 1;
  min-width: 0;
}
.fs-frame {
  flex: none;
  width: 129px;
  height: 76px;
  padding: 0;
  overflow: hidden;
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
.fs-frame img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
</style>
