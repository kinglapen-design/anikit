from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import shutil
import urllib.request
import zipfile

PACKAGES = {
    "Microsoft.ML.OnnxRuntime.DirectML": {
        "version": "1.24.4",
        "sha256": "57e9f11b73437bef7a309496135d4c1f96b1a8e9ddba60013fa27bfc1d788681",
        "files": {
            "runtimes/win-x64/native/onnxruntime.dll": "onnxruntime.dll",
            "runtimes/win-x64/native/onnxruntime_providers_shared.dll": "onnxruntime_providers_shared.dll",
            "LICENSE": "licenses/OnnxRuntime-LICENSE.txt",
            "ThirdPartyNotices.txt": "licenses/OnnxRuntime-ThirdPartyNotices.txt",
        },
    },
    "Microsoft.AI.DirectML": {
        "version": "1.15.4",
        "sha256": "4e7cb7ddce8cf837a7a75dc029209b520ca0101470fcdf275c1f49736a3615b9",
        "files": {
            "bin/x64-win/DirectML.dll": "DirectML.dll",
            "LICENSE.txt": "licenses/DirectML-LICENSE.txt",
            "LICENSE-CODE.txt": "licenses/DirectML-LICENSE-CODE.txt",
            "ThirdPartyNotices.txt": "licenses/DirectML-ThirdPartyNotices.txt",
        },
    },
}
EXPECTED_DLLS = {
    "onnxruntime.dll": "e7eedec6a6f26dc39dc948276a75ef6d2bee3fff944d874ceed0bbd3b97bff40",
    "onnxruntime_providers_shared.dll": "265c8daf29637cb259cac8be9f08f2cd45f3883f0f0e4949cbfddd5b4cbec3b6",
    "DirectML.dll": "9c9e6d822561c6c41b90e6994b3e8857cf1d66dbfb1e0c4c799c7c89b4e92da1",
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


def main() -> None:
    root = Path(os.environ["SPIKE_ROOT"]).resolve()
    runtime = root / "runtime"
    baseline = root.parent / "semantic-clip" / "runtime"
    report = {"packages": [], "runtime_files": [], "sources": {}}
    for name, spec in PACKAGES.items():
        version = spec["version"]
        url = f"https://www.nuget.org/api/v2/package/{name}/{version}"
        package = root / ".cache" / "nuget" / f"{name}.{version}.nupkg"
        archive = None
        for member, relative_destination in spec["files"].items():
            destination = runtime / relative_destination
            source = baseline / relative_destination
            if source.is_file():
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(source, destination)
                report["sources"][relative_destination] = "copied from read-only semantic-clip runtime"
            else:
                if archive is None:
                    download(url, package)
                    if sha256(package) != spec["sha256"]:
                        raise RuntimeError(f"NuGet hash mismatch: {package}")
                    archive = zipfile.ZipFile(package)
                destination.parent.mkdir(parents=True, exist_ok=True)
                destination.write_bytes(archive.read(member))
                report["sources"][relative_destination] = url
        if archive is not None:
            archive.close()
        report["packages"].append({
            "name": name,
            "version": version,
            "url": url,
            "sha256": spec["sha256"],
        })

    for filename, expected_hash in EXPECTED_DLLS.items():
        path = runtime / filename
        actual_hash = sha256(path)
        if actual_hash != expected_hash:
            raise RuntimeError(f"Runtime DLL hash mismatch for {filename}: {actual_hash}")
        report["runtime_files"].append({
            "path": str(path.relative_to(root)).replace("\\", "/"),
            "bytes": path.stat().st_size,
            "sha256": actual_hash,
        })
    output = root / "outputs"
    output.mkdir(parents=True, exist_ok=True)
    (output / "runtime-manifest.json").write_text(
        json.dumps(report, indent=2) + "\n", encoding="utf-8"
    )
    print(json.dumps(report["runtime_files"], indent=2))


if __name__ == "__main__":
    main()
