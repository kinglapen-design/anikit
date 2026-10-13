<script setup lang="ts">
// 管理工作空间面板（2026-10-02 落地；设计稿 = PreviewModals「管理工作空间」组三轮陈列，
// 用户走查定稿）——链接进来（方向 b 门台账）与链出去（方向 a′ portals 台账）双区块一屏总览。
// 壳 = BaseModal framed 640（分组管理先例）；行/组卡片化（g4 底 + 1px g5 + 圆角 8，
// ni-card 升档）；操作组 = 污染文件 item 同款 ni-slide hover 滑入（18 盒 l4→l2）；
// 添加钮 = ActionButton solid md 真件；断链 / 门没了 = dimmed 0.4 置灰 + warning-small（§65 同档）；
// 删除走 confirmPrompt 警告黄钮（门进系统回收站可找回）；即时生效无底部按钮行（§61 同口径）。
// 数据 = ws_link_overview（打开拉 + 操作后重拉；台账对账 = 后端现场探测门与指向）。
// props.mock = preview 陈列用（有 mock 不调后端）。
import { computed, onMounted, ref } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SectionTitle from "./items/SectionTitle.vue";
import AppIcon from "./AppIcon.vue";
import TwoToneIcon from "./items/TwoToneIcon.vue";
import "./items/items.css";
import { useSelectionStore } from "../stores/selection";
import { useToastStore } from "../stores/toast";
import { useWsStore } from "../stores/ws";
import {
  linkOutWsFolder,
  wsLinkOverview,
  wsRemovePortal,
  wsRevealExternal,
  type WsLinkOverview,
} from "../lib/wsOps";

const props = defineProps<{ mock?: WsLinkOverview }>();
const emit = defineEmits<{ close: [] }>();
const toast = useToastStore();
const selection = useSelectionStore();
const ws = useWsStore();
const fail = (e: unknown) => toast.notify({ message: String(e), icon: "fail" });

const overview = ref<WsLinkOverview>(props.mock ?? { links: [], portals: [] });
async function refresh(): Promise<void> {
  if (props.mock) return;
  overview.value = await wsLinkOverview().catch(() => overview.value);
}
onMounted(() => void refresh());

// mock 陈列态（preview）：操作钮只演示 hover 滑入，点击不动作（无后端环境）
const isMock = computed(() => props.mock !== undefined);

function baseName(rel: string): string {
  return rel.split("/").pop() ?? rel;
}

// 目录选择器（添加链接文件夹 / 添加入口 / 链出去新文件夹共用）：取消 = null 则止
async function pickDir(title?: string): Promise<string | null> {
  const picked = await open({ directory: true, multiple: false, ...(title ? { title } : {}) });
  return typeof picked === "string" ? picked : null;
}

// —— 链进来（方向 b 门）：重新定位 = 断链送修同径；删除链接 = 只删门进回收站 ——
async function retarget(rel: string): Promise<void> {
  if (isMock.value) return;
  const dir = await pickDir();
  if (dir === null) return;
  await ws.retargetLink(rel, dir).catch(fail);
  await refresh();
}
function reveal(path: string): void {
  if (isMock.value) return;
  void wsRevealExternal(path).catch(fail);
}
function removeLink(rel: string, name: string): void {
  if (isMock.value) return;
  selection.confirmPrompt = {
    title: "删除链接",
    body: `只删除链接「${name}」本身（进系统回收站，可找回），目标文件夹内容不受影响。`,
    confirmLabel: "删除链接",
    kind: "warning",
    onConfirm: () =>
      void ws
        .removeLink(rel)
        .then(() => refresh())
        .catch((e: unknown) => fail(e)),
  };
}
// 添加链接文件夹（挂根 = 面板无目录上下文，换位置走树拖拽移动）
async function addLink(): Promise<void> {
  if (isMock.value) return;
  const dir = await pickDir();
  if (dir === null) return;
  await ws.addLink("", dir).catch(fail);
  await refresh();
}

// —— 链出去（方向 a′ 入口）：添加入口 = 既有文件夹追加；删除入口 = 门进回收站 + 剔登记 ——
async function addPortal(rel: string): Promise<void> {
  if (isMock.value) return;
  const dir = await pickDir("选择链接入口的摆放位置");
  if (dir === null) return;
  const link = await linkOutWsFolder(rel, dir).catch((e: unknown) => {
    fail(e);
    return null;
  });
  if (link !== null) {
    toast.notify({ message: `已创建外部链接入口：${link}`, icon: "success" });
    await refresh();
  }
}
function removePortal(rel: string, portal: string, gone: boolean): void {
  if (isMock.value) return;
  selection.confirmPrompt = {
    title: gone ? "删除记录" : "删除入口",
    body: gone
      ? "入口已不在外部位置。从清单中删除这条记录即可（不影响工作空间内的文件夹）。"
      : `删除外部入口「${portal}」（进系统回收站，可找回），工作空间内的文件夹内容不受影响。`,
    confirmLabel: gone ? "删除记录" : "删除入口",
    kind: "warning",
    onConfirm: () =>
      void wsRemovePortal(rel, portal)
        .then(() => refresh())
        .catch((e: unknown) => fail(e)),
  };
}
// 链出去新文件夹：两步选择（选 WS 内文件夹 → 选入口摆放位置）
async function linkOutNew(): Promise<void> {
  if (isMock.value) return;
  const root = ws.root?.rootPath;
  if (!root) {
    fail("请先打开储存空间");
    return;
  }
  const folder = await pickDir("选择工作空间内的文件夹");
  if (folder === null) return;
  // 绝对路径 → rel（归一化比对前缀，切片回原串保大小写，\ → /）
  const norm = (s: string) => s.replace(/\\/g, "/").replace(/\/+$/, "").toLowerCase();
  const r = norm(root);
  const p = norm(folder);
  if (!p.startsWith(`${r}/`)) {
    fail("选中的文件夹不在当前工作空间内");
    return;
  }
  const rel = folder.replace(/\\/g, "/").slice(r.length + 1);
  await addPortal(rel);
}
</script>

<template>
  <BaseModal title="管理工作空间" :width="640" framed class="ws-manage" @close="emit('close')">
    <div class="wm-body">
      <!-- 区块一：链进来（方向 b 门清单） -->
      <div class="wm-sec">
        <SectionTitle variant="block" title="链接进来的文件夹" :count="overview.links.length" expanded :actions="false" />
        <div v-if="overview.links.length" class="wm-cards">
          <div v-for="l in overview.links" :key="l.rel" class="wm-card ni-hover" :class="{ dim: !l.ok }">
            <AppIcon name="linked-file-badge" :size="18" class="wm-lead" />
            <div class="wm-text">
              <span class="wm-name">
                {{ baseName(l.rel) }}<TwoToneIcon v-if="!l.ok" name="warning-small" :size="14" class="wm-warn" />
              </span>
              <span class="wm-path" :title="l.target">← {{ l.target }}</span>
            </div>
            <span class="ni-slide"><span class="ni-slide-inner">
              <button class="wm-act" title="重新定位…" @click="void retarget(l.rel)"><AppIcon name="link-small" :size="18" /></button>
              <button class="wm-act" title="在资源管理器中显示" @click="reveal(l.target)"><AppIcon name="open-in-explorer" :size="18" /></button>
              <button class="wm-act" title="删除链接（只删门进回收站）" @click="removeLink(l.rel, baseName(l.rel))"><AppIcon name="trash" :size="18" /></button>
            </span></span>
          </div>
        </div>
        <p v-else class="wm-empty">还没有链接进来的文件夹——右键工作空间空白处或文件夹，选「添加链接文件夹…」</p>
        <ActionButton label="添加链接文件夹…" icon="add-small" variant="solid" size="md" title="选外部文件夹，挂载到工作空间根目录" @click="void addLink()" />
      </div>
      <!-- 区块二：链出去（方向 a′ 入口台账）；顶部分隔线 + 大间距区分功能 -->
      <div class="wm-sec sep">
        <SectionTitle variant="block" title="链出去的文件夹" :count="overview.portals.length" expanded :actions="false" />
        <div v-if="overview.portals.length" class="wm-cards">
          <div v-for="g in overview.portals" :key="g.rel" class="wm-gcard">
            <div class="wm-ghead" :class="{ dim: !g.relOk }">
              <TwoToneIcon name="folder-closed" :size="18" />
              <span class="wm-name">
                {{ baseName(g.rel) }}<TwoToneIcon v-if="!g.relOk" name="warning-small" :size="14" class="wm-warn" />
              </span>
              <span class="ni-slide"><span class="ni-slide-inner">
                <button class="wm-act" title="添加入口路径…" @click="void addPortal(g.rel)"><AppIcon name="add-small" :size="18" /></button>
              </span></span>
            </div>
            <div v-for="p in g.portals" :key="p.path" class="wm-sub ni-hover">
              <AppIcon name="link-small" :size="14" class="wm-sublead" />
              <span class="wm-subpath" :class="{ dim: !p.ok }" :title="p.path">
                {{ p.path }}<TwoToneIcon v-if="!p.ok" name="warning-small" :size="14" class="wm-warn" />
              </span>
              <span class="ni-slide"><span class="ni-slide-inner">
                <button v-if="p.ok" class="wm-act" title="在资源管理器中显示" @click="reveal(p.path)"><AppIcon name="open-in-explorer" :size="18" /></button>
                <button
                  class="wm-act"
                  :title="p.ok ? '删除入口（门进回收站 + 剔登记）' : '删除记录（入口已不在，仅剔登记）'"
                  @click="removePortal(g.rel, p.path, !p.ok)"
                >
                  <AppIcon name="trash" :size="18" />
                </button>
              </span></span>
            </div>
          </div>
        </div>
        <p v-else class="wm-empty">还没有链出去的文件夹——右键工作空间文件夹，选「在外部位置创建链接…」</p>
        <ActionButton label="链出去新文件夹…" icon="add-small" variant="solid" size="md" title="选工作空间文件夹 → 选入口摆放位置" @click="void linkOutNew()" />
      </div>
    </div>
  </BaseModal>
</template>

<style scoped>
/* 管理工作空间面板（样式 = preview 三轮定稿稿搬入沉淀，口径变更与 PreviewModals 陈列同步）：
   行/组卡片化（g4 底 + 1px g5 + 圆角 8；行卡 hover g5 底 g6 描边——ni-card 升一档）；
   操作组 = ni-slide hover 滑入（items.css 共享类）；两区块 = 父 gap 24 + 1px g5 顶线 + 24 留白 */
.wm-body {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-24);
  padding: 0 var(--n-space-24) var(--n-space-24);
}
/* 区块容器：标题 → 卡组 → 添加钮 */
.wm-sec {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 区块二：顶部分隔线 + 线后 24 留白（父 gap 24 在线前——合计 24+1+24 功能分界） */
.wm-sec.sep {
  border-top: 1px solid var(--n-g5);
  padding-top: var(--n-space-24);
}
.wm-cards {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-8);
}
/* 行卡（链进来门）：g4 底 + 1px g5 + 圆角 8 + padding 12；hover g5 底 g6 描边 */
.wm-card {
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
.wm-card:hover {
  background: var(--n-g5);
  border-color: var(--n-g6);
}
/* 断链 / 文件夹没了 = 整卡置灰 dimmed 0.4 既有档（§5 / §65） */
.wm-card.dim {
  opacity: 0.4;
}
.wm-lead {
  color: var(--n-l2);
  flex: none;
}
/* 双行文本列：名 13/18 一级 / 目标路径 12/16 三级（次要文字档），均截断 + title 全显 */
.wm-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}
.wm-name {
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
.wm-path {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.wm-warn {
  flex: none;
}
/* 操作钮（污染文件 item .act 同款）：18 盒 / 常态四级 hover 二级 */
.wm-act {
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
.wm-act:hover {
  color: var(--n-l2);
}
/* 组卡（链出去文件夹 + 其入口子行）：同卡底但列向；hover 不变底（让位子行 hover 层），仅首行滑入「添加入口」 */
.wm-gcard {
  display: flex;
  flex-direction: column;
  padding: var(--n-space-4) var(--n-space-12);
  box-sizing: border-box;
  background: var(--n-g4);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
}
/* 组卡 hover 仅出首行「添加入口」（ni-slide 直达子选择器——不波及子行各自的滑入组） */
.wm-gcard:hover > .wm-ghead > .ni-slide {
  grid-template-columns: 1fr;
  opacity: 1;
}
/* 组首行：36 高 / 图标 18 + 名 13/18 一级 + 右端滑入添加钮；底部 1px g5 与子行分隔 */
.wm-ghead {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  height: 36px;
  box-sizing: border-box;
  border-bottom: 1px solid var(--n-g5);
}
.wm-ghead.dim {
  opacity: 0.4;
}
.wm-ghead .wm-name {
  flex: 1;
  min-width: 0;
}
/* 入口子行：30 高 / 缩进 26（18 图标位 + gap 8 对齐组首行名）/ 链接图标 14 三级 +
   路径 12/16 二级截断；hover = g5 底圆角 6 小底（嵌套层级）+ 操作组滑入 */
.wm-sub {
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
.wm-sub:hover {
  background: var(--n-g5);
}
.wm-sublead {
  color: var(--n-l3);
  flex: none;
}
.wm-subpath {
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
.wm-subpath.dim {
  opacity: 0.4;
}
/* 空态引导行：次要文字 12/16 三级，上下留白 12 */
.wm-empty {
  margin: 0;
  padding: var(--n-space-12) 0;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}
</style>
