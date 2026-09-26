<script setup lang="ts">
// 提示词 tab（7.1 二十三轮贴稿，稿 = 右侧边栏-详情面板-提示词；提示词轮接业务）：
// 提示词-区块（padding 12 / gap 12 / border-top 通用/三级）：
//   编辑框（240×232 / padding 12 / 通用二级底 + 通用三级 1px / 圆角 8）——内文 次要文字 12/16 四级，
//     悬浮复制钮（右上内缩 12，30×30 通用/半透明-2 + 立体感-通用 / 圆角 6 / copy 图标 18 三级）；
//   按钮组两行（gap 8）：行一三钮均分（保存提示词 save-prompt / 打开编辑器 editor / 删除提示词 trash）、
//     行二两钮均分（分析提示词 analyze-prompt / 指定提示词文件 set-prompt-file）——纯图标钮 =
//     buttons/IconButton solid（高 34 填充式 / 图标 18 恒二级 / hover 通用六级）。
// 业务（提示词轮，2026-09-05 走查三态定案）：五钮恒显按态 disabled（稿 prose「按上下文显示」定案 = disabled）——
//   未指定 = 分析（仅文件且 png/jpg/jpeg/webp）/ 指定 可用，保存 / 打开编辑器 / 删除 / 复制 禁用；
//   已扫描 = 保存（存为 md 并绑定）/ 删除（= 清空）/ 分析（重新分析）/ 指定 / 复制 可用，打开编辑器禁用；
//   已指定 = 保存（= 同步快照）/ 打开编辑器 / 删除（= 断开）/ 复制 可用，分析 / 指定禁用；
//   档案目标 = 恒无「分析提示词」钮（v-if 摘掉，同排「指定」grow 均分自然占满）。
// 展示语义：编辑框只读；已指定态实时经 read_text_asset 读源最新内容显示（invoke 失败 / 无 assetId
// 回落快照 content）；无 prompt 显示空 placeholder「提示词」。
// 显隐 = 信息查看管理面板眼睛（2026-09-26）：hidden 即整 tab 空白（单区块，允许全隐藏）。
import { computed, ref, watch } from "vue";
import IconButton from "../buttons/IconButton.vue";
import AppIcon from "../AppIcon.vue";
import { useContentStore } from "../../stores/content";
import { useUiStore } from "../../stores/ui";
import type { DetailTarget } from "../../lib/detailSections";
import type { PickerTarget } from "../../lib/relationOps";
import {
  analyzePrompt,
  assignViaPicker,
  clearPrompts,
  copyPromptText,
  isPromptAnalyzableExt,
  newPromptDocuments,
  openPromptEditor,
  promptStateOfBinding,
  refreshPromptSnapshot,
  type PromptTargetRef,
} from "../../lib/promptOps";
import { readTextAsset } from "../../lib/textOps";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const content = useContentStore();
const ui = useUiStore();

// 区块显隐（2026-09-26 信息查看管理面板）：单区块 hidden 即整 tab 空白（允许全隐藏，无空态文案）
const blockHidden = computed(() => ui.detailBlocks.prompt.hidden.includes("prompt"));

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

// 提示词绑定（detail / archive data 同径带出）与三态
const prompt = computed(() => (file.value ? file.value.detail.prompt : archive.value?.data.prompt) ?? null);
const state = computed(() => promptStateOfBinding(prompt.value));

// 命令目标引用（文件 = assetId；档案 = folderId）
const targetRef = computed<PromptTargetRef>(() =>
  file.value
    ? { kind: "asset", assetId: file.value.detail.assetId }
    : { kind: "archive", folderId: archive.value?.node.id ?? "" },
);

// —— 展示文本：已指定 = 实时读源（read_text_asset 失败回落快照）；否则快照 content；无 prompt = 空 ——
const displayText = ref("");
async function loadText(): Promise<void> {
  const t = props.target;
  const p = prompt.value;
  if (!p) {
    displayText.value = "";
    return;
  }
  if (p.assetId) {
    try {
      const r = await readTextAsset(p.assetId);
      if (props.target !== t || prompt.value !== p) return; // await 期间已切目标 / 换绑定，丢弃
      displayText.value = r.content;
      return;
    } catch {
      // 源不可读（回收站 / 已删）回落快照 content
    }
  }
  if (props.target !== t || prompt.value !== p) return;
  displayText.value = p.content;
}
// 目标 / 详情编辑落盘（detailRev）变化即重拉；immediate 承担挂载首拉
watch([() => props.target, () => content.detailRev], () => void loadText(), { immediate: true });

// —— 五钮按态可用性（恒显不增删；disabled = 仅去交互不改色，§15 口径）——
// 分析：档案恒无此钮（模板 v-if）；文件 = 已扫描（重新分析）恒可，未指定仅图片格式（png/jpg/jpeg/webp）
const canAnalyze = computed(() => {
  if (!file.value) return false;
  if (state.value === "scanned") return true;
  return state.value === "none" && isPromptAnalyzableExt(file.value.placement.ext);
});
const canAssign = computed(() => state.value !== "linked"); // 未指定 / 已扫描可指定
// 源文档活 placement（回收站 / 已删 = 无）——打开编辑器禁用兜底（动作内 openPromptEditor 再护一道）
const sourceLive = computed(() => {
  const id = prompt.value?.assetId;
  return !!id && content.placements.some((x) => x.assetId === id);
});
const canOpenEditor = computed(() => state.value === "linked" && sourceLive.value);
const canSave = computed(() => state.value !== "none"); // 已扫描 = 存为 md 并绑定；已指定 = 同步快照
const canDelete = computed(() => state.value !== "none"); // 已扫描 = 清空；已指定 = 断开
const canCopy = computed(() => displayText.value.length > 0);

// —— 动作 ——
function onSave(): void {
  if (state.value === "linked") {
    void refreshPromptSnapshot(targetRef.value); // 已指定 = 同步快照（重抓源内容）
    return;
  }
  // 已扫描 = 保存提示词为 .md 文档（快照内容写盘并绑定；newPromptDocuments 单目标自动打开编辑器）
  if (file.value) {
    const p = file.value.placement;
    void newPromptDocuments({ assets: [{ assetId: p.assetId, folderId: p.folderId }], archives: [] }, "md");
  } else if (archive.value) {
    void newPromptDocuments({ assets: [], archives: [archive.value.node.id] }, "md");
  }
}

function onOpenEditor(): void {
  void openPromptEditor(targetRef.value);
}

function onDelete(): void {
  // 已指定 = 断开提示词文档链接；已扫描 = 清空提示词——同命令 clear_*_prompt（断开/清空共用）
  const t = targetRef.value;
  const verb = state.value === "linked" ? ("断开提示词文档链接" as const) : ("清空提示词" as const);
  if (t.kind === "asset") {
    void clearPrompts({ assets: [{ assetId: t.assetId, folderId: file.value?.placement.folderId ?? "root" }], archives: [] }, verb);
  } else {
    void clearPrompts({ assets: [], archives: [t.folderId] }, verb);
  }
}

function onAnalyze(): void {
  if (file.value) void analyzePrompt([file.value.detail.assetId]);
}

function onAssign(): void {
  // 详情面板 = 单目标（batch = [origin]；多选批量归卡片右键菜单）
  const t = props.target;
  const origin: PickerTarget =
    t.kind === "file"
      ? { kind: "asset", id: t.detail.assetId, name: t.placement.name, ext: t.placement.ext, folderId: t.placement.folderId, origin: t.placement.origin }
      : { kind: "archive", id: t.node.id, name: t.node.name };
  assignViaPicker(origin, [origin]);
}

function onCopy(): void {
  void copyPromptText(displayText.value);
}
</script>

<template>
  <section v-if="!blockHidden" class="prompt-block">
    <!-- 编辑框（稿：isolate 供复制钮绝对定位）；只读展示，无 prompt = 空 placeholder「提示词」 -->
    <div class="editor">
      <textarea
        :value="displayText"
        class="area"
        placeholder="提示词"
        readonly
        spellcheck="false"
      />
      <button class="copy-btn" :disabled="!canCopy" v-tip="'复制提示词'" @click="onCopy">
        <AppIcon name="copy" :size="18" />
      </button>
    </div>
    <!-- 按钮组（稿：两行 gap 8，钮均分 flex 1）——恒显按态 disabled；档案无「分析」钮（指定 grow 均分占满） -->
    <div class="btn-row">
      <IconButton name="save-prompt" variant="solid" title="保存提示词" class="grow" :disabled="!canSave" @click="onSave" />
      <IconButton name="editor" variant="solid" title="打开编辑器" class="grow" :disabled="!canOpenEditor" @click="onOpenEditor" />
      <IconButton name="trash" variant="solid" title="删除提示词" class="grow" :disabled="!canDelete" @click="onDelete" />
    </div>
    <div class="btn-row">
      <IconButton v-if="file" name="analyze-prompt" variant="solid" title="分析提示词" class="grow" :disabled="!canAnalyze" @click="onAnalyze" />
      <IconButton name="set-prompt-file" variant="solid" title="指定提示词文件" class="grow" :disabled="!canAssign" @click="onAssign" />
    </div>
  </section>
</template>

<style scoped>
.prompt-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
}
/* 编辑框（稿：240×232 / padding 12 / 通用二级底 + 通用三级 1px / 圆角 8） */
.editor {
  position: relative;
  height: 232px;
  padding: var(--n-space-12);
  box-sizing: border-box;
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
/* 内文 = 次要文字 12/16 亮色/四级（稿占位语境；只读口径） */
.area {
  width: 100%;
  height: 100%;
  padding: 0;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  font-family: var(--n-font-family);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l4);
}
.area::placeholder {
  color: var(--n-l4);
}
/* 悬浮复制钮（稿：右上内缩 12 / 30×30 / 通用半透明-2 + 立体感-通用 / 圆角 6 / 图标 18 三级） */
.copy-btn {
  position: absolute;
  top: var(--n-space-12);
  right: var(--n-space-12);
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  border-radius: var(--n-radius-6);
  background: var(--n-g-a2);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.copy-btn:hover {
  color: var(--n-l2);
}
/* 按钮组（稿：gap 8；钮 = IconButton solid 填充式，均分 grow 1——组件宽 100% 由父容器定，
   行内各钮提权 flex 1 均分） */
.btn-row {
  display: flex;
  gap: var(--n-space-8);
}
.btn-row .grow {
  flex: 1;
  min-width: 0;
}
</style>
