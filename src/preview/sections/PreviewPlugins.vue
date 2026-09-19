<script setup lang="ts">
// 插件面板展区——docs/06_UI重构/Design Token/布局/主窗口/工具栏展开窗口/插件.md（列表浮层 400×450）
// + Animate 插件.md（详情子窗口 544×538）（2026-09-06 插件窗轮）；MCP 详情窗复刻（544×640，
// 2026-09-20 MCP 条目轮：服务组开关行 SwitchButton + 教程配置行 copy 钮）。
// PluginListPanel 是 AppMenu fixed 浮层、AnimatePluginWindow/McpPluginWindow 挂载即调后端命令——
// 均不便静态展示，按 PreviewModals 的 pv-modal-shell 手法复刻口径静态陈列（口径变更时与组件同步）；
// 复刻壳内尽量嵌真实组件（SegmentedControl text tone=g2 / IconButton bare / ActionButton lg 三变体 / SwitchButton）。
import { computed, ref } from "vue";
import AppIcon from "../../components/AppIcon.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import SwitchButton from "../../components/buttons/SwitchButton.vue";
import { PLUGINS, PLUGIN_CATEGORIES } from "../../lib/plugins";

// 面板复刻的分类分段可交互（本地状态，纯前端过滤静态清单；空分类出空态「暂无插件」）
const cat = ref<string>(PLUGIN_CATEGORIES[0]);
const catOptions = PLUGIN_CATEGORIES.map((c) => ({ value: c, label: c }));
const shown = computed(() => PLUGINS.filter((p) => p.category === cat.value));

// MCP 详情窗复刻的开关行（纯展示，不接后端）
const mcpSvcOn = ref(true);
const mcpWriteOn = ref(false);
</script>

<template>
  <section id="design-system-plugins" class="pv-section">
    <h2 class="pv-sec-title">面板 · 插件（工具栏展开窗口）</h2>
    <p class="pv-sec-desc">
      布局/主窗口/工具栏展开窗口/插件.md + Animate 插件.md（2026-09-06）：列表浮层 400×450 = g3 底 + 1px g5
      真 border + 大阴影 + 圆角 8；分类分段 = SegmentedControl text 型 tone=g2（容器 通用/二级 40 高）；
      item 376×32 圆角 6（抓手 14 盒 + 图标 24 盒 + 名称 16/22 Bold 二级 + 更多钮 18 盒四级），hover 底
      亮色/半透明-3（稿未画，就近套菜单项口径，待走查）；空分类 = 「暂无插件」13/18 四级居中；
      底部开发者选项（border-top g5）点亮不接业务。详情子窗口 544×538 = padding 24 / 列 gap 24，
      按钮行右对齐（关闭 cancel 128 / 卸载 danger 128 / 安装 primary 200，均 lg 38）。
      壳为预览复刻；图标 = 命名对照十一批入库（grip / dev-options / animate-plugin / batch-thumbnails）。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">插件列表面板（浮层；分类分段可点切——空分类显空态）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-plugin-shell">
            <div class="pv-plugin-head">
              <SegmentedControl v-model="cat" :options="catOptions" variant="text" tone="g2" />
            </div>
            <div class="pv-plugin-list">
              <div
                v-for="p in shown"
                :key="p.id"
                class="pv-plugin-item"
                :class="{ disabled: p.disabled }"
              >
                <span class="lead">
                  <span class="grip"><AppIcon name="grip" :size="14" /></span>
                  <span class="p-icon"><AppIcon :name="p.icon" :size="22" /></span>
                  <span class="p-name">{{ p.name }}</span>
                </span>
                <IconButton name="more-large" variant="bare" class="more-btn" />
              </div>
              <div v-if="shown.length === 0" class="pv-plugin-empty">暂无插件</div>
            </div>
            <div class="pv-plugin-foot">
              <span class="dev-icon"><AppIcon name="dev-options" :size="14" /></span>
              <span class="dev-label">开发者选项</span>
            </div>
          </div>
          <span class="pv-cap">400×450；「批量生成缩略图」业务未立项 = 禁用占位（降四级不响应）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-item-demo">
            <div class="pv-plugin-item">
              <span class="lead">
                <span class="grip"><AppIcon name="grip" :size="14" /></span>
                <span class="p-icon"><AppIcon name="animate-plugin" :size="22" /></span>
                <span class="p-name">Animate 插件</span>
              </span>
              <IconButton name="more-large" variant="bare" class="more-btn" />
            </div>
            <span class="pv-cap">item 常态</span>
          </div>
          <div class="pv-item-demo">
            <div class="pv-plugin-item hover">
              <span class="lead">
                <span class="grip"><AppIcon name="grip" :size="14" /></span>
                <span class="p-icon"><AppIcon name="animate-plugin" :size="22" /></span>
                <span class="p-name">Animate 插件</span>
              </span>
              <IconButton name="more-large" variant="bare" class="more-btn" />
            </div>
            <span class="pv-cap">item hover（底 亮色/半透明-3，待走查）</span>
          </div>
          <div class="pv-item-demo">
            <div class="pv-plugin-item disabled">
              <span class="lead">
                <span class="grip"><AppIcon name="grip" :size="14" /></span>
                <span class="p-icon"><AppIcon name="batch-thumbnails" :size="22" /></span>
                <span class="p-name">批量生成缩略图</span>
              </span>
              <IconButton name="more-large" variant="bare" class="more-btn" />
            </div>
            <span class="pv-cap">item 禁用（名称/图标降四级；位图图标不吃 color，降档仅作用于名称——待走查）</span>
          </div>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">Animate 插件详情子窗口（544×538；图示 = 未安装态：卸载 disabled / 安装 CEP 插件点亮）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-aw-shell">
            <h1 class="pv-aw-title">Animate 插件</h1>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">使用教程</h2>
              <ul class="pv-aw-lines">
                <li>点击右下角的 <b>[安装]</b> 按钮，安装 Animate CEP 桥接插件。</li>
                <li>可以在 Animate <b>[窗口/扩展/AniKit]</b> 里打开 AniKit 面板。</li>
                <li>安装后，右键菜单会添加 <b>[发送至 Animate]</b> 的菜单项。</li>
              </ul>
            </section>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">功能列表</h2>
              <ul class="pv-aw-lines">
                <li>选中 Animate 的元件， CEP 里导出元件为新 fla 文档并附带缩略图。</li>
                <li>
                  在 CEP 面板可选择快速导出当前 fla 文档的缩略图，若 fla 文档本就在资源库中，则缩略图会自动导入至素材库，并链接至
                  fla 文档；若 fla 文档不在资源库中，则缩略图会保存在原 fla 文档的同一路径下。
                </li>
                <li>对资源库里的 fla 文档右键，则可看见 <b>[发送至 Animate]</b> 的菜单，一键将 fla 资产快速导入至 Animate 舞台。</li>
              </ul>
            </section>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">路径</h2>
              <ul class="pv-aw-lines">
                <li>CEP 安装路径: C:\Users\kingl\AppData\Roaming\Adobe\CEP\extensions</li>
                <li>Temp: 资源库\temp\cep-out</li>
              </ul>
            </section>
            <div class="pv-aw-spacer" />
            <div class="pv-aw-actions">
              <span class="pv-aw-btn" style="width: 128px">
                <ActionButton label="关闭" variant="cancel" size="lg" />
              </span>
              <span class="pv-aw-btn" style="width: 128px">
                <ActionButton label="卸载" variant="danger" size="lg" disabled />
              </span>
              <span class="pv-aw-btn" style="width: 200px">
                <ActionButton label="安装 CEP 插件" variant="primary" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">已安装态：卸载点亮、主钮文案「重新安装 / 升级」；按钮 cancel / danger 为本轮 ActionButton 新扩变体</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">MCP 详情子窗口（544×640，2026-09-20；四组 = 服务 / 使用教程 / 功能列表 / 路径，图示 = 服务开·只读态）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-aw-shell pv-aw-shell--mcp">
            <h1 class="pv-aw-title">AI 外接通道（MCP）</h1>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">服务</h2>
              <div class="pv-svc-lines">
                <div class="pv-svc-row">
                  <SwitchButton v-model="mcpSvcOn" />
                  <span class="pv-svc-label">MCP 服务</span>
                </div>
                <div class="pv-svc-row">
                  <SwitchButton v-model="mcpWriteOn" :disabled="!mcpSvcOn" />
                  <span class="pv-svc-label">允许写入</span>
                </div>
                <p class="pv-svc-status">运行中 · http://127.0.0.1:17701/mcp</p>
              </div>
            </section>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">使用教程</h2>
              <ul class="pv-aw-lines">
                <li>开启上方 <b>[MCP 服务]</b>（默认开），AniKit 会在本机拉起 MCP 服务供外部 Agent 接入。</li>
                <li>在你的 Agent 工具里接入 AniKit（三选一，点右侧复制钮拷贝配置）：</li>
                <li class="pv-cfg-row">
                  <span class="pv-cfg-text"><b>Kimi Code</b>：mcp.json 加 { "mcpServers": { "anikit": { "url": "http://127.0.0.1:17701/mcp" } } }</span>
                  <IconButton name="copy" variant="bare" size="sm" title="复制" />
                </li>
                <li class="pv-cfg-row">
                  <span class="pv-cfg-text"><b>Codex</b>：config.toml 加 [mcp_servers.anikit] 与 url = "http://127.0.0.1:17701/mcp"</span>
                  <IconButton name="copy" variant="bare" size="sm" title="复制" />
                </li>
                <li class="pv-cfg-row">
                  <span class="pv-cfg-text"><b>Claude Code</b>：运行 claude mcp add --transport http anikit http://127.0.0.1:17701/mcp</span>
                  <IconButton name="copy" variant="bare" size="sm" title="复制" />
                </li>
                <li>接入后 Agent 默认只能<b>只读</b>（搜索、浏览、看图等 8 个工具）；需要 Agent 写入时开启上方 <b>[允许写入]</b>。</li>
              </ul>
            </section>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">功能列表</h2>
              <ul class="pv-aw-lines">
                <li><b>只读（8 个）</b>：当前库状态 / 搜索素材 / 文件夹树 / 文件夹资产清单 / 资产详情 / 磁盘路径（看图入口）/ 读文本资产 / 档案列表。</li>
                <li><b>写入（9 个，需开「允许写入」）</b>：新建文档 / 覆写文本 / 批量改名 / 新建文件夹 / 移入文件夹 / 移动文件夹 / 链接汇总 / 导入入库 / 丢回收站。</li>
              </ul>
            </section>
            <section class="pv-aw-group">
              <h2 class="pv-aw-sub">路径</h2>
              <ul class="pv-aw-lines">
                <li>服务地址: http://127.0.0.1:17701/mcp</li>
                <li>端口文件: %APPDATA%\AniKit\mcp-server.port</li>
              </ul>
            </section>
            <div class="pv-aw-spacer" />
            <div class="pv-aw-actions">
              <span class="pv-aw-btn" style="width: 128px">
                <ActionButton label="关闭" variant="cancel" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            版式仿 Animate 窗；开关行 = SwitchButton 左 + 标签右 gap 8（退役 SettingsModal 口径沿入），服务关时「允许写入」禁用；
            配置行 = 深缩进 24 子行 + IconButton bare sm copy；无安装/卸载钮（MCP 无文件可装）
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* —— 插件列表面板复刻壳（与 plugins/PluginListPanel.vue 口径同步；AppMenu 浮层不便静态陈列） —— */
.pv-plugin-shell {
  display: flex;
  flex-direction: column;
  width: 400px;
  height: 450px;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
}
.pv-plugin-head {
  flex: none;
  height: 64px;
  box-sizing: border-box;
  padding: var(--n-space-12);
  border-bottom: 1px solid var(--n-g5);
}
.pv-plugin-list {
  flex: 1;
  min-height: 0;
  box-sizing: border-box;
  padding: var(--n-space-12);
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
.pv-plugin-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-6);
  height: 32px;
  padding: var(--n-space-4) var(--n-space-6);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  flex: none;
}
.pv-plugin-item.hover {
  background: var(--n-l-a3);
}
.pv-plugin-item.disabled {
  pointer-events: none;
}
.pv-plugin-item.disabled .p-name,
.pv-plugin-item.disabled .p-icon {
  color: var(--n-l4);
}
.pv-plugin-item .lead {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  flex: 1;
  min-width: 0;
}
.pv-plugin-item .grip {
  width: 14px;
  height: 14px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--n-l4);
}
.pv-plugin-item .p-icon {
  width: 24px;
  height: 24px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
}
.pv-plugin-item .p-name {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pv-plugin-item .ic-btn.more-btn {
  color: var(--n-l4);
}
.pv-plugin-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l4);
  user-select: none;
}
.pv-plugin-foot {
  flex: none;
  height: 42px;
  box-sizing: border-box;
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding: var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
}
.pv-plugin-foot .dev-icon {
  width: 18px;
  height: 18px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  flex: none;
  color: var(--n-l3);
}
.pv-plugin-foot .dev-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
/* item 状态陈列列（常态 / hover / 禁用）：复刻 item 宽 376 稿值 */
.pv-item-demo {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
  width: 376px;
  margin-bottom: var(--n-space-8);
}

/* —— Animate 插件详情窗复刻壳（与 plugins/AnimatePluginWindow.vue 口径同步；组件挂载即调后端不便静态陈列） —— */
.pv-aw-shell {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  width: 544px;
  height: 538px;
  box-sizing: border-box;
  padding: var(--n-space-24);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
}
.pv-aw-title {
  flex: none;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.pv-aw-group {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.pv-aw-sub {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-aw-lines {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.pv-aw-lines li {
  position: relative;
  padding-left: var(--n-space-12);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pv-aw-lines li::before {
  content: "•";
  position: absolute;
  left: 0;
  top: 0;
}
.pv-aw-lines b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-aw-spacer {
  flex: 1;
  min-height: 0;
}
.pv-aw-actions {
  flex: none;
  height: 38px;
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
}
.pv-aw-btn {
  flex: none;
  display: block;
  height: 38px;
}
/* —— MCP 详情窗复刻（与 plugins/McpPluginWindow.vue 口径同步，2026-09-20）：壳高 640，余同 pv-aw 系 —— */
.pv-aw-shell--mcp {
  height: 640px;
}
.pv-svc-lines {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.pv-svc-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
.pv-svc-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.pv-svc-status {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  overflow-wrap: anywhere;
}
/* 接入配置行：深缩进 24 子行（无 bullet）+ copy 钮贴右 */
.pv-aw-lines li.pv-cfg-row {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-6);
  padding-left: var(--n-space-24);
}
.pv-aw-lines li.pv-cfg-row::before {
  content: none;
}
.pv-cfg-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.pv-cfg-row .ic-btn {
  flex: none;
  margin-top: 1px;
}
</style>
