<script setup lang="ts">
// 组件 按钮展区——docs/06_UI重构/Design Token/组件/按钮/ 六族 17 稿的全态展示与验收面。
// forceState 强制 hover 用于静态验收；分段控制器 / 拉下按钮为真实交互实例（v-model）。
// 动画时长：全族 hover / 颜色过渡 150ms；拉下按钮展开收起 250ms；分段控制器滑块 300ms。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import TagButton from "../../components/buttons/TagButton.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import FontInstallButton from "../../components/buttons/FontInstallButton.vue";
import DropButton from "../../components/buttons/DropButton.vue";
import Checkbox from "../../components/buttons/Checkbox.vue";
import Radio from "../../components/buttons/Radio.vue";
import SwitchButton from "../../components/buttons/SwitchButton.vue";

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
const segSm = ref("auto");
const segSmOpts = [
  { value: "auto", label: "自适应" },
  { value: "grid", label: "瀑布流" },
  { value: "list", label: "列表" },
];
const unit = ref("KB");
// 拉下按钮间距矩阵静态验收（2026-08-29 走查：选中 MB/GB 时缺缝挤死，改缝恒由后项提供）
const unitMb = ref("MB");
const unitGb = ref("GB");
// 标签按钮-小-立体 贴稿假数据（男主角 = 选中）
const tagRow = [
  { label: "男", on: false },
  { label: "16岁", on: false },
  { label: "183cm", on: false },
  { label: "男主角", on: true },
  { label: "无限穿越", on: false },
];
const filterRow = ["颜色", "时长", "标签", "扩展名", "文件夹", "比例", "链接", "大小", "添加日期", "修改日期"];
// 复选框 / 单选框 / 开关按钮（2026-09-01 三稿）真实交互实例
const cbOn = ref(true);
const cbOff = ref(false);
const radioPick = ref("static");
const swOn = ref(true);
const swOff = ref(false);
</script>

<template>
  <section id="design-system-buttons" class="pv-section">
    <h2 class="pv-sec-title">组件 · 按钮</h2>
    <p class="pv-sec-desc">
      Design Token/组件/按钮/ 六族 17 稿；force-hover 列 = 强制态静态验收，分段控制器与拉下按钮可直接点击交互。<br />
      动画时长：全族 hover / 颜色过渡 <b>150ms</b>（通用）；拉下按钮展开 / 收起与箭头旋转 <b>250ms</b>（小位移）；分段控制器滑块 <b>300ms</b>（形态切换）。
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
        <div class="pv-demo-cell">
          <IconButton name="chevron-large" variant="circle" class="pv-qn-up" />
          <span class="pv-cap">纯圆 · 常态（快速到顶）</span>
        </div>
        <div class="pv-demo-cell">
          <IconButton name="chevron-large" variant="circle" class="pv-qn-down" force-state="hover" />
          <span class="pv-cap">hover（快速到底）</span>
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
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <TagButton label="雪牧" variant="solid" size="sm" removable />
          <span class="pv-cap">removable · 常态（hover 删除钮右至左滑入）</span>
        </div>
        <div class="pv-demo-cell">
          <TagButton label="雪牧" variant="solid" size="sm" removable force-state="hover" />
          <span class="pv-cap">removable · hover（删除钮 12 + 左距 6，150ms）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">分段控制器（图标 / 文字 / 文字小档 28 高，选中滑动 300ms；禁用 = 30% 透明度）</h3>
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
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 268px">
          <SegmentedControl v-model="segSm" :options="segSmOpts" variant="text" tone="g2" size="sm" />
          <span class="pv-cap">文字小档 sm · 查看方式稿（28 高 / padding 2 / g2 底，可点击）</span>
        </div>
        <div class="pv-demo-cell" style="width: 134px">
          <SegmentedControl :model-value="'asc'" :options="segSmOpts" variant="text" tone="g2" size="sm" disabled />
          <span class="pv-cap">禁用（30% 透明度，不响应）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">字体安装按钮 / 拉下按钮-横向（展开收起 250ms 小位移）</h3>
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
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <DropButton v-model="unitMb" :options="['KB', 'MB', 'GB']" initial-expanded />
          <span class="pv-cap">间距验收 · 展开选中 MB</span>
        </div>
        <div class="pv-demo-cell">
          <DropButton v-model="unitGb" :options="['KB', 'MB', 'GB']" initial-expanded />
          <span class="pv-cap">间距验收 · 展开选中 GB</span>
        </div>
        <div class="pv-demo-cell">
          <DropButton v-model="unitMb" :options="['KB', 'MB', 'GB']" />
          <span class="pv-cap">间距验收 · 收起选中 MB</span>
        </div>
        <div class="pv-demo-cell">
          <DropButton v-model="unitGb" :options="['KB', 'MB', 'GB']" />
          <span class="pv-cap">间距验收 · 收起选中 GB</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">复选框 / 单选框 / 开关按钮（2026-09-01 三稿）</h3>
      <p class="pv-group-note">
        对勾与单选圆点：弹出 250ms 过冲、收回 150ms 快收；开关横移 250ms 小位移；底 / 描边过渡 150ms。
        选中态 hover 与禁用态稿未给（开关 disabled 仅不响应点击），待补稿。
      </p>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <Checkbox v-model="cbOn" label="递归显示" />
          <span class="pv-cap">复选框 · 选中（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <Checkbox v-model="cbOff" label="递归显示" />
          <span class="pv-cap">复选框 · 未选中（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <Checkbox :model-value="false" label="递归显示" force-state="hover" />
          <span class="pv-cap">hover 未选中（描边 通用/六级）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <Radio v-model="radioPick" value="static" label="静态图" />
          <span class="pv-cap">单选框 · 选中（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <Radio v-model="radioPick" value="anim" label="动态图" />
          <span class="pv-cap">单选框 · 未选中（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <Radio model-value="x" value="y" label="动态图" force-state="hover" />
          <span class="pv-cap">hover 未选中（描边 通用/六级）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <SwitchButton v-model="swOn" />
          <span class="pv-cap">开关 · 开（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <SwitchButton v-model="swOff" />
          <span class="pv-cap">开关 · 关（可点击）</span>
        </div>
        <div class="pv-demo-cell">
          <SwitchButton :model-value="false" disabled />
          <span class="pv-cap">禁用（稿未给样式，仅不响应）</span>
        </div>
      </div>
    </div>
  </section>
</template>
