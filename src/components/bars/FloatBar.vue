<script setup lang="ts">
// 多功能悬浮栏——搜索栏（含展开成筛选）/ MarkDown 工具栏 / 播放条 同一容器（稿 prose）：
// 切换时底色不变、宽度平滑伸缩、功能按钮 Y 轴出场入场（250ms 小位移 + 位移 4px，2026-08-29 用户定）。
// 宽度机制 = DropButton 同款 JS 实测（内容自适应宽，CSS 无法过渡 auto，踩坑 055）：
// 内容槽 .fb-slot 恒 max-content 可测（离场内容 absolute 脱离流不干扰测量），壳显式 width 过渡，
// ready 类挂载 rAF 后开过渡防首帧闪；模式切换 / 搜索栏展开收起·聚焦失焦（expanded-change /
// focused-change）/ 字体就绪 均触发重测。
// 无需悬浮栏的界面 = 宽度和透明度退场（稿 prose）→ visible=false 时 width 0 + opacity 0。
// 模式内容的 props / 事件经 $attrs 透传（inheritAttrs: false），如 v-model:query / v-model:progress /
// @action / @toggle-play；布局口径 = 中间面板居中、距底 12（中间面板-多功能悬浮栏稿，业务接入轮）。
import { computed, nextTick, onMounted, ref, watch } from "vue";
import FloatBarSearch from "./FloatBarSearch.vue";
import FloatBarMarkdown from "./FloatBarMarkdown.vue";
import FloatBarPlay from "./FloatBarPlay.vue";
import "./bars.css";

defineOptions({ inheritAttrs: false });

const props = withDefaults(
  defineProps<{
    mode?: "search" | "markdown" | "play";
    visible?: boolean;
    width?: number; // 显式宽度覆盖（跳过实测；帧条同宽组合等父级定宽场景）
  }>(),
  { mode: "search", visible: true, width: undefined },
);

const comps = { search: FloatBarSearch, markdown: FloatBarMarkdown, play: FloatBarPlay };
const modeComp = computed(() => comps[props.mode]);

const slotEl = ref<HTMLElement>();
const shellW = ref(0);
const ready = ref(false); // 测量完成前禁过渡，防首帧闪动画

async function syncWidth(): Promise<void> {
  if (props.width != null) {
    shellW.value = props.width; // 显式覆盖：跳过实测
    return;
  }
  await nextTick();
  if (slotEl.value) shellW.value = slotEl.value.offsetWidth;
}

// 模式切换：Transition 同帧挂新内容（离场 absolute 不占宽），nextTick 即测得新宽
watch(() => props.mode, syncWidth);
watch(() => props.width, syncWidth);

onMounted(() => {
  syncWidth().then(() => requestAnimationFrame(() => (ready.value = true)));
  // Fira Mono 等内嵌字体就绪后数字/筛选钮宽度会变，补一次重测
  document.fonts?.ready.then(() => syncWidth());
});
</script>

<template>
  <div
    class="fb-shell"
    :class="[{ anim: ready, gone: !visible }]"
    :style="{ width: `${shellW}px` }"
  >
    <div ref="slotEl" class="fb-slot">
      <Transition name="fb">
        <component
          :is="modeComp"
          :key="mode"
          v-bind="$attrs"
          @expanded-change="syncWidth"
          @focused-change="syncWidth"
        />
      </Transition>
    </div>
  </div>
</template>

<style scoped>
/* 壳宽 = 内容槽实测宽（js 绑定），过渡 250ms 小位移；gone = 宽度 + 透明度退场（稿 prose） */
.fb-shell.anim {
  transition:
    width var(--n-dur-250) var(--n-ease-in-out),
    opacity var(--n-dur-250) var(--n-ease-in-out);
}
.fb-shell.gone {
  width: 0 !important;
  opacity: 0;
}

/* 内容槽恒 max-content（壳 overflow hidden 裁切换动画溢出） */
.fb-slot {
  position: relative;
  width: max-content;
}

/* 功能按钮 Y 轴出场入场：入场自下 4px、出场向上 4px + 渐隐，250ms 小位移（2026-08-29 用户定）；
   离场 absolute 脱离流，不撑内容槽、不干扰测宽（同名 Transition 类作用于子组件根，scoped 可及） */
.fb-enter-active,
.fb-leave-active {
  transition:
    opacity var(--n-dur-250) var(--n-ease-in-out),
    transform var(--n-dur-250) var(--n-ease-in-out);
}
.fb-enter-from {
  opacity: 0;
  transform: translateY(4px);
}
.fb-leave-to {
  opacity: 0;
  transform: translateY(-4px);
}
.fb-leave-active {
  position: absolute;
  top: 0;
  left: 0;
}
</style>
