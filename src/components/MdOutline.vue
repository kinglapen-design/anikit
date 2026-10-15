<script setup lang="ts">
// MarkDown 大纲栏（2026-10-15 大纲轮，PRD 04_文件预览「md 大纲栏」；交互参照 = 飞书文档大纲）：
// 纯展示件——标题清单 / 当前节行号由 PreviewText 计算喂入，点击抛 jump(line)；样式无稿初案
//（preview 陈列定稿口径见速查 §71）：宽 168（设置面板左导航同宽，§70）/ 右缘 1px g3 竖线
//（md 分屏分割条同档，滚轮 hover 压线一小段可接受 = 2026-08-13 主窗口统一口径）/
// item 26 高 / 圆角 6 / padding 4 8 / 正文 13/18 l2（菜单 item 同档，§3），缩进 12/级；
// hover 底 l-a3（菜单 item hover 同档）；当前节 cur = 底 l-a2 + 文字 l1（设置面板导航选中同口径，
// 选中态不叠 hover，§3）；空态「本文档没有标题」l3；底 66 悬浮栏遮挡补偿（§18）；
// 滚动 = kit-scroll + ScrollThumb（§2）。forceHoverLine = preview 陈列演示态桥接。
import { nextTick, ref, watch } from "vue";
import ScrollThumb from "./ScrollThumb.vue";
import type { MdOutlineItem } from "../lib/mdOutline";

const props = withDefaults(
  defineProps<{ items: MdOutlineItem[]; activeLine?: number | null; forceHoverLine?: number | null }>(),
  { activeLine: null, forceHoverLine: null },
);
const emit = defineEmits<{ jump: [line: number] }>();

const scrollRef = ref<HTMLElement>();

// 当前节滚进视野（正文滚动联动高亮时，防高亮条目被甩出大纲视口）
watch(
  () => props.activeLine,
  async () => {
    if (props.activeLine == null) return;
    await nextTick();
    const root = scrollRef.value;
    const el = root?.querySelector<HTMLElement>(".mo-item.cur");
    if (!root || !el) return;
    const r = root.getBoundingClientRect();
    const er = el.getBoundingClientRect();
    if (er.top < r.top) root.scrollTop += er.top - r.top;
    else if (er.bottom > r.bottom) root.scrollTop += er.bottom - r.bottom;
  },
);
</script>

<template>
  <aside class="md-outline">
    <div ref="scrollRef" class="mo-scroll kit-scroll">
      <template v-if="items.length">
        <button
          v-for="h in items"
          :key="h.line"
          type="button"
          class="mo-item"
          :class="{ cur: h.line === activeLine, 'force-hover': h.line === forceHoverLine }"
          :style="{ paddingLeft: `${8 + (h.level - 1) * 12}px` }"
          @click="emit('jump', h.line)"
        >
          {{ h.text }}
        </button>
      </template>
      <p v-else class="mo-empty">本文档没有标题</p>
    </div>
    <ScrollThumb :body="scrollRef" />
  </aside>
</template>

<style scoped>
.md-outline {
  position: relative; /* ScrollThumb 锚（§2：relative 包裹层 + 滚动容器 + 悬浮件） */
  flex: none;
  width: 168px; /* 设置面板左导航同宽（§70） */
  min-height: 0;
  display: flex;
  border-right: 1px solid var(--n-g3); /* md 分屏分割条同档竖线 */
}
.mo-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: var(--n-space-12) var(--n-space-8) 66px; /* 底 66 = 悬浮栏遮挡补偿（§18 同口径） */
  display: flex;
  flex-direction: column;
  gap: 2px; /* 菜单组内 item 间距同档 */
}
.mo-item {
  flex: none;
  display: block;
  width: 100%;
  height: 26px;
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
  text-align: left;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  transition:
    background var(--n-dur-150) var(--n-ease-in-out),
    color var(--n-dur-150) var(--n-ease-in-out);
}
.mo-item:hover,
.mo-item.force-hover {
  background: var(--n-l-a3); /* 菜单 item hover 同档 */
}
.mo-item.cur {
  background: var(--n-l-a2); /* 设置面板导航选中 = a2 底 + l1 文字（§70 同口径） */
  color: var(--n-l1);
}
.mo-item.cur:hover {
  background: var(--n-l-a2); /* 选中态行 hover 不再变色（§3） */
}
.mo-empty {
  padding: var(--n-space-12) var(--n-space-8);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
</style>
