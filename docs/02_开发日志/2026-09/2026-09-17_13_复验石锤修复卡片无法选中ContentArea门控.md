# 2026-09-17_13_复验石锤修复 · 卡片无法选中（ContentArea setActive 与 click 链冲突）

## 现象与定位

用户实机报「严重问题，文件都不能选中了」（点卡片无选中环、右栏不出文件详情），并锁定在复验修复批之后。单变量排查：**回滚 ContentArea 的 `@pointerdown="panelApi.setActive()"` 后选中立即恢复**（实机截图：WAV 卡带品牌环 + 右栏详情）。根因 = 复验修复③「分屏空白聚焦」的整区 pointerdown 激活——卡片 pointerdown 上调用 `panelApi.setActive()` 后，dockview 激活链（含项目补丁 withOrigin）引发的即时重渲染把 in-flight click 的目标元素在 pointerup 前换掉，**click 事件丢失** → onCardClick 根本不触发（所有卡片全灭，hover 不受影响）。

## 修复

`onRootPointerdown` 门控——**只对真空白激活**：事件目标在 `.asset-card / .list-row / .folder-card / .archive-card / .tree-item / button / input / textarea / .app-menu / a / 分组标题 / 空组提示框 / 滚动条` 内时不激活。卡片 pointerdown 上 setActive 永不触发（构造上保证 card click 链完整）；空白 pointerdown 激活满足「点空白聚焦 pane」原诉求。已知取舍：非激活 pane 的空白 click 若触发激活重渲染，本次框选清空选中的 click 可能同样被换目标吞掉（激活 pane 下不受影响——setActive 幂等不触发）；如有异样报走查。

## 顺手

文件夹单卡 180→220 回宽（实机走查「单卡太窄太挤」；多卡行仍 minmax(180,1fr)）。

## 验证

`pnpm typecheck` 0 错；实机截图验证（撤版：选中环 + 详情恢复；门控版 = 卡片链构造上无 setActive）。待用户走查：卡片选中恢复 / 空白点击聚焦 pane / 分屏互拖不受影响。
