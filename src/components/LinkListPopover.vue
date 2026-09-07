<script setup lang="ts">
// 徽章悬停链接列表浮层（阶段七稿：docs/06_UI重构/Design Token/组件/悬浮栏/悬停-tip/悬停-链接列表）：
// 悬停卡片徽章（origin ★ / 链接 icon）弹出该文件全部活位置列表。
// 壳 = 通用/半透明-1 + 1px 亮色/半透明-3 真 border + 小阴影 + blur 小 + 圆角 6；宽随内容（max-content，上限 360）。
// 行结构 = LinkListRows（2026-09-05 抽出共用：右键「打开文件链接所在位置」二级菜单同构）。
// 机制：Teleport body fixed 定位（徽章上方，右对齐；上方不够翻下方），150ms 延迟出，
// 徽章→浮层 hover 桥（120ms 容错），Esc / 行点击关闭。
import { onBeforeUnmount, onMounted, ref } from "vue";
import LinkListRows from "./LinkListRows.vue";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView; anchor?: HTMLElement }>();
const emit = defineEmits<{ close: []; hover: [inside: boolean] }>();

// —— 定位（徽章上方右对齐；上方空间不足翻下方）——
const pos = ref({ left: 0, top: 0 });
const popRef = ref<HTMLElement>();
function place(): void {
  const a = props.anchor;
  if (!a) return;
  const r = a.getBoundingClientRect();
  const el = popRef.value;
  const h = el?.offsetHeight ?? 0;
  const w = el?.offsetWidth ?? 0;
  let top = r.top - h - 4;
  if (top < 8) top = r.bottom + 4; // 上方不够翻下方
  let left = r.right - w;
  if (left < 8) left = 8;
  pos.value = { left, top };
}

// —— 开关：150ms 延迟出（防划过）；Esc 关闭 ——
const shown = ref(false);
let timer = 0;
onMounted(() => {
  timer = window.setTimeout(() => {
    shown.value = true;
    requestAnimationFrame(place);
  }, 150);
  window.addEventListener("keydown", onKey, true);
});
onBeforeUnmount(() => {
  window.clearTimeout(timer);
  window.removeEventListener("keydown", onKey, true);
});
function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    emit("close");
  }
}
</script>

<template>
  <Teleport to="body">
    <div
      v-if="shown"
      ref="popRef"
      class="link-pop"
      :style="{ left: `${pos.left}px`, top: `${pos.top}px` }"
      @mouseenter="emit('hover', true)"
      @mouseleave="emit('hover', false)"
    >
      <LinkListRows :asset-id="props.asset.assetId" @jump="emit('close')" />
    </div>
  </Teleport>
</template>

<style scoped>
/* 浮层壳（悬停-链接列表稿）：通用/半透明-1 + 1px 半透明-3 真 border + 小阴影 + blur 小 + 圆角 6；
   宽随内容（稿 182 = 内容快照），上限 360（行容器 LinkListRows 自限）；z-2000 同悬停浮层档 */
.link-pop {
  position: fixed;
  z-index: 2000;
  width: max-content;
  border: 1px solid var(--n-l-a3);
  border-radius: var(--n-radius-6);
  background: var(--n-g-a1);
  box-shadow: var(--n-shadow-sm);
  backdrop-filter: blur(var(--n-blur-sm));
  overflow: hidden;
}
</style>
