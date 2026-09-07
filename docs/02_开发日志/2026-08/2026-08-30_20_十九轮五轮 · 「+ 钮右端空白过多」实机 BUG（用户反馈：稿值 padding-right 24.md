# 十九轮五轮 · 「+ 钮右端空白过多」实机 BUG（用户反馈：稿值 padding-right 24）
## 十九轮五轮 · 「+ 钮右端空白过多」实机 BUG（用户反馈：稿值 padding-right 24）

- 现象：标签较多时 + 钮右端多 ~60px 无名空白；桩环境（DPR 1）不复现。定位 = dockview 7 **标签溢出下拉**（`.dv-tabs-overflow-dropdown-root`，渲染进 right-actions）：OverflowObserver 按 scrollW > clientW 判溢出，**DPR 缩放下 flex shrink 亚像素取整累积**恒假溢出 1-2px → 下拉恒显占位。本项目 shrink 平分策略标签永不下车，下拉无意义。
- 修法：`DockviewVue` 传 `:disable-tabs-overflow-list="true"`（原生选项根治）。排查中附带加固：T13 新立 pad-win 生命周期断言（右栏收起 160 让位浮块 / 展开解除回 24；sidebarAnimating 复位——动画 + RO 灌宽时序不定，断言改轮询等待，原固定 sleep 在 7 标签拥挤布局下 flake 过一次）。
- 探针 T14 新立（7 标签强 shrink：无下拉元素 + right-actions 宽 0 + + 钮右缘距栏内容右缘 = 24 稿值）；连续两跑全 16 项 PASS；踩坑 061 新立；速查表 §21 补第三关。

