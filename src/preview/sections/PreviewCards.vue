<script setup lang="ts">
// 组件 卡片展区——docs/06_UI重构/Design Token/组件/卡片/ 十稿的展示与验收面。
// force-hover / force-selected = 强制态静态验收；悬停控件（胶囊 / 播放条 / 波形扫过）hover 才现（2026-08-29 用户定）。
// 动画时长：hover / 颜色 150ms；位移 -4 + 小阴影 250ms；列表表头箭头旋转 300ms。
// 演示缩略图 = 内联 SVG data URI（纯前端渲染，不调后端命令）。
import FileCard from "../../components/cards/FileCard.vue";
import FolderCard from "../../components/cards/FolderCard.vue";
import ArchiveCard from "../../components/cards/ArchiveCard.vue";
import ListRowCard from "../../components/cards/ListRowCard.vue";
import ListHeader from "../../components/cards/ListHeader.vue";
import FormatBadge from "../../components/cards/FormatBadge.vue";

const svgUri = (s: string): string => `data:image/svg+xml;utf8,${encodeURIComponent(s)}`;
// 风景照（通用 / 视频 / 列表行缩略图共用）
const demoPhoto = svgUri(
  `<svg xmlns="http://www.w3.org/2000/svg" width="395" height="222" viewBox="0 0 395 222">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#2b3a55"/><stop offset="1" stop-color="#141b29"/></linearGradient></defs>` +
    `<rect width="395" height="222" fill="url(#g)"/>` +
    `<circle cx="320" cy="45" r="26" fill="#e8c26a"/>` +
    `<path d="M0 170 L120 80 L210 160 L300 100 L395 175 L395 222 L0 222 Z" fill="#1d2740"/>` +
    `<path d="M0 195 L150 130 L260 190 L395 150 L395 222 L0 222 Z" fill="#10151f"/>` +
    `</svg>`,
);
// 档案封面（方形）
const demoCover = svgUri(
  `<svg xmlns="http://www.w3.org/2000/svg" width="250" height="250" viewBox="0 0 250 250">` +
    `<defs><linearGradient id="g" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#3d2b52"/><stop offset="1" stop-color="#16101f"/></linearGradient></defs>` +
    `<rect width="250" height="250" fill="url(#g)"/>` +
    `<circle cx="125" cy="105" r="58" fill="#c9a06a"/>` +
    `<path d="M40 250 Q125 160 210 250 Z" fill="#5a4432"/>` +
    `</svg>`,
);
// shell 缩略图（灰文档形，50% 居中演示）
const demoShell = svgUri(
  `<svg xmlns="http://www.w3.org/2000/svg" width="96" height="96" viewBox="0 0 96 96">` +
    `<rect x="22" y="12" width="52" height="72" rx="6" fill="#4d4d4d"/>` +
    `<rect x="32" y="30" width="32" height="4" rx="2" fill="#242424"/>` +
    `<rect x="32" y="42" width="32" height="4" rx="2" fill="#242424"/>` +
    `<rect x="32" y="54" width="20" height="4" rx="2" fill="#242424"/>` +
    `</svg>`,
);
// 音频演示波形（47 根 = 稿 376 区 / 节距 8；根数恒定，美观优先）
const demoPeaks = Array.from({ length: 47 }, (_, i) => 0.2 + 0.72 * Math.abs(Math.sin(i * 1.7)));
// txt-md 演示正文（溢出出滚动胶囊）
const demoText = `@图片1 是故事发生的场景，金属怪人的巢穴场景，空间巨大，可轻松容纳百米高的金属怪人，中间有圆形的巨大深坑。
@图片2 是金属怪人的角色造型参考，身躯由无数机械零件和管道组合而成，身高百米。
@图片3 是重型悬浮战车，车高约十米，全金属材质，磁悬浮底盘持续朝地面喷射蓝色尾焰，车顶驾着巨大的火炮。

故事内容：此时整个巢穴突然地动山摇起来！一只体型足有百米高、浑身由扭曲的钢铁和狂暴的沙暴能量组成的恐怖金属怪人，缓缓从地面爬出。周围的空气瞬间被抽干，形成了恐怖的龙卷风。
导演阐述：营造出金属怪人出场时的压迫感。`;

const folderLines = [
  ["1,233 个文件", "1.25 GB"],
  ["23 个档案", "12 个子文件夹"],
];
const archiveAttrs = ["男", "18岁", "主角"];
const archiveStats = ["1,233 个文件", "1.25 GB"];
</script>

<template>
  <section id="design-system-cards" class="pv-section">
    <h2 class="pv-sec-title">组件 · 卡片</h2>
    <p class="pv-sec-desc">
      Design Token/组件/卡片/ 十稿：文件卡（通用 / 视频 / 音频 / 字体 / txt-md 五稿合一 FileCard）+ 文件夹 / 档案 /
      列表卡 / 列表表头 / 格式角标。悬停控件（右上胶囊 / 视频播放条 / 波形品牌色扫过）<b>hover 才现</b>，force-hover 列为静态验收。<br />
      动画时长：hover / 颜色 <b>150ms</b>；悬停位移 -4px + 小阴影 <b>250ms</b>；表头排序箭头旋转 <b>300ms</b>。组件宽高随父容器，此处按稿值演示。<br />
      业务态（7.1 内容容器轮接线）静态陈列：naming 行内重命名（整卡品牌环 + 名称位换输入框）/ dimmed 拖拽源 0.4 / cut 剪切 0.5 /
      dropInto 拖拽并入（品牌/半透明 + 2px 品牌描边）/ picked 选择模式已选（压暗 30%）；thumbFit = cover / contain / icon 三档。
    </p>

    <div class="pv-group">
      <h3 class="pv-group-title">文件卡片-通用（395×222；square = shell 缩略图 / 未导入 50% 居中；thumbFit 三档）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 395px; height: 222px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoPhoto" origin-badge />
          </div>
          <span class="pv-cap">常态</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 395px; height: 222px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoPhoto" origin-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover（六级描边 + 上移 4 + 小阴影）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 395px; height: 222px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoPhoto" origin-badge selected />
          </div>
          <span class="pv-cap">选中（品牌 2px）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoShell" square origin-badge />
          </div>
          <span class="pv-cap">square · 未导入 / shell</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 124px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoShell" />
          </div>
          <span class="pv-cap">thumbFit · cover（默认，充满裁切）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 124px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoShell" thumb-fit="contain" />
          </div>
          <span class="pv-cap">thumbFit · contain（固定格完整显示不裁切）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 124px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoShell" thumb-fit="icon" />
          </div>
          <span class="pv-cap">thumbFit · icon（格式图标 50% 居中，Eagle 口径）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 124px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoPhoto" origin-badge naming />
          </div>
          <span class="pv-cap">命名态（naming 直挂；词干选区，Esc = 取消——文件口径）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 124px">
            <FileCard name="雪牧_自动.fla" info="1920 x 1080" badge="FLA" :thumb-url="demoPhoto" origin-badge picked />
          </div>
          <span class="pv-cap">选择模式已选 picked（内容压暗 30%，大勾由壳层加）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">文件卡片-视频（421×252，悬停胶囊 = 帧数 + 播放/暂停 + 音量/静音）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 421px; height: 252px">
            <FileCard kind="video" name="第一集.mp4" info="12:23" badge="MP4" :thumb-url="demoPhoto" capsule-text="3214" :progress="0.35" origin-badge />
          </div>
          <span class="pv-cap">常态（悬停控件隐）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 421px; height: 252px">
            <FileCard kind="video" name="第一集.mp4" info="12:23" badge="MP4" :thumb-url="demoPhoto" capsule-text="3214" :progress="0.35" origin-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover（胶囊 + 播放条 35%）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 421px; height: 252px">
            <FileCard kind="video" name="第一集.mp4" info="12:23" badge="MP4" :thumb-url="demoPhoto" capsule-text="3214" :progress="0.35" playing muted origin-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover · 播放中 + 静音</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">文件卡片-音频（400×200，进度 = 品牌色波形扫过，与根数解耦）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 400px; height: 200px">
            <FileCard kind="audio" name="雪牧.wav" info="12:23" badge="WAV" :peaks="demoPeaks" capsule-text="03:12" origin-badge />
          </div>
          <span class="pv-cap">常态</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 400px; height: 200px">
            <FileCard kind="audio" name="雪牧.wav" info="12:23" badge="WAV" :peaks="demoPeaks" capsule-text="03:12" :progress="0.4" origin-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover（胶囊 + 扫过 40%）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 400px; height: 200px">
            <FileCard kind="audio" name="雪牧.wav" info="12:23" badge="WAV" :peaks="demoPeaks" capsule-text="03:12" :progress="0.4" playing origin-badge selected />
          </div>
          <span class="pv-cap">选中 · 播放中</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">文件卡片-字体（220×220，示例字日/韩环境业务侧替换）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 220px">
            <FileCard kind="font" name="yahei.ttf" info="Light" badge="TTF" origin-badge />
          </div>
          <span class="pv-cap">常态 · 未安装</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 220px">
            <FileCard kind="font" name="yahei.ttf" info="Light" badge="TTF" origin-badge installed force-state="hover" />
          </div>
          <span class="pv-cap">hover · 已安装（品牌底）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 220px">
            <FileCard kind="font" name="yahei.ttf" info="Light" badge="TTF" origin-badge selected />
          </div>
          <span class="pv-cap">选中</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">文件卡片-txt-md（300×240，溢出出滚动胶囊，hover 胶囊升五级）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 300px; height: 240px">
            <FileCard kind="text" name="镜头1_提示词.md" info="2.1 KB" text-title="镜头1_提示词" :text-body="demoText" origin-badge />
          </div>
          <span class="pv-cap">常态（内容可滚动）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 300px; height: 240px">
            <FileCard kind="text" name="镜头1_提示词.md" info="2.1 KB" text-title="镜头1_提示词" :text-body="demoText" origin-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 300px; height: 240px">
            <FileCard kind="text" name="镜头1_提示词.md" info="2.1 KB" text-title="镜头1_提示词" :text-body="demoText" origin-badge selected />
          </div>
          <span class="pv-cap">选中</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">列表卡片 + 列表表头（856；表头箭头 hover 现 / 当前排序列恒显，点击可切向；尺寸列不可排序——无箭头不响应点击）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell" style="width: 856px">
          <div style="display: flex; flex-direction: column; gap: 6px">
            <ListHeader sort-key="size" sort-dir="desc" />
            <ListRowCard name="雪牧_自动.fla" :thumb-url="demoPhoto" badge="FLA" dims="1920 x 1080" file-size="12 MB" mod-date="2026/08/24" mod-time="16:07" origin-badge />
            <ListRowCard name="第一集.mp4" :thumb-url="demoPhoto" badge="MP4" dims="1920 x 1080" file-size="128 MB" mod-date="2026/08/25" mod-time="09:41" origin-badge force-state="hover" />
            <ListRowCard name="雪牧.wav" badge="WAV" dims="—" file-size="36 MB" mod-date="2026/08/26" mod-time="21:03" selected />
            <ListRowCard name="雪牧_自动.fla" badge="FLA" dims="1920 x 1080" file-size="12 MB" mod-date="2026/08/24" mod-time="16:07" linked-badge />
            <ListRowCard name="镜头1_提示词.md" badge="MD" dims="—" file-size="2.1 KB" mod-date="2026/08/27" mod-time="10:12" naming />
          </div>
          <span class="pv-cap">表头（文件大小 ↓ 降序）+ 行：常态 / hover / 选中 / 链接徽章 linkedBadge / 命名态 naming（整行品牌环，Esc 取消）</span>
        </div>
        <div class="pv-demo-cell" style="width: 856px">
          <ListHeader sort-key="name" sort-dir="asc" />
          <span class="pv-cap">表头 · 当前排序列 = 名称 ↑ 升序（箭头恒显；换向旋转 300ms）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">文件夹卡片（220×122，hover 仅底升六级；图标恒默认色——变色只留树容器；业务态 dimmed / cut / naming / dropInto）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" />
          </div>
          <span class="pv-cap">常态</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" force-state="hover" />
          </div>
          <span class="pv-cap">hover（通用/六级底）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" selected />
          </div>
          <span class="pv-cap">选中</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" dimmed />
          </div>
          <span class="pv-cap">拖拽源 dimmed（半透明 0.4）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" cut />
          </div>
          <span class="pv-cap">剪切中 cut（半透明 0.5）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" naming />
          </div>
          <span class="pv-cap">命名态 naming（整卡品牌环 + 输入框；Esc 保存，同树容器口径）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 220px; height: 122px">
            <FolderCard name="01_人物" :lines="folderLines" drop-into />
          </div>
          <span class="pv-cap">拖拽并入 dropInto（品牌/半透明 + 2px 品牌描边）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">档案卡片（250×250 正方形，角标 = 属性文字；右下 = 链接文件-徽章；业务态 dimmed / cut / naming / dropInto / picked / pickerSelf）</h3>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" linked-badge />
          </div>
          <span class="pv-cap">常态</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" linked-badge force-state="hover" />
          </div>
          <span class="pv-cap">hover</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" linked-badge selected />
          </div>
          <span class="pv-cap">选中</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" dimmed />
          </div>
          <span class="pv-cap">拖拽源 dimmed（半透明 0.4）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" cut />
          </div>
          <span class="pv-cap">剪切中 cut（半透明 0.5）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" naming />
          </div>
          <span class="pv-cap">命名态 naming（名称位换输入框，整卡品牌环）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" drop-into />
          </div>
          <span class="pv-cap">拖拽并入 dropInto（品牌/半透明罩 + 2px 品牌描边）</span>
        </div>
      </div>
      <div class="pv-demo-row">
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" picked />
          </div>
          <span class="pv-cap">选择模式已选 picked（压暗 30% + 绿圆大勾，速查 §12）</span>
        </div>
        <div class="pv-demo-cell">
          <div style="width: 250px">
            <ArchiveCard name="雪牧.archive" :cover-url="demoCover" badge="主角" :attrs="archiveAttrs" :stats="archiveStats" picker-self />
          </div>
          <span class="pv-cap">发起档案自身 pickerSelf（半透明 0.5 + 禁选）</span>
        </div>
      </div>
    </div>

    <div class="pv-group">
      <h3 class="pv-group-title">格式角标（统一样式：半透明-2 + 立体感-通用 + 背景模糊-小）</h3>
      <div class="pv-demo-row">
        <div v-for="b in ['FLA', 'MP4', 'WAV', 'TTF', '序列帧', '主角']" :key="b" class="pv-demo-cell">
          <FormatBadge :label="b" />
          <span class="pv-cap">{{ b }}</span>
        </div>
      </div>
    </div>
  </section>
</template>
