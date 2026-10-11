// 全局键盘 composable（2026-08-12 自 App.vue 拆出，规范第 3 条行数控制）：
// window keydown 四处理器（剪贴板导入 / 撤销重做 / 选择 / 操作快捷键）+ 注册注销；
// 输入框豁免判定 isEditableTarget 为 App / AssetGrid（空格）共用
//（SidebarLeft 的 Ctrl+N 随 7.1 十八轮左栏轮撤下——新增按钮新稿无位，接菜单栏业务时恢复）。
// 预览让位判定 per-panel（2026-09-25 会话化）：事件时刻取激活 panel 的预览会话
//（activePreviewSession，只查不建）——**激活 panel 预览中**才让位 / 生效，其余 panel 的预览不抢全局键。
import { onBeforeUnmount, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { placementKey, useContentStore } from "../stores/content";
import { usePickerStore } from "../stores/picker";
import { activePreviewSession } from "../stores/preview";
import { useSearchStore } from "../stores/search";
import { useSelectionStore } from "../stores/selection";
import { useTabsStore } from "../stores/tabs";
import { useToastStore } from "../stores/toast";
import { useUiStore } from "../stores/ui";
import { useUndoStore } from "../stores/undo";
import { useWsStore } from "../stores/ws";
import {
  copySelection,
  cutFolders,
  deletePlacements,
  deleteRecycle,
  duplicateSelected,
  pasteClipboard,
  pasteDuplicate,
  recycleSelectedAssetIds,
  sendToAnimate,
} from "./placementOps";
import { cloneFolderFlow, currentLinkTarget, deleteFoldersFlow, enterLinkPicker, findParentId, moveAssetsFlow, moveFoldersFlow } from "./menuOps";
import { openPathWithDefaultApp } from "./wsOps";
import { sendWsToAnimate } from "./menuSpecs/workspace";

// 输入框豁免判定：文本输入态（input / textarea / contenteditable）不拦截全局快捷键
export function isEditableTarget(t: HTMLElement): boolean {
  return t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.isContentEditable;
}

export function useGlobalKeys(opts: { previewParam: string | null; needLibrarySetup: () => boolean }): void {
  const content = useContentStore();
  const picker = usePickerStore();
  const selection = useSelectionStore();
  const undo = useUndoStore();
  const search = useSearchStore();
  const tabs = useTabsStore();
  const ui = useUiStore();

  // 粘贴仲裁（2026-09-17 实机走查修订）：内部剪贴板非空时——系统剪贴板序号未变 = 内部粘贴
  //（链接 / 移动，原 6.1 定案）；**序号变了 = 用户在外部复制了新内容**（覆盖 APP 内剪贴板）→
  // Ctrl+V 让位系统剪贴板导入（「复制本地音频粘贴还是 APP 内图片」实锤）。读取失败 / 未记录
  // 保持原口径（内部优先）。onPaste 与 onOperationKeys 的 Ctrl+V 共用本判定（防双触发）
  async function internalClipboardLive(): Promise<boolean> {
    if (!selection.clipboard) return false;
    if (selection.clipboardSeq === null) return true;
    const seq = await invoke<number>("get_clipboard_seq").catch(() => -1);
    return seq < 0 || seq === selection.clipboardSeq;
  }

  // 剪贴板导入（PRD 05-01；2026-07-27 扩：图片 + 文件列表，恒 Copy）；剪贴板无图无文件属常态，静默降级
  // 6.1 仲裁（2026-07-28 定案）：内部剪贴板非空时 Ctrl+V 归 onOperationKeys（链接粘贴 / 移动，
  // 2026-08-18 起链接粘贴静默不弹窗），本函数只处理"内部剪贴板为空"的系统剪贴板导入；
  // **2026-09-17 修订：内部非空但系统序号已变（外部覆盖）→ 也走本函数系统导入**（见 internalClipboardLive）；
  // Ctrl+Shift+V = 粘贴为副本快捷键，不走导入
  async function onPaste(e: KeyboardEvent): Promise<void> {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "v") return;
    if (e.shiftKey) return;
    if (await internalClipboardLive()) return;
    if (isEditableTarget(e.target as HTMLElement)) return;
    if (opts.needLibrarySetup() || content.importDraft) return;
    if (usePickerStore().active) return; // 选择模式（6.2）：导入是编辑类，禁用
    // 只读视口拒入（2026-09-08 用户定，isNoImportView 统一判定；2026-09-09 收窄扩「全部」）：
    // 全部 / 最近使用 / 自动导入 / 缩略图 / 回收站不允许额外导入——系统剪贴板导入同拖拽口径
    // 弹错误弹窗（与拖拽共用 ui.importRejectOpen）；未分类 = 候车区总入口豁免（落 root）。
    // 工作空间（PRD 02-13，2026-10-01）静默让位：ws 的 Ctrl+V 归 onOperationKeys ws 分支
    //（store 内序号仲裁——内部剪贴板或系统剪贴板文件粘贴进当前目录），库的系统剪贴板导入
    //（图片 / 文本入库）在 ws 视口不触发，不弹拒入窗
    if (content.isWorkspaceView) return;
    if (content.isNoImportView) {
      ui.importRejectOpen = true;
      return;
    }
    try {
      await content.startImportClipboard(content.currentFolderId);
    } catch (err) {
      console.debug("剪贴板导入不可用：", err);
    }
  }

  // 撤销 / 重做（6.0）：Ctrl+Z / Ctrl+Shift+Z；输入框内不拦截（保留文本编辑自身的撤销）
  function onUndoRedo(e: KeyboardEvent): void {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "z") return;
    if (isEditableTarget(e.target as HTMLElement)) return;
    e.preventDefault();
    if (e.shiftKey) {
      void undo.redo();
    } else {
      void undo.undo();
    }
  }

  // 选择快捷键（6.1，2026-07-28 用户定案）：Ctrl+A 全选素材卡片 / Esc 清空选择。
  // 与 onPaste / onUndoRedo 同口径：输入框内不拦截；激活 panel 预览中 / 预览子窗口不抢
  //（PreviewPanel 自行处理 Esc 等）；菜单（AppMenu）/ 悬停大图浮层（HoverZoomOverlay）
  // 打开时让位——两者各自捕获 Esc 关闭，此刻不清选择
  function onSelectionKeys(e: KeyboardEvent): void {
    if (isEditableTarget(e.target as HTMLElement)) return;
    const preview = activePreviewSession(); // 事件时刻取激活 panel 会话（只查不建；无 = null）
    if (opts.previewParam || preview?.assetKey || opts.needLibrarySetup()) return;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "a") {
      if (picker.active) return; // 选择模式：全选是选择操作，禁用
      e.preventDefault();
      selection.selectAll();
      return;
    }
    // 让位判定读"Esc 按下时刻"快照（2026-08-16 用户拍板根治）：AppMenu capture 关菜单后
    // DOM 同步移除，bubble 再查 .app-menu 已失效（误清选择实锤）——capture 早序监听先打标，
    // 此处只读标记：本次 Esc 被菜单吃掉就不清选择；无菜单时维持原语义（清选择）
    if (e.key === "Escape" && !escHadOverlay) {
      // 选择模式（6.2）：Esc = 取消模式全部放弃（PRD 03_数据链接 04），优先于清选择
      if (picker.active) {
        picker.exit();
        return;
      }
      selection.clear();
    }
  }

  // 操作快捷键（6.1 阶段 C，2026-07-28 用户定案，与 onSelectionKeys 同口径）：
  // F2 = 重命名（单选 = 就地重命名：素材卡片；文件夹域单选 = 文件夹重命名——条带可见走卡片命名态，
  // 否则树命名态，3.4 七轮接入；多选素材 = 批量重命名弹窗，2026-09-23 接入；
  // 文件夹域多选 = 批量重命名弹窗文件夹模式，2026-10-11 接入；
  // Ctrl+R 废除 = WebView2 刷新键吞掉防误刷，2026-08-18 用户定）；
  // Del = 删除选中项（2026-07-29 用户修订：取消两段式询问，直接移入回收站，撤销可找回）；
  // Ctrl+C / Ctrl+X = 写内部剪贴板（仅素材）；Ctrl+V 仲裁（内部非空 → copy 静默链接粘贴 /
  // cut 移动；空 → onPaste 系统剪贴板导入）；Ctrl+Shift+V = 直接副本不弹窗（名称同源）；
  // Ctrl+D = 创建副本（2026-08-18 补绑）。
  // 回收站视口（6.1 二轮，2026-07-29）：F2 不触发（只读）；Del 分流彻底删除两段式
  //（delete_recycle，虚拟片段引用确认窗，不可恢复无撤销）——delete_placements 路径不再走。
  // 快速访问视图不作粘贴目标（2026-09-09 收窄修订：未分类豁免 = 候车区总入口可粘贴落 root，
  // 其余快速访问视口 pasteTarget 拦截 toast 提示）；菜单 / 浮层 / 弹窗打开时让位
  async function onOperationKeys(e: KeyboardEvent): Promise<void> {
    if (isEditableTarget(e.target as HTMLElement)) return;
    const preview = activePreviewSession(); // 事件时刻取激活 panel 会话（只查不建；无 = null）
    // Ctrl+E = md 编辑↔预览互切（2026-09-25 二轮走查）：预览态进编辑——预览态本函数整体让位（下行），
    // 故 md 语境独提前判；编辑态退出由 textarea 自拦（输入框让位全局收不到）；「发送至 Animate」
    // fla 键在非预览语境恒先生效（预览的是 md 时 fla 判定本就不命中，双向零冲突）
    if (
      (e.ctrlKey || e.metaKey) &&
      e.key.toLowerCase() === "e" &&
      preview?.assetKey &&
      preview.kind === "text" &&
      (preview.current?.ext ?? "").toLowerCase() === "md"
    ) {
      e.preventDefault();
      preview.requestMdAction("edit");
      return;
    }
    if (opts.previewParam || preview?.assetKey || opts.needLibrarySetup()) return;
    if (document.querySelector(".app-menu, .zoom-mask, .kit-mask, .bm-mask")) return;
    // 选择模式（6.2）：编辑类快捷键全禁（单击已不入选择域，本守卫双保险）；查看类（Shift+Enter
    // 默认应用打开）因选择域恒空天然无效，不单独放行
    if (picker.active) return;
    // 工作空间磁盘实况（PRD 02 步骤 2 + 2026-10-01 库菜单移植轮）：F2 重命名 / Del 回收站删除 /
    // Ctrl+C·X 写 ws 剪贴板（store 内同步写系统剪贴板）/ Ctrl+V 粘贴（恒调，store 内序号仲裁：
    // 内部剪贴板 / 系统剪贴板文件二选一）/ Ctrl+D 创建副本（同目录 _n 规避）/
    // Shift+Enter 默认应用打开 / Ctrl+E 发送至 Animate（单选 fla）接 ws 按路径命令；
    // 其余库快捷键（Y 移动 / J 链接 / L 添加链接等）在 ws 无对应语义，一律不响应——assetId = ws:
    // 合成假 id 严禁落入任何 assetId 类命令。选中集可含文件夹键（2026-09-26 文件夹卡片 / 树行入素材域），
    // 复制 / 剪切 / 删除 / 副本对文件与文件夹混合整批受理（ws 命令按路径无差别）
    if (content.isWorkspaceView) {
      const ws = useWsStore();
      const rels = () =>
        selection.selectedRefs().filter((r) => r.assetId.startsWith("ws:")).map((r) => r.assetId.slice(3));
      const fail = (err: unknown) => useToastStore().notify({ message: String(err), icon: "fail" });
      // Shift+Enter = 默认应用打开（库 6.3 右键菜单同键移植，按路径版命令）
      if (e.key === "Enter" && e.shiftKey) {
        if (selection.count === 1) {
          const targets = rels();
          const abs = targets.length === 1 ? ws.absPath(targets[0]) : null;
          if (abs) {
            e.preventDefault();
            void openPathWithDefaultApp(abs).catch(fail);
          }
        }
        return;
      }
      if (e.key === "F2") {
        if (selection.count === 1) {
          e.preventDefault();
          // 文件夹键路由到文件夹卡片命名态（renamingKey 只有素材卡消费）；仅当前视口条带内
          // 可命名——树行选中的当前目录自身不在条带内，保持无响应（右键菜单 / 名称双击可改）
          const key = selection.first;
          const [assetId, folderId] = key.split("|");
          const entry = ws.entries((folderId ?? "").slice(3)).find((x) => `ws:${x.relPath}` === assetId);
          if (entry?.isDir) {
            if (content.stripFolders.some((f) => f.id === entry.relPath)) selection.namingFolderCardId = entry.relPath;
          } else {
            selection.renamingKey = key;
          }
        }
        return;
      }
      if (e.key === "Delete") {
        const targets = rels();
        if (targets.length > 0) {
          e.preventDefault();
          void ws.removeEntries(targets).catch(fail); // 同库 Del 口径：直接进回收站（可找回），不两段确认
        }
        return;
      }
      if (e.ctrlKey || e.metaKey) {
        const k = e.key.toLowerCase();
        if (k === "c" || k === "x") {
          const targets = rels();
          if (targets.length > 0) {
            e.preventDefault();
            ws.setClip(targets, k === "x");
          }
          return;
        }
        if (k === "v" && !e.shiftKey && !e.altKey) {
          // 粘贴到当前视口目录（sidebarSelected = 活动 viewKey，tabs 同步口径）；
          // 2026-10-01 扩为恒调（PRD 02-13）：内部剪贴板空 / 序号已变时 store 内仲裁落
          // 系统剪贴板文件粘贴（Explorer 复制·剪切的文件粘进当前目录）
          e.preventDefault();
          void ws.paste(content.sidebarSelected.slice(3)).catch(fail);
          return;
        }
        // Ctrl+D = 创建副本（库素材域同键移植）：选中集同目录复制，后端重名 _n 规避
        if (k === "d" && !e.shiftKey && !e.altKey) {
          const targets = rels();
          if (targets.length > 0) {
            e.preventDefault();
            void ws.duplicate(targets).catch(fail);
          }
          return;
        }
        // Ctrl+E = 发送至 Animate（8.3 同键按路径移植）：单选 fla 生效（菜单同款口径）
        if (k === "e" && !e.shiftKey && !e.altKey) {
          if (selection.count === 1) {
            const [assetId, folderId] = selection.first.split("|");
            const entry = ws.entries((folderId ?? "").slice(3)).find((x) => `ws:${x.relPath}` === assetId);
            if (entry && !entry.isDir && entry.ext === "fla") {
              const abs = ws.absPath(entry.relPath);
              if (abs) {
                e.preventDefault();
                sendWsToAnimate(abs);
              }
            }
          }
          return;
        }
      }
      return;
    }
    // F2 = 重命名：单选 = 就地重命名（素材卡片；文件夹域 = 文件夹重命名，3.4 七轮接入——
    // 当前视口条带含该项走卡片命名态（用户视线在卡片），否则树命名态；档案同径）；
    // 多选素材 = 批量重命名弹窗（2026-09-23 接入，右键菜单「批量重命名」同径——37 稿 renameDraft 驱动；
    // 文件夹域多选 = 同弹窗文件夹模式（2026-10-11，renameFolderDraft 驱动）；
    // 回收站只读不触发，虚拟视图与单选同口径放行）
    if (e.key === "F2") {
      if (content.isRecycleView) return;
      // 文件夹域多选 = 批量重命名弹窗（2026-10-11，PRD 02-14；素材多选同径renameDraft）：
      // 编号按树 DFS 视觉序（selectedFolderIdsInTreeOrder），单选维持就地重命名
      if (selection.selectedFolders.size > 1) {
        e.preventDefault();
        selection.renameFolderDraft = selection.selectedFolderIdsInTreeOrder();
        return;
      }
      if (selection.selectedFolders.size === 1) {
        e.preventDefault();
        const id = selection.orderedSelectedFolderIds()[0];
        const parent = findParentId(content.folders, id) ?? null;
        const cur = content.currentFolderId;
        if (cur !== null && parent === cur) selection.namingFolderCardId = id;
        else selection.namingFolderId = id;
        return;
      }
      if (selection.selectedFolders.size === 0 && selection.count === 1) {
        e.preventDefault();
        selection.renamingKey = selection.first;
        return;
      }
      if (selection.selectedFolders.size === 0 && selection.count > 1) {
        e.preventDefault();
        selection.renameDraft = selection.selectedRefs();
      }
      return;
    }
    // Shift+Enter = 默认应用打开（6.3 右键菜单同款，2026-07-29 用户设计）
    if (e.key === "Enter" && e.shiftKey) {
      if (selection.count === 1 && !content.isRecycleView) {
        e.preventDefault();
        const p = content.visiblePlacements.find((v) => placementKey(v) === selection.first);
        if (p) {
          void invoke("open_with_default_app", { assetId: p.assetId }).then(() => content.refreshRecent());
        }
      }
      return;
    }
    if (e.key === "Delete") {
      // 文件夹域分流（3.4 批量删除，2026-08-18）：与素材域互斥，优先判定——
      // 原实现只读素材域，多选文件夹按 Delete 无反应（右键菜单也只删被右键的一个）
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        deleteFoldersFlow(selection.orderedSelectedFolderIds());
        return;
      }
      if (selection.count > 0) {
        e.preventDefault();
        if (content.isRecycleView) {
          void deleteRecycle(recycleSelectedAssetIds()); // 回收站 = 彻底删除两段式
        } else if (e.shiftKey) {
          // Shift+Delete = 从文件夹中移除（3.4 八轮：去未分类；链接记录直接移除）；
          // 虚拟视图禁（2026-09-09 收窄：全部/最近使用/缩略图不是位置，"从哪个文件夹移除"不成立）
          if (!content.isVirtualView) void deletePlacements(selection.selectedRefs(), "uncategorized");
        } else {
          void deletePlacements(selection.selectedRefs()); // Delete = 丢到回收站（虚拟视图同留：删除 = 资产判断）
        }
      }
      return;
    }
    if (!(e.ctrlKey || e.metaKey)) return;
    const k = e.key.toLowerCase();
    if (k === "c" || k === "x") {
      // 文件夹域（3.4 剪切粘贴）：Ctrl+X 剪切文件夹（= 移动，粘贴走 move_folders 护栏）；
      // Ctrl+C 对文件夹无语义（文件夹复制 = 克隆，走 Ctrl+D 克隆）
      if (selection.selectedFolders.size > 0) {
        if (k === "x") {
          e.preventDefault();
          cutFolders(selection.orderedSelectedFolderIds());
        }
        return;
      }
      // 素材域：虚拟视图禁剪切（2026-09-09 收窄：剪切 = 移动前半段，位置级黑箱）；
      // 复制 = 资产引用标记只读，全视口留
      if (selection.count > 0 && (k === "c" || !content.isVirtualView)) {
        e.preventDefault();
        void copySelection(k === "c" ? "copy" : "cut");
      }
      return;
    }
    // Ctrl+D = 创建副本（素材域）/ 克隆文件夹（文件夹域，3.4 四轮复制克隆）
    if (k === "d") {
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        const first = selection.orderedSelectedFolderIds()[0];
        if (first) cloneFolderFlow(first); // 触发项在域内 → 批量分流整个选中集
        return;
      }
      // 素材域副本 = "就地"落点生成新 ID——虚拟视图禁（2026-09-09 收窄：不是位置，"就地"不成立）
      if (selection.count > 0 && !content.isRecycleView && !content.isVirtualView) {
        e.preventDefault();
        void duplicateSelected();
      }
      return;
    }
    // Ctrl+Y = 移动面板（3.4，29 稿）：文件夹域 → 移动文件夹/档案；素材域 → 移动文件
    //（回收站只读不开；虚拟视图禁——2026-09-09 收窄：不是位置，移动 = 黑箱）
    if (k === "y") {
      if (selection.selectedFolders.size > 0) {
        e.preventDefault();
        const first = selection.orderedSelectedFolderIds()[0];
        if (first) moveFoldersFlow(first); // 触发项在域内 → 批量分流整个选中集
      } else if (selection.count > 0 && !content.isRecycleView && !content.isVirtualView) {
        e.preventDefault();
        moveAssetsFlow();
      }
      return;
    }
    // Ctrl+J = 文件链接管理（17 稿，6.2 三轮）：单选素材打开链接管理弹窗（回收站只读不开；
    // 档案无链接管理——多位置未立项，菜单栏与档案右键占位 2026-10-12 已摘，文件夹域不响应）
    if (k === "j") {
      if (selection.selectedFolders.size === 0 && selection.count === 1 && !content.isRecycleView) {
        e.preventDefault();
        const p = content.visiblePlacements.find((v) => placementKey(v) === selection.first);
        if (p) selection.linkManageDraft = { assetId: p.assetId, name: p.name };
      }
      return;
    }
    // Ctrl+L = 添加文件/档案链接（6.2 三轮）：进选择模式文件模板，落点 = 当前文件夹视口
    if (k === "l") {
      const t = currentLinkTarget();
      if (t && !content.isRecycleView) {
        e.preventDefault();
        enterLinkPicker(t);
      }
      return;
    }
    // Ctrl+R 废除（2026-08-18 用户定：WebView2 默认刷新页面快捷键，吞掉防误刷；重命名统一 F2；
    // 批量重命名入口 = 多选右键「批量重命名」菜单项）
    if (k === "r") {
      e.preventDefault();
      return;
    }
    // Ctrl+E = 发送至 Animate（8.3，PRD 07-04）：单选 fla 素材生效（菜单同款口径；回收站只读不发）
    if (k === "e") {
      if (selection.selectedFolders.size === 0 && selection.count === 1 && !content.isRecycleView) {
        const p = content.visiblePlacements.find((v) => placementKey(v) === selection.first);
        if (p && p.ext === "fla") {
          e.preventDefault();
          void sendToAnimate(p.assetId);
        }
      }
      return;
    }
    // 粘贴两键（2026-08-18 定案）：Ctrl+V 链接 / Ctrl+Shift+V 副本；Ctrl+Alt+V 键位废除不响应；
    // 2026-09-17 仲裁修订：系统剪贴板序号已变（外部覆盖）→ 让位 onPaste 系统导入
    if (k === "v" && selection.clipboard) {
      if (e.altKey) return;
      e.preventDefault();
      if (e.shiftKey) {
        void pasteDuplicate(); // Ctrl+Shift+V = 副本：显式键位不参与仲裁（原 6.1：不走导入）
        return;
      }
      if (await internalClipboardLive()) void pasteClipboard();
    }
  }

  // Ctrl+F = 聚焦搜索框（PRD 03_查找文件 01，3.6 一轮）：输入框内也生效（任何输入态一步跳回搜索框）；
  // 预览态（激活 panel）/ 向导态不抢（搜索框不可见）；preventDefault 吞掉 WebView2 页面内查找
  function onSearchFocus(e: KeyboardEvent): void {
    if (!(e.ctrlKey || e.metaKey) || e.key.toLowerCase() !== "f") return;
    const preview = activePreviewSession();
    if (opts.previewParam || preview?.assetKey || opts.needLibrarySetup()) return;
    e.preventDefault();
    search.requestFocus(tabs.activeTabId);
  }

  // Alt+1~4 = 查看方式布局切换（2026-09-06 菜单栏「视图」接业务轮，稿 = 查看方式子菜单快捷键标注）：
  // 自适应 / 瀑布流 / 网格 / 列表 ↔ ui.layoutMode（与视图菜单 / ViewOptionsMenu 同状态源）。
  // 查看类操作——选择模式白名单放行（§12.1 查看方式）；输入框 / 预览态（激活 panel）/ 向导态不抢；
  // 菜单 / 浮层 / 弹窗打开时让位（同 onOperationKeys 口径）
  function onViewKeys(e: KeyboardEvent): void {
    if (!e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
    const modes: Record<string, "justified" | "waterfall" | "grid" | "list"> = {
      "1": "justified",
      "2": "waterfall",
      "3": "grid",
      "4": "list",
    };
    const mode = modes[e.key];
    if (!mode) return;
    if (isEditableTarget(e.target as HTMLElement)) return;
    const preview = activePreviewSession();
    if (opts.previewParam || preview?.assetKey || opts.needLibrarySetup()) return;
    if (document.querySelector(".app-menu, .zoom-mask, .kit-mask, .bm-mask")) return;
    e.preventDefault();
    useUiStore().layoutMode = mode;
  }

  // Esc 按下时刻的浮层快照（2026-08-16 根治"Esc 关菜单误清选择"）：
  // capture 阶段早序记录（本监听随 App 挂载注册，恒早于任何菜单的 capture 关闭处理），
  // bubble 的 onSelectionKeys 只读此标记，不再查实时 DOM（菜单关闭后 DOM 已同步移除，
  // 实时查询必失效）。覆盖全部 AppMenu 系弹层（.app-menu）与悬停大图浮层（.zoom-mask）；
  // AppModal 不吃 Esc（既定口径），不参与。
  let escHadOverlay = false;
  function onEscCapture(e: KeyboardEvent): void {
    if (e.key === "Escape") escHadOverlay = !!document.querySelector(".app-menu, .zoom-mask");
  }

  onMounted(() => {
    window.addEventListener("keydown", onEscCapture, true);
    window.addEventListener("keydown", onPaste);
    window.addEventListener("keydown", onUndoRedo);
    window.addEventListener("keydown", onSelectionKeys);
    window.addEventListener("keydown", onOperationKeys);
    window.addEventListener("keydown", onSearchFocus);
    window.addEventListener("keydown", onViewKeys);
  });
  onBeforeUnmount(() => {
    window.removeEventListener("keydown", onEscCapture, true);
    window.removeEventListener("keydown", onPaste);
    window.removeEventListener("keydown", onUndoRedo);
    window.removeEventListener("keydown", onSelectionKeys);
    window.removeEventListener("keydown", onOperationKeys);
    window.removeEventListener("keydown", onSearchFocus);
    window.removeEventListener("keydown", onViewKeys);
  });
}
