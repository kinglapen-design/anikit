import { invoke } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import type { FileNode, TemplateFile } from "./types";

export interface Backend {
  pickFolder(): Promise<string | null>;
  openMdFile(): Promise<string | null>;
  saveMdAs(defaultName: string): Promise<string | null>;
  scanMarkdown(root: string): Promise<FileNode>;
  readMarkdown(path: string): Promise<string>;
  writeMarkdown(path: string, content: string): Promise<void>;
  createMarkdown(dir: string, name: string): Promise<string>;
  templateFiles(): Promise<TemplateFile[]>;
}

function isTauri(): boolean {
  return "__TAURI_INTERNALS__" in window;
}

const MD_FILTER = [{ name: "Markdown", extensions: ["md"] }];

const tauriBackend: Backend = {
  pickFolder: () => open({ directory: true, multiple: false }),
  openMdFile: () => open({ filters: MD_FILTER, multiple: false }),
  saveMdAs: (defaultName) => save({ defaultPath: defaultName, filters: MD_FILTER }),
  scanMarkdown: (root) => invoke<FileNode>("scan_markdown", { root }),
  readMarkdown: (path) => invoke<string>("read_markdown", { path }),
  writeMarkdown: (path, content) => invoke("write_markdown", { path, content }),
  createMarkdown: (dir, name) => invoke<string>("create_markdown", { dir, name }),
  templateFiles: () => invoke<TemplateFile[]>("template_files"),
};

const SAMPLE_P1 = `# P1_AI

> △破旧窗台上，手机屏幕疯狂闪烁，星辰激动的声音几乎穿透听筒。
> 星辰（电话音，尖叫失声）：雪牧！别睡了！快去查分！你看没看全省大榜？！

# P1_用户

\`\`\` text
资产清单
角色清单：雪牧=@、雪牧音频=@；星辰=@、星辰音频=@。
场景清单：雪牧卧室_正打=@；雪牧卧室_反打=@；星辰卧室=@。

画风约束
皮克斯式高品质风格化3D卡通动画；御兽奇幻动画，色彩明亮且柔和。

时间线：11秒/5个镜头，镜头切换快。

镜头 1：[0-1.5秒]
- 景别：近景。
- 机位与运镜：雪牧左前方，微仰拍，云台稳定，慢推镜头。
- 构图与景深：画面右侧近景是雪牧，面朝左侧；手机在雪牧手上。
- 表演和台词：雪牧双手拿着手机在打字，突然铃声响起，雪牧身体微弹。
- 音效：打字音效和来电铃声。
\`\`\`

用户待填写模板（非可提交成稿）

| 用途 | 实际文件与资产ID |
|---|---|
| 星辰卧室空间站位参考图 | SC01_星辰卧室顶视站位.png · AN2610132242-DGN57 |

---
`;

const SAMPLE_P2 = `# P2_雪牧分屏_AI

> 星辰（电话音，语无伦次）：何止是没跑！你赶紧登省教务网！

# P2_雪牧分屏_用户

## 资产清单

**角色清单：**雪牧。

## 镜头 1：[0-4秒]（接电话反应镜头）

- 景别：近景。
- 机位与运镜：雪牧左前方，微仰拍，云台稳定，慢推镜头。
- 表演和台词：雪牧保持着接电话的动作，将双肘支在桌面上，随后微微一愣。
`;

const MOCK_DOC_TEMPLATE = `# Pn_AI

> （剧本片段粘贴于此）

# Pn_用户

## 资产清单

角色清单：=@。

## 镜头 1：[0-X秒]

- 景别：。
- 机位与运镜：。
- 构图与景深：。
- 表演和台词：。
- 音效：。
`;

function makeMockBackend(): Backend {
  const files = new Map<string, string>([
    ["mock://示例-分镜/P1.md", SAMPLE_P1],
    ["mock://示例-分镜/P2.md", SAMPLE_P2],
    ["mock://templates/docs/分镜提示词.md", MOCK_DOC_TEMPLATE],
    ["mock://templates/menus/机位.md", "正前方\n左前方\n右前方\n左侧\n右侧\n"],
    ["mock://templates/menus/仰俯.md", "微仰拍\n仰拍\n大仰拍\n平视\n微俯拍\n俯拍\n大俯拍\n"],
    ["mock://templates/menus/摄影机类型.md", "云台稳定\n手持摄影晃动\n固定机位\n"],
    ["mock://templates/menus/运镜.md", "慢推镜头\n慢拉镜头\n急拉镜头\n跟随\n固定镜头\n"],
    [
      "mock://templates/chains.json",
      '{ "chains": [{ "name": "镜头机位", "rules": ["机位", "仰俯", "摄影机类型", "运镜"] }] }\n',
    ],
  ]);
  return {
    pickFolder: async () => "mock://示例-分镜",
    openMdFile: async () => "mock://示例-分镜/P2.md",
    saveMdAs: async (defaultName) => `mock://新文档/${defaultName}`,
    scanMarkdown: async () => {
      throw new Error("mock 不支持文件夹扫描");
    },
    readMarkdown: async (path) => {
      const hit = files.get(path);
      if (hit === undefined) throw new Error(`mock 文件不存在: ${path}`);
      return hit;
    },
    writeMarkdown: async (path, content) => {
      files.set(path, content);
    },
    createMarkdown: async (dir, name) => {
      const path = `${dir}/${name.replace(/\.md$/i, "")}.md`;
      files.set(path, "");
      return path;
    },
    templateFiles: async () => [
      { group: "docs", name: "分镜提示词", path: "mock://templates/docs/分镜提示词.md" },
      { group: "menus", name: "机位", path: "mock://templates/menus/机位.md" },
      { group: "menus", name: "仰俯", path: "mock://templates/menus/仰俯.md" },
      { group: "menus", name: "摄影机类型", path: "mock://templates/menus/摄影机类型.md" },
      { group: "menus", name: "运镜", path: "mock://templates/menus/运镜.md" },
      { group: "chains", name: "规则链配置", path: "mock://templates/chains.json" },
    ],
  };
}

export const backend: Backend = isTauri() ? tauriBackend : makeMockBackend();
export const isMock = !isTauri();
