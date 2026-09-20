// 卡片扩展点注册表类型（cardSpecs，2026-09-21 R5 扩展点注册表化 · 卡片轮，范式 = src/lib/barSpecs/）——
// 扩展性口径：
// FileCard 内容区（缩略图 / 波形 / 字形 / 文本…）与 AssetCard 悬停播放层（视频 / 动图 / 序列帧 /
// 音频）双双注册表化，宿主只查表分发。新增卡片内容形态（未来 3D 卡 / 插件格式卡）= 加一个 spec
// 文件 + index.ts 一行注册，FileCard / AssetCard 宿主零改动。
//
// 两张表不合表（键空间与契约都不同）：
// - contentRegistry：FileCard kind（内建 general / video / audio / font / text）→ CardContentSpec
//   = 内容区组件 + props 映射 + 可选附加件（悬停胶囊 / 播放条 / 安装钮，kc-clip 外、DOM 序 = 数组序）
//   + 可选滚动胶囊接线（scrollThumb）；未注册 kind 宿主回退 general。
// - hoverRegistry：AssetCard kind（内建 anim / sequence / video / audio；other 不注册 = 无悬停层）
//   → HoverLayerSpec = 层组件 + 挂载语义（autoplay：true = 悬停或自动播放名额授予才挂、传 auto 参；
//   false = 仅悬停即挂——音频无自动播放）。
//
// props 接线契约：FileCardProps 即 FileCard 公开 props 全量（单一事实源，宿主 defineProps 同用）；
// 内容件 / 附加件按名声明所需子集（inheritAttrs 关闭防透传），spec 的 props 映射显式取值——
// 宿主 prop 改名 / 改型此处即刻 typecheck 报错。悬停层 props 契约 = { asset: PlacementView;
// auto?: boolean }（autoplay 档宿主才传 auto）。
import type { Component } from "vue";

/** FileCard 公开 props（宿主与注册表共用的接线契约） */
export interface FileCardProps {
  kind?: string; // 内容形态（注册表键；未注册回退 general）
  name: string; // 名称（必填）
  info?: string; // 简介行（尺寸 / 时长 / 字重…，内容业务侧定）
  badge?: string; // 格式角标文案（不传不出）
  thumbUrl?: string; // 缩略图（thumb 类内容件）
  thumbFit?: "cover" | "contain" | "icon"; // 缩略图适配（icon = 格式图标 50% 居中）
  square?: boolean; // shell 缩略图 / 未导入卡 = 正方形 + 50% 居中
  capsuleText?: string; // 悬停胶囊文本（video = 帧数 / audio = 时长）
  playing?: boolean; // 胶囊 播放 ↔ 暂停
  muted?: boolean; // video 胶囊 音量 ↔ 静音
  progress?: number; // 0..1：video 播放条 / audio 波形扫过
  peaks?: number[]; // audio 波形（0..1 归一化）
  fontLines?: string[]; // font 示例字
  installed?: boolean; // font 安装钮选中
  textTitle?: string; // text 卡内标题
  textBody?: string; // text 卡正文
  originBadge?: boolean; // 原始文件-徽章（右下 12/12）
  selected?: boolean;
  naming?: boolean; // 行内重命名输入态
  picked?: boolean; // 选择模式已选（内容压暗 30%，大勾在壳层）
  forceState?: "" | "hover" | "selected";
}

/** kind 专属附加件（kc-clip 外浮件：悬停胶囊 / 视频播放条 / 字体安装钮…） */
export interface CardContentOverlay {
  comp: Component;
  /** 宿主 props → 附加件 props 映射（静态变体参也走这里，如胶囊音量钮档） */
  props: (p: FileCardProps) => Record<string, unknown>;
}

/** FileCard 内容形态（kind → 内容区 + kind 专属附加件） */
export interface CardContentSpec {
  /** 内容区组件（kc-clip 圆角裁剪层内、渐变罩下） */
  content: Component;
  /** 宿主 props → 内容件 props 映射 */
  contentProps: (p: FileCardProps) => Record<string, unknown>;
  /** 附加件列表（kc-clip 外，DOM 序 = 数组序）；无 = 不出 */
  overlays?: readonly CardContentOverlay[];
  /** 内容溢出滚动胶囊（text 卡）：宿主渲染拇指 + 接线——content 组件须单根元素、根即滚动容器 */
  scrollThumb?: boolean;
}

/** AssetCard 悬停播放层（kind → 层组件 + 挂载语义） */
export interface HoverLayerSpec {
  /** 悬停播放层组件（props 契约 = { asset: PlacementView; auto?: boolean }，注入 FileCard 默认插槽） */
  layer: Component;
  /** true = animPlaying（悬停 || 自动播放名额授予）挂载、宿主传 auto=!hoverActive；
      false = 仅 hoverActive 挂载（音频无自动播放）、不传 auto */
  autoplay: boolean;
}
