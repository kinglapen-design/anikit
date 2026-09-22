<script setup lang="ts">
// 浮层与滚动展区——右键/弹出菜单真实实例、hover 预览层四件、悬浮滚轮、悬浮栏宿主。
// 覆盖清单与处理模式：
// - ContextMenu.vue / AppMenu.vue（真实实例）：本展区自带 ContextMenu 宿主 + openContextMenu 桩 spec 触发
//   真实弹出（纯前端，action 全空操作）；AppMenu 按按钮坐标直接真实弹出（menu / tip 双 tone）。
//   PreviewMenus 展区已有 MenuItem 静态陈列，此处重点是整菜单真实组件（fixed 定位 / 边界钳制 / 级联 flyout）。
// - HoverZoomOverlay.vue（真实实例 + 复刻壳双模式）：桩资产 assetId = "ws:" 合成假 id 且 wsPath=null，
//   onMounted 直渲分支不触后端（严禁 assetId 类命令的 ws 口径天然豁免）；空图源只验浮层壳。
// - HoverVideoLayer / HoverSequenceLayer（静态复刻）：onMounted 经 useHoverPlayer
//   延迟后 invoke get_media_source / list_sequence_frames——预览红线不调后端，改静态复刻壳，
//   复刻范围 = 控件铬（胶囊 / 播放条），媒体本体不演。
//   （HoverAudioLayer 复刻展区 2026-09-22 撤：R6 audio 接线后该层收窄为透明 seek 交互层无视觉，
//   波形 / 扫过 / 胶囊视觉全归 PreviewCards 文件卡片-音频展区验收）
// - ScrollThumb.vue（真实实例）：relative 包裹层 + 滚动容器 + thumb 的宿主标准用法，纯前端测量。
// - FloatBarHost.vue（真实实例）：不传 panelId/viewKey = 搜索侧 store 全禁（独立预览窗口同口径），
//   previewActive=false 恒搜索态；验宿主定位层（底部居中距底 12 / 两侧 12 呼吸 / 穿透空隙）。
import { ref } from "vue";
import ContextMenu from "../../components/ContextMenu.vue";
import AppMenu from "../../components/AppMenu.vue";
import MenuItem from "../../components/MenuItem.vue";
import HoverZoomOverlay from "../../components/HoverZoomOverlay.vue";
import ScrollThumb from "../../components/ScrollThumb.vue";
import ViewOptionsMenu from "../../components/ViewOptionsMenu.vue";
import FloatBarHost from "../../components/FloatBarHost.vue";
import AppIcon from "../../components/AppIcon.vue";
import ActionButton from "../../components/buttons/ActionButton.vue";
import { openContextMenu, type CtxEntry } from "../../lib/contextMenus";
import { FOLDER_SWATCHES } from "../../lib/customColors";
import type { PlacementView } from "../../stores/content";

// —— ContextMenu 真实弹出：桩 spec（分组 / 快捷键 / 对勾 / 二级 flyout / 色块行 / 禁用），action 全空操作 ——
const ctxEntries: CtxEntry[] = [
  { label: "打开文件夹", shortcut: "Enter", action: () => {} },
  { label: "新标签页打开", shortcut: "Ctrl+Enter", action: () => {} },
  "divider",
  {
    label: "排序依据",
    submenu: [
      { label: "自定义", checkable: true, checked: true, keepOpen: true, action: () => {} },
      { label: "名称", checkable: true, keepOpen: true, action: () => {} },
      { label: "文件大小", checkable: true, keepOpen: true, action: () => {} },
    ],
  },
  {
    label: "更改颜色",
    swatches: { colors: FOLDER_SWATCHES, current: "green", onPick: () => {} },
  },
  "divider",
  { label: "偏好设置", disabled: true },
  { label: "退出", shortcut: "Alt+F4", action: () => {} },
];
function popContextMenu(e: MouseEvent): void {
  openContextMenu({ clientX: e.clientX, clientY: e.clientY }, ctxEntries);
}

// —— AppMenu 真实弹出：按钮下方弹出（menu / tip 双 tone），点组外 / Esc 关闭 ——
const appMenuOpen = ref(false);
const tipMenuOpen = ref(false);
const appMenuPos = ref({ x: 0, y: 0 });
const tipMenuPos = ref({ x: 0, y: 0 });
function popAppMenu(e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  appMenuPos.value = { x: r.left, y: r.bottom + 4 };
  appMenuOpen.value = !appMenuOpen.value;
  tipMenuOpen.value = false;
}
function popTipMenu(e: MouseEvent): void {
  const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
  tipMenuPos.value = { x: r.left, y: r.bottom + 4 };
  tipMenuOpen.value = !tipMenuOpen.value;
  appMenuOpen.value = false;
}

// —— HoverZoomOverlay 真实实例：ws 合成假 id + wsPath=null → onMounted 不触后端，空图源只验壳 ——
const zoomOpen = ref(false);
const zoomAsset: PlacementView = {
  assetId: "ws:demo-zoom",
  folderId: "demo",
  name: "演示_占位.png",
  w: null,
  h: null,
  ext: "png",
  thumbPath: null,
  thumbKind: null,
  origin: true,
  frameCount: null,
  animated: false,
  duration: null,
  bpm: null,
  importedFrom: null,
  size: 0,
  createdAt: null,
  modifiedAt: "2026-09-20T00:00:00Z",
  importedAt: "2026-09-20T00:00:00Z",
  tags: [],
  hidden: false,
  wsPath: null,
};

// —— ScrollThumb 真实实例：relative 包裹层 + 滚动容器（模板 ref 传 body）——
const scrollBody = ref<HTMLElement>();
const scrollRows = Array.from({ length: 30 }, (_, i) => `演示行 ${String(i + 1).padStart(2, "0")} —— 滚动我`);

// —— ViewOptionsMenu 真实实例：ui store 预览环境纯内存（initSettings 不经 App.vue 不跑，无写回）——
const viewOptionsOpen = ref(false);
const viewOptionsAnchor = ref({ x: 0, y: 0 });
const viewOptionsTrigger = ref<HTMLElement | null>(null);
function popViewOptions(e: MouseEvent): void {
  const el = e.currentTarget as HTMLElement;
  const r = el.getBoundingClientRect();
  viewOptionsTrigger.value = el;
  viewOptionsAnchor.value = { x: r.left + 175, y: r.bottom + 4 };
  viewOptionsOpen.value = !viewOptionsOpen.value;
}
</script>

<template>
  <section id="design-system-overlays" class="pv-section">
    <h2 class="pv-sec-title">组件 · 浮层与滚动</h2>
    <p class="pv-sec-desc">
      浮层类组件验收面：右键/弹出菜单（数据驱动 lib/contextMenus + 容器 AppMenu）、卡片 hover 预览层四件、
      悬浮滚轮 ScrollThumb、悬浮栏宿主 FloatBarHost。真实实例优先；onMounted 调 invoke 的（音视频/序列帧
      悬停层）按预览红线改静态复刻并在 pv-cap 注明。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">右键菜单 ContextMenu + 容器 AppMenu（真实弹出；新体系——7.1 二十四轮按新稿重构，全 --n- token）</h3>
      <p class="pv-group-note">
        ContextMenu = 全局唯一宿主（实机挂 App.vue，本展区自带一个宿主实例）读 menuState 渲染；
        spec 数据驱动（lib/menuSpecs/*，divider 切分组 / submenu 级联 flyout / swatches 色块行）。
        AppMenu = fixed 容器：内容自适应宽 + 边界钳制 + 弹出动画 140ms + 超高挂 ScrollThumb。
      </p>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="弹出真实右键菜单" variant="solid" size="md" @click="popContextMenu" />
          <span class="pv-cap">
            真实实例（桩 spec，action 空操作）：分组 / 快捷键 / 对勾 / hover「排序依据」出二级 flyout /
            色块行 / 禁用项；点组外或 Esc 关闭
          </span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="弹出 AppMenu（menu）" variant="solid" size="md" @click="popAppMenu" />
          <span class="pv-cap">真实容器：g3 底 + inset g5 描边 + 小阴影 + 圆角 8，min-w 250（一级条目菜单口径）</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="弹出 AppMenu（tip）" variant="solid" size="md" @click="popTipMenu" />
          <span class="pv-cap">tip 壳（linkList 二级同款）：半透明-1 + 1px 半透明-3 真 border + blur + 圆角 6，pad 0 行贴边</span>
        </div>
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="弹出查看方式面板" variant="solid" size="md" @click="popViewOptions" />
          <span class="pv-cap">
            ViewOptionsMenu 真实实例（2026-09-20 按查看方式 01~06 稿换代）：404 面板 + 大阴影 + 分段 sm 28 /
            禁用 30% / picker 下拉；ui store 预览环境纯内存（不写回后端）
          </span>
        </div>
      </div>
    </div>

    <!-- 本展区自带的 ContextMenu 宿主（实机宿主在 App.vue；预览页不挂 App，自带一个供真实弹出） -->
    <ContextMenu />
    <ViewOptionsMenu
      v-if="viewOptionsOpen"
      :anchor="viewOptionsAnchor"
      :trigger="viewOptionsTrigger"
      @close="viewOptionsOpen = false"
    />
    <AppMenu v-if="appMenuOpen" :x="appMenuPos.x" :y="appMenuPos.y" :min-w="250" @close="appMenuOpen = false">
      <div class="pv-menu-group">
        <MenuItem label="新建文件夹" shortcut="Ctrl+Shift+N" />
        <MenuItem label="新建档案" shortcut="Shift+N" force-state="hover" />
      </div>
      <div class="pv-menu-group">
        <MenuItem label="删除文件夹" shortcut="Delete" disabled />
      </div>
    </AppMenu>
    <AppMenu v-if="tipMenuOpen" :x="tipMenuPos.x" :y="tipMenuPos.y" tone="tip" pad="0" @close="tipMenuOpen = false">
      <div class="pv-tip-row">雪牧_自动.fla</div>
      <div class="pv-tip-row">文件夹 / 01_人物</div>
    </AppMenu>

    <div class="pv-group">
      <h3 class="pv-group-title">空格放大预览浮层 HoverZoomOverlay（真实实例 + 复刻壳；旧体系待换代——仍吃 --c3 / --border-1 / --shadow-pop 旧 token）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 240px">
          <ActionButton label="打开真实浮层" variant="solid" size="md" @click="zoomOpen = true" />
          <span class="pv-cap">
            真实实例：桩资产 ws: 假 id（wsPath=null）不触后端，空图源只验壳——60vw×70vh 居中 +
            全屏透明遮罩 + pointer-events 穿透；Esc / 空格关闭
          </span>
        </div>
        <div class="pv-demo-cell">
          <div class="pv-zoom-shell">
            <span class="pv-cap">图 contain 居中区</span>
          </div>
          <span class="pv-cap">
            复刻壳（--n- 近似，预览只吃 --n-；实件 = C3 底 / inset 1px border-1 / 圆角 8 / --shadow-pop / padding 8，改口径时与组件同步）
          </span>
        </div>
      </div>
    </div>
    <HoverZoomOverlay v-if="zoomOpen" :asset="zoomAsset" @close="zoomOpen = false" @space="zoomOpen = false" />

    <div class="pv-group">
      <h3 class="pv-group-title">视频卡片播放层 HoverVideoLayer（静态复刻——onMounted 延迟后 invoke get_media_source，红线不真挂）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-card-stage">
            <div class="pv-kc-float pv-hv-capsule">
              <span class="pv-hv-pos">00:12</span>
              <AppIcon name="card-hover-play" :size="14" class="pv-hv-btn" />
              <AppIcon name="card-hover-mute" :size="14" class="pv-hv-btn" />
            </div>
            <div class="pv-hv-bar"><div class="pv-hv-fill" style="width: 35%" /></div>
          </div>
          <span class="pv-cap">
            悬停模式控件复刻：右上胶囊（帧号/时间 + 播放暂停 + 音量静音，kc-float 铬）+ 底部 4px 播放条
            （轨道 亮色/半透明-2、填充 亮色/一级，贴底 4 左右 12）；控件区新体系（7.1 内容容器轮换代），
            跟随鼠标 scrub / 范围循环逻辑不演
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">序列帧卡片播放层 HoverSequenceLayer（静态复刻，同上红线；旧体系待换代——层底仍吃旧 --c1）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-card-stage pv-seq-stage">
            <span class="pv-cap">帧图轮播区（无控件）</span>
          </div>
          <span class="pv-cap">
            复刻：absolute 铺满卡面，按 fps 轮播 info/frames 帧原图（cover / grid contain），
            层底 C1 防透明帧双重影像——此处以 --n-g1 近似；加载走 list_sequence_frames 不真挂
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">悬浮滚轮 ScrollThumb（真实实例；已新体系——g4/g6 色档 + --n-radius-pill 胶囊）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div class="pv-scroll-wrap">
            <div ref="scrollBody" class="pv-scroll-body">
              <div v-for="row in scrollRows" :key="row" class="pv-scroll-row">{{ row }}</div>
            </div>
            <ScrollThumb :body="scrollBody" />
          </div>
          <span class="pv-cap">
            真实实例：relative 包裹层 + 滚动容器（body ref）标准用法；悬停包裹层显 / 离开隐 / 拖拽锁显，
            6px 胶囊贴右（right 0）。色档 --n-g4 常态 / --n-g6 hover（2026-09-06 走查），
            圆角 --n-radius-pill（2026-09-20 入档）——已新体系，台账归白名单
          </span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">悬浮栏宿主 FloatBarHost（真实实例；新体系——全 --n- token）</h3>
      <p class="pv-group-note">
        宿主定位层 = 底部居中距底 12、两侧各 12 呼吸、pointer-events 穿透空隙；搜索/播放单例 morph
        （FloatBar 本体见悬浮栏展区）。本实例不传 panelId/viewKey（独立预览窗口口径）= 搜索侧 store
        全禁、纯前端；播放态 spec 依赖预览媒体链路（barSpecs → preview/previewMedia store），演示从简未接。
      </p>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 100%">
          <div class="pv-fbh-stage">
            <FloatBarHost />
          </div>
          <span class="pv-cap">
            真实实例（搜索态）：栏锚底 12 居中、两侧 12 呼吸随舞台宽伸缩；可输入（draft 本地，不接 search
            store）；帧条 / 弹层（倍率·音量·时间显示·笔记）为播放态件，此处不出
          </span>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
/* AppMenu 静态内容的分组容器（复刻 ContextMenu .menu-group 口径，同 PreviewMenus pv-menu-group） */
.pv-menu-group {
  display: flex;
  flex-direction: column;
  gap: var(--n-space-2);
  padding: var(--n-space-4) 0;
}
.pv-menu-group:first-child {
  padding-top: 0;
}
.pv-menu-group:last-child {
  padding-bottom: 0;
}
.pv-menu-group:not(:last-child) {
  box-shadow: inset 0 -1px 0 var(--n-g5);
}
/* tip 壳演示行（pad 0 行贴边口径，行自带左右 12 内距） */
.pv-tip-row {
  padding: var(--n-space-6) var(--n-space-12);
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l2);
  white-space: nowrap;
}
.pv-tip-row + .pv-tip-row {
  box-shadow: inset 0 1px 0 var(--n-l-a3);
}

/* HoverZoomOverlay 复刻壳（实件仍旧 --c3/--border-1/--shadow-pop，此处 --n- 近似；口径变更与组件同步） */
.pv-zoom-shell {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 320px;
  height: 200px;
  padding: var(--n-space-8);
  box-sizing: border-box;
  border-radius: var(--n-radius-8);
  background: var(--n-g3);
  box-shadow:
    inset 0 0 0 1px var(--n-g5),
    var(--n-shadow-lg);
}

/* 卡片舞台（hover 播放层三件复刻共用）：卡面比例 3:2 桩，底 = 缩略图占位 */
.pv-card-stage {
  position: relative;
  width: 300px;
  height: 200px;
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
  overflow: hidden;
}
/* kc-float 铬复刻（cards.css 全局类未进预览构建，按源口径复刻：半透明-2 底 + 立体感-通用 + blur 小 + 圆角 4） */
.pv-kc-float {
  box-sizing: border-box;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  padding: var(--n-space-2) var(--n-space-6);
  background: var(--n-g-a2);
  box-shadow: var(--n-effect-raised);
  backdrop-filter: blur(var(--n-blur-sm));
  border-radius: var(--n-radius-4);
}
/* 右上胶囊（HoverVideoLayer .hv-capsule：右上 12/12、gap 6） */
.pv-hv-capsule {
  position: absolute;
  right: var(--n-space-12);
  top: var(--n-space-12);
  gap: var(--n-space-6);
}
/* 帧号 / 时长：数字档 12/16 二级 */
.pv-hv-pos {
  font-family: var(--n-font-mono);
  font-size: var(--n-fs-num);
  line-height: var(--n-lh-num);
  font-weight: var(--n-fw-medium);
  color: var(--n-l2);
}
.pv-hv-btn {
  color: var(--n-l2);
}
/* 底部 4px 播放条（贴底 4、左右 12、圆头 99；轨道 亮色/半透明-2、填充 亮色/一级） */
.pv-hv-bar {
  position: absolute;
  left: var(--n-space-12);
  right: var(--n-space-12);
  bottom: var(--n-space-4);
  height: 4px;
  border-radius: 99px;
  background: var(--n-l-a2);
}
.pv-hv-fill {
  display: block;
  height: 100%;
  border-radius: 99px;
  background: var(--n-l1);
}
/* 序列帧层复刻（实件层底 = 旧 --c1，此处 --n-g1 近似） */
.pv-seq-stage {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--n-g1);
}

/* ScrollThumb 演示：relative 包裹层 + 滚动容器（thumb top 按 body offsetTop 换算，须为包裹层直接子元素） */
.pv-scroll-wrap {
  position: relative;
  width: 320px;
  border-radius: var(--n-radius-8);
  background: var(--n-g2);
  box-shadow: inset 0 0 0 1px var(--n-l-a1);
}
.pv-scroll-body {
  height: 180px;
  overflow-y: auto;
  padding: var(--n-space-8) var(--n-space-16) var(--n-space-8) var(--n-space-12);
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
  gap: var(--n-space-4);
}
.pv-scroll-row {
  flex: none;
  font-size: var(--n-fs-secondary);
  line-height: var(--n-lh-secondary);
  color: var(--n-l3);
}

/* FloatBarHost 演示舞台：relative 供 fbh-anchor 锚定（宿主定位 = 底部居中距底 12、两侧 12 呼吸） */
.pv-fbh-stage {
  position: relative;
  width: 100%;
  height: 140px;
  border-radius: var(--n-radius-8);
  background: var(--n-g1);
  box-shadow: inset 0 0 0 1px var(--n-l-a3);
}
</style>
