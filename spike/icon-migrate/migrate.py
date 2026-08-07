#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""阶段一 1.1 图标迁移（一次性）：new-icon / custom-icon / 档案-卡片 → kebab-case 入库。

规则（命名对照.md 颜色约定）：
- 文字四档（#F7FFFC/69.41% 常态、#EAFFF7/34.12% 标签、#FEFFFF/92.94% 高亮、#EEFFF8/23.53% 极淡）→ currentColor，删除对应 opacity（明暗由 CSS color 档位表达）。
- 结构色（品牌 #0A8079、状态 #CF4040/#278422、边框 #E6FBE8/14.12% 等）与软件品牌彩色（AE/PS/chrome…）保留原值。
"""
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]  # AniKit 项目根
ICONS = ROOT / "assets" / "icons"
DEFAULT = ICONS / "01_icon-default"

# new-icon → 01_icon-default/
NEW_ICON_MAP = {
    "AE - 未选中": "ae-off", "AE - 选中": "ae-on",
    "AI 对话框": "ai-chat", "AI 工具集": "ai-tools",
    "Animate - 未选中": "animate-off", "Animate - 选中": "animate-on",
    "chrome - 未选中": "chrome-off", "chrome - 选中": "chrome-on",
    "edge - 未选中": "edge-off", "edge - 选中": "edge-on",
    "photoshop - 未选中": "photoshop-off", "photoshop - 选中": "photoshop-on",
    "上下帧": "step-frame-vertical",
    "丢到回收站": "drop-to-trash",
    "从文件夹中移除": "remove-from-folder",
    "作为副本粘贴": "paste-as-copy",
    "作为连接粘贴": "paste-as-link",
    "作为连接粘贴并重命名": "paste-as-link-rename",
    "保存提示词": "save-prompt",
    "克隆": "clone",
    "全屏": "fullscreen",
    "关闭": "window-close",
    "分析提示词": "analyze-prompt",
    "分组管理": "group-manage", "分组菜单": "group-menu",
    "切换储存空间": "switch-space", "切换资源库": "switch-library",
    "创建副本": "duplicate",
    "删除": "clear",
    "剪切文件": "cut-file",
    "勾选": "check",
    "区块管理": "section-manage",
    "单选框 - 选中圆形": "radio-dot",
    "原图大小": "original-size",
    "原始文件 - 徽章": "original-file-badge",
    "发送至 Animate": "send-to-animate",
    "合并资源库": "merge-libraries",
    "后退 - 前进 - 搜索框下拉": "nav-history",
    "回收站": "trash",
    "在其他应用打卡": "open-in-other-app",
    "在资源管理器打开": "open-in-explorer",
    "复制": "copy", "复制数据": "copy-data", "复制文件": "copy-file",
    "复制档案": "copy-archive", "复制链接": "copy-link",
    "复选框 - 未选中": "checkbox-off", "复选框 - 选中": "checkbox-on",
    "失败": "fail",
    "导入本地文件": "import-file", "导入本地文件夹": "import-folder",
    "导入素材包": "import-pack",
    "导出到本地磁盘": "export-to-disk", "导出文件": "export-file",
    "展开 - 折叠": "tree-expand-arrow",
    "展开 - 收起同阶层文件夹": "expand-siblings",
    "展开 - 收起所有文件夹": "expand-all",
    "展开 - 收起文件夹": "expand-folder",
    "工作空间": "workspace",
    "帮助": "help",
    "循环": "loop",
    "快捷导出": "quick-export",
    "快进 - 快退": "seek",
    "成功": "success",
    "打开 MarkDown 编辑器": "open-markdown-editor",
    "打开序列帧包": "open-sequence-pack",
    "打开提示词编辑器": "open-prompt-editor",
    "打开文件链接所在位置": "reveal-file-link",
    "打开档案链接所在位置": "reveal-archive-link",
    "打开链接": "open-link",
    "打开预览": "open-preview",
    "扫描文件 - 档案链接": "scan-links",
    "扫描重复文件": "scan-duplicates",
    "指定 MarkDown 为提示词": "set-markdown-as-prompt",
    "排序依据": "sort-by",
    "搜索": "search",
    "播放": "play",
    "收起侧边栏": "collapse-sidebar", "收起详情面板": "collapse-detail",
    "文件夹 - 关闭": "folder-closed", "文件夹 - 打开": "folder-open",
    "文件链接管理": "file-link-manage",
    "新增": "add-large",
    "新建储存空间": "new-space", "新建子文件夹": "new-subfolder",
    "新建文件夹": "new-folder", "新建档案": "new-archive",
    "新建符号链接": "new-symlink",
    "新标签页打开": "open-in-new-tab", "新窗口打开": "open-new-window",
    "更改图标": "edit-icon", "更改封面": "edit-cover", "更改颜色": "edit-color",
    "最大化": "window-maximize", "最小化": "window-minimize",
    "最近使用": "recent", "未分类": "uncategorized",
    "查看方式": "view-options",
    "档案": "archive", "档案提示圆点": "archive-dot",
    "水平翻转": "flip-h",
    "汉堡菜单": "menu",
    "添加": "add-small",
    "添加文件链接": "add-file-link", "添加笔记": "add-note",
    "添加至其他资源库": "add-to-library",
    "清空回收站": "empty-trash",
    "清除缓存并重新加载": "clear-cache-reload",
    "火狐 - 未选中": "firefox-off", "火狐 - 选中": "firefox-on",
    "眼睛 - 睁开": "eye-open", "眼睛 - 闭上": "eye-closed",
    "移动抓手": "drag-handle", "移动文件": "move-file",
    "窗口置顶 - 取消": "window-pin-off", "窗口置顶 - 常态": "window-pin-on",
    "箭头 - 小": "chevron-small", "箭头": "chevron-large",
    "粘贴文件夹": "paste-folder",
    "红色删除": "delete-red",
    "编辑描述": "edit-description", "编辑数据区块": "edit-data-section",
    "缩略图": "thumbnail", "自动导入": "auto-import",
    "解散序列帧包导入": "import-sequence-unpack",
    "警告": "warning",
    "设置缩略图": "set-thumbnail",
    "资源库 - 双色 - 小": "library-duotone-small",
    "资源库 - 双色": "library-duotone", "资源库": "library",
    "还原全部项目": "restore-all",
    "适合窗口": "fit-window",
    "速率": "playback-rate",
    "重命名": "rename",
    "链接文件 - 徽章": "linked-file-badge", "链接文件管理": "linked-file-manage",
    "错误": "error-large",
    "音量": "volume",
    "音频 - item - 播放": "audio-play", "音频 - item - 暂停": "audio-pause",
    "音频 - item - 波形图": "audio-waveform",
    "顺时针旋转": "rotate-cw",
    "默认应用打开": "open-default",
}

# custom-icon → 05_custom-icon/
CUSTOM_ICON_MAP = {
    "动物": "animal", "场景": "scene", "家具": "furniture", "工具": "tool",
    "建筑": "building", "怪物": "monster", "数码": "digital", "文本": "text",
    "服饰": "clothing", "武器": "weapon", "游戏": "game", "表情": "emoji",
    "视频": "video", "角色": "character", "设施": "facility",
    "载具": "vehicle", "音频": "audio", "项目": "project", "食物": "food",
}

# 档案-卡片 → 04_archive_card/
ARCHIVE_CARD_MAP = {
    "回形针": "paperclip",
    "档案 - 背景纸张 - 01": "archive-paper-01",
    "档案 - 背景纸张 - 02": "archive-paper-02",
}

TEXT_TONES = ["#F7FFFC", "#EAFFF7", "#FEFFFF", "#EEFFF8"]
TONES_RE = "|".join(TEXT_TONES)


def convert_colors(content: str) -> str:
    """标签级处理：四档文字色 → currentColor，并删除对应 opacity（不动结构色 opacity）。"""
    def repl(m: re.Match) -> str:
        tag = m.group(0)
        new = tag
        if re.search(rf'fill="(?:{TONES_RE})"', new, re.I):
            new = re.sub(rf'fill="(?:{TONES_RE})"', 'fill="currentColor"', new, flags=re.I)
            new = re.sub(r'\s*fill-opacity="[0-9.]+"', '', new)
        if re.search(rf'stroke="(?:{TONES_RE})"', new, re.I):
            new = re.sub(rf'stroke="(?:{TONES_RE})"', 'stroke="currentColor"', new, flags=re.I)
            new = re.sub(r'\s*stroke-opacity="[0-9.]+"', '', new)
        return new
    return re.sub(r'<[^>]+>', repl, content)


def migrate(src_dir: Path, dst_dir: Path, mapping: dict) -> list[str]:
    errors = []
    dst_dir.mkdir(parents=True, exist_ok=True)
    src_files = {p.stem: p for p in src_dir.glob("*.svg")}
    missing = set(src_files) - set(mapping)
    if missing:
        errors.append(f"{src_dir.name} 存在未映射文件: {sorted(missing)}")
    for zh, en in mapping.items():
        src = src_files.get(zh)
        if not src:
            errors.append(f"映射无源文件: {src_dir.name}/{zh}.svg")
            continue
        dst = dst_dir / f"{en}.svg"
        if dst.exists():
            errors.append(f"目标已存在（跳过）: {dst.name}")
            continue
        dst.write_text(convert_colors(src.read_text(encoding="utf-8")), encoding="utf-8")
    return errors


def main() -> int:
    # 先删旧默认图标（git 历史可恢复；新图标沿用旧名者直接覆盖位）
    for old in DEFAULT.glob("*.svg"):
        old.unlink()

    errors = []
    errors += migrate(DEFAULT / "new-icon", DEFAULT, NEW_ICON_MAP)
    errors += migrate(DEFAULT / "custom-icon", ICONS / "05_custom-icon", CUSTOM_ICON_MAP)
    errors += migrate(DEFAULT / "档案-卡片", ICONS / "04_archive_card", ARCHIVE_CARD_MAP)
    if errors:
        print("\n".join(errors))
        return 1

    # 全部成功才删三个源目录
    for d in (DEFAULT / "new-icon", DEFAULT / "custom-icon", DEFAULT / "档案-卡片"):
        shutil.rmtree(d)

    # 验证：新文件不得残留四档文字色
    leftover = []
    for d in (DEFAULT, ICONS / "04_archive_card", ICONS / "05_custom-icon"):
        for p in d.glob("*.svg"):
            if re.search(rf'(?:{TONES_RE})', p.read_text(encoding="utf-8"), re.I):
                leftover.append(p.name)
    n = len(NEW_ICON_MAP) + len(CUSTOM_ICON_MAP) + len(ARCHIVE_CARD_MAP)
    print(f"迁移完成 {n} 个；四档色残留: {leftover or '无'}")
    return 0 if not leftover else 1


if __name__ == "__main__":
    sys.exit(main())
