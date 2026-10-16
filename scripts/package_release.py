#!/usr/bin/env python3
"""Kokoa tarkistetut julkaisutiedostot. Python 3.11+, vain vakiokirjasto."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parent.parent
TARGETS = {
    "x86_64-unknown-linux-gnu": ("linux-x86_64", ""),
    "x86_64-pc-windows-msvc": ("windows-x86_64", ".exe"),
}
BINARIES = ("as2026", "as2026-gui", "as2026-launcher")
STABLE_VERSION = re.compile(r"(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\Z")
LICENSE_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT")
MAX_FILES = 4096
MAX_EXPANDED = 512 * 1024 * 1024
PROJECT_LICENSE = "BSD-2-Clause"
BSD_2_CLAUSE_TEXT = '''BSD 2-Clause License

Copyright (c) 2026 AS2026 contributors

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS" AND
ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
'''


def validate_license_text(raw: bytes) -> None:
    try:
        text = raw.decode("utf-8")
    except UnicodeDecodeError as error:
        raise ValueError("AS2026:n LICENSE-tiedoston pitää olla UTF-8-muotoinen BSD-2-Clause-lisenssi.") from error
    # Line wrapping and platform-specific line endings do not change the
    # license, but every word of the notice, conditions and disclaimer matters.
    if " ".join(text.split()) != " ".join(BSD_2_CLAUSE_TEXT.split()):
        raise ValueError("AS2026:n LICENSE-tiedosto ei vastaa hyväksyttyä täydellistä BSD-2-Clause-lisenssiä.")


def validate_project_license(root: Path = ROOT) -> None:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    if manifest["package"].get("license") != PROJECT_LICENSE:
        raise ValueError("Cargo.toml:n AS2026-lisenssin pitää olla BSD-2-Clause.")
    if manifest["package"].get("license-file") not in (None, "LICENSE"):
        raise ValueError("AS2026:n lisenssitiedoston pitää olla projektin LICENSE.")
    validate_license_text((root / "LICENSE").read_bytes())


def validate_metadata_license(metadata: dict) -> None:
    root_id = metadata.get("resolve", {}).get("root")
    package = next((item for item in metadata.get("packages", []) if item["id"] == root_id), None)
    if package is None or package.get("license") != PROJECT_LICENSE:
        raise ValueError("Cargo-metatietojen AS2026-lisenssin pitää olla BSD-2-Clause. Päivitä vanha metadata.")


def stable_version(value: str) -> str:
    if not STABLE_VERSION.fullmatch(value):
        raise ValueError("Julkaisuversion pitää olla vakaa numeroitu versio, esimerkiksi 0.5.0.")
    return value


def project_version(root: Path = ROOT) -> str:
    manifest = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))
    return stable_version(manifest["package"]["version"])


def verify_release(tag: str, root: Path = ROOT, main_ref: str | None = "origin/main") -> str:
    validate_project_license(root)
    version = project_version(root)
    if tag != f"v{version}":
        raise ValueError("Julkaisutagin ja Cargo.toml-version on vastattava toisiaan.")
    if main_ref is not None:
        head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip()
        tagged = subprocess.check_output(["git", "rev-parse", f"refs/tags/{tag}^{{}}"], cwd=root, text=True).strip()
        if head != tagged:
            raise ValueError("Työpuun HEAD ei vastaa julkaisutagia.")
        subprocess.run(["git", "merge-base", "--is-ancestor", head, main_ref], cwd=root, check=True)
    return version


def asset_name(version: str, target: str) -> str:
    platform, extension = TARGETS[target]
    archive_extension = "zip" if extension else "tar.gz"
    return f"AS2026-{stable_version(version)}-{platform}.{archive_extension}"


def safe_name(value: str) -> str:
    path = PurePosixPath(value)
    if (
        not value
        or len(value.encode("utf-8")) > 512
        or not path.parts
        or "\\" in value
        or ":" in value
        or path.is_absolute()
        or str(path) != value
        or any(part in ("", ".", "..") for part in path.parts)
        or any(part.endswith((".", " ")) for part in path.parts)
        or any(ord(char) < 32 for char in value)
    ):
        raise ValueError(f"Virheellinen julkaisutiedoston nimi: {value!r}")
    return value


def payload_files(payload: Path) -> list[dict]:
    result = []
    names = set()
    for path in sorted(payload.rglob("*")):
        if path.is_symlink():
            raise ValueError("Julkaisupaketissa ei sallita symbolisia linkkejä.")
        if not path.is_file():
            continue
        name = safe_name(path.relative_to(payload).as_posix())
        if name.casefold() in names:
            raise ValueError("Julkaisupaketissa on Windowsissa samaksi tulkittavia tiedostonimiä.")
        names.add(name.casefold())
        if name in ("release.json", "release.sig"):
            raise ValueError("Allekirjoitustiedosto ei saa olla hyötytiedostojen joukossa.")
        data = path.read_bytes()
        result.append({"name": name, "size": len(data), "sha256": hashlib.sha256(data).hexdigest()})
    if not result:
        raise ValueError("Julkaisupaketti on tyhjä.")
    if len(result) > MAX_FILES or sum(entry["size"] for entry in result) > MAX_EXPANDED:
        raise ValueError("Julkaisupaketti ylittää päivittäjän tiedostomäärä- tai kokorajan.")
    return result


def dependency_packages(metadata: dict) -> list[dict]:
    resolve = metadata.get("resolve")
    if not resolve or not resolve.get("root"):
        raise ValueError("Cargo metadata ei sisällä ratkaistua AS2026-riippuvuuspuuta.")
    nodes = {node["id"]: node for node in resolve["nodes"]}
    root_id = resolve["root"]
    reached, pending = set(), [root_id]
    while pending:
        package_id = pending.pop()
        if package_id in reached:
            continue
        reached.add(package_id)
        node = nodes[package_id]
        pending.extend(dependency["pkg"] for dependency in node["deps"])
    return sorted(
        (package for package in metadata["packages"] if package["id"] in reached and package["id"] != root_id),
        key=lambda package: (package["name"], package["version"]),
    )


def license_paths(package: dict) -> list[Path]:
    crate = Path(package["manifest_path"]).parent.resolve()
    result = set()
    explicit = package.get("license_file")
    if explicit:
        explicit_path = (crate / explicit).resolve()
        if not explicit_path.is_relative_to(crate):
            raise ValueError(f"Lisenssitiedosto on Cargo-paketin ulkopuolella: {package['name']}")
        if explicit_path.is_file():
            result.add(explicit_path)
    for path in crate.rglob("*"):
        if path.is_symlink():
            continue
        relative = path.relative_to(crate)
        if path.is_file() and any(part.upper().startswith(LICENSE_PREFIXES) for part in relative.parts):
            result.add(path)
    return sorted(result)


def collect_licenses(metadata: dict, destination: Path, overrides: Path) -> None:
    sources_path = overrides / "sources.json"
    sources = json.loads(sources_path.read_text(encoding="utf-8")) if sources_path.exists() else []
    overrides_by_package = {(item["name"], item["version"]): item for item in sources}
    index = []
    for package in dependency_packages(metadata):
        crate = Path(package["manifest_path"]).parent.resolve()
        key = (package["name"], package["version"])
        crate_name = safe_name(f"{key[0]}-{key[1]}")
        if "/" in crate_name:
            raise ValueError("Cargo-paketin nimi ei kelpaa lisenssihakemistoksi.")
        output = destination / crate_name
        output.mkdir(parents=True)
        notices = license_paths(package)
        files = []
        for source in notices:
            relative = safe_name(source.relative_to(crate).as_posix())
            target = output / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, target)
            files.append(relative)
        override = overrides_by_package.get(key)
        origin = None
        if not notices and override:
            override_name = safe_name(override["file"])
            shutil.copyfile(overrides / override_name, output / "LICENSE-upstream.txt")
            files.append("LICENSE-upstream.txt")
            origin = override["source"]
        if not files:
            raise ValueError(f"Riippuvuuden lisenssiteksti puuttuu: {crate_name}. Lisää tarkistettu kopio scripts/license-overrides-hakemistoon.")
        # REUSE projects keep author notices in source headers rather than in
        # their shared SPDX license templates. Preserve these notices as well.
        copyright_lines = set()
        for source in crate.rglob("*"):
            if source.is_symlink() or not source.is_file() or source.suffix.lower() not in (".rs", ".slint", ".c", ".cpp", ".h", ".hpp"):
                continue
            with source.open("rb") as input_file:
                header = input_file.read(65536).decode("utf-8", errors="replace")
            copyright_lines.update(line.strip() for line in header.splitlines() if "SPDX-FileCopyrightText:" in line)
        if copyright_lines:
            (output / "COPYRIGHT-NOTICES.txt").write_text(
                "SPDX copyright notices from the upstream package source headers:\n\n"
                + "\n".join(sorted(copyright_lines)) + "\n", encoding="utf-8",
            )
            files.append("COPYRIGHT-NOTICES.txt")
        index.append({
            "name": key[0], "version": key[1], "license": package.get("license"),
            "authors": package.get("authors", []), "repository": package.get("repository"),
            "source": package.get("source"), "license_origin": origin,
            "license_notice": override.get("notice") if origin else None, "files": files,
        })
    destination.mkdir(parents=True, exist_ok=True)
    (destination / "index.json").write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    (destination / "LUE-MINUT.txt").write_text(
        "AS2026:n Cargo-riippuvuuksien lisenssit ja tekijämaininnat\n\n"
        "index.json luettelee paketin, version, lisenssin ja alkuperän. Mukana on myös\n"
        "käännöstyökaluja; kaikkia näitä kirjastoja ei jaeta ajettavan ohjelman osana.\n"
        "Kopioidut lisenssit kuuluvat alkuperäisille tekijöilleen. AS2026:n oma\n"
        "BSD-2-Clause-lisenssi ei korvaa näitä ehtoja. Slintin valittu työpöytälisenssi\n"
        "ja ohjelman edellyttämä maininta on kuvattu tiedostossa THIRD_PARTY.md.\n",
        encoding="utf-8",
    )


def prepare(target: str, output: Path, binary_dir: Path, metadata: dict, root: Path = ROOT) -> dict:
    validate_project_license(root)
    validate_metadata_license(metadata)
    version = project_version(root)
    if output.exists():
        raise ValueError("Julkaisun väliaikaishakemisto on jo olemassa. Valitse uusi tyhjä polku.")
    suffix = TARGETS[target][1]
    binaries = [binary_dir / f"{name}{suffix}" for name in BINARIES]
    for binary in binaries:
        if binary.is_symlink() or not binary.is_file() or binary.stat().st_size == 0:
            raise ValueError(f"Julkaisun suoritettava tiedosto puuttuu tai on tyhjä: {binary.name}")
    payload = output / "payload"
    payload.mkdir(parents=True)
    try:
        for binary in binaries:
            shutil.copyfile(binary, payload / binary.name)
            os.chmod(payload / binary.name, 0o755 if not suffix else 0o644)
        for name in ("LICENSE", "THIRD_PARTY.md"):
            shutil.copyfile(root / name, payload / name)
        validate_license_text((payload / "LICENSE").read_bytes())
        collect_licenses(metadata, payload / "dependency-licenses", root / "scripts" / "license-overrides")
        descriptor = {
            "schema": 1, "version": version, "license": PROJECT_LICENSE, "target": target,
            "asset": asset_name(version, target), "files": payload_files(payload),
        }
        (output / "target.json").write_text(json.dumps(descriptor, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        return descriptor
    except BaseException:
        shutil.rmtree(output)
        raise


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subcommands = parser.add_subparsers(dest="command", required=True)
    subcommands.add_parser("check-license", help="Varmista Cargo.toml:n ja LICENSE-tiedoston täydellinen BSD-2-Clause-lisenssi.")
    verify = subcommands.add_parser("verify", help="Tarkista Cargo-versio, vakaa tagi ja main-haaran historia.")
    verify.add_argument("--tag", required=True)
    verify.add_argument("--main-ref", default="origin/main")
    stage = subcommands.add_parser("prepare", help="Kokoa yhden alustan binäärit ja riippuvuuksien lisenssit.")
    stage.add_argument("--target", choices=TARGETS, required=True)
    stage.add_argument("--output", type=Path, required=True)
    stage.add_argument("--binary-dir", type=Path, default=ROOT / "target" / "release")
    stage.add_argument("--metadata", type=Path, help="Valmiiksi haettu Cargo metadata JSON (muuten Cargo hakee sen).")
    args = parser.parse_args()
    if args.command == "check-license":
        validate_project_license()
        print("AS2026:n Cargo-tiedot ja täydellinen BSD-2-Clause-lisenssi hyväksytty.")
    elif args.command == "verify":
        print(verify_release(args.tag, main_ref=args.main_ref))
    else:
        validate_project_license()
        if args.metadata:
            metadata = json.loads(args.metadata.read_text(encoding="utf-8"))
        else:
            metadata = json.loads(subprocess.check_output([
                "cargo", "metadata", "--locked", "--format-version", "1", "--features", "gui",
                "--filter-platform", args.target,
            ], cwd=ROOT))
        descriptor = prepare(args.target, args.output, args.binary_dir, metadata)
        print(f"{descriptor['asset']}: {len(descriptor['files'])} tiedostoa valmisteltu.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        print(f"Julkaisupaketin valmistelu epäonnistui: {error}", file=sys.stderr)
        sys.exit(1)
