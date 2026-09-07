# 4. 行内重命名（一律 `useInlineRename`）


- 任何行内重命名（树节点 / 卡片 / 工程文件 / 先建后命名）**必须**走 `useInlineRename` 封装：词干选区不包后缀、blur 恒提交、Enter/Esc 语义、扩展名兜底补回。**禁**另写输入态/双击判定。
