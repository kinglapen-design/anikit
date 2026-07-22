<script setup lang="ts">
// 重复添加提示（03_UI 布局/10_重复添加提示）：导入完成后逐个询问 hash 冲突项。
// 左：已存在（库内）/ 右：新的文件（默认 50% 透明）；选项：用已存在文件导入 / 保留两个 / 关联文件夹；
// "全部应用"开关把当前选项批量应用到剩余全部；关闭 = 跳过当前（不导入，源文件保留），自动弹下一个。
import { computed, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppCheckbox from "./AppCheckbox.vue";
import IconButton from "./IconButton.vue";
import { useContentStore, type ConflictResolution } from "../stores/content";

const content = useContentStore();

const current = computed(() => content.importConflicts?.[0] ?? null);
const total = computed(() => content.importConflicts?.length ?? 0);

const action = ref<"useExisting" | "copy" | "link">("useExisting");
const linkName = ref("");
const applyAll = ref(false);
const seqStart = ref("1");
const busy = ref(false);
const imgError = ref(false);

// 每换一项重置选择状态
watch(
  () => current.value?.sourcePath,
  () => {
    action.value = "useExisting";
    linkName.value = "";
    applyAll.value = false;
    seqStart.value = "1";
    imgError.value = false;
  },
);

const IMAGE_EXTS = new Set(["png", "jpg", "jpeg", "webp", "gif", "bmp"]);
function extOf(name: string): string {
  const i = name.lastIndexOf(".");
  return i >= 0 ? name.slice(i + 1).toLowerCase() : "";
}
function stemOf(name: string): string {
  return name.replace(/\.[^.]+$/, "");
}

// 新文件预览：图片格式直读源文件（asset 协议），非图片显示格式占位
const sourceUrl = computed(() => {
  const c = current.value;
  if (!c || imgError.value || !IMAGE_EXTS.has(extOf(c.sourceName))) return null;
  return convertFileSrc(c.sourcePath);
});
const existingUrl = computed(() =>
  current.value?.existingThumbPath ? convertFileSrc(current.value.existingThumbPath) : null,
);

function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 ** 2) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 ** 3) return `${(bytes / 1024 ** 2).toFixed(1)} MB`;
  return `${(bytes / 1024 ** 3).toFixed(2)} GB`;
}
// 尺寸 / 文件大小；没有尺寸就只显示文件大小（10_重复添加提示）
function metaText(w: number | null, h: number | null, size: number): string {
  const s = formatSize(size);
  return w != null && h != null ? `${w} x ${h} / ${s}` : s;
}
// 链接显示名：用户输入不带扩展名时按源文件追加
function withExt(base: string, sourceName: string): string {
  const ext = extOf(sourceName);
  if (ext === "" || base.toLowerCase().endsWith(`.${ext}`)) return base;
  return `${base}.${ext}`;
}
// 批量命名（关联 + 全部应用）：基名-序号递增，序号取输入框起始值
function batchLinkName(base: string, sourceName: string, i: number, start: number): string {
  const ext = extOf(sourceName);
  const stem = stemOf(base || sourceName);
  return `${stem}-${start + i}${ext ? `.${ext}` : ""}`;
}

function buildResolutions(): ConflictResolution[] {
  const queue = content.importConflicts ?? [];
  const targets = applyAll.value ? queue : queue.slice(0, 1);
  const base = linkName.value.trim();
  const start = Number.parseInt(seqStart.value, 10) || 1;
  return targets.map((c, i) => ({
    sourcePath: c.sourcePath,
    existingAssetId: c.existingAssetId,
    targetFolder: c.targetFolder,
    action: action.value,
    linkName:
      action.value !== "link"
        ? null
        : applyAll.value
          ? batchLinkName(base, c.sourceName, i, start)
          : base
            ? withExt(base, c.sourceName)
            : null, // 未输入：后端用源文件原名
  }));
}

async function onConfirm(): Promise<void> {
  if (busy.value || !current.value) return;
  busy.value = true;
  try {
    const report = await content.resolveConflicts(buildResolutions());
    if (report.failed.length > 0) content.importFailed = report.failed;
    const queue = content.importConflicts ?? [];
    content.importConflicts = applyAll.value ? null : queue.slice(1);
    if (!content.importConflicts || content.importConflicts.length === 0) {
      content.importConflicts = null;
      await content.refresh();
    }
  } finally {
    busy.value = false;
  }
}

// 关闭 = 跳过当前（不导入，源文件保留），自动弹下一个（纯前端出队，后端无需动作）
function onSkip(): void {
  const queue = content.importConflicts ?? [];
  content.importConflicts = queue.length > 1 ? queue.slice(1) : null;
  if (!content.importConflicts) void content.refresh();
}
</script>

<template>
  <div v-if="current" class="dup-mask">
    <div class="dup-card">
      <!-- 标题栏（08_二级窗口#带关闭按钮标题栏）：标题居中 + 关闭（跳过当前） -->
      <header class="dup-titlebar">
        <h2 class="dup-title">重复文件</h2>
        <IconButton name="window-close" title="跳过这个文件" @click="onSkip" />
      </header>

      <!-- 图片对照区：已存在 / 新的文件 -->
      <div class="compare">
        <div class="side">
          <div class="img-box">
            <img v-if="existingUrl" :src="existingUrl" :alt="current.existingName" draggable="false" />
            <span v-else class="no-img">{{ current.existingExt.toUpperCase() }}</span>
            <span class="tag exists">已存在</span>
          </div>
          <span class="name">{{ current.existingName }}</span>
          <span class="meta">{{ metaText(current.existingW, current.existingH, current.existingSize) }}</span>
          <span class="folder">{{ current.existingFolderName }}</span>
        </div>
        <div class="side">
          <div class="img-box">
            <img
              v-if="sourceUrl"
              class="new-img"
              :class="{ solid: action === 'copy' }"
              :src="sourceUrl"
              :alt="current.sourceName"
              draggable="false"
              @error="imgError = true"
            />
            <span v-else class="no-img">{{ extOf(current.sourceName).toUpperCase() }}</span>
            <span class="tag new">新的文件</span>
          </div>
          <span class="name">{{ current.sourceName }}</span>
          <span class="meta">{{ metaText(current.sourceW, current.sourceH, current.sourceSize) }}</span>
          <span class="folder">{{ current.targetFolderName }}</span>
        </div>
      </div>

      <!-- 底栏：单选项组合 + 导入按钮组 -->
      <footer class="dup-footer">
        <div class="options">
          <AppCheckbox
            tone="dialog"
            label="用已存在文件导入"
            :model-value="action === 'useExisting'"
            @update:model-value="action = 'useExisting'"
          />
          <AppCheckbox
            tone="dialog"
            label="保留两个"
            :model-value="action === 'copy'"
            @update:model-value="action = 'copy'"
          />
          <AppCheckbox
            tone="dialog"
            label="关联文件夹"
            :model-value="action === 'link'"
            @update:model-value="action = 'link'"
          />
          <Transition name="fade">
            <input
              v-if="action === 'link'"
              v-model="linkName"
              class="name-input"
              type="text"
              placeholder="留空默认"
            />
          </Transition>
          <Transition name="fade">
            <span v-if="applyAll && action === 'link'" class="seq-label">序号</span>
          </Transition>
          <Transition name="fade">
            <input
              v-if="applyAll && action === 'link'"
              v-model="seqStart"
              class="name-input seq"
              type="text"
              inputmode="numeric"
            />
          </Transition>
        </div>
        <div class="actions">
          <!-- 全部应用（01_按钮#08 开关按钮 + 文字）：平移动画 -->
          <button class="apply-all" :class="{ on: applyAll }" @click="applyAll = !applyAll">
            <span class="switch"><span class="dot" /></span>
            <span class="apply-text">全部应用 ({{ total }})</span>
          </button>
          <button class="btn primary" :disabled="busy" @click="onConfirm">确认导入</button>
        </div>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩（同 LibrarySetupModal）：C1-80 */
.dup-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器（10）：1052×732 / C5 / 1px C4-15（inset）/ 圆角 8 / --shadow-pop */
.dup-card {
  box-sizing: border-box;
  width: 1052px;
  height: 732px;
  display: flex;
  flex-direction: column;
  background: var(--c5);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
  overflow: hidden;
}
/* 标题栏：高 44 / 内边距 8 8 8 16 / 下分割线 C4-7（inset，规范禁 border） */
.dup-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-16);
  height: 44px;
  flex: none;
  box-shadow: inset 0 -1px 0 0 var(--c4-7);
}
.dup-title {
  margin: 0; /* 左对齐（用户验收：模态窗标题不要居中） */
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
/* 图片对照区：内边距 16 / 间距 20 */
.compare {
  flex: 1;
  min-height: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 20px;
  padding: var(--space-16);
}
.side {
  width: 500px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-8);
}
/* 图片容器：500×500 内图片按比例（最大不超过 500） */
.img-box {
  position: relative;
  width: 500px;
  height: 500px;
  display: flex;
  align-items: center;
  justify-content: center;
}
.img-box img {
  max-width: 500px;
  max-height: 500px;
  border-radius: var(--radius-8);
  box-shadow: inset 0 0 0 1px var(--c4-15); /* 两侧图片统一带边框 */
}
/* 新的文件：图片 30% 透明（用户验收：50% 与已存在拉不开）+ 1px C4-15（选"保留两个"后 100%，透明度过渡动画） */
.new-img {
  opacity: 0.3;
  transition: opacity 0.3s ease;
}
.new-img.solid {
  opacity: 1;
}
/* 标签提示：内边距 2 4 / 圆角 6 / 图片上居中 */
.tag {
  position: absolute;
  left: 50%;
  top: 50%;
  transform: translate(-50%, -50%);
  padding: var(--space-2) var(--space-4);
  border-radius: var(--radius-6);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  color: var(--c4);
}
.tag.exists {
  background: var(--success);
}
.tag.new {
  background: var(--warning); /* 警告色（用户验收：与已存在成功色区分） */
}
.no-img {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4-40);
}
.name {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
  max-width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.meta {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
}
.folder {
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
  max-width: 100%;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 底栏：高 70 / 内边距 16 / 上分割线 C4-15（inset） */
.dup-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-16);
  padding: var(--space-16);
  height: 70px;
  flex: none;
  box-shadow: inset 0 1px 0 0 var(--c4-15);
}
/* 单选项组合：间距 14 */
.options {
  display: flex;
  align-items: center;
  gap: var(--space-14);
  min-width: 0;
}
/* 输入框（03_输入框#01 通用）：高 32 / 内边距 6 8 / C1-15 底 + 1px C4-15 / 圆角 8 */
.name-input {
  width: 111px;
  height: 32px;
  padding: var(--space-6) var(--space-8);
  border: none;
  outline: none;
  background: var(--c1-15);
  box-shadow: inset 0 0 0 1px var(--c4-15);
  border-radius: var(--radius-8);
  font-family: var(--font-family);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
}
.name-input:focus {
  box-shadow: inset 0 0 0 1px var(--brand-1);
}
.name-input.seq {
  width: 33px;
  text-align: center;
  padding: var(--space-6) 0;
}
.seq-label {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4);
}
/* 透明度出入动画（关联输入框 / 序号组） */
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.25s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
/* 导入按钮组：间距 16 */
.actions {
  display: flex;
  align-items: center;
  gap: var(--space-16);
  flex: none;
}
/* 全部应用：开关按钮 50×28（01_按钮#08）+ 文字；圆点平移动画 */
.apply-all {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  padding: 0;
}
.switch {
  box-sizing: border-box;
  display: flex;
  align-items: center;
  width: 50px;
  height: 28px;
  padding: var(--space-4);
  border-radius: 14px;
  background: var(--c4-15);
  transition: background 0.2s ease;
}
.dot {
  width: 20px;
  height: 20px;
  border-radius: 999px;
  background: var(--c4);
  transition: transform 0.2s ease;
}
.apply-all.on .switch {
  background: var(--brand-1);
}
.apply-all.on .dot {
  transform: translateX(22px);
}
.apply-text {
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--c4-40);
}
.apply-all.on .apply-text {
  color: var(--c4);
}
/* 确认导入（01_按钮#07 大标题按钮-小按钮 / 主要） */
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: 8px 16px;
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--c4);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn:disabled {
  cursor: default;
  opacity: 0.6;
}
</style>
