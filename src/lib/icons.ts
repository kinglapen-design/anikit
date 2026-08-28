// 图标统一从 assets/icons 内联载入——currentColor 与文件夹卡片 var() 均要求内联 SVG。
// 命名对照见 assets/icons/命名对照.md；新增图标丢进 assets/icons 即可按文件名引用。
// 旧版/ = MVP 旧图标归档（2026-08-28 阶段七前置），负向模式排除：不注册、不打包。
const modules = import.meta.glob(
  ["../../assets/icons/**/*.svg", "!../../assets/icons/旧版/**"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
) as Record<string, string>;

const icons: Record<string, string> = {};
for (const [path, raw] of Object.entries(modules)) {
  const name = path.split("/").pop()!.replace(/\.svg$/, "");
  icons[name] = raw;
}

export function getIcon(name: string): string {
  const svg = icons[name];
  if (!svg) throw new Error(`图标不存在: ${name}（检查 assets/icons/命名对照.md）`);
  return svg;
}
