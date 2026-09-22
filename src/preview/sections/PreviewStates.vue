<script setup lang="ts">
// 组件 空态与预览形态展区——空文件夹稿 + 字体预览稿 + 此文件不支持预览稿的展示与验收面。
// EmptyFolderState 实机根层 pointer-events none 穿透（不挡空白右键 / 框选 / 拖入导入），仅按钮可点；
// 按钮接真实业务（menuOps.pickImportPaths 系统导入对话框）——预览页纯展示口径：演示舞台整层禁指针（2026-08-30 内容容器轮）。
// 字体预览 / 不支持预览 = 静态复刻（R4 2026-09-20 换代后口径；本体依赖预览数据，壳内嵌真实 SegmentedControl/FontInstallButton/ActionButton）。
import { ref } from "vue";
import EmptyFolderState from "../../components/EmptyFolderState.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import FontInstallButton from "../../components/buttons/FontInstallButton.vue";
import ActionButton from "../../components/buttons/ActionButton.vue";

const fontTab = ref("preview");
const fontTabs = [
  { value: "preview", label: "预览" },
  { value: "size", label: "大小" },
  { value: "glyph", label: "字形" },
  { value: "other", label: "其他" },
];
</script>

<template>
  <section id="design-system-states" class="pv-section">
    <h2 class="pv-sec-title">组件 · 空态与预览形态</h2>
    <p class="pv-sec-desc">
      布局/主窗口/中间面板/中间面板-内容容器-空文件夹 稿：空文件夹插画 200（opacity 0.6 + drop-shadow 0 4 20 40%）+
      标题「暂无文件」（标题档 二级）+ 提示「将文件拖拽至此处导入」（正文 三级）+ 导入文件 / 导入文件夹双钮
      （ActionButton solid md + 前置图标 18，各 137 稿值，gap 8）；块 padding-bottom 24。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">空文件夹空态（EmptyFolderState 铺满列表容器居中；按钮实机可点，演示舞台禁指针纯展示）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-ef-stage">
            <EmptyFolderState />
          </div>
          <span class="pv-cap">舞台 = 列表容器示意（720×420）；按钮接真实导入业务（系统对话框），此处不响应</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">字体预览（R4 换代：中间面板-内容容器-字体预览稿；静态复刻嵌真实分段/安装钮）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-font-shell">
            <div class="pv-font-range">
              <div class="pv-font-head">
                <div class="pv-font-name-row">
                  <span class="pv-font-name">得意黑</span>
                  <FontInstallButton />
                </div>
                <span class="pv-font-sub">Smiley Sans Oblique</span>
              </div>
              <SegmentedControl v-model="fontTab" :options="fontTabs" variant="text" />
              <div class="pv-font-page">
                <div class="pv-font-demo">
                  <p class="pv-font-demo-title">荷塘月色</p>
                  <p class="pv-font-demo-body">
                    曲曲折折的荷塘上面，弥望的是田田的叶子。叶子出水很高，像亭亭的舞女的裙。
                  </p>
                </div>
              </div>
            </div>
          </div>
          <span class="pv-cap">
            复刻本体 R4 口径：显示范围 padding 0 72 / 名称 36 超大标题 + 安装钮（FontInstallButton）/ 副标题 18 标题档 /
            分段通栏（真实可点）/ 预览页 2px g4 顶线 + H1 30/40 居中 + 正文 15/25；字号滑杆 12-72 悬浮底部（本体接，
            此从略）；大小/字形两段 09 待办未开
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">此文件不支持预览（R4 落地：中间面板-内容容器-此文件不支持预览稿；静态复刻嵌真实按钮）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-us-stage">
            <div class="pv-us-body">
              <span class="pv-us-icon">ZIP</span>
              <span class="pv-us-title">素材打包备份.zip</span>
              <span class="pv-us-hint">暂不支持此文件格式预览，请用其他应用打开</span>
              <div class="pv-us-actions">
                <ActionButton label="默认应用打开" variant="solid" size="md" icon="open-default" />
                <ActionButton label="其他应用打开" variant="solid" size="md" icon="open-in-other-app" />
              </div>
            </div>
          </div>
          <span class="pv-cap">
            稿：块 column gap 24 + padding-bottom 24 / 格式图标 200 / 标题 18/26 Bold 二级（含扩展名）/ 提示 13/18 三级 /
            双钮 150×34 solid md（图标 18 三级）；本体 ws 合成项隐藏按钮组；预览环境不接真实打开命令；
            样例格式 2026-09-22 由 docx 改 zip（docx 已支持预览，R6）
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 空态演示舞台：relative 供 EmptyFolderState 绝对铺满；整层禁指针（按钮接真实业务，预览纯展示） */
.pv-ef-stage {
  position: relative;
  width: 720px;
  height: 420px;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  pointer-events: none;
}

/* 字体预览复刻壳（口径与 PreviewFont.vue R4 同步） */
.pv-font-shell {
  width: 720px;
  padding: var(--n-space-12) var(--n-space-12) 0;
  box-sizing: border-box;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.pv-font-range {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: 0 var(--n-space-72);
}
.pv-font-head {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.pv-font-name-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
}
.pv-font-name {
  font-size: var(--n-fs-display);
  line-height: var(--n-lh-display);
  font-weight: var(--n-fw-bold);
  color: var(--n-md-title);
}
.pv-font-sub {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-md-other);
}
.pv-font-page {
  border-top: 2px solid var(--n-g4);
  padding: var(--n-space-12) 0;
}
.pv-font-demo-title {
  font-size: 30px;
  line-height: 1.333;
  font-weight: var(--n-fw-bold);
  color: var(--n-md-title);
  text-align: center;
}
.pv-font-demo-body {
  margin-top: var(--n-space-12);
  font-size: 15px;
  line-height: 1.667;
  color: var(--n-md-body);
}

/* 不支持预览复刻壳（口径与 PreviewPanel.vue R4 同步） */
.pv-us-stage {
  position: relative;
  width: 720px;
  height: 420px;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  display: flex;
  align-items: center;
  justify-content: center;
}
.pv-us-body {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-24);
}
.pv-us-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 200px;
  height: 200px;
  border-radius: var(--n-radius-16);
  background: var(--n-g3);
  color: var(--n-l4);
  font-size: var(--n-fs-display);
  font-weight: var(--n-fw-bold);
  letter-spacing: 2px;
}
.pv-us-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-us-hint {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pv-us-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--n-space-8);
  width: 308px;
}
</style>
