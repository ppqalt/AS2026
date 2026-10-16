//! AS2026 release packaging and Ed25519 signing without Python.
//! Keep schema 1, payload validation and archive layout compatible with the 0.5.4 updater.
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey, pkcs8::DecodePrivateKey};
use flate2::{Compression, GzBuilder};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    process::Command,
};

const PROJECT_LICENSE: &str = "BSD-2-Clause";
const BINARIES: [&str; 3] = ["as2026", "as2026-gui", "as2026-launcher"];
const LINUX: &str = "x86_64-unknown-linux-gnu";
const WINDOWS: &str = "x86_64-pc-windows-msvc";
const MAX_FILES: usize = 4096;
const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
const MAX_MANIFEST: usize = 2 * 1024 * 1024;
const MAX_ARCHIVE: u64 = 128 * 1024 * 1024;
const LICENSE_TEXT: &str = include_str!("../../../LICENSE");

type Result<T> = std::result::Result<T, String>;
fn fail(message: impl Into<String>) -> String { message.into() }
fn io_error(error: io::Error) -> String { format!("Tiedostotoiminto epäonnistui: {error}") }
fn load(path: &Path) -> Result<Vec<u8>> { fs::read(path).map_err(io_error) }
fn read_text(path: &Path) -> Result<String> { fs::read_to_string(path).map_err(io_error) }
fn write_file(path: &Path, content: impl AsRef<[u8]>) -> Result<()> {
    fs::write(path, content).map_err(io_error)
}
fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    fs::copy(source, destination).map_err(io_error)?;
    Ok(())
}
fn parse_json(bytes: &[u8]) -> Result<Value> {
    serde_json::from_slice(bytes).map_err(|e| format!("Virheellinen JSON: {e}"))
}
fn pretty_json(value: &Value) -> Result<Vec<u8>> {
    let mut contents = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    contents.push(b'\n');
    Ok(contents)
}
fn root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..") }
fn canonical_version(value: &str) -> Result<&str> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3 || parts.iter().any(|p| p.is_empty() ||
        (p.len() > 1 && p.starts_with('0')) || !p.bytes().all(|c| c.is_ascii_digit())) {
        return Err(fail("Julkaisuversion pitää olla vakaa numeroitu versio, esimerkiksi 0.5.0."));
    }
    Ok(value)
}
fn cargo_manifest(root: &Path) -> Result<toml::Value> {
    toml::from_str(&read_text(&root.join("Cargo.toml"))?)
        .map_err(|e| format!("Virheellinen Cargo.toml: {e}"))
}
fn project_version(root: &Path) -> Result<String> {
    let manifest = cargo_manifest(root)?;
    let version = manifest.get("package").and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str).ok_or("Cargo.toml: versio puuttuu")?;
    Ok(canonical_version(version)?.to_owned())
}
fn normalized_license(value: &str) -> String { value.split_whitespace().collect::<Vec<_>>().join(" ") }
fn check_license_bytes(contents: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(contents).map_err(|_| "LICENSE ei ole UTF-8-muotoinen")?;
    if normalized_license(text) != normalized_license(LICENSE_TEXT) {
        return Err(fail("LICENSE ei vastaa hyväksyttyä täydellistä BSD-2-Clause-lisenssiä"));
    }
    Ok(())
}
fn check_license(root: &Path) -> Result<()> {
    let manifest = cargo_manifest(root)?;
    let pkg = manifest.get("package").ok_or("Cargo.toml: package puuttuu")?;
    if pkg.get("license").and_then(toml::Value::as_str) != Some(PROJECT_LICENSE) ||
        !matches!(pkg.get("license-file").and_then(toml::Value::as_str), None | Some("LICENSE")) {
        return Err(fail("Cargo.toml:n AS2026-lisenssin pitää olla BSD-2-Clause"));
    }
    check_license_bytes(&load(&root.join("LICENSE"))?)
}
fn git(root: &Path, args: &[&str]) -> Result<String> {
    let result = Command::new("git").args(args).current_dir(root)
        .output().map_err(io_error)?;
    if !result.status.success() {
        return Err(fail(format!("Git-tarkistus epäonnistui: git {}", args.join(" "))));
    }
    String::from_utf8(result.stdout).map(|s| s.trim().to_owned()).map_err(|e| e.to_string())
}
fn verify_tag(root: &Path, tag: &str, main_ref: &str) -> Result<String> {
    check_license(root)?;
    let version = project_version(root)?;
    if tag != format!("v{version}") {
        return Err(fail("Julkaisutagin ja Cargo.toml-version on vastattava toisiaan"));
    }
    let head = git(root, &["rev-parse", "HEAD"])?;
    let tagged = git(root, &["rev-parse", &format!("refs/tags/{tag}^{{}}")])?;
    if head != tagged { return Err(fail("Työpuun HEAD ei vastaa julkaisutagia")); }
    git(root, &["merge-base", "--is-ancestor", &head, main_ref])?;
    Ok(version)
}
fn target_asset(version: &str, target: &str) -> Result<String> {
    canonical_version(version)?;
    match target {
        LINUX => Ok(format!("AS2026-{version}-linux-x86_64.tar.gz")),
        WINDOWS => Ok(format!("AS2026-{version}-windows-x86_64.zip")),
        _ => Err(fail("Tuntematon julkaisualusta")),
    }
}
fn safe_relative(name: &str) -> Result<PathBuf> {
    let path = Path::new(name);
    if name.is_empty() || name.len() > 512 || name.contains(['\\', ':']) ||
        name.chars().any(char::is_control) || path.is_absolute() ||
        name.split('/').any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' '])) ||
        path.components().any(|p| !matches!(p, Component::Normal(_))) {
        return Err(fail(format!("Virheellinen julkaisutiedoston nimi: {name:?}")));
    }
    Ok(path.to_owned())
}
fn walk(path: &Path, result: &mut Vec<PathBuf>, reject_symlinks: bool) -> Result<()> {
    let mut entries = fs::read_dir(path).map_err(io_error)?
        .map(|entry| entry.map(|e| e.path()).map_err(io_error)).collect::<Result<Vec<_>>>()?;
    entries.sort();
    for item in entries {
        let kind = fs::symlink_metadata(&item).map_err(io_error)?.file_type();
        if kind.is_symlink() {
            if reject_symlinks { return Err(fail("Julkaisupaketissa ei sallita symbolisia linkkejä")); }
        } else if kind.is_dir() {
            walk(&item, result, reject_symlinks)?;
        } else if kind.is_file() {
            result.push(item);
        } else if reject_symlinks {
            return Err(fail("Julkaisupaketissa on odottamaton tiedostotyyppi"));
        }
    }
    Ok(())
}
fn files_under(path: &Path, reject_symlinks: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    walk(path, &mut files, reject_symlinks)?;
    Ok(files)
}
fn digest(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn payload_files(payload: &Path) -> Result<Vec<Value>> {
    let mut files = Vec::new();
    let mut names = BTreeSet::new();
    let mut total = 0u64;
    for path in files_under(payload, true)? {
        let name = path.strip_prefix(payload).map_err(|e| e.to_string())?
            .to_str().ok_or("Julkaisupolun pitää olla UTF-8")?.replace('\\', "/");
        safe_relative(&name)?;
        if !names.insert(name.to_lowercase()) || name == "release.json" || name == "release.sig" {
            return Err(fail("Julkaisupaketissa on ristiriitaisia tiedostonimiä"));
        }
        let data = load(&path)?;
        total = total.checked_add(data.len() as u64).ok_or("Julkaisupaketti on liian suuri")?;
        files.push(json!({"name": name, "size": data.len(), "sha256": digest(&data)}));
    }
    if files.is_empty() || files.len() > MAX_FILES || total > MAX_EXPANDED {
        return Err(fail("Julkaisupaketti ylittää päivittäjän tiedostomäärä- tai kokorajan"));
    }
    files.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    Ok(files)
}
fn metadata_json(root: &Path, target: &str) -> Result<Value> {
    let output = Command::new("cargo").current_dir(root)
        .args(["metadata", "--locked", "--format-version", "1", "--features", "gui", "--filter-platform", target])
        .output().map_err(io_error)?;
    if !output.status.success() { return Err(fail("cargo metadata epäonnistui")); }
    parse_json(&output.stdout)
}
fn metadata_packages(meta: &Value) -> Result<Vec<Value>> {
    let resolved = &meta["resolve"];
    let root = resolved["root"].as_str().ok_or("Cargo metadata: juuripaketti puuttuu")?;
    let mut nodes = BTreeMap::new();
    for node in resolved["nodes"].as_array().ok_or("Cargo metadata: nodes puuttuu")? {
        let id = node["id"].as_str().ok_or("Cargo metadata: node id puuttuu")?;
        nodes.insert(id, node);
    }
    let mut visited = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !visited.insert(id) { continue; }
        let node = nodes.get(id).ok_or("Cargo metadata: riippuvuus puuttuu")?;
        for dep in node["deps"].as_array().ok_or("Cargo metadata: deps puuttuu")? {
            pending.push(dep["pkg"].as_str().ok_or("Cargo metadata: pkg puuttuu")?);
        }
    }
    let mut packages: Vec<Value> = meta["packages"].as_array().ok_or("Cargo metadata: packages puuttuu")?
        .iter().filter(|p| p["id"].as_str().is_some_and(|id| id != root && visited.contains(id)))
        .cloned().collect();
    packages.sort_by(|a, b| (a["name"].as_str(), a["version"].as_str()).cmp(&(b["name"].as_str(), b["version"].as_str())));
    Ok(packages)
}
fn license_file(path: &Path) -> bool {
    const PREFIXES: &[&str] = &["LICENSE", "LICENCE", "COPYING", "NOTICE", "COPYRIGHT"];
    path.components().any(|part| {
        let text = part.as_os_str().to_string_lossy().to_uppercase();
        PREFIXES.iter().any(|prefix| text.starts_with(prefix))
    })
}
fn collect_licenses(meta: &Value, destination: &Path, overrides: &Path) -> Result<()> {
    let source_file = overrides.join("sources.json");
    let sources = if source_file.exists() { parse_json(&load(&source_file)?)? } else { json!([]) };
    let list = sources.as_array().ok_or("sources.json ei ole lista")?;
    let mut index = Vec::new();
    for package in metadata_packages(meta)? {
        let name = package["name"].as_str().ok_or("Riippuvuuden nimi puuttuu")?;
        let version = package["version"].as_str().ok_or("Riippuvuuden versio puuttuu")?;
        let id = format!("{name}-{version}");
        if safe_relative(&id)?.components().count() != 1 { return Err(fail("Virheellinen crate-nimi")); }
        let crate_dir = Path::new(package["manifest_path"].as_str().ok_or("manifest_path puuttuu")?)
            .parent().ok_or("manifest_path ei ole tiedostopolku")?.canonicalize().map_err(io_error)?;
        let package_out = destination.join(&id);
        fs::create_dir_all(&package_out).map_err(io_error)?;
        let mut licenses = BTreeSet::new();
        if let Some(explicit) = package["license_file"].as_str() {
            let entry = crate_dir.join(explicit);
            if entry.exists() {
                let resolved = entry.canonicalize().map_err(io_error)?;
                if !resolved.starts_with(&crate_dir) { return Err(fail("Lisenssitiedosto on Cargo-paketin ulkopuolella")); }
                licenses.insert(resolved);
            }
        }
        let source_files = files_under(&crate_dir, false)?;
        for source in &source_files {
            if license_file(source.strip_prefix(&crate_dir).map_err(|e| e.to_string())?) {
                licenses.insert(source.clone());
            }
        }
        let mut copied = Vec::new();
        for source in &licenses {
            let relative = source.strip_prefix(&crate_dir).map_err(|_| "Virheellinen lisenssipolku")?;
            let relative_name = relative.to_str().ok_or("Virheellinen UTF-8-lisenssipolku")?.replace('\\', "/");
            safe_relative(&relative_name)?;
            let destination_file = package_out.join(relative);
            fs::create_dir_all(destination_file.parent().ok_or("Virheellinen lisenssipolku")?).map_err(io_error)?;
            copy_file(source, &destination_file)?;
            copied.push(relative_name);
        }
        let override_entry = list.iter().find(|item| item["name"] == name && item["version"] == version);
        let mut origin: Option<String> = None;
        let mut notice: Option<String> = None;
        if copied.is_empty() {
            if let Some(item) = override_entry {
                let file = item["file"].as_str().ok_or("Lisenssin override-tiedosto puuttuu")?;
                safe_relative(file)?;
                copy_file(&overrides.join(file), &package_out.join("LICENSE-upstream.txt"))?;
                copied.push("LICENSE-upstream.txt".to_owned());
                origin = item["source"].as_str().map(str::to_owned);
                notice = item["notice"].as_str().map(str::to_owned);
            }
        }
        if copied.is_empty() { return Err(fail(format!("Riippuvuuden lisenssiteksti puuttuu: {id}"))); }
        let mut copyright = BTreeSet::new();
        for source in &source_files {
            let extension = source.extension().and_then(|ext| ext.to_str()).unwrap_or("").to_lowercase();
            if !["rs", "slint", "c", "cpp", "h", "hpp"].contains(&extension.as_str()) { continue; }
            let mut prefix = vec![0u8; 65_536];
            let mut file = fs::File::open(source).map_err(io_error)?;
            let count = file.read(&mut prefix).map_err(io_error)?;
            let header = String::from_utf8_lossy(&prefix[..count]);
            for line in header.lines() {
                if line.contains("SPDX-FileCopyrightText:") { copyright.insert(line.trim().to_owned()); }
            }
        }
        if !copyright.is_empty() {
            write_file(&package_out.join("COPYRIGHT-NOTICES.txt"), format!(
                "SPDX copyright notices from the upstream package source headers:\n\n{}\n",
                copyright.into_iter().collect::<Vec<_>>().join("\n")))?;
            copied.push("COPYRIGHT-NOTICES.txt".to_owned());
        }
        copied.sort();
        index.push(json!({
            "name": name, "version": version, "license": package["license"],
            "authors": package["authors"], "repository": package["repository"],
            "source": package["source"], "license_origin": origin,
            "license_notice": notice, "files": copied,
        }));
    }
    fs::create_dir_all(destination).map_err(io_error)?;
    write_file(&destination.join("index.json"), pretty_json(&json!(index))?)?;
    write_file(&destination.join("LUE-MINUT.txt"), "AS2026:n Cargo-riippuvuuksien lisenssit ja tekijämaininnat\n\nindex.json luettelee paketin, version, lisenssin ja alkuperän. Mukana on myös\nkäännöstyökaluja; kaikkia näitä kirjastoja ei jaeta ajettavan ohjelman osana.\nKopioidut lisenssit kuuluvat alkuperäisille tekijöilleen. AS2026:n oma\nBSD-2-Clause-lisenssi ei korvaa näitä ehtoja. Slintin valittu työpöytälisenssi\nja ohjelman edellyttämä maininta on kuvattu tiedostossa THIRD_PARTY.md.\n")?;
    Ok(())
}
fn metadata_license(meta: &Value) -> Result<()> {
    let root = meta["resolve"]["root"].as_str().ok_or("Cargo metadata ei sisällä juuripakettia")?;
    if !meta["packages"].as_array().ok_or("Cargo metadata ei sisällä paketteja")?
        .iter().any(|p| p["id"] == root && p["license"] == PROJECT_LICENSE) {
        return Err(fail("Cargo-metatietojen AS2026-lisenssin pitää olla BSD-2-Clause"));
    }
    Ok(())
}
fn regular_nonempty(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path).map_err(io_error)?;
    if !meta.is_file() || meta.file_type().is_symlink() || meta.len() == 0 {
        return Err(fail(format!("Suoritettava tiedosto puuttuu tai on tyhjä: {}", path.display())));
    }
    Ok(())
}
#[cfg(unix)]
fn set_mode(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(mode)).map_err(io_error)
}
#[cfg(not(unix))]
fn set_mode(_: &Path, _: u32) -> Result<()> { Ok(()) }
fn prepare(root: &Path, target: &str, output: &Path, binary_dir: &Path, metadata: &Value) -> Result<Value> {
    check_license(root)?;
    metadata_license(metadata)?;
    let version = project_version(root)?;
    let asset = target_asset(&version, target)?;
    if output.exists() { return Err(fail("Julkaisun väliaikaishakemisto on jo olemassa")); }
    let suffix = if target == WINDOWS { ".exe" } else { "" };
    let binaries: Vec<_> = BINARIES.iter().map(|name| binary_dir.join(format!("{name}{suffix}"))).collect();
    for binary in &binaries { regular_nonempty(binary)?; }
    let payload = output.join("payload");
    fs::create_dir_all(&payload).map_err(io_error)?;
    let result = (|| {
        for source in &binaries {
            let name = source.file_name().ok_or("Virheellinen suoritustiedosto")?;
            let destination = payload.join(name);
            copy_file(source, &destination)?;
            set_mode(&destination, if target == LINUX { 0o755 } else { 0o644 })?;
        }
        for name in ["LICENSE", "THIRD_PARTY.md"] {
            copy_file(&root.join(name), &payload.join(name))?;
        }
        check_license_bytes(&load(&payload.join("LICENSE"))?)?;
        collect_licenses(metadata, &payload.join("dependency-licenses"), &root.join("scripts/license-overrides"))?;
        let descriptor = json!({
            "schema": 1, "version": version, "license": PROJECT_LICENSE,
            "target": target, "asset": asset, "files": payload_files(&payload)?
        });
        write_file(&output.join("target.json"), pretty_json(&descriptor)?)?;
        Ok(descriptor)
    })();
    if result.is_err() { let _ = fs::remove_dir_all(output); }
    result
}
fn payloads(input: &Path, version: &str) -> Result<(Value, BTreeMap<String, PathBuf>)> {
    let mut targets = serde_json::Map::new();
    let mut paths = BTreeMap::new();
    let mut dirs = fs::read_dir(input).map_err(io_error)?
        .map(|e| e.map(|x| x.path()).map_err(io_error)).collect::<Result<Vec<_>>>()?;
    dirs.sort();
    for dir in dirs {
        let metadata = fs::symlink_metadata(&dir).map_err(io_error)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(fail("Allekirjoituksen lähdehakemisto sisältää odottamattoman tiedoston"));
        }
        let desc = parse_json(&load(&dir.join("target.json"))?)?;
        let target = desc["target"].as_str().ok_or("Julkaisualusta puuttuu")?;
        if ![LINUX, WINDOWS].contains(&target) || targets.contains_key(target) {
            return Err(fail("Julkaisun alusta on tuntematon tai sama alusta esiintyy kahdesti"));
        }
        if desc["schema"] != 1 || desc["version"] != version || desc["license"] != PROJECT_LICENSE ||
            desc["asset"] != target_asset(version, target)? {
            return Err(fail("Alustan julkaisuversio, lisenssi tai rakenne on virheellinen"));
        }
        let payload = dir.join("payload");
        let meta = fs::symlink_metadata(&payload).map_err(io_error)?;
        if !meta.is_dir() || meta.file_type().is_symlink() {
            return Err(fail("Julkaisun hyötytiedostohakemisto puuttuu"));
        }
        let files = payload_files(&payload)?;
        // Compare by file identity, not platform-specific filesystem enumeration order.
        let expected = desc["files"].as_array().ok_or("Tiedostoluettelo puuttuu")?;
        let mut expected_sorted = expected.clone();
        expected_sorted.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        if files != expected_sorted { return Err(fail("Alustan hyötytiedostot ovat muuttuneet tarkistussumman laskemisen jälkeen")); }
        let suffix = if target == WINDOWS { ".exe" } else { "" };
        let names: BTreeSet<_> = files.iter().filter_map(|file| file["name"].as_str()).collect();
        if !BINARIES.iter().all(|binary| names.contains(format!("{binary}{suffix}").as_str())) ||
            !["LICENSE", "THIRD_PARTY.md", "dependency-licenses/index.json"].iter().all(|n| names.contains(n)) {
            return Err(fail("Julkaisusta puuttuu suoritettava tiedosto tai lisenssi"));
        }
        check_license_bytes(&load(&payload.join("LICENSE"))?)?;
        targets.insert(target.to_owned(), json!({"asset": desc["asset"], "files": files}));
        paths.insert(target.to_owned(), payload);
    }
    if paths.len() != 2 || !paths.contains_key(LINUX) || !paths.contains_key(WINDOWS) {
        return Err(fail("Julkaisu edellyttää sekä Linuxin että Windowsin valmista pakettia"));
    }
    Ok((json!({"schema": 1, "version": version, "license": PROJECT_LICENSE, "targets": targets}), paths))
}
fn parse_key(private_pem: &str, public_hex: &str) -> Result<SigningKey> {
    let key = SigningKey::from_pkcs8_pem(private_pem.trim()).map_err(|_| "Virheellinen Ed25519-allekirjoitusavain")?;
    let decoded = public_hex.trim();
    if decoded.len() != 64 || !decoded.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(fail("Julkaisun julkinen Ed25519-avain on virheellinen"));
    }
    let mut public = [0u8; 32];
    for (i, byte) in public.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&decoded[2 * i..2 * i + 2], 16).map_err(|e| e.to_string())?;
    }
    if key.verifying_key().to_bytes() != public {
        return Err(fail("Allekirjoitusavaimen julkinen osa ei vastaa ohjelmaan kiinnitettyä avainta"));
    }
    Ok(key)
}
fn archive_mode(target: &str, name: &str) -> u32 {
    if target == LINUX && BINARIES.contains(&name) { 0o755 } else { 0o644 }
}
fn write_archive(archive: &Path, target: &str, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    let output = fs::OpenOptions::new().write(true).create_new(true).open(archive).map_err(io_error)?;
    if target == LINUX {
        let gzip = GzBuilder::new().mtime(0).write(output, Compression::best());
        let mut tar = tar::Builder::new(gzip);
        for (name, bytes) in files {
            let mut header = tar::Header::new_ustar();
            header.set_size(bytes.len() as u64);
            header.set_mode(archive_mode(target, name));
            header.set_mtime(0);
            header.set_uid(0);
            header.set_gid(0);
            header.set_cksum();
            tar.append_data(&mut header, name, &bytes[..]).map_err(io_error)?;
        }
        let gzip = tar.into_inner().map_err(io_error)?;
        gzip.finish().map_err(io_error)?;
    } else {
        let mut zip = zip::ZipWriter::new(output);
        for (name, bytes) in files {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .last_modified_time(zip::DateTime::default())
                .unix_permissions(archive_mode(target, name));
            zip.start_file(name, options).map_err(|e| e.to_string())?;
            zip.write_all(bytes).map_err(io_error)?;
        }
        zip.finish().map_err(|e| e.to_string())?;
    }
    if fs::metadata(archive).map_err(io_error)?.len() > MAX_ARCHIVE {
        return Err(fail("Julkaisuarkisto ylittää päivittäjän kokorajan"));
    }
    Ok(())
}
fn sign(root: &Path, input: &Path, output: &Path, public_key_file: &Path) -> Result<Value> {
    check_license(root)?;
    let version = project_version(root)?;
    let (manifest, payload_paths) = payloads(input, &version)?;
    let raw = pretty_json(&manifest)?;
    if raw.len() > MAX_MANIFEST { return Err(fail("Julkaisuluettelo ylittää päivittäjän kokorajan")); }
    if output.exists() { return Err(fail("Allekirjoitettua pakettia ei ylikirjoiteta")); }
    let key_pem = env::var("AS2026_RELEASE_SIGNING_KEY")
        .map_err(|_| "AS2026_RELEASE_SIGNING_KEY-salaisuutta ei ole asetettu")?;
    let public_hex = read_text(public_key_file)?;
    let key = parse_key(&key_pem, &public_hex)?;
    let signature: Signature = key.sign(&raw);
    key.verifying_key().verify(&raw, &signature).map_err(|_| "Ed25519-allekirjoituksen tarkistus epäonnistui")?;
    fs::create_dir_all(output).map_err(io_error)?;
    let result = (|| {
        write_file(&output.join("release.json"), &raw)?;
        write_file(&output.join("release.sig"), signature.to_bytes())?;
        for (target, dir) in &payload_paths {
            let mut files = BTreeMap::new();
            for entry in manifest["targets"][target.as_str()]["files"].as_array().ok_or("Manifestin tiedostoluettelo puuttuu")? {
                let name = entry["name"].as_str().ok_or("Tiedostonimi puuttuu")?;
                files.insert(name.to_owned(), load(&dir.join(safe_relative(name)?))?);
            }
            files.insert("release.json".to_owned(), raw.clone());
            files.insert("release.sig".to_owned(), signature.to_bytes().to_vec());
            let filename = manifest["targets"][target.as_str()]["asset"].as_str().ok_or("Arkiston nimi puuttuu")?;
            write_archive(&output.join(filename), target, &files)?;
        }
        Ok(())
    })();
    if result.is_err() { let _ = fs::remove_dir_all(output); }
    result.map(|_| manifest)
}
fn option(args: &[String], name: &str) -> Result<Option<String>> {
    let matches: Vec<_> = args.windows(2).filter(|window| window[0] == name)
        .map(|window| window[1].clone()).collect();
    if matches.len() > 1 { return Err(fail(format!("Toistuva argumentti {name}"))); }
    Ok(matches.into_iter().next())
}
fn required(args: &[String], name: &str) -> Result<String> {
    option(args, name)?.ok_or_else(|| fail(format!("Argumentti {name} puuttuu")))
}
fn run(args: &[String]) -> Result<()> {
    let root = root();
    match args.first().map(String::as_str) {
        Some("check-license") => {
            check_license(&root)?;
            println!("AS2026:n täydellinen BSD-2-Clause-lisenssi hyväksytty.");
        }
        Some("verify") => {
            let tag = required(args, "--tag")?;
            let main_ref = option(args, "--main-ref")?.unwrap_or_else(|| "origin/main".to_owned());
            println!("{}", verify_tag(&root, &tag, &main_ref)?);
        }
        Some("prepare") => {
            let target = required(args, "--target")?;
            target_asset(&project_version(&root)?, &target)?;
            let output = PathBuf::from(required(args, "--output")?);
            let binaries = PathBuf::from(option(args, "--binary-dir")?.unwrap_or_else(|| "target/release".to_owned()));
            let metadata = if let Some(path) = option(args, "--metadata")? {
                parse_json(&load(Path::new(&path))?)?
            } else { metadata_json(&root, &target)? };
            let descriptor = prepare(&root, &target, &output, &binaries, &metadata)?;
            println!("{}: {} tiedostoa valmisteltu.", descriptor["asset"],
                descriptor["files"].as_array().map_or(0, Vec::len));
        }
        Some("sign") => {
            let input = PathBuf::from(required(args, "--input")?);
            let output = PathBuf::from(required(args, "--output")?);
            let public = PathBuf::from(option(args, "--public-key")?
                .unwrap_or_else(|| root.join("release-public-key.hex").to_string_lossy().into_owned()));
            let manifest = sign(&root, &input, &output, &public)?;
            println!("AS2026 {}: Linux- ja Windows-paketit allekirjoitettu.", manifest["version"]);
        }
        _ => return Err(fail("Käyttö: as2026-release <check-license|verify|prepare|sign> [--tag/--target/--output/--input ...]")),
    }
    Ok(())
}
fn main() {
    if let Err(message) = run(&env::args().skip(1).collect::<Vec<_>>()) {
        eprintln!("Julkaisutyökalun virhe: {message}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn version_and_paths_reject_unsafe_inputs() {
        for version in ["0.5", "01.5.4", "0.5.4-rc1", "0.5.4+meta", "0.5.4\n"] {
            assert!(canonical_version(version).is_err(), "{version}");
        }
        assert!(canonical_version("0.5.4").is_ok());
        for name in ["", ".", "..", "../evil", "a/../b", "/root", "a//b", "a\\b", "C:/x", "file.", "a\nb"] {
            assert!(safe_relative(name).is_err(), "{name:?}");
        }
        assert!(safe_relative("dependency-licenses/pkg/LICENSE").is_ok());
    }
    #[test]
    fn license_rejects_changes_and_accepts_line_endings() {
        let crlf = LICENSE_TEXT.replace('\n', "\r\n");
        assert!(check_license_bytes(crlf.as_bytes()).is_ok());
        let modified = LICENSE_TEXT.replace("BSD 2-Clause License", "Other License");
        assert!(check_license_bytes(modified.as_bytes()).is_err());
    }
    #[test]
    fn hashes_and_name_collisions_are_checked() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("file"), "contents").unwrap();
        let list = payload_files(temp.path()).unwrap();
        assert_eq!(list[0]["sha256"], digest(b"contents"));
        fs::write(temp.path().join("FILE"), "contents").unwrap();
        if fs::read_dir(temp.path()).unwrap().count() == 2 {
            assert!(payload_files(temp.path()).is_err());
        }
    }
    #[test]
    fn manifest_requires_both_targets_and_unchanged_content() {
        let temp = tempfile::tempdir().unwrap();
        let first = temp.path().join(LINUX);
        fs::create_dir_all(first.join("payload")).unwrap();
        fs::write(first.join("target.json"), "{}").unwrap();
        assert!(payloads(temp.path(), "0.5.4").is_err());
    }
    #[test]
    fn archives_contain_release_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let mut files = BTreeMap::new();
        files.insert("release.json".into(), b"{}\n".to_vec());
        files.insert("release.sig".into(), vec![42; 64]);
        files.insert("as2026-gui".into(), b"binary".to_vec());
        let linux = temp.path().join("a.tar.gz");
        write_archive(&linux, LINUX, &files).unwrap();
        let file = fs::File::open(linux).unwrap();
        let gzip = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(gzip);
        let mut found = BTreeSet::new();
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let path = entry.path().unwrap().to_string_lossy().into_owned();
            if path == "as2026-gui" { assert_eq!(entry.header().mode().unwrap(), 0o755); }
            let mut contents = Vec::new();
            entry.read_to_end(&mut contents).unwrap();
            assert_eq!(contents, files[&path]);
            found.insert(path);
        }
        assert_eq!(found.len(), files.len());
        let windows = temp.path().join("a.zip");
        write_archive(&windows, WINDOWS, &files).unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(windows).unwrap()).unwrap();
        for name in files.keys() {
            let mut data = Vec::new();
            archive.by_name(name).unwrap().read_to_end(&mut data).unwrap();
            assert_eq!(&data, &files[name]);
        }
    }
}
