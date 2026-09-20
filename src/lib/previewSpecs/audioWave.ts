// 音频波形大图 spec（原 PreviewPanel waveUrl 分支）：PreviewAudioWave 自管高清重采样 +
// info 缩略图回退；分支判定「靠 waveUrl 非 kind」由组件内 v-if 承接
// （未就绪渲染空 = 原模板链 fallthrough 等价），stage 测量元素经 PREVIEW_STAGE_KEY 注入。
import PreviewAudioWave from "../../components/PreviewAudioWave.vue";
import type { PreviewPaneSpec } from "./types";

export const audioWavePaneSpec: PreviewPaneSpec = { kind: "audio", component: PreviewAudioWave };
