<script setup lang="ts">
// 回收站彻底删除确认窗（6.1 二轮，2026-07-29）：Del / 右键「彻底删除」「清空回收站」先干跑
// （confirmed=false），needConfirm 时弹出本窗——文案含"被 N 个虚拟片段引用"警示与不可恢复
// 说明，确认后带 confirmed=true 再调（不可恢复，toast 无撤销按钮）；取消 = 放弃（干跑未动数据）。
// 2026-09-21 换代 modals/ConfirmModal danger（用户指「不是新的弹窗样式」）：手写 del-* 壳 /
// 手写按钮 / absolute C1-80 遮罩全废——壳与遮罩随组件层（kit-mask 黑色-50%）；关闭 X 废
//（ConfirmModal 口径：关闭只走取消/确认），Esc 放弃保留；危险确认钮 = danger 红
//（旧品牌蓝主左次右废，次左主右）；警示数量词高亮 = 加粗 + 亮色/二级（§24 pw-lines b 口径）。
// 快捷键让位：useGlobalKeys 遮罩查询已随换代改 .kit-mask/.bm-mask（原 .del-mask 死类名）。
import { computed, onBeforeUnmount, onMounted } from "vue";
import ConfirmModal from "./modals/ConfirmModal.vue";
import { confirmRecycleDelete } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";

const selection = useSelectionStore();

const isClear = computed(() => selection.recyclePrompt?.kind === "clear");
const title = computed(() => (isClear.value ? "清空回收站" : "彻底删除"));

// 警示文案分段（strong = 数量词加粗高亮）：虚拟片段引用数 >0 时前置引用警示；
// 快照链接数 >0 时提示链接一并清除（2026-08-18 用户定）；尾句恒带不可恢复说明
const bodySegments = computed<{ t: string; strong?: boolean }[]>(() => {
  const p = selection.recyclePrompt;
  if (!p) return [];
  const segs: { t: string; strong?: boolean }[] = isClear.value
    ? [{ t: "将清空回收站中的全部素材" }]
    : [{ t: "将彻底删除选中的 " }, { t: `${p.assetIds.length}`, strong: true }, { t: " 个素材" }];
  if (p.segmentRefs > 0)
    segs.push(
      { t: "，其中素材被 " },
      { t: `${p.segmentRefs}`, strong: true },
      { t: " 个虚拟片段引用，删除后引用将失效" },
    );
  if (p.linkRefs > 0)
    segs.push({ t: "，其关联的 " }, { t: `${p.linkRefs}`, strong: true }, { t: " 条链接将一并清除" });
  segs.push({ t: "。彻底删除不可恢复，确定继续吗？" });
  return segs;
});

function onConfirm(): void {
  void confirmRecycleDelete();
}
function onCancel(): void {
  selection.recyclePrompt = null; // 干跑未动数据，放弃即关闭
}

function onKey(e: KeyboardEvent): void {
  if (e.key === "Escape") {
    e.stopPropagation();
    onCancel();
  }
}
onMounted(() => window.addEventListener("keydown", onKey, true));
onBeforeUnmount(() => window.removeEventListener("keydown", onKey, true));
</script>

<template>
  <ConfirmModal
    v-if="selection.recyclePrompt"
    kind="danger"
    :title="title"
    :confirm-label="title"
    cancel-label="取消"
    @confirm="onConfirm"
    @cancel="onCancel"
  >
    <p class="del-text">
      <template v-for="(s, i) in bodySegments" :key="i"><b v-if="s.strong">{{ s.t }}</b
      ><template v-else>{{ s.t }}</template></template>
    </p>
  </ConfirmModal>
</template>

<style scoped>
.del-text {
  margin: 0;
}
/* 数量词高亮 = 加粗 + 亮色/二级（§24 pw-lines b 口径） */
.del-text b {
  font-weight: var(--n-fw-bold);
  color: var(--n-l2);
}
</style>
