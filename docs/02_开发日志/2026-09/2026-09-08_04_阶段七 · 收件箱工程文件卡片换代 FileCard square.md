# 阶段七 · 收件箱工程文件卡片换代 FileCard square（2026-09-08）

## 起因

用户走查发现：自动导入视口的「工程文件」磁盘实况区（InboxDiskSection）卡片还是旧样式——2026-08-13 同步素材卡片 14 稿的散装实现（C2 图标区 + C3 简介区上下两段、实色扩展名角标、双伪元素 1px/1.5px 边框）。用户定：改成新的统一样式，shell 缩略图走通用稿。

## 定案

设计稿「文件卡片-通用」稿注本就写明：shell 缩略图与自动导入未导入卡片采用正方形比例、缩略图占容器 50% 居中（速查 §16 第 12 条，FileCard 的 square 变体早已封装）。本轮就是把该区块从散装旧样式换代到定稿组件：

- 卡片芯 = `cards/FileCard`（kind=general + square）：g2 底圆角 8、底部渐变罩名称/简介、hover 位移 -4 + 小阴影、选中 2px 品牌环，全走 cards.css kc 族；
- 系统格式图标（iconPath）= thumb-url，square 档自动 50% 居中（与旧「shell 图标 50%」口径同档）；
- 格式角标 = FormatBadge 统一铬（左上 12/12），扩展名大写恒出，不再按格式设色——formatBadgeColor 实色映射在本区的最后一个引用随之废除（与 AssetCard 2026-08-30 口径变化同款）；
- 重命名输入态归 FileCard 内建（naming prop + name-commit/cancel/edit，useInlineRename 统一语义），本区散装 useInlineRename 接线删除，只留目标条目与「扩展名补回」落命令逻辑。

两处新稿未画的存量元素保旧（同 AssetCard 笔记角标先例）：目录条目 folder-closed 40 整卡居中（与 shell 缩略图同位）；「被占用」标记保旧视觉旧位（右上各 4，c3 底警告字），壳层附件随卡芯 hover 位移 -4px（AssetCard 位置徽章同参数跟随模式）。

## 验证

`pnpm typecheck` 绿。网格右键调度（gridDispatch 的 `.disk-card` 选择器）类名保留不动，右键/选中/重命名交互路径未变。未实机走查：正方形卡行高变化（260 列宽 → 260 高）、目录图标与被占用标记在新卡上的观感，交用户实机定夺。

## 同步

速查 §16 补「业务接入登记」条；base.css 重命名共享基座注释中「收件箱磁盘区 C4 实底族另一套」的过时描述已修正（该区现归 FileCard naming 内建）。
