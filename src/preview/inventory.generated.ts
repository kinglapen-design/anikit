// 本文件由 tools/gen-component-inventory.mjs 生成（2026-09-20），勿手改——重跑 `pnpm inventory` 刷新。
// 阶段七重构「组件台账」数据源：status = new 新体系 / legacy 旧件待烧 / business 业务件 / dead 零消费休眠。
export type InventoryStatus = "new" | "legacy" | "business" | "dead";
export interface InventoryEntry {
  path: string;
  name: string;
  status: InventoryStatus;
  consumers: number;
  legacyTokens: number;
  replacement: string;
}
export const inventoryDate = "2026-09-20";
export const inventoryTotals = {
  "new": 66,
  "legacy": 5,
  "business": 70,
  "dead": 2,
  "legacyConsumers": 54,
  "legacyTokens": 426
};
export const inventory: InventoryEntry[] = [
  {
    "path": "components/AppButton",
    "name": "AppButton",
    "status": "legacy",
    "consumers": 16,
    "legacyTokens": 16,
    "replacement": "buttons/ActionButton"
  },
  {
    "path": "components/IconButton",
    "name": "IconButton",
    "status": "legacy",
    "consumers": 16,
    "legacyTokens": 4,
    "replacement": "buttons/IconButton"
  },
  {
    "path": "components/AppModal",
    "name": "AppModal",
    "status": "legacy",
    "consumers": 13,
    "legacyTokens": 12,
    "replacement": "modals/BaseModal"
  },
  {
    "path": "components/AppInput",
    "name": "AppInput",
    "status": "legacy",
    "consumers": 7,
    "legacyTokens": 10,
    "replacement": "inputs/TextInput"
  },
  {
    "path": "components/AppSearchBar",
    "name": "AppSearchBar",
    "status": "legacy",
    "consumers": 2,
    "legacyTokens": 8,
    "replacement": ""
  },
  {
    "path": "components/DetailNotes",
    "name": "DetailNotes",
    "status": "dead",
    "consumers": 0,
    "legacyTokens": 8,
    "replacement": ""
  },
  {
    "path": "components/FolderPickNode",
    "name": "FolderPickNode",
    "status": "dead",
    "consumers": 0,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FloatBarHost",
    "name": "FloatBarHost",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FolderIconGrid",
    "name": "FolderIconGrid",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/GroupRulesEditor",
    "name": "GroupRulesEditor",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 25,
    "replacement": ""
  },
  {
    "path": "components/LinkListRows",
    "name": "LinkListRows",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/PreviewPanel",
    "name": "PreviewPanel",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 2,
    "replacement": ""
  },
  {
    "path": "components/PreviewTitleBar",
    "name": "PreviewTitleBar",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/SideSlider",
    "name": "SideSlider",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/WorkspaceTree",
    "name": "WorkspaceTree",
    "status": "business",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/AnimImgLayer",
    "name": "AnimImgLayer",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ArchiveStrip",
    "name": "ArchiveStrip",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/AssetCard",
    "name": "AssetCard",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/AssetGrid",
    "name": "AssetGrid",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 2,
    "replacement": ""
  },
  {
    "path": "components/BatchRenameDialog",
    "name": "BatchRenameDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 16,
    "replacement": ""
  },
  {
    "path": "components/BlockManageDialog",
    "name": "BlockManageDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 3,
    "replacement": ""
  },
  {
    "path": "components/BlockManageRow",
    "name": "BlockManageRow",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 9,
    "replacement": ""
  },
  {
    "path": "components/ContentArea",
    "name": "ContentArea",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/ContentTitleBar",
    "name": "ContentTitleBar",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ContextMenu",
    "name": "ContextMenu",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/DescEditDialog",
    "name": "DescEditDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 5,
    "replacement": ""
  },
  {
    "path": "components/DetailPanel",
    "name": "DetailPanel",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/DetailSections",
    "name": "DetailSections",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/DockHost",
    "name": "DockHost",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/DuplicateDialog",
    "name": "DuplicateDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 8,
    "replacement": ""
  },
  {
    "path": "components/DuplicateRow",
    "name": "DuplicateRow",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 21,
    "replacement": ""
  },
  {
    "path": "components/DupScanCard",
    "name": "DupScanCard",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 16,
    "replacement": ""
  },
  {
    "path": "components/DupScanDialog",
    "name": "DupScanDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 38,
    "replacement": ""
  },
  {
    "path": "components/DupScanGroupRow",
    "name": "DupScanGroupRow",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FailedListDialog",
    "name": "FailedListDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FolderIconPanel",
    "name": "FolderIconPanel",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FolderStrip",
    "name": "FolderStrip",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/FolderTree",
    "name": "FolderTree",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/GroupEditDialog",
    "name": "GroupEditDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 7,
    "replacement": ""
  },
  {
    "path": "components/GroupManageDialog",
    "name": "GroupManageDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 12,
    "replacement": ""
  },
  {
    "path": "components/HoverAudioLayer",
    "name": "HoverAudioLayer",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/HoverSequenceLayer",
    "name": "HoverSequenceLayer",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/HoverVideoLayer",
    "name": "HoverVideoLayer",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/HoverZoomOverlay",
    "name": "HoverZoomOverlay",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/ImportProgressBar",
    "name": "ImportProgressBar",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/InboxDiskSection",
    "name": "InboxDiskSection",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 4,
    "replacement": ""
  },
  {
    "path": "components/LibraryView",
    "name": "LibraryView",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 3,
    "replacement": ""
  },
  {
    "path": "components/LinkManageDialog",
    "name": "LinkManageDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 28,
    "replacement": ""
  },
  {
    "path": "components/MoveNode",
    "name": "MoveNode",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/MovePanel",
    "name": "MovePanel",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/OrphanInfosAlert",
    "name": "OrphanInfosAlert",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 6,
    "replacement": ""
  },
  {
    "path": "components/PathErrorDialog",
    "name": "PathErrorDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 13,
    "replacement": ""
  },
  {
    "path": "components/PickerPanel",
    "name": "PickerPanel",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/PreviewFont",
    "name": "PreviewFont",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 9,
    "replacement": ""
  },
  {
    "path": "components/PreviewNavigator",
    "name": "PreviewNavigator",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/PreviewStreamCanvas",
    "name": "PreviewStreamCanvas",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/PreviewText",
    "name": "PreviewText",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 17,
    "replacement": ""
  },
  {
    "path": "components/PreviewWindow",
    "name": "PreviewWindow",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/PropsDialog",
    "name": "PropsDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 9,
    "replacement": ""
  },
  {
    "path": "components/RecycleConfirmDialog",
    "name": "RecycleConfirmDialog",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 13,
    "replacement": ""
  },
  {
    "path": "components/SectionHeader",
    "name": "SectionHeader",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 11,
    "replacement": ""
  },
  {
    "path": "components/SequencePackView",
    "name": "SequencePackView",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 12,
    "replacement": ""
  },
  {
    "path": "components/SetupWizard",
    "name": "SetupWizard",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 15,
    "replacement": ""
  },
  {
    "path": "components/SidebarExpandButton",
    "name": "SidebarExpandButton",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/SidebarLeft",
    "name": "SidebarLeft",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/SidebarRight",
    "name": "SidebarRight",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/SplitBar",
    "name": "SplitBar",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/TabAddButton",
    "name": "TabAddButton",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/TabLabel",
    "name": "TabLabel",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/TermChipsInput",
    "name": "TermChipsInput",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 9,
    "replacement": ""
  },
  {
    "path": "components/TreeItem",
    "name": "TreeItem",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 30,
    "replacement": ""
  },
  {
    "path": "components/ViewOptionsMenu",
    "name": "ViewOptionsMenu",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/WindowControls",
    "name": "WindowControls",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/WizardStepLibrary",
    "name": "WizardStepLibrary",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 5,
    "replacement": ""
  },
  {
    "path": "components/WizardStepPlaceholder",
    "name": "WizardStepPlaceholder",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 3,
    "replacement": ""
  },
  {
    "path": "components/WizardStepSpace",
    "name": "WizardStepSpace",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 8,
    "replacement": ""
  },
  {
    "path": "components/WsTreeRows",
    "name": "WsTreeRows",
    "status": "business",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/AppIcon",
    "name": "AppIcon",
    "status": "new",
    "consumers": 57,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ScrollThumb",
    "name": "ScrollThumb",
    "status": "new",
    "consumers": 22,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/ActionButton",
    "name": "ActionButton",
    "status": "new",
    "consumers": 13,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/IconButton",
    "name": "IconButton",
    "status": "new",
    "consumers": 10,
    "legacyTokens": 0,
    "replacement": "buttons/IconButton"
  },
  {
    "path": "components/items/SectionTitle",
    "name": "SectionTitle",
    "status": "new",
    "consumers": 8,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/SegmentedControl",
    "name": "SegmentedControl",
    "status": "new",
    "consumers": 6,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/TwoToneIcon",
    "name": "TwoToneIcon",
    "status": "new",
    "consumers": 6,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/SwitchButton",
    "name": "SwitchButton",
    "status": "new",
    "consumers": 5,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/cards/FormatBadge",
    "name": "FormatBadge",
    "status": "new",
    "consumers": 5,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/inputs/TextInput",
    "name": "TextInput",
    "status": "new",
    "consumers": 5,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/TreeRowItem",
    "name": "TreeRowItem",
    "status": "new",
    "consumers": 5,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/modals/BaseModal",
    "name": "BaseModal",
    "status": "new",
    "consumers": 5,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/AppMenu",
    "name": "AppMenu",
    "status": "new",
    "consumers": 4,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/BarSlider",
    "name": "BarSlider",
    "status": "new",
    "consumers": 4,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/Radio",
    "name": "Radio",
    "status": "new",
    "consumers": 4,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/Checkbox",
    "name": "Checkbox",
    "status": "new",
    "consumers": 3,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/MenuItem",
    "name": "MenuItem",
    "status": "new",
    "consumers": 3,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/cards/FileCard",
    "name": "FileCard",
    "status": "new",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/BaseDataItem",
    "name": "BaseDataItem",
    "status": "new",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/modals/ConfirmModal",
    "name": "ConfirmModal",
    "status": "new",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ToastStack",
    "name": "ToastStack",
    "status": "new",
    "consumers": 2,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/AppConfirm",
    "name": "AppConfirm",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/FloatBar",
    "name": "FloatBar",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/FloatBarMarkdown",
    "name": "FloatBarMarkdown",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/FloatBarPlay",
    "name": "FloatBarPlay",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/FloatBarSearch",
    "name": "FloatBarSearch",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/FrameStrip",
    "name": "FrameStrip",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/NotesPop",
    "name": "NotesPop",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/RatePop",
    "name": "RatePop",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/VolumePop",
    "name": "VolumePop",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/bars/WaveProgress",
    "name": "WaveProgress",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/FontInstallButton",
    "name": "FontInstallButton",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/TagButton",
    "name": "TagButton",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/cards/ArchiveCard",
    "name": "ArchiveCard",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 1,
    "replacement": ""
  },
  {
    "path": "components/cards/FolderCard",
    "name": "FolderCard",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/cards/ListHeader",
    "name": "ListHeader",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/cards/ListRowCard",
    "name": "ListRowCard",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/CreateLibraryModal",
    "name": "CreateLibraryModal",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/EmptyFolderState",
    "name": "EmptyFolderState",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ImportDialog",
    "name": "ImportDialog",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/ColorSwatchItem",
    "name": "ColorSwatchItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/LinkAudioItem",
    "name": "LinkAudioItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/LinkFileItem",
    "name": "LinkFileItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/LinkRelationItem",
    "name": "LinkRelationItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/PickFileItem",
    "name": "PickFileItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/PickRelationItem",
    "name": "PickRelationItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/TabItem",
    "name": "TabItem",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/LinkListPopover",
    "name": "LinkListPopover",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ListHeader",
    "name": "ListHeader",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/ListRowCard",
    "name": "ListRowCard",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/MergeLibraryModal",
    "name": "MergeLibraryModal",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/PickerBanner",
    "name": "PickerBanner",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/plugins/AnimatePluginWindow",
    "name": "AnimatePluginWindow",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/plugins/McpPluginWindow",
    "name": "McpPluginWindow",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/plugins/PluginListPanel",
    "name": "PluginListPanel",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/CoverSettings",
    "name": "CoverSettings",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/FolderDetail",
    "name": "FolderDetail",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/SectionPinned",
    "name": "SectionPinned",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/TabBasic",
    "name": "TabBasic",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/TabLinks",
    "name": "TabLinks",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/sections/TabPrompt",
    "name": "TabPrompt",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/SequenceDialog",
    "name": "SequenceDialog",
    "status": "new",
    "consumers": 1,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/buttons/DropButton",
    "name": "DropButton",
    "status": "new",
    "consumers": 0,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/LinkThumbItem",
    "name": "LinkThumbItem",
    "status": "new",
    "consumers": 0,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/NoteItem",
    "name": "NoteItem",
    "status": "new",
    "consumers": 0,
    "legacyTokens": 0,
    "replacement": ""
  },
  {
    "path": "components/items/PollutionFileItem",
    "name": "PollutionFileItem",
    "status": "new",
    "consumers": 0,
    "legacyTokens": 0,
    "replacement": ""
  }
];
