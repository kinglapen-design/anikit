# 阶段七 · 预览悬浮栏同日二轮（两侧呼吸 / 播放条填满 / 弹层对齐 / morph 落工程 + barSpecs 扩展性）

2026-09 09 同日二轮。用户实机三条 + 一条架构追问。

## 三条实机修复

- **悬浮栏两侧 12 呼吸**：anchor 改 `left/right: 12px`（搜索展开被挤压后两侧也保 12，播放条同口径）。
- **播放条横向填满**：FloatBar 加 `fill` prop（壳 width 100% 跳过实测，resize 指定值不变即时跟随不过渡；退场 gone 宽 0 仍过渡）；FloatBarPlay fill 下进度条 grow 1 吃余量（fill 不走 max-content 探针，正好绕开踩坑 072）。
- **弹层对齐触发件**：原「栏体水平居中」作废，回接旧通栏「触发件正上方居中」——FloatBarPlay hover/open 事件带触发事件对象，宿主 placePop 算 anchor 内 x + translateX(-50%) + 钳制。坑：placePop 读弹层宽时被**上一弹层的离场残留**（150ms leave 动画内仍在 DOM）带偏钳制——按当前弹层类选择 + `:not(.fbh-pop-leave-active)` 排除，探针实证对齐到 ±0。

## 架构追问：多状态切换动画的扩展性

用户问「形态切换动画落到工程里没有 + 以后 3D / 第三方格式怎么办」。定案两层解耦：

1. **morph 落工程**：搜索壳 SearchFloatBar 与播放壳 PreviewFloatBar 合并为 **FloatBarHost 根部单例**（ContentArea 预览/列表两分支之外，预览进出不卸载），mode = search ↔ play 走 FloatBar 既有 Transition（宽伸缩 + Y 出入 300ms），稿 prose「同一容器」兑现；无播放条 kind = visible 退场。探针补 morph 回程断言（关预览切回搜索形态 250）。
2. **扩展性 = barSpecs 注册表**（`src/lib/barSpecs/`，参照 menuSpecs 先例）：PlayBarSpec 接口（show 组 / 时间码 / 进度 / 控制面全函数取值 + 可选 frames / waveFallback）+ `registerPlayBarSpec(kind, factory)` 开放入口。内建 video / audio / sequence·anim 四 kind 三工厂；未来 3D / 插件格式 = 一个 spec 文件 + 注册一行，宿主与容器零改动。

## 验证

typecheck 绿；cdp-playbar-probe（+fill 断言 / 弹层对齐断言 / morph 回程断言）全绿；cdp-search-probe（S20a 加两侧 12 数值）全绿；cdp-bars-shot 预览页绿。未验证项：morph 进场手感、fill 拖侧栏跟随、笔记弹层对齐实机观感——交用户走查。
