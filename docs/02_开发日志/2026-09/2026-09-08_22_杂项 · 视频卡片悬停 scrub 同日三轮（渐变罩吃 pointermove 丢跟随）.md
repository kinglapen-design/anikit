# 杂项 · 视频卡片悬停 scrub 同日三轮（渐变罩吃 pointermove 丢跟随）
## 杂项 · 视频卡片悬停 scrub 同日三轮（渐变罩吃 pointermove 丢跟随）

二轮（2026-09-08_21）后用户实机再报：scrub 总会丢失鼠标跟随——鼠标在动但画面与进度条都不快进，只要不再次移动鼠标就永远不快进过来。用户并指出关键判断：画面加载（seek）慢可以理解，进度条是纯 UI 行为应该很便宜，没理由不跟手。

**根因**（用户参与实锤 = 渐变罩）：`onScrub` 的 UI 直写本是无条件的，UI 不跟手 ⟺ pointermove 根本没到达播放层。FileCard 结构里悬停播放层在默认插槽（缩略图上、渐变罩下），`.kc-overlay` 渐变罩贴底 ≈74px（padding 24+12、名称 18、gap 4、info 16）层叠在播放层之上且 pointer-events 默认 auto——鼠标横扫轨迹偏低钻进罩区时，pointermove 落在渐变罩上，冒泡路径不经过播放层，`onScrub` 整段不触发：UI 停在进入罩区前的最后位置（已挂的 seek 补发链也随之停滞），移出罩区才恢复——正合"丢失跟随、再移动才快进"的表现。

**修法**（`FileCard.vue` scoped 覆盖，不动 cards.css 共享基座）：`.kc-overlay` `pointer-events: none` 整体穿透，scrub 区域 = 整卡；**名称文本行与命名输入恢复 `pointer-events: auto`**——v-tip / 双击重命名 / 输入交互保留，"底部名称行不 scrub"旧口径精确化到文本行（info 行无交互不恢复，一并穿透可 scrub）。FolderCard / ArchiveCard 等 cards.css 其他消费方不受影响；FileCard 另一消费场景（收件箱工程卡，无播放层）点击行为等价（穿透后冒泡殊途同归到卡根）。

**验证**：`pnpm typecheck` 0 错、`pnpm build` 通过。文档同步：AssetCard / FileCard 头注与模板注、口径速查 22（"渐变罩在播放层之上，底部名称行不 scrub 与旧一致"改为"pointer-events 穿透，仅名称文本行恢复交互"）。**行为变化交实机走查：悬停视频卡横扫（含划过底部渐变罩区域）帧号 / 进度条全程跟手不丢；名称行仍可悬停出 tip、双击重命名。**
