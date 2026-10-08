# 跳转感知修复（预览中不复用）+ serde default 补齐（2026-10-08）

_09 落地后用户实机两报，一轮修复 + 实证闭环。

## 两报与定性

**①Obsidian 点 anikit:// 每次弹「Open external link?」确认窗**——Obsidian 对所有外部协议的标准安全确认（Eagle 的 eagle:// 同款），非我们 bug：弹窗里勾「Always open anikit:// links」一次永久免弹。

**②APP 内 MD 点链接「手型指针但没跳转」**——用户自己查明真相：其实跳转了，只是图片与文档同文件夹时 `openInFreshTab` 复用当前页 = 原地不动，而当前页盖着 MD 预览，选中的卡片在预览背后零感知（详情面板其实已显示图片数据）；不同文件夹则正常新开页。

## 修复：复用仅限纯网格态

`openInFreshTab` 加预览态排除（tabs.ts）：已有同 viewKey 页**正在预览**时激活它看不到定位（预览盖网格），此情形新开一页——原页预览保留不打断阅读，新页网格定位图片；已有页是纯网格态才激活复用（防堆积口径不变）。判断 = `peekPreviewSession(panelId)?.assetKey`（只查不建，exitPreview 同口径）。

## 实证：探针 12 项全 PASS

新立 `tools/cdp-md-link-probe.py`（桩沿用 cdp-md-scroll-repro 框架）：md 文档含五种引用（anikit 链接 / anikit 图 / http 图 / http 带跳转组合 / 真外链）——渲染形态 img.md-asset ×3 + a[href^=anikit:] 保留；预览中点同文件夹链接 = 新开页（tabs 1→2，原页预览保留）；重复点击 = 复用不堆积；http open 形态被认出 = 内部跳转零浏览器调用；真外链 = open_external_url 一次；整窗零导航（location 不变）。

## 顺带抓出 S5 serde bug（生产实证修复）

curl 真机调 MCP 时发现 `search_assets` 不带 folderId 报「missing field folderId」——**serde 的 deserialize_with 会让 Option 字段丢失内建「缺省 = None」**，S5 给 Option 字段挂 de_ref_id_opt 后缺省调用全炸（用户 dev 实例被 tauri watch 自动重编至最新代码暴露）。修复 = 8 处 Option 字段补 `#[serde(default)]`（search/folders/write/structure×2/prompt×3）。MCP tools 因 cfg(not(test))（踩坑 014）编不进测试构建，此类 serde 行为只能真机 curl 验——修复后复验 search 正常且 uri 字段实际输出（S5 增强生产在跑）。

## 用户文档查证（MCP 直读实证）

经 MCP get_asset_origin_path 拿磁盘路径直读 P2.md：文档里引用是标准半角带跳转形态、在代码块外——markdown-it 必正常解析。用户截图里「显示为源码文本」的行是**编辑中间态**（draft 未保存 / 未回预览态），保存并回预览态（眼睛钮 / Ctrl+E）即渲染为图。

typecheck 0 错 + cargo check 净。**待实机走查：同文件夹预览中点图 = 新开页定位 / 不同文件夹 / 重复点击不堆积 / Obsidian 勾 Always 后免弹**。探针留档回归。

## 走查补记（同日晚，用户复测后答疑轮）

- **媒体端点三态 curl 实证**（用户 dev 实例 17701）：`/item/{真id}` = 200 image/png 1.8MB 真图 / 假 id = 404 / `/open/item/{id}` = 200 极简页——端点行为全对。
- **VS Code 预览裂图答疑**：用户报 VS Code 预览里图片裂图——非端点问题（curl 实证出图），是 VS Code 预览安全级别 Strict（默认）拦 http 本地图片（横幅「已禁用此文档中的部分内容」），点横幅切「允许不安全内容」即显示（[vscode#32251](https://github.com/microsoft/vscode/issues/32251) 同款）。外部编辑器兼容性清单补进接口文档 anikit协议.md（Obsidian 勾 Always 免弹 / VS Code 切档 / Typora 与内部正常）。
