// dockview 主题对象（1.2 多标签页）：继承 dark 底，className 挂 AniKit 覆盖层
// （src/styles/dockview-theme.css）；tabAnimation smooth = 浏览器式拖拽换位动画。
import { themeDark, type DockviewTheme } from "dockview-vue";

export const anikitDockTheme: DockviewTheme = {
  ...themeDark,
  name: "anikit",
  className: "anikit-dock-theme",
  gap: 0, // 分屏组间无留白（细分隔线由 sash 承担）
  tabAnimation: "smooth",
};
