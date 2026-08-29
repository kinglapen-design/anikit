// 图标统一从 assets/icons 内联载入——currentColor 与文件夹卡片 var() 均要求内联 SVG。
// 命名对照见 assets/icons/命名对照.md；新增图标丢进 assets/icons 即可按文件名引用。
// 旧版/ = MVP 旧图标归档（2026-08-28 阶段七前置）：终态 = 不注册不打包。
// 【过渡回退层，7.1 十七轮（2026-08-30）用户定】legacy 面板仍引用未迁移旧图标，
// 主窗口启动链路渲染即崩（命名对照.md 已载「预期过渡态」）——回退层让 getIcon 新库
// 查无 时落旧版同名 + console.warn 打标；面板轮逐件按稿接线，回退命中清零后本层删除退役。
const modules = import.meta.glob(
  ["../../assets/icons/**/*.svg", "!../../assets/icons/旧版/**"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
) as Record<string, string>;

// 旧版回退层（过渡期限定，见头部注释）
const legacyModules = import.meta.glob("../../assets/icons/旧版/**/*.svg", {
  query: "?raw",
  import: "default",
  eager: true,
}) as Record<string, string>;

const icons: Record<string, string> = {};
for (const [path, raw] of Object.entries(modules)) {
  const name = path.split("/").pop()!.replace(/\.svg$/, "");
  icons[name] = raw;
}

const legacyIcons: Record<string, string> = {};
for (const [path, raw] of Object.entries(legacyModules)) {
  const name = path.split("/").pop()!.replace(/\.svg$/, "");
  legacyIcons[name] = raw;
}

// 已打标清单：同一图标只 warn 一次，不刷屏
const fallbackWarned = new Set<string>();

export function getIcon(name: string): string {
  const svg = icons[name];
  if (svg) return svg;
  const legacy = legacyIcons[name];
  if (legacy) {
    if (!fallbackWarned.has(name)) {
      fallbackWarned.add(name);
      console.warn(`[icons] 「${name}」命中旧版回退层——面板轮按稿接线后删（见命名对照.md 过渡态条）`);
    }
    return legacy;
  }
  throw new Error(`图标不存在: ${name}（检查 assets/icons/命名对照.md）`);
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
