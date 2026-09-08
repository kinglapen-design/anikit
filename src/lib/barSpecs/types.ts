// 播放条形态描述（barSpecs 注册表类型，2026-09-09 预览悬浮栏二轮）——扩展性口径：
// FloatBar 容器 / morph 动画 / FloatBarPlay 陈列件与业务解耦；每种预览格式一份 spec（show 组删减 +
// 时间码 / 进度 / 播放控制 / 可选帧条与波形），宿主 FloatBarHost 只查表分发。
// 未来新格式（3D / 第三方插件）= 加一个 spec 文件 + registerPlayBarSpec 一行，宿主与容器零改动；
// 控件组不同 = spec.show 数组声明（新按钮种类才动 FloatBarPlay 的 PlayGroup）。
// 全部为函数取值（宿主 computed 里调用，store 响应式追踪照常）；可选方法与 show 组对应
// （组不在 show 即不会触发，缺方法安全）。

export type PlayGroup =
  | "transport" // 快退 / 快进
  | "step" // 上一帧 / 下一帧
  | "volume" // 声音 / 静音
  | "rate" // 倍速
  | "loop" // 循环播放
  | "note" // 笔记
  | "cover" // 设为封面
  | "fullscreen"; // 全屏

/** 帧条数据接口（spec 可选提供；提供 = 帧条显示，分页与排版归宿主） */
export interface FrameStripSpec {
  count(): number; // 总帧数
  thumb(i: number): string | null; // 第 i 帧图 url（convertFileSrc 后；未就绪 / 越界 null = 空槽）
  frame(): number; // 当前帧索引（页跟随 = floor(frame/5) 归宿主）
  setFrame(i: number): void;
  stepFrame(d: number): void; // 两端尖括号钮 = 上/下一帧
}

export interface PlayBarSpec {
  readonly show: readonly PlayGroup[]; // 控件组删减（稿 prose「根据上下文，选择保留哪些按钮」）
  playing(): boolean;
  disabled(): boolean; // 转码态（无转码概念的格式恒 false）
  progressGet(): number; // 0~1
  progressSet(p: number): void;
  tcCurr(): string; // 时间码两段（父级格式化档，FloatBarPlay 纯陈列）
  tcTotal(): string;
  rate(): number;
  setRate(r: number): void;
  togglePlay(): void;
  seekBack?(): void; // transport 组（∓5s）
  seekForward?(): void;
  stepPrev?(): void; // step 组（逐帧，步进即暂停）
  stepNext?(): void;
  muted?(): boolean; // volume 组
  volumeGet?(): number; // 显示映射（静音显 0、实际音量保留）
  volumeSet?(v: number): void;
  toggleMute?(): void;
  loopOn?(): boolean; // loop 组
  toggleLoop?(): void;
  setCover?(): void; // cover 组（当前画面 / 帧设为预览图）
  fullscreen?(): boolean;
  toggleFullscreen?(): void;
  dragStart?(): void; // 进度拖动起止（scrubbing 挂钩；无此概念的格式缺省）
  dragEnd?(): void;
  frames?: FrameStripSpec; // 帧条（sequence / anim）
  waveFallback?(): string | null; // 音频波形回退图（提供 = 进度区换 WaveProgress）
}
