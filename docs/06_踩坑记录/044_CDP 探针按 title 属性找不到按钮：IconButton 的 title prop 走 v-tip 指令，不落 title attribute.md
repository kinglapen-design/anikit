## 044 CDP 探针按 title 属性找不到按钮：IconButton 的 title prop 走 v-tip 指令，不落 title attribute

- **现象：** 2026-08-20 新探针断言面包屑按钮：`[...document.querySelectorAll('.panel-toolbar .icon-btn')].find(b => b.title === '后退')` 恒 undefined。
- **原因：** IconButton 的 `title` prop 绑给 `v-tip` 自定义指令（悬浮 tip 体系），**不落原生 title attribute**——`el.title` 读不到。同族：菜单项文案在 `.menu-item .label` 文本里（探针一直按文本找，正确）；多 panel 并存时 `.panel-toolbar` 有多个，非激活 panel 的工具栏也在 DOM 里（offsetParent 为 null）。
- **解法：** 探针按**结构与序位**定位：可见工具栏（`offsetParent !== null` 过滤）`.crumb-group .icon-btn` 的序位（非档案视口 = 后退 / 前进两枚）；断言禁用态读 `button.disabled`。
- **预防：** 探针定位器优先级：data-* 属性 > 文本内容 > 结构序位；**不要依赖组件 prop 与 DOM attribute 的同名假设**——封装组件（IconButton / AppInput 等）的 prop 落到哪个 attribute 由组件实现定，写探针前 grep 组件模板确认。
