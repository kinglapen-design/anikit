<script setup lang="ts">
// 文件树 item（figma-css 02_目录 item）：高 32、padding 6/8、radius 6；
// 2026-08-08 用户简化定案：折叠箭头包在 item 底色内（不再两段式），item 不携带任何层级缩进
// （层级 = 子层容器缩进 + 缩进线，见 FolderTree）；快速访问（quick）= 无底色、无折叠占位。
// variant 四形态：root = 根父级（C5 底 + 1px 亮-选中边框）/ child = 其他层级（C5-底色，无边框）/
// archive = 档案（C5-底色 + 品牌色提示圆点替代展开箭头）/ quick = 快速访问（无底色无箭头）；
// tint = 自定义色名（red/orange/...）：root 底 = custom-{tint}，child/archive 底 = custom-{tint}-bg；
// hover 叠 暗-悬停；选中叠 亮-选中 + 1px 文字常态内阴影边框（不掺白，2026-08-07 用户定）。
// 命名输入态（6.1 C3）：受控 naming prop——Enter / Esc / 失焦一律保存退出，空或不变 = 保留默认名。
// 拖拽落点（6.1 C4c）：item 高度三等分——上/下 1/3 出 2px 插入线 + hover 底（仅文件夹拖拽），
// 中间 1/3（素材任何位置）= 并入高亮（品牌 12% 底 + inset 1px 品牌色）。
import { computed } from "vue";
import AppIcon from "./AppIcon.vue";
import { dragView } from "../lib/dragOperation";
import { useInlineRename } from "../lib/useInlineRename";
import { toNewTint } from "../lib/customColors";

const props = withDefaults(
  defineProps<{
    icon: string;
    label: string;
    count?: number;
    directCount?: number; // 直接子项文件数（2026-08-20：展开态显示此值，折叠态显示 count 递归总数）
    depth?: number; // 层级（内容缩进：padding-left = 8 + depth×20；底色块满宽不缩进，2026-08-08 用户定）
    expandable?: boolean;
    open?: boolean;
    selected?: boolean;
    // 选择域联动选中：半透明-2 底、无描边（2026-08-30 TreeRowItem 二轮改定 + 2026-09-20 用户确认
    // 「树选中没有边框」；旧 1.5px 品牌描边口径废）——树上"当前目录"= selected 立体感高亮（你在这里），
    // "选择域成员"（文件夹卡片选中联动 / 树多选）= 底色；FolderTree 绑定时互斥
    domainSelected?: boolean;
    variant?: "root" | "child" | "archive" | "quick";
    tint?: string; // 自定义色名（custom-{tint} / custom-{tint}-bg）
    dimmed?: boolean; // 拖拽源半透明（2026-07-28 复验反馈 6①）
    cut?: boolean; // 剪切中半透明（3.4，0.5 档）
    // 落点视觉覆盖（3.4 移动面板：本地 hover 三分判定驱动，不走拖拽引擎 dragView）；
    // 缺省 undefined = 维持引擎驱动（主窗口拖拽）
    dropZoneOverride?: "before" | "after" | "into" | null;
    folderId?: string; // 拖拽导入落点判定（App.vue elementFromPoint → closest）
    naming?: boolean; // 命名输入态（C3 新建文件夹临时节点）
    namingError?: boolean; // 名称校验违规：输入框描边 --n-danger
  }>(),
  {
    count: undefined,
    directCount: undefined,
    depth: 0,
    expandable: false,
    open: false,
    selected: false,
    domainSelected: false,
    variant: "child",
    tint: undefined,
    dimmed: false,
    cut: false,
    dropZoneOverride: undefined,
    folderId: undefined,
    naming: false,
    namingError: false,
  },
);
const emit = defineEmits<{
  toggle: [];
  select: [e: MouseEvent]; // 携带修饰键（2026-07-28 复验反馈 5：Ctrl 加选 / Shift 范围选）
  nameCommit: [name: string];
}>();

// 数量标签显示规则（2026-08-20 用户修订，替换 2026-07-25"父级展开后隐藏"——展开后父级数量消失，
// 用户实机误判"显示有问题"：其实文件直接在父级下、子文件夹是空的）：
// 折叠 = 递归总数（含全部子文件夹，提醒"哪个文件夹里有文件"）；
// 展开 = 直接子项文件数（子文件夹的归属已各自成行，父级数量只算自己的文件）；
// 当前档位为 0 不显示（去视觉噪音）
const displayCount = computed(() => (props.open ? (props.directCount ?? 0) : (props.count ?? 0)));
const showCount = computed(() => displayCount.value > 0);

// 命名输入（2026-07-29 复验三轮"先建后命名"）：初值 = 节点当前名（默认名），挂载聚焦全选；
// Enter / Esc / 失焦一律保存退出（空或不变的判定在父级 FolderTree）
const { draft: nameDraft, bindInput, commit: commitName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.label,
  escSaves: true,
  onCommit: (name) => emit("nameCommit", name),
});

// 拖拽落点视觉（C4c）：引擎按 data-folder-id 匹配，zone 见 dragOperation；
// dropZoneOverride 优先（3.4 移动面板本地 hover 驱动，不经引擎）
const dropZone = computed(
  () =>
    props.dropZoneOverride ??
    (props.folderId && dragView.tree?.folderId === props.folderId ? dragView.tree.zone : null),
);

// 自定义色（3.3 预留；7.1 二十四轮 2026-09-01 迁新名）：tint 经 toNewTint 转正（存量旧名映射、新名原样）；
// root 底 = --n-custom-{tint}-2 暗档，child/archive = 其 50% color-mix 派生（常态-2 半透明同口径）——
// 移动面板等旧体系视觉的近似档，随其重构轮贴稿；经 --tint-bg 注入，hover/选中罩层照常叠加（不掺白）
const tintBg = computed(() => {
  const t = toNewTint(props.tint);
  if (!t) return undefined;
  return props.variant === "root" ? `var(--n-custom-${t}-2)` : `color-mix(in srgb, var(--n-custom-${t}-2) 50%, transparent)`;
});
// 单击选中 + 双击展开/折叠（用 e.detail 连击计数自判，不用 dblclick 事件——Chromium 的 dblclick
// 只在连击序列第 2 次点击派发，第 4/6… 次不再派发，连续双击展开/折叠会失效，2026-08-08 走查）
function onClick(e: MouseEvent): void {
  emit("select", e);
  if (props.expandable && e.detail > 1 && e.detail % 2 === 0) emit("toggle");
}
</script>

<template>
  <div
    class="tree-item"
    :class="[
      variant,
      {
        selected,
        'domain-selected': domainSelected,
        naming,
        'naming-error': namingError,
        'drop-before': dropZone === 'before',
        'drop-after': dropZone === 'after',
        'drop-into': dropZone === 'into',
        dimmed,
        cut,
      },
      tint ? `tint-${tint}` : '',
    ]"
    :style="{ paddingLeft: `${8 + depth * 20}px`, '--tint-bg': tintBg }"
    :data-folder-id="folderId"
    @click="onClick"
  >
    <!-- 档案 item：品牌色提示圆点替代展开箭头（figma 02_目录 item 3）；快速访问无折叠占位 -->
    <span v-if="variant === 'archive'" class="expand archive-dot-slot">
      <AppIcon name="archive-dot" :size="12" />
    </span>
    <button
      v-else-if="variant !== 'quick'"
      class="expand"
      :class="{ placeholder: !expandable }"
      @click.stop="emit('toggle')"
      @dblclick.stop
    >
      <AppIcon name="tree-expand-arrow" :size="12" class="chev" :class="{ closed: !open }" />
    </button>
    <!-- 命名态用 div（input 套 button 是非法嵌套，Chromium 虽可用但行为无保障）；
         Enter / Esc / 失焦一律保存（2026-07-29 复验三轮，不再有"取消新建"） -->
    <div v-if="naming" class="body">
      <AppIcon :name="icon" :size="18" class="deco" />
      <input
        :ref="bindInput"
        v-model="nameDraft"
        class="rename-input"
        :class="{ error: namingError }"
        @keydown.enter.prevent="commitName"
        @keydown.esc.prevent="commitName"
        @blur="commitName"
        @click.stop
        @pointerdown.stop
      />
    </div>
    <button v-else class="body">
      <AppIcon :name="icon" :size="18" class="deco" />
      <span class="label">{{ label }}</span>
      <span v-if="showCount" class="count">{{ displayCount.toLocaleString() }}</span>
    </button>
  </div>
</template>

<style scoped>
/* item 整卡片（2026-08-08 简化定案：箭头包在底色内，item 无层级 padding）：
   高 32 / padding 6 8 / 圆角 6 / 间距 6（figma 02_目录 item） */
.tree-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 32px;
  box-sizing: border-box;
  padding: var(--n-space-6) var(--n-space-8);
  border-radius: var(--n-radius-6);
  cursor: pointer; /* 整卡片可点（2026-08-08 走查：padding 区不再是默认光标盲区） */
  transition: background 0.15s ease;
}
/* 形态底色（tint 时由 --tint-bg 覆盖）：root = C5 + 1px 亮-选中边框（内阴影模拟防抖动）；
   child / archive = C5-底色无边框；quick = 无底色 */
.tree-item.root {
  background: var(--tint-bg, var(--n-g5));
  box-shadow: inset 0 0 0 1px var(--n-l-a2);
}
.tree-item.child,
.tree-item.archive {
  background: var(--tint-bg, var(--n-g4));
}
.tree-item.quick {
  background: transparent; /* 无箭头占位，图标即首列（figma 02_目录 item 4） */
}
/* hover 叠 暗-悬停 罩层（不掺白，2026-08-07 用户定）；选中项悬停保持选中态不变（2026-08-08 走查）。
   并入高亮项同样排除（2026-08-10 走查：:hover:not() 含伪类计特异性 (0,4,0)，压过
   .tree-item.child.drop-into (0,3,0)——拖拽悬停目标时 hover 罩层盖掉并入填充，只见边框不见底） */
.tree-item.quick:hover:not(.selected):not(.drop-into) {
  background: linear-gradient(var(--n-l-a3), var(--n-l-a3));
}
.tree-item.root:hover:not(.selected):not(.drop-into) {
  background: linear-gradient(var(--n-l-a3), var(--n-l-a3)), var(--tint-bg, var(--n-g5));
}
.tree-item.child:hover:not(.selected):not(.drop-into),
.tree-item.archive:hover:not(.selected):not(.drop-into) {
  background: linear-gradient(var(--n-l-a3), var(--n-l-a3)), var(--tint-bg, var(--n-g4));
}
/* 选中：半透明-2 底 + 立体感-通用，无描边（2026-09-20 用户定「树选中没有边框」，对齐 items/TreeRowItem 定稿） */
.tree-item.selected {
  background: var(--n-l-a2);
  box-shadow: var(--n-effect-raised);
}
.tree-item.root.selected {
  background: linear-gradient(var(--n-l-a2), var(--n-l-a2)), var(--tint-bg, var(--n-g5));
}
.tree-item.child.selected,
.tree-item.archive.selected {
  background: linear-gradient(var(--n-l-a2), var(--n-l-a2)), var(--tint-bg, var(--n-g4));
}
/* 选择域联动选中：去边框只留底色（半透明-2，对齐 TreeRowItem 2026-08-30 二轮改定——
   2026-09-20 用户确认「树选中没有边框」；旧 1.5px 品牌描边口径废） */
.tree-item.domain-selected {
  background: linear-gradient(var(--n-l-a2), var(--n-l-a2));
}
/* 拖拽源整项半透明（2026-07-28 复验反馈 6①，拖拽结束引擎清态即恢复） */
.tree-item.dimmed {
  opacity: 0.4;
}
/* 剪切中整项半透明（3.4，0.5 档） */
.tree-item.cut {
  opacity: 0.5;
}
/* 拖拽排序插入线（C4c，仅文件夹拖拽上/下 1/3）：2px 品牌色于 item 上/下沿 */
.tree-item.drop-before::before,
.tree-item.drop-after::after {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  height: 2px;
  background: var(--n-brand);
  z-index: 1;
  pointer-events: none;
}
.tree-item.drop-before::before {
  top: 0;
}
.tree-item.drop-after::after {
  bottom: 0;
}
/* 拖拽并入高亮（C4c 中间 1/3 / 素材任何位置）：品牌/半透明填充（--n-brand-50）叠形态底色
   （保留 tint 自定义色底）+ 2px 品牌/常态描边（2026-09-20 用户定 2px 档，对齐 TreeRowItem；
   原"直接替换底色"写法会抹掉 c5-base / tint 底，观感偏暗不一致） */
.tree-item.drop-into {
  background: linear-gradient(var(--n-brand-50), var(--n-brand-50));
  box-shadow: inset 0 0 0 2px var(--n-brand);
}
.tree-item.root.drop-into {
  background: linear-gradient(var(--n-brand-50), var(--n-brand-50)), var(--tint-bg, var(--n-g5));
}
.tree-item.child.drop-into,
.tree-item.archive.drop-into {
  background: linear-gradient(color-mix(in srgb, var(--n-brand) 30%, transparent), color-mix(in srgb, var(--n-brand) 30%, transparent)), var(--tint-bg, var(--n-g4));
}
/* 展开按钮容器：12×12（figma 02_目录 item），箭头源图形向下：收起 = rotate(-90) 朝右。
   点击热区（2026-08-08 用户定）：::after 外扩——上/下各 10（吃满 item 上下 padding，全高 32）、
   左 8（吃到 item 左缘）、右 6（吃箭头与图标间距）——布局零影响（伪元素参与命中判定） */
.expand {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 12px;
  height: 12px;
  color: var(--n-l3);
  flex: none;
}
.expand::after {
  content: "";
  position: absolute;
  inset: -10px -6px -10px -8px;
}
/* 无子项时占位保留（藏图标不藏容器：列对齐） */
.expand.placeholder {
  pointer-events: none;
}
.expand.placeholder .chev {
  opacity: 0;
}
.chev {
  transition: transform 0.15s ease;
}
.chev.closed {
  transform: rotate(-90deg);
}
.expand:hover .chev {
  color: var(--n-l2);
}
/* 档案提示圆点：品牌色（SVG 自带结构色），替代展开箭头 */
.archive-dot-slot {
  color: var(--n-brand);
}
/* 内容行：图标 + 名称 + 数量（底色在 item 卡片上，本行透明） */
.body {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  flex: 1;
  min-width: 0;
  color: var(--n-l2);
  text-align: left;
}
/* 装饰图标 18px 常态档（figma：图标 svg = 文字/常态） */
.deco {
  color: var(--n-l2);
  flex: none;
}
.label {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 命名态：整 item 2px 品牌/常态描边（2026-09-20 用户定 2px 档，对齐 TreeRowItem）；
   校验违规（重名等）整 item 描边 --n-danger 覆盖 */
.tree-item.naming {
  box-shadow: inset 0 0 0 2px var(--n-brand);
}
.tree-item.naming-error {
  box-shadow: inset 0 0 0 2px var(--n-danger);
}
/* 命名输入：透明底不单独描边（整 item 高亮提示，统一口径）；共享基座在 base.css（类名并入
   .rename-input，原 .name-input 作废），本地差异 = 弹性占满 + 常态字色 + 左对齐 */
.rename-input {
  flex: 1;
  min-width: 0;
  color: var(--n-l2);
  text-align: left;
}
/* 数量标签：标签档，右侧（figma 02_目录 item） */
.count {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  flex: none;
}
</style>
