// 插件列表面板（浮层）全局状态与打开逻辑（2026-09-06 插件面板轮）：
// 三入口共用（SidebarLeft「插件」文字钮 / ContentTitleBar 工具栏 plugin 钮 / PreviewTitleBar plugin 钮），
// 宿主 App.vue 唯一实例（预览子窗口同在）——ContextMenu 菜单同位口径。
// 非模态浮层（用户定）：点面板外 / Esc 关（AppMenu 机制）、同钮再点 toggle 关（trigger 豁免，
// 点触发钮不算点外——速查 §26 M7 教训）；选择模式（picker.active）禁开（视觉不改——稿无禁用态）。
// 弹出位 = 触发钮下方（左缘对齐 rect.left、顶 = rect.bottom + 4，菜单栏「菜单」钮同位口径），
// 视口右/底缘钳制归 AppMenu。
import { reactive } from "vue";
import { usePickerStore } from "../stores/picker";

export const pluginPanelState = reactive({
  open: false,
  x: 0,
  y: 0,
  trigger: null as HTMLElement | null,
});

export function openPluginPanel(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  // 同钮再点 = toggle 关；他钮开着时换锚重开
  if (pluginPanelState.open && pluginPanelState.trigger === el) {
    pluginPanelState.open = false;
    return;
  }
  if (usePickerStore().active) return; // 选择模式禁开（白名单收窄，§12.1 同口径）
  const rect = el.getBoundingClientRect();
  pluginPanelState.x = rect.left;
  pluginPanelState.y = rect.bottom + 4;
  pluginPanelState.trigger = el;
  pluginPanelState.open = true;
}

export function closePluginPanel(): void {
  pluginPanelState.open = false;
}
