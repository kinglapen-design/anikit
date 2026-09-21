# 47 新建分组与分组管理统一组卡片口径（2026-09-21；GroupNameInput 抽件）

> 用户实机指 preview 展区两弹窗规则行观感不一：「这两个是一个业务的，统一成下面的（分组管理）」。

## 差异根因

两处规则行是同一个 GroupRulesEditor 的 `.rule-row`（g3 底 + inset 描边 a3）——分组管理里它落在 g6 组卡片上清晰可见，GroupEditDialog（新建分组）里直接贴在 g3 壳上**同底隐身**，只剩描边微光。不是样式分叉，是语境底色对比缺失。

## 落地口径

**GroupEditDialog 组块 = 组卡片**（与分组管理同口径）：通用/六级底 + 立体感-通用 + 圆角 8 + padding 12，旧组块底线废；规则行 g3 块在 g6 卡片上自然显形。名称行随卡片内形态 = 42 高（padding-bottom 12 + 底线 亮色/禁用 + 下距 12，行高由 30 名称框撑起）；**「分组名称:」标签保留**（34 稿有，单组新建语境引导用；分组管理多组列表语境稿无标签，不反向统一）。滚动区 padding 0·24 → 12·24 撑卡片上下间距。

**GroupNameInput 抽件**（`src/components/GroupNameInput.vue`）：组名称框共用衍生 = 30 高 / padding 6·12 / 通用/四级底 / 文字亮色/一级，TextInput sm 提权（描边 半透明-3、圆角 8 合稿不动，左对齐归基件 §46）。§42「衍生覆盖写在本组件」的名称框覆盖在 GroupEditDialog 出现第二处使用，升格为共用件——GroupManageDialog 原覆盖删除迁入，preview 两格同嵌真实件。

## §42 待办勾销

§42 记「GroupRulesEditor 内部与稿不符（行容器 / 添加规则钮 / 间距）另立项」——本轮用户认可分组管理现行观感（含 g3 行块、g5 添加钮）为定稿形态，02 稿「规则 item 无底色」等差项不再追，**该待办勾销**。编辑器内部本轮零改动（g3 行块 + g5 添加钮在 g6 卡片上对比均成立）。

## preview 同步

PreviewDialogs：新建分组格组块卡片化 + 名称行 42 形态（pv-bm-body flush 改回 12·24 padding 撑卡片间距）；两格名称框嵌真实 GroupNameInput；pv-gm-name 提权样式删（含 §46 左对齐清理漏网的 text-align:left 残留）；caption 同步。
