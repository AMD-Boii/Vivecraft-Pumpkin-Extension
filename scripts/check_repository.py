"""Check Git-visible publication candidates without reading ignored runtime/private data."""
from __future__ import annotations

import hashlib
from pathlib import Path, PurePosixPath
import re
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
WRAPPER = "gradle/wrapper/gradle-wrapper.jar"
WRAPPER_SHA256 = "76805e32c009c0cf0dd5d206bddc9fb22ea42e84db904b764f3047de095493f3"
LOCAL_PARTS = {
    ".git", ".local", ".claude", ".idea", ".vscode", ".gradle", "__pycache__",
    "target", "build", "dist", "vendor", "node_modules", "logs", "crash-reports",
}
BINARY_SUFFIXES = {
    ".wasm", ".jar", ".class", ".exe", ".dll", ".so", ".dylib", ".pdb",
    ".zip", ".gz", ".7z", ".rar", ".db", ".sqlite", ".log", ".pyc",
}
PRIVATE_SUFFIXES = {".key", ".pem", ".p12", ".pfx"}
RUNTIME_ROOTS = {
    "plugins", "world", "worlds", "pumpkin.toml", "eula.txt", "ops.json",
    "usercache.json", "whitelist.json", "banned-players.json", "banned-ips.json",
}
SECRET_PATTERNS = [
    re.compile(r"-----BEGIN (?:RSA |EC |OPENSSH |DSA )?PRIVATE KEY-----"),
    re.compile(r"\bgh[pousr]_[A-Za-z0-9]{30,}\b"),
    re.compile(r"\bgithub_pat_[A-Za-z0-9_]{30,}\b"),
    re.compile(r"\bAKIA[0-9A-Z]{16}\b"),
]


def candidate_paths(root: Path = ROOT) -> list[str]:
    output = subprocess.check_output(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
        cwd=root,
    )
    return sorted({p.decode("utf-8") for p in output.split(b"\0") if p})


def path_issue(name: str) -> str | None:
    path = PurePosixPath(name)
    if not name or path.is_absolute() or "\\" in name or ":" in name:
        return "not a portable relative path"
    if any(part in {"", ".", ".."} for part in name.split("/")):
        return "unsafe path component"
    if any(part.lower() in LOCAL_PARTS for part in path.parts):
        return "local/generated directory"
    if path.parts[0].lower() in RUNTIME_ROOTS:
        return "server/runtime data"
    if path.name.lower() in {"id_rsa", "id_ed25519", "credentials", "credentials.toml"}:
        return "private credential filename"
    if path.name.lower().startswith(".env") and path.name.lower() != ".env.example":
        return "local environment file"
    if path.suffix.lower() in PRIVATE_SUFFIXES:
        return "private signing/credential material"
    if path.suffix.lower() in BINARY_SUFFIXES and name != WRAPPER:
        return "compiled binary/archive/runtime output"
    return None


def audit(root: Path, paths: list[str]) -> list[str]:
    problems = []
    for name in paths:
        issue = path_issue(name)
        if issue:
            problems.append(f"{name}: {issue}")
            continue
        path = root / name
        if path.is_symlink() or not path.is_file() or not path.resolve().is_relative_to(root.resolve()):
            problems.append(f"{name}: missing, linked, or outside repository")
            continue
        data = path.read_bytes()
        if name == WRAPPER:
            if hashlib.sha256(data).hexdigest() != WRAPPER_SHA256:
                problems.append(f"{name}: wrapper checksum changed; review upstream provenance")
            continue
        if len(data) > 1024 * 1024:
            problems.append(f"{name}: exceeds 1 MiB; review before publication")
        if path.suffix.lower() in {".png", ".jpg", ".jpeg", ".gif", ".webp", ".ico"}:
            continue
        try:
            text = data.decode("utf-8-sig")
        except UnicodeDecodeError:
            problems.append(f"{name}: unexpected binary/non-UTF-8 file")
            continue
        for number, line in enumerate(text.splitlines(), 1):
            if any(pattern.search(line) for pattern in SECRET_PATTERNS):
                problems.append(f"{name}:{number}: possible secret (value omitted)")
    return problems


def main() -> int:
    paths = candidate_paths()
    problems = audit(ROOT, paths)
    for required in ("LICENSE.md", "LICENSES/GPL-3.0-only.txt", "LICENSES/Apache-2.0.txt", "THIRD_PARTY_NOTICES.md"):
        if required not in paths:
            problems.append(f"{required}: required license/provenance file missing")
    if problems:
        print("Publication inventory failed:\n" + "\n".join(problems), file=sys.stderr)
        return 1
    print(f"Publication inventory passed: {len(paths)} Git-visible files; wrapper provenance verified.")
    print("Ignored files and Git history are not scanned; this is not a complete secret/license audit.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
