<script setup lang="ts">
// 标题 item 族（区块标题-树容器 / 区块标题-详情链接 / 分组标题）——
// docs/06_UI重构/Design Token/组件/item/标题 item/
// 常态下动作钮（展开/更多/链接/新建）不显示，hover 出现；按钮组 hover 提亮 亮色/二级。
// 抓手指针只在热区出现（bar = 整行 / lead = 仅文字区，热区外普通指针——
// 2026-10-02 用户定：行空白抓手误导像可拖拽，§63）。
// 例外 = staticActions（2026-08-31 用户实机定）：折叠钮与动作组常显——详情面板链接关系 tab 五区块
// 按区块稿常态帧（稿上按钮组/展开钮即常态画出；「标题上的添加按钮本身就是入口」），
// 树容器 / 列表容器等其余调用方维持 hover 现身口径不变。
import AppIcon from "../AppIcon.vue";

withDefaults(
  defineProps<{
    variant: "block" | "group"; // block = 区块标题（12 Bold 四级，16 高）/ group = 分组标题（18 Bold 二级，26 高）
    title: string;
    count?: number | string; // 「标题 / 数量」；不传则无数量段
    expanded?: boolean; // 展开钮朝向（默认收起 >）
    withLink?: boolean; // block 型详情链接变体：剪贴板有链接时才显示链接钮（业务传入）
    withMore?: boolean; // more 钮显隐（详情链接区块稿按钮组 = 链接+添加两钮无 more，7.1 二十三轮扩）
    actions?: boolean; // 动作组（更多/添加）显隐——false = 整块不渲染（列表容器区块标题稿无动作钮，7.1 内容容器轮）
    staticActions?: boolean; // 折叠钮与动作组常显（链接关系 tab 五区块稿口径，2026-08-31 用户定；默认 hover 现身）
    forceState?: "" | "hover"; // 预览页强制态（hover 展开动作组）
    hotZone?: "bar" | "lead"; // 折叠点击热区：bar = 整行（默认，侧栏/详情/弹窗）/ lead = 仅文字区（中间列表容器口径，2026-10-02 用户实机定，§63）
  }>(),
  { count: undefined, expanded: false, withLink: false, withMore: true, actions: true, staticActions: false, forceState: "", hotZone: "bar" },
);

const emit = defineEmits<{ toggle: []; more: [e: MouseEvent]; add: [e: MouseEvent]; link: [e: MouseEvent] }>();
</script>

<template>
  <div
    class="st"
    :class="[variant, staticActions ? 'static-actions' : '', hotZone === 'lead' ? 'hot-lead' : '', forceState ? `force-${forceState}` : '']"
    @click="hotZone === 'bar' && emit('toggle')"
  >
    <!-- hotZone=lead：折叠热区收文字区（标题/数量/展开钮——chev 自带 stop 不重复触发），行空白点击落空不折叠 -->
    <span class="lead" @click="hotZone === 'lead' && emit('toggle')">
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
    <!-- 右组：tail 插槽（常驻自定义内容，如文件夹区块「递归显示」复选框——7.1 内容容器轮加；
         点击自停冒泡防触发行折叠）+ 动作组（actions=false 整块不渲染——列表容器区块标题稿无动作钮） -->
    <span v-if="$slots.tail || actions" class="st-right">
      <span v-if="$slots.tail" class="st-tail" @click.stop><slot name="tail" /></span>
      <span v-if="actions" class="acts">
      <button v-if="withLink" class="act" @click.stop="emit('link', $event)">
        <AppIcon name="link-small" :size="14" />
      </button>
      <button v-if="withMore" class="act" @click.stop="emit('more', $event)">
        <AppIcon :name="variant === 'group' ? 'more-large' : 'more-small'" :size="variant === 'group' ? 18 : 14" />
      </button>
      <button class="act" @click.stop="emit('add', $event)">
        <AppIcon :name="variant === 'group' ? 'add-large' : 'add-small'" :size="variant === 'group' ? 18 : 14" />
      </button>
      </span>
    </span>
  </div>
</template>

<style scoped>
.st {
  display: flex;
  align-items: center;
  justify-content: space-between;
  /* 稿值 gap 10 = figma 默认自动占位（2026-08-28 用户确认：space-between 下仅为最小间距、
     视觉无差），按间距 token 档用 8 */
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
  position: relative;
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
    opacity var(--n-dur-100) var(--n-ease-in-out),
    color var(--n-dur-100) var(--n-ease-in-out);
}
/* 点击热区外扩（2026-08-28 走查：小钮判定太小，缝隙跳抓手/指针光标抖动）——
   伪元素参与命中判定、布局零影响（旧 TreeItem 展开钮同款机制）；
   左 -6 吃满名称与展开的间距，其余 -4；hover 提亮二级（走查定） */
.chev::after {
  content: "";
  position: absolute;
  inset: -4px -4px -4px -6px;
}
.chev:hover {
  color: var(--n-l2);
}
.st:hover .chev,
.st.force-hover .chev {
  opacity: 1;
  visibility: visible;
}
/* staticActions（链接关系 tab 五区块稿口径）：折叠钮与动作组常显，不依赖 hover */
.st.static-actions .chev,
.st.static-actions .acts {
  opacity: 1;
  visibility: visible;
}
.chev:not(.open) {
  transform: rotate(-90deg);
}
/* 右组：tail（常驻内容，如递归显示复选框）+ 动作组横排，space-between 下整体右置 */
.st-right {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: none;
  min-width: 0;
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
/* hover 抓手指针收窄到热区（2026-10-02 用户定：抓手只在真正热区出现，
   行空白抓手误导像可拖拽）——bar 档热区 = 整行；lead 档 = 仅文字区 */
.st:not(.hot-lead):hover,
.st:not(.hot-lead).force-hover {
  cursor: grab;
}
.st.hot-lead .lead:hover {
  cursor: grab;
}
.st:hover .acts,
.st.force-hover .acts {
  opacity: 1;
  visibility: visible;
}
.act {
  position: relative;
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
/* 点击热区外扩：钮间距两半各吃（block gap 8 → -4 / group gap 12 → -6），缝隙不再落空 */
.act::after {
  content: "";
  position: absolute;
  inset: -4px;
}
.group .act::after {
  inset: -6px;
}
.act:hover {
  color: var(--n-l2);
}
</style>
