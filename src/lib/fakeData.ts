// 阶段 1 假数据：固定 seed 伪随机，每次启动数据一致，便于验收对照。
// 阶段 3.1 起内容区接真实数据（stores/content.ts → list_placements / list_folder_tree）；
// 本文件保留作 10 万级性能压测（阶段 6 打磨时手动接回，勿删）。

export interface FakeAsset {
  id: number;
  name: string;
  w: number;
  h: number;
  color: string;
  format: string;
}

export interface FakeFolder {
  id: number;
  name: string;
  count: number;
  children: FakeFolder[];
}

// mulberry32：小而够用的确定性伪随机
function mulberry32(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a |= 0;
    a = (a + 0x6d2b79f5) | 0;
    let t = Math.imul(a ^ (a >>> 15), 1 | a);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

const SUBJECTS = ["场景", "角色", "分镜", "概念稿", "道具", "背景", "特效帧", "立绘", "表情", "动作"];
const FORMATS = ["psd", "png", "jpg", "fla", "webp", "mp4"];

export function genAssets(count: number): FakeAsset[] {
  const rand = mulberry32(20260719);
  const out = new Array<FakeAsset>(count);
  for (let i = 0; i < count; i++) {
    // 动画素材常见比例分布：横图 / 竖图 / 方图
    const kind = rand();
    let w: number;
    let h: number;
    if (kind < 0.45) {
      w = 1280 + Math.floor(rand() * 640);
      h = 720 + Math.floor(rand() * 360);
    } else if (kind < 0.75) {
      w = 720 + Math.floor(rand() * 360);
      h = 1280 + Math.floor(rand() * 640);
    } else {
      w = 600 + Math.floor(rand() * 500);
      h = w + Math.floor(rand() * 100) - 50;
    }
    const format = FORMATS[Math.floor(rand() * FORMATS.length)];
    const subject = SUBJECTS[Math.floor(rand() * SUBJECTS.length)];
    out[i] = {
      id: i,
      name: `${subject}${String((i % 97) + 1).padStart(2, "0")}.${format}`,
      w,
      h,
      // 深色低饱和色块，贴近缩略图观感
      color: `hsl(${Math.floor(rand() * 360)} ${18 + Math.floor(rand() * 22)}% ${16 + Math.floor(rand() * 20)}%)`,
      format,
    };
  }
  return out;
}

export function genFolderTree(): FakeFolder[] {
  let fid = 0;
  const node = (name: string, count: number, children: FakeFolder[] = []): FakeFolder => ({
    id: fid++,
    name,
    count,
    children,
  });
  // 形状对照参考图：6 个一级节点，部分带子层
  return [
    node("新建文件夹", 1235),
    node("展开文件夹", 1235, [
      node("新建文件夹", 132, [node("新建文件夹", 12), node("新建文件夹", 45)]),
      node("新建文件夹", 321),
      node("新建文件夹", 87),
    ]),
    node("新建文件夹", 1235),
    node("新建文件夹", 1235, [node("新建文件夹", 56), node("新建文件夹", 78)]),
    node("新建文件夹", 1235),
    node("新建文件夹", 1235),
  ];
}
