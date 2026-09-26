<script setup lang="ts">
// 字体预览（5.6；R4 版式换代，稿 = docs/06_UI重构/Design Token/布局/主窗口/中间面板/中间面板-内容容器-字体预览.md）：
// 结构 = 头部（名称 超大标题档 + FontInstallButton 安装圆钮 + 全名 标题档）→ SegmentedControl text 通栏
// （预览/其他两段；稿四段的 大小/字形 仍后置 09 待办）→ 页区（顶 2px g4 线 + padding 12 0）。
// 预览页 = 演示文本（origin 经 FontFace API 加载，webview 原生排版，后端不重渲染）：
// 文本源 = assets/text/荷塘月色_中文.md + 荷塘月色_英文.md（?raw 构建期内联；中文全文 + 英译全文双段排布，
// 中英字体形态都看得出）；H1 居中 MarkDown/H1 档 + 正文 MarkDown/正文档，默认字号对齐稿值（正文 15 / 标题 ×2=30）。
// 字号控制 = 顶部预览工具栏尺寸滑块（会话态 preview.fontSize；2026-09-26 用户定：预览区底部悬浮滑杆废——
// 稿预览工具栏本有尺寸滑块位）。
// 其他页 = name 表元数据卡（g2 底 + 1px g3 描边 + 圆角 8；行间 1px g3 顶线；
// 行 = 标签 按钮文字-大 三级 ↔ 值 按钮文字 二级 两端对齐）。
// 安装口径（2026-08-23 用户痛点定案）：用户级安装（HKCU 注册表 + 用户字体目录 + 广播），
// 其他软件立即可见——避免 Eagle"装了识别不到"。
// FontFace 管理：family 名按 assetId 命名防与系统字体冲突；切素材 / 卸载组件时 fonts.delete 清理。
import { computed, inject, onBeforeUnmount, ref, watch } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import FontInstallButton from "./buttons/FontInstallButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import { getFontInfo, installFont, uninstallFont, type FontInfo } from "../lib/fontOps";
import { PREVIEW_SESSION_KEY } from "../lib/previewSessionKey";
import { useToastStore } from "../stores/toast";
import DEMO_ZH_RAW from "../../assets/text/荷塘月色_中文.md?raw";
import DEMO_EN_RAW from "../../assets/text/荷塘月色_英文.md?raw";

const preview = inject(PREVIEW_SESSION_KEY)!;
const toast = useToastStore();

const TABS = [
  { value: "preview", label: "预览" },
  { value: "info", label: "其他" },
];

// 演示文本块（标题 / 正文）。源 md 极简解析：## 行 = 标题块，其余非空段（空行分隔）= 正文块；
// > 引用 / *斜体* / <u> 下划线标记一律剥离——演示文本只取纯文本看字形
type DemoBlock = { kind: "title" | "para"; text: string };
function parseDemo(raw: string): DemoBlock[] {
  const out: DemoBlock[] = [];
  for (const chunk of raw.split(/\n\s*\n/)) {
    let t = chunk.trim();
    if (!t) continue;
    const h = /^#{1,6}\s+(.*)$/.exec(t);
    if (h) {
      out.push({ kind: "title", text: h[1].trim() });
      continue;
    }
    t = t.replace(/^>\s?/, "").replace(/<\/?u>/g, "");
    if (t.length > 1 && t.startsWith("*") && t.endsWith("*")) t = t.slice(1, -1).trim();
    out.push({ kind: "para", text: t });
  }
  return out;
}
const DEMO_BLOCKS: DemoBlock[] = [...parseDemo(DEMO_ZH_RAW), ...parseDemo(DEMO_EN_RAW)];

const tab = ref("preview");
const info = ref<FontInfo | null>(null);
const loadError = ref<string | null>(null);
const fontReady = ref(false);
const busy = ref(false); // 安装 / 卸载在途
const previewScrollRef = ref<HTMLElement>();
const infoScrollRef = ref<HTMLElement>();

// 预览字体 family（按 assetId 命名防与系统字体冲突；未就绪时回退系统字体，加载完成即热替换）
const family = computed(() => `anikit-font-${preview.current?.assetId ?? "none"}`);
const demoStyle = computed(() => (fontReady.value ? { fontFamily: `"${family.value}", sans-serif` } : undefined));

// 头部：名称 = 字体内部 name（未载完回退素材名）；副标题 = 全名（稿 名称+安装按钮 / AlibabaPuHuiTiM 两行结构）
const headerName = computed(() => info.value?.name || preview.current?.name || "");
const headerSub = computed(() => info.value?.fullName ?? "");

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

// 元数据行（Eagle「其它」页同款字段 + 系统状态；空值显示 -。
// 稿九行含 制造商/版权/商标——后端 FontInfo 无此三字段（font.rs 唯一写点，本轮不动），维持现七行）
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
    <div class="range">
      <!-- 头部：名称 + 安装圆钮（space-between 两端）+ 全名 -->
      <div class="head">
        <div class="name-row">
          <span class="name">{{ headerName }}</span>
          <FontInstallButton :selected="info?.installed ?? false" @click="toggleInstall" />
        </div>
        <span v-if="headerSub" class="sub">{{ headerSub }}</span>
      </div>
      <SegmentedControl v-model="tab" :options="TABS" variant="text" />
      <!-- 预览页：演示文本（滚动区；字号由顶部预览工具栏尺寸滑块控制 = preview.fontSize） -->
      <div v-show="tab === 'preview'" class="page">
        <div ref="previewScrollRef" class="scroll kit-scroll">
          <div class="demo" :style="demoStyle">
            <template v-for="(b, i) in DEMO_BLOCKS" :key="i">
              <h1 v-if="b.kind === 'title'" class="demo-title" :style="{ fontSize: `${preview.fontSize * 2}px` }">{{ b.text }}</h1>
              <p v-else class="demo-body" :style="{ fontSize: `${preview.fontSize}px` }">{{ b.text }}</p>
            </template>
          </div>
        </div>
        <ScrollThumb :body="previewScrollRef" />
      </div>
      <!-- 其他页：元数据卡（行 = 标签 ↔ 值 两端对齐，行间 1px g3 顶线） -->
      <div v-show="tab === 'info'" class="page">
        <div ref="infoScrollRef" class="scroll kit-scroll">
          <div v-if="info" class="info-list">
            <div v-for="r in rows" :key="r.k" class="info-row">
              <span class="k">{{ r.k }}</span>
              <span class="v">{{ r.v }}</span>
            </div>
          </div>
        </div>
        <ScrollThumb :body="infoScrollRef" />
      </div>
    </div>
    <span v-if="loadError" class="error-text">{{ loadError }}</span>
  </div>
</template>

<style scoped>
/* 稿 字体预览 根：column padding 12 12 0 + 通用/一级底（gap 24 唯一子级无效，略） */
.font-preview {
  position: absolute;
  inset: 0;
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12) var(--n-space-12) 0;
  background: var(--n-g1);
  user-select: text; /* 演示文本 / 元数据可选中复制（stage 全局 none 覆盖回） */
}
/* 稿 显示范围：column padding 0 72 + gap 12（0.72 不透明度 = figma mock 调灰，色值已是终稿档，不乘） */
.range {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: 0 var(--n-space-72);
}
/* 稿 字体名称+安装按钮 块：column gap 4 */
.head {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
/* 稿 名称+安装按钮 行：space-between 居中（gap 稿值 10 无档，就近 --n-space-8） */
.name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
}
/* 稿 名称：超大标题档 36/46 Bold + MarkDown/标题色；长名省略（稿未画溢出态） */
.name {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-display);
  line-height: var(--n-lh-display);
  font-weight: var(--n-fw-bold);
  color: var(--n-md-title);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 稿 全名：标题档 18/26 Bold + MarkDown/引用色 */
.sub {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-md-other);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 稿 预览文章 / 其他 页：顶 2px 通用/四级线 + padding 12 0 */
.page {
  position: relative;
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  border-top: 2px solid var(--n-g4);
  padding: var(--n-space-12) 0;
}
.scroll {
  flex: 1;
  min-height: 0;
}
/* 演示文本：块间 gap 12（稿 预览文章 gap） */
.demo {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
/* 稿 H1 居中 MarkDown/H1 档（行高比 40/30；字号随滑杆动态，默认 30 贴稿） */
.demo-title {
  margin: 0;
  line-height: 1.333;
  font-weight: var(--n-fw-bold);
  text-align: center;
  color: var(--n-md-title);
}
/* 稿 正文 MarkDown/正文档（行高比 25/15；默认 15 贴稿） */
.demo-body {
  margin: 0;
  line-height: 1.667;
  color: var(--n-md-body);
}
/* 稿 信息列表：通用/二级底 + 1px 通用/三级描边 + 圆角 8（全局 border-box，border 内收不占外廓） */
.info-list {
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
/* 稿 行：padding 12 16 + space-between 两端对齐（gap 稿值 10 无档，就近 --n-space-8）；
   行间 1px 通用/三级顶线（首行无） */
.info-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
  padding: var(--n-space-12) var(--n-space-16);
}
.info-row + .info-row {
  border-top: 1px solid var(--n-g3);
}
/* 稿 标签：按钮文字-大 16/22 Bold + 亮色/三级 */
.info-row .k {
  flex: none;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
/* 稿 值：按钮文字 15/20 + 亮色/二级（右端对齐；长版权串折行） */
.info-row .v {
  min-width: 0;
  font-size: var(--n-fs-btn);
  line-height: var(--n-lh-btn);
  color: var(--n-l2);
  text-align: right;
  word-break: break-all;
}
/* 解析失败文案（说明文字档，居中底部，同 PreviewPanel error-text 口径） */
.error-text {
  position: absolute;
  left: 50%;
  bottom: var(--n-space-16);
  transform: translateX(-50%);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  white-space: nowrap;
  color: var(--n-danger);
}
</style>
