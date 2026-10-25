from __future__ import annotations

import csv
import json
import os
from pathlib import Path
import re

GROUPS = {
    "sink": ["sink", "faucet", "tap", "water"],
    "washing": ["wash", "washing", "clean", "cleaning", "scrub", "rinse"],
    "dishware": ["dish", "dishes", "plate", "bowl", "cup", "spoon", "fork", "knife", "utensil"],
    "pot_pan": ["pot", "pan", "lid"],
    "sponge_brush": ["sponge", "brush"],
    "cloth": ["cloth", "rag", "towel", "napkin"],
    "kitchen_ops": ["microwave", "oven", "stove", "drawer", "cabinet", "dishwasher"],
}
FRAME = re.compile(r"/im_(\d+)\.jpg$")


def main() -> None:
    root = Path(os.environ["EVAL_ROOT"])
    rows = []
    row_lengths = {}
    with (root / ".cache" / "traj_links.csv").open(newline="", encoding="utf-8-sig") as handle:
        for line_number, row in enumerate(csv.reader(handle), 1):
            if not row or not any(row):
                continue
            row_lengths[len(row)] = row_lengths.get(len(row), 0) + 1
            if len(row) < 3:
                raise RuntimeError(f"Unexpected CSV row {line_number}: {row}")
            initial, final = row[0], row[1]
            match = FRAME.search(final)
            if "/images0/" not in initial or not match:
                raise RuntimeError(f"Unexpected paths at row {line_number}: {row[:2]}")
            rows.append({
                "line_number": line_number,
                "trajectory_key": initial.split("/images0/")[0],
                "initial": initial,
                "final": final,
                "last_frame_index": int(match.group(1)),
                "annotations": [value for value in row[2:] if value],
            })
    report = {"row_count": len(rows), "row_lengths": row_lengths, "groups": {}}
    for group, keywords in GROUPS.items():
        matches = [
            row for row in rows
            if any(keyword in " ".join(row["annotations"]).lower() for keyword in keywords)
        ]
        report["groups"][group] = {
            "count": len(matches),
            "sample": matches[:40],
        }
    output = root / "logs" / "source-stats.json"
    output.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({
        "row_count": len(rows),
        "row_lengths": row_lengths,
        "group_counts": {name: value["count"] for name, value in report["groups"].items()},
    }, indent=2))


if __name__ == "__main__":
    main()
