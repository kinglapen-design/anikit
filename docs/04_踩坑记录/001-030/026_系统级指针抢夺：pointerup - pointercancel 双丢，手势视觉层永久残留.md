## 026 系统级指针抢夺：pointerup / pointercancel 双丢，手势视觉层永久残留

- **现象：** 框选进行中按系统截图（或第三方截图工具）、以及框选拖到屏幕边缘触发 Win11 snap 分屏预览后，框选矩形永远盖在界面上不消失；UI 不卡死、可继续操作，但矩形一直在。
- **原因：** 框选手势的清除完全依赖 viewport 的 `pointerup` / `pointercancel`（setPointerCapture 定向送达）；系统截图工具抢焦点、snap 预览/窗口 System Move 抢指针时，这两个事件**双双不送达**——手势状态机永远停在"拖拽中"，marquee 矩形无人清除。
- **解法：** 两道兜底强制取消（`useMarquee.forceCancel`）：① viewport 监听 **lostpointercapture**——capture 被系统剥夺时的对症事件（不伴随 up/cancel 的场景全靠它）；② `window blur`——截图工具抢焦点场景。拖拽引擎（dragOperation）同根：begin 时挂 `window blur → endDrag(true)` 取消兜底，防 ghost / 落点指示同类残留。
- **预防：** 任何"按下-移动-松开"三段式手势（框选 / 拖拽 / scrub），结束条件不能只挂 pointerup/pointercancel——系统层随时可能抢指针（截图、snap、系统级窗口移动、远程桌面），**lostpointercapture + window blur 双兜底是标配**；WebView2/Chromium 对 capture 丢失不发 cancel 没有跨平台保证。
