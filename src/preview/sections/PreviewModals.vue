<script setup lang="ts">
// 模态窗口展区——docs/06_UI重构/Design Token/组件/模态窗口/弹窗.md（警告与错误，2026-09-01）。
// ConfirmModal 是 fixed 遮罩层不便静态展示：.pv-modal-shell 复刻窗口口径（口径变更时与组件同步），
// 末行真实弹出实例验收遮罩（黑色-50%）与弹出动画（缩放+淡入 150ms）。
import { ref } from "vue";
import "../../components/items/items.css";
import ActionButton from "../../components/buttons/ActionButton.vue";
import IconButton from "../../components/buttons/IconButton.vue";
import AppIcon from "../../components/AppIcon.vue";
import TwoToneIcon from "../../components/items/TwoToneIcon.vue";
import ConfirmModal from "../../components/modals/ConfirmModal.vue";
import WsManageDialog from "../../components/WsManageDialog.vue";
import type { WsLinkOverview } from "../../lib/wsOps";
import SettingsDialog from "../../components/SettingsDialog.vue";
import type { SettingsMockData } from "../../lib/settingsData";
import CreateLibraryModal from "../../components/CreateLibraryModal.vue";
import MergeLibraryModal from "../../components/MergeLibraryModal.vue";
import ImportDialog from "../../components/ImportDialog.vue";
import SequenceDialog from "../../components/SequenceDialog.vue";
import TextInput from "../../components/inputs/TextInput.vue";
import SectionTitle from "../../components/items/SectionTitle.vue";
import TreeRowItem from "../../components/items/TreeRowItem.vue";
import DestTreeDialog from "../../components/DestTreeDialog.vue";
import { useSelectionStore } from "../../stores/selection";
import type { ImportDraft } from "../../stores/content";

const demoOpen = ref(false);
const createOpen = ref(false);
const renameOpen = ref(false);
const mergeOpen = ref(false);
const importFilesOpen = ref(false);
const importFolderOpen = ref(false);
const seqOpen = ref(false);
// 管理工作空间真实弹出（WsManageDialog 已沉淀，2026-10-02；mock 台账数据，操作钮 mock 态不动作）
const wsManageOpen = ref(false);
const wsManageMock: WsLinkOverview = {
  links: [
    { rel: "01_projects", target: "D:\\anikit\\01_projects", ok: true },
    { rel: "参考素材", target: "D:\\参考\\动作参考", ok: false },
  ],
  portals: [
    { rel: "02_创作", relOk: true, portals: [{ path: "D:\\常用\\02_创作", ok: true }, { path: "E:\\工作盘\\创作", ok: true }] },
    { rel: "04_文库", relOk: true, portals: [{ path: "C:\\Users\\kingl\\Desktop\\文库", ok: false }] },
  ],
};
const mockLibItems = [
  { name: "沙雕动画资源库", sizeBytes: 320 * 1024 ** 3 },
  { name: "学习参考", sizeBytes: 12.6 * 1024 ** 3 },
  { name: "项目素材备份", sizeBytes: 860 * 1024 ** 2 },
];
// 设置面板真实弹出（1.0 收尾第 1 项，preview 先行——纯展示组件，不接 store / 后端）：
// mock 缓存各项给代表字节数（GB/MB/KB/B 全覆盖 formatSize 各档）、盘符「已挂载 Z:」、版本 0.1.0
const settingsOpen = ref(false);
const settingsMock: SettingsMockData = {
  spaceZoom: "hover",
  doubleClick: "preview",
  autoImport: true,
  inboxDrive: "Z:",
  cacheLib: [
    { id: "lib-thumb", label: "缩略图与动图", note: "清理后自动重建", bytes: 1.28 * 1024 ** 3 },
    { id: "lib-preview", label: "预览与帧缓存", bytes: 864 * 1024 ** 2 },
    { id: "lib-media", label: "音视频转码与波形", bytes: 412.5 * 1024 ** 2 },
    { id: "lib-icon", label: "系统格式图标", bytes: 768 * 1024 },
  ],
  cacheWs: [
    { id: "ws-thumb", label: "缩略图与图标", bytes: 96.4 * 1024 ** 2 },
    { id: "ws-media", label: "音视频转码与波形", bytes: 128 * 1024 ** 2 },
    { id: "ws-probe", label: "探测缓存", bytes: 512 },
  ],
  cep: { maxSecs: 10, maxMB: 2, fps: 0 },
  version: "0.1.0",
};
const draftFiles: ImportDraft = { kind: "files", paths: ["C:/演示/主角-走01.png"], targetFolder: null };
const draftFolder: ImportDraft = { kind: "folder", paths: ["C:/演示/角色素材"], targetFolder: null };
// 目的树选择器真实弹出（DestTreeDialog，2026-10-03 库 ↔ 工作空间互通；mock 草稿——
// 确认闭包不动作直接关；库模式数据源 = content.blockSections，预览环境连真实库陈列完整树）
const selection = useSelectionStore();
function openDestTree(): void {
  selection.destTreeDraft = {
    tree: "library",
    title: "导入到库",
    sourceName: "演示文件.png",
    onPick: () => Promise.resolve(true),
  };
}
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
      <h3 class="pv-group-title">管理工作空间（2026-10-02 已落地接生产 = components/WsManageDialog——链接进来 / 链出去双向总览；静态复刻壳留作常态陈列，真实弹出走 mock 台账）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="打开交互演示" variant="solid" size="md" @click="wsManageOpen = true" />
          <span class="pv-cap">真实弹出（mock 台账：链进来 1 正常 + 1 断链 / 链出去 2 组含门没了态）——操作钮 mock 态只演示 hover 滑入，点击不动作</span>
        </div>
      </div>
      <WsManageDialog v-if="wsManageOpen" :mock="wsManageMock" @close="wsManageOpen = false" />
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-move-shell w640">
            <div class="pv-move-titlebar">
              <h2 class="pv-move-title">管理工作空间</h2>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-ws-body">
              <!-- 区块一：链进来（方向 b 门清单） -->
              <div class="pv-ws-sec">
                <SectionTitle variant="block" title="链接进来的文件夹" :count="2" expanded :actions="false" />
                <div class="pv-ws-cards">
                  <!-- 正常链接门：徽章 + 门名 / ← 目标路径；操作组 hover 滑入（污染文件 item 同款） -->
                  <div class="pv-ws-card ni-hover">
                    <AppIcon name="linked-file-badge" :size="18" class="pv-ws-lead" />
                    <div class="pv-ws-text">
                      <span class="pv-ws-name">01_projects</span>
                      <span class="pv-ws-path" title="D:\anikit\01_projects">← D:\anikit\01_projects</span>
                    </div>
                    <span class="ni-slide"><span class="ni-slide-inner">
                      <button class="pv-ws-act" title="重新定位…"><AppIcon name="link-small" :size="18" /></button>
                      <button class="pv-ws-act" title="在资源管理器中显示"><AppIcon name="open-in-explorer" :size="18" /></button>
                      <button class="pv-ws-act" title="删除链接（只删门进回收站）"><AppIcon name="trash" :size="18" /></button>
                    </span></span>
                  </div>
                  <!-- 断链门：整卡 dimmed 0.4 置灰 + 名后 warning 徽章（§65 同档） -->
                  <div class="pv-ws-card ni-hover dim">
                    <AppIcon name="linked-file-badge" :size="18" class="pv-ws-lead" />
                    <div class="pv-ws-text">
                      <span class="pv-ws-name">参考素材<TwoToneIcon name="warning-small" :size="14" class="pv-ws-warn" /></span>
                      <span class="pv-ws-path" title="D:\参考\动作参考（目标已不存在）">← D:\参考\动作参考</span>
                    </div>
                    <span class="ni-slide"><span class="ni-slide-inner">
                      <button class="pv-ws-act" title="重新定位…"><AppIcon name="link-small" :size="18" /></button>
                      <button class="pv-ws-act" title="在资源管理器中显示"><AppIcon name="open-in-explorer" :size="18" /></button>
                      <button class="pv-ws-act" title="删除链接（只删门进回收站）"><AppIcon name="trash" :size="18" /></button>
                    </span></span>
                  </div>
                </div>
                <ActionButton label="添加链接文件夹…" icon="add-small" variant="solid" size="md" title="选外部文件夹，挂载到工作空间根目录" />
              </div>
              <!-- 区块二：链出去（方向 a′ 入口台账）；顶部分隔线 + 大间距区分功能 -->
              <div class="pv-ws-sec sep">
                <SectionTitle variant="block" title="链出去的文件夹" :count="2" expanded :actions="false" />
                <div class="pv-ws-cards">
                  <!-- 组 1：文件夹 + 两条入口（均正常）；组卡 hover 首行出「添加入口」，子行各自 hover 出操作 -->
                  <div class="pv-ws-gcard">
                    <div class="pv-ws-ghead">
                      <TwoToneIcon name="folder-closed" :size="18" />
                      <span class="pv-ws-name">02_创作</span>
                      <span class="ni-slide"><span class="ni-slide-inner">
                        <button class="pv-ws-act" title="添加入口路径…"><AppIcon name="add-small" :size="18" /></button>
                      </span></span>
                    </div>
                    <div class="pv-ws-sub ni-hover">
                      <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                      <span class="pv-ws-subpath" title="D:\常用\02_创作">D:\常用\02_创作</span>
                      <span class="ni-slide"><span class="ni-slide-inner">
                        <button class="pv-ws-act" title="在资源管理器中显示"><AppIcon name="open-in-explorer" :size="18" /></button>
                        <button class="pv-ws-act" title="删除入口（门进回收站 + 剔记录）"><AppIcon name="trash" :size="18" /></button>
                      </span></span>
                    </div>
                    <div class="pv-ws-sub ni-hover">
                      <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                      <span class="pv-ws-subpath" title="E:\工作盘\创作">E:\工作盘\创作</span>
                      <span class="ni-slide"><span class="ni-slide-inner">
                        <button class="pv-ws-act" title="在资源管理器中显示"><AppIcon name="open-in-explorer" :size="18" /></button>
                        <button class="pv-ws-act" title="删除入口（门进回收站 + 剔记录）"><AppIcon name="trash" :size="18" /></button>
                      </span></span>
                    </div>
                  </div>
                  <!-- 组 2：入口被外部删除（门没了 = 子行置灰 + warning，操作只留删除记录） -->
                  <div class="pv-ws-gcard">
                    <div class="pv-ws-ghead">
                      <TwoToneIcon name="folder-closed" :size="18" />
                      <span class="pv-ws-name">04_文库</span>
                      <span class="ni-slide"><span class="ni-slide-inner">
                        <button class="pv-ws-act" title="添加入口路径…"><AppIcon name="add-small" :size="18" /></button>
                      </span></span>
                    </div>
                    <div class="pv-ws-sub ni-hover">
                      <AppIcon name="link-small" :size="14" class="pv-ws-sublead" />
                      <span class="pv-ws-subpath dim" title="C:\Users\kingl\Desktop\文库（入口已不存在）">
                        C:\Users\kingl\Desktop\文库<TwoToneIcon name="warning-small" :size="14" class="pv-ws-warn" />
                      </span>
                      <span class="ni-slide"><span class="ni-slide-inner">
                        <button class="pv-ws-act" title="删除记录（入口已不在，仅剔登记）"><AppIcon name="trash" :size="18" /></button>
                      </span></span>
                    </div>
                  </div>
                </div>
                <ActionButton label="链出去新文件夹…" icon="add-small" variant="solid" size="md" title="选工作空间文件夹 → 选入口摆放位置" />
              </div>
            </div>
          </div>
          <span class="pv-cap">
            壳 640 = 分组管理先例（pv-move-shell 复刻壳加 w640 档）；卡片化分层（g4 底 + 1px g5 + 圆角 8 = ni-card 升档，
            行卡 hover g5/g6，断链整卡 dimmed 置灰）；操作组 = 污染文件 item 同款 ni-slide hover 滑入（18 盒 l4→l2）；
            添加钮 = ActionButton solid md 真件；两区块 24+1px g5 顶线+24 分界；无底部按钮行（即时生效，§61 同口径）。
            <b>已接生产（2026-10-02）</b>：本体 = WsManageDialog（台账 = workspace.json 的 links + portals，
            对账 = 后端现场探测；链出去文件夹改名 / 移动后台账词法平移、旧门指向原路径即对账断）；
            本复刻壳留作常态陈列（口径变更与组件同步）
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-move-shell w640">
            <div class="pv-move-titlebar">
              <h2 class="pv-move-title">管理工作空间</h2>
              <IconButton name="close-large" title="关闭" />
            </div>
            <div class="pv-ws-body">
              <div class="pv-ws-sec">
                <SectionTitle variant="block" title="链接进来的文件夹" :count="0" expanded :actions="false" />
                <p class="pv-ws-empty">还没有链接进来的文件夹——右键工作空间空白处或文件夹，选「添加链接文件夹…」</p>
                <ActionButton label="添加链接文件夹…" icon="add-small" variant="solid" size="md" title="选外部文件夹，挂载到工作空间根目录" />
              </div>
              <div class="pv-ws-sec sep">
                <SectionTitle variant="block" title="链出去的文件夹" :count="0" expanded :actions="false" />
                <p class="pv-ws-empty">还没有链出去的文件夹——右键工作空间文件夹，选「在外部位置创建链接…」</p>
                <ActionButton label="链出去新文件夹…" icon="add-small" variant="solid" size="md" title="选工作空间文件夹 → 选入口摆放位置" />
              </div>
            </div>
          </div>
          <span class="pv-cap">空态形态：区块无行 = 引导文案一行（次要文字 12/16 三级），添加行照常</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">设置面板（2026-10-13 已接生产 = components/SettingsDialog + lib/settingsData——主菜单「偏好设置」/ Ctrl+, 入口；mock 陈列保留作常态陈列）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="打开交互演示" variant="solid" size="md" @click="settingsOpen = true" />
          <span class="pv-cap">
            真实弹出（mock 全量：缓存 B/KB/MB/GB 各档代表值、盘符「已挂载 Z:」、版本 0.1.0）。
            2026-10-13 四轮定稿（截图箭头）= Eagle 式左右布局·左浅右深**全出血**：壳 BaseModal framed 720×480 定高；
            左导航列 168 壳色 g3（树行口径 32 高 / hover a3 / 选中 a2 + l1；行首 18px 图标占位空盒后补）；
            列右缘 1px g5 竖线顶贴标题线到底缘通高；右内容区 = g1 深色填满（撤三轮内嵌卡片——
            标题横线 / 竖线 / 两块底色相连无断缝；framed 列 gap 归 0 + 内容段 padding 四边归零——
            注意 fallthrough class 落在 BaseModal 根 = 遮罩上，压壳样式须 :deep 进 .bm-modal.framed（五轮修横带的教训））；切分类 = 详情面板 tab 切换同款（150 缓出 + 250 缓入 delay 100 位移 48px）。
            无按钮行（即时生效，§61/§66）；Esc/X 关闭，遮罩点击不关闭。
            <b>已接生产（2026-10-13）</b>：业务收口 lib/settingsData.ts（UI 无业务）——通用 = ui 持久化键
            （spaceZoomMode / doubleClickOpen）；自动导入 = 空间级 set_auto_import + 盘符 get_inbox_drive_letter；
            缓存 = get_cache_overview / clear_cache（全部清理走 confirmPrompt warning）+ library-thumbs-rebuilt 事件；
            插件 = cep_export_settings 读写（MB↔字节换算，护栏同后端 validate）；关于 = getVersion +
            show_logs_in_explorer。mock 弹出 = 陈列态（isMock 早退，命令一律不发）；
            快捷键 = 组件内静态数组（bound 标志），未接入行 = 键位降四级 +「未接入」警告橙尾标（颜色即状态区分）
          </span>
        </div>
      </div>
      <SettingsDialog v-if="settingsOpen" :mock="settingsMock" @close="settingsOpen = false" />
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
        <div class="pv-demo-cell">
          <ActionButton label="目的树选择器" variant="solid" size="md" @click="openDestTree" />
          <span class="pv-cap">
            真实弹出（DestTreeDialog，库 ↔ 工作空间互通共用：壳 / 搜索 / MoveNode 树全复用移动面板，
            差异 = 行级 hover 恒并入 + 首行「未分类」根落点）——mock 草稿确认即关；
            单击行出 ConfirmPrompt 两段确认（移动面板同口径）；库数据空时只出未分类行
          </span>
        </div>
      </div>
    </div>

    <DestTreeDialog v-if="selection.destTreeDraft" />

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

/* —— 管理工作空间提案（2026-10-02 二轮，用户走查后卡片化）——
   行 / 组 = 卡片分层（g4 底 + 1px g5 + 圆角 8；行卡 hover g5 底 g6 描边——ni-card 口径升一档）；
   操作组 = 污染文件 item 同款 ni-slide hover 滑入（18 盒 l4→l2）；添加行整行按钮化（g4 底 hover g5）；
   两区块 = 父 gap 24 + 1px g5 顶线 + 24 留白（功能分界）——全 token 零新档 */
.pv-ws-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding: 0 var(--n-space-24) var(--n-space-24);
}
/* 区块容器：标题 → 卡组 → 添加行 */
.pv-ws-sec {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 区块二：顶部分隔线 + 线后 24 留白（父 gap 24 在线前——合计 24+1+24 功能分界） */
.pv-ws-sec.sep {
  border-top: 1px solid var(--n-g5);
  padding-top: var(--n-space-24);
}
.pv-ws-cards {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 行卡（链进来门）：g4 底 + 1px g5 + 圆角 8 + padding 12；hover g5 底 g6 描边 */
.pv-ws-card {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  padding: var(--n-space-12);
  box-sizing: border-box;
  background: var(--n-g4);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
  transition:
    background var(--n-dur-100) var(--n-ease-in-out),
    border-color var(--n-dur-100) var(--n-ease-in-out);
}
.pv-ws-card:hover {
  background: var(--n-g5);
  border-color: var(--n-g6);
}
/* 断链整卡置灰 = dimmed 0.4 既有档（§5 / §65） */
.pv-ws-card.dim {
  opacity: 0.4;
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
.pv-ws-warn {
  flex: none;
}
/* 操作钮（污染文件 item .act 同款）：18 盒 / 常态四级 hover 二级 */
.pv-ws-act {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  flex: none;
  padding: 0;
  color: var(--n-l4);
  cursor: pointer;
  transition: color var(--n-dur-100) var(--n-ease-in-out);
}
.pv-ws-act:hover {
  color: var(--n-l2);
}
/* 组卡（链出去文件夹 + 其入口子行）：同卡底但列向；hover 不变底（让位子行 hover 层），仅首行滑入「添加入口」 */
.pv-ws-gcard {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-4) var(--n-space-12);
  box-sizing: border-box;
  background: var(--n-g4);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
}
/* 组卡 hover 仅出首行「添加入口」（ni-slide 直达子选择器——不波及子行各自的滑入组） */
.pv-ws-gcard:hover > .pv-ws-ghead > .ni-slide {
  grid-template-columns: 1fr;
  opacity: 1;
}
/* 组首行：36 高 / 图标 18 + 名 13/18 一级 + 右端滑入添加钮；底部 1px g5 与子行分隔 */
.pv-ws-ghead {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 36px;
  box-sizing: border-box;
  border-bottom: 1px solid var(--n-g5);
}
.pv-ws-ghead .pv-ws-name {
  flex: 1;
  min-width: 0;
}
/* 入口子行：30 高 / 缩进 26（18 图标位 + gap 8 对齐组首行名）/ 链接图标 14 三级 +
   路径 12/16 二级截断；hover = g5 底圆角 6 小底（嵌套层级）+ 操作组滑入 */
.pv-ws-sub {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 30px;
  padding-left: 26px;
  padding-right: var(--n-space-4);
  box-sizing: border-box;
  border-radius: var(--n-radius-6);
  transition: background var(--n-dur-100) var(--n-ease-in-out);
}
.pv-ws-sub:hover {
  background: var(--n-g5);
}
.pv-ws-sublead {
  color: var(--n-l3);
  flex: none;
}
.pv-ws-subpath {
  flex: 1;
  min-width: 0;
  display: inline-flex;
  align-items: center;
  gap: var(--n-space-4);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l2);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.pv-ws-subpath.dim {
  opacity: 0.4;
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
