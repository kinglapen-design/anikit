## 023 dockview-vue 的 props 传递约定：整组包在一个 `params` prop 里

- **现象：** 自定义 panel / tab / header-actions 组件报 `Missing required prop: "api"`、setup 里读 `api.isActive` 抛 TypeError。
- **原因：** dockview 类型定义给人的直觉是 Vue 组件按字段收 props（`api` / `containerApi` / `params`…），但 dockview-vue v7 源码（`VueRenderer.init`）实际把整组 framework props **包进名为 `params` 的单一 prop**：组件收到的是 `{ params: { params: 用户参数, api, containerApi, tabLocation } }`；`updateParameters` 后该对象整体换新（响应式跟随）。文档未写明这一点。
- **解法：** `defineProps<{ params: Record<string, unknown> }>()`（字面量声明绕开 rolldown 无法解析 node_modules 类型的限制）+ 使用处断言回 `IDockviewPanelProps` 等类型；读用户参数用 `props.params.params.viewKey`。
- **预防：** 用 dockview-vue 自定义任何渲染器（panel / tab / header actions / watermark）都按"`params` 单 prop 包裹"约定接；第三方库 props 行为以源码为准、类型定义只作参考。
