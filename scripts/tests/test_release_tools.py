"""Standard-library regression tests for publication and packaging boundaries."""
import hashlib
import json
from pathlib import Path
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.dont_write_bytecode = True
SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import check_repository
import release


class RepositoryChecks(unittest.TestCase):
    def test_allowed_source_paths(self):
        for path in ("Cargo.lock", ".github/workflows/rust.yml", "crates/vivecraft-core/src/lib.rs", "scripts/smoke-pumpkin.toml", check_repository.WRAPPER):
            with self.subTest(path=path):
                self.assertIsNone(check_repository.path_issue(path))

    def test_rejects_generated_and_private_paths_even_if_tracked(self):
        for path in ("target/plugin.wasm", ".local/server/world/level.dat", "plugins/config.toml", "pumpkin.toml", ".env", "secrets/signing.key", "id_ed25519", "cache/server.jar", ".claude/settings.local.json"):
            with self.subTest(path=path):
                self.assertIsNotNone(check_repository.path_issue(path))

    def test_rejects_nonportable_and_escaping_paths(self):
        for path in ("../secret", "a/../secret", "/secret", "D:/secret", "a\\secret", "a//secret", "./secret"):
            with self.subTest(path=path):
                self.assertIsNotNone(check_repository.path_issue(path))

    def test_secret_values_are_not_reported(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            token = "ghp_" + "a" * 36
            (root / "notes.md").write_text(token, encoding="utf-8")
            problems = check_repository.audit(root, ["notes.md"])
            self.assertEqual(len(problems), 1)
            self.assertIn("notes.md:1", problems[0])
            self.assertNotIn(token, problems[0])

    def test_wrapper_is_not_a_blanket_jar_exception(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            wrapper = root / check_repository.WRAPPER
            wrapper.parent.mkdir(parents=True)
            wrapper.write_bytes(b"not the Gradle wrapper")
            self.assertIn("checksum changed", check_repository.audit(root, [check_repository.WRAPPER])[0])
            self.assertIsNotNone(check_repository.path_issue("gradle/other.jar"))


class ReleaseChecks(unittest.TestCase):
    def test_namespaced_tag_must_match_workspace(self):
        release.check_tag("pumpkin-v0.1.0", "0.1.0")
        release.check_tag("pumpkin-v0.2.0-beta.1", "0.2.0-beta.1")
        release.check_tag(None, "0.1.0")
        for tag in ("v0.1.0", "1.3.17-0", "pumpkin-v0.2.0", "pumpkin-v0.1.0/extra"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release.check_tag(tag, "0.1.0")

    def test_bad_version_rejected(self):
        for version in ("../release", "01.0.0", "0.1.0-01", "0.1.0-beta..1"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                release.check_tag(None, version)

    def test_component_header_and_market_size_limit(self):
        with tempfile.TemporaryDirectory() as temp:
            path = Path(temp) / "plugin.wasm"
            path.write_bytes(release.COMPONENT_HEADER)
            release.validate_artifact(path)
            path.write_bytes(b"\x00asm\x01\x00\x00\x00")
            with self.assertRaises(ValueError):
                release.validate_artifact(path)
            path.write_bytes(release.COMPONENT_HEADER)
            with path.open("ab") as stream:
                stream.truncate(release.MAX_WASM_BYTES + 1)
            with self.assertRaises(ValueError):
                release.validate_artifact(path)

    def test_vendor_config_has_only_portable_directory(self):
        original = '[source.crates-io]\nreplace-with = "vendored-sources"\n[source.vendored-sources]\ndirectory = "C:/private/temp/vendor"\n'
        result = release.vendor_config(original)
        self.assertIn('directory = "vendor"', result)
        self.assertNotIn("private", result)
        with self.assertRaises(ValueError):
            release.vendor_config('[source.crates-io]\nreplace-with = "x"')

    def test_dependency_notices_are_verbatim_and_missing_license_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            vendor = root / "vendor"
            crate = vendor / "example-1.0.0"
            crate.mkdir(parents=True)
            original = b"Copyright example contributors\nMIT license text\n"
            (crate / "LICENSE-MIT").write_bytes(original)
            notices = root / "notices"
            notices.mkdir()
            package = {"name": "example", "version": "1.0.0", "license": "MIT", "source": "registry+example"}
            release.collect_notices([package], vendor, notices)
            copied = notices / "dependencies/example-1.0.0/LICENSE-MIT"
            self.assertEqual(copied.read_bytes(), original)
            self.assertIn("vendor/example-1.0.0/", (notices / "DEPENDENCIES.md").read_text())
            package["license"] = None
            with self.assertRaises(ValueError):
                release.collect_notices([package], vendor, notices)

    def test_empty_license_directory_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "vendor/example-1.0.0").mkdir(parents=True)
            (root / "notices").mkdir()
            package = {"name": "example", "version": "1.0.0", "license": "MIT", "source": "registry+example"}
            with self.assertRaises(ValueError):
                release.collect_notices([package], root / "vendor", root / "notices")

    def test_runtime_notices_are_required(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(release, "run", return_value=temp):
            with self.assertRaises(ValueError):
                release.collect_runtime_notices(Path(temp))

    def test_archives_are_portable_and_deterministic(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            (source / "vendor/example").mkdir(parents=True)
            (source / "vendor/example/LICENSE").write_text("license", encoding="utf-8")
            (source / "gradlew").write_text("#!/bin/sh\n", encoding="utf-8")
            release.write_tar(source, root / "one.tar.gz", 1_700_000_000)
            release.write_tar(source, root / "two.tar.gz", 1_700_000_000)
            self.assertEqual((root / "one.tar.gz").read_bytes(), (root / "two.tar.gz").read_bytes())
            with tarfile.open(root / "one.tar.gz") as archive:
                self.assertEqual(set(archive.getnames()), {"source/gradlew", "source/vendor/example/LICENSE"})
                self.assertEqual(archive.getmember("source/gradlew").mode, 0o755)
            release.write_zip(source, root / "notices.zip")
            with zipfile.ZipFile(root / "notices.zip") as archive:
                self.assertEqual(archive.read("vendor/example/LICENSE"), b"license")

    def test_archive_refuses_a_symlink_flag_on_every_platform(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (source / "file").write_text("content", encoding="utf-8")
            with patch.object(Path, "is_symlink", return_value=True):
                with self.assertRaises(ValueError):
                    release.write_tar(source, root / "invalid.tar.gz", 1_700_000_000)

    def test_archive_refuses_symlinks(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = root / "source"
            source.mkdir()
            (root / "private").write_text("outside", encoding="utf-8")
            try:
                (source / "link").symlink_to(root / "private")
            except OSError:
                self.skipTest("Creating symbolic links is not permitted on this Windows installation")
            with self.assertRaises(ValueError):
                release.write_tar(source, root / "invalid.tar.gz", 1_700_000_000)

    def test_checksums_cover_each_output_not_the_manifest_itself(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            (root / "plugin.wasm").write_bytes(release.COMPONENT_HEADER)
            (root / "release.json").write_text(json.dumps({"version": "0.1.0"}), encoding="utf-8")
            release.write_checksums(root)
            lines = (root / "SHA256SUMS").read_text().splitlines()
            self.assertEqual(len(lines), 2)
            for line in lines:
                digest, name = line.split("  ")
                self.assertEqual(digest, hashlib.sha256((root / name).read_bytes()).hexdigest())
                self.assertNotEqual(name, "SHA256SUMS")


if __name__ == "__main__":
    unittest.main()
