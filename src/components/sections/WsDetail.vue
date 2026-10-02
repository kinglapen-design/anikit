<script setup lang="ts">
// 工作空间磁盘实况文件 / 文件夹详情形态（2026-09-26 文件落地，同日文件夹随选中链接通）：
// ws 合成项（assetId = ws: 假 id，禁发 assetId 类命令）的简化「基础信息」——结构沿文件夹详情
// 形态（FolderDetail）：输入框区块（名称 + 位置）+ 基本数据卡（BaseDataItem）；
// **无分段控制器、无 pinned 缩略图、无信息查看管理钮**
//（PRD 02：ws 无任何附加数据——无标签/注释/评分，只展示磁盘事实 + 缓存，不接库的属性/描述/
// 提示词/关系模块；不进 ui.detailBlocks 三 tab 配置体系）。
// 字段口径：
//   名称 = 磁盘文件名（跟随「显示扩展名」，displayName 同库侧口径；文件夹不去后缀——目录名里
//   的点是名称本体）；位置 = 所在目录绝对路径
//   （TextInput 只读展示长路径——BaseDataItem 值档 nowrap 放不下路径，链接输入框同款只读形态）；
//   尺寸行仅图片显示——图片条目 w/h = 源像素（后端魔数探测）；视频/音频条目 w/h = 缓存缩略图尺寸、
//   字体 = 试排样张尺寸（thumb_formats font 适配器 SPECIMEN 常量），皆不可当「分辨率」展示误导用户；
//   创建/修改日期 = ctimeMs/mtimeMs（0 = 平台不支持，不加行——无数据不加行旧口径沿）；
//   文件夹（2026-10-03 目录属性落地，Windows 属性窗口口径）= 类型行 + 统计三行（文件数 /
//   子文件夹数 / 占用空间 = 递归字节求和，库 FolderDetail 同标签同口径；异步 ws_dir_stats
//   拉取，未到位不加行）+ 创建/修改日期（目录时间 2026-10-03 后端放开；根视口合成条目无
//   entry 时间 → 取 stats 根目录值）。
import { computed, ref, watch } from "vue";
import TextInput from "../inputs/TextInput.vue";
import BaseDataItem from "../items/BaseDataItem.vue";
import { displayName } from "../../lib/formatBadge";
import { formatDetailDate, type DetailTarget } from "../../lib/detailSections";
import { formatSize } from "../../lib/formatSize";
import { wsDirStats, type WsDirStats } from "../../lib/wsOps";
import { useUiStore } from "../../stores/ui";

const props = defineProps<{ target: Extract<DetailTarget, { kind: "ws" }> }>();
const ui = useUiStore();

// w/h = 源像素的格式（后端 thumbnail::probe 魔数 + thumb_formats psd/svg 适配器探测值）；
// 其余格式 w/h 非源分辨率（缩略图 / 样张尺寸），尺寸行不显示
const SRC_DIM_EXTS = new Set(["png", "jpg", "jpeg", "bmp", "webp", "gif", "psd", "psb", "svg"]);

// 目录属性统计（选中文件夹 / 视口回退共用）：entry 对象 identity 作 watch 源——所在目录重扫
// 换新对象即重拉快照（Explorer 属性窗口快照口径：打开时的值，深层变化不实时跟）；await 期间
// 切走防错配（DetailPanel.loadDetail 同模式）；未开空间 / 目录没了静默 null 不加行
const stats = ref<WsDirStats | null>(null);
watch(
  () => props.target.entry,
  async (entry) => {
    stats.value = null;
    if (!entry.isDir) return;
    const s = await wsDirStats(entry.relPath).catch(() => null);
    if (props.target.entry === entry) stats.value = s;
  },
  { immediate: true },
);

// 名称与库侧同口径：跟随「显示扩展名」（ws 无序列帧/动图，frameCount/animated 给常量）；
// 文件夹原名直出（目录名里的点是名称本体，displayName 会误去后缀）
const name = computed(() =>
  props.target.entry.isDir
    ? props.target.entry.name
    : displayName({ name: props.target.entry.name, frameCount: null, animated: false }, ui.showExt),
);

// 修改 / 创建日期：Unix 毫秒 → formatDetailDate 两截（日期 + 时间，BaseDataItem 多截值口径）
function fmtMs(ms: number): string[] {
  return formatDetailDate(new Date(ms).toISOString()).split(" ");
}

// 基本数据行（无数据源的行不加——SectionInfo 旧口径沿）
const dataRows = computed<{ label: string; values: string[] }[]>(() => {
  const e = props.target.entry;
  // 文件夹（2026-10-03 目录属性）：类型行 + 统计三行（异步未到位不加）+ 创建/修改日期
  //（行序贴 Windows 属性窗口：类型 → 包含/大小 → 时间；统计标签 = 库 FolderDetail 同款）
  if (e.isDir) {
    const out: { label: string; values: string[] }[] = [{ label: "类型", values: ["文件夹"] }];
    const s = stats.value;
    if (s) {
      out.push({ label: "文件数", values: [s.files.toLocaleString()] });
      out.push({ label: "子文件夹数", values: [s.dirs.toLocaleString()] });
      out.push({ label: "占用空间", values: [formatSize(s.totalSize)] });
    }
    // 时间：条目值优先（选中 / 视口回退子目录即时有）；根视口合成条目恒 0 → 取 stats 根目录值
    const ctime = e.ctimeMs > 0 ? e.ctimeMs : (s?.ctimeMs ?? 0);
    const mtime = e.mtimeMs > 0 ? e.mtimeMs : (s?.mtimeMs ?? 0);
    if (ctime > 0) out.push({ label: "创建日期", values: fmtMs(ctime) });
    if (mtime > 0) out.push({ label: "修改日期", values: fmtMs(mtime) });
    return out;
  }
  const out: { label: string; values: string[] }[] = [];
  if (e.w != null && e.h != null && SRC_DIM_EXTS.has(e.ext)) {
    out.push({ label: "尺寸", values: [`${e.w} x ${e.h}`] });
  }
  out.push({ label: "文件大小", values: [formatSize(e.size)] });
  out.push({ label: "类型", values: [e.ext ? `${e.ext.toUpperCase()} 文件` : "文件"] });
  if (e.ctimeMs > 0) out.push({ label: "创建日期", values: fmtMs(e.ctimeMs) });
  if (e.mtimeMs > 0) out.push({ label: "修改日期", values: fmtMs(e.mtimeMs) });
  return out;
});
</script>

<template>
  <!-- 输入框区块（详情区块公共壳：padding 12 / gap 8 / border-top 通用/三级——同 FolderDetail fd-inputs）；
       位置框 root 未就绪的窗口期 dirPath 为 null → 空值显 placeholder -->
  <section class="wd-inputs">
    <TextInput :model-value="name" readonly placeholder="名称" />
    <TextInput :model-value="target.dirPath ?? ''" readonly placeholder="位置" />
  </section>
  <!-- 基本数据区块（详情区块公共壳：padding 12 / border-top；BaseDataItem 信息卡） -->
  <section class="wd-data">
    <BaseDataItem :rows="dataRows" />
  </section>
</template>

<style scoped>
/* 输入框 / 数据区块壳（padding 12 / border-top 通用/三级——详情区块公共壳口径，与 TabBasic .db-block /
   FolderDetail .fd-* 同值；区块组件自带壳为本目录既有结构惯例） */
.wd-inputs {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  flex: none;
}
.wd-data {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
  flex: none;
}
</style>
