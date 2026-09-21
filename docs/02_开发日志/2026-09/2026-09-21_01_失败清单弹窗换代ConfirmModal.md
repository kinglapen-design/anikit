# 2026-09-21_01 失败清单弹窗换代 ConfirmModal（导入失败旧样式根治）

> 用户实机报「导入失败还是旧样式」——FailedListDialog 是 §40 遗留的 alert 系两消费方之一（无新稿待用户定），本轮按 §24 已定映射直接落地。口径记 [速查 §44](../../06_UI重构/规范/口径速查/44_阶段七失败清单弹窗换代轮（2026-09-21；FailedListDialog换ConfirmModal-danger）.md)。

## 改动

**FailedListDialog 换代**：AppModal alert error + AppButton → modals/ConfirmModal kind="danger"。48 error 徽章废（§24「纯靠按钮颜色区分轻重」）；按钮序翻为次左主右（退出左 / 重试右，§40 口径）；按钮文案 confirm-label="重试" / cancel-label="退出"；失败列表沿旧用户定（间距 8 / 上限 160 / 负 margin 对贴右缘），旧 372 限宽随 520 定宽壳废。导入/导出两处（App.vue）同一组件，一并换新。

**提示音归宿主**：ConfirmModal 不带音，App.vue 加 `watch([importFailed, exportFailed]) → playAlert()`，与 deleteLibraryState 同先例——侵入式提示音语义不丢。

**preview 同步**：PreviewDialogs FailedListDialog 改真实弹出（ConfirmModal danger），② 档待稿件减为 OrphanInfosAlert 一件；台账脚本注释同步，`pnpm inventory` 刷新：旧件余 2（AppButton/AppModal，消费方全在 OrphanInfosAlert）。

## 验证与未验证项

`pnpm typecheck` 绿。未验证项交实机走查：导入失败弹窗开合（重试红色钮 / 退出左重试右 / 无徽章）、提示音、长文件名折行与列表超高滚动贴右缘；导出失败同组件一并过。探针账：cdp-import-failed-probe / cdp-export-failed-probe 断言旧 alert 壳失效，随用随修（§40 纪律）。

## 余账

OrphanInfosAlert（开库孤儿警告）仍旧 AppModal alert warning + AppButton——单按钮「知道了」形态 ConfirmModal 不支持（固定双钮），稿未画单钮弹窗，待用户定稿（ConfirmModal 加单钮形态，还是该窗改双钮）。
