<script setup lang="ts">
// 提示词 tab（7.1 二十三轮贴稿，稿 = 右侧边栏-详情面板-提示词）：
// 提示词-区块（padding 12 / gap 12 / border-top 通用/三级）：
//   编辑框（240×232 / padding 12 / 通用二级底 + 通用三级 1px / 圆角 8）——内文 次要文字 12/16 四级，
//     悬浮复制钮（右上内缩 12，30×30 通用/半透明-2 + 立体感-通用 / 圆角 6 / copy 图标 18 三级）；
//   按钮组两行（gap 8）：行一三钮均分（保存提示词 save-prompt / 打开编辑器 editor / 删除提示词 trash）、
//     行二两钮均分（分析提示词 analyze-prompt / 指定提示词文件 set-prompt-file）——纯图标钮 =
//     buttons/IconButton solid（高 34 填充式 / 图标 18 恒二级 / hover 通用六级）。
// **业务全部不接**（提示词功能未立项，2026-08-31 用户定：UI 重构只做 UI——编辑框只读占位、
// 按钮点亮不接业务；稿 prose「提示词按钮根据上下文需求显示」的上下文条件随业务轮再接）。
// 显隐 = 信息查看管理面板眼睛（2026-09-26）：hidden 即整 tab 空白（单区块，允许全隐藏）。
import { computed, ref } from "vue";
import IconButton from "../buttons/IconButton.vue";
import AppIcon from "../AppIcon.vue";
import { useUiStore } from "../../stores/ui";

const ui = useUiStore();
const promptText = ref(""); // 业务未接：恒空（placeholder 占位）
// 区块显隐（2026-09-26 信息查看管理面板）：单区块 hidden 即整 tab 空白（允许全隐藏，无空态文案）
const blockHidden = computed(() => ui.detailBlocks.prompt.hidden.includes("prompt"));
</script>

<template>
  <section v-if="!blockHidden" class="prompt-block">
    <!-- 编辑框（稿：isolate 供复制钮绝对定位） -->
    <div class="editor">
      <textarea
        v-model="promptText"
        class="area"
        placeholder="提示词"
        readonly
        spellcheck="false"
      />
      <button class="copy-btn" v-tip="'复制提示词'">
        <AppIcon name="copy" :size="18" />
      </button>
    </div>
    <!-- 按钮组（稿：两行 gap 8，钮均分 flex 1）——点亮不接业务 -->
    <div class="btn-row">
      <IconButton name="save-prompt" variant="solid" title="保存提示词" class="grow" />
      <IconButton name="editor" variant="solid" title="打开编辑器" class="grow" />
      <IconButton name="trash" variant="solid" title="删除提示词" class="grow" />
    </div>
    <div class="btn-row">
      <IconButton name="analyze-prompt" variant="solid" title="分析提示词" class="grow" />
      <IconButton name="set-prompt-file" variant="solid" title="指定提示词文件" class="grow" />
    </div>
  </section>
</template>

<style scoped>
.prompt-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  border-top: 1px solid var(--n-g3);
}
/* 编辑框（稿：240×232 / padding 12 / 通用二级底 + 通用三级 1px / 圆角 8） */
.editor {
  position: relative;
  height: 232px;
  padding: var(--n-space-12);
  box-sizing: border-box;
  background: var(--n-g2);
  border: 1px solid var(--n-g3);
  border-radius: var(--n-radius-8);
}
/* 内文 = 次要文字 12/16 亮色/四级（稿占位语境；只读口径） */
.area {
  width: 100%;
  height: 100%;
  padding: 0;
  border: none;
  outline: none;
  resize: none;
  background: transparent;
  font-family: var(--n-font-family);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l4);
}
.area::placeholder {
  color: var(--n-l4);
}
/* 悬浮复制钮（稿：右上内缩 12 / 30×30 / 通用半透明-2 + 立体感-通用 / 圆角 6 / 图标 18 三级） */
.copy-btn {
  position: absolute;
  top: var(--n-space-12);
  right: var(--n-space-12);
  width: 30px;
  height: 30px;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  border: none;
  border-radius: var(--n-radius-6);
  background: var(--n-g-a2);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l3);
  transition: color var(--n-dur-150) var(--n-ease-in-out);
}
.copy-btn:hover {
  color: var(--n-l2);
}
/* 按钮组（稿：gap 8；钮 = IconButton solid 填充式，均分 grow 1——组件宽 100% 由父容器定，
   行内各钮提权 flex 1 均分） */
.btn-row {
  display: flex;
  gap: var(--n-space-8);
}
.btn-row .grow {
  flex: 1;
  min-width: 0;
}
</style>
