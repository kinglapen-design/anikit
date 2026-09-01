<script setup lang="ts">
// CEP 导出设置弹窗（8.1 二轮 2b）：导出参数（时长/大小/尺寸，settings.cepExport 单写点——
// CEP 面板与 8.2 批量导出共用，保存后 WS 实时推送在线插件）+ CEP 插件一键安装
// （extensions 复制 + PlayerDebugMode 确保）。壳 = ConfirmModal（稿口径），输入 = TextInput。
import { onMounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import ConfirmModal from "./ConfirmModal.vue";
import TextInput from "../inputs/TextInput.vue";
import ActionButton from "../buttons/ActionButton.vue";
import { useToastStore } from "../../stores/toast";

interface CepExportSettings {
  maxSecs: number;
  maxBytes: number;
  maxEdge: number;
}
interface CepInstallStatus {
  sourceAvailable: boolean;
  installed: boolean;
  installedVersion: string | null;
  bundledVersion: string;
  debugModeReady: boolean;
}

const emit = defineEmits<{ close: [] }>();
const toast = useToastStore();

const maxSecs = ref("10");
const maxMB = ref("2");
const maxEdge = ref("1024");
const install = ref<CepInstallStatus | null>(null);
const busy = ref(false);

onMounted(async () => {
  try {
    const s = await invoke<CepExportSettings>("get_cep_export_settings");
    maxSecs.value = String(s.maxSecs);
    maxMB.value = (s.maxBytes / 1048576).toFixed(s.maxBytes % 1048576 === 0 ? 0 : 1);
    maxEdge.value = String(s.maxEdge);
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  }
  await refreshInstall();
});

async function refreshInstall(): Promise<void> {
  try {
    install.value = await invoke<CepInstallStatus>("get_cep_install_status");
  } catch {
    install.value = null;
  }
}

async function save(): Promise<void> {
  const mb = Number(maxMB.value);
  const payload: CepExportSettings = {
    maxSecs: Number(maxSecs.value),
    maxBytes: Math.round(mb * 1048576),
    maxEdge: Number(maxEdge.value),
  };
  busy.value = true;
  try {
    await invoke("save_cep_export_settings", { settings: payload });
    toast.notify({ message: "导出设置已保存（在线插件实时生效）", icon: "success" });
    emit("close");
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    busy.value = false;
  }
}

async function doInstall(): Promise<void> {
  busy.value = true;
  try {
    install.value = await invoke<CepInstallStatus>("install_cep_plugin");
    toast.notify({ message: `CEP 插件已安装（v${install.value.bundledVersion}），重启 Animate 后在 窗口 > 扩展 中打开`, icon: "success" });
  } catch (e) {
    toast.notify({ message: String(e), icon: "fail" });
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <ConfirmModal
    title="CEP 导出设置"
    kind="brand"
    confirm-label="保存"
    :cancel-label="'关闭'"
    @confirm="save"
    @cancel="emit('close')"
  >
    <div class="form">
      <label class="field">
        <span class="label">动态时长上限（秒）</span>
        <TextInput v-model="maxSecs" size="sm" placeholder="10" />
      </label>
      <label class="field">
        <span class="label">大小上限（MB，软上限自动降级）</span>
        <TextInput v-model="maxMB" size="sm" placeholder="2" />
      </label>
      <label class="field">
        <span class="label">尺寸上限（像素，长边）</span>
        <TextInput v-model="maxEdge" size="sm" placeholder="1024" />
      </label>
      <p class="hint">作用于 Animate CEP 插件导出与 APP 内批量导出；保存后实时推送在线插件。</p>

      <div class="divider" />

      <div class="install-row">
        <div class="install-status">
          <template v-if="install?.installed">
            CEP 插件已安装 v{{ install.installedVersion }}
            <span v-if="install.installedVersion !== install.bundledVersion" class="warn">（可升级 v{{ install.bundledVersion }}）</span>
          </template>
          <template v-else>CEP 插件未安装</template>
        </div>
        <ActionButton
          variant="solid"
          size="md"
          :label="install?.installed ? '重新安装 / 升级' : '一键安装'"
          :disabled="busy || install?.sourceAvailable === false"
          @click="doInstall"
        />
      </div>
    </div>
  </ConfirmModal>
</template>

<style scoped>
.form {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-12);
}
.field {
  display: flex;
  align-items: center;
  gap: var(--n-space-12);
}
.label {
  flex: 1;
  font-size: var(--n-fs-body);
  color: var(--n-l2);
}
.field :deep(.ti-root) {
  width: 120px;
  flex: none;
}
.hint {
  font-size: var(--n-fs-caption);
  color: var(--n-l4);
}
.divider {
  height: 1px;
  background: var(--n-g5);
}
.install-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--n-space-12);
}
.install-status {
  font-size: var(--n-fs-body);
  color: var(--n-l2);
}
.warn {
  color: var(--n-warning);
}
</style>
