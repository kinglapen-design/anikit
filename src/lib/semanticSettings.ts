import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useToastStore } from "../stores/toast";
import {
  clearSemanticProviderKey,
  getSemanticProviderSettings,
  getSemanticSettings,
  saveSemanticProviderSettings,
  saveSemanticSettings,
  setSemanticProviderKey,
  testSemanticProvider,
  testSemanticRuntime,
  type ProviderConnection,
  type ProviderSettings,
  type SemanticRuntimeStatus,
  type SemanticSettings,
} from "./semantic";

export interface SemanticSettingsMock {
  device?: "cpu" | "gpu";
  modelDir?: string;
  runtime?: SemanticRuntimeStatus;
  provider?: Partial<ProviderSettings>;
  hasKey?: boolean;
  connection?: ProviderConnection;
}

const DEFAULT_PROVIDER: ProviderSettings = { kind: "openai-compatible", endpoint: "", model: "", timeoutSecs: 90 };

export function useSemanticSettings(mock?: SemanticSettingsMock) {
  const toast = useToastStore();
  const isMock = mock !== undefined;
  const disposed = ref(false);
  const deviceLocal = ref(mock?.device ?? "cpu");
  const modelDir = ref(mock?.modelDir ?? "");
  const runtime = ref<SemanticRuntimeStatus>();
  const runtimeMessage = ref("");
  const testingRuntime = ref(false);
  const provider = ref<ProviderSettings>({ ...DEFAULT_PROVIDER, ...mock?.provider });
  const providerTimeout = ref(String(provider.value.timeoutSecs));
  const apiKey = ref("");
  const hasKey = ref(mock?.hasKey ?? false);
  const connection = ref<ProviderConnection | undefined>(mock?.connection);
  const savingProvider = ref(false);
  const savingKey = ref(false);
  const testingProvider = ref(false);
  let savedSnapshot = JSON.stringify(provider.value);
  let localVersion = 0;
  let providerVersion = 0;
  let lastGood: SemanticSettings = { device: deviceLocal.value as "cpu" | "gpu", modelDir: modelDir.value || null };
  let localWrite = Promise.resolve();

  runtime.value = mock?.runtime;
  const device = computed<string>({
    get: () => deviceLocal.value,
    set: (value) => {
      if (value !== "cpu" && value !== "gpu") return;
      deviceLocal.value = value;
      void saveLocal();
    },
  });
  const providerKind = computed<string>({
    get: () => provider.value.kind,
    set: (value) => {
      if (value !== "ollama" && value !== "openai-compatible") return;
      provider.value.kind = value;
      void saveProvider();
    },
  });
  const runtimeNote = computed(() => {
    if (testingRuntime.value) return "正在加载本地文本模型并检测运行环境…";
    if (runtimeMessage.value) return runtimeMessage.value;
    const status = runtime.value;
    if (!status) return "尚未检测。基础分析与检索不调用外部 provider。";
    const backend = status.actualBackend === "cpu" ? "CPU" : status.actualBackend.startsWith("directml") ? "DirectML" : status.actualBackend;
    const fallback = status.requestedBackend === "directml" && status.actualBackend === "cpu" ? "（DirectML 不可用，已回退）" : "";
    return `实际后端：${backend}${fallback} · ${status.dimension} 维`;
  });
  const keyNote = computed(() => hasKey.value ? "密钥已保存在 Windows 钥匙串；输入新密钥可替换。" : "密钥只写入 Windows 钥匙串，不进入配置文件；本机服务可留空。");
  const connectionNote = computed(() => {
    if (testingProvider.value) return "正在测试用户配置的服务…";
    const result = connection.value;
    return result ? `${result.message}${result.ok ? ` · ${result.latencyMs} ms` : ""}` : "保存配置后可测试连接；连接失败不影响基础画面分析。";
  });

  function notify(error: unknown): void {
    if (!disposed.value) toast.notify({ message: String(error), icon: "fail" });
  }

  function saveLocal(): void {
    const next: SemanticSettings = { device: deviceLocal.value as "cpu" | "gpu", modelDir: modelDir.value.trim() || null };
    const version = ++localVersion;
    runtime.value = undefined;
    runtimeMessage.value = "";
    localWrite = localWrite.then(async () => {
      if (isMock) {
        lastGood = next;
        return;
      }
      try {
        await saveSemanticSettings(next);
        lastGood = next;
      } catch (error) {
        if (version === localVersion && !disposed.value) {
          deviceLocal.value = lastGood.device;
          modelDir.value = lastGood.modelDir ?? "";
        }
        notify(error);
      }
    });
  }

  async function chooseModelDir(): Promise<void> {
    if (isMock) {
      modelDir.value = "D:\\AniKit模型\\clip-vit-b32-multilingual";
      return;
    }
    try {
      const selected = await open({ directory: true, title: "选择含 manifest 的本地 CLIP 模型目录", defaultPath: modelDir.value || undefined });
      if (selected) {
        modelDir.value = selected;
        saveLocal();
      }
    } catch (error) {
      notify(error);
    }
  }

  function resetModelDir(): void {
    modelDir.value = "";
    saveLocal();
  }

  async function testRuntime(): Promise<void> {
    if (testingRuntime.value) return;
    testingRuntime.value = true;
    runtimeMessage.value = "";
    try {
      await localWrite;
      runtime.value = isMock
        ? {
            requestedBackend: deviceLocal.value === "gpu" ? "directml" : "cpu",
            actualBackend: "cpu",
            modelDir: modelDir.value || "D:\\AniKit模型\\clip-vit-b32-multilingual",
            runtimeDir: "D:\\AniKit运行时",
            dimension: 512,
          }
        : await testSemanticRuntime();
    } catch (error) {
      runtimeMessage.value = String(error);
      notify(error);
    } finally {
      if (!disposed.value) testingRuntime.value = false;
    }
  }

  function providerPayload(): ProviderSettings {
    const previous = JSON.parse(savedSnapshot) as ProviderSettings;
    const raw = Number(providerTimeout.value);
    const timeoutSecs = Number.isFinite(raw) && raw >= 1 ? Math.trunc(raw) : previous.timeoutSecs;
    providerTimeout.value = String(timeoutSecs);
    return { kind: provider.value.kind, endpoint: provider.value.endpoint.trim(), model: provider.value.model.trim(), timeoutSecs };
  }

  async function saveProvider(): Promise<boolean> {
    if (savingProvider.value) return false;
    savingProvider.value = true;
    const payload = providerPayload();
    if (JSON.stringify(payload) === savedSnapshot) {
      savingProvider.value = false;
      return true;
    }
    try {
      if (!isMock) await saveSemanticProviderSettings(payload);
      provider.value = payload;
      savedSnapshot = JSON.stringify(payload);
      connection.value = undefined;
      return true;
    } catch (error) {
      notify(error);
      return false;
    } finally {
      if (!disposed.value) savingProvider.value = false;
    }
  }

  async function testProvider(): Promise<void> {
    if (testingProvider.value || !(await saveProvider())) return;
    testingProvider.value = true;
    try {
      connection.value = isMock
        ? { ok: true, message: `连接成功，已找到模型「${provider.value.model || "演示视觉模型"}」`, latencyMs: 36 }
        : await testSemanticProvider();
    } catch (error) {
      connection.value = { ok: false, message: String(error), latencyMs: 0 };
      notify(error);
    } finally {
      if (!disposed.value) testingProvider.value = false;
    }
  }

  async function saveKey(): Promise<void> {
    if (savingKey.value || !apiKey.value.trim()) return;
    savingKey.value = true;
    try {
      if (!isMock) await setSemanticProviderKey(apiKey.value);
      apiKey.value = "";
      hasKey.value = true;
      connection.value = undefined;
    } catch (error) {
      notify(error);
    } finally {
      if (!disposed.value) savingKey.value = false;
    }
  }

  async function clearKey(): Promise<void> {
    if (savingKey.value) return;
    savingKey.value = true;
    try {
      if (!isMock) await clearSemanticProviderKey();
      hasKey.value = false;
      connection.value = undefined;
    } catch (error) {
      notify(error);
    } finally {
      if (!disposed.value) savingKey.value = false;
    }
  }

  watch(provider, () => {
    providerVersion += 1;
    connection.value = undefined;
  }, { deep: true });

  onMounted(async () => {
    if (isMock) return;
    const localAt = localVersion;
    const providerAt = providerVersion;
    try {
      const settings = await getSemanticSettings();
      if (localAt === localVersion && !disposed.value) {
        deviceLocal.value = settings.device;
        modelDir.value = settings.modelDir ?? "";
        lastGood = settings;
      }
    } catch (error) {
      notify(error);
    }
    try {
      const view = await getSemanticProviderSettings();
      if (providerAt === providerVersion && !disposed.value) {
        provider.value = view.settings;
        providerTimeout.value = String(view.settings.timeoutSecs);
        hasKey.value = view.hasKey;
        savedSnapshot = JSON.stringify(view.settings);
      }
    } catch (error) {
      notify(error);
    }
  });
  onBeforeUnmount(() => {
    disposed.value = true;
    apiKey.value = "";
  });

  return {
    device, modelDir, runtime, runtimeNote, testingRuntime, chooseModelDir, resetModelDir, testRuntime,
    provider, providerKind, providerTimeout, connectionNote, savingProvider, testingProvider, saveProvider, testProvider,
    apiKey, hasKey, keyNote, savingKey, saveKey, clearKey,
  };
}
