"""Package a WASI component, license notices, and rebuildable locked source; never publish."""
from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import zipfile

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent))
from check_repository import ROOT, audit, candidate_paths  # noqa: E402

MAX_WASM_BYTES = 5 * 1024 * 1024
COMPONENT_HEADER = b"\x00asm\x0d\x00\x01\x00"


def run(*args: str, cwd: Path = ROOT) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True, encoding="utf-8").strip()


def check_tag(tag: str | None, version: str) -> None:
    identifier = r"(?:0|[1-9][0-9]*)"
    prerelease = r"(?:0|[1-9][0-9]*|[0-9A-Za-z-]*[A-Za-z-][0-9A-Za-z-]*)"
    if not re.fullmatch(rf"{identifier}\.{identifier}\.{identifier}(?:-{prerelease}(?:\.{prerelease})*)?", version):
        raise ValueError("Unsupported workspace version; use a SemVer release/prerelease")
    if tag is not None and tag != f"pumpkin-v{version}":
        raise ValueError(f"Tag must be pumpkin-v{version}, not {tag!r}")


def validate_artifact(path: Path) -> None:
    size = path.stat().st_size
    if not 8 <= size <= MAX_WASM_BYTES:
        raise ValueError(f"WASM size {size} is outside the Market upload range (8..{MAX_WASM_BYTES})")
    with path.open("rb") as stream:
        if stream.read(8) != COMPONENT_HEADER:
            raise ValueError("Expected the pinned wasm32-wasip2 component header, not a core module")


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def tree_files(root: Path) -> list[Path]:
    paths = sorted(root.rglob("*"))
    if any(p.is_symlink() or not p.resolve().is_relative_to(root.resolve()) for p in paths):
        raise ValueError("Source/notice staging tree contains a symbolic link or escaping path")
    return [p for p in paths if p.is_file()]


def write_tar(source: Path, destination: Path, epoch: int) -> None:
    with destination.open("xb") as output, gzip.GzipFile(filename="", mode="wb", fileobj=output, mtime=epoch) as compressed:
        with tarfile.open(fileobj=compressed, mode="w", format=tarfile.PAX_FORMAT) as archive:
            for path in tree_files(source):
                name = path.relative_to(source).as_posix()
                info = tarfile.TarInfo(f"{source.name}/{name}")
                info.size = path.stat().st_size
                info.mtime = epoch
                info.mode = 0o755 if name == "gradlew" or name.endswith(".sh") else 0o644
                with path.open("rb") as stream:
                    archive.addfile(info, stream)


def write_zip(source: Path, destination: Path) -> None:
    with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in tree_files(source):
            info = zipfile.ZipInfo(path.relative_to(source).as_posix(), date_time=(1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_DEFLATED
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())


def license_files(package: Path) -> list[Path]:
    prefixes = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT", "UNLICENSE")
    return [p for p in tree_files(package) if p.name.upper().startswith(prefixes)]


def vendor_config(config: str) -> str:
    parsed = tomllib.loads(config)
    sources = parsed.get("source", {})
    if not any("directory" in value for value in sources.values()):
        raise ValueError("cargo vendor did not return a directory-source configuration")
    return re.sub(r"(?m)^directory\s*=.*$", 'directory = "vendor"', config) + "\n"


def collect_notices(packages: list[dict], vendor: Path, notices: Path) -> None:
    lines = [
        "# Locked dependency inventory", "",
        "These are declared package licenses, not a claim that all packages are linked.",
        "Original notices and source are preserved; review all applicable terms.", "",
        "| Package | Version | Declared license | Source archive path |",
        "| --- | --- | --- | --- |",
    ]
    for package in sorted(packages, key=lambda p: (p["name"], p["version"])):
        if package.get("source") is None:
            continue
        name = f'{package["name"]}-{package["version"]}'
        if not re.fullmatch(r"[A-Za-z0-9_.+\-]+", name):
            raise ValueError("Unexpected dependency path")
        folder = vendor / name
        licenses = license_files(folder)
        if not package.get("license") or not licenses:
            raise ValueError(f"Review missing license expression/texts for {name}")
        lines.append(f'| {package["name"]} | {package["version"]} | {package["license"]} | `vendor/{name}/` |')
        for source in licenses:
            destination = notices / "dependencies" / name / source.relative_to(folder)
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)
    (notices / "DEPENDENCIES.md").write_text("\n".join(lines) + "\n", encoding="utf-8")


def collect_runtime_notices(notices: Path) -> None:
    sysroot = Path(run("rustc", "--print", "sysroot"))
    documentation = sysroot / "share/doc/rust"
    report = documentation / "COPYRIGHT-library.html"
    licenses = documentation / "licenses"
    if not report.is_file() or not licenses.is_dir():
        raise ValueError("Pinned Rust runtime copyright/license material is missing")
    destination = notices / "rust-runtime"
    destination.mkdir()
    shutil.copyfile(report, destination / report.name)
    shutil.copytree(licenses, destination / "licenses")


def write_checksums(output: Path) -> None:
    files = sorted(p for p in output.iterdir() if p.is_file() and p.name != "SHA256SUMS")
    (output / "SHA256SUMS").write_text(
        "".join(f"{sha256(p)}  {p.name}\n" for p in files), encoding="utf-8",
    )


def package_release(args: argparse.Namespace, version: str) -> None:
    artifact = Path(args.artifact).resolve()
    validate_artifact(artifact)
    paths = candidate_paths()
    problems = audit(ROOT, paths)
    if problems:
        raise ValueError("Publication inventory failed:\n" + "\n".join(problems))
    dirty = bool(run("git", "status", "--porcelain", "--untracked-files=normal"))
    if dirty and not args.allow_dirty:
        raise ValueError("Commit the intended source before packaging, or use --allow-dirty for LOCAL verification only")
    if args.allow_dirty and os.environ.get("GITHUB_ACTIONS") == "true":
        raise ValueError("--allow-dirty is forbidden in GitHub Actions")
    commit = run("git", "rev-parse", "HEAD")
    epoch = int(run("git", "show", "-s", "--format=%ct", "HEAD"))
    metadata = json.loads(run("cargo", "metadata", "--locked", "--offline", "--format-version", "1"))
    output = Path(args.output).resolve()
    if output.exists():
        raise ValueError("Output directory already exists; choose a new directory to avoid overwrites")
    if output == ROOT or not output.is_relative_to(ROOT / ".local") and not output.is_relative_to(ROOT / "dist"):
        raise ValueError("Use a new output directory under ignored .local/ or dist/")
    output.mkdir(parents=True)
    base = f"vivecraft-pumpkin-{version}"
    with tempfile.TemporaryDirectory(prefix="package-", dir=output.parent) as temporary:
        source = Path(temporary) / f"{base}-source"
        notices = Path(temporary) / "notices"
        source.mkdir()
        notices.mkdir()
        for name in paths:
            destination = source / name
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(ROOT / name, destination)
        vendor = source / "vendor"
        config = run("cargo", "vendor", "--locked", "--offline", "--versioned-dirs", str(vendor))
        (source / ".cargo").mkdir(exist_ok=True)
        (source / ".cargo/config.toml").write_text(vendor_config(config), encoding="utf-8")
        for name in ("LICENSE.md", "THIRD_PARTY_NOTICES.md", "README.md", "config.example.toml"):
            shutil.copyfile(ROOT / name, notices / name)
        shutil.copytree(ROOT / "LICENSES", notices / "LICENSES")
        collect_notices(metadata["packages"], vendor, notices)
        collect_runtime_notices(notices)
        shutil.copytree(notices, source / "release-notices")
        manifest = {
            "version": version,
            "source_commit": commit,
            "working_tree_modified": dirty,
            "local_verification_only": args.allow_dirty,
            "target": "wasm32-wasip2",
            "rustc": run("rustc", "--version"),
            "cargo": run("cargo", "--version"),
            "sdk": next(p["version"] for p in metadata["packages"] if p["name"] == "pumpkin-plugin-api"),
            "binary_sha256": sha256(artifact),
            "binary_bytes": artifact.stat().st_size,
            "pumpkin_market_signature": "not embedded; this is the original upload input",
        }
        manifest_text = json.dumps(manifest, indent=2) + "\n"
        (source / "RELEASE.json").write_text(manifest_text, encoding="utf-8")
        (output / "release.json").write_text(manifest_text, encoding="utf-8")
        shutil.copyfile(artifact, output / f"{base}.wasm")
        write_tar(source, output / f"{base}-source.tar.gz", epoch)
        write_zip(notices, output / f"{base}-notices.zip")
    write_checksums(output)
    print(f"Packaged {version} in {output}")
    if args.allow_dirty:
        print("LOCAL verification package: uncommitted source; do not publish it.")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)
    check = subparsers.add_parser("check", help="Reject a tag/workspace version mismatch")
    check.add_argument("--tag", required=True)
    package = subparsers.add_parser("package", help="Create binary, notices, corresponding source and checksums")
    package.add_argument("--tag")
    package.add_argument("--artifact", default="target/wasm32-wasip2/release/vivecraft_pumpkin.wasm")
    package.add_argument("--output", required=True, help="A NEW directory under .local/ or dist/")
    package.add_argument("--allow-dirty", action="store_true", help="LOCAL pre-commit verification only; never publish")
    args = parser.parse_args()
    try:
        version = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
        check_tag(args.tag, version)
        if args.command == "check":
            print(f"Release tag matches workspace version {version}.")
        else:
            package_release(args, version)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"Release preparation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
