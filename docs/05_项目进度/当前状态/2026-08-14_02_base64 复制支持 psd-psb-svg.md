# base64 复制支持 psd/psb/svg（2026-08-14，feat/new-ui，用户实机右键 PSD"复制 base64"报不支持）

：`get_asset_base64` 原仅 6 种静态图片原字节 ≤10MB。**口径（用户定"解码成 PNG"）**：psd/psb/svg 走 thumb_formats 解码合成图 → 长边 ≤4096（复用全尺寸预览口径）→ PNG 编码 → base64，10MB 上限对编码后 PNG 判定——贴给 AI / 网页可用；静态图片分支不变，解码失败报"解码失败（PSD 可能未存合成图）"。**命令层两阶段**：locate_base64_source（锁内定位）+ base64_from_origin（锁外解码/编码，大 PSD 秒级不占库锁）。测试 2 新 1 改写，cargo test 295 全绿。PRD 02 §13 新增条目（原 PRD 无此功能条文）/ 07 接口文档同步；前端零改动。

