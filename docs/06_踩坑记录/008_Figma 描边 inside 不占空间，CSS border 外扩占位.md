## 008 Figma 描边 inside 不占空间，CSS border 外扩占位

- **现象：** 按 Figma 标注的 padding 实现后，所有带边框容器的视觉间距都比设计稿宽 1~2px（如主窗口 padding 6 看着像 8）。
- **原因：** Figma 描边（stroke）默认 inside 对齐、不占布局空间，标注 padding 6 中有 2px 被描边占用；CSS `border` 在 padding 之外另占空间，`border 2px + padding 6px` 视觉就是 8px。每个带边框容器都差一个边框厚度。
- **解法：** 组件线条一律用 `box-shadow: inset 0 0 0 {宽}px {色}` 模拟（不占布局空间，与 Figma inside 描边逐像素一致）；hover/选中换色改 shadow 即可，还省掉了"透明边框占位防抖动"的 hack。
- **预防：** 已写入 `01_开发规范` 第 4 条——UI 组件禁止用 `border` 画设计线条，统一 inset box-shadow（真正分隔内容用的分割线另议）。
