<script setup lang="ts">
// 文件夹卡片——docs/06_UI重构/Design Token/组件/卡片/文件夹卡片.md：
// 通用三级底 + 立体感-通用 + 圆角 8 + padding 12；hover 仅底升通用六级（稿只给 background-color，
// 无位移无阴影——与媒体卡不同）；选中 2px 品牌描边（cards.css .kc-sel）。
// 顶行 = 名称（标题档 一级，截断）+ 文件夹-大双色调图标 24——**图标恒默认色不接 tint**
//（2026-08-29 用户定：卡片变色整屏噪声太大，自定义色只留树容器）；
// 简介 = 两行统计（数字档 亮色/三级，段间 " / " 分隔 gap 4）；名称行与简介间距 36（稿值，填满 122 高链）。
// 业务态（7.1 内容容器轮接线，模式同 items/TreeRowItem）：naming = 行内重命名输入态
//（useInlineRename，速查 §4；空/不变判定在父级，nameCommit 上抛）；拖拽源 dimmed / 剪切 cut =
// 整卡半透明 0.4 / 0.5（速查 §5 既有档；dimmed 复用：ws 断链文件夹整卡置灰同档，junction 方向 b
// 2026-10-02）；链接门徽章 badge（2026-10-02）= 10px 图标叠 fd-head 文件夹图标右下角（挂链 =
// linked-file-badge 三级 / 断链 = warning-small + dimmed）；拖拽并入 dropInto = 品牌/半透明填充 +
// 2px 品牌/常态描边（::after 伪元素，速查 §7 / §20 口径）。名称双击 = nameEdit（父级进命名态）；
// 悬停 tip = 名称 + 简介行。
import { computed } from "vue";
import AppIcon from "../AppIcon.vue";
import TwoToneIcon from "../items/TwoToneIcon.vue";
import { useInlineRename } from "../../lib/useInlineRename";
import "./cards.css";

const props = withDefaults(
  defineProps<{
    name: string;
    lines?: string[][]; // 统计行（每行若干段，段间 " / "）
    selected?: boolean;
    dimmed?: boolean; // 拖拽源半透明（0.4）；ws 断链文件夹置灰复用本档（junction 方向 b，2026-10-02）
    cut?: boolean; // 剪切半透明（0.5）
    naming?: boolean; // 行内重命名输入态
    dropInto?: boolean; // 拖拽并入落点（父级按引擎判定算好传入）
    badge?: string; // 文件夹图标右下角 10px 徽章图标名（junction 方向 b，2026-10-02：linked-file-badge = 挂链 / warning-small = 断链）
    forceState?: "" | "hover" | "selected";
  }>(),
  { lines: () => [], selected: false, dimmed: false, cut: false, naming: false, dropInto: false, badge: "", forceState: "" },
);

const emit = defineEmits<{
  nameCommit: [name: string];
  nameEdit: []; // 名称双击 = 进命名态（父级置 naming）
}>();

// 行内重命名（useInlineRename 封装：词干选区不包后缀 / blur 恒提交 / Enter/Esc 语义）——
// 空或不变直接退出不动、改名落 renameFolder 命令，判定在父级（TreeRowItem 同口径）
const { draft: nameDraft, bindInput, commit: commitName } = useInlineRename({
  active: () => props.naming,
  initial: () => props.name,
  escSaves: true,
  onCommit: (name) => emit("nameCommit", name),
});

// 名称悬停 tip（2026-08-10 用户定口径沿用）：完整名称 + 简介档（统计行拼一行，column 竖排）
const nameTip = computed(() => {
  const meta = props.lines.map((l) => l.join(" / ")).join(" / ");
  return { text: props.name, keys: meta || undefined, column: true };
});
</script>

<template>
  <div
    class="kc-sel folder-card"
    :class="[{ selected, dimmed, cut, naming, 'drop-into': dropInto }, forceState ? `force-${forceState}` : '']"
  >
    <div class="fd-head">
      <!-- 命名态：input 替换名称位（整卡品牌描边提示，输入框透明底，速查 §4 统一口径）；
           命名中 dblclick 不冒泡（防触发卡片双击"进入"） -->
      <input
        v-if="naming"
        :ref="bindInput"
        v-model="nameDraft"
        class="rename-input"
        @keydown.enter.prevent="commitName"
        @keydown.esc.prevent="commitName"
        @blur="commitName"
        @click.stop
        @dblclick.stop
        @pointerdown.stop
      />
      <span v-else class="fd-name" v-tip="nameTip" @dblclick.stop="emit('nameEdit')">{{ name }}</span>
      <span class="fd-fico">
        <TwoToneIcon name="folder-large" :size="24" />
        <!-- 链接门徽章（junction 方向 b，2026-10-02）：10px 叠图标右下角；warning-small = 双色调
             结构图标走 TwoToneIcon 变量通道，单灰线条徽章（linked-file-badge）走 AppIcon currentColor -->
        <span v-if="badge" class="fd-badge" aria-hidden="true">
          <TwoToneIcon v-if="badge === 'warning-small'" :name="badge" :size="10" />
          <AppIcon v-else :name="badge" :size="10" />
        </span>
      </span>
    </div>
    <div class="fd-stats">
      <div v-for="(line, i) in lines" :key="i" class="fd-line">
        <template v-for="(seg, j) in line" :key="j">
          <span v-if="j > 0" class="fd-sep">/</span>
          <span>{{ seg }}</span>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.folder-card {
  box-sizing: border-box;
  position: relative;
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  gap: var(--n-space-36);
  background: var(--n-g3);
  box-shadow: var(--n-effect-raised);
  border-radius: var(--n-radius-8);
  transition: background var(--n-dur-150) var(--n-ease-in-out);
}
.folder-card:hover,
.folder-card.force-hover {
  background: var(--n-g6);
}

.fd-head {
  display: flex;
  flex-direction: row;
  align-items: center;
  justify-content: space-between; /* 图标恒右缘（名称收缩后站位不变） */
  gap: var(--n-space-8);
}
/* 文件夹图标盒（badge 锚点）：尺寸同 TwoToneIcon 24 盒 */
.fd-fico {
  position: relative;
  display: inline-flex;
  width: 24px;
  height: 24px;
  flex: none;
}
/* 链接门徽章（junction 方向 b，2026-10-02）：10px 叠文件夹图标右下角（无稿零件最小方案——
   尺寸 10 / 右下 0 偏移待用户实机走查）；色档 = 亮色/三级（卡片纯底，行类徽章 §57 同档）；
   断链卡置灰由调用方传 dimmed（整卡 0.4） */
.fd-badge {
  position: absolute;
  right: 0;
  bottom: 0;
  width: 10px;
  height: 10px;
  display: inline-flex;
  pointer-events: none;
  color: var(--n-l3);
}
/* 名称收缩到文本宽（2026-10-02 用户实机定，§64）：双击重命名热区 = 真实标题文本区域，
   行空白双击 = 冒泡卡片「进入」不误重命名；flex 0 1 auto + max-width 保长名截断 */
.fd-name {
  flex: 0 1 auto;
  min-width: 0;
  max-width: 100%;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 名称区悬停文本光标（双击可改名） */
.fd-name:hover {
  cursor: text;
}
/* 命名输入：透明底不单独描边（整卡品牌环提示）；共享基座 base.css .rename-input，
   本地覆盖 = 标题档 18/26 与名称视觉连续 */
.rename-input {
  flex: 1;
  min-width: 0;
  height: 26px;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
  text-align: left;
}
/* 命名态整卡品牌描边（复用 .kc-sel 选中环层，opacity 恒亮） */
.folder-card.naming::before {
  opacity: 1;
}

/* 简介统计：两行 gap 4，行内段间 " / " gap 4，数字档 亮色/三级 */
.fd-stats {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.fd-line {
  display: flex;
  flex-direction: row;
  align-items: center;
  gap: var(--n-space-4);
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 拖拽源 / 剪切：整卡半透明（速查 §5 既有档 0.4 / 0.5） */
.folder-card.dimmed {
  opacity: 0.4;
}
.folder-card.cut {
  opacity: 0.5;
}
/* 拖拽并入：品牌/半透明填充 + 2px 品牌/常态描边（::after 伪元素，速查 §7 / §20 口径） */
.folder-card.drop-into {
  background: var(--n-brand-50);
}
.folder-card.drop-into::after {
  content: "";
  position: absolute;
  inset: 0;
  border: 2px solid var(--n-brand);
  border-radius: var(--n-radius-8);
  pointer-events: none;
}
</style>
