<script setup lang="ts">
// 标题 item 族（区块标题-树容器 / 区块标题-详情链接 / 分组标题）——
// docs/UI 重构/Design Token/组件/item/标题 item/
// 常态下动作钮（展开/更多/链接/新建）不显示，hover 出现（抓手指针）；按钮组 hover 提亮 亮色/二级。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    variant: "block" | "group"; // block = 区块标题（12 Bold 四级，16 高）/ group = 分组标题（18 Bold 二级，26 高）
    title: string;
    count?: number | string; // 「标题 / 数量」；不传则无数量段
    expanded?: boolean; // 展开钮朝向（默认收起 >）
    withLink?: boolean; // block 型详情链接变体：剪贴板有链接时才显示链接钮（业务传入）
    forceState?: "" | "hover"; // 预览页强制态（hover 展开动作组）
  }>(),
  { count: undefined, expanded: false, withLink: false, forceState: "" },
);

const emit = defineEmits<{ toggle: []; more: []; add: []; link: [] }>();
</script>

<template>
  <div
    class="st"
    :class="[variant, forceState ? `force-${forceState}` : '']"
    @click="emit('toggle')"
  >
    <span class="lead">
      <span class="tt">
        <span class="txt">{{ title }}</span>
        <template v-if="count !== undefined">
          <span class="sep">/</span>
          <span class="num">{{ count }}</span>
        </template>
      </span>
      <button class="chev" :class="{ open: expanded }" @click.stop="emit('toggle')">
        <AppIcon name="expand" :size="14" />
      </button>
    </span>
    <span class="acts">
      <button v-if="withLink" class="act" @click.stop="emit('link')">
        <AppIcon name="link-small" :size="14" />
      </button>
      <button class="act" @click.stop="emit('more')">
        <AppIcon :name="variant === 'group' ? 'more-large' : 'more-small'" :size="variant === 'group' ? 18 : 14" />
      </button>
      <button class="act" @click.stop="emit('add')">
        <AppIcon :name="variant === 'group' ? 'add-large' : 'add-small'" :size="variant === 'group' ? 18 : 14" />
      </button>
    </span>
  </div>
</template>

<style scoped>
.st {
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 稿值 gap 10 不在间距 token 档（2/4/6/8/12…），space-between 下仅为最小间距，
     取最近 token 档 8 兜底（自由空间由 space-between 分配，视觉无差）——待用户定是否补 10 档 */
  gap: var(--n-space-8);
  padding-left: var(--n-space-8);
  cursor: default;
}
.st.block {
  height: 16px;
}
.st.group {
  height: 26px;
}
.lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  min-width: 0;
}
.tt {
  display: flex;
  align-items: center;
  gap: var(--n-space-4);
  min-width: 0;
  white-space: nowrap;
}
.block .tt {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-l4);
}
.group .tt {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.txt {
  overflow: hidden;
  text-overflow: ellipsis;
}
.chev {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 14px;
  height: 14px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  /* 稿：常态下展开/更多/新建都不显示，hover 出现——展开钮同样藏入 hover 显现组 */
  opacity: 0;
  visibility: hidden;
  transition:
    transform var(--n-dur-100) var(--n-ease-in-out),
    opacity var(--n-dur-100) var(--n-ease-in-out);
}
.st:hover .chev,
.st.force-hover .chev {
  opacity: 1;
  visibility: visible;
}
.chev:not(.open) {
  transform: rotate(-90deg);
}
/* 动作组：常态隐藏，hover 出现（opacity + 可见性，稿未写滑入）；hover 行 = 抓手指针 */
.acts {
  display: flex;
  align-items: center;
  flex: none;
  opacity: 0;
  visibility: hidden;
  transition: opacity var(--n-dur-100) var(--n-ease-in-out);
}
.block .acts {
  gap: var(--n-space-8);
}
.group .acts {
  gap: var(--n-space-12);
}
.st:hover,
.st.force-hover {
  cursor: grab;
}
.st:hover .acts,
.st.force-hover .acts {
  opacity: 1;
  visibility: visible;
}
.act {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  transition: color var(--n-dur-100) var(--n-ease-in-out);
}
.block .act {
  width: 14px;
  height: 14px;
}
.group .act {
  width: 18px;
  height: 18px;
}
.act:hover {
  color: var(--n-l2);
}
</style>
