from __future__ import annotations

import json
import os
from pathlib import Path

PRIMARY_KEYS = ("top1_hit_rate", "top5_hit_rate", "top10_hit_rate", "mrr")


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def ratio(current: float, baseline: float) -> float:
    return current / baseline


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    baseline_root = root.parent / "semantic-multilingual"
    current_quality = load(root / "outputs/quality-cpu.json")
    baseline_quality = load(baseline_root / "outputs/quality-cpu.json")
    assert current_quality["status"] == baseline_quality["status"] == "passed"
    current_data = current_quality["quality"]
    baseline_data = baseline_quality["quality"]
    assert current_data["dataset"] == baseline_data["dataset"]
    assert current_data["dataset"]["regular_query_count"] == 17
    assert current_data["dataset"]["diagnostic_query_count"] == 1
    diagnostic_ids = [item["id"] for item in current_data["diagnostic_queries"]]
    assert diagnostic_ids == ["robot_washes_dishes_diagnostic"]

    metrics = {}
    for key in PRIMARY_KEYS:
        current = current_data["aggregate"][key]
        baseline = baseline_data["aggregate"][key]
        metrics[key] = {
            "siglip": current,
            "multilingual_v1": baseline,
            "absolute_delta": current - baseline,
            "relative_delta": current / baseline - 1.0,
            "query_count_delta": round((current - baseline) * 17) if key != "mrr" else None,
        }

    baseline_ranks = {
        item["id"]: item for item in baseline_data["per_query"]
    }
    per_query = []
    for item in current_data["per_query"]:
        baseline = baseline_ranks[item["id"]]
        per_query.append({
            "id": item["id"],
            "text": item["text"],
            "siglip_rank": item["best_positive_rank"],
            "multilingual_rank": baseline["best_positive_rank"],
            "rank_delta_positive_is_worse": item["best_positive_rank"] - baseline["best_positive_rank"],
            "siglip_reciprocal_rank": item["reciprocal_rank"],
            "multilingual_reciprocal_rank": baseline["reciprocal_rank"],
        })

    current_manifest = load(root / "outputs/model-manifest.json")
    baseline_manifest = load(baseline_root / "outputs/model-manifest.json")

    def sizes(manifest: dict) -> dict[str, int]:
        files = {Path(item["path"]).name: item["bytes"] for item in manifest["files"]}
        return {
            "text": files["text.onnx"],
            "vision": files["vision.onnx"],
            "tokenizer": files["tokenizer.json"],
            "models": files["text.onnx"] + files["vision.onnx"],
            "deployment_total": files["text.onnx"] + files["vision.onnx"] + files["tokenizer.json"],
        }

    current_sizes = sizes(current_manifest)
    baseline_sizes = sizes(baseline_manifest)
    size_comparison = {
        key: {
            "siglip_bytes": current_sizes[key],
            "multilingual_bytes": baseline_sizes[key],
            "delta_bytes": current_sizes[key] - baseline_sizes[key],
            "ratio": ratio(current_sizes[key], baseline_sizes[key]),
        }
        for key in current_sizes
    }

    performance = {}
    for backend in ("cpu", "directml"):
        current = load(root / f"outputs/rust-{backend}.json")
        baseline = load(baseline_root / f"outputs/rust-{backend}.json")
        current_t = current["timings_ms"]
        baseline_t = baseline["timings_ms"]
        performance[backend] = {
            "cold_start_to_first_text_ms": {
                "siglip": current_t["cold_start_to_first_text"],
                "multilingual": baseline_t["cold_start_to_first_text"],
                "ratio": ratio(current_t["cold_start_to_first_text"], baseline_t["cold_start_to_first_text"]),
            },
            "text_warmed": {
                "siglip": {key: current_t["text_warmed"][key] for key in ("p50_ms", "p95_ms")},
                "multilingual": {key: baseline_t["text_warmed"][key] for key in ("p50_ms", "p95_ms")},
                "p50_ratio": ratio(current_t["text_warmed"]["p50_ms"], baseline_t["text_warmed"]["p50_ms"]),
                "p95_ratio": ratio(current_t["text_warmed"]["p95_ms"], baseline_t["text_warmed"]["p95_ms"]),
            },
            "vision_warmed": {
                "siglip": {key: current_t["vision_warmed"][key] for key in ("p50_ms", "p95_ms")},
                "multilingual": {key: baseline_t["vision_warmed"][key] for key in ("p50_ms", "p95_ms")},
                "p50_ratio": ratio(current_t["vision_warmed"]["p50_ms"], baseline_t["vision_warmed"]["p50_ms"]),
                "p95_ratio": ratio(current_t["vision_warmed"]["p95_ms"], baseline_t["vision_warmed"]["p95_ms"]),
            },
            "peak_rss": {
                "siglip_bytes": current["memory"]["final"]["peak_rss_bytes"],
                "multilingual_bytes": baseline["memory"]["final"]["peak_rss_bytes"],
                "ratio": ratio(current["memory"]["final"]["peak_rss_bytes"], baseline["memory"]["final"]["peak_rss_bytes"]),
            },
        }

    report = {
        "dataset": current_data["dataset"],
        "diagnostic_query_excluded": diagnostic_ids,
        "metrics": metrics,
        "hard_negatives": {
            "siglip": current_data["aggregate"]["hard_negatives"],
            "multilingual_v1": baseline_data["aggregate"]["hard_negatives"],
        },
        "per_query": per_query,
        "improved_rank_query_ids": [item["id"] for item in per_query if item["rank_delta_positive_is_worse"] < 0],
        "worsened_rank_query_ids": [item["id"] for item in per_query if item["rank_delta_positive_is_worse"] > 0],
        "unchanged_rank_query_ids": [item["id"] for item in per_query if item["rank_delta_positive_is_worse"] == 0],
        "size": size_comparison,
        "embedding_dimensions": {
            "siglip": current_manifest["embedding_dimension"],
            "multilingual_v1": baseline_manifest["embedding_dimension"],
            "ratio": current_manifest["embedding_dimension"] / baseline_manifest["embedding_dimension"],
        },
        "performance": performance,
        "typical_error_count": len(current_data["typical_errors"]),
    }
    (root / "outputs/comparison.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (root / "outputs/typical-errors.json").write_text(
        json.dumps({"schema_version": 1, "errors": current_data["typical_errors"]}, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
    )
    print(json.dumps({
        "metrics": metrics,
        "improved_rank_query_ids": report["improved_rank_query_ids"],
        "worsened_rank_query_ids": report["worsened_rank_query_ids"],
        "unchanged_rank_query_ids": report["unchanged_rank_query_ids"],
        "size": size_comparison,
        "performance": performance,
        "typical_error_count": report["typical_error_count"],
    }, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
