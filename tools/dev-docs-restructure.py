# -*- coding: utf-8 -*-
"""docs 二级瘦身：开发日志/当前状态 按月分子文件夹，踩坑按编号段分；根 Index 收敛为一行一月/一段。"""
import os, re, subprocess, sys, collections

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS = os.path.join(ROOT, "docs")

def git_mv(src, dst_dir):
    os.makedirs(dst_dir, exist_ok=True)
    subprocess.run(["git", "mv", src, os.path.join(dst_dir, os.path.basename(src))], check=True, cwd=ROOT)

def read(p):
    with open(p, encoding="utf-8") as f: return f.read()

def write(p, s):
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, "w", encoding="utf-8", newline="\n") as f: f.write(s)

# ---------- 1. 开发日志 ----------
log_dir = os.path.join(DOCS, "02_开发日志")
idx = read(os.path.join(log_dir, "Index.md"))
lines = idx.split("\n")
bullets = [l for l in lines if l.startswith("- **20")]
n_links = sum(len(re.findall(r"\]\(2026-[^)]*\.md\)", l)) for l in bullets)
assert n_links == 462, f"开发日志 Index 链接数 {n_links} != 462"
by_month = collections.OrderedDict()
for l in bullets:
    m = re.match(r"- \*\*(2026-\d\d)-\d\d\*\*", l)
    assert m, l[:60]
    by_month.setdefault(m.group(1), []).append(l)

moved = [f for f in os.listdir(log_dir) if re.match(r"2026-\d\d-\d\d.*\.md$", f)]
assert len(moved) == 462, f"开发日志文件数 {len(moved)} != 462"
for f in moved:
    month = f[:7]
    git_mv(os.path.join(log_dir, f), os.path.join(log_dir, month))

month_links = []
for month, bl in by_month.items():
    body = (f"# {month} 开发日志\n\n"
            f"> 本月 {len(bl)} 个任务文件（命名 `YYYY-MM-DD[_NN]_标题.md`）。总目录见 [Index](../Index.md)。\n\n"
            + "\n".join(bl) + "\n")
    write(os.path.join(log_dir, month, "Index.md"), body)
    month_links.append(f"- **{month}**（{len(bl)} 个任务）→ [{month}/Index.md]({month}/Index.md)")

root = ("# 02_开发日志 Index\n\n"
        "> 做了什么、为什么、决策依据。**一个任务一个文件**（2026-09-07 起施行），命名 `YYYY-MM-DD[_NN]_标题.md`，"
        "按月份分文件夹存放（`YYYY-MM/`），月 Index 列任务清单，本文件只管月份导航。写新日志照此拆（开发规范第 4 条）。\n\n"
        + "\n".join(month_links) + "\n")
write(os.path.join(log_dir, "Index.md"), root)

# ---------- 2. 当前状态 ----------
cur_dir = os.path.join(DOCS, "03_项目进度", "当前状态")
idx = read(os.path.join(cur_dir, "Index.md"))
lines = idx.split("\n")
cut = lines.index("## 条目总表（按日期倒序）")
preamble = lines[:cut]
rows = [l for l in lines[cut:] if l.startswith("| [20")]
assert len(rows) == 124, f"当前状态表行数 {len(rows)} != 124"
by_month = collections.OrderedDict()
for l in rows:
    m = re.search(r"\]\((2026-\d\d)-\d\d_", l)
    assert m, l[:60]
    by_month.setdefault(m.group(1), []).append(l)

moved = [f for f in os.listdir(cur_dir) if re.match(r"2026-\d\d-\d\d.*\.md$", f)]
for f in moved:
    git_mv(os.path.join(cur_dir, f), os.path.join(cur_dir, f[:7]))

month_links = []
for month, rl in by_month.items():
    body = (f"# 当前状态 · {month}\n\n"
            f"> 本月 {len(rl)} 条状态记录。返回 [当前状态 Index](../Index.md)。\n\n"
            "| 文件 | 日期 | 标题 |\n| --- | --- | --- |\n"
            + "\n".join(rl) + "\n")
    write(os.path.join(cur_dir, month, "Index.md"), body)
    month_links.append(f"- **{month}**（{len(rl)} 条）→ [{month}/Index.md]({month}/Index.md)")

root = "\n".join(preamble).rstrip("\n") + "\n\n## 条目总表（按月份）\n\n" + "\n".join(month_links) + "\n"
write(os.path.join(cur_dir, "Index.md"), root)

# ---------- 3. 踩坑记录 ----------
pit_dir = os.path.join(DOCS, "04_踩坑记录")
idx = read(os.path.join(pit_dir, "Index.md"))
lines = idx.split("\n")
head = [l for l in lines if l.startswith("# 06_踩坑记录 Index") or l.startswith("> 四段式")]
assert len(head) == 2, head
rows = [l for l in lines if re.match(r"\| \d{3} ", l)]
assert len(rows) == 69, f"踩坑表行数 {len(rows)} != 69"
by_seg = collections.OrderedDict()
for l in rows:
    n = int(l[2:5])
    seg = f"{(n - 1) // 30 * 30 + 1:03d}-{(n - 1) // 30 * 30 + 30:03d}"
    by_seg.setdefault(seg, []).append(l)

moved = [f for f in os.listdir(pit_dir) if re.match(r"\d{3}_.*\.md$", f)]
for f in moved:
    n = int(f[:3])
    seg = f"{(n - 1) // 30 * 30 + 1:03d}-{(n - 1) // 30 * 30 + 30:03d}"
    git_mv(os.path.join(pit_dir, f), os.path.join(pit_dir, seg))

seg_links = []
for seg, rl in by_seg.items():
    body = (f"# 踩坑记录 {seg}\n\n"
            f"> 四段式：现象 / 原因 / 解法 / 预防。共 {len(rl)} 条。返回 [Index](../Index.md)。\n\n"
            "| 编号 | 标题 | 日期 | 链接 |\n| --- | --- | --- | --- |\n"
            + "\n".join(rl) + "\n")
    write(os.path.join(pit_dir, seg, "Index.md"), body)
    seg_links.append(f"- **{seg}**（{len(rl)} 条）→ [{seg}/Index.md]({seg}/Index.md)")

root = ("# 04_踩坑记录 Index\n\n"
        "> 四段式：现象 / 原因 / 解法 / 预防。踩到即记，写完把能沉淀的预防措施补进 `开发规范`。"
        "按编号每 30 条一个子文件夹，子 Index 列明细，本文件只管分段导航。\n\n"
        + "\n".join(seg_links) + "\n")
write(os.path.join(pit_dir, "Index.md"), root)

# ---------- 4. 移动后修正相对链接深度（](../ → ](../../） ----------
fixed = 0
for d in [os.path.join(DOCS, "02_开发日志"), os.path.join(DOCS, "03_项目进度", "当前状态")]:
    for month in sorted(os.listdir(d)):
        md = os.path.join(d, month)
        if not os.path.isdir(md): continue
        for f in os.listdir(md):
            if not f.endswith(".md"): continue
            p = os.path.join(md, f)
            s = read(p)
            n = s.count("](../")
            if n:
                write(p, s.replace("](../", "](../../"))
                fixed += n
print("OK: 开发日志 462 → 月文件夹; 当前状态 124 → 月文件夹; 踩坑 69 → 编号段; 链接修正", fixed, "处")
