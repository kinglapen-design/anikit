# 3. 列表 item（树 / 组列表 / 横条行）


- hover = 叠**暗-悬停**罩层（`--line-hover`）；选中 = **亮-选中**罩层（`--line-active`）+ **1.5px 文字常态**内阴影（与目录树 `TreeItem.selected` 同款，勿自创）。
- 选择域联动描边 = 1.5px `--brand-border`（`domain-selected`）。
- **选中态行 hover 不再变色**（`:not(.cur):hover`）；可点行 `cursor: pointer`；item 圆角 6。
- 复选/单选/开关一律 AppCheckbox / AppRadio / AppSwitch（阶段七新写面板改用 `buttons/Checkbox` / `buttons/Radio` / `buttons/SwitchButton`，见 §24；旧件随迁移退役）；红色圆减号 = IconButton `delete-red` 形态。
- **表单行标签冒号一律英文 `:`**（2026-08-19 用户定，全库统一——「规则:」「范围:」「分组名称:」等；句子/toast 正文里的标点不动）。
- **多值 chips 区换行向下排、行高自适应**（`min-height` 不用固定高）；左侧「× 规则 范围」标签组包 `.row-head` 恒 28 高居首行——chips 变两行时标签**保持首行垂直居中**，不随整体居中（33 稿规则行口径；错误信号：固定 44 高 + wrap → chips 溢出/标签跑两行中间）。
- 菜单项「上移/下移」类**跟在文字右侧的小图标 = MenuItem `trailingIcon`**（12px 亮色/三级，禁用随父色；`trailingIconClass: "rot-180"` 转向；2026-09-01 菜单新稿确认保留——用户原话「这是功能，不能丢」，用新「箭头-小」chevron-small）。
- **右键菜单（2026-09-01 新稿定案，`Design Token/组件/上下文菜单/` 五件 + 用户容器口径；全量细则见组件目录菜单节）**：容器 = **宽度内容自适应**（二十五轮用户改定，废 300 统一宽：名称→**36px**→右件最小间距 = MenuItem flex gap 36，容器宽由最长 item 决定，视口封顶 100vw-8）/ padding 6 / 底 `--n-g3` / 1px `--n-g5` inset 描边 / `--n-shadow-sm` / 圆角 8；item = 26 高 / padding 4 8 / 正文 13/18 `--n-l2` / hover `--n-l-a3` / 禁用 `--n-l-disabled`；分组 = 组容器 gap 2 / padding 上下 4 首末 0 / 非末组 `inset 0 -1px 0 --n-g5` 线（**禁孤立分割线元素**）；对勾 = 右侧 18 盒常态隐 / hover 三级 / 选中一级；快捷键 = 纯文本 Fira Mono 三级（KbdBadge 废）；展开箭头 = expand 14 盒 rotate(-90°)。**「更改颜色」内嵌色块行业务口径（2026-09-01 走查修订）**：十色首块「默认」= **显式默认**——落盘存 `"default"` 哨兵断开父级链路强制常态灰（Folder.color 无值 = 跟随祖先色，子级清 None 会被父色盖回永退不回灰）；**多选批量** = 触发项在多选域内 → 整个选中集一并换色（`set_folder_colors` 单命令单撤销步，同剪切/克隆分流口径）；**档案菜单同接入**（树节点 + 档案卡共用 archiveCardSpec，生效于树档案圆点/图标，与徽标色无关）。**「更改图标」网格 = 30 格（默认 + CUSTOM_ICONS 29 个）——archive 档案专属不可选**（同日移出清单，存量 icon 值开库自检回退 None）。**错误信号**：菜单项左侧出图标（左图标位 2026-09-05 走查部分恢复——仅限设计稿明确的项（新建文档组），其余项乱用即错，占位对齐口径见 §25）/ 菜单带搜索框（已废）/ 菜单写死宽度（300 统一宽已废，改内容自适应）/ 用旧 token（--line-hover/--c2/--text-normal）写菜单 / 子级选「默认」写 None（必须发 "default" 哨兵）/ 多选换色只改触发项（漏批量分流）/ 图标网格出现档案图标。
- 分词输入 = **TermChipsInput**（chips + 回车/空格/+/失焦提交；IME isComposing 守卫——回车空格是选词键，不守卫会把中文输入确认误提交成 chip）。
- **主窗口搜索 = 多功能悬浮栏搜索模式（2026-08-31 用户定案，7.1 二十二轮；2026-08-19 tag 化 chips 旧口径废除）**：业务壳 `SearchFloatBar`（ContentArea 每 panel 一个，`.sf-anchor` absolute 底部居中距底 12、pointer-events 穿透空隙）+ `bars/FloatBar mode="search"`。单行输入实时分词（v-model ↔ store draft，防抖 150 在 store）；`~` 手敲全局（后端剥前缀，`//` 等效直输，`/` 按普通文本搜）；Ctrl+F 聚焦（requestFocus → FloatBar expose focusSearch → FloatBarSearch focus）；Esc = 清空并失焦；筛选按钮组**点亮不接业务**（点击仅切本地选中态 `.on`，不触发搜索——筛选弹窗 9 子稿 + 后端筛选条件另轮）。旧「`/`、`//` 前缀 tag + 空格成 chip + kbd 提示组 + 失焦提交」交互全废。
