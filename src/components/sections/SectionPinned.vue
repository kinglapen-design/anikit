<script setup lang="ts">
// pinned 缩略图 / 档案封面区块（7.1 二十三轮贴稿重构，稿 = 右侧边栏-详情面板-缩略图 / 档案封面）：
// 恒顶（不参与 tab 切换）、无区块标题、无 border-top（详情面板首块）。
// 结构 = 缩略图容器 240×240（通用二级底 + 通用三级 1px / 圆角 8；图 contain 不裁剪——速查表口径）
//   + 格式徽标 FormatBadge（左上 12/12；静态图不出 = formatBadgeColor null 判定沿用）
//   + 颜色列表 ColorSwatchItem（真色 = detail.colors 前 7，空 = 7 占位块——6.1 口径；
//     仅 thumbKind=asset（有内容缩略图，含 fla 伴生 webp 代理）时显示——无内容缩略图的格式
//     （docx 等格式图标占位）不提色、整行隐藏，2026-09-23 用户定）。
// 名称 / 链接输入框已下移基础信息 tab 输入框区块（旧 pinned 四件口径作废）。
// 档案形态：封面容器同款 240×240（封面 cover 充满——稿裁切语义）+ 徽标 FormatBadge（badge.text）；
//   封面链 = 自定义封面 → 子项首图 → 默认封面图（2026-09-08 用户定，lib/archiveCover）；
//   hover = 纯黑压暗 50% + set-cover 图标居中 二级（稿 prose 1_hover-封面图）；
//   点击 = 内联展开封面设置组（CoverSettings，6.1 业务不变；设置组贴稿随本轮档案形态阶段）——
//   展开 / 收起 = pinned 根高度 250ms 缓入缓出补间（稿 prose「向下展开设置」，2026-08-31 用户定档）。
//   档案无颜色列表（稿未画）。
import { computed, nextTick, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import CoverSettings from "./CoverSettings.vue";
import FormatBadge from "../cards/FormatBadge.vue";
import ColorSwatchItem from "../items/ColorSwatchItem.vue";
import AppIcon from "../AppIcon.vue";
import { archiveCoverUrl } from "../../lib/archiveCover";
import { formatBadgeColor, isSequence } from "../../lib/formatBadge";
import type { DetailTarget } from "../../lib/detailSections";
import { useContentStore } from "../../stores/content";
import { usePickerStore } from "../../stores/picker";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "file" | "archive" }> }>();
const content = useContentStore();
const picker = usePickerStore();

const file = computed(() => (props.target.kind === "file" ? props.target : null));
const archive = computed(() => (props.target.kind === "archive" ? props.target : null));

// 封面设置编辑态（6.1，43 稿；仅档案）：点击封面图展开，确认 / 取消（done）退回常态；
// 切换展示目标（别的档案 / 文件）时复位退出。
// 封面选择模式回填（2026-09-08，picker cover 用途）：选择模式选定图片退模式后档案恢复选中，
// 草稿（coverDraft）匹配本档案 = 自动展开封面设置组并以该图进入取景态（CoverSettings initialSourcePath）
const coverEditing = ref(false);
const coverDraftSource = ref<string | null>(null);
watch(
  () => (archive.value ? archive.value.node.id : null),
  () => {
    coverEditing.value = false;
    const d = picker.coverDraft;
    if (d && archive.value && d.archiveId === archive.value.node.id) {
      picker.coverDraft = null; // 一次性消费
      coverDraftSource.value = d.sourcePath;
      coverEditing.value = true;
    } else {
      coverDraftSource.value = null;
    }
  },
  { immediate: true },
);

// 展开 / 收起动画（稿 prose「2_点击-封面图：向下展开设置」；250ms 缓入缓出 = --n-dur-250 +
// --n-ease-in-out，2026-08-31 用户定）：pinned 根高度 FLIP 补间（翻转前实测 → 新分支挂载后实测），
// overflow 锁裁剪呈「向下展开」观感；padding 不参与跳变（挂载点 pinned 恒 padding 12，
// CoverSettings 根不自持 padding——二十三轮双重 padding 24 走查 BUG 的修复口径）
const rootRef = ref<HTMLElement>();
let coverAnimTimer: ReturnType<typeof setTimeout> | undefined;
async function animatePinnedHeight(): Promise<void> {
  const root = rootRef.value;
  if (!root) return;
  const from = root.getBoundingClientRect().height; // pre flush：旧分支当帧实际高
  await nextTick(); // 等新分支挂载
  const to = root.getBoundingClientRect().height;
  if (Math.abs(to - from) < 1) return;
  root.style.overflow = "hidden";
  root.style.height = `${from}px`;
  void root.offsetHeight; // reflow：先绘制起点高，再起过渡（同帧写两值不动画）
  root.style.transition = "height 250ms cubic-bezier(.32,.08,.1,1)"; // JS 补间用不了 CSS 变量，字面量同步
  root.style.height = `${to}px`;
  let finished = false;
  const finish = (): void => {
    if (finished) return;
    finished = true;
    root.style.transition = "";
    root.style.height = "";
    root.style.overflow = "";
  };
  root.addEventListener(
    "transitionend",
    (e) => {
      if (e.propertyName === "height") finish();
    },
    { once: true },
  );
  if (coverAnimTimer) clearTimeout(coverAnimTimer);
  coverAnimTimer = setTimeout(finish, 290); // 兜底（display 切换等场景 transitionend 不发）
}
watch(coverEditing, () => void animatePinnedHeight());

const thumbUrl = computed(() => {
  const p = file.value?.placement.thumbPath;
  return p ? convertFileSrc(p) : null;
});
// 档案封面链（2026-09-08 用户定，lib/archiveCover）：自定义封面（cache/covers/ 烘培产物，
// ?v= 版本戳随覆盖重写重载）→ 子项第一张静态图（folderThumbMap 既有口径）→ 默认封面图
const archiveCover = computed(() => {
  const id = archive.value?.node.id;
  if (!id) return null;
  return archiveCoverUrl(archive.value?.data.coverPath, archive.value?.data.coverVer, content.folderThumbMap.get(id));
});
// 格式徽标：静态图 / 无扩展名映射返回 null = 不出徽标（统一铬后仅留此判定，速查表 §16）
const badgeLabel = computed(() => {
  if (!file.value) return null;
  const p = file.value.placement;
  return formatBadgeColor(p.ext, isSequence(p), p.animated) ? p.ext.toUpperCase() : null;
});
// 颜色列表真色（6.1 颜色解析）：detail.colors 前 7（透明像素不计，存 10 显 7）；
// 有内容缩略图但暂未取到色（待懒回填 / 全透明图）= 7 个占位色块
const swatchColors = computed(() => (file.value?.detail.colors ?? []).slice(0, 7));
// 色块行显隐（2026-09-23 用户定）：有内容缩略图才显示（thumbKind = asset，含 fla 经
// thumb_asset_id 代理的伴生 webp——其颜色即伴生图颜色，后端 ensure_asset_colors 同径回填）；
// 无内容缩略图的格式（格式图标占位 / 无图）不需要提取颜色，整行隐藏
const showSwatches = computed(() => file.value?.placement.thumbKind === "asset");
</script>

<template>
  <section ref="rootRef" class="section-pinned">
    <!-- 封面设置编辑态（6.1，43 稿；仅档案）：整组替换（组件不自持 padding——挂载点 pinned
         恒 padding 12，防双重 padding；组件内 gap 12 自管）；
         initialSourcePath = 封面选择模式回填图源（picker cover 用途，选定即取景态） -->
    <CoverSettings
      v-if="archive && coverEditing"
      :node="archive.node"
      :data="archive.data"
      :initial-source-path="coverDraftSource ?? ''"
      @done="coverEditing = false"
    />
    <template v-else>
      <!-- 缩略图 / 封面容器（稿：240×240 / 通用二级底 + 通用三级 1px / 圆角 8） -->
      <div
        class="thumb"
        :class="{ cover: !!archive }"
        v-tip="archive ? '封面设置' : ''"
        @click="archive && (coverEditing = true)"
      >
        <template v-if="file">
          <img v-if="thumbUrl" :src="thumbUrl" alt="" draggable="false" class="fit-contain" />
          <!-- 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 -->
          <span v-else class="ext">{{ file.placement.ext.toUpperCase() }}</span>
          <FormatBadge v-if="badgeLabel" class="badge" :label="badgeLabel" />
        </template>
        <template v-else-if="archive">
          <img v-if="archiveCover" :src="archiveCover" alt="" draggable="false" class="fit-cover" />
          <FormatBadge v-if="archive.data.badge" class="badge" :label="archive.data.badge.text" />
          <!-- hover 封面 = 纯黑压暗 50% + 设为封面图标居中（稿 prose） -->
          <span class="cover-hover" aria-hidden="true">
            <AppIcon name="set-cover" :size="18" />
          </span>
        </template>
      </div>
      <!-- 颜色列表（稿：透明壳横排；仅文件形态且有内容缩略图——无图格式整行隐藏，2026-09-23 用户定） -->
      <ColorSwatchItem v-if="file && showSwatches" :colors="swatchColors" :placeholder-count="7" />
    </template>
  </section>
</template>

<style scoped>
/* pinned 区块（稿：padding 12 / gap 12 纵列；无 border-top——详情面板首块） */
.section-pinned {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  flex: none;
}
/* 缩略图 / 封面容器（稿：240×240 定方，宽随栏 = 100%；通用二级底 + 通用三级 1px / 圆角 8） */
.thumb {
  position: relative;
  width: 100%;
  aspect-ratio: 1;
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  overflow: hidden;
  flex: none;
}
.thumb img {
  width: 100%;
  height: 100%;
  display: block;
  border-radius: var(--n-radius-8); /* 铺满图自带圆角（圆角平滑三件套口径，速查表 §7） */
}
/* 文件缩略图 = contain 不裁剪；档案封面 = cover 充满（稿裁切语义） */
.fit-contain {
  object-fit: contain;
}
.fit-cover {
  object-fit: cover;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.thumb .ext {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-g3);
}
/* 格式 / 档案徽标（稿：左上 12/12；FormatBadge = kc-float 统一铬 + 数字档 12/16 二级） */
.badge {
  position: absolute;
  top: var(--n-space-12);
  left: var(--n-space-12);
  pointer-events: none;
}
/* 档案封面 = 封面设置入口（6.1，43 稿）：点击进入编辑态 */
.thumb.cover {
  cursor: pointer;
}
/* hover 封面 = 纯黑压暗 50% + set-cover 图标居中 二级（稿 prose 1_hover-封面图） */
.cover-hover {
  position: absolute;
  inset: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(0, 0, 0, 0.5);
  color: var(--n-l2);
  opacity: 0;
  transition: opacity var(--n-dur-150) var(--n-ease-in-out);
  pointer-events: none;
}
.thumb.cover:hover .cover-hover {
  opacity: 1;
}
</style>
