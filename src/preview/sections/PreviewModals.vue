<script setup lang="ts">
// 模态窗口展区——docs/06_UI重构/Design Token/组件/模态窗口/弹窗.md（警告与错误，2026-09-01）。
// ConfirmModal 是 fixed 遮罩层不便静态展示：.pv-modal-shell 复刻窗口口径（口径变更时与组件同步），
// 末行真实弹出实例验收遮罩（黑色-50%）与弹出动画（缩放+淡入 150ms）。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import ConfirmModal from "../../components/modals/ConfirmModal.vue";
import CreateLibraryModal from "../../components/CreateLibraryModal.vue";
import MergeLibraryModal from "../../components/MergeLibraryModal.vue";
import ImportDialog from "../../components/ImportDialog.vue";
import SequenceDialog from "../../components/SequenceDialog.vue";
import TextInput from "../../components/inputs/TextInput.vue";
import SectionTitle from "../../components/items/SectionTitle.vue";
import TreeRowItem from "../../components/items/TreeRowItem.vue";
import type { ImportDraft } from "../../stores/content";

const demoOpen = ref(false);
const createOpen = ref(false);
const mergeOpen = ref(false);
const importFilesOpen = ref(false);
const importFolderOpen = ref(false);
const seqOpen = ref(false);
const mockLibItems = [
  { name: "沙雕动画资源库", sizeBytes: 320 * 1024 ** 3 },
  { name: "学习参考", sizeBytes: 12.6 * 1024 ** 3 },
  { name: "项目素材备份", sizeBytes: 860 * 1024 ** 2 },
];
const draftFiles: ImportDraft = { kind: "files", paths: ["C:/演示/主角-走01.png"], targetFolder: null };
const draftFolder: ImportDraft = { kind: "folder", paths: ["C:/演示/角色素材"], targetFolder: null };
</script>

<template>
  <section id="design-system-modals" class="pv-section">
    <h2 class="pv-sec-title">组件 · 模态窗口</h2>
    <p class="pv-sec-desc">
      Design Token/组件/模态窗口/弹窗.md（2026-09-01）：窗口 520 宽 / padding 24 / 列 gap 24，通用/三级底 + 1px
      通用/五级真 border + 大阴影 + 圆角 8；标题 18/26 Bold 亮色/一级，正文 13/18 亮色/三级；按钮高 38 宽自适应内容
      + 左右 48 次左主右（2026-09-13 走查定稿值，废 120 定宽）——取消 通用/五级底 + 立体感-通用，确认 kind 色底 +
      立体感-主要按钮。遮罩 黑色-50%，点击不关闭。单钮形态（2026-09-21 用户定）= cancelLabel 空串隐藏取消钮。
      壳为预览复刻；按钮 hover 态稿未给，待补稿。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">确认弹窗（warning / danger / brand 三档确认钮）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-modal-shell">
            <h2 class="pv-modal-title">该区块内有文件存在，删除后所有文件将会剔除层级回退至未分类！</h2>
            <p class="pv-modal-body">推荐将区块内的文件夹移动至其他区块后删除。</p>
            <div class="pv-modal-actions">
              <button class="pv-modal-btn cancel">取消</button>
              <button class="pv-modal-btn confirm warning">确认</button>
            </div>
          </div>
          <span class="pv-cap">warning（状态/警告）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-modal-shell">
            <h2 class="pv-modal-title">无法保存，存在完全相同的分组！</h2>
            <p class="pv-modal-body">不能出现两个或多个一模一样的分组，请修改规则和范围，区分不同分组的功能。</p>
            <div class="pv-modal-actions">
              <button class="pv-modal-btn cancel">取消</button>
              <button class="pv-modal-btn confirm danger">确认</button>
            </div>
          </div>
          <span class="pv-cap">danger（状态/禁止）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-modal-shell">
            <h2 class="pv-modal-title">需要将文件移动到回收站吗？</h2>
            <p class="pv-modal-body">不危险的提示，确认钮换 品牌/常态。</p>
            <div class="pv-modal-actions">
              <button class="pv-modal-btn cancel">取消</button>
              <button class="pv-modal-btn confirm brand">确认</button>
            </div>
          </div>
          <span class="pv-cap">brand（品牌/常态）</span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-modal-shell">
            <h2 class="pv-modal-title">发现 2 个无主资产文件</h2>
            <p class="pv-modal-body">单钮形态（cancelLabel 空）——孤儿警告「知道了」。</p>
            <div class="pv-modal-actions">
              <button class="pv-modal-btn confirm warning">知道了</button>
            </div>
          </div>
          <span class="pv-cap">单钮（warning 仅确认，2026-09-21）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="打开交互演示" variant="solid" size="md" @click="demoOpen = true" />
          <span class="pv-cap">真实弹出：遮罩 黑色-50% + 弹出动画（点取消/确认关闭）</span>
        </div>
      </div>
    </div>

    <ConfirmModal
      v-if="demoOpen"
      title="该区块内有文件存在，删除后所有文件将会剔除层级回退至未分类！"
      kind="warning"
      @confirm="demoOpen = false"
      @cancel="demoOpen = false"
    >
      推荐将区块内的文件夹移动至其他区块后删除。
    </ConfirmModal>

    <div class="pv-group">
      <h3 class="pv-group-title">功能模态窗（BaseModal 共享壳，布局/模态窗口 四稿：创建 400 / 合并 498 / 导入·序列帧 312 / 移动 460 framed；按钮宽自适应内容 + 左右 48px 次左主右）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="创建资源库" variant="solid" size="md" @click="createOpen = true" />
          <span class="pv-cap">真实弹出：TextInput lg 自动聚焦；确认 = 真建库（接后端，预览环境请勿点确认）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="合并资源库" variant="solid" size="md" @click="mergeOpen = true" />
          <span class="pv-cap">真实弹出：行对勾切换可验；确认钮未接动作（合并后端排期）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="导入文件（单组）" variant="solid" size="md" @click="importFilesOpen = true" />
          <span class="pv-cap">真实弹出：312 宽 / Radio 单选 / 确认·取消仅关闭（预览不接后端）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="导入文件夹（双组）" variant="solid" size="md" @click="importFolderOpen = true" />
          <span class="pv-cap">双组：层级组顶分隔线 1px 亮色/半透明-3 + 上下各 12（稿值）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="检测到序列帧" variant="solid" size="md" @click="seqOpen = true" />
          <span class="pv-cap">只问打包/分散——稿内来源组 = 复制残留（2026-08-12 用户定，不重复问）</span>
        </div>
      </div>
    </div>
    <CreateLibraryModal v-if="createOpen" @close="createOpen = false" />
    <MergeLibraryModal v-if="mergeOpen" :items="mockLibItems" @close="mergeOpen = false" />
    <ImportDialog
      v-if="importFilesOpen"
      :draft="draftFiles"
      @confirm="importFilesOpen = false"
      @cancel="importFilesOpen = false"
    />
    <ImportDialog
      v-if="importFolderOpen"
      :draft="draftFolder"
      @confirm="importFolderOpen = false"
      @cancel="importFolderOpen = false"
    />
    <SequenceDialog v-if="seqOpen" @confirm="seqOpen = false" @cancel="seqOpen = false" />

    <div class="pv-group">
      <h3 class="pv-group-title">移动（BaseModal framed 变体：壳 padding 0 / 列 gap 12 + 标题栏 54 带底线与关闭 X；静态复刻 = 真实组件拼装）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-move-shell">
            <div class="pv-move-titlebar">
              <h2 class="pv-move-title">移动文件</h2>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-move-search">
              <TextInput leading-icon="search" placeholder="搜索文件夹..." />
            </div>
            <div class="pv-move-tree">
              <SectionTitle variant="block" title="文件夹" :count="123" expanded :with-more="false" force-state="hover" />
              <div class="pv-move-nodes">
                <TreeRowItem kind="folder" name="人物" :count="20" :direct-count="20" expanded expandable />
                <TreeRowItem kind="folder" name="场景" :count="8" :depth="1" :expandable="false" drop-zone="into" />
                <TreeRowItem kind="archive" name="角色设定" :count="20" />
              </div>
            </div>
          </div>
          <span class="pv-cap">
            壳复刻 BaseModal framed 口径（口径变更与组件同步）；行 2 = 并入落点高亮（品牌/半透明填充 + 2px 品牌描边）；
            区块标题 force-hover 展示添加钮（稿有画，移动窗新建文件夹未立项 = 点亮不接业务）；业务弹窗依赖库数据，不进真实弹出
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* 预览壳复刻 ConfirmModal 窗口口径（fixed 遮罩层不便静态展示；口径变更时与组件同步） */
.pv-modal-shell {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  width: 520px;
  padding: var(--n-space-24);
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
}
.pv-modal-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.pv-modal-body {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.pv-modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--n-space-8);
}
.pv-modal-btn {
  /* 宽自适应内容 + 左右 48px（2026-09-13 实机走查定稿值，废 120 定宽——与 ConfirmModal .btn 同步） */
  height: 38px;
  padding: var(--n-space-8) 48px;
  border-radius: var(--n-radius-8);
  font-size: var(--n-fs-btn-lg);
  line-height: var(--n-lh-btn-lg);
  font-weight: var(--n-fw-bold);
  flex: none;
}
.pv-modal-btn.cancel {
  background: var(--n-g5);
  box-shadow: var(--n-effect-raised);
  color: var(--n-l2);
}
.pv-modal-btn.confirm {
  color: var(--n-l1);
  box-shadow: var(--n-effect-raised-primary);
}
.pv-modal-btn.confirm.warning {
  background: var(--n-warning);
}
.pv-modal-btn.confirm.danger {
  background: var(--n-danger);
}
.pv-modal-btn.confirm.brand {
  background: var(--n-brand);
}
/* 移动弹窗静态复刻壳（复刻 BaseModal framed 口径：壳 padding 0 / 列 gap 12 / 标题栏 54 + 底线） */
.pv-move-shell {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
  width: 460px;
  box-sizing: border-box;
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow: var(--n-shadow-lg);
}
.pv-move-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
  box-sizing: border-box;
  height: 54px;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-bottom: 1px solid var(--n-g5);
  flex: none;
}
.pv-move-title {
  font-size: var(--n-fs-title);
  line-height: var(--n-lh-title);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.pv-move-search {
  padding: 0 var(--n-space-24);
}
/* 树形容器：0 24 padding / 区块内 标题→树 gap 8；item 间 gap 2（移动稿） */
.pv-move-tree {
  padding: 0 var(--n-space-24);
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
.pv-move-nodes {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
}
</style>
