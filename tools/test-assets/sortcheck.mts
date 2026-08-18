// sortPlacements 自然排序逻辑自验（node --experimental-strip-types 直跑；3.5 一次性验证脚本）
import { naturalCompare, parseCnNumber, sortPlacements } from "../../src/lib/sortPlacements.ts";

let failed = 0;
function eq(actual, expect, label) {
  const ok = JSON.stringify(actual) === JSON.stringify(expect);
  if (!ok) {
    failed++;
    console.error(`FAIL ${label}: 期望 ${JSON.stringify(expect)}，实得 ${JSON.stringify(actual)}`);
  } else {
    console.log(`PASS ${label}`);
  }
}
function sign(n) { return n === 0 ? 0 : n > 0 ? 1 : -1; }

// —— 中文数字解析 ——
eq(parseCnNumber("一"), 1, "一=1");
eq(parseCnNumber("十"), 10, "十=10");
eq(parseCnNumber("十一"), 11, "十一=11");
eq(parseCnNumber("二十"), 20, "二十=20");
eq(parseCnNumber("二十一"), 21, "二十一=21");
eq(parseCnNumber("一百零五"), 105, "一百零五=105");
eq(parseCnNumber("二百五十"), 250, "二百五十=250");
eq(parseCnNumber("一千零二十"), 1020, "一千零二十=1020");
eq(parseCnNumber("一万零二十"), 10020, "一万零二十=10020");

// —— 自然排序比较 ——
eq(sign(naturalCompare("文件2.png", "文件10.png")), -1, "数字串按数值 2<10");
eq(sign(naturalCompare("走姿二", "走姿十")), -1, "中文数字 二<十");
eq(sign(naturalCompare("走姿九", "走姿十一")), -1, "中文数字 九<十一");
eq(sign(naturalCompare("镜头甲", "镜头乙")), -1, "天干 甲<乙");
eq(sign(naturalCompare("镜头癸", "镜头甲")), 1, "天干 癸>甲");
eq(sign(naturalCompare("文件2", "文件十")), -1, "跨体系数值 2<十");
eq(sign(naturalCompare("a.png", "a.png")), 0, "全等=0");
eq(sign(naturalCompare("第1章", "第2章")), -1, "前缀相同比数字");
eq(naturalCompare("b.png", "a.png") > 0, true, "普通文本比较");

// —— sortPlacements 行为 ——
const mk = (assetId, over = {}) => ({
  assetId, folderId: "F", name: `${assetId}.png`, w: null, h: null, ext: "png",
  thumbPath: null, thumbKind: null, origin: true, frameCount: null, animated: false,
  duration: null, bpm: null, importedFrom: null, size: 100,
  createdAt: null, modifiedAt: "2026-08-19T10:00:00+08:00", importedAt: "2026-08-19T10:00:00+08:00",
  ...over,
});
const ids = (list) => list.map((p) => p.assetId);

// 自定义 = 原序不动
eq(ids(sortPlacements([mk("B"), mk("A")], "", true)), ["B", "A"], "自定义原序");

// 名称自然排序
const byName = sortPlacements([mk("x10"), mk("x2"), mk("x一")].map((p) => ({ ...p, name: p.assetId })), "name", true);
eq(ids(byName), ["x一", "x2", "x10"], "名称升序 一<2<10");

// 大小升/降 + 平局按名称
const bySize = sortPlacements(
  [mk("b", { size: 200 }), mk("a", { size: 200 }), mk("c", { size: 50 })], "size", true,
);
eq(ids(bySize), ["c", "a", "b"], "大小升序，平局按名称");
eq(ids(sortPlacements([mk("b", { size: 200 }), mk("a", { size: 200 }), mk("c", { size: 50 })], "size", false)),
  ["a", "b", "c"], "大小降序，平局名称不反转");

// 空值恒排尾（升降都在尾）
const withNull = [mk("n"), mk("v1", { duration: 5 }), mk("v2", { duration: 1 })];
eq(ids(sortPlacements(withNull, "duration", true)), ["v2", "v1", "n"], "时长升序空值排尾");
eq(ids(sortPlacements(withNull, "duration", false)), ["v1", "v2", "n"], "时长降序空值仍排尾");

// 创建时间 null 排尾
const withNullC = [mk("old", { createdAt: null }), mk("new", { createdAt: "2026-01-01T00:00:00+08:00" })];
eq(ids(sortPlacements(withNullC, "createdAt", true)), ["new", "old"], "创建时间 null 排尾");

// 时间字典序
const byTime = sortPlacements(
  [mk("a", { importedAt: "2026-08-19T10:00:00+08:00" }), mk("b", { importedAt: "2026-08-18T09:00:00+08:00" })],
  "importedAt", true,
);
eq(ids(byTime), ["b", "a"], "导入时间升序");

console.log(failed === 0 ? "\n全部通过" : `\n${failed} 项失败`);
process.exit(failed === 0 ? 0 : 1);
