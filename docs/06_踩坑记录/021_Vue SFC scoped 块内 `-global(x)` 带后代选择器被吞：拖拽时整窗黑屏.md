## 021 Vue SFC scoped 块内 `:global(x)` 带后代选择器被吞：拖拽时整窗黑屏

- **现象：** 拖拽卡片一开始整个窗口全黑（只剩窗口边框），松手立即恢复——像渲染崩溃，实为 CSS 事故。
- **原因：** AssetCard scoped 样式块内写 `:global(body.dragging) .thumb .actions { opacity: 0 }`，**SFC scoped 编译把 `:global()` 后面的后代选择器吞掉**，产物只剩 `body.dragging{opacity:0;pointer-events:none}`（dist CSS 实证）。拖拽开始 body 加 dragging 类 → 整个 body 透明度归零、只剩窗口底色 = 全黑，松手移除类即恢复。
- **解法：** 全局规则改写到独立的非 scoped `<style>` 块（`body.dragging .asset-card .thumb .actions`）——类名在 scoped 编译产物中原样保留可正常匹配；特异性算足（(0,4,1) 压过 `.asset-card:hover .actions` (0,4,0)），构建后 grep dist CSS 验证产物。
- **预防：** scoped 块内 `:global(...)` 必须整包完整选择器、不能带后代；要影响全局（body 类切换等）一律写非 scoped 块或全局样式表；写这类"看不见"的规则（opacity / visibility / display）后 grep 一次 dist CSS 确认编译产物没变形。
