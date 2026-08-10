---
link: "[[00_组件目录]]"
---
整体布局为：标题 + 快捷键（如果有的话），例子：新建 ctrl+n

标题为：[[01_样式 token#02. 文字]] / 正文 常态

快捷键：[[01_样式 token#02. 文字]] / 正文 标签（快捷键带 C3 底 + 暗-悬停边框的小徽章，多字母 padding 2px 4px、单字母 2px 6px）

``` css
/* 工具tip（2026-08-07 按 figma-css 06_悬停 tip 更新） */
box-sizing: border-box;

/* Auto layout */
display: flex;
flex-direction: row;
align-items: flex-start;
padding: 6px 8px;
gap: 12px;

position: relative;

background: C2;
border: 1px solid 亮-选中;
box-shadow: 0 4px 40px rgba(0, 0, 0, 0.6);
border-radius: 8px;
```

## 换行与限宽（2026-08-10，卡片名称悬停 tip 定案）

- tip 容器 `max-width: min(520px, 100vw - 16px)`、`white-space: normal`、`word-break: break-all`——长名称在 tip 内换行，不被主窗口截断（配合既有水平钳制与下缘翻转）。
- 名称悬停 tip 用例：文件夹卡片 / 素材卡片名称 span 上 `v-tip="{ text: 完整名称, keys: 当前简介, column: true }"`——**column 变体 = 名称 / 简介两行竖排（gap 4，2026-08-10 用户定）**，简介档不被长名称挤压（`flex: none`）；素材卡片 `briefMode none` 时不传 keys。默认横排（按钮 tip "新建 Ctrl+N"）行为不变。
