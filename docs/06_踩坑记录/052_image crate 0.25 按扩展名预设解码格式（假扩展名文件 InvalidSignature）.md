## 052 image crate 0.25 按扩展名预设解码格式（假扩展名文件 InvalidSignature）

- **现象：** 2026-08-23（5.4 实机走查）用户报"有的 PNG 格式不支持"——样本为壁纸站下载的 .png 文件，魔数实为 JPEG（FFD8FF），自定义缩略图报"该文件无法用作缩略图"。
- **原因：** image 0.25 的 `image::open` / `image::image_dimensions` 内部 `ImageReader::open` **按路径扩展名预设 format**，随后的 `with_guessed_format` 只在 format 为 None 时才按内容魔数猜测——扩展名撒谎时魔数猜测被跳过，PNG 解码器读 JPEG 流报 InvalidSignature。
- **解法：** 绕过 open 的扩展名预设，手动构造无预设 reader：`ImageReader::new(BufReader::new(File::open(p)?)).with_guessed_format()`（format = None → 恒按内容魔数）。thumbnail 的 probe / load_origin 已改此口径（probe_by_signature / decode_by_signature，导入与缩略图三件套同愈），回归用例 `fake_extension_decodes_by_content_signature`。
- **预防：** 用户渠道文件扩展名不可信（壁纸站 / 改后缀收藏常见）；凡"按文件内容判定格式"的场景走魔数版 helper，别再直接用 `image::open` / `image::image_dimensions`。
