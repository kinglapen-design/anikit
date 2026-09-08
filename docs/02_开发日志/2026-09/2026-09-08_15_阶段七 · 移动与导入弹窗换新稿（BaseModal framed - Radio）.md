# 阶段七 · 移动与导入弹窗换新稿（BaseModal framed / Radio，2026-09-08）

任务：布局/模态窗口/ 四稿中剩余两件——移动.md、导入弹窗.md 新样式替换（创建/合并已于 §33 轮落地）。用户指令 = 新样式替换，业务零改。

## 移动面板（MovePanel + MoveNode 换代）

壳换 modals/BaseModal 新扩 `framed` 变体：460 宽稿值、壳 padding 0 / 列 gap 12、标题栏 54 高（padding 12/12/12/24 + 底 1px 通用/五级通栏 + 关闭 X = buttons/IconButton plain close-large）、高 min(822, 100vh−96)。搜索框换 inputs/TextInput lg + 新扩 `leadingIcon`（左置 search 图标 18 亮色/三级，稿占位「搜索文件夹...」）。区块标题换 items/SectionTitle block 型（with-more=false；添加钮稿有画但移动窗新建文件夹未立项——点亮不接业务，同菜单栏口径）。树 item 换 items/TreeRowItem（MoveNode 重写）：落点三分 hover 直接接 TreeRowItem `dropZone` prop（插入线 2px 品牌 / 并入 品牌半透明填充 + 2px 描边，与侧栏拖拽同口径），旧 TreeItem dropZoneOverride 退役；区块与节点展开/收起补 lib/expandAnim 150ms（树范例）；层级缩进 6+depth×20，不画缩进竖线（稿无）。业务零改（move_placements / move_folders + alert warning 确认——AppConfirm 随旧件迁移轮再换）。

## 导入弹窗（ImportDialog + SequenceDialog 换代）

壳换 BaseModal 312 宽稿值；单选换 buttons/Radio；按钮换 ActionButton lg（恒 128×38，次左主右——旧 27_导入稿 88×38「确认左」退役）。双组分隔按稿：来源组下 padding 12 + 层级组顶 1px 亮色/半透明-3 + 上 padding 12。提示行 = 正文档 13/18 亮色/四级；来源组提示换稿定新句「请选择复制一份导入或剪切导入」。

**序列帧窗维持只问打包/分散**：导入弹窗稿「检测到序列帧」内画了来源组（复制/剪切），判定为旧 figma 27 §3 同款复制残留（2026-08-12 用户已定：来源已在导入弹窗选过；且混合导入时层级与打包两问无法并为一组，两步流不能并），不重复落地。若实机确认为有意合并，再立项改流程。

## 验证与文档

vue-tsc / build 全绿。探针：cdp-import-dialog-probe 重写（入口改菜单栏 菜单→文件▸——旧 .add-btn-wrap 新增菜单已随菜单栏轮撤下）三窗全 PASS + 截图 3；cdp-move-panel-probe 断言更新（bm-modal / 宽 460 / SectionTitle 计数 / X = .ic-btn）S1-S6 全 PASS + 截图 2，目验贴稿。预览页 PreviewModals 补导入三真实弹出（mock draft 不接后端）+ 移动 framed 静态复刻（真实组件拼装：标题栏 + TextInput leadingIcon + SectionTitle + TreeRowItem 三行含并入高亮）。速查表 §34 新立、§33 待办勾销、§23 SectionHeader 残留改仅 InboxDiskSection、待办细节清单 2026-08-30 ⑧ 同步。

未验证项（交实机走查）：移动窗树 hover 三分手感与展开动画实机观感；framed 标题栏关闭钮 hover（close-large 为烘焙二级原值不变色，hover 仅出半透明-3 底——稿未画 hover）。
