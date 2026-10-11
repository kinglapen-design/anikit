<script setup lang="ts">
// 批量重命名弹窗（37 稿，3.4，2026-08-18）：双列对照（原始文件名 / 新的文件名）+ 底部
// 模式切换（SegmentedControl：序号命名 / 替换命名——同一时刻只显示当前模式的一行输入，PRD 02-14）。
// 实时预览只读无"应用"钮：序号 = 名称_编号（名称 + `_` + 起始编号，2026-09-23 序号格式统一 `_数字`；
// 位宽按选中数 <100 两位 / ≥100 三位）；
// 替换 = 查找文本全处替换，原名称匹配段品牌色高亮（Eagle 交互参考：视觉上把高亮部分换成新的）。
// 规则只作用词干、扩展名不动；撞名不拦截（后端 rename_placements 不唯一化）；
// 失败项跳过 toast 汇总不整体回滚；无变化项不提交（名称/查找为空 → 该项保持原名）。
// 文件夹模式（2026-10-11，PRD 02-14 文件夹批量重命名）：renameFolderDraft（folderId 树 DFS 视觉序）
// 驱动——仅序号命名（无分段切换 / 无替换模式），无词干拆分（文件夹无扩展名），缩略位换 TwoToneIcon
// 文件夹图标（自定义图标 / 档案 archive / 颜色 tint 同树行口径）；两草稿与两选择域同口径天然互斥。
// 2026-09-20 R2 旧件烧减：壳 = modals/BaseModal framed（973 宽挪 width prop；标题栏 54 带底线 +
// 关闭 X 由 framed 提供——旧 :deep(.app-modal)/.titlebar/.content 三段覆盖尽废）；
// 输入框 = inputs/TextInput lg（clearable 新件原生，旧 medium 档废）；按钮 = buttons/ActionButton lg
//（模态口径：宽自适应 + 左右 48——旧 88 定宽废；次左主右——旧确认左序随新口径废）。
import { computed, nextTick, onMounted, ref } from "vue";
import { convertFileSrc } from "@tauri-apps/api/core";
import BaseModal from "./modals/BaseModal.vue";
import TextInput from "./inputs/TextInput.vue";
import ActionButton from "./buttons/ActionButton.vue";
import SegmentedControl from "./buttons/SegmentedControl.vue";
import ScrollThumb from "./ScrollThumb.vue";
import TwoToneIcon from "./items/TwoToneIcon.vue";
import { placementKey, useContentStore, type FolderNode } from "../stores/content";
import { useSelectionStore, type PlacementRef } from "../stores/selection";
import { renameFoldersBatch, renamePlacementsBatch, type RenameFolderItemInput, type RenameItemInput } from "../lib/placementOps";
import { toNewTint } from "../lib/customColors";

const content = useContentStore();
const selection = useSelectionStore();

// —— 模式与行数据 ——
// folderMode：renameFolderDraft 非空 = 文件夹批量重命名（2026-10-11）；renameDraft = 素材。
// 行数据（draft = 打开时刻选中集快照；素材名称/缩略图读 content.placements 实况，
// 文件夹名称/图标/颜色读 content.folders 实况）
const folderMode = computed(() => selection.renameFolderDraft !== null);
interface Row {
  key: string; // v-for 键（素材 = placementKey；文件夹 = folderId）
  ref: PlacementRef | null; // 素材行定位；文件夹行 = null（folderId 即 key）
  name: string;
  thumb: string | null;
  contain: boolean; // 格式图标 contain 不裁切（速查表 §6）
  ext: string;
  // —— 文件夹行（素材行恒缺省）——
  icon: string; // 自定义图标名（空 = 默认 folder-closed）；档案 = "archive"
  tint: string; // 自定义颜色 tint（toNewTint 转换；空 = 常态双色）
}
const rows = computed<Row[]>(() => {
  if (folderMode.value) {
    const byId = new Map<string, FolderNode>();
    const walk = (nodes: FolderNode[]): void => {
      for (const n of nodes) {
        byId.set(n.id, n);
        walk(n.children);
      }
    };
    walk(content.folders);
    return (selection.renameFolderDraft ?? []).map((id) => {
      const n = byId.get(id);
      return {
        key: id,
        ref: null,
        name: n?.name ?? "",
        thumb: null,
        contain: false,
        ext: "",
        icon: n?.kind === "archive" ? "archive" : (n?.icon ?? ""),
        tint: toNewTint(n?.color ?? undefined),
      };
    });
  }
  const draft = selection.renameDraft ?? [];
  // 撞键行（同资产同文件夹多条位置）按 placementKey 各归各行（2026-10-11 选择域独立化——
  // 原三元组映射把同 origin 重复链接全映射到首行）
  const byKey = new Map(content.placements.map((p) => [placementKey(p), p]));
  return draft.map((ref) => {
    const p = byKey.get(placementKey(ref));
    return {
      key: placementKey(ref),
      ref,
      name: p?.name ?? "",
      thumb: p?.thumbPath ? convertFileSrc(p.thumbPath) : null,
      contain: p?.thumbKind === "formatIcon",
      ext: p?.ext ?? "",
      icon: "",
      tint: "",
    };
  });
});

// —— 模式与输入（segmented 切换，单显一行；起始编号非数字回退 1）——
const MODES = [
  { value: "sequence", label: "序号命名" },
  { value: "replace", label: "替换命名" },
] as const;
const mode = ref<string>("sequence");
const seqName = ref("");
const seqStart = ref("1");
const findText = ref("");
const replaceText = ref("");

// TAB 循环聚焦（2026-09-13 实机走查：输入一个后手不离键盘——查找↔替换、名称↔起始编号互跳）
const seqNameRef = ref<InstanceType<typeof TextInput>>();
const seqStartRef = ref<InstanceType<typeof TextInput>>();
const findRef = ref<InstanceType<typeof TextInput>>();
const replaceRef = ref<InstanceType<typeof TextInput>>();

// 打开即聚焦首输入框（2026-10-11 用户要求：免每次手点；默认序号模式聚焦「名称」）
onMounted(() => {
  void nextTick(() => seqNameRef.value?.focus());
});

// 位宽自动（PRD 02-14：<100 两位、≥100 三位）
const padWidth = computed(() => (rows.value.length < 100 ? 2 : 3));
const startNum = computed(() => {
  const n = parseInt(seqStart.value, 10);
  return Number.isFinite(n) && n >= 0 ? n : 1;
});

// 词干/扩展名拆分（与后端 unique_name 同规则：末点为界、无扩展名或点为首不拆）
function splitName(name: string): { stem: string; ext: string } {
  const i = name.lastIndexOf(".");
  return i > 0 ? { stem: name.slice(0, i), ext: name.slice(i) } : { stem: name, ext: "" };
}

// 新词干（null = 该项不改名：序号模式名称为空 / 替换模式查找为空或无匹配）
function newStem(index: number, stem: string): string | null {
  if (mode.value === "sequence") {
    const base = seqName.value.trim();
    if (!base) return null;
    return `${base}_${String(startNum.value + index).padStart(padWidth.value, "0")}`;
  }
  if (!findText.value || !stem.includes(findText.value)) return null;
  return stem.split(findText.value).join(replaceText.value);
}
function previewName(row: Row, index: number): string {
  // 文件夹模式：无词干拆分（文件夹无扩展名），仅序号命名（mode 恒 sequence，分段切换隐藏）
  if (folderMode.value) {
    const base = seqName.value.trim();
    return base ? `${base}_${String(startNum.value + index).padStart(padWidth.value, "0")}` : row.name;
  }
  const { stem, ext } = splitName(row.name);
  const ns = newStem(index, stem);
  return ns === null ? row.name : ns + ext;
}

// 原名称高亮分段（替换模式：匹配段品牌色高亮；其余模式/无匹配 = 整段不高亮）
function stemSegments(stem: string): { text: string; hit: boolean }[] {
  const f = findText.value;
  if (mode.value !== "replace" || !f) return [{ text: stem, hit: false }];
  const out: { text: string; hit: boolean }[] = [];
  let rest = stem;
  for (;;) {
    const i = rest.indexOf(f);
    if (i < 0) {
      if (rest) out.push({ text: rest, hit: false });
      break;
    }
    if (i > 0) out.push({ text: rest.slice(0, i), hit: false });
    out.push({ text: f, hit: true });
    rest = rest.slice(i + f.length);
  }
  return out;
}

// 可执行项（有变化才提交）；全不变更确认钮禁用
const pendingPlacementItems = computed<RenameItemInput[]>(() => {
  if (folderMode.value) return [];
  return rows.value.flatMap((row, i) => {
    if (!row.ref || newStem(i, splitName(row.name).stem) === null) return [];
    return [
      {
        assetId: row.ref.assetId,
        folderId: row.ref.folderId,
        origin: row.ref.origin,
        newName: previewName(row, i),
        oldName: row.ref.name, // 撞键行附旧名精确到行（后端 RenameItem oldName，2026-10-11）
      },
    ];
  });
});
// 文件夹模式可执行项（2026-10-11）：folderId + 新名；序号名称为空 = 保持原名不提交
const pendingFolderItems = computed<RenameFolderItemInput[]>(() => {
  if (!folderMode.value || !seqName.value.trim()) return [];
  return rows.value.map((row, i) => ({ folderId: row.key, newName: previewName(row, i) }));
});
const pendingCount = computed(() => (folderMode.value ? pendingFolderItems.value.length : pendingPlacementItems.value.length));

const busy = ref(false);
async function onConfirm(): Promise<void> {
  if (pendingCount.value === 0 || busy.value) return;
  busy.value = true;
  const ok = folderMode.value
    ? await renameFoldersBatch(pendingFolderItems.value)
    : await renamePlacementsBatch(pendingPlacementItems.value);
  busy.value = false;
  if (ok) {
    selection.renameDraft = null; // 部分失败也关（toast 已汇总，撤销入口在 toast）
    selection.renameFolderDraft = null;
  }
}
function onClose(): void {
  selection.renameDraft = null;
  selection.renameFolderDraft = null;
}

const listRef = ref<HTMLElement>();
</script>

<template>
  <BaseModal v-if="selection.renameDraft || selection.renameFolderDraft" title="批量重命名" :width="973" framed class="rename-dialog" @close="onClose">
    <!-- 表头（37 稿：双列标题 / 下分隔线 / 列 gap 24；文件夹模式无扩展名语义，列名从宽） -->
    <header class="rd-head">
      <span class="col">{{ folderMode ? "原名称" : "原始文件名" }}</span>
      <span class="col">{{ folderMode ? "新名称" : "新的文件名" }}</span>
    </header>
    <!-- 行列表（滚轮 = ScrollThumb 悬浮件） -->
    <div class="rd-list-wrap">
      <div ref="listRef" class="rd-list kit-scroll">
        <div v-for="(row, i) in rows" :key="row.key" class="rd-row">
          <div class="rd-card">
            <span class="rd-thumb" :class="{ folder: folderMode }">
              <!-- 文件夹模式：文件夹图标（自定义 / 档案 / tint 同树行口径，2026-10-11） -->
              <TwoToneIcon v-if="folderMode" :name="row.icon || 'folder-closed'" :size="24" :tint="row.tint" />
              <template v-else>
                <img v-if="row.thumb" :src="row.thumb" :class="{ icon: row.contain }" alt="" />
                <span v-else class="ph">{{ row.ext || "—" }}</span>
              </template>
            </span>
            <span class="rd-name" :title="row.name">
            <template v-for="(seg, j) in stemSegments(splitName(row.name).stem)" :key="j"
              ><span v-if="seg.hit" class="hit">{{ seg.text }}</span
              ><template v-else>{{ seg.text }}</template></template
            ><span class="ext">{{ splitName(row.name).ext }}</span>
            </span>
          </div>
          <div class="rd-card">
            <span class="rd-thumb" :class="{ folder: folderMode }">
              <TwoToneIcon v-if="folderMode" :name="row.icon || 'folder-closed'" :size="24" :tint="row.tint" />
              <template v-else>
                <img v-if="row.thumb" :src="row.thumb" :class="{ icon: row.contain }" alt="" />
                <span v-else class="ph">{{ row.ext || "—" }}</span>
              </template>
            </span>
            <span class="rd-name" :title="previewName(row, i)">{{ previewName(row, i) }}</span>
          </div>
        </div>
      </div>
      <ScrollThumb :body="listRef" />
    </div>
    <!-- 输入区（同一时刻只显示当前模式一行；PRD 02-14；TAB 两框互跳——2026-09-13 实机走查） -->
    <div class="rd-fields">
      <template v-if="mode === 'sequence'">
        <span class="lbl">名称:</span>
        <TextInput ref="seqNameRef" v-model="seqName" size="lg" clearable class="grow" placeholder="输入名称" @keydown.tab.prevent="seqStartRef?.focus()" />
        <span class="lbl">起始编号:</span>
        <TextInput ref="seqStartRef" v-model="seqStart" size="lg" clearable class="narrow" placeholder="1" @keydown.tab.prevent="seqNameRef?.focus()" />
      </template>
      <template v-else>
        <span class="lbl">查找文本:</span>
        <TextInput ref="findRef" v-model="findText" size="lg" clearable class="grow" placeholder="输入查找文本" @keydown.tab.prevent="replaceRef?.focus()" />
        <span class="lbl">替换为:</span>
        <TextInput ref="replaceRef" v-model="replaceText" size="lg" clearable class="grow" placeholder="输入替换文本" @keydown.tab.prevent="findRef?.focus()" />
      </template>
    </div>
    <!-- 底部栏（37 稿：62 高 / padding 12 12 12 24 / 顶部分隔线；左 segmented 右按钮组——
         按钮 lg 宽自适应 + 左右 48、次左主右，2026-09-13 定稿口径；
         分段 tone g2——g3 轨道贴 g3 壳隐身，2026-09-21 用户报修） -->
    <footer class="rd-foot">
      <!-- 文件夹模式仅序号命名（2026-10-11 用户定「_[序号] 方式命名」），分段切换不渲染 -->
      <SegmentedControl v-if="!folderMode" v-model="mode" :options="MODES" variant="text" tone="g2" style="width: 160px" />
      <span class="spacer" />
      <ActionButton variant="cancel" label="取消" size="lg" @click="onClose" />
      <ActionButton variant="primary" label="确认" size="lg" :disabled="pendingCount === 0 || busy" @click="onConfirm" />
    </footer>
  </BaseModal>
</template>

<style scoped>
/* 37 稿壳：宽 973 挪 BaseModal width prop（旧 = 970 内容宽 + 真 border 3 补偿，速查表 §1）；
   高 750 稿值 + 窗口封顶；framed 壳 padding 0 天然贴边（各区自负边距），gap 归零——
   稿相邻区紧贴：表头线 / 行线 / 按钮栏线各自承担 */
.rename-dialog :deep(.bm-modal) {
  height: min(753px, calc(100vh - 96px));
  max-width: calc(100vw - 96px); /* 旧 AppModal 视口封顶口径沿（BaseModal 不带宽度封顶） */
  gap: 0;
}
/* 表头（37 稿：46 高 / 0 24 容器内 / 下分隔线；双列 flex 1 对齐行内双卡）；
   写法说明 §6 不用 margin：内缩由 padding 承担，分隔线走 ::after 内缩 24（§8 描边 ::after 工程路径） */
.rd-head {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--n-space-24);
  height: 46px;
  padding: 0 var(--n-space-24);
  flex: none;
}
.rd-head::after {
  content: "";
  position: absolute;
  left: var(--n-space-24);
  right: var(--n-space-24);
  bottom: 0;
  border-bottom: 1px solid var(--n-g5);
}
/* 列表表头稿口径：列标签 = 正文-加粗 亮色/三级（cards/ListHeader 同款） */
.rd-head .col {
  flex: 1;
  min-width: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  font-weight: var(--n-fw-bold);
  color: var(--n-l3);
}
/* 行列表：0 24 容器（稿 922 内容宽）；行 = padding 24 0 + 通用/二级底 + 下边框（稿 item 117 高） */
.rd-list-wrap {
  position: relative;
  flex: 1;
  min-height: 0;
}
.rd-list {
  height: 100%;
  overflow-y: auto;
  padding: 0 var(--n-space-24);
}
.rd-row {
  display: flex;
  gap: var(--n-space-24);
  padding: var(--n-space-24) 0;
  background: var(--n-g2);
  border-bottom: 1px solid var(--n-g5);
}
/* 名称卡（37 稿：449×69 / 通用/四级底 / 1px 通用/五级 border / 圆角 8 / padding 12 / gap 8） */
.rd-card {
  display: flex;
  align-items: center;
  gap: var(--n-space-8);
  flex: 1;
  min-width: 0;
  height: 69px;
  padding: var(--n-space-12);
  background: var(--n-g4);
  border: 1px solid var(--n-g5);
  border-radius: var(--n-radius-8);
}
.rd-thumb {
  width: 80px;
  height: 45px;
  border-radius: var(--n-radius-6);
  overflow: hidden;
  flex: none;
}
.rd-thumb img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.rd-thumb img.icon {
  object-fit: contain;
}
/* 文件夹模式缩略位（2026-10-11）：TwoToneIcon 24 档居中（文件夹卡片顶行图标同档），无底块 */
.rd-thumb.folder {
  display: flex;
  align-items: center;
  justify-content: center;
}
/* 无图占位（速查表 §6）：浅色块 #C6C7CC + 扩展名文字 */
.rd-thumb .ph {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  background: #c6c7cc;
  font-size: var(--n-fs-secondary);
  font-weight: var(--n-fw-bold);
  color: var(--n-g3);
}
.rd-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l1);
}
/* 替换模式匹配段：品牌色高亮（Eagle 交互参考——原名称被替换部分一眼可辨） */
.rd-name .hit {
  color: var(--n-brand);
}
/* 输入区（稿：label 标签色；输入框 = TextInput lg 34 高——新件稿值，旧 medium 提权覆盖废） */
.rd-fields {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  padding: var(--n-space-12) var(--n-space-24);
  flex: none;
}
.rd-fields .lbl {
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
  flex: none;
  white-space: nowrap;
}
.rd-fields .grow {
  flex: 1;
  min-width: 0;
}
.rd-fields .narrow {
  width: 82px;
  flex: none;
}
/* 底部栏（37 稿：62 高 / padding 12 12 12 24 / 顶部分隔线通栏） */
.rd-foot {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
  height: 62px;
  padding: var(--n-space-12) var(--n-space-12) var(--n-space-12) var(--n-space-24);
  border-top: 1px solid var(--n-g5);
  flex: none;
}
.rd-foot .spacer {
  flex: 1;
}
</style>
