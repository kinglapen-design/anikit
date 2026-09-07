## 055 grid `0fr→1fr` 宽度推挤动画只适用外部定宽容器——自适应容器里 0fr 轨道仍占 max-content 宽度

- **现象：** 2026-08-29（阶段七十轮，拉下按钮-横向）。容器内容自适应（inline-flex 无显式宽）时，未展开项包 `display: grid; grid-template-columns: 0fr` + 内层 overflow hidden——视觉内容确实被裁掉了，但**容器宽度仍按全部项的 max-content 撑开**：收起态容器带着隐藏项的宽度，选中项与箭头之间一大段空白，"间距全乱"。同机制在 item 族卡片（`.ni-slide`，卡片定宽 240）里完全正常，迁到自适应容器就露馅。
- **根因：** 0fr 在"外部给容器定宽、轨道按比例分配"时才把轨道压到 0；容器走内容自适应（max-content 贡献计算）时，fr 轨道的贡献 = 内容的 max-content——0fr 不减少它。宽度本身就是内容决定的，没有"外部约束"可压。
- **解法：** 自适应容器做宽度展开动画 = **JS 实测内容宽 + 显式 `width` 过渡**：内层 `width: max-content`（任何状态下 offsetWidth 都可测真实内容宽），onMounted 收集各项宽度进响应式数组，wrap 按显隐内联 `width: Npx / 0px` + transition；测量完成前禁 transition（`ready` 类挂载后 rAF 开启）防首帧闪动画。间距用内层左 padding（随宽度一并被裁），不用 flex gap（对 0 宽项不折叠留缝）。
- **预防：** 凡"宽度动画展开/收起"需求，先判容器宽度语境——外部定宽/剩余空间分配 → grid 0fr→1fr 最简；内容自适应 → 实测宽度方案。预览页 demo cell 默认 `align-items: stretch` 还会把自适应宽组件拉宽到 caption 宽，掩盖/放大此类问题（本轮 preview.css 已改 flex-start）。
