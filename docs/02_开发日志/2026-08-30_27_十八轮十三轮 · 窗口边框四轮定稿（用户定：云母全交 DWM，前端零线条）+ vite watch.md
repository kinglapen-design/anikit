# 十八轮十三轮 · 窗口边框四轮定稿（用户定：云母全交 DWM，前端零线条）+ vite watch 漏报坑（踩坑 064）
## 十八轮十三轮 · 窗口边框四轮定稿（用户定：云母全交 DWM，前端零线条）+ vite watch 漏报坑（踩坑 064）

- 四轮定稿（用户定，引 DWM 官方推荐「圆角交 DWM 管理」）：云母模式圆角 = DWM 裁（radius 0，已有）、常态边框 = DWM 染画 #333（已有）、**picking 品牌框 = DWM 切染独存，::after 云母不画**（本轮新改 `.window.fx-mica.picking::after { border: 0 }`）——同色双画被用户否掉（双弧光栅化路径差 ~1 物理 px，直边能遮角弧分叉带救不回）。实色模式不变（::before #333 radius 8 + picking ::after 1.5px 品牌自绘），跨平台兼容无涉（Win10 本就零 DWM 参与）。
- 过程测量存档：DWM 圆角裁切弧 = 10.3 物理 px @150%（≈7 逻辑 px，非 8——PIL 角部像素最小二乘拟合）；用户「圆角调小对齐」的方向判断正确，但四轮终案是 CSS 不画线（无需对齐）。
- 探针 cdp-window-shell-probe.py：W2d 改（mica picking ::after 不画）+ W7d 新立（solid picking ::after 自绘 1.5px 品牌 radius 8）。**初跑 W2d FAIL 牵出踩坑 064**（vite dev 对 agent 写盘不感知，探针/ curl 拿旧 CSS，touch 后重转），复跑全 19 项 PASS；vue-tsc 绿。

