// 提示音（2026-08-10 用户供音频，assets/sound）：notify 成功 / 失败、alert 侵入弹窗。
// vite ?url 打包为前端资源（dev / build 同路径）；play() 被自动播放策略拦截时静默降级。
// 2026-09-05：音源换 mp3（wav 三件随用户替换删除）。
import successUrl from "../../assets/sound/成功.mp3?url";
import failUrl from "../../assets/sound/失败.mp3?url";
import alertUrl from "../../assets/sound/侵入弹窗.mp3?url";

function play(url: string): void {
  const a = new Audio(url);
  void a.play().catch(() => {});
}

/** 通知栏成功反馈音 */
export function playSuccess(): void {
  play(successUrl);
}

/** 通知栏失败反馈音 */
export function playFail(): void {
  play(failUrl);
}

/** alert 侵入式弹窗提示音（AppModal variant="alert"） */
export function playAlert(): void {
  play(alertUrl);
}
