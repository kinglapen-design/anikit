# 二十一轮十一轮（试验→退回：常态 1px 环 border → inset box-shadow 无改善）
## 二十一轮十一轮（试验→退回：常态 1px 环 border → inset box-shadow 无改善）

- **背景**：三件套后亮图角部改善、hover 抖动缓解，纯黑卡片仍明显；用户定「试 box-shadow，效果更好统一替换，不行退回」。1px 零模糊 box-shadow 本为速查 §7 允许档（被否渗线的是 1.5px 档，勿混淆）。
- **经过**：卡片族常态 1px 环 `::after` 绘制 border → `box-shadow: inset 0 0 0 1px` 全族改完验证（vue-tsc 绿、r11 留档无破版），用户实机对比**无改善，当日退回 border 版**（cards.css + ListRowCard hover g5 + ArchiveCard drop-into 2px + 三组件 naming/selected 让位全还原）。
- **结论（防再试）**：圆角平滑的变量不在 border/box-shadow 绘制路径——cards.css `::after` 块注释已立「勿再试」警戒；观感靠三件套（§7），纯黑卡 hover 位移抖动 = 合成层亚像素重采样固有特性，后续要动只能动位移稿值（-4px/250ms，用户定夺）。

