# dockview params 非响应式与 pinia computed 缓存——预览计数停留首视口（2026-10-03）

## 现象

预览标题栏索引「x / y」的总数停留在**本次会话首次打开预览的视口**文件数，之后切任何文件夹再开预览都不刷新：在「全部」（1338）开过预览后，进只有 1 个文件的文件夹双击预览显示「1 / 6」（首视口恰好 6 项）。更迷惑的是上/下一个按钮**是亮的**（按缓存总数 6 判定可用），点击却**毫无反应**（step 按实时列表 1 项，next=1 越界 return）。

## 原因

tabs store 文件头自述「dockview api 非响应式（内部对象大且自管理）；tabs / activeTabId 是面向组件的只读镜像」，但 `viewKeyOf` 直读 `api.getPanel(panelId).params.viewKey`——**非响应式取值**。preview store 的 `index` / `total` 是 pinia options getter = computed：

- `assetKey` 在 `index` 的依赖里——切文件夹再开预览时 assetKey 变 → index 重算（此时 viewKeyOf 取到**新**视口，findIndex 正确，显示「1」）；
- `total` 的依赖只有 `placementsFor` 内部的 placements / sortBy 等——**viewKey 取值不构成依赖**。切视口后 placements（全量库数据）没变 → computed 缓存不失效 → total 停留首视口值。

「当前位对、总数旧」的诡异组合由此而来。按钮 disabled 用缓存 total（6 → 下一个可用亮起）、`step` action 每次实时取列表（1 项越界 return）——判定数据源分裂把「亮但点不动」的迷惑性拉满。

## 解法

`viewKeyOf` 先读响应式镜像 `tabs.value.find(...)` 建立依赖，取值仍以 `api` 实时为准（syncMirror 前的瞬态不读旧值）：视口替换 → updateParameters → onDidLayoutChange → syncMirror 重建 tabs → getter 失效重算。锚点加在取值唯一入口，current / index / total 三个 getter 同愈（step 是 action 本就实时，无须改）。

探针 `cdp-preview-count-probe.py` 双向验证：stash 修复码反向复现「项目D 预览 1/6 + 下一个 disabled=false」用户现场；恢复后正向断言计数 6→1→3 随视口刷新、单文件禁用、多文件可导航，双绿。

## 预防

- **pinia computed / getter 里取的外部数据源，先问「它响应式吗」**——dockview api.params、DOM、原生窗口对象都不是。非响应式源必须经响应式镜像锚定（touch 镜像建依赖）或事件驱动写进 store state，禁止直读。
- 本项目 tabs / activeTabId 镜像早已存在，缺的是「读镜像」的纪律——镜像存在的意义就是给 computed 当依赖，绕过镜像直读 api 等于自废武功；**取值收口为唯一入口的价值 = 锚点只需加一次**。
- 「一半刷新一半不刷新」是 computed 依赖缺失的典型信号（本坑 = index 对、total 旧）——遇到先列该 computed 的全部取值源，逐个查响应式，比加手动刷新补丁靠谱。
