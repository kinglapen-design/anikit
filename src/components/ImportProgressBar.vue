<script setup lang="ts">
// 导入 / 导出进度条（03_UI 布局/11_导入进度条）：插在中间面板标题栏与卡片容器之间。
// 构造：进度条（胶囊 6px，占剩余空间）+ 标签（正在导入/导出文件 x/y）+ 预计剩余 + 关闭按钮。
// 关闭 = 取消：进度条立即隐藏，后台把当前文件处理完后真实停止（cancel 语义，见 11 文档注）。
// 2026-08-14 用户定：导出复用本组件样式与位置——导入优先（同跑时先显导入），
// 导出总数 = 文件数（序列帧按帧计），清点期（total=0）不定态同"扫描中"。
// 2026-10-03 工作空间搜索轮：爬盘回退进度复用本组件（用户定「参考导入文件的那个进度条」）——
// 无导入导出且本 panel 爬盘进行中时显示：总数未知恒不定态（扫描中滑块），标签「已扫 N 个文件」，
// 关闭 = 取消爬盘（保留已命中的部分结果）；导入 / 导出优先（同跑先显导入导出）。
import { computed, onMounted } from "vue";
import IconButton from "./buttons/IconButton.vue";
import { useContentStore } from "../stores/content";
import { useSearchStore } from "../stores/search";
import { useSemanticTaskStore } from "../stores/semantic";

const props = withDefaults(defineProps<{ panelId?: string }>(), { panelId: undefined });
const content = useContentStore();
const search = useSearchStore();
const semanticTasks = useSemanticTaskStore();
onMounted(() => void semanticTasks.init());
const isExport = computed(() => !content.importProgress && !!content.exportProgress);
const p = computed(() => content.importProgress ?? content.exportProgress);
const semantic = computed(() => {
  const task = semanticTasks.active;
  return !p.value && task && ["running", "paused"].includes(task.status) ? task : null;
});
// 爬盘态（ws 搜索回退层）：本 panel 进行中爬盘；导入 / 导出优先
const crawl = computed(() => (props.panelId ? search.crawlActiveFor(props.panelId) : null));
const crawling = computed(() => !p.value && !semantic.value && crawl.value !== null);

const scanning = computed(() => semantic.value ? semantic.value.total === 0 : p.value?.total === 0);
const ratio = computed(() => {
  if (semantic.value) return semantic.value.total > 0 ? Math.min(1, semantic.value.completed / semantic.value.total) : 0;
  if (!p.value || p.value.total === 0) return 0;
  return Math.min(1, p.value.done / p.value.total);
});
const label = computed(() => {
  if (semantic.value) {
    const task = semantic.value;
    const prefix = task.status === "paused" ? "已暂停" : "正在";
    return task.total === 0 ? `${prefix}准备语义模型…` : `${prefix}语义分析 (${task.completed} / ${task.total})`;
  }
  if (crawling.value) return `正在搜索（已扫 ${crawl.value!.scanned.toLocaleString()} 个文件）`;
  if (!p.value) return "";
  if (p.value.total === 0) return isExport.value ? "正在清点文件…" : "正在扫描文件…";
  return isExport.value
    ? `正在导出文件 (${p.value.done} / ${p.value.total})`
    : `正在导入文件 (${p.value.done} / ${p.value.total})`;
});
// 预计剩余：按已处理速率估算（done 变化即重算）；爬盘总数未知不出 ETA
const eta = computed(() => {
  if (semantic.value || crawling.value) return "";
  if (!p.value || p.value.done === 0 || p.value.total === 0) return "";
  const elapsed = (Date.now() - p.value.startedAt) / 1000;
  if (elapsed <= 0) return "";
  const remain = ((p.value.total - p.value.done) / p.value.done) * elapsed;
  if (remain < 60) return `预计剩余约 ${Math.max(1, Math.ceil(remain))} 秒`;
  if (remain < 3600) return `预计剩余约 ${Math.ceil(remain / 60)} 分钟`;
  return `预计剩余约 ${Math.floor(remain / 3600)} 小时`;
});

function cancel(): void {
  if (semantic.value) {
    void semanticTasks.cancelActive();
    return;
  }
  if (crawling.value && props.panelId) {
    search.cancelCrawlFor(props.panelId);
    return;
  }
  if (isExport.value) void content.cancelExport();
  else void content.cancelImport();
}
</script>

<template>
  <div v-if="p || crawling" class="import-progress">
    <div class="bar" :class="{ scanning: scanning || crawling }">
      <div class="fill" :style="scanning ? undefined : { width: `${ratio * 100}%` }" />
    </div>
    <span class="label">{{ label }}</span>
    <span v-if="eta" class="eta">{{ eta }}</span>
    <IconButton
      name="close-small"
      variant="plain"
      :title="semantic ? '取消语义分析' : crawling ? '取消搜索' : isExport ? '取消导出' : '取消导入'"
      @click="cancel"
    />
  </div>
</template>

<style scoped>
/* 容器（11）：padding 12 16（与中间面板工具栏同 padding 对齐，2026-08-10 用户走查）/
   无底无边 / 子项间距 6 / 高 = 关闭按钮 28 + 24；
   2026-09-13 实机走查：全件旧 token 换代（填充 = 品牌色-常态 --n-brand，关闭钮换代 buttons/IconButton） */
.import-progress {
  display: flex;
  align-items: center;
  gap: var(--n-space-6);
  padding: var(--n-space-12) var(--n-space-16);
  box-sizing: border-box;
  height: 52px;
  flex: none;
}
/* 进度条（10_进度条）：高 6 胶囊 / 轨道 亮色/半透明-2（旧 border-1 视觉等效 14% 白）/
   填充 品牌色-常态（2026-09-13 实机走查定），占剩余空间 */
.bar {
  position: relative;
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--n-l-a2);
  overflow: hidden;
}
.fill {
  height: 100%;
  border-radius: 3px;
  background: var(--n-brand);
  transition: width 0.2s ease;
}
/* 扫描中（总数未知）：品牌色块往返滑动不定态 */
.bar.scanning .fill {
  position: absolute;
  width: 30%;
  animation: slide 1.2s ease-in-out infinite alternate;
}
@keyframes slide {
  from {
    left: 0;
  }
  to {
    left: 70%;
  }
}
/* 标签与预计时间：次要文字档 亮色/三级 */
.label,
.eta {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
  white-space: nowrap;
  flex: none;
}
</style>
