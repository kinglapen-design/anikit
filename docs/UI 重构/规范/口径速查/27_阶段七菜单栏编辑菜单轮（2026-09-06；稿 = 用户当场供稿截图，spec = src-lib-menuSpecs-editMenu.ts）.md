# 27. 阶段七菜单栏编辑菜单轮（2026-09-06；稿 = 用户当场供稿截图，spec = src/lib/menuSpecs/editMenu.ts）


- **编辑菜单不收预览操作**（用户定案）：放大/缩小/旋转/翻转/上一文件·下一文件归「视图」钮（轮到时候按稿接）；播放/暂停/上·下帧归预览播放条（PRD 04 已定案，含 `,` `.` 逐帧快捷键）——编辑菜单 = 选中集 / 数据操作语义，不混查看播放类。
- **创建副本 / 克隆 同键 Ctrl+D = 按选中域分流显示**（用户定案）：素材域出「创建副本」（duplicateSelected）、文件夹·档案域出「克隆」（cloneFolderFlow）——同 useGlobalKeys Ctrl+D 分流；稿上两项并列同键为供稿冲突，落地分流；空选回素材域形态（创建副本 disabled）。
- **「从文件夹中删除链接」= 供稿笔误**（用户定案）：删除链接语义已不存在（03 稿链接分流项 2026-08-18 废除），编辑菜单与既有右键菜单一律「从文件夹中移除」（Shift+Delete，挪未分类不删记录）。
- **全项 = useGlobalKeys 既有快捷键的菜单面**（分流 / 禁用同口径）：文件夹域剪切 / 克隆 / 删除走 folders 流；复制无文件夹语义置灰；粘贴两项 = 内部剪贴板非空才可（空时 Ctrl+V 归系统剪贴板导入，不在本菜单语义；落点非法由 pasteClipboard toast 拦截）；重命名单选才可（F2 同径：文件夹看条带可见性分流卡片 / 树命名态）。**撤销 / 重做置灰读 undo store**（canUndo / canRedo，操作收尾 refreshState 已全量覆盖）。
- **回收站视口只读** = 编辑类全 disabled（appMenu 整理同口径；回收站的删除入口仍是右键「彻底删除」，Delete 键两段式不动）；**选择模式（picker.active）「编辑」钮禁开**（全编辑类，§12.1 白名单——同「菜单」钮闸，视觉不改）。
- SidebarLeft「编辑」钮 = 「菜单」钮同款机制（按钮下方弹出：左缘对齐 + 底 +4 / 同钮 toggle / trigger 豁免全级）；插件 / 视图两钮仍点亮不接业务。
- **探针**：cdp-app-menu-probe.py +E 段 22 项（合计 **87 项全 PASS**）——E1 空选态 12 项 6 组 + 禁用态 + 弹出位 + min-width 250 / E2 素材选中点亮 / E3 复制 → 粘贴点亮 → 副本粘贴 duplicate_placements 实证 / E4 文件夹域分流 + clone_folders 实证 / E5 撤销点亮链（桩 `__undo` 有状态 + 操作收尾 refreshState 拉取——**桩纪律：get_undo_state 须深拷贝返**）/ E6 同钮 toggle / E7 回收站只读；截图 app-menu-edit-empty / edit-asset。
