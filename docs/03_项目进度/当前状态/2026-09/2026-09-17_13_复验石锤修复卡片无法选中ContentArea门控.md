# 复验石锤修复 · 卡片无法选中 + 文件夹单卡回宽（2026-09-17，feat/new-ui）

用户实机报「严重问题，文件都不能选中了」——单变量定位 = 复验修复③的 ContentArea 整区 `@pointerdown="panelApi.setActive()"`：卡片 pointerdown 触发 dockview 激活链即时重渲染，in-flight click 目标元素被换 → click 丢失 → 选中全灭（回滚后立即恢复，实机截图 WAV 环 + 详情）。修复 = `onRootPointerdown` 真空白门控（卡片/树行/按钮/输入框/菜单/分组标题等交互件不激活），空白聚焦诉求保留、card click 链构造上完整。顺手：文件夹单卡 180→220（单卡太窄，多卡行仍 180 弹性）。

验证：typecheck 0 错 + 实机截图（撤版选中恢复）。待走查：选中恢复 / 空白聚焦 / 分屏互拖。
