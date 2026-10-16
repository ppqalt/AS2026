#!/usr/bin/env python3
"""Allekirjoita molempien alustojen manifesti ja kokoa jakelupaketit."""

from __future__ import annotations

import argparse
import gzip
import io
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile

from package_release import (
    BINARIES, PROJECT_LICENSE, ROOT, TARGETS, asset_name, payload_files,
    project_version, validate_license_text, validate_project_license,
)


ED25519_PUBLIC_KEY_PREFIX = bytes.fromhex("302a300506032b6570032100")
MAX_MANIFEST = 2 * 1024 * 1024
MAX_ARCHIVE = 128 * 1024 * 1024


def run_openssl(*arguments: str) -> bytes:
    try:
        return subprocess.check_output(["openssl", *arguments], stderr=subprocess.PIPE)
    except subprocess.CalledProcessError as error:
        # Private-key material and OpenSSL diagnostics never enter CI logs.
        raise ValueError("OpenSSL ei pystynyt allekirjoittamaan tai varmistamaan julkaisua.") from error


def load_public_key(path: Path) -> bytes:
    value = path.read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[0-9a-fA-F]{64}", value):
        raise ValueError("Julkaisun julkisen Ed25519-avaimen pitää olla 32 tavua heksamuodossa.")
    return bytes.fromhex(value)


def sign_manifest(manifest: bytes, private_pem: str, public_key: bytes) -> bytes:
    if not private_pem.strip():
        raise ValueError("AS2026_RELEASE_SIGNING_KEY-salaisuutta ei ole asetettu.")
    with tempfile.TemporaryDirectory(prefix="as2026-sign-") as directory:
        temporary = Path(directory)
        key_path = temporary / "private.pem"
        descriptor = os.open(key_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "w", encoding="ascii", newline="\n") as output:
            output.write(private_pem)
        public_der = run_openssl("pkey", "-in", str(key_path), "-pubout", "-outform", "DER")
        if public_der != ED25519_PUBLIC_KEY_PREFIX + public_key:
            raise ValueError("Allekirjoitusavaimen julkinen osa ei vastaa ohjelmaan kiinnitettyä avainta.")
        manifest_path = temporary / "release.json"
        manifest_path.write_bytes(manifest)
        signature_path = temporary / "release.sig"
        run_openssl("pkeyutl", "-sign", "-rawin", "-inkey", str(key_path), "-in", str(manifest_path), "-out", str(signature_path))
        signature = signature_path.read_bytes()
        if len(signature) != 64:
            raise ValueError("Ed25519-allekirjoituksen pituus ei ole 64 tavua.")
        public_path = temporary / "public.der"
        public_path.write_bytes(public_der)
        run_openssl("pkeyutl", "-verify", "-rawin", "-pubin", "-inkey", str(public_path), "-keyform", "DER", "-in", str(manifest_path), "-sigfile", str(signature_path))
        return signature


def load_payloads(source: Path, version: str) -> tuple[dict, dict[str, Path]]:
    targets, payloads = {}, {}
    for directory in sorted(source.iterdir()):
        if not directory.is_dir() or directory.is_symlink():
            raise ValueError("Allekirjoituksen lähdehakemisto sisältää odottamattoman tiedoston.")
        descriptor = json.loads((directory / "target.json").read_text(encoding="utf-8"))
        target = descriptor["target"]
        if target not in TARGETS or target in targets:
            raise ValueError("Julkaisun alusta on tuntematon tai sama alusta esiintyy kahdesti.")
        if descriptor["schema"] != 1 or descriptor["version"] != version:
            raise ValueError("Alustan julkaisuversio tai manifestin rakenne poikkeaa julkaisusta.")
        if descriptor.get("license") != PROJECT_LICENSE:
            raise ValueError("Alustan AS2026-lisenssin pitää olla BSD-2-Clause.")
        if descriptor["asset"] != asset_name(version, target):
            raise ValueError("Alustan julkaisupaketin nimi poikkeaa odotetusta.")
        payload = directory / "payload"
        if payload.is_symlink() or not payload.is_dir():
            raise ValueError("Julkaisun hyötytiedostohakemisto puuttuu.")
        files = payload_files(payload)
        # Windows- ja Linux-Path lajittelevat isot ja pienet kirjaimet eri tavoin.
        # Vertaa täsmälleen samoja nimiä, kokoja ja SHA-256-tiivisteitä
        # riippumatta alustalla tuotetun tiedostoluettelon järjestyksestä.
        if sorted(files, key=lambda entry: entry["name"]) != sorted(
            descriptor["files"], key=lambda entry: entry["name"]
        ):
            raise ValueError("Alustan hyötytiedostot ovat muuttuneet tarkistussumman laskemisen jälkeen.")
        suffix = TARGETS[target][1]
        required = {f"{binary}{suffix}" for binary in BINARIES} | {"LICENSE", "THIRD_PARTY.md", "dependency-licenses/index.json"}
        if not required.issubset({entry["name"] for entry in files}):
            raise ValueError("Julkaisusta puuttuu suoritettava tiedosto tai lisenssi.")
        validate_license_text((payload / "LICENSE").read_bytes())
        targets[target] = {"asset": descriptor["asset"], "files": files}
        payloads[target] = payload
    if set(targets) != set(TARGETS):
        raise ValueError("Julkaisu edellyttää sekä Linuxin että Windowsin valmista pakettia.")
    return {"schema": 1, "version": version, "license": PROJECT_LICENSE, "targets": targets}, payloads


def archive_mode(name: str, target: str) -> int:
    return 0o755 if target.endswith("linux-gnu") and name in BINARIES else 0o644


def write_archive(path: Path, target: str, files: dict[str, bytes]) -> None:
    if path.name.endswith(".tar.gz"):
        with path.open("xb") as raw:
            with gzip.GzipFile(filename="", mode="wb", fileobj=raw, mtime=0) as compressed:
                with tarfile.open(mode="w", fileobj=compressed, format=tarfile.USTAR_FORMAT) as archive:
                    for name, data in sorted(files.items()):
                        info = tarfile.TarInfo(name)
                        info.size, info.mode, info.mtime = len(data), archive_mode(name, target), 0
                        archive.addfile(info, io.BytesIO(data))
    elif path.name.endswith(".zip"):
        with zipfile.ZipFile(path, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for name, data in sorted(files.items()):
                info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | archive_mode(name, target)) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(info, data)
    else:
        raise ValueError("Tuntematon julkaisuarkiston muoto.")


def publish_files(source: Path, output: Path, public_key_path: Path, private_pem: str, version: str, root: Path = ROOT) -> dict:
    validate_project_license(root)
    if project_version(root) != version:
        raise ValueError("Allekirjoitettavan paketin versio poikkeaa Cargo.toml:sta.")
    manifest, payloads = load_payloads(source, version)
    raw_manifest = (json.dumps(manifest, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    if len(raw_manifest) > MAX_MANIFEST:
        raise ValueError("Julkaisuluettelo ylittää päivittäjän kokorajan.")
    signature = sign_manifest(raw_manifest, private_pem, load_public_key(public_key_path))
    if output.exists():
        raise ValueError("Julkaisun tuloshakemisto on jo olemassa. Allekirjoitettuja paketteja ei ylikirjoiteta.")
    output.mkdir(parents=True)
    try:
        (output / "release.json").write_bytes(raw_manifest)
        (output / "release.sig").write_bytes(signature)
        for target, payload in payloads.items():
            entries = {entry["name"]: (payload / entry["name"]).read_bytes() for entry in manifest["targets"][target]["files"]}
            entries.update({"release.json": raw_manifest, "release.sig": signature})
            archive_path = output / manifest["targets"][target]["asset"]
            write_archive(archive_path, target, entries)
            if archive_path.stat().st_size > MAX_ARCHIVE:
                raise ValueError("Julkaisuarkisto ylittää päivittäjän kokorajan.")
    except BaseException:
        shutil.rmtree(output)
        raise
    return manifest


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--public-key", type=Path, default=ROOT / "release-public-key.hex")
    args = parser.parse_args()
    validate_project_license()
    manifest = publish_files(args.input, args.output, args.public_key, os.environ.get("AS2026_RELEASE_SIGNING_KEY", ""), project_version())
    print(f"AS2026 {manifest['version']}: Linux- ja Windows-paketit allekirjoitettu ja tarkistettu.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Julkaisun allekirjoitus epäonnistui: {error}", file=sys.stderr)
        sys.exit(1)
