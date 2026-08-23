<script setup lang="ts">
// 封面设置组（43_封面设置稿，6.1；仅档案，2026-08-23 用户定：文件不做本面板）：
// SectionPinned 档案分支点击封面图展开（内联替换常态组），确认 / 取消退回常态。
// 结构（稿：容器 padding 16 / gap 12）：封面框 218×168（C2 / 1px 暗-悬停 / 圆角 8，
// 徽标 chip 左上 6,6 实时预览）→ 尺寸滑块（218×10，SideSlider 复用；未选新图禁用）
// → 更换封面钮（C5 通栏 38，挂 10 稿菜单 152 宽：从本地磁盘中选择（点亮）/
// 从资源库中选择（disabled——主窗口选择模式后置））→ 徽标行（输入框留空清除 + 颜色下拉，
// 点击展开 9 色格行：16 圆形色格与 pinned 颜色 bar 同尺寸，选中 2px 常态 border 同 FolderColorPanel 口径）
// → 确认（主左）/ 取消（次右）flex 1 38 高。
// 取景（2026-08-23 用户定：滑块缩放 + 拖动位移）：选新图后进入取景态——图按 cover 铺满 × scale
// 渲染并随拖动偏移（边缘 clamp 不露底），确认时 crop 随源图路径提交后端烘培（covers/ 独立存）。
import { computed, onUnmounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import AppButton from "../AppButton.vue";
import AppIcon from "../AppIcon.vue";
import AppInput from "../AppInput.vue";
import SideSlider from "../SideSlider.vue";
import { openContextMenu } from "../../lib/contextMenus";
import { CUSTOM_COLORS } from "../../lib/customColors";
import { setArchiveCoverSettings, type CropInput } from "../../lib/customDataOps";
import { useContentStore, type ArchiveDetailView, type FolderNode } from "../../stores/content";

// 封面框规格（43 稿 218×168；与后端烘培输出 416×320 同比例——crop 语义与像素无关）
const FRAME_W = 218;
const FRAME_H = 168;

const props = defineProps<{ node: FolderNode; data: ArchiveDetailView }>();
const emit = defineEmits<{ done: [] }>();
const content = useContentStore();

// —— 徽标草稿（输入框留空 = 清除徽标；颜色默认红色档 = 43 稿色块样例 --custom-red）——
const badgeText = ref(props.data.badge?.text ?? "");
const badgeColor = ref(props.data.badge?.color ?? CUSTOM_COLORS[0].hex);
const colorOpen = ref(false); // 颜色下拉展开态（9 色格行）

// —— 封面预览：当前封面（自定义 coverPath 优先，回退子项首图）或取景态新图 ——
const currentCover = computed(() => {
  const p = props.data.coverPath ?? content.folderThumbMap.get(props.node.id) ?? null;
  return p ? convertFileSrc(p) : null;
});
// 取景态：选中的本地图（sourcePath + 自然尺寸）与取景参数
const picked = ref<{ path: string; url: string; w: number; h: number } | null>(null);
const crop = ref<CropInput>({ x: 0, y: 0, scale: 1 });

// 滑块 = scale × 100（SideSlider 整数档 100–400 = 1.00–4.00 倍）
const scalePct = computed({
  get: () => Math.round(crop.value.scale * 100),
  set: (v: number) => setCrop({ scale: v / 100 }),
});

// 取景渲染：图显示尺寸 dw/dh = cover 铺满 × scale；中心 = 框中心 + (x·框宽, y·框高)
const viewStyle = computed(() => {
  const p = picked.value;
  if (!p) return null;
  const z0 = Math.max(FRAME_W / p.w, FRAME_H / p.h);
  const dw = p.w * z0 * crop.value.scale;
  const dh = p.h * z0 * crop.value.scale;
  return {
    width: `${dw}px`,
    height: `${dh}px`,
    left: `${FRAME_W / 2 + crop.value.x * FRAME_W - dw / 2}px`,
    top: `${FRAME_H / 2 + crop.value.y * FRAME_H - dh / 2}px`,
  };
});

// 取景参数更新（边缘 clamp：图片边缘不缩进框内，不露底）
function setCrop(patch: Partial<CropInput>): void {
  const p = picked.value;
  if (!p) return;
  const next = { ...crop.value, ...patch };
  next.scale = Math.max(1, next.scale);
  const z0 = Math.max(FRAME_W / p.w, FRAME_H / p.h);
  const mx = Math.max(0, (p.w * z0 * next.scale - FRAME_W) / 2 / FRAME_W);
  const my = Math.max(0, (p.h * z0 * next.scale - FRAME_H) / 2 / FRAME_H);
  next.x = Math.max(-mx, Math.min(mx, next.x));
  next.y = Math.max(-my, Math.min(my, next.y));
  crop.value = next;
}

// 拖动位移（window 监听 pointermove/up + buttons 活性检查自清，与拖拽引擎同口径——
// 不挂元素级 pointer capture：headless CDP 合成事件下捕获会吃掉位移，2026-08-23 探针实证）
let dragging = false;
let lastX = 0;
let lastY = 0;
function onFramePointerDown(e: PointerEvent): void {
  if (!picked.value || e.button !== 0) return;
  dragging = true;
  lastX = e.clientX;
  lastY = e.clientY;
  window.addEventListener("pointermove", onFrameDragMove);
  window.addEventListener("pointerup", onFrameDragEnd, { once: true });
  window.addEventListener("blur", onFrameDragEnd, { once: true });
  e.preventDefault();
}
function onFrameDragMove(e: PointerEvent): void {
  if (!dragging) return;
  if ((e.buttons & 1) === 0) {
    onFrameDragEnd(); // 僵尸自清（窗外松手等）
    return;
  }
  setCrop({ x: crop.value.x + (e.clientX - lastX) / FRAME_W, y: crop.value.y + (e.clientY - lastY) / FRAME_H });
  lastX = e.clientX;
  lastY = e.clientY;
}
function onFrameDragEnd(): void {
  dragging = false;
  window.removeEventListener("pointermove", onFrameDragMove);
  window.removeEventListener("pointerup", onFrameDragEnd);
  window.removeEventListener("blur", onFrameDragEnd);
}
onUnmounted(onFrameDragEnd); // 组件卸下（确认 / 取消 / 切目标）时清窗级监听

// 更换封面菜单（10 稿 152 宽两项；从资源库中选择 = 主窗口选择模式后置，disabled 占位）
function openCoverMenu(e: MouseEvent): void {
  openContextMenu(
    e,
    [
      { label: "从本地磁盘中选择", action: () => void pickLocal() },
      { label: "从资源库中选择", disabled: true },
    ],
    undefined,
    { width: 152 },
  );
}
async function pickLocal(): Promise<void> {
  const pickedPath = await open({
    multiple: false,
    filters: [{ name: "图片", extensions: ["png", "jpg", "jpeg", "bmp", "webp", "gif"] }],
  });
  if (typeof pickedPath !== "string") return;
  const url = convertFileSrc(pickedPath);
  // 读自然尺寸（cover 铺满基准）；加载失败按未选处理
  const dims = await new Promise<{ w: number; h: number } | null>((resolve) => {
    const img = new Image();
    img.onload = () => resolve({ w: img.naturalWidth, h: img.naturalHeight });
    img.onerror = () => resolve(null);
    img.src = url;
  });
  if (!dims || dims.w < 1 || dims.h < 1) return;
  picked.value = { path: pickedPath, url, ...dims };
  crop.value = { x: 0, y: 0, scale: 1 };
}

const busy = ref(false);
async function onConfirm(): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const text = badgeText.value.trim();
    const badge = text ? { text, color: badgeColor.value } : null;
    const cover = picked.value ? { sourcePath: picked.value.path, crop: crop.value } : undefined;
    if (await setArchiveCoverSettings(props.node.id, badge, cover)) emit("done");
  } finally {
    busy.value = false;
  }
}
function onCancel(): void {
  if (busy.value) return;
  emit("done");
}
</script>

<template>
  <!-- 43 稿容器：padding 16 / gap 12（250 面板内内容宽 218） -->
  <div class="cover-settings">
    <!-- 封面框（218×168 / C2 / 1px 暗-悬停 / 圆角 8）：取景态图可拖动，徽标 chip 左上实时预览 -->
    <div
      class="cs-frame"
      :class="{ framing: picked }"
      @pointerdown="onFramePointerDown"
    >
      <img v-if="picked" class="cs-view" :src="picked.url" :style="viewStyle ?? undefined" alt="" draggable="false" />
      <img v-else-if="currentCover" class="cs-still" :src="currentCover" alt="" draggable="false" />
      <span v-if="badgeText.trim()" class="cs-badge" :style="{ background: badgeColor }">{{ badgeText.trim() }}</span>
    </div>
    <!-- 尺寸滑块（稿 218×10）：未选新图禁用（取景参数只对新图生效——已烘培封面不重烘培防质量递减） -->
    <div class="cs-slider" :class="{ off: !picked }" v-tip="picked ? undefined : '先更换封面'">
      <SideSlider v-model="scalePct" :min="100" :max="400" />
    </div>
    <!-- 更换封面（稿 218×38 C5 通栏）：挂 10 稿菜单 -->
    <AppButton variant="sidebar" label="更换封面" class="cs-change" @click="openCoverMenu" />
    <!-- 徽标行（稿：标签 32 标签色 / 输入框 28 高留空清除 / 颜色下拉 57×28 C4 + 色块 + 展开箭头） -->
    <div class="cs-badge-row">
      <span class="cs-label">徽标:</span>
      <AppInput v-model="badgeText" size="small" placeholder="留空清除" class="cs-badge-input" />
      <span class="cs-label">颜色:</span>
      <button class="cs-color" :class="{ open: colorOpen }" @click="colorOpen = !colorOpen">
        <span class="sw" :style="{ background: badgeColor }" />
        <AppIcon name="tree-expand-arrow" :size="12" class="arrow" :class="{ up: colorOpen }" />
      </button>
    </div>
    <!-- 颜色格行（下拉展开：9 色 16 圆形格均分一行——与 pinned 颜色 bar 同尺寸，选中 2px 常态 border） -->
    <div v-if="colorOpen" class="cs-colors">
      <button
        v-for="c in CUSTOM_COLORS"
        :key="c.tint"
        class="cell"
        :class="{ cur: badgeColor === c.hex }"
        :style="{ background: `var(--custom-${c.tint})` }"
        :aria-label="c.label"
        @click="badgeColor = c.hex; colorOpen = false"
      />
    </div>
    <!-- 按钮行（稿：确认主左 / 取消次右，flex 1 各 103×38 gap 12） -->
    <div class="cs-actions">
      <AppButton variant="primary" label="确认" :disabled="busy" @click="onConfirm" />
      <AppButton variant="panel" label="取消" :disabled="busy" @click="onCancel" />
    </div>
  </div>
</template>

<style scoped>
.cover-settings {
  display: flex;
  flex-direction: column;
  gap: var(--space-12);
  padding: var(--space-16);
}
/* 封面框（43 稿：218×168 / C2 / 1px 暗-悬停 / 圆角 8） */
.cs-frame {
  position: relative;
  width: 218px;
  height: 168px;
  border-radius: var(--radius-8);
  background: var(--c2);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  overflow: hidden;
  flex: none;
}
/* 取景态：拖动位移手感 */
.cs-frame.framing {
  cursor: grab;
}
/* 取景图：绝对定位按取景参数渲染（width/height/left/top 由 viewStyle 给） */
.cs-view {
  position: absolute;
  max-width: none;
  max-height: none;
  user-select: none;
}
/* 常态封面：contain 不裁剪（pinned 口径） */
.cs-still {
  width: 100%;
  height: 100%;
  object-fit: contain;
  display: block;
}
/* 徽标 chip（43 稿样例：padding 2 4 / 圆角 4 / 徽标档 10/14 700 常态色，与 pinned fmt-badge 同式） */
.cs-badge {
  position: absolute;
  top: 6px;
  left: 6px;
  max-width: calc(100% - 12px);
  padding: 2px var(--space-4);
  border-radius: var(--radius-4);
  font-size: var(--text-badge-size);
  line-height: var(--text-badge-line);
  font-weight: var(--text-badge-weight);
  color: var(--text-normal);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  pointer-events: none;
}
/* 尺寸滑块行（稿 218×10）：SideSlider 宽覆盖 218；禁用 = 压暗 + 不响应 */
.cs-slider {
  display: flex;
  align-items: center;
  width: 218px;
  height: 10px;
  flex: none;
}
.cs-slider :deep(.side-slider) {
  width: 218px;
}
.cs-slider.off {
  opacity: 0.5;
  pointer-events: none;
}
/* 更换封面钮（稿：通栏 218×38 C5 = AppButton sidebar） */
.cs-change {
  width: 218px;
  height: 38px;
}
/* 徽标行（稿：28 高 gap 8；标签 14/20 标签色） */
.cs-badge-row {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  width: 218px;
  height: 28px;
  flex: none;
}
.cs-label {
  width: 32px;
  flex: none;
  color: var(--text-label);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.cs-badge-input {
  flex: 1;
  min-width: 0;
}
/* 颜色下拉框（稿：57×28 C4 / 1px 暗-悬停 / 圆角 6；色块 17×17 圆角 4 + 展开箭头 20×20） */
.cs-color {
  display: flex;
  align-items: center;
  gap: var(--space-8);
  width: 57px;
  height: 28px;
  box-sizing: border-box;
  padding: 4px 4px 4px 8px;
  border-radius: var(--radius-6);
  background: var(--c4);
  box-shadow: inset 0 0 0 1px var(--line-hover);
  flex: none;
}
.cs-color .sw {
  width: 17px;
  height: 17px;
  border-radius: var(--radius-4);
  flex: none;
}
/* 展开箭头（库内下拉统一 tree-expand-arrow 12 标签档，ViewOptionsMenu picker 同口径），展开时朝上 */
.cs-color .arrow {
  color: var(--text-label);
  flex: none;
}
.cs-color .arrow.up :deep(svg) {
  transform: rotate(180deg);
}
/* 颜色格行（9 色 16 圆形格均分一行；选中 2px 常态 border 内缩——FolderColorPanel 选中口径） */
.cs-colors {
  display: flex;
  justify-content: space-between;
  width: 218px;
  flex: none;
}
.cs-colors .cell {
  width: 16px;
  height: 16px;
  box-sizing: border-box;
  border-radius: 50%;
}
.cs-colors .cell.cur {
  border: 2px solid var(--text-normal);
}
/* 按钮行（稿：gap 12，两钮 flex 1 各 103×38） */
.cs-actions {
  display: flex;
  gap: var(--space-12);
  width: 218px;
  flex: none;
}
.cs-actions :deep(.app-btn) {
  flex: 1;
  height: 38px;
}
</style>
