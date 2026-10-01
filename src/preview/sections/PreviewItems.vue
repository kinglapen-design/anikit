<script setup lang="ts">
// 组件 item 展区——docs/06_UI重构/Design Token/组件/item/ 六族 15 稿的全态展示与验收面。
// forceState 强制态用于静态验收（hover / 选中 / 输入态），常态实例保持真实交互可 hover。
import { ref } from "vue";
import TreeRowItem from "../../components/items/TreeRowItem.vue";
import SectionTitle from "../../components/items/SectionTitle.vue";
import TabItem from "../../components/items/TabItem.vue";
import NoteItem from "../../components/items/NoteItem.vue";
import LinkFileItem from "../../components/items/LinkFileItem.vue";
import LinkThumbItem from "../../components/items/LinkThumbItem.vue";
import LinkAudioItem from "../../components/items/LinkAudioItem.vue";
import LinkRelationItem from "../../components/items/LinkRelationItem.vue";
import PollutionFileItem from "../../components/items/PollutionFileItem.vue";
import PickFileItem from "../../components/items/PickFileItem.vue";
import PickRelationItem from "../../components/items/PickRelationItem.vue";
import BaseDataItem from "../../components/items/BaseDataItem.vue";
import ColorSwatchItem from "../../components/items/ColorSwatchItem.vue";
import AppIcon from "../../components/AppIcon.vue";
import Checkbox from "../../components/buttons/Checkbox.vue";

const tints = ["", "red", "blue", "yellow", "yellowgreen", "green", "cyan", "purple", "pink", "peach"];

// SectionTitle tail 插槽演示（常驻「递归显示」复选框，7.1 内容容器轮）
const tailRecursive = ref(true);

// 基础数据 item 假数据（贴稿六行）
const baseDataRows = [
  { label: "尺寸", values: ["1920 x 1080"] },
  { label: "文件大小", values: ["123 MB"] },
  { label: "格式", values: ["PSD"] },
  { label: "添加日期", values: ["2026/07/12", "14:26"] },
  { label: "创建日期", values: ["2026/07/12", "14:26"] },
  { label: "修改日期", values: ["2026/07/12", "14:26"] },
];
// 色块 item 假数据（稿七色）
const swatchDemo = ["#943743", "#F5D0D0", "#984343", "#623C30", "#FCF7F4", "#6B4B6E", "#E19E73"];
</script>

<template>
  <section id="design-system-items" class="pv-section">
    <h2 class="pv-sec-title">组件 · item</h2>
    <p class="pv-sec-desc">
      Design Token/组件/item/ 六族 15 稿；forceState 列 = 强制态静态验收，常态列可直接 hover 交互。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">树容器 item（文件夹 / 快速访问 / 档案）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" />
          <span class="pv-cap">文件夹 · 常态（收起 >）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" force-state="hover" />
          <span class="pv-cap">hover（亮色/半透明-3）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" force-state="selected" />
          <span class="pv-cap">选中（半透明-2 + 立体感 + 提亮一级）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :direct-count="7" expanded />
          <span class="pv-cap">展开（∨ + 文件夹-打开；数量 = 直接子项）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" :expandable="false" />
          <span class="pv-cap">无子项（藏箭头保占位）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div v-for="t in tints" :key="t || 'default'" class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" :tint="t" />
          <span class="pv-cap">{{ t || "常态双色" }}</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" icon="dog" name="角色绑定" :count="8" />
          <span class="pv-cap">自定义图标 · 常态双色</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" icon="camera" name="分镜参考" :count="12" tint="purple" />
          <span class="pv-cap">自定义图标 + tint（改色联动，2026-09-08 修复）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" icon="music" name="音效" :count="45" tint="green" force-state="selected" />
          <span class="pv-cap">自定义图标 + tint + 选中</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" domain-selected />
          <span class="pv-cap">多选联动（半透明-2 底，无框）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" drop-zone="before" />
          <span class="pv-cap">拖拽插入线（2px 品牌常态）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" drop-zone="into" />
          <span class="pv-cap">并入（品牌半透明填充 + 常态描边）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="人物" :count="20" cut />
          <span class="pv-cap">剪切中（半透明 0.5）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="folder" name="二级文件夹" :count="3" :depth="1" />
          <span class="pv-cap">层级缩进（depth 1 = +20）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="library" name="动画素材" :count="3134" switchable />
          <span class="pv-cap">资源库行 · 库名 + 切换钮（2026-09-07 定）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="library" name="一个名字特别特别长的资源库" :count="3134" switchable force-state="selected" />
          <span class="pv-cap">长库名截断 · 选中（切换钮不挤压）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="uncategorized" name="未分类" :count="132" />
          <span class="pv-cap">未分类</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="recent" name="最近使用" :count="12" force-state="selected" />
          <span class="pv-cap">最近使用 · 选中</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="auto-import" name="自动导入" :count="3" />
          <span class="pv-cap">自动导入</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="thumbnail" name="缩略图" :count="8" />
          <span class="pv-cap">缩略图</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="quick" icon="trash" name="回收站" :count="5" />
          <span class="pv-cap">回收站</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="archive" name="人物" :count="20" />
          <span class="pv-cap">档案 · 常态（提示装饰圆点 + 档案图标）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="archive" name="人物" :count="20" force-state="selected" />
          <span class="pv-cap">选中</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="archive" name="人物" :count="20" tint="blue" />
          <span class="pv-cap">档案 · tint（档案图标与圆点双色跟随，圆点 = 颜色-2）</span>
        </div>
        <div class="pv-demo-cell" style="width: 230px">
          <TreeRowItem kind="archive" name="人物" :count="20" tint="blue" force-state="selected" />
          <span class="pv-cap">tint + 选中（圆点 = 颜色-1 亮档）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">标题 item（区块标题-树容器 / 详情链接、分组标题；7.1 扩：actions 显隐 + tail 常驻插槽 + staticActions 常显）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="文件夹" :count="123" />
          <span class="pv-cap">区块标题-树容器 · 常态（hover 出动作组）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="文件夹" :count="123" force-state="hover" />
          <span class="pv-cap">hover（更多 / 新建）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="链接" with-link force-state="hover" />
          <span class="pv-cap">详情链接 · hover（链接 / 更多 / 新建）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="链接" with-link :with-more="false" force-state="hover" />
          <span class="pv-cap">详情链接稿按钮组 = 链接 + 添加（withMore=false，7.1 右栏轮扩）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="文件" :count="46" :actions="false" force-state="hover" />
          <span class="pv-cap">actions=false（force-hover 也不渲染动作组——列表容器区块标题稿无动作钮）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="文件夹" :count="123" force-state="hover">
            <template #tail><Checkbox v-model="tailRecursive" label="递归显示" /></template>
          </SectionTitle>
          <span class="pv-cap">tail 常驻（递归显示复选框，点击自停冒泡）+ hover 动作组</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <SectionTitle variant="block" title="音频" :with-more="false" static-actions expanded />
          <span class="pv-cap">staticActions 常显（折叠钮+添加钮不随 hover——链接关系 tab 五区块稿口径，2026-08-31 用户定）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 400px">
          <SectionTitle variant="group" title="人物设定" :count="2" />
          <span class="pv-cap">分组标题 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 400px">
          <SectionTitle variant="group" title="人物设定" :count="2" force-state="hover" />
          <span class="pv-cap">hover（更多-大 / 添加-大）</span>
        </div>
        <div class="pv-demo-cell" style="width: 400px">
          <SectionTitle variant="group" title="人物设定" :count="2" hot-zone="lead" />
          <span class="pv-cap">hotZone=lead：折叠热区 + 抓手指针均收文字区（中间列表容器口径，2026-10-02 用户定，§63）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">标签页 item</h3>
      <div class="pv-demo-row">
        <div class="tab-demo">
          <div class="tab-bar">
            <TabItem label="新标签页" />
            <TabItem label="新标签页" force-state="hover" />
            <TabItem label="01_人物" selected />
            <TabItem label="已锁定" locked />
            <button class="tab-add"><AppIcon name="add-large" :size="18" /></button>
          </div>
          <div class="tab-panel" />
          <span class="pv-cap">
            栏底 = 通用三级、面板 = 通用一级（示意）；选中标签与面板无缝 + 肩膀反弧，未选中 hover = 四周 4px 的 6px 圆角罩；锁定 = pin 替代关闭钮（稿未画，沿用 1.2 现口径）。实际标签栏 = dockview 外壳 180 固定宽 + 平分收缩（2026-08-30 用户定），TabLabel 适配器接线
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">笔记 item</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 220px">
          <NoteItem time="0123:" text="这是第一篇笔记…" />
          <span class="pv-cap">常态（hover 滑入关闭钮）</span>
        </div>
        <div class="pv-demo-cell" style="width: 220px">
          <NoteItem time="0123:" text="这是第一篇笔记…" force-state="hover" />
          <span class="pv-cap">hover（半透明-3）</span>
        </div>
        <div class="pv-demo-cell" style="width: 220px">
          <NoteItem time="0123:" text="这是第一篇笔记…" selected force-state="selected" />
          <span class="pv-cap">选中（半透明-2 + 文本升二级）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">详情面板 item（链接文件 / 缩略图 / 音频 / 关系 / 污染文件）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <LinkFileItem block-name="文件夹" folder-name="01_人物" file-name="雪牧_自动.fla" />
          <span class="pv-cap">链接文件 · 常态（hover 滑入关闭钮）</span>
        </div>
        <div class="pv-demo-cell">
          <LinkFileItem block-name="文件夹" folder-name="01_人物" file-name="雪牧_自动.fla" force-state="hover" />
          <span class="pv-cap">hover（通用四级底 + 六级描边）</span>
        </div>
        <div class="pv-demo-cell">
          <LinkFileItem block-name="文件夹" folder-name="01_人物" file-name="雪牧_自动.fla" origin force-state="hover" />
          <span class="pv-cap">原始文件（品牌星徽章右下）</span>
        </div>
        <div class="pv-demo-cell">
          <LinkFileItem block-name="文件夹" folder-name="01_人物" file-name="雪牧_自动.fla" linked force-state="hover" />
          <span class="pv-cap">链接文件（链接徽章右下 亮色/三级，2026-09-23 补）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <LinkThumbItem name="雪牧_自动.webp" :count="2" />
          <span class="pv-cap">链接缩略图 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <LinkThumbItem name="雪牧_自动.webp" :count="2" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <LinkAudioItem name="雪牧-常态" />
          <span class="pv-cap">链接音频 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <LinkAudioItem name="雪牧-常态" playing force-state="hover" />
          <span class="pv-cap">播放中（暂停图标）+ hover</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <LinkRelationItem relation="父亲" target="雪牧_自动.fla" />
          <span class="pv-cap">链接关系 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <LinkRelationItem relation="父亲" target="雪牧_自动.fla" force-state="hover" />
          <span class="pv-cap">整卡 hover（四级底 + 关闭钮滑入）</span>
        </div>
        <div class="pv-demo-cell">
          <LinkRelationItem relation="父亲" target="雪牧_自动.fla" force-state="chip-hover" />
          <span class="pv-cap">关系标签 hover（半透明-2 + 输入指针）</span>
        </div>
        <div class="pv-demo-cell">
          <LinkRelationItem relation="父亲" target="雪牧_自动.fla" editing />
          <span class="pv-cap">输入态（去立体感 + 二级底 + 描边 + 展开宽度）</span>
        </div>
        <div class="pv-demo-cell">
          <PollutionFileItem name="恢复_雪牧.fla" />
          <span class="pv-cap">污染文件 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <PollutionFileItem name="恢复_雪牧.fla" force-state="hover" />
          <span class="pv-cap">hover（按钮组滑入）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">基础数据 item（详情面板 · 信息列表）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <BaseDataItem :rows="baseDataRows" />
          <span class="pv-cap">常态 · 纯展示无 hover（标签 四级 Bold / 值 数字档三级，宽度随父容器）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">色块 item（详情面板 · 主色）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <ColorSwatchItem :colors="swatchDemo" />
          <span class="pv-cap">常态（16 圆块 + 半透明-1 描边；hover 单块放大 1.25）</span>
        </div>
        <div class="pv-demo-cell">
          <ColorSwatchItem :colors="swatchDemo" force-state="hover" />
          <span class="pv-cap">force 全块放大示意（实际交互仅悬停块放大）</span>
        </div>
        <div class="pv-demo-cell">
          <ColorSwatchItem :colors="[]" :placeholder-count="7" />
          <span class="pv-cap">空态 7 占位（详情面板 pinned 恒显口径，7.1 右栏轮扩）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">选择模式 item（选择文件-通用 / 选择关系）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <PickFileItem name="雪牧_主动.fla" />
          <span class="pv-cap">选择文件 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <PickFileItem name="雪牧_主动.fla" force-state="hover" />
          <span class="pv-cap">hover（四级底 + 六级描边）</span>
        </div>
        <div class="pv-demo-cell">
          <PickFileItem name="雪牧_主动.fla" editing />
          <span class="pv-cap">输入态（二级底 + 半透明-1 描边）</span>
        </div>
        <div class="pv-demo-cell">
          <PickRelationItem picked="雪天_自动.fla" from-label="父亲" to-label="儿子" origin="雪天_自动.fla" />
          <span class="pv-cap">选择关系 · 常态（关系框可输入，聚焦描边 半透明-1）</span>
        </div>
        <div class="pv-demo-cell">
          <PickFileItem name="" editing placeholder="点击卡片选择" readonly :closable="false" />
          <span class="pv-cap">选择文件 · 空态占位行（只读 + 无关闭钮，7.1 右栏轮扩）</span>
        </div>
        <div class="pv-demo-cell">
          <PickRelationItem
            picked=""
            picked-placeholder="点击卡片选择"
            from-label=""
            to-label=""
            origin="雪天_自动.fla"
            disabled
            :closable="false"
          />
          <span class="pv-cap">选择关系 · 空态占位组（双框禁用 + 无关闭钮）</span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.tab-demo {
  display: flex;
  flex-direction: column;
  width: 720px;
}
.tab-bar {
  display: flex;
  align-items: flex-end;
  gap: var(--n-space-4);
  height: 38px;
  padding: 0 var(--n-space-24) 0 var(--n-space-36);
  background: var(--n-g3);
  border-radius: var(--n-radius-8) var(--n-radius-8) 0 0;
  overflow: hidden;
}
.tab-bar :deep(.tab) {
  flex: none;
  width: 180px;
}
.tab-add {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  margin-bottom: var(--n-space-4);
  flex: none;
  padding: 0;
  border-radius: var(--n-radius-6);
  color: var(--n-l3);
  transition:
    background var(--n-dur-100) var(--n-ease-in-out),
    color var(--n-dur-100) var(--n-ease-in-out);
}
.tab-add:hover {
  background: var(--n-l-a3);
  color: var(--n-l2);
}
.tab-panel {
  height: 48px;
  background: var(--n-g1);
  border-radius: 0 0 var(--n-radius-8) var(--n-radius-8);
}
.tab-demo .pv-cap {
  margin-top: var(--n-space-8);
}
</style>
