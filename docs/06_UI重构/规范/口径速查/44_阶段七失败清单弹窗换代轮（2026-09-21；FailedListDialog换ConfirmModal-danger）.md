# 44 阶段七失败清单弹窗换代轮（2026-09-21；FailedListDialog 换 ConfirmModal danger）

> 用户实机报「导入失败还是旧样式」。§40 遗留缺口「AppModal/AppButton 余 FailedListDialog / OrphanInfosAlert 两消费方（alert 系无新稿待用户定）」本轮定一半：FailedListDialog 落地换代，OrphanInfosAlert 单按钮形态仍待稿。

## FailedListDialog 换代口径

导入/导出失败清单弹窗由旧 AppModal alert error + AppButton 换代为 **modals/ConfirmModal kind="danger"**——即 §24 嵌套确认映射（alert error → danger）在 alert 系直用方的落地，样式随 ConfirmModal 组件层（520 宽 / padding 24 / 标题 18/26 Bold 亮色一级 / 正文 13/18 三级 / 按钮 38 高宽自适应 + 左右 48）。

- **48 error 徽章废**：§24 已定「旧 alert 48 徽章没了，纯靠按钮颜色区分轻重」——重试钮 = danger 色底即错误语义载体，不再配红圈图标。
- **按钮序 = 次左主右**：退出（cancel）左 / 重试（confirm）右。旧弹窗「重试左退出右」是 §40 前的旧口径，本轮随换代翻过来。ConfirmModal 按钮文案由 props 直给：confirm-label="重试" / cancel-label="退出"。
- **提示音不丢**：ConfirmModal 不带音，宿主 App.vue `watch(content.importFailed / content.exportFailed) → playAlert()`（§40 口径，与 deleteLibraryState 同先例）；preview 演示不播音，caption 已注明。
- **失败列表沿旧用户定**：文件名竖排间距 8、超高自滚上限 160、path 缺失项回退显示原因；滚动 = kit-scroll + ScrollThumb + 负 margin 对贴弹窗右缘（壳 padding 24 非 framed，负扩/补回对子沿用）。旧「限宽 372（AppConfirm 正文口径）」随 520 定宽壳废除。

## 台账与余账

`pnpm inventory` 刷新：148 件 = 新 72 / 业务 72 / **旧件 2**（AppButton / AppModal）/ 休眠 2，旧件消费方 2 处全在 OrphanInfosAlert。LEGACY 表保留两件登记，注释更新为「余 OrphanInfosAlert 一消费方」。

**OrphanInfosAlert 仍待稿**：单按钮「知道了」形态 ConfirmModal 不支持（固定取消+确认双钮），「警告与错误」稿未画单钮弹窗——按红线未自创，preview caption 标「仍旧 AppModal alert，待稿」不变。待用户定：ConfirmModal 加单钮形态，还是该窗改双钮。

## 探针账

`cdp-import-failed-probe.py` / `cdp-export-failed-probe.py` 断言旧 alert 壳 + 48 error 徽章，随换代失效——按 §40 探针纪律**随用随修**，不主动重跑。

## preview 同步

PreviewDialogs ② 档「仍旧 AppModal alert 待稿」由 2 件减为 1 件（OrphanInfosAlert）；FailedListDialog 改真实弹出（ConfirmModal danger，重试/退出仅关闭），caption 注明换代口径。
