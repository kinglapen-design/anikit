// 字体卡 spec（稿 = 文件卡片-字体）：内容 = 示例字块居中（日/韩环境示例字业务侧替换）；
// 附加件 = 右上安装钮（复用 buttons/FontInstallButton；fc-install 定位类在宿主 FileCard scoped，
// 与 FormatBadge 的 fc-badge 同款"壳层统一定位"先例）。安装钮显隐走 installable 谓词
//（2026-09-26 ws 字体卡：install/uninstall 是 assetId 系统级命令，ws 合成项不出钮）。
import FontInstallButton from "../../components/buttons/FontInstallButton.vue";
import FontContent from "../../components/cards/contents/FontContent.vue";
import type { CardContentSpec } from "./types";

export const fontContentSpec: CardContentSpec = {
  content: FontContent,
  contentProps: (p) => ({ fontLines: p.fontLines, fontFamily: p.fontFamily }),
  overlays: [
    { comp: FontInstallButton, props: (p) => ({ class: "fc-install", selected: p.installed }), when: (p) => p.installable !== false },
  ],
};
