from __future__ import annotations

import hashlib
import json
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parent.parent
RUNTIME = ROOT / "src-tauri" / "binaries" / "semantic-runtime"
CACHE = ROOT / ".cache" / "semantic-runtime"

PACKAGES = {
    "Microsoft.ML.OnnxRuntime.DirectML": {
        "version": "1.24.4",
        "files": {
            "runtimes/win-x64/native/onnxruntime.dll": "onnxruntime.dll",
            "runtimes/win-x64/native/onnxruntime_providers_shared.dll": "onnxruntime_providers_shared.dll",
            "LICENSE": "licenses/OnnxRuntime-LICENSE.txt",
            "ThirdPartyNotices.txt": "licenses/OnnxRuntime-ThirdPartyNotices.txt",
        },
    },
    "Microsoft.AI.DirectML": {
        "version": "1.15.4",
        "files": {
            "bin/x64-win/DirectML.dll": "DirectML.dll",
            "LICENSE.txt": "licenses/DirectML-LICENSE.txt",
            "LICENSE-CODE.txt": "licenses/DirectML-LICENSE-CODE.txt",
            "ThirdPartyNotices.txt": "licenses/DirectML-ThirdPartyNotices.txt",
        },
    },
}


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def download(url: str, destination: Path) -> None:
    if destination.is_file():
        return
    destination.parent.mkdir(parents=True, exist_ok=True)
    partial = destination.with_suffix(destination.suffix + ".part")
    with urllib.request.urlopen(url) as response, partial.open("wb") as output:
        while chunk := response.read(8 * 1024 * 1024):
            output.write(chunk)
    partial.replace(destination)


def main() -> int:
    report = {"packages": [], "runtime_files": []}
    for name, spec in PACKAGES.items():
        version = spec["version"]
        url = f"https://www.nuget.org/api/v2/package/{name}/{version}"
        package = CACHE / "nuget" / f"{name}.{version}.nupkg"
        print(f"Fetching {name} {version}", flush=True)
        download(url, package)
        report["packages"].append({"name": name, "version": version, "url": url, "sha256": sha256(package)})
        with zipfile.ZipFile(package) as archive:
            for member, relative_destination in spec["files"].items():
                destination = RUNTIME / relative_destination
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(archive.read(member))
                if destination.suffix.lower() == ".dll":
                    record = {
                        "path": str(destination.relative_to(RUNTIME)).replace("\\", "/"),
                        "bytes": destination.stat().st_size,
                        "sha256": sha256(destination),
                    }
                    report["runtime_files"].append(record)
                    print(json.dumps(record), flush=True)
    RUNTIME.mkdir(parents=True, exist_ok=True)
    (RUNTIME / "runtime-manifest.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(f"语义运行时就绪：{RUNTIME}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
