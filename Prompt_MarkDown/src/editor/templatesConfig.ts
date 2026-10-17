import { reactive } from "vue";

export interface ChainDef {
  name: string;
  rules: string[];
}

export type MenuMap = Record<string, string[]>;

export const DEFAULT_MENUS: MenuMap = {
  机位: ["正前方", "左前方", "右前方", "正后方", "左后方", "右后方", "左侧", "右侧"],
  仰俯: ["微仰拍", "仰拍", "大仰拍", "平视", "微俯拍", "俯拍", "大俯拍"],
  摄影机类型: ["云台稳定", "手持摄影晃动", "固定机位"],
  运镜: ["慢推镜头", "慢拉镜头", "急推镜头", "急拉镜头", "慢左移镜头", "慢右移镜头", "跟随", "固定镜头"],
};

export const DEFAULT_CHAINS: ChainDef[] = [
  { name: "镜头机位", rules: ["机位", "仰俯", "摄影机类型", "运镜"] },
];

export const templateConfig = reactive({
  menus: { ...DEFAULT_MENUS } as MenuMap,
  chains: [...DEFAULT_CHAINS] as ChainDef[],
});

export function parseMenuFile(text: string): string[] {
  return text
    .split("\n")
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith("#"));
}

export function applyTemplateData(menus: MenuMap, chains: ChainDef[]) {
  if (Object.keys(menus).length > 0) templateConfig.menus = menus;
  if (chains.length > 0) templateConfig.chains = chains;
}
