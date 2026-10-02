<script setup lang="ts">
// 模态窗口展区——docs/06_UI重构/Design Token/组件/模态窗口/弹窗.md（警告与错误，2026-09-01）。
// ConfirmModal 是 fixed 遮罩层不便静态展示：.pv-modal-shell 复刻窗口口径（口径变更时与组件同步），
// 末行真实弹出实例验收遮罩（黑色-50%）与弹出动画（缩放+淡入 150ms）。
import { ref } from "vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import AppIcon from "../../components/AppIcon.vue";
import TwoToneIcon from "../../components/items/TwoToneIcon.vue";
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
const renameOpen = ref(false);
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
          <span class="pv-cap">真实弹出（mock 三库含容量）：行对勾切换可验；勾 ≥2 底行换显目标 + 确认可点（确认 = 真合并，勿点）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="重命名资源库" variant="solid" size="md" @click="renameOpen = true" />
          <span class="pv-cap">创建弹窗改名模式（renameFrom 预填旧名；确认 = 真改名，勿点）</span>
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
    <CreateLibraryModal v-if="renameOpen" rename-from="动画资产" @close="renameOpen = false" />
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
      <h3 class="pv-group-title">管理工作空间（设计稿提案 2026-10-02 · 未沉淀组件——链接进来 / 链出去双向总览面板）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-move-shell w640">
            <div class="pv-move-titlebar">
              <h2 class="pv-move-title">管理工作空间</h2>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-ws-body">
              <SectionTitle variant="block" title="链接进来的文件夹" :count="2" expanded :with-more="false" />
              <div class="pv-ws-rows">
                <!-- 正常链接门：徽章 + 门名 / ← 目标路径 + 三操作（重新定位 / 显示 / 删除） -->
                <div class="pv-ws-row">
                  <AppIcon name="linked-file-badge" :size="18" class="pv-ws-lead" />
                  <div class="pv-ws-text">
                    <span class="pv-ws-name">01_projects</span>
                    <span class="pv-ws-path" title="D:\anikit\01_projects">← D:\anikit\01_projects</span>
                  </div>
                  <span class="pv-ws-ops">
                    <IconButton variant="bare" size="sm" name="link-small" title="重新定位…" />
                    <IconButton variant="bare" size="sm" name="open-in-explorer" title="在资源管理器中显示" />
                    <IconButton variant="bare" size="sm" name="trash" title="删除链接（只删门进回收站）" />
                  </span>
                </div>
                <!-- 断链门：名后 warning 徽章 + 文本 dimmed 0.4 置灰（§65 同档） -->
                <div class="pv-ws-row">
                  <AppIcon name="linked-file-badge" :size="18" class="pv-ws-lead" />
                  <div class="pv-ws-text dim">
                    <span class="pv-ws-name">参考素材<TwoToneIcon name="warning-small" :size="14" class="pv-ws-warn" /></span>
                    <span class="pv-ws-path" title="D:\参考\动作参考（目标已不存在）">← D:\参考\动作参考</span>
                  </div>
                  <span class="pv-ws-ops">
                    <IconButton variant="bare" size="sm" name="link-small" title="重新定位…" />
                    <IconButton variant="bare" size="sm" name="open-in-explorer" title="在资源管理器中显示" />
                    <IconButton variant="bare" size="sm" name="trash" title="删除链接（只删门进回收站）" />
                  </span>
                </div>
              </div>
              <button class="pv-ws-add">
                <AppIcon name="add-small" :size="16" />
                添加链接文件夹…（挂载到工作空间根目录）
              </button>
              <SectionTitle variant="block" title="链出去的文件夹" :count="2" expanded :with-more="false" />
              <div class="pv-ws-rows">
                <!-- 组 1：文件夹 + 两条入口（均正常） -->
                <div class="pv-ws-group">
                  <div class="pv-ws-ghead">
                    <TwoToneIcon name="folder-closed" :size="18" />
                    <span class="pv-ws-name">02_创作</span>
                    <IconButton variant="bare" size="sm" name="add-small" title="添加入口路径…" class="pv-ws-gadd" />
                  </div>
                  <div class="pv-ws-sub">
                    <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                    <span class="pv-ws-path one" title="D:\常用\02_创作">D:\常用\02_创作</span>
                    <span class="pv-ws-ops">
                      <IconButton variant="bare" size="sm" name="open-in-explorer" title="在资源管理器中显示" />
                      <IconButton variant="bare" size="sm" name="trash" title="删除入口（门进回收站 + 剔记录）" />
                    </span>
                  </div>
                  <div class="pv-ws-sub">
                    <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                    <span class="pv-ws-path one" title="E:\工作盘\创作">E:\工作盘\创作</span>
                    <span class="pv-ws-ops">
                      <IconButton variant="bare" size="sm" name="open-in-explorer" title="在资源管理器中显示" />
                      <IconButton variant="bare" size="sm" name="trash" title="删除入口（门进回收站 + 剔记录）" />
                    </span>
                  </div>
                </div>
                <!-- 组 2：入口被外部删除（门没了 = 置灰 + warning，操作只留删除记录） -->
                <div class="pv-ws-group">
                  <div class="pv-ws-ghead">
                    <TwoToneIcon name="folder-closed" :size="18" />
                    <span class="pv-ws-name">04_文库</span>
                    <IconButton variant="bare" size="sm" name="add-small" title="添加入口路径…" class="pv-ws-gadd" />
                  </div>
                  <div class="pv-ws-sub">
                    <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                    <span class="pv-ws-path one dim" title="C:\Users\kingl\Desktop\文库（入口已不存在）">
                      C:\Users\kingl\Desktop\文库<TwoToneIcon name="warning-small" :size="14" class="pv-ws-warn" />
                    </span>
                    <span class="pv-ws-ops">
                      <IconButton variant="bare" size="sm" name="trash" title="删除记录（入口已不在，仅剔登记）" />
                    </span>
                  </div>
                </div>
              </div>
              <button class="pv-ws-add">
                <AppIcon name="add-small" :size="16" />
                链出去新文件夹…（选工作空间文件夹 → 选入口摆放位置）
              </button>
            </div>
          </div>
          <span class="pv-cap">
            <b>提案待定点</b>：①链出去要显示 ⇒ 登记口径升级（原「不登记」→ workspace.json 记 portals 台账：创建时登记、
            打开面板对账，门没了只标记不剔除可手删——仍不做自动管理）；②添加链进来挂工作空间根（换位置走后补拖拽）；
            ③「链出去新文件夹…」需两步选择（WS 内选文件夹 + 选落点），或保持右键「在外部位置创建链接…」发起、
            面板只管总览与追加；④删除均走 confirmPrompt 警告确认（门进系统回收站可找回）。壳 640 = 分组管理先例
            （pv-move-shell 复刻壳加 w640 档）；行 = DetailBlockRow 52 双行族 / 链出去 40 首行 + 32 子行缩进 26；
            无底部按钮行（即时生效，§61 同口径）。提案未定稿，暂陈列于本区（framed 复刻族旁），定稿后沉淀组件并迁业务弹窗展区
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-move-shell w640">
            <div class="pv-move-titlebar">
              <h2 class="pv-move-title">管理工作空间</h2>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-ws-body">
              <SectionTitle variant="block" title="链接进来的文件夹" :count="0" expanded :with-more="false" />
              <p class="pv-ws-empty">还没有链接进来的文件夹——右键工作空间空白处或文件夹，选「添加链接文件夹…」</p>
              <button class="pv-ws-add">
                <AppIcon name="add-small" :size="16" />
                添加链接文件夹…（挂载到工作空间根目录）
              </button>
              <SectionTitle variant="block" title="链出去的文件夹" :count="0" expanded :with-more="false" />
              <p class="pv-ws-empty">还没有链出去的文件夹——右键工作空间文件夹，选「在外部位置创建链接…」</p>
              <button class="pv-ws-add">
                <AppIcon name="add-small" :size="16" />
                链出去新文件夹…（选工作空间文件夹 → 选入口摆放位置）
              </button>
            </div>
          </div>
          <span class="pv-cap">空态形态：区块无行 = 引导文案一行（次要文字 12/16 三级），添加行照常</span>
        </div>
      </div>
    </div>

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
.pv-move-shell.w640 {
  width: 640px;
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

/* —— 管理工作空间提案（2026-10-02）：行族 = DetailBlockRow 52 双行口径 / 链出去组 = 40 组首行 +
   32 入口子行（缩进 26 对齐组首行名称起点）——全 token 零新档 —— */
.pv-ws-body {
  display: flex;
  flex-direction: column;
  padding: 0 var(--n-space-24) var(--n-space-24);
}
.pv-ws-rows {
  display: flex;
  flex-direction: column;
}
/* 链接门行：52 高（border-box）/ padding 12 0 / gap 8 / 底线 g5 末行去线（DetailBlockRow 同口径） */
.pv-ws-row {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 52px;
  padding: var(--n-space-12) 0;
  box-sizing: border-box;
  border-bottom: 1px solid var(--n-g5);
}
.pv-ws-row:last-child {
  border-bottom: none;
}
.pv-ws-lead {
  color: var(--n-l2);
  flex: none;
}
/* 双行文本列：名 13/18 一级 / 目标路径 12/16 三级（次要文字档），均截断 + title 全显 */
.pv-ws-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.pv-ws-name {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-4);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pv-ws-path {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
/* 断链 / 门没了置灰 = dimmed 0.4 既有档（§5 / §65） */
.pv-ws-text.dim,
.pv-ws-path.dim {
  opacity: 0.4;
}
.pv-ws-warn {
  flex: none;
}
/* 行尾操作组（IconButton bare sm 自带常态三级 hover 二级档） */
.pv-ws-ops {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-2);
  flex: none;
}
/* 区块底添加行：40 高 / 图标+文字三级 hover 一级 + 四级底 hover（文字钮就近档） */
.pv-ws-add {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  height: 40px;
  padding: 0 var(--n-space-8);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  flex: none;
  transition:
    color var(--n-dur-150) var(--n-ease-in-out),
    background var(--n-dur-150) var(--n-ease-in-out);
}
.pv-ws-add:hover {
  color: var(--n-l1);
  background: var(--n-g4);
}
/* 链出去组首行：40 高 / 文件夹图标 18 + 名 13/18 一级 + 行尾添加入口钮 */
.pv-ws-ghead {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 40px;
  box-sizing: border-box;
}
.pv-ws-gadd {
  margin-left: auto;
}
/* 入口子行：32 高 / 缩进 26（18 图标位 + gap 8，对齐组首行名称）/ 链接图标 14 三级 +
   路径 12/16 二级截断 / 行尾操作 */
.pv-ws-sub {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 32px;
  padding-left: 26px;
  box-sizing: border-box;
}
.pv-ws-sublead {
  color: var(--n-l3);
  flex: none;
}
.pv-ws-sub .pv-ws-path {
  flex: 1;
  min-width: 0;
  color: var(--n-l2);
}
.pv-ws-sub .pv-ws-path.one {
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-4);
}
/* 空态引导行：次要文字 12/16 三级，上下留白 12 */
.pv-ws-empty {
  margin: 0;
  padding: var(--n-space-12) 0;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
</style>
