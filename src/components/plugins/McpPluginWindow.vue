<script setup lang="ts">
// AI 外接通道（MCP）详情子窗口（2026-09-20，待办细节清单 09-20 立项；版式仿 AnimatePluginWindow——
// 544×640 非模态独立窗，可移动/缩放、单例聚焦，open_plugin_window 后端命令，URL ?plugin=mcp 分发）：
// 四组内容 = 服务（MCP 服务 + 允许写入 两开关同一行）/ 使用教程（三种 Agent 接入配置 + 复制钮）/
// 功能列表（只读 8 + 写 9）/ 路径（服务地址 + 端口文件）；底部按钮行 = 左件运行状态 + 右件「关闭」钮
//（MCP 无文件可装，无安装/卸载——「真安装动作 = 写 Agent 配置」，本轮做展示+复制，一键写配置属远期项）。
// 同日 UI 调整轮（用户实机标注定案）：① 两开关同一行 gap 12（省纵向空间）；② 运行状态挪按钮行左件，
// 三轮定 = **圆点+字重双指示**（圆点保留——用户澄清要废的是「运行中·http」的间隔中点非绿点）——
// 圆点 8×8 联通 状态/成功色、断开 亮色/四级；状态词加粗 运行中 亮色/二级、已停止 亮色/三级，三件套 gap 8；
// ③ 接入配置行学 md 代码块 = 圆角矩形包裹（圆角 6 / 底色与描边同 亮色/半透明-4，深缩进子行口径废除）；
// ④ 强调词写法统一 = 空格+[词]+空格（[MCP 服务] / [允许写入] / [只读]，直角引号与裸加粗废除；
// 行首类别词 只读（8 个）/ 写入（9 个） 为加粗标签不入此例）；⑤ **UI 显示名 = MCP 连接器**
//（用户定：名字不要括号——「你们特别爱加括号」；文档术语 AI 外接通道（MCP）不变，仅 UI 名去括号）。
// 开关组口径沿退役的 SettingsModal 既有定案（开关左 + 文字右 gap 8，标签 13/18 二级）；
// 复制钮 = IconButton bare sm copy（既有库件，零新样式）。
// 状态零 ui store：子窗口 store 快照与主窗口双写竞态（save_ui_settings 整体替换语义），
// 两开关直走 commands/mcp.rs（settings.mcp 子树单写点），动作后以回包校正本地态（失败不回滚假象）。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import ActionButton from "../buttons/ActionButton.vue";
import IconButton from "../buttons/IconButton.vue";
import SwitchButton from "../buttons/SwitchButton.vue";
import ScrollThumb from "../ScrollThumb.vue";
import { getMcpStatus, setMcpServerEnabled, setMcpWriteEnabled, type McpStatus } from "../../lib/plugins";
import { useToastStore } from "../../stores/toast";

const status = ref<McpStatus | null>(null);
const busy = ref(false); // 开关动作互斥
const bodyRef = ref<HTMLElement>();
const toast = useToastStore();

onMounted(async () => {
  status.value = await getMcpStatus();
  window.addEventListener("keydown", onKeydown, true);
});
onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown, true);
});

// Esc = 关窗口（与「关闭」钮同径；本窗无菜单/输入框争用 Esc）
function onKeydown(e: KeyboardEvent): void {
  if (e.key === "Escape") void getCurrentWindow().close();
}
function closeWin(): void {
  void getCurrentWindow().close();
}

// 服务总开关 / 写入闸：不用 v-model 直绑——点击只发动作，本地态以 command 回包校正
async function onServerToggle(enabled: boolean): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const st = await setMcpServerEnabled(enabled);
    if (st) status.value = st;
  } finally {
    busy.value = false;
  }
}
async function onWriteToggle(enabled: boolean): Promise<void> {
  if (busy.value) return;
  busy.value = true;
  try {
    const st = await setMcpWriteEnabled(enabled);
    if (st) status.value = st;
  } finally {
    busy.value = false;
  }
}

// 状态词（按钮行左件，字重区分：运行中加粗亮二级 / 已停止加粗留三级）；地址 = running 时的实际 url
const stateText = computed(() => {
  const st = status.value;
  if (!st) return "读取中…";
  return st.running ? "运行中" : "已停止";
});

// 接入配置文本：服务未起时按固定端口 17701 兜底（Agent 侧本来就是静态配置）
const url = computed(() => status.value?.url ?? "http://127.0.0.1:17701/mcp");
const cfgKimi = computed(() => `{ "mcpServers": { "anikit": { "url": "${url.value}" } } }`);
const cfgCodex = computed(() => `[mcp_servers.anikit]\nurl = "${url.value}"`);
const cfgClaude = computed(() => `claude mcp add --transport http anikit ${url.value}`);

async function copyCfg(text: string, name: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    toast.notify({ message: `${name} 接入配置已复制`, icon: "success" });
  } catch {
    toast.notify({ message: "复制失败", icon: "fail" });
  }
}
</script>

<template>
  <div class="plugin-window">
    <!-- 顶部拖拽带（AnimatePluginWindow 同口径：通栏 62px，容器与 h1 双挂 data-tauri-drag-region） -->
    <div class="pw-dragband" data-tauri-drag-region>
      <h1 class="pw-title" data-tauri-drag-region>MCP 连接器</h1>
    </div>
    <div class="pw-scroll-wrap">
      <div ref="bodyRef" class="pw-body kit-scroll">
        <!-- 服务组：两开关同一行（2026-09-20 用户定：省纵向空间，两开关间 gap 12） -->
        <section class="pw-group">
          <h2 class="pw-sub">服务</h2>
          <div class="svc-row">
            <span class="svc-item">
              <SwitchButton
                :model-value="status?.serverEnabled ?? false"
                :disabled="busy || !status"
                @update:model-value="onServerToggle"
              />
              <span class="svc-label">MCP 服务</span>
            </span>
            <span class="svc-item">
              <SwitchButton
                :model-value="status?.writeEnabled ?? false"
                :disabled="busy || !status?.serverEnabled"
                @update:model-value="onWriteToggle"
              />
              <span class="svc-label">允许写入</span>
            </span>
          </div>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">使用教程</h2>
          <ul class="pw-lines">
            <li>开启上方 <b>[MCP 服务]</b>（默认开），AniKit 会在本机拉起 MCP 服务供外部 Agent 接入。</li>
            <li>在你的 Agent 工具里接入 AniKit（三选一，点右侧复制钮拷贝配置）：</li>
            <li class="cfg-row">
              <span class="cfg-text"><b>Kimi Code</b>：mcp.json 加 {{ cfgKimi }}</span>
              <IconButton name="copy" variant="bare" size="sm" title="复制" @click="copyCfg(cfgKimi, 'Kimi Code')" />
            </li>
            <li class="cfg-row">
              <span class="cfg-text"><b>Codex</b>：config.toml 加 [mcp_servers.anikit] 与 url = "{{ url }}"</span>
              <IconButton name="copy" variant="bare" size="sm" title="复制" @click="copyCfg(cfgCodex, 'Codex')" />
            </li>
            <li class="cfg-row">
              <span class="cfg-text"><b>Claude Code</b>：运行 {{ cfgClaude }}</span>
              <IconButton name="copy" variant="bare" size="sm" title="复制" @click="copyCfg(cfgClaude, 'Claude Code')" />
            </li>
            <li>
              接入后 Agent 默认只能 <b>[只读]</b>（搜索、浏览、看图等 8 个工具）；需要 Agent 写入时开启上方
              <b>[允许写入]</b>（9 个写工具；彻底删除永不经 MCP，回收站清空须 APP 内人工）。
            </li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">功能列表</h2>
          <ul class="pw-lines">
            <li>
              <b>只读（8 个）</b>：当前库状态 / 搜索素材 / 文件夹树 / 文件夹资产清单 / 资产详情 /
              源文件与缩略图磁盘路径（看图入口）/ 读文本资产 / 档案列表。
            </li>
            <li>
              <b>写入（9 个，需开 [允许写入]）</b>：新建文档 / 覆写文本资产 / 批量改名 / 新建文件夹 /
              资产移入文件夹 / 移动文件夹 / 链接汇总 / 导入磁盘文件入库 / 丢回收站（可逆，APP 内 Ctrl+Z 可撤）。
            </li>
          </ul>
        </section>
        <section class="pw-group">
          <h2 class="pw-sub">路径</h2>
          <ul class="pw-lines">
            <li class="pw-path">服务地址: {{ status?.url ?? "未运行（固定端口 17701 优先，被占回退动态）" }}</li>
            <li class="pw-path">端口文件: %APPDATA%\AniKit\mcp-server.port</li>
          </ul>
        </section>
      </div>
      <ScrollThumb :body="bodyRef" />
    </div>
    <!-- 按钮行（稿口径同 Animate 窗：高 38；左件 = 运行状态（2026-09-20 用户定挪左下角：
         圆点 联通=成功色/断开=四级灰 + 状态文字），右件 = 关闭钮） -->
    <div class="pw-actions">
      <span class="svc-status">
        <span class="svc-dot" :class="{ on: status?.running }" />
        <span class="svc-state" :class="{ on: status?.running }">{{ stateText }}</span>
        <span v-if="status?.url" class="svc-url">{{ status.url }}</span>
      </span>
      <span class="pw-btn">
        <ActionButton label="关闭" variant="cancel" size="lg" @click="closeWin" />
      </span>
    </div>
  </div>
</template>

<style scoped>
/* 版式与 AnimatePluginWindow 同口径（稿 544×538 系：padding 24 / 列 gap 24；窗口壳底色/描边/圆角
   = App.vue .window 承担，本组件只画内容面板）；flex:1 必填（App .window 横向 flex） */
.plugin-window {
  flex: 1;
  min-width: 0;
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding: var(--n-space-24);
}
.pw-title {
  margin: 0;
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
/* 顶部拖拽带 = 通栏 62px（负 margin 三向扩；margin-bottom -12 与 padding-bottom 12 对消） */
.pw-dragband {
  flex: none;
  margin: calc(-1 * var(--n-space-24)) calc(-1 * var(--n-space-24)) calc(-1 * var(--n-space-12));
  padding: var(--n-space-24) var(--n-space-24) var(--n-space-12);
}
/* 组区滚动包裹：relative = ScrollThumb 锚点；margin-right 负扩 24 = 滚轮紧贴窗口右缘 */
.pw-scroll-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
  margin-right: calc(-1 * var(--n-space-24));
}
.pw-body {
  height: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding-right: var(--n-space-24);
}
.pw-group {
  flex: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.pw-sub {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
/* 行组（稿 提示文字 gap 6）：13/18 三级；bullet 悬挂缩进 12 */
.pw-lines {
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-6);
}
.pw-lines li {
  position: relative;
  padding-left: var(--n-space-12);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pw-lines li::before {
  content: "•";
  position: absolute;
  left: 0;
  top: 0;
}
.pw-lines b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
/* 接入配置块（2026-09-20 用户定：学 md 代码块——圆角矩形包裹，圆角 6 / 底色与描边同 亮色/半透明-4，
   描边 inside 不占空间 §口径；padding 6 8 就近套间距档）：无自身 bullet、通栏（深缩进子行口径废除），
   文本长串折行 + 复制钮贴右 */
.pw-lines li.cfg-row {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-6);
  padding: var(--n-space-6) var(--n-space-8);
  background: var(--n-l-a4);
  box-shadow: inset 0 0 0 1px var(--n-l-a4);
  border-radius: var(--n-radius-6);
}
.pw-lines li.cfg-row::before {
  content: none;
}
.cfg-text {
  flex: 1;
  min-width: 0;
  overflow-wrap: anywhere;
}
.cfg-row .ic-btn {
  flex: none;
  margin-top: 1px; /* 与首行文字基线对齐（14 盒对 18 行高） */
}
/* 服务组：两开关同一行 gap 12（2026-09-20 用户定）；单开关组内 = 开关左 + 文字右 gap 8（沿退役 SettingsModal 定案），标签 13/18 二级 */
.svc-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.svc-item {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
}
.svc-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
/* 运行状态（按钮行左件）：圆点 + 字重双指示（2026-09-20 用户定——圆点保留，废的是「运行中·http」的
   间隔中点）——圆点 8×8 联通 状态/成功色、断开 亮色/四级；状态词加粗 运行中 亮色/二级、
   已停止·读取中 亮色/三级（保留层级，仅开启时升二级）；三件套间 gap 8 */
.svc-status {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
}
.svc-dot {
  width: 8px;
  height: 8px;
  border-radius: 99px;
  background: var(--n-l4);
  flex: none;
}
.svc-dot.on {
  background: var(--n-success);
}
.svc-state {
  flex: none;
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
.svc-state.on {
  color: var(--n-l2);
}
.svc-url {
  color: var(--n-l3);
  overflow-wrap: anywhere;
}
/* 路径行：长路径折行 */
.pw-path {
  overflow-wrap: anywhere;
}
/* 按钮行（稿：高 38；左件状态 + 右件关闭 = space-between——纯按钮稿为右对齐，本窗有左件 2026-09-20 用户定） */
.pw-actions {
  flex: none;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-8);
}
.pw-btn {
  flex: none;
  display: block;
  height: 38px;
}
/* 按钮宽自适应内容 + 左右 48px（2026-09-13 实机走查用户定稿值） */
.pw-actions .pw-btn :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) 48px;
}
</style>
