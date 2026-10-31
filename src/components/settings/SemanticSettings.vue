<script setup lang="ts">
import ActionButton from "../buttons/ActionButton.vue";
import SegmentedControl from "../buttons/SegmentedControl.vue";
import TextInput from "../inputs/TextInput.vue";
import { useSemanticSettings, type SemanticSettingsMock } from "../../lib/semanticSettings";

const props = defineProps<{ mock?: SemanticSettingsMock }>();
const {
  device, modelDir, runtime, runtimeNote, testingRuntime, chooseModelDir, resetModelDir, testRuntime,
  provider, providerKind, providerTimeout, connectionNote, savingProvider, testingProvider, saveProvider, testProvider,
  apiKey, hasKey, keyNote, savingKey, saveKey, clearKey,
} = useSemanticSettings(props.mock);

const DEVICE_OPTIONS = [
  { value: "cpu", label: "CPU（默认）" },
  { value: "gpu", label: "DirectML" },
] as const;
const PROVIDER_OPTIONS = [
  { value: "openai-compatible", label: "OpenAI 兼容" },
  { value: "ollama", label: "Ollama" },
] as const;
</script>

<template>
  <div class="sd-rows semantic-settings">
    <span class="sd-sub">本地画面分析</span>
    <div class="sd-row">
      <span class="sd-label">分析设备</span>
      <SegmentedControl v-model="device" :options="DEVICE_OPTIONS" variant="text" tone="g2" size="sm" class="sd-seg" />
    </div>
    <div class="sd-field">
      <div class="sd-row">
        <span class="sd-label">模型目录</span>
        <span class="sd-input-side">
          <ActionButton label="选择目录…" variant="ghost" size="sm" @click="chooseModelDir" />
          <ActionButton label="恢复默认" variant="ghost" size="sm" :disabled="!modelDir" @click="resetModelDir" />
        </span>
      </div>
      <p class="sd-note">{{ modelDir || "默认受管模型目录；也可选择包含 manifest 的离线模型包" }}</p>
    </div>
    <div class="sd-field">
      <div class="sd-row">
        <span class="sd-label">运行环境</span>
        <ActionButton :label="testingRuntime ? '检测中…' : '检测运行环境'" variant="ghost" size="sm" :disabled="testingRuntime" @click="testRuntime" />
      </div>
      <p class="sd-note">{{ runtimeNote }}</p>
      <template v-if="runtime">
        <p class="sd-note">模型：{{ runtime.modelDir }}</p>
        <p class="sd-note">运行时：{{ runtime.runtimeDir }}</p>
      </template>
    </div>

    <div class="sd-field">
      <span class="sd-sub">外部详细分析（可选）</span>
      <p class="sd-note">仅向你配置的服务发送带时间戳的采样帧；描述与事件由本地 CLIP 编码，之后可离线检索。</p>
    </div>
    <div class="sd-row">
      <span class="sd-label">服务类型</span>
      <SegmentedControl v-model="providerKind" :options="PROVIDER_OPTIONS" variant="text" tone="g2" size="sm" class="sd-seg" />
    </div>
    <div class="sd-field">
      <span class="sd-label">服务地址</span>
      <TextInput v-model="provider.endpoint" placeholder="https://…/v1 或 http://127.0.0.1:11434" />
    </div>
    <div class="sd-field">
      <span class="sd-label">视觉模型</span>
      <TextInput v-model="provider.model" placeholder="模型名称；需要支持你选择的图片输入接口" />
    </div>
    <div class="sd-row">
      <span class="sd-label">请求超时</span>
      <span class="sd-input-side">
        <TextInput v-model="providerTimeout" size="sm" class="sd-num" @keydown.enter="saveProvider" />
        <span class="sd-value">秒</span>
      </span>
    </div>
    <div class="sd-field">
      <span class="sd-label">API Key</span>
      <div class="sd-input-side">
        <TextInput v-model="apiKey" type="password" size="sm" placeholder="输入新密钥；不会回显已保存密钥" @keydown.enter="saveKey" />
        <ActionButton :label="savingKey ? '写入中…' : '保存密钥'" variant="ghost" size="sm" :disabled="savingKey || !apiKey.trim()" @click="saveKey" />
        <ActionButton v-if="hasKey" label="清除密钥" variant="ghost" size="sm" :disabled="savingKey" @click="clearKey" />
      </div>
      <p class="sd-note">{{ keyNote }}</p>
    </div>
    <div class="sd-field">
      <div class="sd-row">
        <span class="sd-label">连接配置</span>
        <span class="sd-input-side">
          <ActionButton :label="savingProvider ? '保存中…' : '保存配置'" variant="ghost" size="sm" :disabled="savingProvider || testingProvider" @click="saveProvider" />
          <ActionButton :label="testingProvider ? '测试中…' : '测试连接'" variant="ghost" size="sm" :disabled="testingProvider || savingProvider" @click="testProvider" />
        </span>
      </div>
      <p class="sd-note">{{ connectionNote }}</p>
    </div>
  </div>
</template>
