# 084 Vue watch 实际触发顺序 ≠ 注册顺序——滚动保存被"抢先的 relayout"污染（2026-09-23）

## 现象

视口滚动持久化上线后用户实测全灭：大列表滚到中部，切文件夹再切回来必回顶端；链接文件跳转到目标卡所在视口也停在顶端。探针（`tools/cdp-scroll-restore-probe.py`）逐环插桩后拿到铁证（`__vkLog`）：

``` text
items  → vk=folder:fb, pos=1155   ← items watch 先跑：relayout 把 pos 夹到新视口 max（fb 内容矮 → 0）
viewKey → old=folder:fa, pos=0    ← viewKey watch 后跑：保存了被污染的 0
```

`useGridLayout` 里 `watch(opts.viewKey)` 虽**注册在** `watch(opts.items)` **之前**（注释还写着"同 tick 内按注册顺序触发"），实际触发顺序却是 **items 先、viewKey 后**。

## 原因

两个 watch 的依赖是**不同的 props**（`:items` 与 `:view-key`），由 ContentArea 一次 render 先后 patch——**patch 顺序 = 模板声明顺序（`:items` 在 `:view-key` 前）**，各自触发的 watch 按入队先后执行，**与 setup 里的注册先后无关**。于是一连串全崩：

- **保存污染**：items watch 的 relayout 先把 `pos` 夹到新视口（矮视口 → 0），viewKey watch 随后把 0 当旧视口位置存下——恢复自然是 0；
- **恢复失效**：viewKey watch 的 `restorePos` 在 relayout **之后**才就位，本轮恢复落空，还悬到下一次 relayout 继续作祟；
- **reveal 回拉**：链接跳转时 reveal 虽在 relayout 里消费滚到了目标卡，但迟到的 `restorePos=0` 悬着，下一次 relayout（headerRo/RO 很快触发）又把视图拉回顶端——用户看到"跳转停顶端"。

## 解法

`watch(opts.viewKey, cb, { flush: "sync" })`——props patch 赋值的**瞬间同步执行**，抢在任何排队的 relayout 之前：保存拿到未污染的 pos，`restorePos` 先于 relayout 就位。一修同治三症。探针 S1-S5（保存未污染 / 切回恢复 / 同视口 reveal / 跨视口 reveal 覆盖恢复位 / 各视口按 key 保存）全 PASS。

## 预防

- **两个 watch 若有先后依赖（一个的产物是另一个的输入），禁止靠注册序假设顺序**——需要"A 必先于 B"时 A 用 `flush:"sync"`，或合并成一个 watch 里顺序执行。
- 调试响应式时序别靠推理：CDP 探针页面内插桩（调用间谍 + 事件流水日志 + pinia state **页面内 `JSON.stringify` 后返回**——CDP `returnByValue` 直接序列化 Vue reactive Proxy 会得 `{}` 空壳）一次拿铁证，比猜快。
- 排查教训：config.json 里读到 `scrollPositions` 几乎全 0 时，先区分"保存了 0"与"后被覆盖"——恢复失败 → 停 0 → 再切走保存 0 会抹掉第一现场。
