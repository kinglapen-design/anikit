## 024 dockview-vue 多根 Fragment + 父级 scoped 尺寸失效：dock 区域全黑且无报错

- **现象：** 接入 dockview 后中间面板全黑（连标签栏 DOM 都"看不见"），控制台**没有任何报错**——组件全部正常创建，纯属 CSS 事故（同类事故见 021）。
- **原因：** DockHost 里给 `<DockviewVue class="dock-root">` 配 scoped 样式 `.dock-root { height: 100% }`。DockviewVue 渲染为 **Fragment 多根**（容器 div + portals 组件），父组件的 `data-v-*` scope id 不会自动落到多根组件的任何节点——scoped 规则匹配失败，容器高度为 0，`api.layout(w, 0)` 后全部内部元素绝对定位在 0 高容器里 = 全黑。
- **解法：** 尺寸规则改 `:deep(.dock-root) { width/height: 100% }` 穿透。
- **预防：** 给**第三方多根组件**传 class 配尺寸/布局样式时一律 `:deep()`（或全局样式表）；单根组件根元素才继承父 scope id。"无报错但不显示"先量 `getBoundingClientRect()` 再怀疑 JS。
