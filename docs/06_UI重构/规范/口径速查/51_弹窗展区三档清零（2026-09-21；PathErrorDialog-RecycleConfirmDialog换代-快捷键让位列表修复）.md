# 51 弹窗展区 ③ 档清零（2026-09-21；PathErrorDialog / RecycleConfirmDialog 换代 + 快捷键让位列表修复）

> 用户指 preview 展区两件「样式不对，不是新的弹窗样式」——PathErrorDialog（手写卡片壳）与 RecycleConfirmDialog（手写 del-* 壳）是业务弹窗最后的旧体系遗留。本轮全换代，**业务弹窗 13 件全新体系**；顺手修掉一个真 bug。

## PathErrorDialog（向导内嵌卡片）

不是模态弹窗（SetupWizard 内联，无遮罩），换代 = 卡片壳换新弹窗口径：g3 底 + **1px 通用/五级真 border**（旧 inset 描边废）+ 大阴影 + 圆角 8 + **padding 24 / 列 gap 24**（旧 16 废）；标题 18/26 Bold 一级、正文 13/18 三级（旧 secondary 档废）；**error 徽章废**（§24 alert 系徽章全废的口径延伸）；按钮 = ActionButton lg 宽自适应 + 左右 48 **次左主右**（取消左 / 重新选择路径右——旧 主左次右 + padding 8/16 废；卡片自抄 BaseModal 按钮罩压 lg width:100%）。宽度上限按按钮行实需 = **378**（328 钮行 + padding 48 + 边 2；08 旧 368 上限随换代更新）。

## RecycleConfirmDialog（彻底删除确认）

手写 del-* 壳全废 → **modals/ConfirmModal kind="danger"**：壳与遮罩随组件层（absolute C1-80 遮罩废）；**关闭 X 废**（ConfirmModal 口径：关闭只走取消/确认），Esc 放弃保留；危险确认钮 danger 红（旧品牌蓝废），次左主右；标题/确认钮文案随 kind（彻底删除 / 清空回收站）直给同一字符串。警示文案改分段渲染，**数量词高亮 = 加粗 + 亮色/二级**（§24 pw-lines b 口径）——3 个素材 / 2 个虚拟片段 / N 条链接。

## 快捷键让位判定修复（真 bug）

`useGlobalKeys` 两处遮罩查询还是 `.app-menu, .zoom-mask, .modal-mask, .del-mask, .pick-mask`——modal-mask 随 AppModal 死、del-mask 随本轮死、pick-mask 早死；**新弹窗群（.kit-mask / .bm-mask）从没进过列表**——开着任何 BaseModal/ConfirmModal 弹窗时 F2 / Del / Ctrl+C 等操作键曾不让位（弹窗开着能删底层选中文件）。已更新为 `.app-menu, .zoom-mask, .kit-mask, .bm-mask`。

## preview 同步

③ 档「手写壳」消灭：RecycleConfirm 格换 pv-cm-shell 复刻（pv-cm-btn 补 cancel / danger 两档 + pv-cm-body b 高亮档），PathError 格真实直渲 caption 更新；**pv-old-* 复刻样式全删**（pv-old-shell / pv-old-titlebar / pv-old-title / pv-del-*——旧体系复刻皮随 ③ 档清零全灭）。展区 desc 与分档注释重写：12 件业务弹窗 + OrphanInfosAlert 全部新体系。
