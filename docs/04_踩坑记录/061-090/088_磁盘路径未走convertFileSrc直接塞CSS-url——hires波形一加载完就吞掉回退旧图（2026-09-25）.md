# 088 磁盘路径未走 convertFileSrc 直接塞 CSS url()——hires 波形一加载完就「吞掉」回退旧图（2026-09-25）

## 现象

音频播放条稳定态只剩播放头、波形全空（连回退的 info 旧图也不显示）；但 cache/waveforms 里的高清图本身正常（满幅波形），详情面板 info 缩略图也显示正常——图好、前端层在，就是层不可见。

## 原因

WaveProgress 的接力链断在 URL 转换：`waveUrl = hiresPath ?? fallbackUrl`——`fallbackUrl`（barSpecs audio 的 waveFallback）已 `convertFileSrc` 成 asset URL，而 `hiresPath` 是 invoke 返回的**磁盘绝对路径原样**顶上来，`url("E:\anikit\动画资产\cache\...webp")` 在 CSS 里按相对地址解析（拼到 dev server origin 下）必然加载失败；mask 图像加载失败 = 空遮罩 = 两层波形全不可见。于是出现怪接力：hires 未返回时旧图正常（用户曾看到「旧的波形图」），hires 一返回坏 URL 反而把正常旧图顶掉——「加载完后是空的」。被删的 PreviewAudioWave 旧写法是对的（convertFileSrc 包在 computed 外层、两个源都转），WaveProgress 2026-09-09 移植时把转换漏在了优先级更高的 hires 分支上。

## 解法

WaveProgress 内给 hiresPath 补 `convertFileSrc`（fallback 端维持调用方已转、不二转）；头注写明转换职责划分。排查路径留档：先验图产物（PIL 量 alpha 行范围，满幅正常）→ 再验协议 scope（tauri.conf assetProtocol `**` 放行）→ 最后回到 URL 本身，比对「能显示的 info 图」与「不显示的 hires 图」差在哪——一个过了 convertFileSrc 一个没过。

## 预防

- **磁盘路径进 web 层（img src / CSS url() / mask）前必须过 convertFileSrc**，转换点写在「消费 URL 的组件」或「产出方」一处并注释分工，跨组件接力时逐段检查谁转了谁没转。
- 「回退正常、主路径反而坏」又一次成立（同踩坑 087 信号）：两条链产物不同时，**先比两条链的差异点**（本轮 = 一个转 URL 一个没转），别在公共段（图渲染 / 协议 scope）空转。
- 探针环境（headless Edge 无 asset 协议）测不出这类 URL 错误——探针音频截图一直「只有播放头」与实机同形同因不同，此类断言只能截图目验或实机走查。
