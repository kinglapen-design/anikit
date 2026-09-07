## 035 pointer capture 挂太早：begin 即 capture 会把 dblclick 重定向，名称区双击改名被吞成"打开"

- **现象：** 2026-08-18 用户实机：双击素材卡 / 文件夹卡的**名称区域**不进出改名，反而触发卡片级双击行为（素材开预览 / 文件夹进入）。
- **原因：** 032 的僵尸会话修复把 `setPointerCapture` 挂在拖拽引擎 `begin()`（每次左键按下即挂到卡片根元素）。capture 生效期间**全部指针事件（含兼容鼠标事件 click / dblclick）重定向到 capture 元素**——双击序列第二击与 dblclick 落到卡片根：名称 span 的 `@dblclick.stop` 永远收不到，卡片根的 dblclick（预览 / 进入）照触发。032 注释"capture 到根元素不影响 click 派发"对 click 成立（选中本就绑在卡片根），对 dblclick 子元素分发不成立。
- **解法：** capture 后移到 `startDrag`（位移超阈值、拖拽真正启动时才挂）——拖拽全程 capture 仍有效（窗外松手 pointerup 照收，032 三道防线的治本项保留）；纯点击 / 双击阶段不 capture，dblclick 正常打到名称子元素。begin 只记候选根元素（`sess.capEl`）+ `captured` 标记，endDrag 按标记释放；`onDocLeave` 改恒挂 + 仅在未 captured 时取消（cover 起拖前划出窗口的僵尸窗口期）。另两处同源：`HoverAudioLayer.onSeekDown` 的 `e.preventDefault()` 会抑制兼容鼠标事件使 click 不派发（音频卡缩略图点不选中），删除即可（seek 用 pointer capture 照旧）。
- **预防：** pointer capture 的挂载时机 = 手势确认的那一刻，不是按下那一刻；按下即 capture 会吞掉一切"按下发源于子元素"的高层事件（dblclick、contextmenu 同理）。验证拖拽引擎改动必跑 cdp-drag-zombie-probe（S5 真拖拽）+ 双击改名探针用例（cdp-card-interact-probe S4/S6）。
