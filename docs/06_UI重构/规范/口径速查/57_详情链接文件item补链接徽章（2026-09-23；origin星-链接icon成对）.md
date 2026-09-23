# 57. 详情链接文件 item 补链接徽章（2026-09-23；origin ★ / 链接 icon 成对）

2026-09-23

- **LinkFileItem 补链接半边徽章**：旧只有 `origin` prop（★ 品牌星右下，稿只画了 ★）；非 origin 行无标识（用户实机报「详情面板这里差链接标识」）。新增 `linked` prop（与 origin 互斥）：`linked-file-badge` 14 右下同位，色 **亮色/三级**（同日二轮用户定：详情面板纯底 l1 偏亮——**徽章色档按语境分：卡片压缩略图 = l1（复杂底需亮，§55），面板纯底行 = l3（与链接 tip 列表 LinkListRows 同档）**；★ 为品牌原值 SVG 不吃 color，两徽章互不影响）。
- **判定在调用方 TabLinks**：`:linked="!p.origin && linkItems.length > 1"`——活位置 > 1 才出（2026-09-05 走查同口径：单位置无链接可区分不出）。`.fname.has-badge` 右缩让位条件同步 `origin || linked`。
- 卡片（AssetCard）/ 链接 tip 列表（LinkListRows）/ 列表行（ListRowCard）/ 详情链接行（LinkFileItem）四处的 origin ★ / 链接 icon 徽章自此全部成对。**色档定案（同日二轮用户定）：卡片 = l1（压复杂缩略图需亮，§55），其余行类（LinkListRows / LinkFileItem）= l3**——同一图标不追求同色，按底色复杂度分档。
- 预览页 PreviewItems 补 linked 态陈列；探针零改动（cdp-sidebar-right-probe R10b 断言首行 origin 徽章，`querySelector` 取首个 .badge 按 DOM 序仍命中 ★）。
