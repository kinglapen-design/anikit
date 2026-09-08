# 杂项 · 最大化动画闪 DWM 原生按钮根治（NCCALCSIZE/GETMINMAXINFO 接管，最大化云母保留）
## 杂项 · 最大化动画闪 DWM 原生按钮根治（NCCALCSIZE/GETMINMAXINFO 接管，最大化云母保留）

走查 17 一轮修法（2026-09-08_09：最大化态撤玻璃 + 守卫拦 `WM_WINDOWPOSCHANGING` 动画前预撤）用户实机复验再报：**最大化动画过程还是有 DWM 原生按钮**；对照组 Codex 的最大化完全无此问题、云母效果也都在——问题有解，一轮路线错了。

**根因升级**（071 一轮定性修正）：按钮的容身之地不是玻璃，是**最大化非客户区 caption 条带**——tao 的 `WM_NCCALCSIZE` 最大化特判把客户区设为 rcWork（tao 0.35.3 `event_loop.rs:2124`），与系统默认最大化矩形（rcWork 外扩 frame 厚度）之间让出条带；DWM 在「WS_MAXIMIZE + WS_CAPTION + 条带」上画按钮，过渡动画期按目标态条带绘制。动画前撤玻璃改的是 margins，够不着动画期的绘制依据（条带几何），故照闪。

**修法**（对齐 Codex/Electron 无边框路线，守卫 subclass 接管两条消息让条带不存在，`window_fx::main_window_guard` ②）：① `WM_NCCALCSIZE`（wParam≠0 且已最大化）→ 客户区 = 整窗矩形（`rgrc[0]` 保持输入的窗口矩形，返 0 不下放 tao；非最大化照旧下放 tao 的 shadow insets 分支，边框阴影体系不动；自动隐藏任务栏的边留 1px，照搬 tao `has_edge`/`SHAppBarMessage` 补偿）；② `WM_GETMINMAXINFO` 先下放让 tao 填 min/max 尺寸约束，再覆盖 `ptMaxPosition`/`ptMaxSize` = 所在显示器 rcWork——最大化矩形正好工作区，内容四边不外扩不被裁。配套删除：`FRAME_GLASS_OFF` 状态机 + WINDOWPOSCHANGING 预撤 + lib.rs `Resized` 的 extend_frame 切换，玻璃恒 -1——**最大化云母保留**（Codex 实机先例；DWM 并不因最大化关 backdrop，一轮「微软惯例关云母」的定性一并修正），前端 `.window.fx-mica.maximized` 实色兜底删除（不删会盖住云母）。

**验证**：`cargo test` 446 绿 / `pnpm typecheck` 0 错。**交实机复验清单**：① 最大化稳态无原生按钮；② 最大化/还原动画全程无按钮；③ 最大化云母保留（不再回实色）；④ 还原后云母/边框/阴影如常；⑤ 拖标题栏 snap 半屏不受影响；⑥ 自动隐藏任务栏（若开）最大化不挡触发。回退预案（实机仍见按钮才启用）：改假最大化（拦 `SC_MAXIMIZE` 改 `SetWindowPos` 工作区几何）。踩坑 071 已同步重写（标题改「按钮依附最大化 caption 条带而非玻璃，终局 NCCALCSIZE 归零条带」），口径速查 19 补最大化云母保留口径。
