# 2026-09-20_17 R5 扩展点注册表化：previewSpecs + cardSpecs 落地

> 接续 R4（[16](2026-09-20_16_R4未落地稿补齐三件.md)）。目标 = 待办 2026-09-17 ② 的扩展性红线：新格式 = 加一个 spec 文件 + 一行注册，宿主零改动。定案记 [速查 §43](../../06_UI重构/规范/口径速查/43_阶段七R5扩展点注册表化（2026-09-20；previewSpecs-cardSpecs落地）.md)。

## 落地两件（均 barSpecs 范式）

**previewSpecs（预览视口注册表）**：PreviewPanel 模板链的四条组件型分支（font / text / audio 波形 / other 不支持预览）抽为 `src/lib/previewSpecs/` 注册表——一类一文件 + `registerPreviewPaneSpec` 一行；宿主基建分支（img / video / sequence / anim）不抽，查表未注册 = 不渲染（原兜底等价）。新增 PreviewAudioWave / PreviewUnsupported 两组件（markup/样式/接线逐字搬运）；audio 波形按视口实寸重采样的宿主依赖经 `PREVIEW_STAGE_KEY` provide/inject（项目首个类型化 inject）。

**cardSpecs（卡片注册表，两张表）**：`contentRegistry`（FileCard kind → 内容件 + overlays 附加件 + text 拇指接线）+ `hoverRegistry`（AssetCard kind → 悬停层 + autoplay 语义）——键空间/契约/宿主三者不同故不合表。`FileCardProps` 单一事实源挪 `cardSpecs/types.ts`（宿主 prop 改型 = spec 文件即刻 typecheck 报错）；kind prop 由封闭联合改 `string`。新增 `cards/contents/` 六件（ThumbContent / WaveContent / FontContent / TextContent / MediaCapsule / VideoBar），kind 专属样式逐字随迁。

**评估结论**：菜单（menuSpecs）与悬浮栏（barSpecs）本已注册化/数据驱动；内容视口无 kind-switch 可抽（单一 AssetGrid 内嵌三 Strip）；模态窗贡献点随插件机制立项再评。

## 等价性

两宿主逐分支核对（内容渲染条件 / overlays DOM 序 / 拇指公式 / hover 层挂载条件 / 自动播放 gate / ws 禁用 / picker·naming·selected 态），行为零变化；样式值零改动。

## 验证

`pnpm typecheck` 绿、`pnpm build` 过、覆盖探针 **ALL PASS**；`pnpm inventory`：**148 件**（新 72 / 业务 72 / 旧 2 / 休眠 2），旧件消费方 4、组件内 legacy token 29。

## 未验证项（交用户实机走查）

预览面板四分支实机渲染（字体/文本/音频波形大图/不支持预览双钮实击）；卡片展区五稿与素材网格的视频·音频·动图·序列帧悬停播放（注册表切换后的 hover 层挂载链）。

## 下一步

R6 新功能验证（终极验收）：挑 2~3 个新 kind（候选：筛选面板 / markdown 笔记 / 3D 或新格式卡）走注册表落地，验收「只动自己文件夹 + 一行注册，不动主线」。
