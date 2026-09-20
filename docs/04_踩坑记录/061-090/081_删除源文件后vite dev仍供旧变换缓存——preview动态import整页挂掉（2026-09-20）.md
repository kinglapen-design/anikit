# 081 删除源文件后 vite dev 仍供旧变换缓存——preview 动态 import 整页挂掉（2026-09-20）

## 现象

R2 总烧删掉 `AppInput.vue` / `AppSearchBar.vue` 后，`#design-system` 预览页整页白（vue-tsc 与 vite build 全绿）。CDP 探针报 `Failed to fetch dynamically imported module: DesignSystemPreview.vue?t=...`，网络层抓到对已删文件的 404 请求（`AppInput.vue?t=旧时间戳`）。

## 原因

vite dev 的模块图按 `?t=` 时间戳缓存变换结果：PreviewDialogs.vue 重写后，其某个 `?t=` 版本对应的旧变换（仍 import 已删文件）仍在被服务。vue-tsc 读磁盘全绿、curl 单文件也 200（新版本），但页面链路里串着旧时间戳模块——**typecheck/build 绿 ≠ dev server 模块图一致**。

## 解法

`touch` 变动文件触发 vite 失效重变换（或重启 dev server）；确认旧 `?t=` URL 不再含死引用后探针即 ALL PASS。

## 预防

- R2/R3 这类「删文件」轮次收尾：探针跑不通先怀疑 dev server 模块图陈旧——touch 相关文件或重启 vite，别先改代码。
- 探针收尾时停 dev server 要验证端口真释放（2026-09-20 同日踩过：TaskStop 只杀壳进程，vite 子进程残留占 5373，用户 pnpm tauri dev 起不来）。
