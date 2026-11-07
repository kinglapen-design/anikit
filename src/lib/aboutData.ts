import { onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import { useToastStore } from "../stores/toast";

export const PROJECT_URL = "https://github.com/Lapen/anikit";
export const FEEDBACK_URL = `${PROJECT_URL}/issues`;

export async function openExternalUrl(url: string): Promise<void> {
  await invoke("open_external_url", { url });
}

export function useAboutData(demo = false, demoVersion = "0.1.0") {
  const toast = useToastStore();
  const version = ref(demoVersion);

  function run(action: () => Promise<void>): void {
    if (demo) return;
    void action().catch((error) => toast.notify({ message: String(error), icon: "fail" }));
  }
  function openRepository(): void {
    run(() => openExternalUrl(PROJECT_URL));
  }
  function openFeedback(): void {
    run(() => openExternalUrl(FEEDBACK_URL));
  }
  function openLogs(): void {
    run(() => invoke("show_logs_in_explorer"));
  }

  onMounted(async () => {
    if (!demo) version.value = await getVersion().catch(() => version.value);
  });

  return { version, openRepository, openFeedback, openLogs };
}
