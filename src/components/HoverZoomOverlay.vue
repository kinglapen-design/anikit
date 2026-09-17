<script setup lang="ts">
// 空格放大预览浮层（2026-08-10 用户定：访达 Quick Look 逻辑，取代悬停放大镜）：
// AssetGrid 空格键驱动（单选开 / 再按关 / 选择变化跟随），居中大图浮层（max 80vw×80vh，
// C3 底 / inset 1px border-1 / 圆角 8 / --shadow-pop）。**整体 pointer-events: none**——
// 鼠标事件穿透到下方网格：点击其他卡片 = 选择变化（AssetGrid watch 切换 / 关闭浮层），
// 双击等操作同样穿透（可直进预览）。Esc 兜底关闭。图源复用 get_asset_file_path：
// 直渲格式给源路径；psd/psb 缓存命中给全尺寸解码图、未命中先显 info 缩略图（秒显层）、
// preview-full-ready 事件到达换高清；其余（序列帧等）回退 info 缩略图。纯展示无交互。
import { onBeforeUnmount, onMounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getAssetFilePath, listSequenceFrames } from "../lib/mediaOps";
import { isSequence, isStaticImageExt } from "../lib/formatBadge";
import type { PlacementView } from "../stores/content";

const props = defineProps<{ asset: PlacementView }>();
const emit = defineEmits<{ close: []; space: [] }>();

// 秒显层 = info 缩略图（320 短边），高清图源到达后替换（Eagle 式渐进，同 4.3 预览窗口口径）
const src = ref<string | null>(props.asset.thumbPath ? convertFileSrc(props.asset.thumbPath) : null);

// 序列帧浮层播放（2026-07-28 用户定：动图范围悬停放大播放）：按 fps 轮播 info/frames 帧原图；
// GIF / 动图 WEBP 由 loadFull 直给源文件（webview 原生循环），无需专门分支
const isSeq = isSequence(props.asset);

let unlisten: (() => void) | undefined;
let seqInterval: ReturnType<typeof setInterval> | undefined;
let disposed = false;

onMounted(() => {
  // 工作空间磁盘实况（PRD 02 步骤 1）：assetId 为合成假 id 禁走 get_asset_file_path——
  // 直渲图类（静态图 / GIF / SVG）直 convertFileSrc 源路径；其余（音视频/工程等）保持缩略图层
  if (props.asset.assetId.startsWith("ws:")) {
    const ext = props.asset.ext.toLowerCase();
    if (props.asset.wsPath && (isStaticImageExt(ext) || ext === "gif" || ext === "svg")) {
      src.value = convertFileSrc(props.asset.wsPath);
    }
  } else if (isSeq) void loadSequence();
  else void loadFull();
  window.addEventListener("keydown", onKeydown);
});

async function loadSequence(): Promise<void> {
  try {
    const r = await listSequenceFrames(props.asset.assetId);
    if (disposed || r.frames.length === 0) return;
    const urls = r.frames.map((f) => convertFileSrc(f.fullPath));
    const cover = r.thumbFrame ? r.frames.findIndex((f) => f.name === r.thumbFrame) : -1;
    let idx = cover >= 0 ? cover : 0;
    src.value = urls[idx];
    seqInterval = setInterval(() => {
      idx = (idx + 1) % urls.length;
      src.value = urls[idx];
    }, 1000 / (r.fps ?? 24));
  } catch {
    // 清单加载失败保持 info 缩略图
  }
}

async function loadFull(): Promise<void> {
  try {
    const r = await getAssetFilePath(props.asset.assetId);
    if (disposed) return;
    if (r.path) {
      src.value = convertFileSrc(r.path);
      return;
    }
    if (r.pending) {
      // psd/psb 已进预览解析队列：高清解码完成推送（assetId 匹配即换并退订）
      const ul = await listen<{ assetId: string; path: string }>("preview-full-ready", (e) => {
        if (e.payload.assetId === props.asset.assetId) {
          src.value = convertFileSrc(e.payload.path);
          unlisten?.();
        }
      });
      // await 期间浮层已关闭：立即退订防监听器泄漏（2026-07-28 内存自查）
      if (disposed) {
        ul();
        return;
      }
      unlisten = ul;
    }
    // path = null（序列帧等无直渲源）：保持 info 缩略图
  } catch {
    // 解析失败保持 info 缩略图
  }
}

function onKeydown(e: KeyboardEvent): void {
  // window 级监听（2026-08-10 用户反馈：点到侧边栏后焦点离开网格，viewport 收不到空格——
  // Esc / 空格全局生效）；输入态（重命名等）不触发
  const t = e.target as HTMLElement;
  if (t instanceof HTMLInputElement || t instanceof HTMLTextAreaElement) return;
  if (e.key === "Escape") emit("close");
  else if (e.key === " ") {
    e.preventDefault();
    emit("space"); // 切换 / 关闭由 AssetGrid 判定（悬停对象归它管）
  }
}

onBeforeUnmount(() => {
  disposed = true;
  clearInterval(seqInterval);
  unlisten?.();
  window.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <!-- 卡片在虚拟滚动 transform 层内：fixed 浮层必须 Teleport 到 body（同 InboxDiskSection 右键菜单）；
       整体 pointer-events: none——鼠标穿透，移出卡片区域由 AssetCard 的 mouseleave 关闭浮层 -->
  <Teleport to="body">
    <div class="zoom-mask">
      <div class="zoom-box">
        <img v-if="src" class="zoom-img" :src="src" :alt="asset.name" draggable="false" />
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
/* 遮罩：全屏透明（参考图 3 背景不变暗）+ 事件穿透（悬停即出模式：移出卡片即关，2026-07-28） */
.zoom-mask {
  position: fixed;
  inset: 0;
  z-index: 2000;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: none;
}
/* 浮层壳：C3 底 / inset 1px border-1 / 圆角 8 / --shadow-pop（浮层统一规格），内边距 8。
   固定 60vw×70vh（2026-08-10 二轮用户定：80vw×80vh 太大——下方卡片基本点不到；
   尺寸固定不随图跳，图 contain 居中） */
.zoom-box {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 60vw;
  height: 70vh;
  padding: var(--space-8);
  border-radius: var(--radius-8);
  background: var(--c3);
  box-shadow:
    inset 0 0 0 1px var(--border-1),
    var(--shadow-pop);
}
.zoom-img {
  display: block;
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
</style>
