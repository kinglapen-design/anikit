<script setup lang="ts">
// MarkDown 工具栏（双态，2026-09-25 二轮走查改版）：
// 编辑态（editing=true）= 段落（H1-H6 / 正文 / 引用）/ 列表（无序 / 有序 / 任务）/ 插入（表格 / 代码块 /
// 行内代码块 / 插入链接 / 分割线）/ 格式（加粗 / 斜体 / 删除线）四组 + 视图组（眼睛 = 回预览 / 分屏）；
// 预览态（editing=false）= 仅视图组（铅笔 = 进编辑 / 分屏）——预览态纯阅读不主动进编辑（用户「只想复制文字」），
// 进编辑唯一入口 = 铅笔钮 / Ctrl+E（互切）。
// 图标 = 按钮族 IconButton bare 复用（18 实寸 亮色/三级，hover 升二级——稿 hover-按钮组口径）；
// 悬停 tip = IconButton title prop 走 v-tip。眼睛 / 分屏为库内暂代图标（view-options / collapse-sidebar，
// 待用户补专图标后换名）；组间 .fb-group 左分隔线（稿 border-left 方案）。
// 只出事件（action 语义名）；实际操作 = preview.requestMdAction → PreviewText 执行
// （lib/mdEdit 19 动作；edit / preview / split = 模式与分屏切换）。
// 根 mousedown.prevent = 点击不抢焦点（§36 搜索栏同口径）——编辑器 textarea 选区全程保持。
// ws 只读预览（2026-10-23 文档预览放行）：readonly = 编辑 / 分屏入口不出，视图组只留大纲钮。
import { computed, nextTick, watch } from "vue";
import IconButton from "../buttons/IconButton.vue";
import "./bars.css";

const props = withDefaults(defineProps<{ editing?: boolean; readonly?: boolean }>(), { editing: false, readonly: false });
const emit = defineEmits<{ action: [name: string]; "expanded-change": [v: boolean] }>();

// 双态组数增减 → 内容宽变（2026-09-25 三轮走查「工具栏没展开」根治）：复用搜索栏
// expanded-change 通道催 FloatBar 重测壳宽（probe max-content；根盒恒 100% 随壳，
// ResizeObserver 观测不到内容变化——必须由组件自知上报，RO 方案已撤）
watch(
  () => props.editing,
  async (v) => {
    await nextTick(); // 组渲染完成后上报（probe 量的是新内容）
    emit("expanded-change", v);
  },
);

interface MdIcon {
  name: string;
  action: string;
  tip: string;
}
const formatGroups: readonly MdIcon[][] = [
  // 段落
  [
    { name: "heading-1", action: "h1", tip: "标题 1" },
    { name: "heading-2", action: "h2", tip: "标题 2" },
    { name: "heading-3", action: "h3", tip: "标题 3" },
    { name: "heading-4", action: "h4", tip: "标题 4" },
    { name: "heading-5", action: "h5", tip: "标题 5" },
    { name: "heading-6", action: "h6", tip: "标题 6" },
    { name: "body-text", action: "body", tip: "正文" },
    { name: "quote", action: "quote", tip: "引用" },
  ],
  // 列表
  [
    { name: "list-unordered", action: "ul", tip: "无序列表" },
    { name: "list-ordered", action: "ol", tip: "有序列表" },
    { name: "task-list", action: "task", tip: "任务列表" },
  ],
  // 插入
  [
    { name: "table", action: "table", tip: "表格" },
    { name: "code-block", action: "code-block", tip: "代码块" },
    { name: "code-inline", action: "code-inline", tip: "行内代码" },
    { name: "insert-link", action: "link", tip: "插入链接" },
    { name: "divider", action: "divider", tip: "分割线" },
  ],
  // 格式
  [
    { name: "bold", action: "bold", tip: "加粗" },
    { name: "italic", action: "italic", tip: "斜体" },
    { name: "strikethrough", action: "strike", tip: "删除线" },
  ],
];

// 视图组（编辑↔预览互切 + 分屏 + 大纲）：图标按态切换（铅笔 = 去编辑 / 预览模式 = 回预览）；
// 预览模式 preview-mode / 分屏 edit-and-preview = 2026-09-26 用户补源专图标（十七批，
// 旧暂代 view-options / collapse-sidebar 换名清零，§60 暂代待办勾销）；
// 大纲 = 2026-10-15 大纲轮新增（PRD 04「md 大纲栏」，预览 / 编辑 / 分屏三态可开关）——
// 图标 collapse-sidebar 暂代占位（用户定「先随便找一个占位，到时候一起换」，待补源后换名，速查 §71）
const viewGroup = computed<MdIcon[]>(() => {
  // ws 只读预览（2026-10-23 放行定案）：编辑 / 分屏入口不出（写回磁盘链路另立），只留大纲钮
  if (props.readonly) return [{ name: "collapse-sidebar", action: "outline", tip: "大纲" }];
  return [
    props.editing
      ? { name: "preview-mode", action: "preview", tip: "预览（Ctrl+E）" }
      : { name: "pencil", action: "edit", tip: "编辑（Ctrl+E）" },
    { name: "edit-and-preview", action: "split", tip: "分屏编辑预览" },
    { name: "collapse-sidebar", action: "outline", tip: "大纲" },
  ];
});
const groups = computed<readonly MdIcon[][]>(() =>
  props.editing ? [...formatGroups, viewGroup.value] : [viewGroup.value],
);
</script>

<template>
  <div class="fb-mode fbm-root" @mousedown.prevent>
    <div v-for="(g, gi) in groups" :key="gi" :class="gi === 0 ? 'fbm-first' : 'fb-group'">
      <IconButton
        v-for="ic in g"
        :key="ic.action"
        :name="ic.name"
        variant="bare"
        :title="ic.tip"
        @click="emit('action', ic.action)"
      />
    </div>
  </div>
</template>

<style scoped>
.fbm-root {
  flex: none;
  width: 100%; /* 恒满壳宽（壳 = 唯一宽度动画源，FloatBar 探针实测内容宽） */
}
.fbm-first {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
</style>
