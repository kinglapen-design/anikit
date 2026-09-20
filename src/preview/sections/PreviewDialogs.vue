<script setup lang="ts">
// 业务弹窗展区（阶段七换代盘点；2026-09-20 R2 旧件烧减轮后同步，2026-09-21 跟进）：
// src/components/ 根目录 12 件业务弹窗全量收编 + 附属 OrphanInfosAlert。
// 现状分三档：
//   ① 本体已换代 modals/BaseModal framed（9 件）：BatchRenameDialog 973 / DuplicateDialog 970 /
//     DupScanDialog 714·970 双屏 / GroupEditDialog 603 / GroupManageDialog 603 / LinkManageDialog 602 /
//     BlockManageDialog 559 / DescEditDialog 482 / PropsDialog 481——依赖 store 数据或挂载即 invoke，
//     按 pv-modal-shell 惯例静态复刻：壳 = pv-bm-shell（g3 底 + 1px g5 真 border + 大阴影 + 圆角 8 +
//     壳 padding 0 + 54 高标题栏 标题 18/26 Bold 一级 + buttons/IconButton plain close-large），
//     嵌件 = 新体系真实件（ActionButton / TextInput / SegmentedControl / SwitchButton / Checkbox /
//     TagButton / buttons/IconButton（红删已改 clear X bare sm 同本体）），口径变更与组件同步。
//     嵌套提示（23 稿保存冲突 danger / 26 稿删除区块 warning）本体已换 modals/ConfirmModal——
//     展区给两枚真实弹出（确认/取消仅关闭，预览不接后端）。
//   ② 仍旧 AppModal alert 待稿（2 件）：FailedListDialog（真实弹出，props 直通，重试/退出仅关闭）/
//     OrphanInfosAlert（静态复刻旧口径）——无新稿，待用户定。
//   ③ 手写壳（2 件）：PathErrorDialog 纯 props 卡片真实直渲（仍旧手写壳+手写按钮）；
//     RecycleConfirmDialog 静态复刻（手写 del-* 壳与手写按钮仍旧体系，仅关闭 X 已换 buttons/IconButton；
//     确认 = 彻底删除，只陈列绝不真弹）。
// 复刻壳宽度按各组件稿值，高度按演示内容从简（本体多高随内容/视口）。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import TextInput from "../../components/inputs/TextInput.vue";
import SegmentedControl from "../../components/buttons/SegmentedControl.vue";
import SwitchButton from "../../components/buttons/SwitchButton.vue";
import Checkbox from "../../components/buttons/Checkbox.vue";
import TagButton from "../../components/buttons/TagButton.vue";
import ConfirmModal from "../../components/modals/ConfirmModal.vue";
import AppIcon from "../../components/AppIcon.vue";
import AppButton from "../../components/AppButton.vue"; // 仅 OrphanInfosAlert 旧口径复刻用（该件仍旧 AppModal alert 待稿）
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

// —— FailedListDialog 真实弹出（仍旧 AppModal alert 待稿；props 直通；重试/退出仅关闭演示）——
const failedOpen = ref(false);
const failedDemo = [
  { path: "C:/演示/素材/走路循环_破损.fla", reason: "文件损坏" },
  { path: "C:/演示/素材/表情集_2026.psd", reason: "格式不支持" },
  { path: "", reason: "磁盘读取失败" },
];

// —— 嵌套提示真实弹出（本体已换 ConfirmModal；确认/取消仅关闭，预览不接后端）——
const groupErrOpen = ref(false); // 23 稿保存冲突 danger（GroupEdit / GroupManage 共用）
const blockWarnOpen = ref(false); // 26 稿删除区块警告 warning（BlockManage）

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
// —— GroupManageDialog 复刻桩（R4 稿组卡片；添加/排序接本地草稿——顺势演示稿点名的两动画：
//    新卡片入场 = 淡入 + 24 上移 250ms，排序交换 = 150ms，均纯前端）——
interface GmCard {
  key: string;
  name: string;
  free: boolean;
  rules: GroupRule[];
}
let gmSeq = 0;
const gmCards = ref<GmCard[]>([
  { key: "g-rule", name: "表情", free: false, rules: [{ kind: "ext", values: ["gif", "webp"] }] },
  { key: "g-free", name: "待归类", free: true, rules: [] },
]);
function gmMove(index: number, dir: -1 | 1): void {
  const rows = gmCards.value;
  const j = index + dir;
  if (j < 0 || j >= rows.length) return;
  const [row] = rows.splice(index, 1);
  rows.splice(j, 0, row);
}
function gmAdd(free: boolean): void {
  gmCards.value.push({ key: `g-new-${++gmSeq}`, name: "", free, rules: free ? [] : [{ kind: "ext", values: [] }] });
}

// —— BlockManageDialog 复刻桩（嵌真实 BlockManageRow；行编辑/排序钮为本地演示）——
const bmRows = ref<BlockDraft[]>([
  { key: "b-dir", id: "dir", name: "目录", icon: null },
  { key: "b-ref", id: "b-ref", name: "学习参考", icon: null },
]);

// —— BatchRenameDialog 复刻桩（真实 SegmentedControl/TextInput；行卡 = 壳内复刻）——
const renameMode = ref("sequence");
const RENAME_MODES = [
  { value: "sequence", label: "序号命名" },
  { value: "replace", label: "替换命名" },
] as const;
const renameName = ref("角色");
const renameStart = ref("1");

// —— DescEditDialog 复刻桩（真实 buttons/IconButton + TextInput，删除 = clear X bare sm）——
const descRow = ref({ title: "出处", content: "雪牧老师公开素材" });

// —— PropsDialog 复刻桩（真实 TextInput/TagButton；chips 增删为本地演示，不接全局删除确认）——
const propsKeyword = ref("");
const propsOwned = ref(["雪牧", "走路循环"]);
const propsPool = ref(["参考", "逐帧", "待整理"]);

// —— LinkManageDialog 复刻桩（真实 TextInput/Checkbox/AppIcon；树行 = 壳内复刻）——
const linkQuery = ref("");
const linkOnlyChecked = ref(false);
</script>

<template>
  <section id="design-system-dialogs" class="pv-section">
    <h2 class="pv-sec-title">业务弹窗（阶段七换代盘点）</h2>
    <p class="pv-sec-desc">
      src/components/ 根目录 12 件业务弹窗 + 附属 OrphanInfosAlert。<b>9 件本体已换代 modals/BaseModal framed</b>（宽走
      prop 稿值：973 / 970 / 714·970 / 603 / 602 / 559 / 482 / 481），此区复刻壳 = pv-bm-shell 与新壳同步（g3 底 + 1px
      g5 真 border + 大阴影 + 圆角 8 + 壳 padding 0 + 54 高标题栏），嵌件为新体系真实件，口径变更与组件同步；嵌套提示已换
      ConfirmModal（两枚真实弹出，预览不接后端）。<b>未换代三件</b>：FailedListDialog / OrphanInfosAlert 仍旧 AppModal
      alert（无新稿，待用户定），PathErrorDialog / RecycleConfirmDialog 仍旧手写壳（后者仅关闭 X 已换 buttons/IconButton）。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">导入链路（重复提示 / 扫描合并 / 失败清单 / 无主资产警告）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w970">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">重复文件提示</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body">
              <div class="pv-bm-cols">
                <span class="col">已存在文件</span>
                <span class="col">新的文件</span>
              </div>
              <DuplicateRow
                :item="dupConflict"
                :decision="dupDecision"
                @change="dupDecision = { ...dupDecision, ...$event }"
              />
            </div>
            <div class="pv-bm-foot">
              <span class="pv-bm-foot-label">全部跟随</span>
              <SwitchButton v-model="dupFollowAll" />
              <ActionButton variant="cancel" label="取消" size="lg" />
              <ActionButton variant="primary" label="确认" size="lg" />
            </div>
          </div>
          <span class="pv-cap">
            DuplicateDialog｜导入 hash 冲突逐行决策（radio 三选一 + 跳过，36 稿）｜本体已换代 BaseModal framed 970 +
            SwitchButton/ActionButton；复刻嵌真实 DuplicateRow（行可交互）；本体读 content.importConflicts、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w714">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">扫描重复文件</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <DupScanGroupRow :group="scanGroup" />
            <div class="pv-bm-foot spread">
              <span class="pv-bm-summary">已选择 <b>2</b> 个文件 · 可节约 1.2 MB 储存空间</span>
              <span class="pv-bm-foot-btns">
                <ActionButton variant="cancel" label="取消" size="lg" />
                <ActionButton variant="primary" label="合并" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            DupScanDialog 屏 1（714 宽组列表，35 稿）｜本体已换代 BaseModal framed（屏 1 = 714 / 屏 2 = 970，宽度过渡 200ms 沿）；
            复刻嵌真实 DupScanGroupRow（组复选可交互）；本体读 dupScanState、合并 = delete_placements；屏 2（970
            宽合并前确认双栏）未复刻；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="导入未完成" variant="solid" size="md" @click="failedOpen = true" />
          <span class="pv-cap">
            FailedListDialog｜导入/导出失败清单（figma 09 侵入式）｜<b>仍旧 AppModal alert + AppButton，无新稿待用户定</b>｜真实弹出（props
            直通，alert 提示音随壳播）；重试/退出仅关闭（预览不接后端）
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-old-shell">
            <div class="pv-old-alert">
              <AppIcon name="warning" :size="48" class="pv-old-badge" />
              <div class="pv-old-alert-text">
                <span class="pv-old-alert-title">发现 2 个无主资产文件</span>
                <p class="pv-old-alert-desc">
                  以下文件残留在素材库目录中，但数据层没有对应记录（可能是删除中断的残留），不会显示在库内：
                </p>
                <div class="pv-old-orphan-list">
                  <div class="pv-old-orphan-row">
                    <span class="name">雪牧_自动.fla</span>
                    <span class="path">素材库/info/a1b2c3/info.json</span>
                  </div>
                  <div class="pv-old-orphan-row">
                    <span class="name">走路循环_01.png</span>
                    <span class="path">素材库/info/d4e5f6/info.json</span>
                  </div>
                </div>
              </div>
            </div>
            <div class="pv-old-alert-foot">
              <AppButton variant="primary" label="知道了" />
            </div>
          </div>
          <span class="pv-cap">
            OrphanInfosAlert｜开库自检孤儿 info 警告（2026-09-02 资产污染修复配套）｜<b>仍旧 AppModal alert +
            AppButton，无新稿待用户定</b>（确认钮本体为警告色底，复刻从简）；静态复刻旧口径；本体读
            orphanInfosState，「知道了」仅关闭
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
          <div class="pv-bm-shell w482">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">编辑描述</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body flush">
              <div class="pv-dd-item">
                <div class="pv-dd-line">
                  <IconButton name="chevron-small" variant="bare" size="sm" class="pv-arrow-down" title="下移" />
                  <IconButton name="chevron-small" variant="bare" size="sm" disabled title="上移" />
                  <span class="pv-dd-label">标题:</span>
                  <TextInput v-model="descRow.title" size="sm" class="pv-grow" />
                  <IconButton name="clear" variant="bare" size="sm" title="删除" />
                </div>
                <div class="pv-dd-line">
                  <span class="pv-ghost sm" />
                  <span class="pv-ghost sm" />
                  <span class="pv-dd-label">内容:</span>
                  <TextInput v-model="descRow.content" size="sm" class="pv-grow" />
                  <span class="pv-ghost" />
                </div>
              </div>
              <div class="pv-dd-add">
                <ActionButton variant="solid" size="md" icon="add-large" label="添加描述" />
              </div>
            </div>
            <div class="pv-bm-foot">
              <ActionButton variant="cancel" label="取消" size="lg" />
              <ActionButton variant="primary" label="确认" size="lg" />
            </div>
          </div>
          <span class="pv-cap">
            DescEditDialog｜文件/档案描述条目编辑（标题+内容双行、箭头排序，31 稿）｜本体已换代 BaseModal framed 482 +
            TextInput sm + ActionButton（删除 = clear X bare sm，2026-09-20 用户定红删改 X）；复刻嵌真实件；本体读
            ui.descEdit、确认接后端；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w481">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">属性标签</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body">
              <TextInput v-model="propsKeyword" size="lg" leading-icon="search" clearable placeholder="搜索..." />
              <div class="pv-chips">
                <TagButton
                  v-for="w in propsOwned"
                  :key="w"
                  :label="w"
                  removable
                  @remove="propsOwned = propsOwned.filter((x) => x !== w)"
                />
                <input class="pv-tag-new" placeholder="输入新属性" />
                <IconButton name="add-small" variant="bare" size="sm" title="添加" />
              </div>
              <div class="pv-pool">
                <span class="pv-pool-label">已有属性标签:</span>
                <div class="pv-chips">
                  <TagButton
                    v-for="w in propsPool"
                    :key="w"
                    :label="w"
                    removable
                    @remove="propsPool = propsPool.filter((x) => x !== w)"
                  />
                </div>
              </div>
            </div>
            <div class="pv-bm-foot">
              <ActionButton variant="cancel" label="取消" size="lg" />
              <ActionButton variant="primary" label="确认" size="lg" />
            </div>
          </div>
          <span class="pv-cap">
            PropsDialog｜文件/档案属性标签（拥有 chips + 全库属性池，32 稿）｜本体已换代 BaseModal framed 481 +
            TextInput/TagButton/buttons IconButton（输入新属性小框本体仍手写 input）；复刻 chips 增删为本地演示——本体池内
            × = 全局删除 ConfirmModal 二次确认，复刻不接；本体 onMounted 调 listPropPool；口径变更与组件同步
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">分组与区块（新建分组 / 分组管理 / 区块管理 + 嵌套提示真实弹出）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w603">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">添加分组</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body flush">
              <div class="pv-group-block">
                <div class="pv-dd-line">
                  <span class="pv-dd-label wide">分组名称:</span>
                  <TextInput v-model="geName" size="sm" class="pv-grow" placeholder="未命名分组" />
                </div>
                <GroupRulesEditor v-model="geRules" :items="[]" />
              </div>
            </div>
            <div class="pv-bm-foot">
              <ActionButton variant="cancel" label="取消" size="lg" />
              <ActionButton variant="primary" label="确认" size="lg" />
            </div>
          </div>
          <span class="pv-cap">
            GroupEditDialog｜新建单个分组（名称 + 规则编辑器，34 稿）｜本体已换代 BaseModal framed 603 +
            TextInput/ActionButton；复刻嵌真实 GroupRulesEditor（items 空桩——候选范围按文件夹实况出，此处只展示已选
            chips）；本体读 content.folders、确认接后端；23 稿保存冲突提示已换 ConfirmModal danger（右下真实弹出）；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w603">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">分组管理</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-gm-list">
              <TransitionGroup name="pv-gm-swap" tag="div" class="pv-gm-cards">
                <div v-for="(g, i) in gmCards" :key="g.key" class="pv-gm-card">
                  <div class="pv-gm-title">
                    <IconButton variant="bare" size="sm" name="chevron-small" class="pv-gm-arrow" :disabled="i === 0" title="上移" @click="gmMove(i, -1)" />
                    <IconButton variant="bare" size="sm" name="chevron-small" class="pv-gm-arrow pv-arrow-down" :disabled="i === gmCards.length - 1" title="下移" @click="gmMove(i, 1)" />
                    <TextInput v-model="g.name" size="sm" class="pv-gm-name" placeholder="未命名分组" />
                    <IconButton name="clear" variant="bare" size="sm" title="删除分组" @click="gmCards.splice(i, 1)" />
                  </div>
                  <div class="pv-gm-content">
                    <div v-if="g.free" class="pv-free-hint">无规则（手动归组）——将文件拖入分组，或右键「移动至分组」</div>
                    <GroupRulesEditor v-else v-model="g.rules" :items="[]" />
                  </div>
                </div>
              </TransitionGroup>
            </div>
            <div class="pv-bm-foot bare spread">
              <span class="pv-bm-foot-btns">
                <ActionButton variant="cancel" size="lg" label="添加分组" @click="gmAdd(false)" />
                <ActionButton variant="cancel" size="lg" label="添加空组" @click="gmAdd(true)" />
              </span>
              <span class="pv-bm-foot-btns">
                <ActionButton variant="cancel" label="取消" size="lg" />
                <ActionButton variant="primary" label="确认" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            GroupManageDialog｜文件夹分组列表管理｜本体已按 02_样式标注稿换代（R4：组卡片 g6 + 立体感 / 标题行 42 高 +
            底线 亮色/禁用 / 箭头序 上移→下移 / 名称框 30 高 g4 底，「分组名称:」标签与红删废——删除 = clear X / 底栏左右组
            无顶线 / 列表区 714 高；壳 603 沿 R2，稿 640 未随）；复刻壳同步，稿点名两动画可演示——点底部「添加分组/添加空组」
            看新卡片入场（淡入 + 24 上移 250ms），点卡片箭头看排序交换（150ms）；规则编辑器嵌真实件（items 空桩只展示已选
            chips）；确认/取消仅陈列（预览不接后端）；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w559">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">区块管理</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body flush">
              <BlockManageRow :row="bmRows[0]" is-first :is-last="false" is-default />
              <BlockManageRow :row="bmRows[1]" :is-first="false" is-last :is-default="false" />
              <div class="pv-dd-add">
                <ActionButton variant="solid" size="md" icon="add-large" label="添加区块" />
              </div>
            </div>
            <div class="pv-bm-foot">
              <ActionButton variant="cancel" label="取消" size="lg" />
              <ActionButton variant="primary" label="确认" size="lg" />
            </div>
          </div>
          <span class="pv-cap">
            BlockManageDialog｜区块改名/图标/排序/删除草稿式管理（30 稿）｜本体已换代 BaseModal framed 559；复刻嵌真实
            BlockManageRow 两行（行可编辑，纯前端；图标下拉接真实菜单服务，演示页勿点）；本体读 content.blocks
            快照、确认接后端；26 稿删除区块警告已换 ConfirmModal warning（右下真实弹出）；口径变更与组件同步
          </span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="保存冲突提示" variant="solid" size="md" @click="groupErrOpen = true" />
          <span class="pv-cap">
            23 稿嵌套提示（GroupEdit / GroupManage 保存冲突共用）｜本体已换 ConfirmModal danger + 提示音（提示音由宿主
            watch 播，预览不播）｜真实弹出：「修改分组」/「取消」仅关闭（预览不接后端）
          </span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="删除区块警告" variant="solid" size="md" @click="blockWarnOpen = true" />
          <span class="pv-cap">
            26 稿嵌套警告（BlockManage 删除非空区块）｜本体已换 ConfirmModal warning｜真实弹出：「删除」/「取消」仅关闭（预览不接后端）
          </span>
        </div>
      </div>
    </div>

    <ConfirmModal
      v-if="groupErrOpen"
      kind="danger"
      title="无法保存，存在完全相同的分组！"
      confirm-label="修改分组"
      @confirm="groupErrOpen = false"
      @cancel="groupErrOpen = false"
    >
      不能出现两个或多个一模一样的分组，请修改规则和范围，区分不同分组的功能。
    </ConfirmModal>
    <ConfirmModal
      v-if="blockWarnOpen"
      kind="warning"
      title="删除区块"
      confirm-label="删除"
      @confirm="blockWarnOpen = false"
      @cancel="blockWarnOpen = false"
    >
      区块「学习参考」内还有 3 个文件夹。删除后文件夹将移入文件夹区块。
    </ConfirmModal>

    <div class="pv-group">
      <h3 class="pv-group-title">其他（批量重命名 / 链接管理 / 路径错误 / 回收站确认）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w973">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">批量重命名</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body flush">
              <div class="pv-bm-cols inset">
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
                <TextInput v-model="renameName" size="lg" clearable class="pv-grow" placeholder="输入名称" />
                <span class="pv-dd-label wide">起始编号:</span>
                <TextInput v-model="renameStart" size="lg" clearable class="pv-narrow" placeholder="1" />
              </div>
            </div>
            <div class="pv-bm-foot spread">
              <SegmentedControl v-model="renameMode" :options="RENAME_MODES" variant="text" style="width: 160px" />
              <span class="pv-bm-foot-btns">
                <ActionButton variant="cancel" label="取消" size="lg" />
                <ActionButton variant="primary" label="确认" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            BatchRenameDialog｜选中集批量改名双列实时预览（序号/替换两模式，37 稿）｜本体已换代 BaseModal framed 973 +
            TextInput lg + SegmentedControl text；复刻行卡为壳内复刻、输入框与 segmented 为真实件可交互；本体读
            selection.renameDraft + content.placements、确认接后端；口径变更与组件同步
          </span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-bm-shell w602">
            <div class="pv-bm-titlebar">
              <span class="pv-bm-title">链接管理</span>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-bm-body">
              <TextInput v-model="linkQuery" size="lg" leading-icon="search" clearable placeholder="搜索" />
              <div class="pv-bm-cols">
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
            <div class="pv-bm-foot spread">
              <Checkbox v-model="linkOnlyChecked" label="仅显示已选项" />
              <span class="pv-bm-foot-btns">
                <ActionButton variant="cancel" label="取消" size="lg" />
                <ActionButton variant="primary" label="确认" size="lg" />
              </span>
            </div>
          </div>
          <span class="pv-cap">
            LinkManageDialog｜单文件链接分布管理（左树右链接行 ✓ 增删，17 稿）｜本体已换代 BaseModal framed 602 +
            TextInput search + buttons Checkbox；复刻树行/链接行 = 壳内复刻两行示意（604 高树区从简），搜索框/复选框为真实件；本体读
            selection.linkManageDraft + content 树、确认接后端；口径变更与组件同步
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
            PathErrorDialog｜打开/创建资源库路径校验失败（03_UI 布局 08）｜<b>仍旧手写壳 + 手写按钮，未换代待稿</b>｜真实直渲（纯
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
              <span class="pv-del-btn primary">彻底删除</span>
              <span class="pv-del-btn">取消</span>
            </div>
          </div>
          <span class="pv-cap">
            RecycleConfirmDialog｜回收站彻底删除/清空二次确认（含虚拟片段引用警示，6.1 二轮）｜<b>手写 del-*
            壳与手写按钮仍旧体系待稿</b>（absolute 遮罩非 BaseModal；仅关闭 X 已换 buttons/IconButton plain——复刻为真实件）；静态复刻，确认 =
            彻底删除，预览只陈列绝不真弹；口径变更与组件同步
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* —— BaseModal framed 复刻壳（pv-modal-shell 惯例；壳样式只吃 --n- token，口径变更与 modals/BaseModal 同步）——
   framed 口径：g3 底 + 1px g5 真 border + 大阴影 + 圆角 8 + 壳 padding 0；壳宽 = 各组件 width prop 稿值
   （border-box 含 1px 边）：973 / 970 / 714 / 603 / 602 / 559 / 482 / 481 */
.pv-bm-shell {
  display: flex;
  flex-direction: column;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
  overflow: hidden;
}
.pv-bm-shell.w973 {
  width: 973px;
}
.pv-bm-shell.w970 {
  width: 970px;
}
.pv-bm-shell.w714 {
  width: 714px;
}
.pv-bm-shell.w603 {
  width: 603px;
}
.pv-bm-shell.w602 {
  width: 602px;
}
.pv-bm-shell.w559 {
  width: 559px;
}
.pv-bm-shell.w482 {
  width: 482px;
}
.pv-bm-shell.w481 {
  width: 481px;
}
/* 模态窗按钮罩层（与 BaseModal .bm-modal :deep(.act-btn) 同步）：宽自适应内容 + 左右 48px，
   2026-09-13 实机走查用户定稿值——壳内全部 ActionButton（含 solid md 添加钮）通吃 */
.pv-bm-shell :deep(.act-btn) {
  width: auto;
  padding: var(--n-space-8) 48px;
  flex: none;
}
/* framed 标题栏（54 高 / padding 12 12 12 24 / 底 1px g5 通栏；标题 18/26 Bold 一级 + plain close-large） */
.pv-bm-titlebar {
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
.pv-bm-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
/* 内容区（framed 壳 padding 0，各段自承 0 24 内缩；flush = 内嵌行组件/行件自带 padding 的壳） */
.pv-bm-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) var(--n-space-24);
}
.pv-bm-body.flush {
  gap: 0;
  padding: 0 var(--n-space-24);
}
/* 底部按钮栏（62 高 / 顶线通栏 / padding 12 12 12 24 / 右对齐次左主右；spread = 左件 + 右按钮组两端分布） */
.pv-bm-foot {
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
.pv-bm-foot.spread {
  justify-content: space-between;
}
/* bare 变体（GroupManageDialog R4 稿「按钮」：62 高 / padding 12 四边 / 无顶线；内组 gap 8） */
.pv-bm-foot.bare {
  border-top: none;
  padding: var(--n-space-12);
}
.pv-bm-foot.bare .pv-bm-foot-btns {
  gap: var(--n-space-8);
}
.pv-bm-foot-btns {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.pv-bm-foot-label {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-bm-summary {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-bm-summary b {
  color: var(--n-brand);
}
/* 双列表头（46 高节奏 + 底线 g5；标题档标签色） */
.pv-bm-cols {
  display: flex;
  gap: var(--n-space-24);
  padding-bottom: var(--n-space-12);
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
.pv-bm-cols.inset {
  padding: var(--n-space-12) 0;
}
.pv-bm-cols .col {
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
.pv-ghost.sm {
  width: 14px;
  height: 14px;
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
/* 属性标签复刻件（拥有 chips 行 / 属性池块；输入新属性小框 = 本体手写 input 同款从简） */
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
/* 分组块（组标题行 + 规则编辑器，组块底线口径；末块去线防与添加区叠色） */
.pv-group-block {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  padding: var(--n-space-12) 0;
  border-bottom: 1px solid var(--n-g5);
}
.pv-group-block:last-child {
  border-bottom: none;
}
/* 自由组静态说明行（2026-09-17 无规则空组，稿外件；与本体 .free-hint 同步：g3 底 + inset 1px g5） */
.pv-free-hint {
  display: flex;
  align-items: center;
  min-height: 44px;
  box-sizing: border-box;
  padding: var(--n-space-8) var(--n-space-12);
  border-radius: var(--n-radius-6);
  background: var(--n-g3);
  box-shadow: inset 0 0 0 1px var(--n-g5);
  color: var(--n-l3);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  flex: none;
}
/* —— GroupManageDialog R4 复刻件（02_样式标注稿，与 GroupManageDialog.vue 同步）——
   组卡片：g6 底 + 立体感-通用 + 圆角 8 + padding 12，卡片间 gap 12（稿组间无线） */
.pv-gm-list {
  height: 714px; /* 稿：714 = 窗 830 - 标题栏 54 - 底栏 62 */
  box-sizing: border-box;
  padding: var(--n-space-12) var(--n-space-24);
}
.pv-gm-cards {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.pv-gm-card {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-12);
  border-radius: var(--n-radius-8);
  background: var(--n-g6);
  box-shadow: var(--n-effect-raised);
}
/* 组标题行（稿「输入分组名称」：gap 8 / padding-bottom 12 / 底线 亮色/禁用；行高 42 由 30 输入框撑起） */
.pv-gm-title {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding-bottom: var(--n-space-12);
  border-bottom: 1px solid var(--n-l-disabled);
}
/* 排序箭头（稿：可用 = 亮色/二级，提权压组件自带 .ic-btn.bare；禁用 = 控件族 30% 透明度） */
.pv-gm-title .ic-btn.pv-gm-arrow {
  color: var(--n-l2);
}
.pv-gm-name {
  flex: 1;
  min-width: 0;
}
/* 名称输入框衍生（稿：30 高 / padding 6 12 / 通用/四级底 / 左对齐 / 文字亮色/一级——提权压 .ti-root.sm） */
.pv-gm-title .ti-root.pv-gm-name {
  height: 30px;
  padding: var(--n-space-6) var(--n-space-12);
  background: var(--n-g4);
}
.pv-gm-name :deep(.ti-inner) {
  text-align: left;
  color: var(--n-l1);
}
/* 规则区（稿「规则列表」padding-top 12 自承） */
.pv-gm-content {
  padding-top: var(--n-space-12);
}
/* 稿点名两动画：排序交换 150 通用档；新建卡片入场 = 淡入 + 24 上移 250 小位移档 */
.pv-gm-swap-move {
  transition: transform var(--n-dur-150) var(--n-ease-in-out);
}
.pv-gm-swap-enter-from {
  opacity: 0;
  transform: translateY(var(--n-space-24));
}
.pv-gm-swap-enter-active {
  transition:
    opacity var(--n-dur-250) var(--n-ease-in-out),
    transform var(--n-dur-250) var(--n-ease-in-out);
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

/* —— 旧体系遗留复刻壳（未换代件专用：OrphanInfosAlert 旧 AppModal alert / RecycleConfirmDialog 手写
   del-* 壳；壳样式吃 --n- token 近似旧观感，换代时整体替换）—— */
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
.pv-old-shell.w420 {
  width: 420px;
}
/* AppModal alert 形态（figma 09）：padding 24 / 内容行横排 48 徽章 + 标题/描述竖排；按钮右对齐 */
.pv-old-alert {
  display: flex;
  align-items: flex-start;
  gap: var(--n-space-12);
  padding: var(--n-space-24) var(--n-space-24) 0;
}
.pv-old-badge {
  flex: none;
}
.pv-old-alert-text {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
  min-width: 0;
}
.pv-old-alert-title {
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-old-alert-desc {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pv-old-orphan-list {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
.pv-old-orphan-row {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l2);
}
.pv-old-orphan-row .path {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  overflow-wrap: anywhere;
}
.pv-old-alert-foot {
  display: flex;
  justify-content: flex-end;
  padding: var(--n-space-24);
}
/* RecycleConfirmDialog 手写壳件（del-*：标题栏 44 / 正文 16 32 / 底栏 70；手写按钮以静态件近似） */
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
.pv-del-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  padding: var(--n-space-8) var(--n-space-16);
  height: 38px;
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  background: var(--n-g5);
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
.pv-del-btn.primary {
  background: var(--n-brand);
  color: var(--n-l1);
}
</style>
