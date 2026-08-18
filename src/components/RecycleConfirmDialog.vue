<script setup lang="ts">
// 回收站彻底删除确认窗（6.1 二轮，2026-07-29）：Del / 右键「彻底删除」「清空回收站」先干跑
// （confirmed=false），needConfirm 时弹出本窗——文案含"被 N 个虚拟片段引用"警示与不可恢复
// 说明，确认后带 confirmed=true 再调（不可恢复，toast 无撤销按钮）；
// 右上角关闭 / 取消 = 放弃（干跑未动数据，直接关即可）。
// 类名沿用 del-*——App.vue 快捷键让位判定按 .del-mask 查询。宿主 App.vue。
import { computed, onBeforeUnmount, onMounted } from "vue";
import IconButton from "./IconButton.vue";
import { confirmRecycleDelete } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";

const selection = useSelectionStore();

const isClear = computed(() => selection.recyclePrompt?.kind === "clear");

// 警示文案：虚拟片段引用数 >0 时前置引用警示；快照链接数 >0 时提示链接一并清除
//（2026-08-18 用户定）；尾句恒带不可恢复说明
const bodyText = computed(() => {
  const p = selection.recyclePrompt;
  if (!p) return "";
  const seg =
    p.segmentRefs > 0 ? `，其中素材被 ${p.segmentRefs} 个虚拟片段引用，删除后引用将失效` : "";
  const link = p.linkRefs > 0 ? `，其关联的 ${p.linkRefs} 条链接将一并清除` : "";
  if (isClear.value) {
    return `将清空回收站中的全部素材${seg}${link}。彻底删除不可恢复，确定继续吗？`;
  }
  return `将彻底删除选中的 ${p.assetIds.length} 个素材${seg}${link}。彻底删除不可恢复，确定继续吗？`;
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
  <div v-if="selection.recyclePrompt" class="del-mask">
    <div class="del-card">
      <header class="del-titlebar">
        <h2 class="del-title">{{ isClear ? "清空回收站" : "彻底删除" }}</h2>
        <IconButton name="window-close" title="取消" @click="onCancel" />
      </header>
      <div class="del-body">
        <p class="del-text">{{ bodyText }}</p>
      </div>
      <footer class="del-footer">
        <button class="btn primary" @click="onConfirm">
          {{ isClear ? "清空回收站" : "彻底删除" }}
        </button>
        <button class="btn secondary" @click="onCancel">取消</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
/* 遮罩（同 DuplicateDialog / LinkDialog）：C1-80 */
.del-mask {
  position: absolute;
  inset: 0;
  z-index: 200;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--c1-80);
}
/* 总容器：C3 / 1px border-1（inset）/ 圆角 8 / --shadow-pop */
.del-card {
  box-sizing: border-box;
  width: 420px;
  display: flex;
  flex-direction: column;
  background: var(--c3);
  box-shadow:
    var(--shadow-pop),
    inset 0 0 0 1px var(--border-1);
  border-radius: var(--radius-8);
  overflow: hidden;
}
/* 标题栏：高 44 / 下分割线 border-1（inset） */
.del-titlebar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: var(--space-8) var(--space-8) var(--space-8) var(--space-16);
  height: 44px;
  flex: none;
  box-shadow: inset 0 -1px 0 0 var(--border-1);
}
.del-title {
  margin: 0;
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.del-body {
  padding: var(--space-16) var(--space-32);
}
.del-text {
  margin: 0;
  font-size: var(--text-body-size);
  line-height: var(--text-body-line);
  color: var(--font-icon-2);
}
/* 底栏：高 70 / 上分割线 border-1（inset） */
.del-footer {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-16);
  padding: 0 var(--space-32);
  height: 70px;
  flex: none;
  box-shadow: inset 0 1px 0 0 var(--border-1);
}
/* 按钮（01_按钮#07 大标题按钮-小按钮；主要 / 次要） */
.btn {
  display: flex;
  justify-content: center;
  align-items: center;
  padding: var(--space-8) var(--space-16);
  gap: var(--space-6);
  height: 38px;
  border: none;
  border-radius: var(--radius-6);
  cursor: pointer;
  font-family: var(--font-family);
  font-size: var(--text-title-size);
  line-height: var(--text-title-line);
  font-weight: var(--text-title-weight);
  color: var(--font-icon-1);
}
.btn.primary {
  background: var(--brand-1);
}
.btn.primary:hover {
  background: var(--brand-2);
}
.btn.secondary {
  background: var(--c5);
}
.btn.secondary:hover {
  background: var(--border-1);
}
</style>
