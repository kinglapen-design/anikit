## 086 `<video>` error 不分错误码一刀切转码——seek 瞬时管线错误误诊成格式不可播，白等整段软编码（2026-09-24）

- **现象：** 用户实机：预览普通 MP4（日常播放完全正常、确定可直读直解的片子），拖进度条到开头，偶发播放条显「转码中…」一等几十秒。可直读的源文件转码本不该发生。
- **原因：** 2026-09-13 直读优先定案（踩坑 075）的 error 回退链——前端四个播放点 `<video>`/`<audio>` 一听 error 事件就带 forceTranscode 转码兜底——**不读 MediaError.code 区分错误性质**。seek / 硬解管线的瞬时错误（MEDIA_ERR_DECODE / MEDIA_ERR_NETWORK：快速连续 seek、seek 到边界、ended 回绕瞬间管线复位偶发失败，Chromium 硬解已知脆点）与「格式真不支持」（MEDIA_ERR_SRC_NOT_SUPPORTED）被一视同仁，全部送进 ffmpeg libopenh264 软编码整段重编码。瞬时错误同源重载即可恢复，转码是纯白等；且每次打开素材都先直读，误诊条件复现就再触发一次。
- **解法：** error 回退分两级——`el.error?.code !== MediaError.MEDIA_ERR_SRC_NOT_SUPPORTED` 且本素材未重试过 → 同源 `load()` 重载直读重试一次（loadedmetadata 后恢复原播放位置续播）；重试仍 error 才落转码兜底；SRC_NOT_SUPPORTED（真格式不支持，首装即报）直落转码不白试。四播放点（previewMedia / HoverVideoLayer / HoverAudioLayer / TabLinks）同口径改。
- **预防：** 凡是「出错 → 兜底重活」的回退链，先分清错误性质：瞬时 / 可恢复错误先低成本重试，只有确定性失败（格式不支持、资源不存在）才进昂贵兜底。MediaError 要读 code，别把 error 事件当布尔用——同 075 的教训同构：静态预判（白名单 / 一刀切）永远落后于真实运行时，分级要按真实错误语义。
