<script setup lang="ts">
// 筛选弹窗 · 范围（时长 / 大小共用；稿 = 中间面板-多功能悬浮栏-筛选/时长 328×58、筛选/大小 348×58）：
// 单位切换器（通用/六级底 + 圆角 6 + 高 28：单位文字并列空格分隔，当前一级、其余三级 hover 升二级，
// 尾置 expand 14 装饰朝左，稿「单位下拉菜单」形态）；最小 / 最大输入（TextInput lg 69 宽，占位符
// 「最小/最大」l4，focusout / Enter 提交，非法值不生效）；清除钮（TagButton solid md）= 清空本维度。
// 单位语义：时长 = 帧/秒/分（unit 随条件下发，后端按素材 fps 换算帧）；大小 = KB/MB/GB（纯显示档，
// 条件恒字节——数值由 FloatBarHost 按当前单位换算进出）。
import { ref, watch } from "vue";
import AppIcon from "../AppIcon.vue";
import TagButton from "../buttons/TagButton.vue";
import TextInput from "../inputs/TextInput.vue";
import "./bars.css";

const props = withDefaults(
  defineProps<{
    units: readonly string[]; // ["帧","秒","分"] / ["KB","MB","GB"]
    unit: string; // 当前单位
    min?: number | null; // 当前单位下的数值（时长原值；大小已按显示档换算）
    max?: number | null;
    forceHover?: string; // 预览页静态验收：某单位 hover
  }>(),
  { min: null, max: null, forceHover: undefined },
);
const emit = defineEmits<{
  setUnit: [u: string];
  setRange: [range: { min: number | null; max: number | null }];
  clear: [];
}>();

const minDraft = ref("");
const maxDraft = ref("");
watch(
  () => [props.min, props.max],
  ([mn, mx]) => {
    minDraft.value = mn != null ? String(mn) : "";
    maxDraft.value = mx != null ? String(mx) : "";
  },
  { immediate: true },
);
function commit(): void {
  const mn = minDraft.value.trim() ? Number(minDraft.value) : null;
  const mx = maxDraft.value.trim() ? Number(maxDraft.value) : null;
  if (mn != null && (!Number.isFinite(mn) || mn < 0)) return;
  if (mx != null && (!Number.isFinite(mx) || mx < 0)) return;
  emit("setRange", { min: mn, max: mx });
}
</script>

<template>
  <div class="fb-pop fp-root fp-range">
    <!-- 单位切换器（稿：通用/六级底 + 圆角 6 + 28 高，单位文字并列 + expand 14 装饰朝左） -->
    <div class="fr-unit">
      <button
        v-for="u in units"
        :key="u"
        class="fr-u"
        :class="[{ on: u === unit, 'force-hover': forceHover === u }]"
        @click="emit('setUnit', u)"
      >
        {{ u }}
      </button>
      <AppIcon name="expand" :size="14" class="fr-expand" />
    </div>
    <TextInput v-model="minDraft" size="lg" class="fr-num" placeholder="最小" @focusout="commit" @keydown.enter="commit" />
    <TextInput v-model="maxDraft" size="lg" class="fr-num" placeholder="最大" @focusout="commit" @keydown.enter="commit" />
    <TagButton label="清除" size="md" @click="emit('clear')" />
  </div>
</template>

<style scoped>
.fp-range {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  width: max-content;
}
/* 单位切换器（稿 86×28 通用/六级；大小稿 106 宽随字数自适应） */
.fr-unit {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  height: 28px;
  padding: 0 var(--n-space-8);
  background: var(--n-g6);
  border-radius: var(--n-radius-6);
  flex: none;
}
.fr-u {
  padding: 0 var(--n-space-2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
  white-space: nowrap;
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.fr-u:hover,
.fr-u.force-hover {
  color: var(--n-l2);
}
.fr-u.on {
  color: var(--n-l1);
}
/* expand 装饰朝左（FloatBarSearch 收起钮同款旋转口径） */
.fr-expand {
  color: var(--n-l3);
  transform: rotate(90deg);
}
.fr-num {
  width: 69px;
  flex: none;
}
</style>
