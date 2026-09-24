// 音频预览 spec（2026-09-25 音乐预览稿落地，换代本注册表 R5 的 audioWave 波形大图）：
// 音乐预览 = 唱片-icon + 文件名居中立姿（PreviewAudio）；波形按稿 prose 只留播放条
// （FloatBarPlay #progress 的 WaveProgress，速查 §36），预览视口不再有高清重采样分支。
import PreviewAudio from "../../components/PreviewAudio.vue";
import type { PreviewPaneSpec } from "./types";

export const audioPaneSpec: PreviewPaneSpec = { kind: "audio", component: PreviewAudio };
