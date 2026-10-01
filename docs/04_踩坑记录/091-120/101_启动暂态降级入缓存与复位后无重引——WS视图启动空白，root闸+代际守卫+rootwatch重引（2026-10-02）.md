# 启动暂态降级数据入缓存 × 复位后无人重扫——WS 视口启动空白（2026-10-02）

## 现象

上次关闭 APP 时中间视口停在工作空间目录（如 `04_文库`），下次启动：标签页标题正确恢复（标题 = rel 末段纯字符串派生，不依赖目录数据），但中栏一直全空白（ws 视口没有空态文案，零条目 = 纯空白）、侧栏 ws 树「文件夹 / 0」、常用区块整个消失。手动切一次库分段再切回 ws 才恢复。

## 原因

两条交错缺一不可，都在启动四条并发异步线（设置恢复 / `initWsListeners` / `reopenLastSession` 开空间 / 标签恢复）的竞态窗口里：

1. **启动暂态的降级数据被当真缓存**：ContentArea 与 WorkspaceTree 挂载即 `loadDir`，跑在 `open_space` 完成前，吃 `list_workspace_dir`「未开空间 → `Ok([])`」的刻意暂态降级（踩坑 100 修复时保留的口子）——空列表被当作成功入缓存（`loaded` 置位、`missing` 不置位，与真空目录不可区分）。
2. **`$reset` 后四条重扫通路全断**：`open_space` 兑现后前端 `$reset()` 把毒缓存顺带抹掉，但 `initWsListeners` 幂等（`started` 标记，整会话不再 `ensureRoot`）、ContentArea 的 viewKey watch 已发过、树 onMounted 已跑过、后端开空间只 `start_watch` 不发任何事件——「干净但死寂」：`root=null`、`loaded` 空、`pinned` 空，谁也不再触发加载。「切库再切回」= viewKey 真变化 + 树重挂载，才补上那一次 `ensureDir`，所以能恢复。

## 解法

「空间就绪驱动自愈」三件套（纯前端，零后端）：

- `loadDir` 加 **root 闸**：`ensureRoot()` 后 root 仍 null（未开空间暂态）→ 直接返回，**不入缓存不置 `loaded`**——暂态降级数据从此无缓存可污染，下次调用自然重试；
- `openSpace` / `createSpace` 的裸 `$reset()` 换成 `ws.reseedAfterSpaceChange()`：`$reset` + **代际递增**（在途 `loadDir` 兑现时比对代际，不等即丢弃——防旧空间在途数据写毒复位后的 store）+ 立即重拉根信息（`ensureRoot` 成功分支自带 `loadPinned` + 缩略图补请求）；
- 消费方（ContentArea / WorkspaceTree）**watch `ws.root` 各自重引**——不依赖启动 / 切空间时序（曾评估在 openSpace 尾部按 sidebarSelected 重引，但 restore 线与 openSpace 线并发、时机不可靠）；root 重新就绪是「空间已开」在前端的唯一可信信号。

## 预防

- **暂态降级数据禁止入缓存**——「还没就绪」与「真空」必须可分（与踩坑 100「没了 ≠ 空」同族：100 是状态事实吞错，本条是暂态吞错）；拿不到权威数据就早退，等就绪信号重试。
- **store 复位（`$reset`）必须配「重引」设计**——复位前先列一遍全部加载入口（挂载 / watch / 事件 / 幂等初始化），确认复位后至少有一条会再触发；一条都没有，就把「就绪」做成可 watch 的信号驱动消费方自愈。
- **多异步线并发的启动链路，别靠调用先后保正确性**——时序无关的「信号驱动重引」才稳（本坑若靠调整 openView / 挂载时机来修，只会更脆）。
