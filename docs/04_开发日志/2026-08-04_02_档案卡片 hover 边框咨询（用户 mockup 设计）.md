# 档案卡片 hover 边框咨询（用户 mockup 设计）

**档案卡片 hover 边框咨询（用户 mockup 设计）**：异形装饰（回形针 / 错位纸张）处理——边框只沿卡片主体矩形画，装饰 `pointer-events: none` 排除在判定与描边外；错位纸张首选收进卡片容器内部（FolderCard 既有模式：装饰纸在 icon-wrap 内、描边 ::after 沿容器，零判定成本），必须溢出时排除 + hover 联动提亮防割裂。