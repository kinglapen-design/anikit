# 73. 工作空间树缩进线对齐库树（2026-10-21；.tree-guide 共享类）

- **口径**：工作空间树（「文件夹」区块主树 + 「常用」pin 行子树）画与库树同款缩进竖线，数值全沿库树既有定案（2026-08-08 简化定案 + 2026-08-30 五轮实机）——横坐标 = depth×20+15（父级 item 左 padding 6 + depth×20 + 展开钮半宽 9，正对父级展开钮尖角），纵贯首子项顶部至末子项底部，宽 1px，线色 = 常态-2 半透明 `--n-custom-2-50`；线沉在 item 选中/悬停罩层之下（不设 z-index，罩层半透明透过自然变暗属预期）。**ws 目录无自定义色概念，恒走缺省档**（不传 --guide-color；库树变色档 = 该色-2 的 50% color-mix 派生维持不变）。
- **抽象**：缩进线机制自 FolderTree scoped 样式抽为 base.css 通用类 **`.tree-guide`**（position:relative + ::before 画线；`--guide-x` 定位、`--guide-color` 色档缺省回退 `--n-custom-2-50`），FolderTree / WsTreeRows / WorkspaceTree（pin 行）三处共用；FolderTree 行为零变化（探针 W3 段回归钉死，含蓝色节点 color-mix 档）。
- **挂法**：子层容器挂 `class="... tree-guide"` + 行内 `--guide-x: ${depth*20+15}px`（depth = 父级行缩进档；pin 行 depth 0 → 恒 15px）。
- **不画线处维持不画**：目的树选择器 MoveNode（稿无，2026-10-03 §67 口径）不动。
- 探针：`tools/cdp-ws-tree-guide-probe.py`（W1 depth 0/1 两档线位与贯穿高度 / W2 pin 行 / W3 库树回归含自定义色档；截图 ws-tree-guide.png / ws-tree-guide-lib.png）。
