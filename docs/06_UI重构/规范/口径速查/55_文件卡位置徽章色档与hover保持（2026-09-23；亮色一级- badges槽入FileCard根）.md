# 55. 文件卡位置徽章色档与 hover 保持（2026-09-23；亮色/一级 + badges 槽入 FileCard 根）

2026-09-23

- **位置徽章色档 = 亮色/一级**（2026-09-23 用户实机定：三级在深底图上看不清）。对象 = AssetCard 壳层 `.card-badges`（origin ★ / 链接 icon，currentColor），`--n-l3` → `--n-l1`。档案卡右下 linked-file-badge（§16 二级档）与 FileCard `originBadge` prop（kc-corner-badge，品牌色原值 SVG）均不在本次范围，不动。
- **hover 徽章 = 卡 hover 态保持**（同日用户实机报脱节）：旧结构徽章在 AssetCard 壳层、**非 `.kc-media` 子树**——hover 徽章时 `.kc-media:hover` 丢失，卡芯位移/阴影/描边回退，而徽章跟随规则 `.asset-card:hover .card-badges` 仍成立 → 卡回去了徽章悬在原位，视觉脱节。
  - **修法 = 结构，不复制样式**：FileCard 加 named slot `badges`（kc-clip 裁剪层外、根内末尾，originBadge 旁），AssetCard 把 `.card-badges` 挪入该槽 → 徽章成 `.kc-media` 后代，hover 徽章即 `.kc-media:hover` 保持，位移/阴影/描边/悬停控件全部不回退。
  - **入槽后壳层位移补偿必须同删**（同日二轮走查反馈：徽章位移比卡片大——徽章随卡芯天然 -4px，再叠 `.asset-card:hover .card-badges` 的 -4px = -8px 双份）：`.card-badges` 从壳层附件 transition/transform 组剔除；picked 压暗规则保留（徽章在 clip 外，不被卡内 picked 压暗规则覆盖）。笔记角标 / 已选大勾仍壳层，跟随补偿照旧。
  - 定位几何不变（FileCard 根 100% 撑满壳，right/bottom 12 参照系重合）；LinkListPopover 仍挂壳层，锚点 ref 照常。
  - **教训口径：壳层可交互附件（要吃 hover 的）必须进卡芯子树**；纯展示附件（笔记角标 pointer-events none / 已选大勾）不挡命中，可留壳层。
- **探针零改动**：cdp-link-manage-probe.py 三处 `.card-badges` 断言均为 `.asset-card` 后代查询 + dispatchEvent，层级加深不命中失败。
