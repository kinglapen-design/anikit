<script setup lang="ts">
// 关联 / 副本弹窗（6.1 C5，2026-07-28 用户定案；骨架 / 样式抄 DuplicateDialog）：
// 触发源 = Ctrl+C→Ctrl+V（copy）/ Ctrl+拖拽落文件夹（C4d）/ 卡片右键菜单「粘贴」（C6）。
// 每行 = 源（缩略图 + 所在文件夹名 + 文件名）→ 目标（目标文件夹名 + 文件名输入框，
// 默认填源名且聚焦全选）+「关联 / 副本」互斥单选（默认关联）+ 行尾 ×（踢出该行，行空 = 等同取消）。
// 确认 = 剩余行按选择分两组调 link_placements / duplicate_placements（names 逐行传，空 = 默认名）；
// 取消 / 右上角关闭 = 全部放弃。状态挂 selection store linkDraft，宿主 App.vue。
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppCheckbox from "./AppCheckbox.vue";
import IconButton from "./IconButton.vue";
import { duplicatePlacements, linkPlacements, type PlacementRef } from "../lib/placementOps";
import { useContentStore } from "../stores/content";
import { useSelectionStore } from "../stores/selection";

const content = useContentStore();
const selection = useSelectionStore();

interface LinkRow {
  key: string;
  ref: PlacementRef;
  name: string; // 源文件名
  folderName: string; // 源所在文件夹名
  thumbUrl: string | null;
  draft: string; // 目标文件名（默认源名）
  action: "link" | "duplicate";
}

// 行初始化：三元组回查 PlacementView 取显示数据；陈旧项（已删 / 移动后失配）剔除
const rows = ref<LinkRow[]>([]);
const targetFolderName = ref("根目录");
const busy = ref(false);

onMounted(() => {
  const draft = selection.linkDraft;
  if (!draft) return;
  targetFolderName.value = content.folderName(draft.targetFolder) ?? "根目录";
  for (const ref of draft.items) {
    const p = content.placements.find(
      (v) => v.assetId === ref.assetId && v.folderId === ref.folderId,
    );
    if (!p) continue;
    rows.value.push({
      key: `${ref.assetId}|${ref.folderId}`,
      ref,
      name: p.name,
      folderName: content.folderName(p.folderId) ?? "根目录",
      thumbUrl: p.thumbPath ? convertFileSrc(p.thumbPath) : null,
      draft: p.name,
      action: "link",
    });
  }
  if (rows.value.length === 0) {
    selection.linkDraft = null; // 全部陈旧 = 等同取消
    return;
  }
  void nextTick(() => {
    const el = document.querySelector<HTMLInputElement>(".link-dialog .name-input");
    el?.focus();
    el?.select();
  });
  window.addEventListener("keydown", onKey, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKey, true);
});

function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    onCancel();
  }
}

function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toUpperCase() : "";
}

const total = computed(() => rows.value.length);

// 行尾 ×：踢出该行；行空 = 等同取消（2026-07-28 定案）
function removeRow(key: string): void {
  rows.value = rows.value.filter((r) => r.key !== key);
  if (rows.value.length === 0) selection.linkDraft = null;
}

function onCancel(): void {
  selection.linkDraft = null;
}

// 确认：按行选择分两组，一次执行（关联组 / 副本组各调一次，names 逐行传，空 = 默认名）
async function onConfirm(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const draft = selection.linkDraft;
    selection.linkDraft = null;
    if (!draft) return;
    const linkRows = rows.value.filter((r) => r.action === "link");
    const dupRows = rows.value.filter((r) => r.action === "duplicate");
    if (linkRows.length > 0) {
      await linkPlacements(
        linkRows.map((r) => r.ref),
        draft.targetFolder,
        linkRows.map((r) => r.draft.trim() || null),
      );
    }
    if (dupRows.length > 0) {
      await duplicatePlacements(
        dupRows.map((r) => r.ref),
        draft.targetFolder,
        dupRows.map((r) => r.draft.trim() || null),
      );
    }
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <div class="link-mask">
    <div class="link-card link-dialog">
      <!-- 标题栏（同 DuplicateDialog：高 44 / 左对齐标题 / 右侧关闭 = 取消） -->
      <header class="link-titlebar">
        <h2 class="link-title">粘贴为关联 / 副本（{{ total }}）</h2>
        <IconButton name="window-close" title="取消" @click="onCancel" />
      </header>

      <!-- 行清单：源 → 目标 + 关联/副本单选 + × -->
      <div class="rows">
        <div v-for="r in rows" :key="r.key" class="row">
          <div class="side source">
            <div class="thumb-box">
              <img v-if="r.thumbUrl" :src="r.thumbUrl" :alt="r.name" draggable="false" />
              <span v-else class="no-img">{{ extOf(r.name) }}</span>
            </div>
            <div class="texts">
              <span class="folder">{{ r.folderName }}</span>
              <span class="name" v-tip="r.name">{{ r.name }}</span>
            </div>
          </div>
          <span class="arrow">→</span>
          <div class="side target">
            <div class="texts">
              <span class="folder">{{ targetFolderName }}</span>
              <input
                v-model="r.draft"
                class="name-input"
                type="text"
                placeholder="留空默认源名"
                @focus="($event.target as HTMLInputElement).select()"
              />
            </div>
            <div class="radios">
              <AppCheckbox
                tone="dialog"
                label="关联"
                :model-value="r.action === 'link'"
                @update:model-value="r.action = 'link'"
              />
              <AppCheckbox
                tone="dialog"
                label="副本"
                :model-value="r.action === 'duplicate'"
                @update:model-value="r.action = 'duplicate'"
              />
            </div>
          </div>
          <IconButton name="window-close" small title="移除这项" @click="removeRow(r.key)" />
        </div>
      </div>

      <!-- 底栏：取消 / 确认（主按钮） -->
      <footer class="link-footer">
        <button class="btn secondary" :disabled="busy" @click="onCancel">取消</button>
        <button class="btn primary" :disabled="busy || total === 0" @click="onConfirm">确认</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩（同 DuplicateDialog）：C1-80 */
.link-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器：C3 / 1px border-1（inset）/ 圆角 8 / --shadow-pop（同 DuplicateDialog 口径） */
.link-card {
  box-sizing: border-box;
  width: 720px;
  max-height: 80%;
  display: flex;
  flex-direction: column;
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
  overflow: hidden;
}
/* 标题栏：高 44 / 下分割线 border-1（inset，规范禁 border） */
.link-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-16);
  height: 44px;
  flex: none;
  box-shadow: inset 0 -1px 0 0 var(--border-1);
}
.link-title {
  margin: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
/* 行清单：超出滚动；行间距 8 */
.rows {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--space-8);
  padding: var(--space-16) var(--space-32);
}
.row {
  display: flex;
  align-items: center;
  gap: var(--space-16);
}
.side {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  flex: 1;
  min-width: 0;
}
/* 源缩略图：48×48 方盒 contain（列表行同口径：不裁切） */
.thumb-box {
  width: 48px;
  height: 48px;
  flex: none;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: var(--radius-6);
  background: var(--c1);
  box-shadow: inset 0 0 0 1px var(--border-2);
  overflow: hidden;
}
.thumb-box img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
.no-img {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
}
.texts {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  flex: 1;
  min-width: 0;
}
.folder {
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--font-icon-3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.arrow {
  flex: none;
  color: var(--font-icon-3);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
/* 目标侧：文件夹名 + 文件名输入框（03_输入框#01 通用口径）+ 互斥单选 */
.name-input {
  width: 100%;
  height: 32px;
  box-sizing: border-box;
  padding: var(--space-6) var(--space-8);
  border: none;
  outline: none;
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
  font-family: var(--font-family);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-1);
}
.name-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.name-input::placeholder {
  color: var(--font-icon-placeholder);
}
.radios {
  display: flex;
  align-items: center;
  gap: var(--space-14);
  flex: none;
}
/* 底栏：高 70 / 上分割线 border-1（inset） */
.link-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-16);
  padding: 0 var(--space-32);
  height: 70px;
  flex: none;
  box-shadow: inset 0 1px 0 0 var(--border-1);
}
/* 按钮（01_按钮#07 大标题按钮-小按钮；主要 / 次要） */
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: var(--space-8) var(--space-16);
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c5);
}
.btn.secondary:hover {
  background: var(--border-1);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
