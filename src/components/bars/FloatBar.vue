<script setup lang="ts">
// 多功能悬浮栏——搜索栏（含展开成筛选）/ MarkDown 工具栏 / 播放条 同一容器（稿 prose）：
// 切换时底色不变、宽度平滑伸缩、功能按钮 Y 轴出场入场（300ms 形态切换 + 位移 4px，
// 2026-08-29 用户两轮定：先 250 小位移、走查改 300）。
// 宽度机制 = **壳宽唯一动画源**（2026-08-29 十五轮走查定）：模式根恒 100% 随壳，
// 内容右端件（筛选图标等）随壳边平滑移动——原方案模式根自持 250/450/max-content 宽度类瞬时切换、
// 只有壳在动画，聚焦/失焦时筛选按钮一帧瞬跳（用户实机实锤，作废勿回退）；
// 测量 = 模式根 inline max-content 探针同步读 offsetWidth（同帧还原不闪）；搜索栏定值档
// 250（常态）/ 450（聚焦）不经探针，expanded / 其余模式探针实测；
// 模式切换 / 搜索 expanded-change·focused-change / 字体就绪 均触发重测，ready 类挂载 rAF 后开过渡防首帧闪。
// 无需悬浮栏的界面 = 宽度和透明度退场（稿 prose）→ visible=false 时 width 0 + opacity 0。
// 模式内容的 props / 事件经 $attrs 透传（inheritAttrs: false），如 v-model:query / v-model:progress /
// @action / @toggle-play；布局口径 = 中间面板居中、距底 12（中间面板-多功能悬浮栏稿，业务接入轮）。
import { computed, nextTick, onMounted, ref, watch, type ComponentPublicInstance } from "vue";
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

// 聚焦/失焦转发（业务壳 SearchFloatBar：Ctrl+F 聚焦 / Esc 失焦；仅 search 模式实现 focus/blur）
const modeRef = ref<ComponentPublicInstance | null>(null);
defineExpose({
  focusSearch: () => (modeRef.value as { focus?: () => void } | null)?.focus?.(),
  blurSearch: () => (modeRef.value as { blur?: () => void } | null)?.blur?.(),
});

const slotEl = ref<HTMLElement>();
const shellW = ref(0);
const ready = ref(false); // 测量完成前禁过渡，防首帧闪动画
// 搜索栏状态镜像（FloatBarSearch expanded-change / focused-change 同步；定值宽档判定用）
const searchExpanded = ref(false);
const searchFocused = ref(false);

// inline max-content 探针：同帧设→读→还原，layout 同步完成不闪
function probe(el: HTMLElement): number {
  el.style.width = "max-content";
  const w = el.offsetWidth;
  el.style.width = "";
  return w;
}

async function syncWidth(): Promise<void> {
  if (props.width != null) {
    shellW.value = props.width; // 显式覆盖：跳过实测
    return;
  }
  await nextTick();
  // 切换交叉期槽内新旧两根并存：离场者带 .fb-leave-active，跳过它测新根
  const slot = slotEl.value;
  if (!slot) return;
  const root = [...slot.children].find((c) => !c.classList.contains("fb-leave-active")) as
    | HTMLElement
    | undefined;
  if (!root) return;
  // 搜索栏两档定值（稿值）；展开与其余模式探针实测
  shellW.value =
    props.mode === "search" && !searchExpanded.value
      ? searchFocused.value
        ? 450
        : 250
      : probe(root);
}

function onExpandedChange(v: boolean): void {
  searchExpanded.value = v;
  syncWidth();
}
function onFocusedChange(v: boolean): void {
  searchFocused.value = v;
  syncWidth();
}

// 模式切换：Transition 同帧挂新内容（离场 absolute），nextTick 即测得新宽
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
          ref="modeRef"
          v-bind="$attrs"
          @expanded-change="onExpandedChange"
          @focused-change="onFocusedChange"
        >
          <!-- 插槽透传（2026-09-09 播放条接入轮：业务壳 #progress 波形插槽穿透到模式组件） -->
          <template v-for="(_, name) in $slots" :key="name" #[name]="slotProps">
            <slot :name="name" v-bind="slotProps ?? {}" />
          </template>
        </component>
      </Transition>
    </div>
  </div>
</template>

<style scoped>
/* 壳宽 = 实测/定值目标（js 绑定），过渡 300ms 形态切换；gone = 宽度 + 透明度退场（稿 prose） */
.fb-shell.anim {
  transition:
    width var(--n-dur-300) var(--n-ease-in-out),
    opacity var(--n-dur-300) var(--n-ease-in-out);
}
.fb-shell.gone {
  width: 0 !important;
  opacity: 0;
}

/* 内容槽恒满壳宽——模式根 100% 随壳，内容右端件随壳边平滑移动（十五轮走查定） */
.fb-slot {
  position: relative;
  width: 100%;
}

/* 功能按钮 Y 轴出场入场：入场自下 4px、出场向上 4px + 渐隐，300ms 形态切换（2026-08-29 用户定）；
   离场 absolute 脱离流（宽恒 100% 随壳），同名 Transition 类作用于子组件根，scoped 可及 */
.fb-enter-active,
.fb-leave-active {
  transition:
    opacity var(--n-dur-300) var(--n-ease-in-out),
    transform var(--n-dur-300) var(--n-ease-in-out);
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
  width: 100%;
}
</style>
