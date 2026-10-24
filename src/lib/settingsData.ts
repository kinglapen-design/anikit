// 偏好设置面板数据层（2026-10-13 接生产；preview 六轮定稿后由纯展示件接线，规范第 1 条 UI 无业务——
// 命令调用 / 状态订阅全部收口本 composable，SettingsDialog.vue 只剩模板与样式）：
// 通用 = ui store 持久化键（spaceZoomMode / doubleClickOpen / mdBundleExportDir）；
// 自动导入 = 空间级 currentSpace + setAutoImport（space.json 单写点，libraryOps）+ 盘符薄查询
// get_inbox_drive_letter；缓存 = get_cache_overview / clear_cache（接口 22；全部清理走
// confirmPrompt warning——不可逆口径：自定义缩略图重建后回退派生图不可复原）+ library-thumbs-rebuilt
// 事件 toast + 就地重拉；插件 = get/save_cep_export_settings（后端存字节，前端 MB 显示换算，
// 护栏钳制与后端 validate 同区间）；关于 = getVersion + show_logs_in_explorer。
// mock = preview 陈列态：全部控件本地陈列、命令一律不发（WsManageDialog isMock 先例）。
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import { open } from "@tauri-apps/plugin-dialog";
import { currentSpace, setAutoImport } from "./libraryOps";
import { formatSize } from "./formatSize";
import { useSelectionStore } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import { useUiStore } from "../stores/ui";

export interface SettingsCacheItem {
  id: string;
  label: string;
  note?: string;
  bytes: number;
}
export interface SettingsMockData {
  spaceZoom: "hover" | "selected";
  doubleClick: "preview" | "app";
  mdBundleDir: string;
  autoImport: boolean;
  inboxDrive: string | null;
  cacheLib: SettingsCacheItem[];
  cacheWs: SettingsCacheItem[];
  cep: { maxSecs: number; maxMB: number; fps: number };
  version: string;
}

interface CacheOverview {
  groups: { id: string; label: string; items: { id: string; label: string; bytes: number; files: number }[] }[];
}
interface ClearOutcome {
  category: string;
  freedBytes: number;
  removedFiles: number;
  failedFiles: number;
  rebuildingThumbs: boolean;
}
interface CepExportSettings {
  maxSecs: number;
  maxBytes: number;
  fps: number;
}

// MB 显示换算（后端存字节）：保留两位小数再修零（0.25MB 护栏粒度）
function mbOf(bytes: number): string {
  return String(Math.round((bytes / 1024 / 1024) * 100) / 100);
}
function clampNum(v: number, lo: number, hi: number): number {
  return Math.min(hi, Math.max(lo, v));
}

export function useSettingsData(mock: SettingsMockData | undefined) {
  const ui = useUiStore();
  const toast = useToastStore();
  const selection = useSelectionStore();
  const isMock = mock !== undefined;

  // —— 通用（mock = 本地陈列；真机 = ui store 持久化键直绑，computed 宽化 string 承 SegmentedControl） ——
  const spaceZoomLocal = ref<string>(mock?.spaceZoom ?? "hover");
  const spaceZoom = computed<string>({
    get: () => (isMock ? spaceZoomLocal.value : ui.spaceZoomMode),
    set: (v) => {
      if (isMock) spaceZoomLocal.value = v;
      else if (v === "hover" || v === "selected") ui.spaceZoomMode = v;
    },
  });
  const doubleClickLocal = ref<string>(mock?.doubleClick ?? "preview");
  const doubleClick = computed<string>({
    get: () => (isMock ? doubleClickLocal.value : ui.doubleClickOpen),
    set: (v) => {
      if (isMock) doubleClickLocal.value = v;
      else if (v === "preview" || v === "app") ui.doubleClickOpen = v;
    },
  });
  const mdBundleLocal = ref(mock?.mdBundleDir ?? "");
  const mdBundleDir = computed<string>({
    get: () => (isMock ? mdBundleLocal.value : ui.mdBundleExportDir),
    set: (v) => {
      const dir = v.trim();
      if (isMock) mdBundleLocal.value = dir;
      else ui.mdBundleExportDir = dir;
    },
  });
  async function chooseMdBundleDir(): Promise<void> {
    if (isMock) {
      mdBundleLocal.value = mock?.mdBundleDir || "D:\\AniKit导出\\MD";
      return;
    }
    try {
      const dest = await open({
        directory: true,
        title: "选择 MD 默认打包导出位置",
        defaultPath: ui.mdBundleExportDir || undefined,
      });
      if (dest) ui.mdBundleExportDir = dest;
    } catch (e) {
      toast.notify({ message: String(e), icon: "fail" });
    }
  }
  function resetMdBundleDir(): void {
    mdBundleDir.value = "";
  }

  // —— 自动导入（空间级设置：换空间后重读盘符；开关失败 currentSpace 不翻 = 开关自动回弹） ——
  const autoImportLocal = ref(mock?.autoImport ?? false);
  const autoImport = computed<boolean>({
    get: () => (isMock ? autoImportLocal.value : currentSpace.autoImport),
    set: (v) => {
      if (isMock) autoImportLocal.value = v;
      else void onToggleAutoImport(v);
    },
  });
  const inboxDrive = ref<string | null>(mock?.inboxDrive ?? null);
  async function refreshDrive(): Promise<void> {
    if (isMock) return;
    inboxDrive.value = await invoke<string | null>("get_inbox_drive_letter").catch(() => null);
  }
  async function onToggleAutoImport(v: boolean): Promise<void> {
    // setAutoImport（libraryOps）= space.json 单写点 + 收件箱启停（盘符挂载/卸载），失败 toast 恒音
    if (await setAutoImport(v)) await refreshDrive();
  }
  watch(
    () => currentSpace.name,
    () => void refreshDrive(),
  );

  // —— 缓存（接口 22：组恒在、条目恒在；note 只给 lib-thumbs「清理后自动重建」——稿内双行口径） ——
  const cacheLib = ref<SettingsCacheItem[]>(mock?.cacheLib ?? []);
  const cacheWs = ref<SettingsCacheItem[]>(mock?.cacheWs ?? []);
  const cacheTotal = computed(() => cacheLib.value.concat(cacheWs.value).reduce((s, it) => s + it.bytes, 0));
  const clearing = ref<string[]>([]); // 清理中类别 id（按钮禁用 = §39 30% 透明度档）
  function cacheLabel(id: string): string {
    return [...cacheLib.value, ...cacheWs.value].find((it) => it.id === id)?.label ?? id;
  }
  async function refreshCache(): Promise<void> {
    if (isMock) return;
    const ov = await invoke<CacheOverview>("get_cache_overview").catch((e: unknown) => {
      toast.notify({ message: String(e), icon: "fail" });
      return null;
    });
    if (!ov) return;
    const mapGroup = (g: string): SettingsCacheItem[] =>
      ov.groups
        .find((x) => x.id === g)
        ?.items.map((it) => ({
          id: it.id,
          label: it.label,
          note: it.id === "lib-thumbs" ? "清理后自动重建" : undefined,
          bytes: it.bytes,
        })) ?? [];
    cacheLib.value = mapGroup("library");
    cacheWs.value = mapGroup("workspace");
  }
  async function runClear(id: string): Promise<number> {
    const r = await invoke<ClearOutcome>("clear_cache", { category: id });
    return r.freedBytes;
  }
  // 单项清理不弹确认（接口 22：全部可重建缓存，不动用户数据）；「全部清理」走 confirmPrompt（不可逆提示）
  async function clearOne(id: string): Promise<void> {
    if (isMock || clearing.value.includes(id)) return;
    clearing.value = [...clearing.value, id];
    try {
      const freed = await runClear(id);
      toast.notify({ message: `已清理「${cacheLabel(id)}」，释放 ${formatSize(freed)}`, icon: "success" });
    } catch (e) {
      toast.notify({ message: String(e), icon: "fail" });
    } finally {
      clearing.value = clearing.value.filter((x) => x !== id);
      await refreshCache();
    }
  }
  function clearAll(): void {
    if (isMock) return;
    selection.confirmPrompt = {
      title: "全部清理缓存",
      body: "清理全部可重建缓存（缩略图与动图、预览与帧缓存、音视频转码与波形、格式图标等），清理后自动重建、不影响素材本体。注意：自定义缩略图重建后回退为默认派生图，不可复原。",
      confirmLabel: "全部清理",
      kind: "warning",
      onConfirm: () => void clearAllRun(),
    };
  }
  async function clearAllRun(): Promise<void> {
    const ids = [...cacheLib.value, ...cacheWs.value].map((it) => it.id);
    clearing.value = ids;
    let freed = 0;
    for (const id of ids) {
      // 逐项清理（接口无批量命令）：单项失败不拖垮整批（未开库 / 未开空间等中文报错逐项 toast）
      try {
        freed += await runClear(id);
      } catch (e) {
        toast.notify({ message: String(e), icon: "fail" });
      }
    }
    clearing.value = [];
    toast.notify({ message: `全部清理完成，共释放 ${formatSize(freed)}`, icon: "success" });
    await refreshCache();
  }

  // —— 插件（Animate CEP 导出参数；失焦 / 回车提交 = FilterRangePop 同口径） ——
  const cepSecs = ref(String(mock?.cep.maxSecs ?? 10));
  const cepMB = ref(String(mock?.cep.maxMB ?? 2));
  const cepFps = ref(String(mock?.cep.fps ?? 0));
  // 最后已知后端值（无变化不打扰后端；非法输入与保存失败的显示回滚基准）
  const cepCurrent = { maxSecs: 10, maxBytes: 2 * 1024 * 1024, fps: 0 };
  async function loadCep(): Promise<void> {
    if (isMock) return;
    const s = await invoke<CepExportSettings>("get_cep_export_settings").catch(() => null);
    if (!s) return;
    Object.assign(cepCurrent, s);
    cepSecs.value = String(s.maxSecs);
    cepMB.value = mbOf(s.maxBytes);
    cepFps.value = String(s.fps);
  }
  async function commitCep(): Promise<void> {
    if (isMock) return;
    // 解析 + 护栏钳制（与后端 CepExportSettings::validate 同区间：时长 1-600 秒 /
    // 体积 256KB-50MB（0.25-50 MB）/ 帧率 0（跟随文档）或 1-120）；非法输入回滚当前值
    const secsRaw = parseFloat(cepSecs.value);
    const mbRaw = parseFloat(cepMB.value);
    const fpsRaw = parseFloat(cepFps.value);
    const next: CepExportSettings = {
      maxSecs: Number.isFinite(secsRaw) ? Math.round(clampNum(secsRaw, 1, 600)) : cepCurrent.maxSecs,
      maxBytes: Number.isFinite(mbRaw) ? Math.round(clampNum(mbRaw, 0.25, 50) * 1024 * 1024) : cepCurrent.maxBytes,
      fps: Number.isFinite(fpsRaw) ? (fpsRaw <= 0 ? 0 : clampNum(fpsRaw, 1, 120)) : cepCurrent.fps,
    };
    cepSecs.value = String(next.maxSecs);
    cepMB.value = mbOf(next.maxBytes);
    cepFps.value = String(next.fps);
    if (next.maxSecs === cepCurrent.maxSecs && next.maxBytes === cepCurrent.maxBytes && next.fps === cepCurrent.fps) return;
    try {
      await invoke("save_cep_export_settings", { settings: next });
      Object.assign(cepCurrent, next);
    } catch (e) {
      toast.notify({ message: String(e), icon: "fail" });
      cepSecs.value = String(cepCurrent.maxSecs);
      cepMB.value = mbOf(cepCurrent.maxBytes);
      cepFps.value = String(cepCurrent.fps);
    }
  }

  // —— 关于（版本读 tauri.conf.json；日志 = show_logs_in_explorer，与主菜单「帮助」同动作） ——
  const version = ref(mock?.version ?? "0.1.0");
  function openLogs(): void {
    if (isMock) return;
    void invoke("show_logs_in_explorer").catch((e) => toast.notify({ message: String(e), icon: "fail" }));
  }

  // 打开面板拉取（mock 陈列态一律不发命令）；library-thumbs-rebuilt = lib-thumbs 清理后的
  // 后台重建完成事件（接口 22）——toast 报数 + 就地重拉占用
  let unlistenRebuilt: (() => void) | undefined;
  onMounted(async () => {
    if (isMock) return;
    void refreshDrive();
    void refreshCache();
    void loadCep();
    version.value = await getVersion().catch(() => version.value);
    unlistenRebuilt = await listen<{ rebuilt: number; fallback: number }>("library-thumbs-rebuilt", (e) => {
      toast.notify({
        message: `缩略图重建完成：${e.payload.rebuilt} 张已重建${e.payload.fallback > 0 ? `，${e.payload.fallback} 张回退格式图标` : ""}`,
        icon: "success",
      });
      void refreshCache();
    });
  });
  onBeforeUnmount(() => unlistenRebuilt?.());

  return {
    isMock,
    spaceZoom,
    doubleClick,
    mdBundleDir,
    chooseMdBundleDir,
    resetMdBundleDir,
    autoImport,
    inboxDrive,
    cacheLib,
    cacheWs,
    cacheTotal,
    clearing,
    clearOne,
    clearAll,
    cepSecs,
    cepMB,
    cepFps,
    commitCep,
    version,
    openLogs,
  };
}
