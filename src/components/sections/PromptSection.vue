<script setup lang="ts">
// 提示词区块（基础信息 tab 区块，2026-09-26 提示词二轮挪入——原独占第三 tab 废除，稿 =
// 右侧边栏-详情面板-提示词 的编辑框/复制钮沿用；区块壳 = 基础信息 db-block 同款
// padding 12 / gap 12 / border-top 通用/三级，无标题行同 tab 其余区块）：
//   编辑框（240×232 / padding 12 / 通用二级底 + 通用三级 1px / 圆角 8）——内文 次要文字 12/16 四级，
//     悬浮复制钮（右上内缩 12，30×30 通用/半透明-2 + 立体感-通用 / 圆角 6 / copy 图标 18 三级）；
//   按钮组单行三钮均分（gap 8，IconButton solid 高 34 填充式 / 图标 18 恒二级）——
//     **两态口径（2026-09-26 用户定，废五钮稿）**：
//     无提示词 = 扫描（仅文件且 png/jpg/jpeg/webp 可用；档案恒无此钮）/ 新建（点击弹二级 =
//       新建 .md / 新建 .txt 提示词文档，右键菜单同两项）/ 指定（选择模式）；
//     有提示词 = 编辑 / 断开提示词链接 / 删除提示词文档——已指定：编辑 = 打开内置编辑器、
//       断开 = 解绑（快照随解绑清除）、删除 = 源文档丢回收站 + 解绑；已扫描（无源文档）：
//       编辑 = 先存为 .md 再开编辑器（PRD：分析的提示词必须先保存为 MD 才能用编辑器）、
//       断开 = 清空、删除禁用。
//   图标（2026-09-26 用户补源十八批）：扫描 = analyze-prompt / 新建 = add-new-prompt /
//     指定 = set-prompt-file / 编辑 = open-prompt-editor / 断开 = break-prompt-link / 删除 = trash。
// 展示语义：编辑框只读；已指定态实时经 read_text_asset 读源最新内容显示（invoke 失败 / 无 assetId
// 回落快照 content）；无 prompt 显示空 placeholder「提示词」。
import { computed, ref, watch } from "vue";
import IconButton from "../buttons/IconButton.vue";
import AppIcon from "../AppIcon.vue";
import { useContentStore } from "../../stores/content";
import type { DetailTarget } from "../../lib/detailSections";
import type { PickerTarget } from "../../lib/relationOps";
import { openContextMenu } from "../../lib/contextMenus";
import {
  analyzePrompt,
  assignViaPicker,
  clearPrompts,
  copyPromptText,
  deletePromptDocuments,
  isPromptAnalyzableExt,
  newPromptDocuments,
  openPromptEditor,
  promptStateOfBinding,
  type PromptTargets,
} from "../../lib/promptOps";
import { readTextAsset } from "../../lib/textOps";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const content = useContentStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

// 提示词绑定（detail / archive data 同径带出）与三态
const prompt = computed(() => (file.value ? file.value.detail.prompt : archive.value?.data.prompt) ?? null);
const state = computed(() => promptStateOfBinding(prompt.value));

// 命令目标集（详情面板 = 单目标；多选批量归卡片右键菜单）
const targets = computed<PromptTargets>(() =>
  file.value
    ? { assets: [{ assetId: file.value.detail.assetId, folderId: file.value.placement.folderId }], archives: [] }
    : { assets: [], archives: [archive.value?.node.id ?? ""] },
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

// —— 三钮按态可用性（两态三钮，恒显按态 disabled——disabled = 仅去交互不改色，§15 口径）——
// 扫描：档案恒无此钮（模板 v-if）；文件仅未指定态显示，png/jpg/jpeg/webp 可用
const canScan = computed(() => !!file.value && isPromptAnalyzableExt(file.value.placement.ext));
// 删除：仅已指定有源文档可删（已扫描 = 纯快照无文档，禁用）
const canDeleteDoc = computed(() => state.value === "linked");
const canCopy = computed(() => displayText.value.length > 0);

// —— 动作 ——
function onScan(): void {
  if (file.value) void analyzePrompt([file.value.detail.assetId]);
}

// 新建 = 点击弹二级（右键菜单「新建 .md/.txt 提示词文档」同两项；锚定按钮下缘，菜单栏主菜单同款开法）
function onNew(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  const rect = el.getBoundingClientRect();
  const t = targets.value;
  openContextMenu(
    { clientX: rect.left, clientY: rect.bottom + 4 },
    [
      { label: "新建 .md 提示词文档", icon: "markdown", action: () => void newPromptDocuments(t, "md") },
      { label: "新建 .txt 提示词文档", icon: "txt", action: () => void newPromptDocuments(t, "txt") },
    ],
    undefined,
    { trigger: el },
  );
}

function onAssign(): void {
  const t = props.target;
  const origin: PickerTarget =
    t.kind === "file"
      ? { kind: "asset", id: t.detail.assetId, name: t.placement.name, ext: t.placement.ext, folderId: t.placement.folderId, origin: t.placement.origin }
      : { kind: "archive", id: t.node.id, name: t.node.name };
  assignViaPicker(origin, [origin]);
}

function onEdit(): void {
  if (state.value === "scanned") {
    // 已扫描无源文档：先存为 .md 再开编辑器（PRD 口径；newPromptDocuments 单目标自动打开编辑器）
    void newPromptDocuments(targets.value, "md");
    return;
  }
  const t = props.target;
  void openPromptEditor(
    t.kind === "file" ? { kind: "asset", assetId: t.detail.assetId } : { kind: "archive", folderId: t.node.id },
  );
}

function onDisconnect(): void {
  // 已指定 = 断开提示词链接；已扫描 = 清空提示词——同命令 clear_*_prompt（label 分流撤销标签）
  void clearPrompts(targets.value, state.value === "linked" ? "断开提示词链接" : "清空提示词");
}

function onDeleteDoc(): void {
  void deletePromptDocuments(targets.value);
}

function onCopy(): void {
  void copyPromptText(displayText.value);
}
</script>

<template>
  <section class="prompt-block">
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
    <!-- 按钮组（两态三钮，均分 grow 1；图标暂代待用户补源） -->
    <div v-if="state === 'none'" class="btn-row">
      <IconButton
        v-if="file"
        name="analyze-prompt"
        variant="solid"
        title="扫描提示词"
        class="grow"
        :disabled="!canScan"
        @click="onScan"
      />
      <IconButton name="add-new-prompt" variant="solid" title="新建提示词文档" class="grow" @click="onNew" />
      <IconButton name="set-prompt-file" variant="solid" title="指定提示词文件" class="grow" @click="onAssign" />
    </div>
    <div v-else class="btn-row">
      <IconButton name="open-prompt-editor" variant="solid" title="编辑提示词" class="grow" @click="onEdit" />
      <IconButton name="break-prompt-link" variant="solid" title="断开提示词链接" class="grow" @click="onDisconnect" />
      <IconButton
        name="trash"
        variant="solid"
        title="删除提示词文档"
        class="grow"
        :disabled="!canDeleteDoc"
        @click="onDeleteDoc"
      />
    </div>
  </section>
</template>

<style scoped>
/* 区块壳 = 基础信息 db-block 同款（稿：padding 12 / gap 12 / border-top 通用/三级） */
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
