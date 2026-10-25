from __future__ import annotations

import csv
import hashlib
from io import BytesIO
import json
import os
from pathlib import Path
import time
import urllib.request

from PIL import Image, ImageDraw

FRAME_LABELS = ("first", "mid", "last")


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path) -> bytes:
    if destination.is_file():
        return destination.read_bytes()
    destination.parent.mkdir(parents=True, exist_ok=True)
    last_error = None
    for attempt in range(3):
        try:
            request = urllib.request.Request(url, headers={"User-Agent": "semantic-eval-spike/1.0"})
            with urllib.request.urlopen(request, timeout=45) as response:
                value = response.read()
            if not value.startswith(b"\xff\xd8\xff"):
                raise RuntimeError(f"Not a JPEG: {url}")
            temporary = destination.with_suffix(".part")
            temporary.write_bytes(value)
            temporary.replace(destination)
            return value
        except Exception as error:
            last_error = error
            if attempt < 2:
                time.sleep(0.5 * (attempt + 1))
    raise RuntimeError(f"Download failed after 3 attempts: {url}: {last_error}")


def validate_jpeg(value: bytes) -> tuple[str, int, int]:
    with Image.open(BytesIO(value)) as image:
        image.verify()
        format_name = image.format
    with Image.open(BytesIO(value)) as image:
        image.load()
        if image.format != "JPEG" or image.width <= 0 or image.height <= 0:
            raise RuntimeError("Unreadable or non-JPEG image")
        return format_name or "unknown", image.width, image.height


def main() -> None:
    root = Path(os.environ["EVAL_ROOT"])
    selection_path = root / "manifest" / "selection.json"
    selection = json.loads(selection_path.read_text(encoding="utf-8"))
    source = selection["source"]
    csv_path = root / ".cache" / "traj_links.csv"
    actual_csv_hash = sha256_file(csv_path)
    if actual_csv_hash != source["csv_sha256"]:
        raise RuntimeError(f"CSV hash changed: {actual_csv_hash}")
    review_path = root / "manifest" / "review.json"
    review = json.loads(review_path.read_text(encoding="utf-8")) if review_path.is_file() else {}
    review_samples = review.get("samples", {})
    selected_lines = {entry["csv_line"] for entry in selection["entries"]}
    if len(selected_lines) != len(selection["entries"]):
        raise RuntimeError("Duplicate CSV lines in selection")
    rows = {}
    with csv_path.open(newline="", encoding="utf-8-sig") as handle:
        for line_number, row in enumerate(csv.reader(handle), 1):
            if line_number in selected_lines:
                rows[line_number] = row
    if rows.keys() != selected_lines:
        raise RuntimeError(f"Missing CSV lines: {selected_lines - rows.keys()}")

    trajectories = []
    all_images = []
    for entry in selection["entries"]:
        row = rows[entry["csv_line"]]
        initial, final = row[0], row[1]
        if "/images0/" not in initial or not final.endswith(".jpg"):
            raise RuntimeError(f"Unexpected image paths at CSV line {entry['csv_line']}")
        last_index = int(final.rsplit("/im_", 1)[1].removesuffix(".jpg"))
        frame_indices = (0, last_index // 2, last_index)
        trajectory_key = initial.split("/images0/")[0]
        trajectory = {
            **entry,
            "trajectory_key": trajectory_key,
            "trajectory_url_prefix": source["download_prefix"] + trajectory_key + "/",
            "raw_csv_row": row,
            "annotations": [value for value in row[2:] if value],
            "last_frame_index": last_index,
            "visual_review": review_samples.get(entry["sample_id"], {"status": "pending"}),
            "frames": [],
        }
        for label, frame_index in zip(FRAME_LABELS, frame_indices, strict=True):
            source_path = f"{trajectory_key}/images0/im_{frame_index}.jpg"
            url = source["download_prefix"] + source_path
            relative_path = Path("data") / "images" / f"{entry['sample_id']}_{label}.jpg"
            destination = root / relative_path
            value = download(url, destination)
            format_name, width, height = validate_jpeg(value)
            frame = {
                "image_id": f"{entry['sample_id']}_{label}",
                "frame_label": label,
                "frame_index": frame_index,
                "source_path": source_path,
                "url": url,
                "path": str(relative_path).replace("\\", "/"),
                "format": format_name,
                "width": width,
                "height": height,
                "bytes": len(value),
                "sha256": sha256_bytes(value),
            }
            trajectory["frames"].append(frame)
            all_images.append((entry, trajectory, frame))
        trajectories.append(trajectory)
        print(f"{entry['sample_id']}: {entry['category']} | {trajectory['annotations']}", flush=True)

    manifest = {
        "schema_version": 1,
        "source": source,
        "selection_rule": selection["selection_rule"],
        "trajectory_count": len(trajectories),
        "image_count": len(all_images),
        "review_status": review.get("status", "pending"),
        "dishwashing_assessment": review.get("dishwashing_assessment", "pending visual review"),
        "trajectories": trajectories,
    }
    manifest_path = root / "manifest" / "quality-manifest.json"
    manifest_path.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    make_contact_sheets(root, all_images)
    print(json.dumps({
        "trajectory_count": len(trajectories),
        "image_count": len(all_images),
        "image_bytes": sum(frame["bytes"] for _, _, frame in all_images),
        "manifest": str(manifest_path),
        "review_status": manifest["review_status"],
    }, ensure_ascii=False, indent=2))


def make_contact_sheets(root: Path, images: list[tuple[dict, dict, dict]]) -> None:
    destination = root / "outputs" / "contact-sheets"
    destination.mkdir(parents=True, exist_ok=True)
    frame_width, frame_height, header_height = 320, 240, 54
    for sheet_index, start in enumerate(range(0, len(images), 18), 1):
        chunk = images[start:start + 18]
        sheet = Image.new("RGB", (960, (frame_height + header_height) * (len(chunk) // 3)), "white")
        draw = ImageDraw.Draw(sheet)
        for row_index in range(len(chunk) // 3):
            entry, trajectory, _ = chunk[row_index * 3]
            y = row_index * (frame_height + header_height)
            annotation = trajectory["annotations"][0] if trajectory["annotations"] else ""
            header = f"{entry['sample_id']} CSV {entry['csv_line']} {entry['category']} | {annotation}"
            draw.text((6, y + 5), header[:118], fill="black")
            draw.text((6, y + 23), f"review={trajectory['visual_review'].get('status', 'pending')} | first / mid / last", fill="black")
            for column, (_, _, frame) in enumerate(chunk[row_index * 3:(row_index + 1) * 3]):
                with Image.open(root / frame["path"]) as image:
                    tile = image.convert("RGB").resize((frame_width, frame_height), Image.Resampling.BILINEAR)
                x = column * frame_width
                sheet.paste(tile, (x, y + header_height))
                draw.text((x + 4, y + header_height + 4), f"{frame['frame_label']} im_{frame['frame_index']}", fill="white", stroke_width=2, stroke_fill="black")
        sheet.save(destination / f"sheet_{sheet_index:02d}.jpg", quality=92)


if __name__ == "__main__":
    main()
