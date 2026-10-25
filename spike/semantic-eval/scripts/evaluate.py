from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import time

import numpy as np
import onnxruntime as ort
from PIL import Image
from tokenizers import Tokenizer


def parse_args() -> argparse.Namespace:
    root = Path(os.environ["EVAL_ROOT"])
    clip = Path(os.environ["CLIP_ROOT"])
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, default=root / "manifest" / "quality-manifest.json")
    parser.add_argument("--queries", type=Path, default=root / "manifest" / "queries.json")
    parser.add_argument("--model-dir", type=Path, default=clip / "models")
    parser.add_argument("--model-manifest", type=Path, default=clip / "outputs" / "model-manifest.json")
    parser.add_argument("--output", type=Path, default=root / "outputs" / "chinese-clip")
    parser.add_argument("--text-model", default="text.onnx")
    parser.add_argument("--vision-model", default="vision.onnx")
    parser.add_argument("--tokenizer", default="tokenizer.json")
    parser.add_argument("--text-input", default="text")
    parser.add_argument("--text-output", default="unnorm_text_features")
    parser.add_argument("--image-input", default="image")
    parser.add_argument("--image-output", default="unnorm_image_features")
    parser.add_argument("--context-length", type=int, default=52)
    parser.add_argument("--language", choices=("zh", "en"), default="zh")
    return parser.parse_args()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def normalize(value: np.ndarray) -> np.ndarray:
    return value / np.linalg.norm(value, axis=-1, keepdims=True)


def percentile(values: list[float], quantile: float) -> float:
    ordered = sorted(values)
    position = (len(ordered) - 1) * quantile
    lower = int(position)
    upper = min(lower + 1, len(ordered) - 1)
    fraction = position - lower
    return ordered[lower] * (1.0 - fraction) + ordered[upper] * fraction


def main() -> None:
    args = parse_args()
    root = Path(os.environ["EVAL_ROOT"])
    clip = Path(os.environ["CLIP_ROOT"])
    sys.path.insert(0, str(clip / "vendor" / "Chinese-CLIP" / "cn_clip"))
    sys.path.insert(0, str(clip / "vendor" / "Chinese-CLIP"))
    from clip.utils import image_transform

    manifest_path = args.manifest.resolve()
    queries_path = args.queries.resolve()
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    query_document = json.loads(queries_path.read_text(encoding="utf-8"))
    if not str(manifest.get("review_status", "")).startswith("reviewed_all"):
        raise RuntimeError("Quality manifest has not been visually reviewed")
    trajectories = manifest["trajectories"]
    frames = {}
    for trajectory in trajectories:
        for frame in trajectory["frames"]:
            frames[frame["image_id"]] = (trajectory, frame)
    if len(frames) != manifest["image_count"]:
        raise RuntimeError("Manifest image count or image IDs are inconsistent")

    def resolve(selector: dict) -> list[str]:
        if "image_ids" in selector:
            result = list(selector["image_ids"])
        elif "tags_any" in selector:
            tags = set(selector["tags_any"])
            selected_frames = set(selector.get("frames", ("first", "mid", "last")))
            result = [
                frame["image_id"]
                for trajectory in trajectories
                if tags.intersection(trajectory["visual_review"].get("scene_tags", []))
                for frame in trajectory["frames"]
                if frame["frame_label"] in selected_frames
            ]
        else:
            samples = set(selector.get("samples", []))
            selected_frames = set(selector.get("frames", ("first", "mid", "last")))
            result = [
                frame["image_id"]
                for trajectory in trajectories
                if trajectory["sample_id"] in samples
                for frame in trajectory["frames"]
                if frame["frame_label"] in selected_frames
            ]
        unknown = set(result) - frames.keys()
        if unknown:
            raise RuntimeError(f"Unknown image IDs in selector {selector}: {sorted(unknown)}")
        if len(result) != len(set(result)):
            raise RuntimeError(f"Duplicate image IDs in selector {selector}")
        return result

    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    options = ort.SessionOptions()
    options.intra_op_num_threads = 4
    options.log_severity_level = 2
    text_session = ort.InferenceSession(
        str(args.model_dir / args.text_model), sess_options=options, providers=["CPUExecutionProvider"]
    )
    vision_session = ort.InferenceSession(
        str(args.model_dir / args.vision_model), sess_options=options, providers=["CPUExecutionProvider"]
    )
    if text_session.get_providers() != ["CPUExecutionProvider"] or vision_session.get_providers() != ["CPUExecutionProvider"]:
        raise RuntimeError("Evaluation must use the explicitly requested CPU provider")
    tokenizer = Tokenizer.from_file(str(args.model_dir / args.tokenizer))
    preprocess = image_transform(224)

    query_embeddings = []
    query_timings = []
    text_key = "text_zh" if args.language == "zh" else "text_en"
    for query in query_document["queries"]:
        encoding = tokenizer.encode(query[text_key])
        ids = list(encoding.ids)
        if len(ids) != args.context_length:
            raise RuntimeError(f"Tokenizer returned {len(ids)} IDs for {query['id']}, expected {args.context_length}")
        model_input = np.asarray([ids], dtype=np.int64)
        start = time.perf_counter()
        raw = text_session.run([args.text_output], {args.text_input: model_input})[0]
        query_timings.append((time.perf_counter() - start) * 1000.0)
        if raw.ndim != 2 or raw.shape[0] != 1 or not np.isfinite(raw).all():
            raise RuntimeError(f"Invalid text output for {query['id']}: {raw.shape}")
        query_embeddings.append(normalize(raw)[0])

    image_ids = list(frames)
    image_embeddings = []
    image_timings = []
    for index, image_id in enumerate(image_ids, 1):
        trajectory, frame = frames[image_id]
        image_path = root / frame["path"]
        with Image.open(image_path) as image:
            model_input = np.ascontiguousarray(preprocess(image).unsqueeze(0).numpy(), dtype=np.float32)
        start = time.perf_counter()
        raw = vision_session.run([args.image_output], {args.image_input: model_input})[0]
        image_timings.append((time.perf_counter() - start) * 1000.0)
        if raw.ndim != 2 or raw.shape[0] != 1 or not np.isfinite(raw).all():
            raise RuntimeError(f"Invalid image output for {image_id}: {raw.shape}")
        image_embeddings.append(normalize(raw)[0])
        print(f"[{index}/{len(image_ids)}] {image_id}: {trajectory['annotations']}", flush=True)

    image_matrix = np.asarray(image_embeddings, dtype=np.float32)
    rankings = []
    typical_errors = []
    aggregate_queries = []
    hard_negative_groups = {"subject_match_action_mismatch": [], "action_match_subject_mismatch": []}
    for query_index, query in enumerate(query_document["queries"]):
        similarities = image_matrix @ query_embeddings[query_index]
        order = sorted(range(len(image_ids)), key=lambda index: (-float(similarities[index]), image_ids[index]))
        ranks = {image_ids[index]: rank + 1 for rank, index in enumerate(order)}
        positive_ids = resolve(query["positive"])
        positive_set = set(positive_ids)
        if not query.get("diagnostic_only") and not positive_ids:
            raise RuntimeError(f"Non-diagnostic query has no positives: {query['id']}")
        if positive_ids and not np.isfinite(similarities).all():
            raise RuntimeError(f"Non-finite similarities for {query['id']}")
        best_positive_rank = min((ranks[image_id] for image_id in positive_ids), default=None)
        best_positive_score = max((float(similarities[image_ids.index(image_id)]) for image_id in positive_ids), default=None)
        top10 = []
        for rank, index in enumerate(order[:10], 1):
            image_id = image_ids[index]
            trajectory, frame = frames[image_id]
            top10.append({
                "rank": rank,
                "image_id": image_id,
                "similarity": float(similarities[index]),
                "is_positive": image_id in positive_set,
                "sample_id": trajectory["sample_id"],
                "category": trajectory["category"],
                "frame_index": frame["frame_index"],
                "annotations": trajectory["annotations"],
            })
        record = {
            "id": query["id"],
            "text": query[text_key],
            "annotation_basis": query["annotation_basis"],
            "diagnostic_only": bool(query.get("diagnostic_only")),
            "positive_image_ids": positive_ids,
            "positive_count": len(positive_ids),
            "best_positive_rank": best_positive_rank,
            "reciprocal_rank": 0.0 if best_positive_rank is None else 1.0 / best_positive_rank,
            "hits": {str(k): best_positive_rank is not None and best_positive_rank <= k for k in (1, 5, 10)},
            "top10": top10,
            "hard_negatives": {},
        }
        for class_name, selector in query.get("hard_negatives", {}).items():
            negative_ids = resolve(selector)
            overlap = positive_set.intersection(negative_ids)
            if overlap:
                raise RuntimeError(f"Positive/hard-negative overlap for {query['id']}: {sorted(overlap)}")
            if not negative_ids:
                continue
            best_negative_id = min(negative_ids, key=lambda image_id: ranks[image_id])
            best_negative_score = float(similarities[image_ids.index(best_negative_id)])
            negative_record = {
                "basis": selector["basis"],
                "image_ids": negative_ids,
                "best_image_id": best_negative_id,
                "best_rank": ranks[best_negative_id],
                "best_similarity": best_negative_score,
                "margin_best_positive_minus_best_negative": best_positive_score - best_negative_score,
                "positive_outscores_negative": best_positive_score > best_negative_score,
                "count_before_first_positive": sum(ranks[image_id] < best_positive_rank for image_id in negative_ids),
            }
            record["hard_negatives"][class_name] = negative_record
            hard_negative_groups[class_name].append(negative_record)
            if not negative_record["positive_outscores_negative"] or negative_record["best_rank"] <= 5:
                typical_errors.append({
                    "query_id": query["id"],
                    "type": class_name,
                    "best_positive_rank": best_positive_rank,
                    **negative_record,
                })
        if not query.get("diagnostic_only"):
            aggregate_queries.append(record)
        failed_top = [item for item in top10 if not item["is_positive"]][:3]
        if failed_top and (best_positive_rank is None or best_positive_rank > 1):
            typical_errors.append({
                "query_id": query["id"],
                "type": "top_ranked_nonpositives",
                "best_positive_rank": best_positive_rank,
                "items": failed_top,
            })
        rankings.append(record)

    def aggregate_hard_negative(records: list[dict]) -> dict:
        if not records:
            return {"query_count": 0}
        margins = [record["margin_best_positive_minus_best_negative"] for record in records]
        return {
            "query_count": len(records),
            "positive_outscore_rate": sum(record["positive_outscores_negative"] for record in records) / len(records),
            "hard_negative_top1_rate": sum(record["best_rank"] == 1 for record in records) / len(records),
            "mean_margin": float(np.mean(margins)),
            "min_margin": min(margins),
            "max_margin": max(margins),
        }

    query_count = len(aggregate_queries)
    metrics = {
        "schema_version": 1,
        "model": {
            "model_dir": str(args.model_dir.resolve()),
            "text_model": args.text_model,
            "vision_model": args.vision_model,
            "tokenizer": args.tokenizer,
            "providers": ["CPUExecutionProvider"],
            "language": args.language,
            "context_length": args.context_length,
            "model_manifest": str(args.model_manifest.resolve()),
            "model_manifest_sha256": sha256(args.model_manifest),
        },
        "dataset": {
            "manifest": str(manifest_path),
            "manifest_sha256": sha256(manifest_path),
            "queries": str(queries_path),
            "queries_sha256": sha256(queries_path),
            "trajectory_count": manifest["trajectory_count"],
            "image_count": len(image_ids),
            "regular_query_count": query_count,
            "diagnostic_query_count": len(rankings) - query_count,
            "dishwashing_assessment": manifest["dishwashing_assessment"],
        },
        "metric_definitions": query_document["metric_definitions"],
        "aggregate": {
            "top1_hit_rate": sum(record["hits"]["1"] for record in aggregate_queries) / query_count,
            "top5_hit_rate": sum(record["hits"]["5"] for record in aggregate_queries) / query_count,
            "top10_hit_rate": sum(record["hits"]["10"] for record in aggregate_queries) / query_count,
            "mrr": float(np.mean([record["reciprocal_rank"] for record in aggregate_queries])),
            "hard_negatives": {name: aggregate_hard_negative(records) for name, records in hard_negative_groups.items()},
        },
        "per_query": [{
            "id": record["id"],
            "best_positive_rank": record["best_positive_rank"],
            "reciprocal_rank": record["reciprocal_rank"],
            "hits": record["hits"],
            "hard_negatives": {
                name: {
                    "best_rank": value["best_rank"],
                    "margin": value["margin_best_positive_minus_best_negative"],
                    "positive_outscores_negative": value["positive_outscores_negative"],
                }
                for name, value in record["hard_negatives"].items()
            },
        } for record in aggregate_queries],
        "diagnostic_queries": [{"id": record["id"], "positive_count": 0, "top10": record["top10"]} for record in rankings if record["diagnostic_only"]],
        "inference_timing_ms": {
            "text": {
                "count": len(query_timings),
                "p50": percentile(query_timings, 0.50),
                "p95": percentile(query_timings, 0.95),
                "mean": float(np.mean(query_timings)),
            },
            "vision": {
                "count": len(image_timings),
                "p50": percentile(image_timings, 0.50),
                "p95": percentile(image_timings, 0.95),
                "mean": float(np.mean(image_timings)),
            },
        },
    }
    (output / "metrics.json").write_text(json.dumps(metrics, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output / "rankings.json").write_text(json.dumps({"schema_version": 1, "queries": rankings}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (output / "typical-errors.json").write_text(json.dumps({"schema_version": 1, "errors": typical_errors}, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    np.savez(
        output / "embeddings.npz",
        image_ids=np.asarray(image_ids),
        image_embeddings=image_matrix,
        query_ids=np.asarray([query["id"] for query in query_document["queries"]]),
        query_embeddings=np.asarray(query_embeddings, dtype=np.float32),
    )
    print(json.dumps(metrics["aggregate"], ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
