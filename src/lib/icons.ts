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

// 双色调结构图标（folder-closed / archive / image / warning-small 等，文件保留 Figma 原值：
// 亮层 #BFBFBF / 暗层 #666666）——此处换 CSS 变量通道（fill 的 presentation attribute 不吃 var()，
// 必须内联 style），组件经 --tone-1 / --tone-2 注入：默认 --n-custom-1 / --n-custom-2，
// 文件夹自定义九色对（--n-custom-{色}-1/-2）在组件侧覆盖。2026-08-28 用户定案。
export function getTwoToneIcon(name: string): string {
  return getIcon(name)
    .replaceAll('fill="#BFBFBF"', 'style="fill:var(--tone-1)"')
    .replaceAll('fill="#666666"', 'style="fill:var(--tone-2)"');
}
