<script setup lang="ts">
// 删除两段式询问窗（6.1 C1，2026-07-28 用户定案）：Del / 右键删除先干跑 delete_placements，
// 涉及资源最后一条 Placement 时弹出本窗——「移入回收站」/「保留为未分类」二选一后带
// lastAction 再调；右上角关闭 = 放弃删除（干跑未动数据，直接关即可）。宿主 App.vue。
import { onBeforeUnmount, onMounted } from "vue";
import IconButton from "./IconButton.vue";
import { confirmDelete } from "../lib/placementOps";
import { useSelectionStore } from "../stores/selection";

const selection = useSelectionStore();

function onChoose(action: "recycle" | "uncategorized"): void {
  void confirmDelete(action);
}
function onCancel(): void {
  selection.deletePrompt = null; // 干跑未动数据，放弃即关闭
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
  <div v-if="selection.deletePrompt" class="del-mask">
    <div class="del-card">
      <header class="del-titlebar">
        <h2 class="del-title">删除</h2>
        <IconButton name="window-close" title="取消" @click="onCancel" />
      </header>
      <div class="del-body">
        <p class="del-text">
          选中的内容包含 {{ selection.deletePrompt.lastCount }} 个素材的最后一条放置，删除后这些素材将不再属于任何文件夹，请选择处理方式：
        </p>
      </div>
      <footer class="del-footer">
        <button class="btn secondary" @click="onChoose('uncategorized')">保留为未分类</button>
        <button class="btn primary" @click="onChoose('recycle')">移入回收站</button>
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
