# 2026-09-25_04 播放条波形竖条化——全 app 波形观感统一，get_waveform 退役

2026-09-25

## 需求

用户实机（09-25_03 波形链路全通后）：「播放条为什么是旧样式？新样式不是已经改了么」——卡片波形 09-22 R6 已换竖条新样式（WaveContent，4px 圆头竖条 + get_audio_peaks 128 根），详情 pinned 09-23 已同源，播放条还是 5.1 时代的发丝细密风 hires 图，是最后一处旧渲染。

## 落地

**WaveProgress 竖条化**：`src/components/bars/WaveProgress.vue` 重写——渲染直接复用 WaveContent（竖线 4px 圆头 99 gap 4 通用/五级，played 同构层品牌色 clip 进度），播放条语境差两笔走 `:deep` 覆盖（卡片 inset 12 → 满幅 0；played 层祖先 hover/playing 显隐 → 恒显）；数据改 `cardPeaksFor(preview.current)` 懒加载（get_audio_peaks 128 根，SectionPinned 同范式防错配）；播放头 1px 品牌竖线保留。补 `peaks?` prop 供演示注入。

**spec 链简化**：`PlayBarSpec.waveFallback(): string | null`（供图函数）→ `wave?: boolean`（族标志）——竖条化后 peaks 由 WaveProgress 自拉，spec 只标族不供数；barSpecs/audio、FloatBarHost 同步。

**get_waveform 全链删除**（竖条化后零消费方）：useHiresWaveform.ts、commands/media.rs 命令 + waveform/clamp_wave_size/WaveformImage + 两测试、lib.rs 注册、services/media.rs waveform_image_sized（MASK_ALPHA 常量随删、测试改字面量）；render_waveform 本体保留（info 缩略图 272 高仍在产，09-25_02 的 margin 等比修复与小高度回归钉继续有效）。接口文档 18 删 get_waveform 条目、get_audio_peaks 条目补播放条同源消费；MCP通道.md 只读白名单名单同步除名。cache/waveforms 开库清空名单保留不动（清存量文件用）。

**preview 页**：音频波形格 data-uri 演示机制废除，改静态 peaks 注入（demoWavePeaks 40 根）。

## 验证

`pnpm typecheck` 绿；`cargo test` 全量 486 绿（waveform 两测试随删除链移除）；preview 页截图目验（tools/style-preview/audio-bar-wave.png）：蓝已播 40% + 灰未播 4px 圆头竖条 + 播放头，与卡片完全同款。实机观感（peaks 实拉、随壳宽动画竖条数自适应）待用户走查。

## 文档

速查 §59 追加（同日四轮：竖条化统一 + get_waveform 退役清单）并登记 Index；月 Index 登记。
