// 预览视口形态描述（previewSpecs 注册表类型，2026-09-21 R5 扩展点注册表化）——扩展性口径：
// 画面区「组件型分支」（font / text / audio 音乐预览 / other 不支持预览立姿）与宿主 PreviewPanel
// 解耦；每种格式一份 spec（kind + 渲染组件），宿主按 preview.kind 查表分发。host 基建分支
// （img/video/sequence/anim——缩放 / 元素挂载 / 帧流画布）不抽，仍归宿主模板先行。
// 未来新组件型格式（3D / 第三方插件）= 加一个 spec 文件 + registerPreviewPaneSpec 一行，宿主零改动。
// 分支判定优先级：kind 查表命中即挂载组件；原模板链「audio 靠 waveUrl 非 kind」这类数据条件
// 由组件内部 v-if 承接——数据未就绪渲染空，与链式 fallthrough 等价（各 kind 互斥，查表无顺序问题）。
// 数据口径：组件一律无 props——inject(PREVIEW_SESSION_KEY) 取本 panel 会话；
// 需视口实寸的分支经 PREVIEW_STAGE_KEY 注入 stage 元素。
import type { Component, InjectionKey, Ref } from "vue";

/** 宿主（PreviewPanel）注入的画面区 stage 元素（按视口实际像素取数的分支用） */
export const PREVIEW_STAGE_KEY: InjectionKey<Ref<HTMLElement | undefined>> = Symbol("preview-stage");

/** 组件型预览分支定义：kind → 渲染组件（无 props，自管数据与空态；根元素即画面区一层，定位自理） */
export interface PreviewPaneSpec {
  readonly kind: string; // preview.kind 查表键（未注册 kind = 画面区无组件型分支）
  readonly component: Component;
}
