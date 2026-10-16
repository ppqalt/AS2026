"""Julkaisupaketoinnin regressiot, ei verkkoa eikä oikeita avaimia."""

import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import package_release as package
import sign_release as signing


class PackagingFixture:
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="as2026-release-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "Cargo.toml").write_text('[package]\nname="as2026"\nversion="0.5.0"\nlicense="BSD-2-Clause"\n', encoding="utf-8")
        (self.root / "LICENSE").write_text(package.BSD_2_CLAUSE_TEXT, encoding="utf-8")
        (self.root / "THIRD_PARTY.md").write_text("Synthetic test notices", encoding="utf-8")
        self.crate = self.root / "crate"
        self.crate.mkdir()
        (self.crate / "Cargo.toml").write_text("", encoding="utf-8")
        (self.crate / "LICENSE").write_text("Synthetic dependency license", encoding="utf-8")
        (self.crate / "NOTICE").write_text("Synthetic author notice", encoding="utf-8")
        (self.crate / "lib.rs").write_text("// SPDX-FileCopyrightText: Synthetic test author\n", encoding="utf-8")
        self.metadata = {
            "packages": [
                {"id": "as2026", "name": "as2026", "version": "0.5.0", "license": "BSD-2-Clause"},
                {"id": "test", "name": "test-dependency", "version": "1.0.0", "manifest_path": str(self.crate / "Cargo.toml"), "license": "MIT", "authors": ["Synthetic author"]},
                {"id": "unused", "name": "unused", "version": "1.0.0"},
            ],
            "resolve": {"root": "as2026", "nodes": [
                {"id": "as2026", "deps": [{"pkg": "test"}]}, {"id": "test", "deps": []},
            ]},
        }
        self.binaries = self.root / "binaries"
        self.binaries.mkdir()
        for name in package.BINARIES:
            for extension in ("", ".exe"):
                (self.binaries / (name + extension)).write_bytes(b"synthetic binary " + name.encode())


class PackagingTests(PackagingFixture, unittest.TestCase):
    def test_project_rejects_old_or_other_cargo_license(self):
        package.validate_project_license(self.root)
        cargo = self.root / "Cargo.toml"
        original = cargo.read_text(encoding="utf-8")
        for invalid in ("0BSD", "MIT", "BSD-2-Clause OR MIT"):
            cargo.write_text(original.replace('license="BSD-2-Clause"', f'license="{invalid}"'), encoding="utf-8")
            with self.subTest(license=invalid), self.assertRaisesRegex(ValueError, "Cargo.toml"):
                package.validate_project_license(self.root)

    def test_project_rejects_old_or_incomplete_license_text(self):
        license_path = self.root / "LICENSE"
        for invalid in (
            "Zero-Clause BSD\nCopyright (C) 2026 AS2026 contributors\nPermission to use, copy, modify, and/or distribute this software for any purpose with or without fee is hereby granted.\n",
            package.BSD_2_CLAUSE_TEXT.replace("2. Redistributions in binary form", "Redistributions in binary form"),
            package.BSD_2_CLAUSE_TEXT.replace("EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.", ""),
        ):
            license_path.write_text(invalid, encoding="utf-8")
            output = self.root / "bad-license"
            with self.subTest(text=invalid[:30]), self.assertRaisesRegex(ValueError, "täydellistä BSD-2-Clause"):
                package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
            self.assertFalse(output.exists())

    def test_project_accepts_same_full_license_with_windows_line_endings(self):
        (self.root / "LICENSE").write_bytes(package.BSD_2_CLAUSE_TEXT.replace("\n", "\r\n").encode("utf-8"))
        package.validate_project_license(self.root)

    def test_old_cargo_metadata_cannot_create_new_license_payload(self):
        self.metadata["packages"][0]["license"] = "0BSD"
        output = self.root / "old-metadata"
        with self.assertRaisesRegex(ValueError, "Cargo-metatietojen"):
            package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        self.assertFalse(output.exists())

    def test_third_party_0bsd_license_remains_unchanged(self):
        self.metadata["packages"][1]["license"] = "0BSD"
        original = b"Zero-Clause BSD\nSynthetic third-party copyright and permission notice\n"
        (self.crate / "LICENSE").write_bytes(original)
        output = self.root / "third-party"
        package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        self.assertEqual((output / "payload" / "dependency-licenses" / "test-dependency-1.0.0" / "LICENSE").read_bytes(), original)

    def test_stable_tag_must_match_cargo_version(self):
        self.assertEqual(package.verify_release("v0.5.0", self.root, main_ref=None), "0.5.0")
        for tag in ("v0.4.0", "0.5.0", "v0.5.0-beta.1"):
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                package.verify_release(tag, self.root, main_ref=None)
        for version in ("0.5", "0.5.0-rc.1", "0.5.0+local", "00.5.0", "0.5.0\n"):
            with self.subTest(version=version), self.assertRaises(ValueError):
                package.stable_version(version)

    @unittest.skipUnless(shutil.which("git"), "Historiatarkistus vaatii Gitin.")
    def test_tag_must_point_to_head_and_belong_to_main(self):
        def git(*arguments):
            return subprocess.run(["git", *arguments], cwd=self.root, check=True, capture_output=True)
        git("init", "--initial-branch=main")
        git("config", "user.name", "Synthetic release test")
        git("config", "user.email", "release-test@example.invalid")
        git("add", "Cargo.toml")
        git("commit", "-m", "Synthetic main")
        git("tag", "v0.5.0")
        self.assertEqual(package.verify_release("v0.5.0", self.root, main_ref="main"), "0.5.0")
        git("checkout", "-b", "dev")
        git("commit", "--allow-empty", "-m", "Synthetic development")
        with self.assertRaisesRegex(ValueError, "HEAD ei vastaa"):
            package.verify_release("v0.5.0", self.root, main_ref="main")
        (self.root / "Cargo.toml").write_text('[package]\nname="as2026"\nversion="0.5.1"\nlicense="BSD-2-Clause"\n', encoding="utf-8")
        git("add", "Cargo.toml")
        git("commit", "-m", "Synthetic development version")
        git("tag", "v0.5.1")
        with self.assertRaises(subprocess.CalledProcessError):
            package.verify_release("v0.5.1", self.root, main_ref="main")

    def test_prepare_includes_three_binaries_and_notices(self):
        target = "x86_64-unknown-linux-gnu"
        output = self.root / "payload-linux"
        result = package.prepare(target, output, self.binaries, self.metadata, self.root)
        self.assertEqual(result["license"], "BSD-2-Clause")
        self.assertEqual((output / "payload" / "LICENSE").read_bytes(), (self.root / "LICENSE").read_bytes())
        names = {entry["name"] for entry in result["files"]}
        self.assertTrue(set(package.BINARIES).issubset(names))
        self.assertIn("dependency-licenses/test-dependency-1.0.0/NOTICE", names)
        self.assertIn("dependency-licenses/test-dependency-1.0.0/LICENSE", names)
        self.assertIn("dependency-licenses/test-dependency-1.0.0/COPYRIGHT-NOTICES.txt", names)
        for entry in result["files"]:
            data = (output / "payload" / entry["name"]).read_bytes()
            self.assertEqual(entry["size"], len(data))
            self.assertEqual(entry["sha256"], hashlib.sha256(data).hexdigest())

    def test_missing_binary_rejected_before_any_output(self):
        (self.binaries / "as2026-launcher").unlink()
        output = self.root / "missing"
        with self.assertRaisesRegex(ValueError, "as2026-launcher"):
            package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        self.assertFalse(output.exists())

    def test_missing_license_aborts_and_removes_partial_output(self):
        (self.crate / "LICENSE").unlink()
        (self.crate / "NOTICE").unlink()
        output = self.root / "missing-license"
        with self.assertRaisesRegex(ValueError, "lisenssiteksti puuttuu"):
            package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        self.assertFalse(output.exists())

    def test_pinned_license_override_is_recorded_with_source(self):
        (self.crate / "LICENSE").unlink()
        (self.crate / "NOTICE").unlink()
        overrides = self.root / "scripts" / "license-overrides"
        overrides.mkdir(parents=True)
        (overrides / "test.txt").write_text("Synthetic pinned upstream license", encoding="utf-8")
        source = "https://example.invalid/upstream/commit/LICENSE"
        (overrides / "sources.json").write_text(json.dumps([{"name": "test-dependency", "version": "1.0.0", "file": "test.txt", "source": source}]), encoding="utf-8")
        output = self.root / "override"
        package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        index = json.loads((output / "payload" / "dependency-licenses" / "index.json").read_text(encoding="utf-8"))
        self.assertEqual(index[0]["license_origin"], source)
        self.assertTrue((output / "payload" / "dependency-licenses" / "test-dependency-1.0.0" / "LICENSE-upstream.txt").is_file())

    def test_existing_output_is_never_overwritten(self):
        output = self.root / "existing"
        output.mkdir()
        old = output / "precious.txt"
        old.write_text("keep", encoding="utf-8")
        with self.assertRaises(ValueError):
            package.prepare("x86_64-unknown-linux-gnu", output, self.binaries, self.metadata, self.root)
        self.assertEqual(old.read_text(encoding="utf-8"), "keep")

    def test_unsafe_names_rejected(self):
        for name in ("", ".", "..", "/absolute", "../outside", "a/../b", "a//b", "a\\b", "C:/file", "a\nb", "file.", "folder /file"):
            with self.subTest(name=name), self.assertRaises(ValueError):
                package.safe_name(name)
        self.assertEqual(package.safe_name("dependency-licenses/a-1.0/LICENSE"), "dependency-licenses/a-1.0/LICENSE")

    def test_case_collisions_rejected(self):
        # Windows cannot create both names; exercise this only on case-sensitive filesystems.
        output = self.root / "case"
        output.mkdir()
        (output / "FILE").write_text("a", encoding="utf-8")
        (output / "file").write_text("b", encoding="utf-8")
        if len(list(output.iterdir())) != 2:
            self.skipTest("Tiedostojärjestelmä ei erottele kirjainkokoa.")
        with self.assertRaises(ValueError):
            package.payload_files(output)

    def test_payload_over_updater_limits_is_rejected(self):
        with patch.object(package, "MAX_FILES", 0), self.assertRaisesRegex(ValueError, "päivittäjän"):
            package.payload_files(self.crate)
        with patch.object(package, "MAX_EXPANDED", 0), self.assertRaisesRegex(ValueError, "päivittäjän"):
            package.payload_files(self.crate)

    def test_symlinks_rejected(self):
        output = self.root / "linked"
        output.mkdir()
        try:
            (output / "link").symlink_to(self.crate / "LICENSE")
        except OSError:
            self.skipTest("Symbolisia linkkejä ei voi luoda tässä ympäristössä.")
        with self.assertRaises(ValueError):
            package.payload_files(output)


@unittest.skipUnless(shutil.which("openssl"), "Ed25519-testit vaativat OpenSSL:n.")
class SignedReleaseTests(PackagingFixture, unittest.TestCase):
    def setUp(self):
        super().setUp()
        key = self.root / "test-private.pem"
        subprocess.run(["openssl", "genpkey", "-algorithm", "ED25519", "-out", str(key)], check=True, capture_output=True)
        self.private_key = key.read_text(encoding="ascii")
        public_der = subprocess.check_output(["openssl", "pkey", "-in", str(key), "-pubout", "-outform", "DER"], stderr=subprocess.PIPE)
        self.public_key = public_der[len(signing.ED25519_PUBLIC_KEY_PREFIX):]
        self.public_path = self.root / "public.hex"
        self.public_path.write_text(self.public_key.hex() + "\n", encoding="ascii")
        self.source = self.root / "payloads"
        for target in package.TARGETS:
            package.prepare(target, self.source / target, self.binaries, self.metadata, self.root)

    def test_signed_archives_have_exact_manifest_and_hashes(self):
        output = self.root / "dist"
        manifest = signing.publish_files(self.source, output, self.public_path, self.private_key, "0.5.0", root=self.root)
        self.assertEqual(manifest["license"], "BSD-2-Clause")
        signature = (output / "release.sig").read_bytes()
        self.assertEqual(len(signature), 64)
        raw_manifest = (output / "release.json").read_bytes()
        self.assertEqual(json.loads(raw_manifest), manifest)
        for target, descriptor in manifest["targets"].items():
            archive_path = output / descriptor["asset"]
            if archive_path.name.endswith(".zip"):
                with zipfile.ZipFile(archive_path) as archive:
                    files = {name: archive.read(name) for name in archive.namelist()}
            else:
                with tarfile.open(archive_path) as archive:
                    files = {entry.name: archive.extractfile(entry).read() for entry in archive}
                    self.assertEqual(archive.getmember("as2026-gui").mode, 0o755)
            expected = {entry["name"] for entry in descriptor["files"]} | {"release.json", "release.sig"}
            self.assertEqual(set(files), expected)
            self.assertEqual(files["release.json"], raw_manifest)
            self.assertEqual(files["release.sig"], signature)
            self.assertEqual(files["LICENSE"], (self.root / "LICENSE").read_bytes())
            for entry in descriptor["files"]:
                self.assertEqual(len(files[entry["name"]]), entry["size"])
                self.assertEqual(hashlib.sha256(files[entry["name"]]).hexdigest(), entry["sha256"])

    def test_changed_or_extra_payload_rejected(self):
        payload = self.source / "x86_64-unknown-linux-gnu" / "payload"
        binary = payload / "as2026-gui"
        original = binary.read_bytes()
        binary.write_bytes(original + b"tampered")
        with self.assertRaisesRegex(ValueError, "muuttuneet"):
            signing.load_payloads(self.source, "0.5.0")
        binary.write_bytes(original)
        (payload / "surprise.exe").write_bytes(b"extra")
        with self.assertRaisesRegex(ValueError, "muuttuneet"):
            signing.load_payloads(self.source, "0.5.0")

    def test_both_platforms_are_required(self):
        shutil.rmtree(self.source / "x86_64-pc-windows-msvc")
        with self.assertRaisesRegex(ValueError, "sekä Linuxin että Windowsin"):
            signing.load_payloads(self.source, "0.5.0")

    def test_old_or_missing_payload_license_metadata_is_rejected(self):
        descriptor_path = self.source / "x86_64-unknown-linux-gnu" / "target.json"
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        for invalid in ("0BSD", None):
            if invalid is None:
                descriptor.pop("license", None)
            else:
                descriptor["license"] = invalid
            descriptor_path.write_text(json.dumps(descriptor), encoding="utf-8")
            with self.subTest(license=invalid), self.assertRaisesRegex(ValueError, "BSD-2-Clause"):
                signing.load_payloads(self.source, "0.5.0")

    def test_hash_consistent_payload_with_wrong_license_text_is_not_signed(self):
        directory = self.source / "x86_64-unknown-linux-gnu"
        (directory / "payload" / "LICENSE").write_text("Zero-Clause BSD\nOld license is not allowed\n", encoding="utf-8")
        descriptor_path = directory / "target.json"
        descriptor = json.loads(descriptor_path.read_text(encoding="utf-8"))
        descriptor["files"] = package.payload_files(directory / "payload")
        descriptor_path.write_text(json.dumps(descriptor), encoding="utf-8")
        output = self.root / "old-license-output"
        with self.assertRaisesRegex(ValueError, "täydellistä BSD-2-Clause"):
            signing.publish_files(self.source, output, self.public_path, self.private_key, "0.5.0", root=self.root)
        self.assertFalse(output.exists())

    def test_signing_rechecks_cargo_license_before_using_private_key(self):
        cargo = self.root / "Cargo.toml"
        cargo.write_text(cargo.read_text(encoding="utf-8").replace('license="BSD-2-Clause"', 'license="0BSD"'), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "Cargo.toml"):
            signing.publish_files(self.source, self.root / "bad-cargo-license", self.public_path, "", "0.5.0", root=self.root)

    def test_wrong_key_and_missing_key_fail_closed(self):
        with self.assertRaisesRegex(ValueError, "ei vastaa"):
            signing.sign_manifest(b"test manifest", self.private_key, bytes(32))
        with self.assertRaisesRegex(ValueError, "ei ole asetettu"):
            signing.sign_manifest(b"test manifest", "", self.public_key)

    def test_modified_manifest_does_not_verify(self):
        manifest = self.root / "original.json"
        signature = self.root / "signature"
        public = self.root / "public.der"
        manifest.write_bytes(b"signed manifest")
        signature.write_bytes(signing.sign_manifest(manifest.read_bytes(), self.private_key, self.public_key))
        public.write_bytes(signing.ED25519_PUBLIC_KEY_PREFIX + self.public_key)
        manifest.write_bytes(b"modified manifest")
        with self.assertRaises(ValueError):
            signing.run_openssl("pkeyutl", "-verify", "-rawin", "-pubin", "-inkey", str(public), "-keyform", "DER", "-in", str(manifest), "-sigfile", str(signature))

    def test_publisher_refuses_archives_over_updater_limit(self):
        output = self.root / "too-large"
        with patch.object(signing, "MAX_ARCHIVE", 1), self.assertRaisesRegex(ValueError, "päivittäjän"):
            signing.publish_files(self.source, output, self.public_path, self.private_key, "0.5.0", root=self.root)
        self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
