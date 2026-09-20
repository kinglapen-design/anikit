# 40 阶段七 R2 总烧轮（2026-09-20；AppInput/AppSearchBar 烧删，弹窗群 BaseModal framed 换代）

> 2026-09-20 R2 总烧：四组并行迁 24 个消费方文件，AppInput / AppSearchBar 烧删；AppButton / AppModal 余 FailedListDialog / OrphanInfosAlert 两消费方（alert 系无新稿待用户定）；根目录 IconButton 余 6 消费方（filled/danger/deleteRed/active 变体无新稿待用户定）。台账：141 件 = 新 66 / 业务 70 / 旧件 3 / 休眠 2。

## 弹窗壳换代口径（AppModal standard → BaseModal framed）

- 宽 = **稿值直给 `:width`**（border-box 含 1px 边；旧 AppModal 时代「内容 + 48 padding + 1.5px border 补偿」算法废除——有一格曾按 552+48+2 重补偿，统一为稿值直给，差 1px 走查消化）。
- framed = 标题栏 54 高带底线 + 关闭 X（plain close-large）+ 壳 padding 0；内容段自承 `padding 0 24`；旧 `:deep(.app-modal)/.titlebar/.content` 覆盖全删。
- 视口封顶 BaseModal 不带：需要的弹窗自加 `:deep(.bm-modal) { max-width/max-height: calc(100vw/vh - 96px) }`。
- 滚动：BaseModal 无内建兜底滚轮——内容可超高的弹窗自管 `.scroll-wrap > .kit-scroll(ref)` + ScrollThumb（贴弹窗右缘，framed padding 0 下包裹层天然通栏，负 margin 对子不再必要）。
- 按钮行 = #actions 插槽：`ActionButton lg` 宽自适应 + 左右 48px，**次左主右**（§1 新口径，本轮起全部翻过来）；旧 88/104 定宽一律废。

## 嵌套确认 → ConfirmModal

alert error → kind="danger"、alert warning → kind="warning"、confirm 无险提示 → kind="brand"；confirmLabel/cancelLabel 保旧文案（「修改分组」「删除」「开启」等）。**提示音不丢**：ConfirmModal 不带音，宿主 `watch(弹开态) → playAlert()`（GroupEdit/GroupManage/BlockManage/App.vue 同款）。ConfirmModal 暂无 disabled 能力——busy 重入由 handler 内闸挡（记待办，接入时补档）。

## 控件补能力与缺口

- TextInput 补 `clearable`（clear 12 亮色/四级 hover 升三级，150ms）+ disabled 30% 透明度；ActionButton 补 disabled prop + 30% 透明度（§39② 控件族口径接入）。
- **AppSearchBar 替代形态定案 = TextInput `leading-icon="search"` + clearable + lg**（不再单立搜索框件）。
- 未烧缺口（无新稿，待用户定稿后烧）：AppModal/AppButton（FailedListDialog、OrphanInfosAlert 两件 alert 系）、根目录 IconButton 的 filled / danger / deleteRed / active 四变体（消费点：WindowControls 关闭红、DupScanGroupRow/DescEdit/GroupManage/BlockManageRow 红删、侧栏/标签 active 态）。
- 已知观感差（新档既定，走查确认）：TextInput sm 文字居中（稿年月日小框语境）被套用到描述/分组/区块行内名称框——如应左对齐需补档（记待办待用户）；IconButton small→bare sm 图标 12→14、热区变小。

## 探针账

旧 selector（`.app-modal` / `.app-btn` / `.dup-bottom` 等）的探针（cdp-duplicate-dialog / cdp-scan-duplicates / cdp-wizard-* 等）随换代失效——**随用随修，不主动全量重跑**（探针纪律：断言集中改一批再跑）。vite dev 模块图缓存坑记 [踩坑 081](../../../04_踩坑记录/061-090/081_删除源文件后vite dev仍供旧变换缓存——preview动态import整页挂掉（2026-09-20）.md)。
