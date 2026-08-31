<script setup lang="ts">
// 封面设置组（7.1 二十三轮贴稿重构，稿 = 右侧边栏-详情面板-档案封面 展开态；仅档案，
// 2026-08-23 用户定：文件不做本面板）：SectionPinned 档案分支点击封面图展开（内联替换常态组），
// 确认 / 取消退回常态。
// 结构（稿：264×424 高链 = padding 12 + 封面 240 + 滑块 10 + 更换封面 34 + 输入框 34 + 按钮组 34）：
//   封面框 240×240（通用二级底 + 通用三级 1px / 圆角 8；徽标 FormatBadge 左上 12/12 实时预览）；
//   尺寸滑块 224×10 = BarSlider tone="size"（轨道 3px 通用三级 / 拖钮 10 无描边；未选新图禁用压暗 30%——
//     取景参数只对新图生效，已烘培封面不重烘培防质量递减）；
//   更换封面 = ActionButton solid md + edit-cover 图标（挂 10 稿菜单 152 宽：从本地磁盘中选择（点亮）/
//     从资源库中选择（disabled——主窗口选择模式后置））；
//   角标名输入框 = TextInput lg（placeholder「输入角标名...」，留空 = 清除徽标）——
//     徽标颜色选择 UI 随新稿移除（稿未画；badge.color 提交时沿现值，统一 FormatBadge 铬后视觉本就作废）；
//   按钮组 = 取消（solid 左）/ 确认（primary 右）flex 均分——2026-08-31 用户全局定案「取消左确认右」。
// 取景（2026-08-23 用户定：滑块缩放 + 拖动位移）：选新图后进入取景态——图按 cover 铺满 × scale
// 渲染并随拖动偏移（边缘 clamp 不露底），确认时 crop 随源图路径提交后端烘培（covers/ 独立存）。
import { computed, onUnmounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import ActionButton from "../buttons/ActionButton.vue";
import TextInput from "../inputs/TextInput.vue";
import BarSlider from "../bars/BarSlider.vue";
import FormatBadge from "../cards/FormatBadge.vue";
import { openContextMenu } from "../../lib/contextMenus";
import { CUSTOM_COLORS } from "../../lib/customColors";
import { setArchiveCoverSettings, type CropInput } from "../../lib/customDataOps";
import { useContentStore, type ArchiveDetailView, type FolderNode } from "../../stores/content";

// 封面框规格（新稿 240×240 方形；宽随栏内容宽 100%，取景数学用逻辑像素 240 归一——
// crop 语义与像素无关，后端烘培输出 416×320 口径不变）
const FRAME = 240;

const props = defineProps<{ node: FolderNode; data: ArchiveDetailView }>();
const emit = defineEmits<{ done: [] }>();
const content = useContentStore();

// —— 徽标草稿（输入框留空 = 清除徽标；颜色沿现值——新稿无颜色选择 UI，默认红色档语义保留）——
const badgeText = ref(props.data.badge?.text ?? "");
const badgeColor = ref(props.data.badge?.color ?? CUSTOM_COLORS[0].hex);

// —— 封面预览：当前封面（自定义 coverPath 优先，回退子项首图）或取景态新图 ——
const currentCover = computed(() => {
  const p = props.data.coverPath ?? content.folderThumbMap.get(props.node.id) ?? null;
  return p ? convertFileSrc(p) : null;
});
// 取景态：选中的本地图（sourcePath + 自然尺寸）与取景参数
const picked = ref<{ path: string; url: string; w: number; h: number } | null>(null);
const crop = ref<CropInput>({ x: 0, y: 0, scale: 1 });

// 滑块 = scale 1~4 ↔ BarSlider 0~1 归一化
const scaleRatio = computed({
  get: () => (crop.value.scale - 1) / 3,
  set: (r: number) => setCrop({ scale: 1 + Math.min(1, Math.max(0, r)) * 3 }),
});

// 取景渲染：图显示尺寸 dw/dh = cover 铺满 × scale；中心 = 框中心 + (x·框宽, y·框高)。
// 百分比表达（框实际像素随栏宽变，逻辑 240 仅作比例基准；拖动位移换算用 framePx 实测）
const viewStyle = computed(() => {
  const p = picked.value;
  if (!p) return null;
  const z0 = Math.max(FRAME / p.w, FRAME / p.h);
  const dw = (p.w * z0 * crop.value.scale) / FRAME;
  const dh = (p.h * z0 * crop.value.scale) / FRAME;
  return {
    width: `${dw * 100}%`,
    height: `${dh * 100}%`,
    left: `${(0.5 + crop.value.x - dw / 2) * 100}%`,
    top: `${(0.5 + crop.value.y - dh / 2) * 100}%`,
  };
});

// 取景参数更新（边缘 clamp：图片边缘不缩进框内，不露底）
function setCrop(patch: Partial<CropInput>): void {
  const p = picked.value;
  if (!p) return;
  const next = { ...crop.value, ...patch };
  next.scale = Math.max(1, next.scale);
  const z0 = Math.max(FRAME / p.w, FRAME / p.h);
  const mx = Math.max(0, (p.w * z0 * next.scale - FRAME) / 2 / FRAME);
  const my = Math.max(0, (p.h * z0 * next.scale - FRAME) / 2 / FRAME);
  next.x = Math.max(-mx, Math.min(mx, next.x));
  next.y = Math.max(-my, Math.min(my, next.y));
  crop.value = next;
}

// 拖动位移（window 监听 pointermove/up + buttons 活性检查自清，与拖拽引擎同口径——
// 不挂元素级 pointer capture：headless CDP 合成事件下捕获会吃掉位移，2026-08-23 探针实证）；
// 位移按框实际像素宽换算（getBoundingClientRect——栏宽可拖，逻辑 240 仅作比例基准）
let dragging = false;
let lastX = 0;
let lastY = 0;
let framePx = FRAME;
function onFramePointerDown(e: PointerEvent): void {
  if (!picked.value || e.button !== 0) return;
  framePx = (e.currentTarget as HTMLElement).getBoundingClientRect().width || FRAME;
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
  setCrop({ x: crop.value.x + (e.clientX - lastX) / framePx, y: crop.value.y + (e.clientY - lastY) / framePx });
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
  <!-- 新稿容器：padding 12 / gap 12（栏内容宽 240 随栏） -->
  <div class="cover-settings">
    <!-- 封面框（240×240 方形 / 通用二级底 + 通用三级 1px / 圆角 8）：取景态图可拖动，徽标左上 12/12 实时预览 -->
    <div
      class="cs-frame"
      :class="{ framing: picked }"
      @pointerdown="onFramePointerDown"
    >
      <img v-if="picked" class="cs-view" :src="picked.url" :style="viewStyle ?? undefined" alt="" draggable="false" />
      <img v-else-if="currentCover" class="cs-still" :src="currentCover" alt="" draggable="false" />
      <FormatBadge v-if="badgeText.trim()" class="cs-badge" :label="badgeText.trim()" />
    </div>
    <!-- 尺寸滑块（稿 224×10 居中 = BarSlider tone size；未选新图禁用压暗 30% 档，速查表 §5） -->
    <div class="cs-slider" :class="{ off: !picked }" v-tip="picked ? '' : '先更换封面'">
      <BarSlider v-model="scaleRatio" tone="size" />
    </div>
    <!-- 更换封面（稿 240×34 solid + edit-cover 图标）：挂 10 稿菜单 -->
    <ActionButton label="更换封面" icon="edit-cover" variant="solid" size="md" @click="openCoverMenu" />
    <!-- 角标名（稿 placeholder「输入角标名...」；留空 = 清除徽标） -->
    <TextInput v-model="badgeText" placeholder="输入角标名..." />
    <!-- 按钮组（取消左 / 确认右 flex 均分，2026-08-31 全局定案；忙态防重入在处理器内） -->
    <div class="cs-actions">
      <ActionButton label="取消" variant="solid" size="md" @click="onCancel" />
      <ActionButton label="确认" variant="primary" size="md" @click="onConfirm" />
    </div>
  </div>
</template>

<style scoped>
/* 容器（稿：padding 12 / gap 12 纵列——pinned 区块位，无 border-top） */
.cover-settings {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
}
/* 封面框（稿：240×240 方形随栏宽 / 通用二级底 + 通用三级 1px / 圆角 8） */
.cs-frame {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  overflow: hidden;
  flex: none;
}
/* 取景态：拖动位移手感 */
.cs-frame.framing {
  cursor: grab;
}
/* 取景图：绝对定位按取景参数渲染（width/height/left/top 由 viewStyle 以 % 给，随框实际像素缩放） */
.cs-view {
  position: absolute;
  max-width: none;
  max-height: none;
  user-select: none;
}
/* 常态封面：cover 充满（稿裁切语义）；铺满图自带圆角（圆角平滑三件套口径） */
.cs-still {
  width: 100%;
  height: 100%;
  object-fit: cover;
  display: block;
  border-radius: var(--n-radius-8);
}
/* 徽标（稿：左上 12/12；FormatBadge 统一铬） */
.cs-badge {
  position: absolute;
  top: var(--n-space-12);
  left: var(--n-space-12);
  pointer-events: none;
  max-width: calc(100% - 24px);
}
/* 尺寸滑块行（稿 224×10 居中）；禁用 = 压暗 30% 档 + 不响应 */
.cs-slider {
  width: 224px;
  height: 10px;
  align-self: center;
  flex: none;
}
.cs-slider.off {
  opacity: 0.3;
  pointer-events: none;
}
/* 按钮组（稿：gap 8 两钮均分；ActionButton md 宽 100% 由父定 + flex:none 覆盖为均分，
   同提示词按钮组口径） */
.cs-actions {
  display: flex;
  gap: var(--n-space-8);
}
.cs-actions :deep(.act-btn) {
  flex: 1;
  min-width: 0;
}
</style>
