# 2026-09-25_03 播放条波形全空根治——hiresPath 补 convertFileSrc

2026-09-25

## 需求

用户实机（09-25_02 后端 margin 修复重启后）：播放条依旧没有波形只剩播放头，且这次连 info 旧图接力都不显示。用户问「怎么不用旧图接力、刷新缩略图能不能更新」。

## 定位

先验产物：`cache/waveforms/AN..._869x27.webp` 用 PIL 量 alpha 行范围 = 0~25 满幅 ±13px——margin 修复已生效、图是好的（27 高 = 18×dpr1.5）。再验 asset scope（tauri.conf `**` 放行，详情面板 info 缩略图显示正常）。最后比两条链的 URL：fallback（info 图）过了 convertFileSrc 能显示，hires 图不过——**WaveProgress 的 `waveUrl = hiresPath ?? fallbackUrl` 把 invoke 返回的磁盘绝对路径原样顶进 CSS url()**，按相对地址解析必然加载失败 → mask 空遮罩 → 两层波形不可见。hires 未返回时旧图正常显示；一返回坏 URL 反而顶掉好图——这就是「加载完后是空的」「旧图不接力」的完整机制。踩坑 088 立档。

## 落地

`src/components/bars/WaveProgress.vue`：hiresPath 补 `convertFileSrc`（fallback 端维持调用方已转不二转）；头注与 computed 注释写明转换职责。一行级修复，逻辑无他动。

## 验证

`pnpm typecheck` 绿。探针环境测不出此类错误（headless Edge 无 asset 协议，探针音频截图恒「只有播放头」与实机同形不同因）——**实机未验**，前端改动经 vite HMR 即生效（无需重启 app），待用户打开音频预览确认：预期 hires 返回后波形满幅显示、已播放段品牌色随进度推进。

## 文档

踩坑 088（磁盘路径进 web 层必须过 convertFileSrc；「回退正常主路径反而坏」先比两链差异）+ 速查 §59 追加（同族第三条）+ 月 Index 登记。
