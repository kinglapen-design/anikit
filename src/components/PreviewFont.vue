<script setup lang="ts">
// 字体预览（5.6；Eagle 四页签先「预览 + 其它」两页，大小/字形后置 09 待办）：
// 预览页 = 演示文本（origin 经 FontFace API 加载，webview 原生排版，后端不重渲染）+ 底部字号滑杆；
// 其它页 = name 表元数据（名称/全名/系列/样式/版本/字数，Eagle 同款）+ 系统级安装/卸载。
// 安装口径（2026-08-23 用户痛点定案）：用户级安装（HKCU 注册表 + 用户字体目录 + 广播），
// 其他软件立即可见——避免 Eagle"装了识别不到"。
// FontFace 管理：family 名按 assetId 命名防与系统字体冲突；切素材 / 卸载组件时 fonts.delete 清理。
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import AppButton from "./AppButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import SideSlider from "./SideSlider.vue";
import { getFontInfo, installFont, uninstallFont, type FontInfo } from "../lib/fontOps";
import { usePreviewStore } from "../stores/preview";
import { useToastStore } from "../stores/toast";

const preview = usePreviewStore();
const toast = useToastStore();

const TABS = [
  { value: "preview", label: "预览" },
  { value: "info", label: "其它" },
];

// 演示文本（Eagle 同款英译《荷塘月色》标题与选段 + 中文一段兰亭集序——中文字体也要看得出中文形态）
const DEMO_TITLE = "Moonlight over the Lotus Pond";
const DEMO_EN =
  "It has been rather disquieting these days. Tonight, when I was sitting in the yard enjoying the cool, " +
  "it occurred to me that the Lotus Pond, which I pass by every day, must assume quite a different look " +
  "in such moonlit night. A full moon was rising high in the sky; the laughter of children playing " +
  "outside had died away; in the room, my wife was patting the son, Run-er, sleepily humming a cradle song.";
const DEMO_ZH =
  "永和九年，岁在癸丑，暮春之初，会于会稽山阴之兰亭，修禊事也。群贤毕至，少长咸集。" +
  "此地有崇山峻岭，茂林修竹，又有清流激湍，映带左右，引以为流觞曲水，列坐其次。";

const tab = ref("preview");
const info = ref<FontInfo | null>(null);
const loadError = ref<string | null>(null);
const fontReady = ref(false);
const fontSize = ref(22); // 演示文本正文字号（标题 ×2）
const busy = ref(false); // 安装 / 卸载在途
const previewScrollRef = ref<HTMLElement>();
const infoScrollRef = ref<HTMLElement>();

// 预览字体 family（按 assetId 命名防与系统字体冲突；未就绪时回退系统字体，加载完成即热替换）
const family = computed(() => `anikit-font-${preview.current?.assetId ?? "none"}`);
const demoStyle = computed(() => (fontReady.value ? { fontFamily: `"${family.value}", sans-serif` } : undefined));

let face: FontFace | null = null;

async function load(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId) return;
  unloadFont();
  info.value = null;
  loadError.value = null;
  try {
    const r = await getFontInfo(assetId);
    if (preview.current?.assetId !== assetId) return;
    info.value = r;
    // FontFace 加载（文件损坏 / 读取失败）：回退系统字体渲染演示文本 + 中文提示（元数据页不受影响）
    try {
      const f = new FontFace(family.value, `url("${convertFileSrc(r.originPath)}")`);
      await f.load();
      if (preview.current?.assetId !== assetId) return;
      document.fonts.add(f);
      face = f;
      fontReady.value = true;
    } catch {
      if (preview.current?.assetId !== assetId) return;
      loadError.value = "字体加载失败，以系统字体预览";
    }
  } catch (e) {
    if (preview.current?.assetId !== assetId) return;
    loadError.value = e instanceof Error ? e.message : String(e);
  }
}
function unloadFont(): void {
  if (face) {
    document.fonts.delete(face);
    face = null;
  }
  fontReady.value = false;
}
watch(() => preview.current?.assetId, () => void load(), { immediate: true });
onBeforeUnmount(unloadFont);

async function toggleInstall(): Promise<void> {
  const assetId = preview.current?.assetId;
  if (!assetId || !info.value || busy.value) return;
  busy.value = true;
  try {
    const r = info.value.installed ? await uninstallFont(assetId) : await installFont(assetId);
    if (preview.current?.assetId !== assetId) return;
    info.value = { ...info.value, installed: r.installed };
    toast.notify({ message: r.installed ? "已安装到系统" : "已从系统卸载", icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    busy.value = false;
  }
}

// 元数据行（Eagle「其它」页同款字段 + 系统状态；空值显示 -）
const rows = computed(() => {
  const i = info.value;
  if (!i) return [];
  const dash = (s: string) => (s.trim() ? s : "-");
  return [
    { k: "名称", v: dash(i.name) },
    { k: "全名", v: dash(i.fullName) },
    { k: "系列", v: dash(i.family) },
    { k: "样式", v: dash(i.style) },
    { k: "版本", v: dash(i.version) },
    { k: "字数", v: String(i.glyphCount) },
    { k: "系统状态", v: i.installed ? "已安装到系统" : "未安装" },
  ];
});
</script>

<template>
  <div class="font-preview">
    <div class="tab-bar">
      <SegmentedControl v-model="tab" :options="TABS" variant="text" style="width: 168px" />
    </div>
    <!-- 预览页：演示文本（滚动区）+ 底部字号滑杆 -->
    <div v-show="tab === 'preview'" class="page">
      <div ref="previewScrollRef" class="scroll kit-scroll">
        <div class="demo" :style="demoStyle">
          <h1 class="demo-title" :style="{ fontSize: `${fontSize * 2}px` }">{{ DEMO_TITLE }}</h1>
          <p class="demo-body" :style="{ fontSize: `${fontSize}px` }">{{ DEMO_EN }}</p>
          <p class="demo-body" :style="{ fontSize: `${fontSize}px` }">{{ DEMO_ZH }}</p>
        </div>
      </div>
      <ScrollThumb :body="previewScrollRef" />
      <div class="size-bar">
        <SideSlider v-model="fontSize" :min="12" :max="72" style="width: 240px" />
      </div>
    </div>
    <!-- 其它页：元数据表（斑马纹行）+ 安装/卸载钮 -->
    <div v-show="tab === 'info'" class="page">
      <div ref="infoScrollRef" class="scroll kit-scroll">
        <div class="info-table">
          <div v-for="r in rows" :key="r.k" class="info-row">
            <span class="k">{{ r.k }}</span>
            <span class="v">{{ r.v }}</span>
          </div>
        </div>
      </div>
      <ScrollThumb :body="infoScrollRef" />
      <div v-if="info" class="install-bar">
        <AppButton
          :label="info.installed ? '从系统卸载' : '安装到系统'"
          :variant="info.installed ? 'panel' : 'primary'"
          :disabled="busy"
          @click="toggleInstall"
        />
      </div>
    </div>
    <span v-if="loadError" class="error-text">{{ loadError }}</span>
  </div>
</template>

<style scoped>
.font-preview {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  user-select: text; /* 演示文本 / 元数据可选中复制（stage 全局 none 覆盖回） */
}
.tab-bar {
  flex: none;
  display: flex;
  justify-content: center;
  padding: var(--space-12) var(--space-16) 0;
}
.page {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
.scroll {
  flex: 1;
  min-height: 0;
}
/* 演示文本：左右 48 呼吸，底部给滑杆留位 */
.demo {
  padding: var(--space-24) 48px 72px;
}
.demo-title {
  margin: 0 0 var(--space-24);
  line-height: 1.4;
  font-weight: 700;
  color: var(--text-normal);
}
.demo-body {
  margin: 0 0 var(--space-20);
  line-height: 1.8;
  color: var(--text-normal);
}
/* 底部字号滑杆（Eagle 同款底部居中；±按钮无图标不配） */
.size-bar {
  position: absolute;
  left: 50%;
  bottom: var(--space-16);
  transform: translateX(-50%);
}
/* 元数据表（Eagle 斑马纹：奇数行 C2 底；容器圆角裁边） */
.info-table {
  margin: var(--space-12) var(--space-16);
  border-radius: var(--radius-6, 6px);
  overflow: hidden;
}
.info-row {
  display: flex;
  gap: var(--space-16);
  padding: 10px var(--space-16);
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
}
.info-row:nth-child(odd) {
  background: var(--c2);
}
.info-row .k {
  width: 72px;
  flex: none;
  color: var(--text-label);
}
.info-row .v {
  flex: 1;
  min-width: 0;
  color: var(--text-normal);
  word-break: break-all;
}
.install-bar {
  flex: none;
  display: flex;
  justify-content: center;
  padding: 0 0 var(--space-20);
}
/* 解析失败文案（说明文字档，居中底部，同 PreviewPanel error-text 口径） */
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--space-16);
  transform: translateX(-50%);
  font-size: var(--text-caption-size);
  line-height: var(--text-caption-line);
  white-space: nowrap;
  color: var(--error);
}
</style>
