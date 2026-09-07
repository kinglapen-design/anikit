## 071 DWM 整窗玻璃延伸 + tao 保留 WS_CAPTION——最大化时 DWM 把原生窗口按钮画进框架条带（2026-09-08）

- **现象：** 无边框 + 云母主窗口（063 六轮终态配置），最大化 / 全屏后右上角冒出一组系统默认窗口按钮（最小化 / 还原 / 关闭），压在自定义 `.win-controls-overlay` 上，最大化 / 还原两图标重叠；常态下完全正常。
- **原因：** 三层叠加——① tao 建窗**无条件保留** `WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_MAXIMIZEBOX`（tao-0.35.3 `window_state.rs` to_window_styles），"无边框"靠 WM_NCCALCSIZE 返回 0 把非客户区归零；② 最大化时 tao 的 WM_NCCALCSIZE 分支把客户区调成工作区 rcWork（让出任务栏），窗口矩形与客户区顶缘之间露出 caption 高度的框架条带；③ 项目为填云母月牙缝对主窗口做了 `DwmExtendFrameIntoClientArea(-1)` 整窗玻璃延伸——带 caption 样式的窗口 + 整窗玻璃，DWM 便把原生标题栏按钮画进该条带。常态下客户区铺满整窗无条带，故只有最大化 / 全屏才露。
- **解法：** 最大化 / 全屏态把延伸 margins 改 0、还原回 -1（`window_fx::extend_frame`，启动按当前态 + `on_window_event` Resized 驱动切换）——最大化时 DWM 不裁角不画边（063），延伸填缝职责彼时无对象；云母由 `DWMWA_SYSTEMBACKDROP_TYPE` backdrop 属性提供、与延伸无关。**不走摘样式路线**：WS_CAPTION 这组样式是 DWM 阴影与 Win11 圆角的来源（007 终局），摘了双丢。
- **预防：** `DwmExtendFrameIntoClientArea(-1)` 整窗玻璃 ×「保留 caption 样式的无边框窗」组合有最大化副作用——给窗口加任何 DWM 级 hack 后要**三态目验**（常态 / 最大化 / 全屏）；DWM 层 headless 截不到（063），只能实机验。改 tao 行为先读其 `window_state.rs` / WM_NCCALCSIZE 源码确认样式与帧几何，别凭表象猜。
