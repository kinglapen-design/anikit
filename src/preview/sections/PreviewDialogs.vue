<script setup lang="ts">
// 业务弹窗展区（阶段七换代盘点，2026-09-21）：src/components/ 根目录 12 件业务弹窗全量收编——
// 全部为旧体系（var(--c* / var(--text-* / var(--line-*)）待换代，逐格 caption 标现状。
// 处理模式：
//   真实弹出/直渲（纯 props、不读 store、挂载不调后端）：
//     - FailedListDialog（真实弹出：title/failed props 直通；重试/退出仅关闭，预览不接后端）
//     - PathErrorDialog（真实直渲：纯 props 卡片无遮罩；按钮仅演示不接后端）
//   静态复刻（依赖 pinia store 数据或挂载即 invoke，按 PreviewModals pv-modal-shell 惯例；
//   壳样式只吃 --n- token，壳内尽量嵌真实旧子组件——旧样式即待换代对象本身）：
//     - DuplicateDialog（嵌真实 DuplicateRow；读 content.importConflicts，确认接后端）
//     - DupScanDialog（嵌真实 DupScanGroupRow；读 dupScanState，合并接后端）
//     - DescEditDialog（嵌真实 IconButton/AppInput；读 ui.descEdit，确认接后端）
//     - PropsDialog（嵌真实 AppSearchBar/TagButton removable；onMounted 调 listPropPool）
//     - GroupEditDialog / GroupManageDialog（嵌真实 GroupRulesEditor + AppInput；读 content.folders，确认接后端）
//     - BlockManageDialog（嵌真实 BlockManageRow；读 content.blocks 快照，确认接后端）
//     - BatchRenameDialog（读 selection.renameDraft + content.placements，确认接后端）
//     - LinkManageDialog（读 selection.linkManageDraft + content 树，确认接后端）
//     - RecycleConfirmDialog（读 selection.recyclePrompt；确认 = 彻底删除，绝不真弹）
// 复刻壳宽度按各组件稿值（973/717/603/559/482/420），高度按演示内容从简（真实组件多高随内容/视口）；
// 口径变更时与各组件同步。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import AppIcon from "../../components/AppIcon.vue";
import IconButton from "../../components/IconButton.vue";
import AppButton from "../../components/AppButton.vue";
import AppInput from "../../components/AppInput.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import SwitchButton from "../../components/buttons/SwitchButton.vue";
import Checkbox from "../../components/buttons/Checkbox.vue";
import AppSearchBar from "../../components/AppSearchBar.vue";
import TagButton from "../../components/buttons/TagButton.vue";
import BlockManageRow from "../../components/BlockManageRow.vue";
import DuplicateRow, { type DupDecision } from "../../components/DuplicateRow.vue";
import DupScanGroupRow from "../../components/DupScanGroupRow.vue";
import GroupRulesEditor from "../../components/GroupRulesEditor.vue";
import PathErrorDialog from "../../components/PathErrorDialog.vue";
import FailedListDialog from "../../components/FailedListDialog.vue";
import type { BlockDraft } from "../../lib/blockOps";
import type { ConflictItem } from "../../stores/content";
import type { DupGroup } from "../../lib/dupScanOps";
import type { GroupRule } from "../../lib/grouping";

// —— FailedListDialog 真实弹出（props 直通；重试/退出仅关闭演示）——
const failedOpen = ref(false);
const failedDemo = [
  { path: "C:/演示/素材/走路循环_破损.fla", reason: "文件损坏" },
  { path: "C:/演示/素材/表情集_2026.psd", reason: "格式不支持" },
  { path: "", reason: "磁盘读取失败" },
];

// —— DuplicateDialog 复刻桩（嵌真实 DuplicateRow：radio 三选一 + 跳过开关可交互，纯前端）——
const dupConflict: ConflictItem = {
  sourcePath: "C:/演示/导入/雪牧_自动.fla",
  sourceName: "雪牧_自动.fla",
  sourceSize: 1_258_291,
  sourceW: null,
  sourceH: null,
  sourceThumbPath: null,
  targetFolder: "root",
  targetFolderName: "根文件夹",
  existingAssetId: "demo-existing",
  existingName: "雪牧_自动.fla",
  existingW: null,
  existingH: null,
  existingSize: 1_258_291,
  existingExt: "fla",
  existingThumbPath: null,
  existingFolderName: "01_人物",
};
const dupDecision = ref<DupDecision>({ action: "link", skipped: false });
const dupFollowAll = ref(true);

// —— DupScanDialog 复刻桩（嵌真实 DupScanGroupRow；组复选/卡片横排可交互，纯前端）——
const scanGroup: DupGroup = {
  hash: "demo-hash",
  size: 1_258_291,
  checked: true,
  assets: [
    {
      assetId: "scan-a1",
      name: "跑步循环_01.fla",
      folderId: "f-renwu",
      folderName: "01_人物",
      w: null,
      h: null,
      ext: "fla",
      size: 1_258_291,
      thumbPath: null,
      thumbKind: null,
      placements: [{ folderId: "f-renwu", origin: true }],
    },
    {
      assetId: "scan-a2",
      name: "跑步循环_01 (2).fla",
      folderId: "f-zaji",
      folderName: "杂集",
      w: null,
      h: null,
      ext: "fla",
      size: 1_258_291,
      thumbPath: null,
      thumbKind: null,
      placements: [{ folderId: "f-zaji", origin: true }],
    },
  ],
};

// —— 分组两件复刻桩（嵌真实 GroupRulesEditor；候选范围按 items 出，空 items = 只展示已选 chips）——
const geName = ref("");
const geRules = ref<GroupRule[]>([{ kind: "ext", values: ["png", "fla"] }]);
const gmName = ref("表情");
const gmRules = ref<GroupRule[]>([{ kind: "ext", values: ["gif", "webp"] }]);

// —— BlockManageDialog 复刻桩（嵌真实 BlockManageRow；行编辑/排序钮为本地演示）——
const bmRows = ref<BlockDraft[]>([
  { key: "b-dir", id: "dir", name: "目录", icon: null },
  { key: "b-ref", id: "b-ref", name: "学习参考", icon: null },
]);

// —— BatchRenameDialog 复刻桩（真实 SegmentedControl/AppInput；行卡 = 壳内复刻）——
const renameMode = ref("sequence");
const RENAME_MODES = [
  { value: "sequence", label: "序号命名" },
  { value: "replace", label: "替换命名" },
] as const;
const renameName = ref("角色");
const renameStart = ref("1");

// —— DescEditDialog 复刻桩（真实 IconButton/AppInput）——
const descRow = ref({ title: "出处", content: "雪牧老师公开素材" });

// —— PropsDialog 复刻桩（真实 AppSearchBar/TagButton removable）——
const propsKeyword = ref("");
const propsOwned = ref(["雪牧", "走路循环"]);

// —— LinkManageDialog 复刻桩（真实 AppSearchBar/Checkbox/AppIcon；树行 = 壳内复刻）——
const linkQuery = ref("");
const linkOnlyChecked = ref(false);
</script>

<template>
  <section id="design-system-dialogs" class="pv-section">
    <h2 class="pv-sec-title">业务弹窗（阶段七换代盘点）</h2>
    <p class="pv-sec-desc">
      src/components/ 根目录 12 件业务弹窗全量收编：<b>全部为旧体系（--c* / --text-* / --line-*）待换代</b>。
      真实弹出/直渲仅 FailedListDialog、PathErrorDialog 两件（纯 props、不读 store、挂载不调后端；确认类按钮仅关闭演示，预览不接后端）；
      其余十件依赖 store 数据或挂载即 invoke，按 pv-modal-shell 惯例静态复刻（壳吃 --n- token，壳内嵌真实旧子组件，口径变更与组件同步）。
      壳宽按各组件稿值，高度按演示内容从简。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">导入链路（重复提示 / 扫描合并 / 失败清单）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w973">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">重复文件提示</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body">
              <div class="pv-old-cols">
                <span class="col">已存在文件</span>
                <span class="col">新的文件</span>
              </div>
              <DuplicateRow
                :item="dupConflict"
                :decision="dupDecision"
                @change="dupDecision = { ...dupDecision, ...$event }"
              />
            </div>
            <div class="pv-old-foot">
              <span class="pv-old-foot-label">全部跟随</span>
              <SwitchButton v-model="dupFollowAll" />
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            DuplicateDialog｜导入 hash 冲突逐行决策（radio 三选一 + 跳过，36 稿）｜旧体系待换代｜静态复刻嵌真实
            DuplicateRow（行可交互）；本体读 content.importConflicts、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w717">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">扫描重复文件</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <DupScanGroupRow :group="scanGroup" />
            <div class="pv-old-foot spread">
              <span class="pv-old-summary">已选择 <b>2</b> 个文件 · 可节约 1.2 MB 储存空间</span>
              <span class="pv-old-foot-btns">
                <AppButton variant="primary" label="合并" />
                <AppButton variant="panel" label="取消" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            DupScanDialog 屏 1（717 宽组列表，35 稿）｜旧体系待换代｜静态复刻嵌真实 DupScanGroupRow（组复选可交互）；本体读
            dupScanState、合并 = delete_placements；屏 2（973 宽合并前确认双栏）未复刻，换代时照组件补；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="导入未完成" variant="solid" size="md" @click="failedOpen = true" />
          <span class="pv-cap">
            FailedListDialog｜导入/导出失败清单（alert error + 提示音，figma 09）｜旧体系待换代｜真实弹出（props
            直通）；重试/退出仅关闭（预览不接后端）
          </span>
        </div>
      </div>
    </div>

    <FailedListDialog
      v-if="failedOpen"
      title="导入未完成"
      :failed="failedDemo"
      @close="failedOpen = false"
      @retry="failedOpen = false"
    />

    <div class="pv-group">
      <h3 class="pv-group-title">自定义数据（编辑描述 / 属性标签）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w482">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">编辑描述</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body flush">
              <div class="pv-dd-item">
                <div class="pv-dd-line">
                  <IconButton name="chevron-small" small class="pv-arrow-down" title="下移" />
                  <IconButton name="chevron-small" small disabled title="上移" />
                  <span class="pv-dd-label">标题:</span>
                  <AppInput v-model="descRow.title" size="small" class="pv-grow" />
                  <IconButton name="delete-red" delete-red title="删除" />
                </div>
                <div class="pv-dd-line">
                  <span class="pv-ghost" />
                  <span class="pv-ghost" />
                  <span class="pv-dd-label">内容:</span>
                  <AppInput v-model="descRow.content" size="small" class="pv-grow" />
                  <span class="pv-ghost" />
                </div>
              </div>
              <div class="pv-dd-add">
                <AppButton variant="panel" icon="add-large" label="添加描述" />
              </div>
            </div>
            <div class="pv-old-foot">
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            DescEditDialog｜文件/档案描述条目编辑（标题+内容双行、箭头排序，31 稿）｜旧体系待换代｜静态复刻嵌真实
            IconButton/AppInput；本体读 ui.descEdit、确认接后端；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-old-shell w482">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">属性标签</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body">
              <AppSearchBar v-model="propsKeyword" placeholder="搜索..." />
              <div class="pv-chips">
                <TagButton v-for="w in propsOwned" :key="w" :label="w" removable @remove="propsOwned = propsOwned.filter((x) => x !== w)" />
                <input class="pv-tag-new" placeholder="输入新属性" />
                <IconButton name="add-small" small title="添加" />
              </div>
              <div class="pv-pool">
                <span class="pv-pool-label">已有属性标签:</span>
                <div class="pv-chips">
                  <TagButton label="参考" removable />
                  <TagButton label="逐帧" removable />
                  <TagButton label="待整理" removable />
                </div>
              </div>
            </div>
            <div class="pv-old-foot">
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            PropsDialog｜文件/档案属性标签（拥有 chips + 全库属性池，32 稿）｜旧体系待换代｜静态复刻嵌真实
            AppSearchBar/TagButton removable（拥有 chips 可删=本地演示；池 chips 删除不接全局二次确认）；本体 onMounted 调
            listPropPool；口径变更与组件同步
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">分组与区块（新建分组 / 分组管理 / 区块管理）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w603">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">添加分组</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body flush">
              <div class="pv-group-block">
                <div class="pv-dd-line">
                  <span class="pv-dd-label wide">分组名称:</span>
                  <AppInput v-model="geName" size="small" class="pv-grow" placeholder="未命名分组" />
                </div>
                <GroupRulesEditor v-model="geRules" :items="[]" />
              </div>
            </div>
            <div class="pv-old-foot">
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            GroupEditDialog｜新建单个分组（名称 + 规则编辑器，34 稿）｜旧体系待换代｜静态复刻嵌真实
            GroupRulesEditor（items 空桩——候选范围按文件夹实况出，此处只展示已选 chips）；本体读
            content.folders、确认接后端；23 稿错误 alert 未复刻；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-old-shell w603">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">分组管理</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body flush">
              <div class="pv-group-block">
                <div class="pv-dd-line">
                  <IconButton name="chevron-small" small class="pv-arrow-down" title="下移" />
                  <IconButton name="chevron-small" small disabled title="上移" />
                  <span class="pv-dd-label wide">分组名称:</span>
                  <AppInput v-model="gmName" size="small" class="pv-grow" placeholder="未命名分组" />
                  <IconButton name="delete-red" delete-red title="删除分组" />
                </div>
                <GroupRulesEditor v-model="gmRules" :items="[]" />
              </div>
              <div class="pv-dd-add pair">
                <AppButton variant="panel" icon="add-large" label="添加分组" />
                <AppButton variant="panel" icon="add-large" label="添加空组" />
              </div>
            </div>
            <div class="pv-old-foot">
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            GroupManageDialog｜文件夹分组列表管理（组块排序/删除 + 自由组，33 稿）｜旧体系待换代｜静态复刻嵌真实
            GroupRulesEditor（单组示例；自由组说明行与 23 稿错误 alert 未复刻）；本体读 content.folders、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w559">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">区块管理</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body flush">
              <BlockManageRow :row="bmRows[0]" is-first :is-last="false" is-default />
              <BlockManageRow :row="bmRows[1]" :is-first="false" is-last :is-default="false" />
              <div class="pv-dd-add">
                <AppButton variant="panel" icon="add-large" label="添加区块" />
              </div>
            </div>
            <div class="pv-old-foot">
              <AppButton variant="primary" label="确认" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            BlockManageDialog｜区块改名/图标/排序/删除草稿式管理（30 稿）｜旧体系待换代｜静态复刻嵌真实
            BlockManageRow（行可编辑，纯前端）；本体读 content.blocks 快照、确认接后端；26 稿删除警告 alert
            未复刻；口径变更与组件同步
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">其他（批量重命名 / 链接管理 / 路径错误 / 回收站确认）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w973">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">批量重命名</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body flush">
              <div class="pv-old-cols inset">
                <span class="col">原始文件名</span>
                <span class="col">新的文件名</span>
              </div>
              <div class="pv-rd-row">
                <div class="pv-rd-card">
                  <span class="pv-rd-thumb">fla</span>
                  <span class="pv-rd-name">雪牧_自动.fla</span>
                </div>
                <div class="pv-rd-card">
                  <span class="pv-rd-thumb">fla</span>
                  <span class="pv-rd-name">角色01.fla</span>
                </div>
              </div>
              <div class="pv-rd-fields">
                <span class="pv-dd-label wide">名称:</span>
                <AppInput v-model="renameName" size="medium" class="pv-grow" placeholder="输入名称" />
                <span class="pv-dd-label wide">起始编号:</span>
                <AppInput v-model="renameStart" size="medium" class="pv-narrow" placeholder="1" />
              </div>
            </div>
            <div class="pv-old-foot spread">
              <SegmentedControl v-model="renameMode" variant="text" :options="RENAME_MODES" style="width: 160px" />
              <span class="pv-old-foot-btns">
                <AppButton variant="primary" label="确认" />
                <AppButton variant="panel" label="取消" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            BatchRenameDialog｜选中集批量改名双列实时预览（序号/替换两模式，37 稿）｜旧体系待换代｜静态复刻（行卡为壳内复刻，
            segmented/输入框为真实件可交互）；本体读 selection.renameDraft + content.placements、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-old-shell w603">
            <div class="pv-old-titlebar">
              <span class="pv-old-title">链接管理</span>
              <IconButton name="window-close" title="关闭" />
            </div>
            <div class="pv-old-body">
              <AppSearchBar v-model="linkQuery" placeholder="搜索" />
              <div class="pv-old-cols">
                <span class="col">路径</span>
                <span class="col">名称</span>
              </div>
              <div class="pv-lm-line">
                <div class="pv-lm-cell root">
                  <AppIcon name="folder-closed" :size="18" />
                  <span class="pv-lm-fname">01_人物</span>
                  <span class="pv-lm-count">20</span>
                </div>
                <div class="pv-lm-cell link">
                  <span class="pv-lm-fname">雪牧_自动.fla</span>
                  <AppIcon name="original-file-badge" :size="15" />
                  <AppIcon name="check" :size="15" class="pv-lm-check" />
                </div>
              </div>
              <div class="pv-lm-line">
                <div class="pv-lm-cell child">
                  <AppIcon name="folder-closed" :size="18" />
                  <span class="pv-lm-fname">表情</span>
                  <span class="pv-lm-count">8</span>
                </div>
                <div class="pv-lm-cell empty">
                  <AppIcon name="check" :size="15" class="pv-lm-check ghost" />
                </div>
              </div>
            </div>
            <div class="pv-old-foot spread">
              <Checkbox v-model="linkOnlyChecked" label="仅显示已选项" />
              <span class="pv-old-foot-btns">
                <AppButton variant="primary" label="确认" />
                <AppButton variant="panel" label="取消" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            LinkManageDialog｜单文件链接分布管理（左树右链接行 ✓ 增删，17 稿）｜旧体系待换代｜静态复刻（树行/链接行
            = 壳内复刻两行示意，604 高树区从简；搜索框/复选框为真实件）；本体读 selection.linkManageDraft +
            content 树、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <PathErrorDialog
            message="所选路径不可写或不存在：D:/素材库/学习参考"
            :busy="false"
            @retry="() => {}"
            @cancel="() => {}"
          />
          <span class="pv-cap">
            PathErrorDialog｜打开/创建资源库路径校验失败（03_UI 布局 08）｜旧体系待换代（手写卡片 + 手写按钮）｜真实直渲（纯
            props 卡片无遮罩）；重新选择路径/取消仅演示（预览不接后端）
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-old-shell w420 del">
            <div class="pv-old-titlebar slim">
              <span class="pv-old-title">彻底删除</span>
              <IconButton name="window-close" title="取消" />
            </div>
            <div class="pv-del-body">
              将彻底删除选中的 3 个素材，其中素材被 2 个虚拟片段引用，删除后引用将失效。彻底删除不可恢复，确定继续吗？
            </div>
            <div class="pv-del-foot">
              <AppButton variant="primary" label="彻底删除" />
              <AppButton variant="panel" label="取消" />
            </div>
          </div>
          <span class="pv-cap">
            RecycleConfirmDialog｜回收站彻底删除/清空二次确认（含虚拟片段引用警示，6.1 二轮）｜旧体系待换代（del-*
            手写壳与按钮、absolute 遮罩非 AppModal）｜静态复刻；确认 = 彻底删除，预览只陈列绝不真弹；口径变更与组件同步
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* —— 旧弹窗复刻壳（pv-modal-shell 惯例；壳样式只吃 --n- token，口径变更与各组件同步）——
   壳宽 = 各组件稿值（定宽弹窗含真 border 补偿后的外宽）：973 / 717 / 603 / 559 / 482 / 420 */
.pv-old-shell {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
  overflow: hidden;
}
.pv-old-shell.w973 {
  width: 973px;
}
.pv-old-shell.w717 {
  width: 717px;
}
.pv-old-shell.w603 {
  width: 603px;
}
.pv-old-shell.w559 {
  width: 559px;
}
.pv-old-shell.w482 {
  width: 482px;
}
.pv-old-shell.w420 {
  width: 420px;
}
/* 标题栏（standard 54 高通用口径；slim = RecycleConfirmDialog 的 44 高手写标题栏） */
.pv-old-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  height: 54px;
  box-sizing: border-box;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
.pv-old-titlebar.slim {
  height: 44px;
  padding: var(--n-space-8) var(--n-space-8) var(--n-space-8) var(--n-space-16);
}
.pv-old-title {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
/* 内容区（通用 0 24 内缩；flush = 内嵌真实行组件自带 padding 的壳） */
.pv-old-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) var(--n-space-24);
}
.pv-old-body.flush {
  gap: 0;
  padding: 0 var(--n-space-24);
}
/* 底部按钮栏（62 高 / 顶线 / 右对齐主左次右通用口径；spread = 左件 + 右按钮组两端分布） */
.pv-old-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--n-space-12);
  height: 62px;
  box-sizing: border-box;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
.pv-old-foot.spread {
  justify-content: space-between;
}
.pv-old-foot-btns {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.pv-old-foot-label {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-old-summary {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-old-summary b {
  color: var(--n-brand);
}
/* 双列表头（46 高 + 底线，标题档标签色口径） */
.pv-old-cols {
  display: flex;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-12);
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
.pv-old-cols.inset {
  padding: var(--n-space-12) 0;
}
.pv-old-cols .col {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
/* 描述/分组行件（31/33 稿：28 高行 / gap 8；标签 32 宽，wide = 「分组名称:」自适应） */
.pv-dd-item {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
}
.pv-dd-line {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  min-height: 28px;
}
.pv-dd-label {
  flex: none;
  width: 32px;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  white-space: nowrap;
}
.pv-dd-label.wide {
  width: auto;
}
.pv-grow {
  flex: 1;
  min-width: 0;
}
.pv-narrow {
  width: 82px;
  flex: none;
}
.pv-ghost {
  width: 20px;
  height: 20px;
  flex: none;
  visibility: hidden;
}
.pv-arrow-down :deep(svg) {
  transform: rotate(180deg);
}
.pv-dd-add {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
}
/* 属性标签复刻件（拥有 chips 行 / 属性池块） */
.pv-chips {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--n-space-8);
}
.pv-tag-new {
  width: 76px;
  height: 25px;
  box-sizing: border-box;
  padding: var(--n-space-4) var(--n-space-8);
  border: none;
  border-radius: var(--n-radius-6);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
  color: var(--n-l2);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
.pv-tag-new::placeholder {
  color: var(--n-l4);
}
.pv-pool {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g1);
}
.pv-pool-label {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
/* 分组块（组标题行 + 规则编辑器，组块底线口径） */
.pv-group-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
}
/* 批量重命名行卡复刻（37 稿：双卡 gap 24 / 卡 69 高 / 缩略占位 80×45） */
.pv-rd-row {
  display: flex;
  gap: var(--n-space-24);
  padding: var(--n-space-24) 0;
  border-bottom: 1px solid var(--n-g5);
}
.pv-rd-card {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: 1;
  min-width: 0;
  height: 69px;
  box-sizing: border-box;
  padding: var(--n-space-12);
  border-radius: var(--n-radius-8);
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.pv-rd-thumb {
  width: 80px;
  height: 45px;
  border-radius: var(--n-radius-6);
  background: var(--n-g6);
  display: flex;
  align-items: center;
  justify-content: center;
  flex: none;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-g3);
}
.pv-rd-name {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pv-rd-fields {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
}
/* 链接管理行复刻（17 稿：行 32 高 gap 12；文件夹格 270 / 链接格 232） */
.pv-lm-line {
  display: flex;
  gap: var(--n-space-12);
  height: 32px;
}
.pv-lm-cell {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 32px;
  box-sizing: border-box;
  padding: var(--n-space-6) var(--n-space-8);
  border-radius: var(--n-radius-6);
  flex: none;
  color: var(--n-l2);
}
.pv-lm-cell.root {
  width: 270px;
  background: var(--n-g5);
  border: 1px solid var(--n-l-a3);
}
.pv-lm-cell.child {
  width: 270px;
  background: var(--n-g2);
}
.pv-lm-cell.link {
  width: 232px;
  background: var(--n-g4);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
.pv-lm-cell.empty {
  width: 232px;
  justify-content: flex-end;
}
.pv-lm-fname {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pv-lm-count {
  flex: none;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pv-lm-check {
  flex: none;
}
.pv-lm-check.ghost {
  color: var(--n-l4);
}
/* RecycleConfirmDialog 复刻件（del-* 手写壳：正文 16 32 / 底栏 70 高） */
.pv-del-body {
  padding: var(--n-space-16) var(--n-space-32, 32px);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.pv-del-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--n-space-16, 16px);
  height: 70px;
  box-sizing: border-box;
  padding: 0 var(--n-space-32, 32px);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
</style>
