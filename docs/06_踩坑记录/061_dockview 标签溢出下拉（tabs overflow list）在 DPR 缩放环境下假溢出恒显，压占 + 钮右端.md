## 061 dockview 标签溢出下拉（tabs overflow list）在 DPR 缩放环境下假溢出恒显，压占 + 钮右端

- **现象：** 2026-08-30（阶段七 7.1 十九轮五轮，用户实机）。标签较多时 + 钮右端多出 ~60px 无名空白（用户报「+ 距离右边太多」）；桩环境（DPR 1、1440 窗）不复现。排查链：elementFromPoint / 各容器 rect 全正常 → 复现脚本多标签 shrink 场景对比 → 定位 = `.dv-tabs-overflow-dropdown-root`（chevron + 数字钮）渲染进 `.dv-right-actions-container`。
- **根因：** dockview 7 的 OverflowObserver 以 `hasScrollX = scrollWidth > clientWidth` 判定标签溢出；**DPR 125%/150% 缩放下 flex shrink 的亚像素取整累积**（7 枚 tab 各自四舍五入）可使 scrollWidth 恒超 clientWidth 1-2px——下拉恒显。本项目标签页 = shrink 平分策略（180 固定 basis + flex 0 1 auto），标签永不下车，下拉控件本就无意义。
- **解法：** `DockviewVue` 传 `:disable-tabs-overflow-list="true"`（dockview 原生选项，showTabsOverflowControl 关掉，observer 不挂）。探针补 T14（7 标签强 shrink 场景：无下拉元素 + right-actions 宽 0 + shrink 撑满时 + 钮右缘距栏内容右缘 = padding-right 24 稿值）。
- **预防：** dockview 升级/改版后先列其**标签栏附加件清单**（059 自绘滚动条、060 scrollable 裁层、061 溢出下拉——dockview 7 的 titlebar 远不止 tabs 一行）；实机 DPR 缩放环境的 1-2px 级布局问题 headless（DPR 1）探针复现不了，实机报告优先往「亚像素 + 溢出判定」方向想。
