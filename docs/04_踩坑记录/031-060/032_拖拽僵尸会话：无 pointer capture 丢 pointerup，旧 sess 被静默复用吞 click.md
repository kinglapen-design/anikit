## 032 拖拽僵尸会话：无 pointer capture 丢 pointerup，旧 sess 被静默复用吞 click

- **现象：** 2026-08-13 用户多次实机遇到"文件树 / 快速访问 item 点不动（卡死），非要双击才能选中；快速访问与目录树 item 同时带选中态"——间歇出现、莫名其妙，每次重启解决（内存态损坏特征）。
- **原因：** 拖拽引擎（lib/dragOperation）`begin()` 未做 pointer capture——item 上按下后鼠标划出窗口松手，`pointerup` 不派发到 webview，sess 变僵尸（`onWindowBlur` 兜底只在窗口**失焦**时触发，划出划回不失焦就漏）。僵尸 sess 被后续按下经 `if (sess) return` **静默复用**：移动超 6px 阈值误 `startDrag`——① `suppressClick` 吞掉紧随的 click（点击无效；原地双击位移小不触发，故"双击才行"）；② `startDrag` 内 `clickFolder` 把无关文件夹误入选择域，之后点当前快速访问项走 `openView` 等值 return 分支不清域（双选中残留）；③ 严重时执行意外 drop（静默移动文件）。
- **解法：** 三道防线互补——① **治本**：begin 时对按下的 `.tree-item/.asset-card/.list-row/.folder-card` 根元素 `setPointerCapture`（窗外松手 pointerup 也能收到；capture 到自身根元素不影响 click 派发——down/up 共同祖先仍是他；**切勿 capture 到 documentElement**——共同祖先变根会让 click 派发到根，纯点击全灭），endDrag 释放；② `onMove` 首行 `(e.buttons & 1) === 0` 立即 `endDrag(true)`——pointerup 丢失的僵尸在下一个 move 即刻自清；③ begin 里 sess 已存在先 `endDrag(true)` 再建新（不静默复用）。另修次要路径：FolderTree 的 `nextTick(clickFolder)` 加 `sidebarSelected === folder:id` 守卫（连点"树 item → 快速访问"同 tick 切走时不再误入域残留描边）。
- **预防：** 手写 pointer 拖拽引擎三件套必须齐：capture（或等价窗外 up 保证）/ buttons 活性检查 / 会话重建护栏；window 级 up 监听永远会丢（窗外松手、系统抢指针），不能只靠 blur 兜底。
