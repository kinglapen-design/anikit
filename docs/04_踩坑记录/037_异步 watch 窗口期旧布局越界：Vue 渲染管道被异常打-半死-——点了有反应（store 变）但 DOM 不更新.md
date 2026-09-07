## 037 异步 watch 窗口期旧布局越界：Vue 渲染管道被异常打"半死"——点了有反应（store 变）但 DOM 不更新

- **现象：** 2026-08-19 用户实机：分组/删组操作多了后 UI"卡死"——目录树选中态取消不掉、递归显示开关点不动；不是主线程忙（无长任务），而是**点击照常生效（store 状态变）、界面却不重渲染**，越操作越坏。
- **原因：** useGridLayout 的 `items`/`sections` watch 是异步 flush——items 已变短（切视图/归组变化）而 boxes/rows 还是上一轮旧布局的窗口期里，`visibleCards` 用 `items[旧box.i]` 取到 `undefined`，模板 key 求值 `c.asset.assetId` 抛 TypeError → Vue 更新队列中断并被污染（连锁 `shouldUpdateComponent` 读 null 的 `emitsOptions`——已卸载组件仍被 patch）。渲染管道一旦被打断，后续每次操作继续抛错，DOM 永久落后状态——用户观感即"卡死"。CDP 混合压力探针（24 轮：弹窗增删/菜单删组/折叠+树切换/滚动+悬停轮转）稳定复现：R17 起异常连发、递归开关切换后卡片数不跟随（与用户症状同构）；headless 桩环境即可现，与后端/缩略图无关。
- **解法：** 两处窗口期防御——`visibleCards` 跳过 `undefined` asset、`anchorKey` 取不到卡就不锚定（旧布局在 items 变化后本就该等 relayout 一致化，跳过是唯一正确语义）。修后同探针 24 轮零异常、递归开关联动恢复。
- **预防：** "items 与布局结果（boxes/rows）来自两个响应式源、靠异步 watch 一致化"的手写虚拟滚动/布局结构，**一切交叉索引读都按可能越界写**（`items[box.i]` 必须先判存在）。症状为"界面点了没反应但进程不崩"时，先查渲染管道是否被未捕获异常打半死（Console 红字 / exceptionThrown），再怀疑事件监听泄漏或主线程卡死。
