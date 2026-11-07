<script setup lang="ts">
import BaseModal from "./modals/BaseModal.vue";
import ActionButton from "./buttons/ActionButton.vue";
import { useAboutData } from "../lib/aboutData";
import { useDialogEscape } from "../lib/dialogKeys";
import logoUrl from "../../assets/image/anikit-logo.png?url";

const props = defineProps<{ demo?: boolean; demoVersion?: string }>();
const emit = defineEmits<{ close: []; openHelp: [] }>();
const { version, openRepository, openFeedback, openLogs } = useAboutData(props.demo, props.demoVersion);
useDialogEscape(() => emit("close"));
</script>

<template>
  <BaseModal title="关于 AniKit" :width="520" framed class="about-dialog" @close="emit('close')">
    <div class="about-content">
      <img :src="logoUrl" alt="AniKit LOGO" class="about-logo" />
      <div class="about-heading">
        <span class="about-name">AniKit</span>
        <span class="about-tagline">面向动画师的素材管理软件</span>
      </div>
      <div class="about-release">
        <span class="about-version">v{{ version }}</span>
        <span class="about-stage">早期版本</span>
      </div>
      <p class="about-description">集中收管动画素材，也能直接整理现有磁盘目录。支持语义检索、Markdown、Animate 插件与 AI 外接通道。</p>
      <div class="about-primary-actions">
        <ActionButton label="使用教程" variant="solid" size="md" @click="emit('openHelp')" />
        <ActionButton label="GitHub 仓库" variant="solid" size="md" @click="openRepository" />
      </div>
      <div class="about-secondary-actions">
        <ActionButton label="意见反馈" variant="ghost" size="sm" @click="openFeedback" />
        <ActionButton label="查看软件日志" variant="ghost" size="sm" @click="openLogs" />
      </div>
      <p class="about-platform">Windows 10 / 11 · 本地数据优先</p>
    </div>
  </BaseModal>
</template>

<style scoped>
.about-dialog :deep(.bm-modal.framed) {
  gap: 0;
  max-width: calc(100vw - var(--n-space-72));
}
.about-dialog :deep(.bm-modal .about-content .act-btn.sm) {
  width: auto;
  padding: var(--n-space-4) var(--n-space-8);
}
.about-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--n-space-16);
  padding: var(--n-space-24);
  text-align: center;
}
.about-logo {
  width: var(--n-space-72);
  height: var(--n-space-72);
  object-fit: contain;
}
.about-heading {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.about-name {
  font-size: var(--n-fs-display);
  line-height: var(--n-lh-display);
  font-weight: var(--n-fw-bold);
  color: var(--n-l1);
}
.about-tagline,
.about-description,
.about-platform {
  margin: 0;
  font-size: var(--n-fs-body);
  line-height: var(--n-lh-body);
  color: var(--n-l3);
}
.about-release {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--n-space-8);
}
.about-version,
.about-stage {
  padding: var(--n-space-4) var(--n-space-8);
  border-radius: var(--n-radius-6);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
.about-version {
  font-family: var(--n-font-mono);
  color: var(--n-l2);
  background: var(--n-g4);
}
.about-stage {
  color: var(--n-brand-bright);
  box-shadow: inset 0 0 0 1px var(--n-brand-50);
}
.about-description {
  max-width: 100%;
  color: var(--n-l2);
  overflow-wrap: anywhere;
}
.about-primary-actions {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--n-space-8);
  width: 100%;
}
.about-secondary-actions {
  display: flex;
  align-items: center;
  justify-content: center;
  flex-wrap: wrap;
  gap: var(--n-space-8);
}
.about-platform {
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
}
</style>
