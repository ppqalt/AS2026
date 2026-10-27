//! Vain allekirjoitetut GitHub Releases -paketit. Verkkotyö tehdään GUI:n ulkopuolella.
use ed25519_dalek::{Signature, VerifyingKey};
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub const REPOSITORY: &str = "ppqalt/AS2026";
const LATEST: &str = "https://api.github.com/repos/ppqalt/AS2026/releases/latest";
const NO_RELEASES: &str = "GitHubissa ei vielä ole vakaata julkaisua.";
const MAX_MANIFEST: usize = 2 * 1024 * 1024;
const MAX_ARCHIVE: usize = 128 * 1024 * 1024;
const MAX_EXPANDED: u64 = 512 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 8192;
// Mukana myös TAR-otsakkeet, kohdistus, allekirjoitustiedostot ja loppunollat.
const MAX_TAR_STREAM: u64 = MAX_EXPANDED + MAX_MANIFEST as u64 + 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BundleFile {
    pub name: String,
    pub size: u64,
    pub sha256: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub asset: String,
    pub files: Vec<BundleFile>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub schema: u32,
    pub license: String,
    pub version: String,
    pub targets: BTreeMap<String, Target>,
}
#[derive(Debug, Clone)]
pub struct VerifiedBundle {
    pub version: String,
    pub files: Vec<BundleFile>,
}
#[derive(Debug, Clone)]
pub struct Update {
    pub version: String,
    target: Target,
    manifest: Vec<u8>,
    signature: Vec<u8>,
}
#[derive(Debug, Clone)]
pub enum UpdateCheck {
    NoRelease,
    Current,
    Available(Update),
}

pub fn target_name() -> Result<&'static str, String> {
    if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        Ok("x86_64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        Ok("x86_64-pc-windows-msvc")
    } else {
        Err("Tälle käyttöjärjestelmälle tai suoritintyypille ei vielä ole päivityspakettia".into())
    }
}
fn asset_name(version: &str, target: &str) -> String {
    if target == "x86_64-pc-windows-msvc" {
        format!("AS2026-{version}-windows-x86_64.zip")
    } else {
        format!("AS2026-{version}-linux-x86_64.tar.gz")
    }
}
fn stable_version(value: &str) -> Result<Version, String> {
    let version = Version::parse(value).map_err(|_| "Julkaisun versionumero on virheellinen")?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err("Päivityksen pitää olla vakaa julkaisu".into());
    }
    Ok(version)
}
fn public_key() -> Result<VerifyingKey, String> {
    let key: [u8; 32] = hex_bytes(include_str!("../release-public-key.hex").trim())?
        .try_into()
        .map_err(|_| "Julkaisun varmennusavain on virheellinen")?;
    VerifyingKey::from_bytes(&key).map_err(|_| "Julkaisun varmennusavain on virheellinen".into())
}
fn hex_bytes(value: &str) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) || !value.is_ascii() {
        return Err("Tarkistussumman muoto on virheellinen".into());
    }
    (0..value.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&value[i..i + 2], 16)
                .map_err(|_| "Tarkistussumman muoto on virheellinen".into())
        })
        .collect()
}
pub fn safe_relative(name: &str) -> Result<PathBuf, String> {
    if name.is_empty()
        || name.len() > 512
        || name.contains(['\\', ':'])
        || name.chars().any(char::is_control)
        || name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.ends_with(['.', ' ']))
    {
        return Err("Päivityspaketissa on turvaton tiedostonimi".into());
    }
    let path = Path::new(name);
    if path.is_absolute()
        || path
            .components()
            .any(|p| !matches!(p, Component::Normal(_)))
    {
        return Err("Päivityspaketin tiedostopolku ei ole sallittu".into());
    }
    Ok(path.to_owned())
}
fn signed_manifest(
    raw: &[u8],
    signature: &[u8],
    key: &VerifyingKey,
) -> Result<(Manifest, Target), String> {
    if raw.len() > MAX_MANIFEST {
        return Err("Julkaisuluettelo on liian suuri".into());
    }
    let signature =
        Signature::from_slice(signature).map_err(|_| "Julkaisun allekirjoitus on virheellinen")?;
    key.verify_strict(raw, &signature)
        .map_err(|_| "Julkaisun aitoutta ei voitu varmistaa. Päivitystä ei asenneta")?;
    let manifest: Manifest = serde_json::from_slice(raw)
        .map_err(|_| "Allekirjoitetun julkaisuluettelon sisältö on virheellinen")?;
    if manifest.license != "BSD-2-Clause" {
        return Err("Julkaisun projektilisenssi ei vastaa BSD-2-Clause-lisenssiä".into());
    }
    if manifest.schema != 1 {
        return Err("Päivityspaketin versiota ei tueta".into());
    }
    stable_version(&manifest.version)?;
    let target = manifest
        .targets
        .get(target_name()?)
        .ok_or("Julkaisusta puuttuu tämän tietokoneen päivityspaketti")?
        .clone();
    if target.asset != asset_name(&manifest.version, target_name()?) || target.files.len() > 4096 {
        return Err("Julkaisupaketin tiedot ovat virheelliset".into());
    }
    let mut names = BTreeSet::new();
    let mut size = 0u64;
    for file in &target.files {
        safe_relative(&file.name)?;
        if file.name == "release.json"
            || file.name == "release.sig"
            || !names.insert(file.name.to_lowercase())
            || hex_bytes(&file.sha256)?.len() != 32
        {
            return Err("Julkaisupaketin tiedostoluettelo on ristiriitainen".into());
        }
        size = size
            .checked_add(file.size)
            .ok_or("Julkaisupaketti on liian suuri")?;
    }
    if size > MAX_EXPANDED {
        return Err("Julkaisupaketti on liian suuri".into());
    }
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    for base in ["as2026-gui", "as2026", "as2026-launcher"] {
        if !names.contains(&format!("{base}{suffix}")) {
            return Err("Julkaisupaketista puuttuu ohjelmatiedosto".into());
        }
    }
    Ok((manifest, target))
}
fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| crate::io_message(&e))?;
    if !metadata.is_file() || metadata.len() > limit as u64 {
        return Err("Päivityspaketin tiedosto on virheellinen tai liian suuri".into());
    }
    let mut data = Vec::new();
    fs::File::open(path)
        .map_err(|e| crate::io_message(&e))?
        .take(limit as u64 + 1)
        .read_to_end(&mut data)
        .map_err(|e| crate::io_message(&e))?;
    if data.len() > limit {
        return Err("Päivityspaketin tiedosto on liian suuri".into());
    }
    Ok(data)
}
pub fn verify_bundle(path: &Path) -> Result<VerifiedBundle, String> {
    verify_bundle_key(path, &public_key()?)
}
fn verify_bundle_key(path: &Path, key: &VerifyingKey) -> Result<VerifiedBundle, String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| crate::io_message(&e))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Päivityskansion pitää olla tavallinen kansio".into());
    }
    let raw = read_bounded(&path.join("release.json"), MAX_MANIFEST)?;
    let signature = read_bounded(&path.join("release.sig"), 64)?;
    let (manifest, target) = signed_manifest(&raw, &signature, key)?;
    let mut allowed: BTreeSet<String> = target.files.iter().map(|f| f.name.clone()).collect();
    allowed.extend(["release.json".into(), "release.sig".into()]);
    check_tree(path, path, &allowed)?;
    for file in &target.files {
        let location = path.join(safe_relative(&file.name)?);
        let metadata = fs::symlink_metadata(&location).map_err(|e| crate::io_message(&e))?;
        if !metadata.is_file() || metadata.len() != file.size {
            return Err(format!(
                "Päivitystiedosto puuttuu tai sen koko on väärä: {}",
                file.name
            ));
        }
        let mut input = fs::File::open(location).map_err(|e| crate::io_message(&e))?;
        let mut hash = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let count = input.read(&mut buffer).map_err(|e| crate::io_message(&e))?;
            if count == 0 {
                break;
            }
            hash.update(&buffer[..count]);
        }
        if format!("{:x}", hash.finalize()) != file.sha256 {
            return Err(format!(
                "Latauksen eheystarkistus epäonnistui: {}",
                file.name
            ));
        }
    }
    Ok(VerifiedBundle {
        version: manifest.version,
        files: target.files,
    })
}
fn check_tree(root: &Path, path: &Path, allowed: &BTreeSet<String>) -> Result<(), String> {
    for entry in fs::read_dir(path).map_err(|e| crate::io_message(&e))? {
        let entry = entry.map_err(|e| crate::io_message(&e))?;
        let metadata = fs::symlink_metadata(entry.path()).map_err(|e| crate::io_message(&e))?;
        // Normalisoi vain alustan erottimet; häviöllinen teksti voisi saada
        // ylimääräisen tiedoston näyttämään allekirjoitetulta polulta.
        let entry_path = entry.path();
        let relative = entry_path
            .strip_prefix(root)
            .map_err(|_| "Päivityspaketin polku on virheellinen")?
            .components()
            .map(|component| component.as_os_str().to_str())
            .collect::<Option<Vec<_>>>()
            .ok_or("Päivityspaketin tiedostonimi ei ole kelvollista Unicode-tekstiä")?
            .join("/");
        safe_relative(&relative)?;
        if metadata.file_type().is_symlink() {
            return Err("Päivityspaketti ei saa sisältää symbolisia linkkejä".into());
        }
        if metadata.is_dir() {
            if !allowed
                .iter()
                .any(|name| name.starts_with(&format!("{relative}/")))
            {
                return Err("Päivityspaketissa on ylimääräinen kansio".into());
            }
            check_tree(root, &entry.path(), allowed)?;
        } else if !metadata.is_file() || !allowed.contains(&relative) {
            return Err("Päivityspaketissa on ylimääräinen tiedosto".into());
        }
    }
    Ok(())
}

/// Pieni vaihdettava verkkorajapinta laitteettomia virhe- ja regressiotestejä varten.
pub trait Fetcher {
    fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String>;
}
pub struct GithubClient(reqwest::blocking::Client);
impl GithubClient {
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(concat!("AS2026/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5
                    || attempt.url().scheme() != "https"
                    || !matches!(
                        attempt.url().host_str(),
                        Some(
                            "github.com"
                                | "api.github.com"
                                | "release-assets.githubusercontent.com"
                                | "objects.githubusercontent.com"
                        )
                    )
                {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|_| "Suojatun verkkoyhteyden valmistelu epäonnistui")?;
        Ok(Self(client))
    }
}
impl Fetcher for GithubClient {
    fn get(&self, url: &str, limit: usize) -> Result<Vec<u8>, String> {
        let response = self
            .0
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .send()
            .map_err(|e| {
                if e.is_timeout() {
                    "Verkkoyhteys aikakatkaistiin. Yritä myöhemmin uudelleen".to_owned()
                } else {
                    "GitHubiin ei saada suojattua yhteyttä. Tarkista internetyhteys".to_owned()
                }
            })?;
        if url == LATEST && response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(NO_RELEASES.into());
        }
        if !response.status().is_success() {
            return Err(format!(
                "GitHub vastasi virheellä {}. Päivityksiä ei voitu tarkistaa tai ladata",
                response.status().as_u16()
            ));
        }
        if response
            .content_length()
            .is_some_and(|size| size > limit as u64)
        {
            return Err("Ladattava tiedosto on liian suuri".into());
        }
        let mut data = Vec::new();
        response
            .take(limit as u64 + 1)
            .read_to_end(&mut data)
            .map_err(|_| "Päivityksen lataus keskeytyi. Aiempi versio säilyy")?;
        if data.len() > limit {
            return Err("Ladattava tiedosto on liian suuri".into());
        }
        Ok(data)
    }
}
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
}
fn download_url(version: &str, name: &str) -> String {
    format!("https://github.com/{REPOSITORY}/releases/download/v{version}/{name}")
}
pub fn check() -> Result<UpdateCheck, String> {
    check_with(
        &GithubClient::new()?,
        env!("CARGO_PKG_VERSION"),
        &public_key()?,
    )
}
fn check_with(
    fetcher: &impl Fetcher,
    current: &str,
    key: &VerifyingKey,
) -> Result<UpdateCheck, String> {
    let raw = match fetcher.get(LATEST, MAX_MANIFEST) {
        Ok(raw) => raw,
        Err(error) if error == NO_RELEASES => return Ok(UpdateCheck::NoRelease),
        Err(error) => return Err(error),
    };
    let release: GithubRelease =
        serde_json::from_slice(&raw).map_err(|_| "GitHubin julkaisutietoja ei voitu lukea")?;
    if release.draft || release.prerelease {
        return Err("GitHub ei palauttanut vakaata julkaisua".into());
    }
    let version = release
        .tag_name
        .strip_prefix('v')
        .ok_or("Julkaisun tunniste on virheellinen")?;
    if stable_version(version)? <= stable_version(current)? {
        return Ok(UpdateCheck::Current);
    }
    let raw = fetcher.get(&download_url(version, "release.json"), MAX_MANIFEST)?;
    let signature = fetcher.get(&download_url(version, "release.sig"), 64)?;
    let (manifest, target) = signed_manifest(&raw, &signature, key)?;
    if manifest.version != version {
        return Err("Julkaisutunniste ja allekirjoitettu versio eivät vastaa toisiaan".into());
    }
    Ok(UpdateCheck::Available(Update {
        version: version.to_owned(),
        target,
        manifest: raw,
        signature,
    }))
}
pub fn cache_dir() -> Result<PathBuf, String> {
    directories::ProjectDirs::from("fi", "AS2026", "AS2026")
        .map(|d| d.cache_dir().join("updates"))
        .ok_or_else(|| "Päivitysten välimuistikansiota ei löydy".into())
}
pub fn download(update: &Update) -> Result<PathBuf, String> {
    download_with(&GithubClient::new()?, update, &cache_dir()?, &public_key()?)
}
fn download_with(
    fetcher: &impl Fetcher,
    update: &Update,
    cache: &Path,
    key: &VerifyingKey,
) -> Result<PathBuf, String> {
    let (manifest, target) = signed_manifest(&update.manifest, &update.signature, key)?;
    if manifest.version != update.version || target.asset != update.target.asset {
        return Err("Päivityksen tiedot muuttuivat. Tarkista päivitykset uudelleen".into());
    }
    let archive = fetcher.get(&download_url(&update.version, &target.asset), MAX_ARCHIVE)?;
    fs::create_dir_all(cache).map_err(|e| crate::io_message(&e))?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "Järjestelmän kello on virheellinen")?
        .as_nanos();
    let destination = cache.join(format!("{}-{}-{stamp}", update.version, std::process::id()));
    fs::create_dir(&destination).map_err(|e| crate::io_message(&e))?;
    let result = (|| {
        fs::write(destination.join("release.json"), &update.manifest)
            .map_err(|e| crate::io_message(&e))?;
        fs::write(destination.join("release.sig"), &update.signature)
            .map_err(|e| crate::io_message(&e))?;
        extract(
            &archive,
            &target,
            &update.manifest,
            &update.signature,
            &destination,
        )?;
        verify_bundle_key(&destination, key)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            for name in ["as2026-gui", "as2026", "as2026-launcher"] {
                fs::set_permissions(destination.join(name), fs::Permissions::from_mode(0o755))
                    .map_err(|e| crate::io_message(&e))?;
            }
        }
        Ok(destination.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&destination);
    }
    result
}
/// Laskuri rajoittaa myös arkistometadatan ja ohitettavien tavujen purkamisen.
/// Pelkkä manifestin kokoraja ei suojaa purkukirjaston sisäisiä lukuja.
struct BoundedReader<R> {
    inner: R,
    remaining: u64,
}
impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        if self.remaining == 0 {
            let mut extra = [0u8; 1];
            if self.inner.read(&mut extra)? == 0 {
                return Ok(0);
            }
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Päivityspaketin purkukokoraja ylittyi",
            ));
        }
        let limit = buffer
            .len()
            .min(self.remaining.min(usize::MAX as u64) as usize);
        let read = self.inner.read(&mut buffer[..limit])?;
        self.remaining -= read as u64;
        Ok(read)
    }
}

struct ZipEntry {
    name: String,
    size: u64,
    compressed_size: u64,
    crc32: u32,
    offset: u64,
}
fn little16(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn little32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ])
}

/// Omat paketit ovat ZIP32-paketteja ilman arkistokommenttia. Keskushakemisto
/// tarkastetaan rajatusta tavusiivusta: ZipArchive::new ei saa allokoida muistia
/// hyökkääjän EOCD/ZIP64-tietueeseen kirjoittaman tiedostomäärän perusteella.
fn check_zip_directory(archive: &[u8], start: usize, entries: &[ZipEntry]) -> Result<(), String> {
    let malformed = || "Päivityksen ZIP-paketin hakemisto on virheellinen".to_owned();
    let mut position = start;
    for entry in entries {
        let header = archive
            .get(position..position.checked_add(46).ok_or_else(malformed)?)
            .ok_or_else(malformed)?;
        if &header[..4] != b"PK\x01\x02" {
            return Err(malformed());
        }
        let name_len = usize::from(little16(header, 28));
        let extra_len = usize::from(little16(header, 30));
        let comment_len = usize::from(little16(header, 32));
        let end = position
            .checked_add(46 + name_len + extra_len + comment_len)
            .ok_or_else(malformed)?;
        let record = archive.get(position..end).ok_or_else(malformed)?;
        let name = std::str::from_utf8(&record[46..46 + name_len]).map_err(|_| malformed())?;
        if name != entry.name
            || u64::from(little32(header, 20)) != entry.compressed_size
            || u64::from(little32(header, 24)) != entry.size
            || little32(header, 16) != entry.crc32
            || u64::from(little32(header, 42)) != entry.offset
            || little16(header, 34) != 0
        {
            return Err(malformed());
        }
        let mode = little32(header, 38) >> 16;
        let kind = mode & 0o170000;
        // Linkkejä ei luoda edes paikallisotsakkeen perusteella. Myös niiden
        // keskushakemistomerkintä hylätään, samoin muut erikoistiedostot.
        if kind != 0 && kind != 0o100000 && !(kind == 0o040000 && name.ends_with('/')) {
            return Err("Päivityspaketti ei saa sisältää linkkejä tai erikoistiedostoja".into());
        }
        if name.ends_with('/') && entry.size != 0 {
            return Err(malformed());
        }
        position = end;
    }
    let footer = archive.get(position..).ok_or_else(malformed)?;
    if footer.len() != 22
        || &footer[..4] != b"PK\x05\x06"
        || little16(footer, 4) != 0
        || little16(footer, 6) != 0
        || usize::from(little16(footer, 8)) != entries.len()
        || usize::from(little16(footer, 10)) != entries.len()
        || little16(footer, 20) != 0
        || little32(footer, 12) as usize != position - start
        || little32(footer, 16) as usize != start
    {
        return Err(malformed());
    }
    Ok(())
}

fn extract(
    archive: &[u8],
    target: &Target,
    manifest: &[u8],
    signature: &[u8],
    destination: &Path,
) -> Result<(), String> {
    extract_with_limit(
        archive,
        target,
        manifest,
        signature,
        destination,
        MAX_TAR_STREAM,
    )
}
fn extract_with_limit(
    archive: &[u8],
    target: &Target,
    manifest: &[u8],
    signature: &[u8],
    destination: &Path,
    tar_stream_limit: u64,
) -> Result<(), String> {
    if archive.len() > MAX_ARCHIVE {
        return Err("Päivityspaketti on liian suuri".into());
    }
    let mut seen = BTreeSet::new();
    let mut save = |name: &str, input: &mut dyn Read, size: u64| -> Result<(), String> {
        let relative = safe_relative(name)?;
        if !seen.insert(name.to_lowercase()) {
            return Err("Päivityspaketissa on toistuva tiedosto".into());
        }
        let expected = if name == "release.json" {
            Some(manifest)
        } else if name == "release.sig" {
            Some(signature)
        } else {
            None
        };
        let entry = target.files.iter().find(|f| f.name == name);
        let limit = expected
            .map(|v| v.len() as u64)
            .or_else(|| entry.map(|f| f.size))
            .ok_or("Päivityspaketissa on ylimääräinen tiedosto")?;
        if limit != size {
            return Err("Päivityspaketin tiedoston koko ei vastaa luetteloa".into());
        }
        if let Some(expected) = expected {
            let mut bytes = Vec::new();
            input
                .take(limit + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| "Päivityspaketti on vioittunut")?;
            if bytes != expected {
                return Err("Päivityspaketin allekirjoitustiedot muuttuivat".into());
            }
            return Ok(());
        }
        let path = destination.join(relative);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| crate::io_message(&e))?;
        }
        let mut output = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| crate::io_message(&e))?;
        let written = std::io::copy(&mut input.take(limit + 1), &mut output)
            .map_err(|_| "Päivityspaketin purkaminen epäonnistui")?;
        if written != limit {
            return Err("Päivityspaketti loppui kesken".into());
        }
        output.flush().map_err(|e| crate::io_message(&e))?;
        output.sync_all().map_err(|e| crate::io_message(&e))?;
        Ok(())
    };
    if target.asset.ends_with(".zip") {
        let mut input = std::io::Cursor::new(archive);
        let mut entries = Vec::new();
        let central_start = loop {
            let offset = input.position();
            let file = zip::read::read_zipfile_from_stream(&mut input)
                .map_err(|_| "Päivityksen ZIP-paketti on vioittunut")?;
            let Some(mut file) = file else {
                break offset as usize;
            };
            if entries.len() >= MAX_ARCHIVE_ENTRIES {
                return Err("Päivityspaketissa on liikaa tiedostoja".into());
            }
            let name = file.name().to_owned();
            let size = file.size();
            entries.push(ZipEntry {
                name: name.clone(),
                size,
                compressed_size: file.compressed_size(),
                crc32: file.crc32(),
                offset,
            });
            if file.is_dir() {
                safe_relative(name.trim_end_matches('/'))?;
                if size != 0 {
                    return Err("Päivityspaketin kansio ei saa sisältää dataa".into());
                }
            } else {
                save(&name, &mut file, size)?;
            }
        };
        check_zip_directory(archive, central_start, &entries)?;
    } else {
        let decoder = flate2::read::GzDecoder::new(archive);
        let bounded = BoundedReader {
            inner: decoder,
            remaining: tar_stream_limit,
        };
        let mut tar = tar::Archive::new(bounded);
        for (index, entry) in tar
            .entries()
            .map_err(|_| "Päivityksen TAR-paketti on vioittunut")?
            .raw(true)
            .enumerate()
        {
            if index >= MAX_ARCHIVE_ENTRIES {
                return Err("Päivityspaketissa on liikaa tiedostoja".into());
            }
            let mut file = entry
                .map_err(|_| "Päivityksen TAR-paketti on vioittunut tai purkukokoraja ylittyi")?;
            // Raakatietueet: GNU/PAX/sparse-metadatan automaattinen käsittely ei
            // saa tehdä rajattomia lukuja ennen näitä tyyppi- ja kokotarkistuksia.
            let kind = file.header().entry_type();
            if !kind.is_file() && !kind.is_dir() {
                return Err("Päivityspaketti ei saa sisältää linkkejä, erikoistiedostoja tai laajennettua metadataa".into());
            }
            let name = file
                .path()
                .map_err(|_| "Päivityspaketin polku on virheellinen")?
                .to_string_lossy()
                .into_owned();
            let size = file.size();
            if kind.is_dir() {
                safe_relative(name.trim_end_matches('/'))?;
                if size != 0 {
                    return Err("Päivityspaketin kansio ei saa sisältää dataa".into());
                }
            } else {
                save(&name, &mut file, size)?;
            }
        }
        // Varmista gzipin CRC/loppu ja rajoita myös TAR-loppumerkin jälkeinen
        // data. Omat USTAR-paketit sisältävät lopussa vain nollakohdistusta.
        let mut tail = tar.into_inner();
        let mut buffer = [0u8; 8192];
        loop {
            let count = tail
                .read(&mut buffer)
                .map_err(|_| "Päivityksen TAR-paketti on vioittunut tai purkukokoraja ylittyi")?;
            if count == 0 {
                break;
            }
            if buffer[..count].iter().any(|byte| *byte != 0) {
                return Err("Päivityksen TAR-paketin lopussa on ylimääräistä sisältöä".into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    struct Mock(BTreeMap<String, Result<Vec<u8>, String>>);
    impl Fetcher for Mock {
        fn get(&self, url: &str, _: usize) -> Result<Vec<u8>, String> {
            self.0
                .get(url)
                .cloned()
                .unwrap_or_else(|| Err("Verkkoyhteys puuttuu".into()))
        }
    }
    type TestFiles = Vec<(String, Vec<u8>)>;
    type TestPayload = (SigningKey, Vec<u8>, Vec<u8>, TestFiles);
    fn payload() -> TestPayload {
        let key = SigningKey::from_bytes(&[7u8; 32]); // Vain synteettinen testiavain.
        let suffix = if cfg!(windows) { ".exe" } else { "" };
        let files: Vec<_> = ["as2026-gui", "as2026", "as2026-launcher"]
            .iter()
            .map(|n| (format!("{n}{suffix}"), b"synteettinen ohjelma".to_vec()))
            .collect();
        let target = Target {
            asset: asset_name("0.6.0", target_name().unwrap()),
            files: files
                .iter()
                .map(|(n, v)| BundleFile {
                    name: n.clone(),
                    size: v.len() as u64,
                    sha256: format!("{:x}", Sha256::digest(v)),
                })
                .collect(),
        };
        let raw = serde_json::to_vec(&Manifest {
            schema: 1,
            license: "BSD-2-Clause".into(),
            version: "0.6.0".into(),
            targets: BTreeMap::from([(target_name().unwrap().into(), target)]),
        })
        .unwrap();
        let sig = key.sign(&raw).to_bytes().to_vec();
        (key, raw, sig, files)
    }
    #[test]
    fn offline_and_no_new_version_are_distinct() {
        let key = SigningKey::from_bytes(&[7; 32]).verifying_key();
        assert!(
            check_with(&Mock(BTreeMap::new()), "0.5.0", &key)
                .unwrap_err()
                .contains("Verkkoyhteys")
        );
        let mock = Mock(BTreeMap::from([(
            LATEST.into(),
            Ok(br#"{"tag_name":"v0.5.0","draft":false,"prerelease":false}"#.to_vec()),
        )]));
        assert!(matches!(
            check_with(&mock, "0.5.0", &key).unwrap(),
            UpdateCheck::Current
        ));
    }
    #[test]
    fn no_published_release_is_normal_not_a_network_failure() {
        let key = SigningKey::from_bytes(&[7; 32]).verifying_key();
        let mock = Mock(BTreeMap::from([(LATEST.into(), Err(NO_RELEASES.into()))]));
        assert!(matches!(
            check_with(&mock, "0.5.3", &key).unwrap(),
            UpdateCheck::NoRelease
        ));
    }
    #[test]
    fn version_comparison_is_semantic_and_signed_content_is_required() {
        let (key, raw, sig, _) = payload();
        let mut responses = BTreeMap::from([
            (
                LATEST.into(),
                Ok(br#"{"tag_name":"v0.6.0","draft":false,"prerelease":false}"#.to_vec()),
            ),
            (download_url("0.6.0", "release.json"), Ok(raw)),
            (download_url("0.6.0", "release.sig"), Ok(sig)),
        ]);
        assert!(matches!(
            check_with(&Mock(responses.clone()), "0.5.0", &key.verifying_key()).unwrap(),
            UpdateCheck::Available(_)
        ));
        responses.insert(download_url("0.6.0", "release.sig"), Ok(vec![0; 64]));
        assert!(
            check_with(&Mock(responses), "0.5.0", &key.verifying_key())
                .unwrap_err()
                .contains("aitoutta")
        );
        assert!(Version::parse("0.10.0").unwrap() > Version::parse("0.9.0").unwrap());
    }
    #[test]
    fn valid_signature_does_not_allow_old_project_license_in_new_update() {
        let (key, raw, _, _) = payload();
        let mut manifest: Manifest = serde_json::from_slice(&raw).unwrap();
        manifest.license = "0BSD".into();
        let raw = serde_json::to_vec(&manifest).unwrap();
        let signature = key.sign(&raw).to_bytes();
        let error = signed_manifest(&raw, &signature, &key.verifying_key()).unwrap_err();
        assert!(error.contains("BSD-2-Clause"));
    }
    #[test]
    fn signatures_mutation_missing_file_and_extra_file_are_rejected() {
        let (key, raw, sig, files) = payload();
        let dir =
            std::env::temp_dir().join(format!("as2026-signature-test-{}", std::process::id()));
        fs::create_dir(&dir).unwrap();
        fs::write(dir.join("release.json"), &raw).unwrap();
        fs::write(dir.join("release.sig"), sig).unwrap();
        assert!(verify_bundle_key(&dir, &key.verifying_key()).is_err());
        for (name, bytes) in &files {
            fs::write(dir.join(name), bytes).unwrap();
        }
        assert!(verify_bundle_key(&dir, &key.verifying_key()).is_ok());
        fs::write(dir.join(&files[0].0), b"vahingoittunut").unwrap();
        assert!(verify_bundle_key(&dir, &key.verifying_key()).is_err());
        fs::write(dir.join(&files[0].0), &files[0].1).unwrap();
        fs::write(dir.join("vieras.exe"), b"x").unwrap();
        assert!(verify_bundle_key(&dir, &key.verifying_key()).is_err());
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    fn signed_bundle_with_named_file(name: &str) -> (TestDirectory, SigningKey) {
        let (key, raw, _, mut files) = payload();
        let bytes = b"synteettinen lisenssi".to_vec();
        let mut manifest: Manifest = serde_json::from_slice(&raw).unwrap();
        manifest
            .targets
            .get_mut(target_name().unwrap())
            .unwrap()
            .files
            .push(BundleFile {
                name: name.into(),
                size: bytes.len() as u64,
                sha256: format!("{:x}", Sha256::digest(&bytes)),
            });
        files.push((name.into(), bytes));
        let raw = serde_json::to_vec(&manifest).unwrap();
        let directory = TestDirectory::new();
        directory.prepared(&raw, &key.sign(&raw).to_bytes());
        for (name, bytes) in files {
            let path = directory.0.join(name);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        assert!(verify_bundle_key(&directory.0, &key.verifying_key()).is_ok());
        (directory, key)
    }

    #[cfg(unix)]
    #[test]
    fn unsigned_backslash_filename_cannot_alias_a_signed_subdirectory_file() {
        let (directory, key) = signed_bundle_with_named_file("licenses/NOTICE");
        fs::write(directory.0.join("licenses\\NOTICE"), b"allekirjoittamaton").unwrap();
        assert!(verify_bundle_key(&directory.0, &key.verifying_key()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn unsigned_non_utf8_filename_cannot_alias_a_signed_unicode_file() {
        use std::os::unix::ffi::OsStringExt;

        let (directory, key) = signed_bundle_with_named_file("notice\u{fffd}.txt");
        let name = std::ffi::OsString::from_vec(b"notice\xff.txt".to_vec());
        fs::write(directory.0.join(name), b"allekirjoittamaton").unwrap();
        assert!(verify_bundle_key(&directory.0, &key.verifying_key()).is_err());
    }

    #[test]
    fn unsafe_package_paths_are_never_accepted() {
        for bad in [
            "../x",
            "/tmp/x",
            "C:/x",
            "a\\b",
            "a/../../x",
            "a/./b",
            "a//b",
            "x ",
            "x.",
        ] {
            assert!(safe_relative(bad).is_err(), "{bad}");
        }
        assert!(safe_relative("dependency-licenses/example/LICENSE").is_ok());
    }
    #[test]
    fn bad_archive_does_not_change_existing_data() {
        let (key, raw, sig, _) = payload();
        let (_, target) = signed_manifest(&raw, &sig, &key.verifying_key()).unwrap();
        let update = Update {
            version: "0.6.0".into(),
            target: target.clone(),
            manifest: raw,
            signature: sig,
        };
        let cache =
            std::env::temp_dir().join(format!("as2026-download-test-{}", std::process::id()));
        fs::create_dir(&cache).unwrap();
        fs::write(cache.join("kilpailu.toml"), b"vanha kilpailu").unwrap();
        let mock = Mock(BTreeMap::from([(
            download_url("0.6.0", &target.asset),
            Ok(b"viallinen lataus".to_vec()),
        )]));
        assert!(download_with(&mock, &update, &cache, &key.verifying_key()).is_err());
        assert_eq!(
            fs::read(cache.join("kilpailu.toml")).unwrap(),
            b"vanha kilpailu"
        );
        assert_eq!(fs::read_dir(&cache).unwrap().count(), 1);
        fs::remove_dir_all(cache).unwrap();
    }

    static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "as2026-archive-test-{}-{}",
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }
        fn prepared(&self, manifest: &[u8], signature: &[u8]) {
            fs::write(self.0.join("release.json"), manifest).unwrap();
            fs::write(self.0.join("release.sig"), signature).unwrap();
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn archive_files(raw: &[u8], signature: &[u8], files: &TestFiles) -> TestFiles {
        let mut entries = files.clone();
        entries.extend([
            ("release.json".into(), raw.to_vec()),
            ("release.sig".into(), signature.to_vec()),
        ]);
        entries
    }
    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        encoder.write_all(bytes).unwrap();
        encoder.finish().unwrap()
    }
    fn tar_header(name: &str, kind: tar::EntryType, size: u64) -> tar::Header {
        let mut header = tar::Header::new_ustar();
        assert!(name.len() <= 100);
        // Suora nimi mahdollistaa myös tarkoituksella vaarallisen polun testin.
        header.as_mut_bytes()[..name.len()].copy_from_slice(name.as_bytes());
        header.set_entry_type(kind);
        header.set_size(size);
        header.set_mode(0o644);
        header.set_cksum();
        header
    }
    fn tar_bytes(entries: &[(String, tar::EntryType, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for (name, kind, content) in entries {
            bytes.extend_from_slice(tar_header(name, *kind, content.len() as u64).as_bytes());
            bytes.extend_from_slice(content);
            bytes.resize(bytes.len().next_multiple_of(512), 0);
        }
        bytes.extend_from_slice(&[0; 1024]);
        bytes
    }
    fn tar_archive(files: &TestFiles) -> Vec<u8> {
        gzip(&tar_bytes(
            &files
                .iter()
                .map(|(name, bytes)| (name.clone(), tar::EntryType::Regular, bytes.clone()))
                .collect::<Vec<_>>(),
        ))
    }
    fn zip_archive(files: &TestFiles) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, bytes) in files {
            writer
                .start_file(
                    name,
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Deflated),
                )
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    fn extraction_target(raw: &[u8], signature: &[u8], key: &SigningKey, zip: bool) -> Target {
        let (_, mut target) = signed_manifest(raw, signature, &key.verifying_key()).unwrap();
        target.asset = if zip { "test.zip" } else { "test.tar.gz" }.into();
        target
    }

    #[test]
    fn both_archive_formats_extract_and_verify_the_signed_files() {
        let (key, raw, signature, files) = payload();
        let entries = archive_files(&raw, &signature, &files);
        for is_zip in [false, true] {
            let directory = TestDirectory::new();
            directory.prepared(&raw, &signature);
            let target = extraction_target(&raw, &signature, &key, is_zip);
            let archive = if is_zip {
                zip_archive(&entries)
            } else {
                tar_archive(&entries)
            };
            extract(&archive, &target, &raw, &signature, &directory.0).unwrap();
            assert_eq!(
                verify_bundle_key(&directory.0, &key.verifying_key())
                    .unwrap()
                    .version,
                "0.6.0"
            );
            for (name, bytes) in &files {
                assert_eq!(&fs::read(directory.0.join(name)).unwrap(), bytes);
            }
        }
    }

    #[test]
    fn tar_extensions_and_links_are_rejected_without_reading_their_declared_payload() {
        let (key, raw, signature, _) = payload();
        let target = extraction_target(&raw, &signature, &key, false);
        for kind in [
            tar::EntryType::GNULongName,
            tar::EntryType::GNULongLink,
            tar::EntryType::XHeader,
            tar::EntryType::XGlobalHeader,
            tar::EntryType::GNUSparse,
            tar::EntryType::Symlink,
            tar::EntryType::Link,
        ] {
            let directory = TestDirectory::new();
            let header = tar_header("metadata", kind, MAX_EXPANDED * 4);
            let archive = gzip(header.as_bytes());
            let error = extract(&archive, &target, &raw, &signature, &directory.0).unwrap_err();
            assert!(
                error.contains("linkkejä") || error.contains("metadataa"),
                "{kind:?}: {error}"
            );
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
        }
    }

    #[test]
    fn tar_limit_counts_headers_and_trailing_padding_as_well_as_file_content() {
        let (key, raw, signature, _) = payload();
        let target = extraction_target(&raw, &signature, &key, false);
        let members = vec![
            ("Ensimmäinen".into(), tar::EntryType::Directory, Vec::new()),
            ("Toinen".into(), tar::EntryType::Directory, Vec::new()),
        ];
        let archive = gzip(&tar_bytes(&members));
        let directory = TestDirectory::new();
        let error = extract_with_limit(&archive, &target, &raw, &signature, &directory.0, 1536)
            .unwrap_err();
        assert!(error.contains("purkukokoraja"));
        extract_with_limit(&archive, &target, &raw, &signature, &directory.0, 2048).unwrap();
    }

    #[test]
    fn directories_must_be_empty_and_traversal_never_leaves_the_destination() {
        let (key, raw, signature, _) = payload();
        for (name, kind, bytes) in [
            ("Hakemisto/", tar::EntryType::Directory, b"x".to_vec()),
            ("../outside", tar::EntryType::Regular, b"x".to_vec()),
        ] {
            let directory = TestDirectory::new();
            let archive = gzip(&tar_bytes(&[(name.into(), kind, bytes.clone())]));
            let target = extraction_target(&raw, &signature, &key, false);
            assert!(extract(&archive, &target, &raw, &signature, &directory.0).is_err());
            let archive = zip_archive(&vec![(name.into(), bytes)]);
            let target = extraction_target(&raw, &signature, &key, true);
            assert!(extract(&archive, &target, &raw, &signature, &directory.0).is_err());
            assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 0);
        }
    }

    #[test]
    fn zip_symlink_metadata_is_rejected_instead_of_materializing_a_link() {
        let (key, raw, signature, files) = payload();
        let directory = TestDirectory::new();
        let target = extraction_target(&raw, &signature, &key, true);
        let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        writer
            .add_symlink(
                &files[0].0,
                String::from_utf8(files[0].1.clone()).unwrap(),
                zip::write::SimpleFileOptions::default(),
            )
            .unwrap();
        for (name, bytes) in &files[1..] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        let archive = writer.finish().unwrap().into_inner();
        assert!(
            extract(&archive, &target, &raw, &signature, &directory.0)
                .unwrap_err()
                .contains("linkkejä")
        );
        assert!(
            !fs::symlink_metadata(directory.0.join(&files[0].0))
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn malformed_zip_and_forged_directory_counts_are_rejected_before_count_allocations() {
        let (key, raw, signature, files) = payload();
        let target = extraction_target(&raw, &signature, &key, true);
        let archive = zip_archive(&files);
        let footer = archive.len() - 22;
        let mut forged = archive.clone();
        forged[footer + 8..footer + 12].fill(0xff); // ZIP64 entrymäärä ilman aitoa rakennetta.
        let mut truncated = archive.clone();
        truncated.pop();
        let mut trailing = archive;
        trailing.push(0);
        for invalid in [b"viallinen ZIP".to_vec(), forged, truncated, trailing] {
            let directory = TestDirectory::new();
            assert!(extract(&invalid, &target, &raw, &signature, &directory.0).is_err());
        }
    }

    #[test]
    fn signed_size_mismatch_and_changed_manifest_bytes_are_rejected_during_extraction() {
        let (key, raw, signature, files) = payload();
        for is_zip in [false, true] {
            let target = extraction_target(&raw, &signature, &key, is_zip);
            let mut wrong_size = files.clone();
            wrong_size[0].1.push(0);
            let mut wrong_manifest = raw.clone();
            wrong_manifest[0] ^= 1;
            for entries in [wrong_size, vec![("release.json".into(), wrong_manifest)]] {
                let directory = TestDirectory::new();
                let archive = if is_zip {
                    zip_archive(&entries)
                } else {
                    tar_archive(&entries)
                };
                assert!(extract(&archive, &target, &raw, &signature, &directory.0).is_err());
            }
        }
    }

    #[test]
    fn corrupt_gzip_footer_and_nonzero_tar_trailing_data_are_not_ignored() {
        let (key, raw, signature, files) = payload();
        let target = extraction_target(&raw, &signature, &key, false);
        let mut corrupt = tar_archive(&files);
        let crc = corrupt.len() - 8;
        corrupt[crc] ^= 1;
        let mut with_trailing = tar_bytes(
            &files
                .iter()
                .map(|(name, bytes)| (name.clone(), tar::EntryType::Regular, bytes.clone()))
                .collect::<Vec<_>>(),
        );
        with_trailing.extend_from_slice(b"piilotettu tietue");
        for archive in [corrupt, gzip(&with_trailing)] {
            let directory = TestDirectory::new();
            assert!(extract(&archive, &target, &raw, &signature, &directory.0).is_err());
        }
    }

    #[test]
    fn changed_signed_payload_and_missing_files_leave_competition_data_untouched() {
        let (key, raw, signature, files) = payload();
        let (_, target) = signed_manifest(&raw, &signature, &key.verifying_key()).unwrap();
        let update = Update {
            version: "0.6.0".into(),
            target: target.clone(),
            manifest: raw.clone(),
            signature: signature.clone(),
        };
        let mut changed = files.clone();
        changed[0].1[0] ^= 1;
        for entries in [changed, files[1..].to_vec()] {
            let cache = TestDirectory::new();
            fs::write(cache.0.join("kilpailu.toml"), b"kilpailu pysyy ennallaan").unwrap();
            let entries = archive_files(&raw, &signature, &entries);
            let archive = if target.asset.ends_with(".zip") {
                zip_archive(&entries)
            } else {
                tar_archive(&entries)
            };
            let mock = Mock(BTreeMap::from([(
                download_url("0.6.0", &target.asset),
                Ok(archive),
            )]));
            assert!(download_with(&mock, &update, &cache.0, &key.verifying_key()).is_err());
            assert_eq!(
                fs::read(cache.0.join("kilpailu.toml")).unwrap(),
                b"kilpailu pysyy ennallaan"
            );
            assert_eq!(fs::read_dir(&cache.0).unwrap().count(), 1);
        }
    }
}
