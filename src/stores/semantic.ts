import { ref } from "vue";
import { defineStore } from "pinia";
import { listen } from "@tauri-apps/api/event";
import {
  cancelSemanticAnalysis,
  getSemanticAnalysisTask,
  pauseSemanticAnalysis,
  resumeSemanticAnalysis,
  retrySemanticAnalysis,
  startSemanticAnalysis,
  startDetailedSemanticAnalysis,
  createSemanticSegment,
  type AnalysisProgress,
  type AnalysisTask,
} from "../lib/semantic";
import { useToastStore } from "./toast";
import { useSearchStore } from "./search";
import { useContentStore } from "./content";
import { useTabsStore } from "./tabs";

export const useSemanticTaskStore = defineStore("semanticTasks", () => {
  const active = ref<AnalysisProgress | null>(null);
  const last = ref<AnalysisTask | null>(null);
  const startedAt = ref(0);
  const listening = ref(false);

  async function init(): Promise<void> {
    if (listening.value) return;
    listening.value = true;
    await listen<AnalysisProgress>("semantic-analysis-progress", (event) => {
      active.value = event.payload;
      if (["done", "cancelled", "failed"].includes(event.payload.status)) {
        void finish(event.payload.taskId, event.payload.status);
      }
    });
  }

  async function finish(taskId: string, status: string): Promise<void> {
    const snapshot = await getSemanticAnalysisTask(taskId);
    if (!snapshot) return;
    last.value = snapshot;
    if (status === "done") {
      const tabs = useTabsStore();
      await useSearchStore().refreshSemanticSearches((panelId) => tabs.viewKeyOf(panelId));
    }
    const toast = useToastStore();
    if (status === "done" && snapshot.errors.length === 0) toast.notify({ message: "语义分析完成" });
    else if (status === "done") toast.notify({ message: `语义分析完成，${snapshot.errors.length} 个素材失败`, icon: "fail" });
    else if (status === "cancelled") toast.notify({ message: "语义分析已取消" });
    else toast.notify({ message: "语义分析中断，可稍后重试", icon: "fail" });
  }

  async function begin(assetIds: string[], detailed: boolean): Promise<string | null> {
    await init();
    try {
      const taskId = await (detailed ? startDetailedSemanticAnalysis(assetIds) : startSemanticAnalysis(assetIds));
      active.value = {
        taskId, status: "running", currentAssetId: null, completed: 0, total: assetIds.length,
        actualBackend: "unknown", error: null,
      };
      startedAt.value = Date.now();
      return taskId;
    } catch (error) {
      useToastStore().notify({ message: error instanceof Error ? error.message : String(error), icon: "fail" });
      return null;
    }
  }

  async function start(assetIds: string[]): Promise<string | null> {
    return begin(assetIds, false);
  }

  async function startDetailed(assetIds: string[]): Promise<string | null> {
    return begin(assetIds, true);
  }

  async function cancelActive(): Promise<void> {
    if (active.value) await cancelSemanticAnalysis(active.value.taskId);
  }

  async function pauseActive(): Promise<void> {
    if (active.value) await pauseSemanticAnalysis(active.value.taskId);
  }

  async function resumeActive(): Promise<void> {
    if (!active.value) return;
    const taskId = await resumeSemanticAnalysis(active.value.taskId);
    if (active.value) active.value.taskId = taskId;
  }

  async function retryLast(): Promise<string | null> {
    const taskId = last.value?.id ?? active.value?.taskId;
    if (!taskId) return null;
    const nextId = await retrySemanticAnalysis(taskId);
    if (active.value) active.value.taskId = nextId;
    return nextId;
  }

  async function create(assetId: string, physical: boolean, panelId: string): Promise<void> {
    const search = useSearchStore();
    await search.runNow(panelId, useTabsStore().viewKeyOf(panelId));
    const hit = search.semanticHitFor(panelId, assetId)?.best;
    const toast = useToastStore();
    if (!hit) {
      toast.notify({ message: "请先在语义搜索中打开该素材，再创建命中片段", icon: "fail" });
      return;
    }
    try {
      await createSemanticSegment(assetId, hit.segmentId, physical);
      await useContentStore().refresh();
      toast.notify({ message: physical ? "物理片段已创建" : "虚拟片段已创建", icon: "success" });
    } catch (error) {
      toast.notify({ message: error instanceof Error ? error.message : String(error), icon: "fail" });
    }
  }

  void init();

  return { active, last, startedAt, init, start, startDetailed, create, cancelActive, pauseActive, resumeActive, retryLast };
});
