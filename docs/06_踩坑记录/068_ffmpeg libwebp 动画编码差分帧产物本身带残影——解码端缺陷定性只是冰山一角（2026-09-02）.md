## 068 ffmpeg libwebp 动画编码差分帧产物本身带残影——解码端缺陷定性只是冰山一角（2026-09-02）

- **现象：** 小动画缓存（ffmpeg sidecar 转码 animated webp）在卡片 `<img>` 播放时"所有帧都重叠"——CEP 伴生 webp（大透明区角色动画）必发，GIF（不透明全帧）无恙；同源 origin 走帧流/帧条（image-webp 解码）却正常。
- **定性链：** ① Pillow（libwebp 参考实现）规范合成转码产物第 4 帧起即带残影 → **产物帧本身就错**（排除 Chromium 播放缺陷、排除 image-webp 解码缺陷）；② ffmpeg 解码 origin 输出 PNG 干净 → 解码环节无恙；③ 坐实 **ffmpeg libwebp 编码差分帧（矩形裁剪 + blend/dispose 组合）生成错误**。2026-09-01 曾把同类重影定性为"image-webp 画布合成缺陷"——那只是差分帧危害链的一环：**不合规的差分产物在任何解码端都可能叠影**。
- **修法：** 小动画转码链整体弃用 ffmpeg sidecar，改纯 Rust 编解自洽——`anim::decode_frames`（完整画布帧）→ `shrink_to_edge` 缩放 → `anim::encode_webp_anim` 全帧组包（blend=0 全画布矩形，与 CEP 导出同一口径）；存量坏缓存由 `is_fresh` 加**全帧组包结构判定**（逐 ANMF chunk 校验 x=0,y=0、矩形=全画布、flags bit1=1——无版本戳可判时的结构特征口径）自动翻新重生成。
- **预防：** ① 转码类产物**必须经第二个独立解码端验证**（本例 Pillow 一测即现形，ffmpeg 自解自产物测不出来——编解同体的缺陷互相掩盖）；② 带 alpha 的动画编码是高危区：差分优化（矩形裁剪/blend/dispose）+ 有损 alpha 的组合，编码器实现质量参差，输出全帧组包（blend=0 源覆盖）是消除变量的终版姿势；③ 缓存类文件要翻新存量时，结构特征判定（文件内部格式指纹）比版本戳/mtime 更可靠。
