#!/usr/bin/env python3
"""Package pinned release bytes only. Never builds, installs, or publishes products."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
DIST = ROOT / "distribution"
BUNDLE = "b2ige-verify-0.1.0.mcpb"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def verified_asset(cache, base, name, expected):
    path = cache / name
    if path.exists():
        data = path.read_bytes()
    else:
        with urllib.request.urlopen(f"{base}/{name}", timeout=120) as response:
            data = response.read()
    if sha256(data) != expected:
        raise ValueError(f"SHA-256 mismatch: {name}; refusing to package")
    if not path.exists():
        path.write_bytes(data)
    return path


def member_bytes(archive, name):
    # Read only exact regular members; never extract paths/links supplied by tar.
    matches = [m for m in archive.getmembers() if m.name == name]
    if len(matches) != 1 or not matches[0].isfile():
        raise ValueError(f"expected one regular archive member: {name}")
    return archive.extractfile(matches[0]).read()


def normalize_zip(source, destination):
    # mcpb pack writes wall-clock ZIP timestamps. Preserve payload bytes/modes,
    # but canonicalize ZIP order and metadata for reproducible distribution.
    with zipfile.ZipFile(source) as packed, zipfile.ZipFile(destination, "w") as output:
        for name in sorted(packed.namelist()):
            original = packed.getinfo(name)
            entry = zipfile.ZipInfo(name, (2026, 9, 16, 0, 0, 0))
            entry.create_system = 3
            entry.external_attr = original.external_attr
            entry.compress_type = zipfile.ZIP_DEFLATED
            output.writestr(entry, packed.read(name), compresslevel=9)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mcpb", default="mcpb", help="path to @anthropic-ai/mcpb 2.1.2")
    parser.add_argument("--cache", type=Path, default=ROOT / "release/artifacts/agent-distribution/downloads")
    parser.add_argument("--output", type=Path, default=ROOT / "release/artifacts/agent-distribution")
    args = parser.parse_args()
    lock = json.loads((DIST / "release-inputs.json").read_text())
    version = subprocess.check_output([args.mcpb, "--version"], text=True).strip()
    if version != lock["mcpb_cli_version"]:
        raise ValueError(f"expected mcpb {lock['mcpb_cli_version']}, got {version}")
    cache, output = args.cache.resolve(), args.output.resolve()
    cache.mkdir(parents=True, exist_ok=True)
    output.mkdir(parents=True, exist_ok=True)
    final = output / BUNDLE
    if final.exists():
        raise ValueError(f"output already exists: {final}; use a fresh --output directory")
    sums_path = verified_asset(cache, lock["download_base"], "SHA256SUMS", lock["checksums_sha256"])
    sums = {name: digest for digest, name in (line.split() for line in sums_path.read_text().splitlines())}
    with tempfile.TemporaryDirectory(prefix="b2ige-mcpb-") as temporary:
        work = Path(temporary)
        stage = work / "bundle"
        stage.mkdir()
        for name in ("manifest.json", "launch.sh", "README.md"):
            shutil.copyfile(DIST / "mcpb" / name, stage / name)
            (stage / name).chmod(0o755 if name == "launch.sh" else 0o644)
        shutil.copyfile(DIST / "release-inputs.json", stage / "release-inputs.json")
        for item in lock["inputs"]:
            for key in ("archive", "manifest"):
                if sums[item[key]] != item[f"{key}_sha256"]:
                    raise ValueError(f"release checksum disagrees with pinned {key}")
            manifest_path = verified_asset(cache, lock["download_base"], item["manifest"], item["manifest_sha256"])
            manifest = json.loads(manifest_path.read_text())
            if (manifest["git_commit"] != lock["source_commit"]
                    or manifest["built_target"] != item["target"]
                    or manifest["binary_sha256"]["b2ige-mcp"] != item["binary_sha256"]):
                raise ValueError("release manifest identity mismatch")
            archive_path = verified_asset(cache, lock["download_base"], item["archive"], item["archive_sha256"])
            prefix = item["archive"].removesuffix(".tar.gz")
            with tarfile.open(archive_path, "r:gz") as archive:
                data = member_bytes(archive, f"{prefix}/bin/b2ige-mcp")
                if sha256(data) != item["binary_sha256"]:
                    raise ValueError(f"binary digest mismatch: {item['target']}")
                binary = stage / "bin" / item["target"] / "b2ige-mcp"
                binary.parent.mkdir(parents=True)
                binary.write_bytes(data)
                binary.chmod(0o755)
                for name in ("LICENSE", "SECURITY.md", "TRADEMARKS.md", "THIRD-PARTY-NOTICES.txt"):
                    data = member_bytes(archive, f"{prefix}/{name}")
                    path = stage / name
                    if path.exists() and path.read_bytes() != data:
                        raise ValueError(f"release notices differ between platforms: {name}")
                    path.write_bytes(data)
                    path.chmod(0o644)
        (stage / "release-inputs.json").chmod(0o644)
        subprocess.run([args.mcpb, "validate", str(stage / "manifest.json")], check=True)
        packed = work / "packed.mcpb"
        subprocess.run([args.mcpb, "pack", str(stage), str(packed)], check=True)
        normalized = work / BUNDLE
        normalize_zip(packed, normalized)
        with zipfile.ZipFile(normalized) as bundle:
            if bundle.testzip() is not None:
                raise ValueError("ZIP integrity failure")
            for item in lock["inputs"]:
                name = f"bin/{item['target']}/b2ige-mcp"
                if sha256(bundle.read(name)) != item["binary_sha256"]:
                    raise ValueError("packaging changed product bytes")
                if not (bundle.getinfo(name).external_attr >> 16) & 0o111:
                    raise ValueError("packaging lost executable mode")
            extracted_manifest = work / "manifest.json"
            extracted_manifest.write_bytes(bundle.read("manifest.json"))
        subprocess.run([args.mcpb, "validate", str(extracted_manifest)], check=True)
        digest = sha256(normalized.read_bytes())
        # Never replace an already reviewed distribution artifact.
        with final.open("xb") as destination:
            destination.write(normalized.read_bytes())
        (output / f"{BUNDLE}.sha256").write_text(f"{digest}  {BUNDLE}\n")
        print(f"SHA-256 {digest}\nMCPB {final}")


if __name__ == "__main__":
    main()
