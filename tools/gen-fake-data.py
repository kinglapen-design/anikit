# -*- coding: utf-8 -*-
"""生成 10 万条假素材数据（阶段 0 · 任务 4 滚动原型）

输出：spike/scroll-proto/out/fake-items.json（已被 .gitignore 排除）
"""
import json
import os
import random

random.seed(42)

NAMES = ["角色", "场景", "道具", "表情", "走路", "跑步", "特效", "背景", "分镜", "动作"]
EXTS = ["png", "jpg", "fla", "webp", "psd", "gif"]
RATIOS = [(1920, 1080), (1080, 1920), (1000, 1000), (800, 1200), (1200, 800), (640, 640)]
COLORS = ["#3a7bfd", "#e06c75", "#98c379", "#d19a66", "#61afef", "#c678dd", "#56b6c2", "#be8c66"]

items = []
for i in range(100000):
    w, h = random.choice(RATIOS)
    items.append({
        "id": i,
        "name": "%s-%05d.%s" % (random.choice(NAMES), i, random.choice(EXTS)),
        "w": w,
        "h": h,
        "color": random.choice(COLORS),
    })

out = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "spike", "scroll-proto", "out")
os.makedirs(out, exist_ok=True)
path = os.path.join(out, "fake-items.json")
with open(path, "w", encoding="utf-8") as f:
    json.dump(items, f, ensure_ascii=False, separators=(",", ":"))
print("已生成 %d 条 -> %s（%.1f MB）" % (len(items), path, os.path.getsize(path) / 1024 / 1024))
