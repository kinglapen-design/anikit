<script setup lang="ts">
// 组件 按钮展区——docs/UI 重构/Design Token/组件/按钮/ 六族 17 稿的全态展示与验收面。
// forceState 强制 hover 用于静态验收；分段控制器 / 拉下按钮为真实交互实例（v-model）。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import TagButton from "../../components/buttons/TagButton.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import FontInstallButton from "../../components/buttons/FontInstallButton.vue";
import DropButton from "../../components/buttons/DropButton.vue";

const segIcon = ref("library");
const segText = ref("preview");
const segIconOpts = [
  { value: "library", icon: "library" },
  { value: "workspace", icon: "workspace" },
  { value: "ai-chat", icon: "ai-chat" },
];
const segTextOpts = [
  { value: "preview", label: "预览" },
  { value: "size", label: "大小" },
  { value: "glyph", label: "字形" },
  { value: "other", label: "其他" },
];
const unit = ref("KB");
// 标签按钮-小-立体 贴稿假数据（男主角 = 选中）
const tagRow = [
  { label: "男", on: false },
  { label: "16岁", on: false },
  { label: "183cm", on: false },
  { label: "男主角", on: true },
  { label: "无限穿越", on: false },
];
const filterRow = ["颜色", "时长", "标签", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"];
</script>

<template>
  <section id="design-system-buttons" class="pv-section">
    <h2 class="pv-sec-title">组件 · 按钮</h2>
    <p class="pv-sec-desc">
      Design Token/组件/按钮/ 六族 17 稿；force-hover 列 = 强制态静态验收，分段控制器与拉下按钮可直接点击交互。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">操作按钮（主按钮 / 文字按钮-立体 / 文字按钮 / 图标+文字）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="确认" variant="primary" size="lg" />
          <span class="pv-cap">主按钮-大 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="确认" variant="primary" size="lg" force-state="hover" />
          <span class="pv-cap">hover（品牌/提亮）</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="确认" variant="primary" size="md" />
          <span class="pv-cap">主按钮-小 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="确认" variant="primary" size="md" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="取消" variant="solid" size="lg" />
          <span class="pv-cap">文字按钮-大-立体 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="取消" variant="solid" size="lg" force-state="hover" />
          <span class="pv-cap">hover（通用/六级）</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="取消" variant="solid" size="md" />
          <span class="pv-cap">文字按钮-小-立体 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <ActionButton label="取消" variant="solid" size="md" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <ActionButton label="菜单" variant="ghost" size="sm" />
          <span class="pv-cap">文字按钮 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <ActionButton label="菜单" variant="ghost" size="sm" force-state="hover" />
          <span class="pv-cap">hover（半透明-3 + 二级）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="信息查看管理" variant="solid" size="md" icon="set-prompt-file" />
          <span class="pv-cap">图标+文字-立体 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton
            label="信息查看管理"
            variant="solid"
            size="md"
            icon="set-prompt-file"
            force-state="hover"
          />
          <span class="pv-cap">hover</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">图标按钮（通用 / 大 / 小 / 立体 / 立体-半透明）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <IconButton name="collapse-sidebar" variant="plain" />
          <span class="pv-cap">通用 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="collapse-sidebar" variant="plain" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="set-cover" variant="bare" size="lg" />
          <span class="pv-cap">大 · 常态（裸 18）</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="set-cover" variant="bare" size="lg" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="add-small" variant="bare" size="sm" />
          <span class="pv-cap">小 · 常态（裸 14）</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="add-small" variant="bare" size="sm" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 116px">
          <IconButton name="set-prompt-file" variant="solid" />
          <span class="pv-cap">立体 · 常态</span>
        </div>
        <div class="pv-demo-cell" style="width: 116px">
          <IconButton name="set-prompt-file" variant="solid" force-state="hover" />
          <span class="pv-cap">hover（通用/六级）</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="copy" variant="float" />
          <span class="pv-cap">立体-半透明 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="copy" variant="float" force-state="hover" />
          <span class="pv-cap">hover（通用/四级）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">标签按钮（大-立体 / 小-立体 / 立体-半透明）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <TagButton label="清除" variant="solid" size="md" />
          <span class="pv-cap">大 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <TagButton label="清除" variant="solid" size="md" force-state="hover" />
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <TagButton label="无限穿越" variant="solid" size="sm" force-state="hover" />
          <span class="pv-cap">小 · hover</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div v-for="t in tagRow" :key="t.label" class="pv-demo-cell">
          <TagButton :label="t.label" variant="solid" size="sm" :selected="t.on" />
          <span class="pv-cap">{{ t.on ? "小 · 选中（品牌底）" : "小 · 常态" }}</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div v-for="(f, i) in filterRow" :key="f" class="pv-demo-cell">
          <TagButton :label="f" variant="ghost" size="sm" :selected="i === 3" />
          <span class="pv-cap">{{ i === 3 ? "半透明 · 选中" : "半透明 · 常态" }}</span>
        </div>
        <div class="pv-demo-cell">
          <TagButton label="颜色" variant="ghost" size="sm" force-state="hover" />
          <span class="pv-cap">hover（半透明-2）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">分段控制器（图标 / 文字，选中滑动 300ms）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 230px">
          <SegmentedControl v-model="segIcon" :options="segIconOpts" variant="icon" />
          <span class="pv-cap">图标 · 详情/链接切换（可点击）</span>
        </div>
        <div class="pv-demo-cell" style="width: 480px">
          <SegmentedControl v-model="segText" :options="segTextOpts" variant="text" />
          <span class="pv-cap">文字 · 字体预览（可点击）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">字体安装按钮 / 拉下按钮-横向</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <FontInstallButton />
          <span class="pv-cap">安装钮 · 常态</span>
        </div>
        <div class="pv-demo-cell">
          <FontInstallButton force-state="hover" />
          <span class="pv-cap">hover（描边 亮色/四级）</span>
        </div>
        <div class="pv-demo-cell">
          <FontInstallButton selected />
          <span class="pv-cap">选中（品牌底 + 半透明-3 边）</span>
        </div>
        <div class="pv-demo-cell">
          <DropButton v-model="unit" :options="['KB', 'MB', 'GB']" />
          <span class="pv-cap">拉下按钮 · 常态（箭头朝右，可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <DropButton v-model="unit" :options="['KB', 'MB', 'GB']" initial-expanded />
          <span class="pv-cap">展开态（点项选中并收起）</span>
        </div>
      </div>
    </div>
  </section>
</template>
