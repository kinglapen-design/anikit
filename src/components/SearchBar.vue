<script setup lang="ts">
// 搜索框（1.2 重构，figma-css 01_主窗口 11648 搜索框段）：通栏 / C3 / 上边框 暗-悬停。
// 3.6 一轮点亮分词搜索（2026-08-19 用户定 tag 化交互）：
// - 输入区 = chips + 内联草稿（交互口径同 TermChipsInput：空格/回车提交 chip、IME 守卫、
//   空草稿 Backspace 删尾 chip、× 删、失焦提交草稿）；草稿未提交也即时参与搜索
// - 前缀 tag：`~` →「~ 全局搜索」（全局范围，无范围按钮——tag 即范围）、`//` →「// 搜索」（显式分词，
//   与直接输入等效）、`/` →「/ 筛选」（占位不参与搜索，3.6 二轮点亮）
// - kbd 徽章组 = 未聚焦且无内容时的提示；聚焦或有内容隐藏、输入区贯穿（用户定，类 ctrl+f 提示口径）
// - Ctrl+F 聚焦（useGlobalKeys → search.requestFocus）；搜索按钮 / 历史下拉无业务保持占位
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import AppIcon from "./AppIcon.vue";
import { useSearchStore } from "../stores/search";
import { useUiStore } from "../stores/ui";

const props = defineProps<{ panelId: string; viewKey: string }>();
const search = useSearchStore();
const ui = useUiStore();

// 初始回读（面板热重载 / 布局恢复残留不丢字）；chips 直接读 store（响应式数组引用）
const draft = ref(search.draftFor(props.panelId));
const inputRef = ref<HTMLInputElement>();
const focused = ref(false);

const chips = computed(() => search.chipsFor(props.panelId));

watch(draft, (v) => search.setDraft(props.panelId, props.viewKey, v));

// 视图 / 递归开关变化：搜索保持并按新范围重搜（2026-08-19 定案）
watch([() => props.viewKey, () => ui.showSubfolders], () => {
  search.rerun(props.panelId, props.viewKey);
});

// Ctrl+F 聚焦请求（requestFocus 记 panelId，仅本搜索框响应）
watch(
  () => search.focusSeq,
  () => {
    if (search.focusPanelId === props.panelId) {
      inputRef.value?.focus();
    }
  },
);

/** chip 显示文本：前缀 tag 带语义文案，关键字原样 */
function chipText(c: string): string {
  if (c === "~") return "~ 全局搜索";
  if (c === "//") return "// 搜索";
  if (c === "/") return "/ 筛选";
  return c;
}

/** 提交草稿为 chip（TermChipsInput 口径）：`~` / `//` 前缀剥落为独立 tag 后剩余为关键字
   （`~人物` →「~ 全局搜索」+「人物」，可叠加 `~//人物`）；纯 `/` = 筛选占位 tag（3.6 二轮点亮） */
function commit(): void {
  let v = draft.value.trim();
  if (!v) return;
  draft.value = "";
  for (;;) {
    if (v.startsWith("//")) {
      search.addChip(props.panelId, props.viewKey, "//");
      v = v.slice(2).trimStart();
    } else if (v.startsWith("~")) {
      search.addChip(props.panelId, props.viewKey, "~");
      v = v.slice(1).trimStart();
    } else break;
  }
  if (v === "/") {
    search.addChip(props.panelId, props.viewKey, "/");
    return;
  }
  if (v) search.addChip(props.panelId, props.viewKey, v);
}

function onKeydown(e: KeyboardEvent): void {
  if (e.isComposing) return; // IME 组合期：回车/空格是选词，不提交
  if (e.key === "Enter" || e.key === " ") {
    e.preventDefault();
    commit();
    return;
  }
  if (e.key === "Backspace" && draft.value === "" && chips.value.length > 0) {
    search.popChip(props.panelId, props.viewKey);
    return;
  }
  // Esc = 清空全部（chips + 草稿）并失焦
  if (e.key === "Escape") {
    draft.value = "";
    search.clear(props.panelId, props.viewKey);
    inputRef.value?.blur();
  }
}

function onBlur(): void {
  focused.value = false;
  commit(); // 失焦提交草稿（TermChipsInput 口径）
}

onMounted(() => {
  // 热重载 / 布局恢复：残留内容命中可能已失效，补搜一次恢复结果态
  search.rerun(props.panelId, props.viewKey);
});

onBeforeUnmount(() => search.removePanel(props.panelId));
</script>

<template>
  <div class="search-bar">
    <button class="search-btn" v-tip="'搜索'">
      <AppIcon name="search" :size="18" />
    </button>
    <!-- 输入区（chips + 草稿）：点击空白聚焦草稿框 -->
    <div class="chips-wrap" @click="inputRef?.focus()">
      <span v-for="(c, i) in chips" :key="i" class="chip">
        <span class="chip-text">{{ chipText(c) }}</span>
        <button class="chip-del" v-tip="'删除'" @click.stop="search.removeChip(props.panelId, props.viewKey, c)">
          <AppIcon name="clear" :size="12" />
        </button>
      </span>
      <input
        ref="inputRef"
        v-model="draft"
        class="draft-input"
        type="text"
        :placeholder="chips.length === 0 ? '/ 筛选看板    // 分词搜索    ~ 搜索全局' : ''"
        @keydown="onKeydown"
        @focus="focused = true"
        @blur="onBlur"
      />
    </div>
    <!-- 快捷键徽章组 = 未聚焦且无内容时的提示（聚焦 / 有内容隐藏，输入区贯穿） -->
    <div v-if="!focused && chips.length === 0 && !draft" class="kbd-group">
      <span class="kbd">/ 筛选</span>
      <span class="kbd">// 搜索</span>
      <span class="kbd">~ 全局</span>
    </div>
    <button class="drop-btn" v-tip="'搜索历史'">
      <AppIcon name="nav-history" :size="18" class="rot" />
    </button>
  </div>
</template>

<style scoped>
/* 总容器（figma 11648）：通栏 min-height 46 / padding 8 16 / 间距 6 / C3 / 上边框 暗-悬停；
   chips 换行时向上撑高（min-height，内容区 flex 自适应让位） */
.search-bar {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  padding: var(--space-8) var(--space-16);
  min-height: 46px;
  box-sizing: border-box;
  flex: none;
  background: var(--c3);
  border-top: 1px solid var(--line-hover);
}
/* 搜索按钮（figma 11675）：30×30 padding 6 圆角 6，图标标签档 */
.search-btn,
.drop-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  padding: var(--space-6);
  border-radius: var(--radius-6);
  color: var(--text-label);
  flex: none;
  transition: background 0.15s ease;
}
.search-btn:hover,
.drop-btn:hover {
  background: var(--line-hover);
}
/* 下拉按钮图标（figma 11752 整体 rotate 90°） */
.rot {
  transform: rotate(90deg);
}
/* 输入区：chips + 草稿横排换行，点击任意处聚焦草稿框 */
.chips-wrap {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex-wrap: wrap;
  flex: 1;
  min-width: 0;
  cursor: text;
}
/* 已提交 chip（TermChipsInput 口径：品牌色底 + 1px 亮-选中 + 高亮字 + × 删） */
.chip {
  display: inline-flex;
  align-items: center;
  gap: var(--space-6);
  height: 28px;
  box-sizing: border-box;
  padding: var(--space-4) var(--space-8);
  border-radius: var(--radius-6);
  background: var(--brand);
  box-shadow: inset 0 0 0 1px var(--line-active);
  color: var(--text-highlight);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  white-space: nowrap;
  flex: none;
}
.chip-del {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  color: var(--text-highlight);
  opacity: 0.7;
  transition: opacity 0.12s ease;
}
.chip-del:hover {
  opacity: 1;
}
/* 草稿输入：裸形（透明底无边框），高度与 chip 行平齐 */
.draft-input {
  flex: 1;
  min-width: 72px;
  height: 28px;
  background: transparent;
  color: var(--text-normal);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.draft-input::placeholder {
  color: var(--text-faint);
}
/* 快捷键徽章组（figma 11808 等）：高 25 / padding 4 8 / C3 + 暗-悬停边框 / 圆角 6 / 极淡档 */
.kbd-group {
  display: flex;
  align-items: center;
  gap: var(--space-6);
  flex: none;
}
.kbd {
  display: inline-flex;
  align-items: center;
  padding: var(--space-4) var(--space-8);
  height: 25px;
  box-sizing: border-box;
  background: var(--c3);
  border: 1px solid var(--line-hover);
  border-radius: var(--radius-6);
  font-size: var(--text-label-size);
  line-height: var(--text-label-line);
  color: var(--text-faint);
  white-space: nowrap;
}
</style>
