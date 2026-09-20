# 43 阶段七 R5 扩展点注册表化（2026-09-20；previewSpecs / cardSpecs 落地，四注册表成族）

> 目标（待办 2026-09-17 ②）：一切 kind-switch 改注册表——一类一文件 + 一行注册，宿主零改动；未来插件（代办/随手记/网页/3D/读剧本/QQ 音乐控件）只填坑不进主线。本轮回合后 AniKit 有四张注册表，全部 barSpecs 范式。

## 四张注册表总览

| 注册表 | 位置 | 宿主 | 键空间 | 状态 |
|---|---|---|---|---|
| 播放条 barSpecs | `src/lib/barSpecs/` | FloatBarHost | 预览 kind → 播放条形态 | 既有（2026-09-09 二轮） |
| 菜单 menuSpecs | `src/lib/menuSpecs/` | ContextMenu / AppMenu | 菜单场景 → 数据驱动 spec | 既有（数据驱动，无需再注册化） |
| **预览视口 previewSpecs** | `src/lib/previewSpecs/` | PreviewPanel | 预览 kind → 组件型视口 | 本轮落地 |
| **卡片 cardSpecs** | `src/lib/cardSpecs/` | FileCard / AssetCard | 卡片 kind → 内容+附加件；hover kind → 悬停层 | 本轮落地 |

## previewSpecs（预览视口）

- `PreviewPaneSpec = { kind, component }`；宿主基建分支（img 缩放 / video 元素挂载 / sequence / anim 流画布）不抽，组件型分支（font / text / audioWave / other）查表渲染（`<component :is>`）。
- 分支数据条件（audio 靠 waveUrl 非 kind）由组件内部 v-if 承接，spec 不加 active() 钩子（避免把含 watch/生命周期的 composable 塞进工厂）。
- 宿主-owned 依赖经 `PREVIEW_STAGE_KEY` provide/inject（audio 波形按视口实寸高清重采样）——项目首个类型化 inject 用例。
- 未注册 kind = 不渲染（与原链式兜底一致）；重复注册后者覆盖（插件扩展点）。

## cardSpecs（卡片）

- **两张表不合表**（键空间/契约/宿主三者不同，合表逼出半空对象）：`contentRegistry`（FileCard kind → 内容件 + overlays 附加件 + scrollThumb 接线开关，未注册回退 general）与 `hoverRegistry`（AssetCard kind → 悬停层 + autoplay 挂载语义）。
- `FileCardProps` 单一事实源在 `cardSpecs/types.ts`，FileCard 直接复用；内容件 props 由 spec 显式映射（宿主 prop 改型 = spec 文件即刻 typecheck 报错）。
- spec 为无状态组件引用对象（区别于 barSpecs 工厂闭包——播放条要逐实例捕获 store，卡片件不需要）。
- 新内容件落 `src/components/cards/contents/`（ThumbContent / WaveContent / FontContent / TextContent / MediaCapsule / VideoBar）；kind prop 由封闭联合改 `string`——新形态 = spec 文件 + 一行注册，宿主真零改动。

## 视口与模态窗贡献点（评估结论）

- 内容视口：ContentArea 无 kind-switch（单一 AssetGrid 内嵌三 Strip），无注册表可抽；分屏/标签走 dockview layouts（另一件事）。
- 模态窗：BaseModal/ConfirmModal 已是共享壳，业务弹窗挂 App.vue 集中托管，暂无需注册表（插件弹窗贡献点随插件机制立项再评）。

## 等价性与验证

两宿主行为零变化（逐分支核对表见开发日志 2026-09-20_17）；`pnpm typecheck` / `pnpm build` / 覆盖探针 ALL PASS；台账 148 件（新 72 / 业务 72 / 旧 2 / 休眠 2）。
