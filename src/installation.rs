//! Käyttäjäkohtainen, versioitu sovellusasennus. Kilpailuihin tai asetuksiin
//! ei kirjoiteta tässä moduulissa. Pysyvä käynnistin lukee vain current.toml:n.

use crate::updates::{VerifiedBundle, verify_bundle};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Component, Path, PathBuf},
    process::{Child, Command},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

static NEXT_FILE: AtomicU64 = AtomicU64::new(1);
const HEALTH_TIMEOUT: Duration = Duration::from_secs(30);
#[cfg(target_os = "linux")]
const ICON: &str = include_str!("../assets/as2026.svg");
#[cfg(windows)]
const WINDOWS_ICON: &[u8] = include_bytes!("../assets/as2026.ico");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallOutcome {
    pub version: String,
    pub previous_version: Option<String>,
    pub already_installed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Current {
    format: u32,
    active: String,
    previous: Option<String>,
    /// Kesken olevan aktivoinnin vanhan tilan varaversio.
    rollback_previous: Option<String>,
    pending: Option<String>,
}

impl Current {
    fn validate(&self) -> Result<(), String> {
        if self.format != 1 {
            return Err("Sovellusasennuksen versiotietoa ei tunnisteta".into());
        }
        validate_version(&self.active)?;
        if let Some(previous) = &self.previous {
            validate_version(previous)?;
        }
        if let Some(previous) = &self.rollback_previous {
            validate_version(previous)?;
        }
        if let Some(token) = &self.pending {
            validate_token(token)?;
        }
        Ok(())
    }
}

fn io_error(path: &Path, error: &std::io::Error) -> String {
    format!("{}: {}", path.display(), crate::io_message(error))
}

/// Käyttöjärjestelmän pysyvä paikallinen sovelluskansio, erillään kilpailuista.
pub fn install_root() -> Result<PathBuf, String> {
    directories::BaseDirs::new()
        .map(|dirs| dirs.data_local_dir().join("AS2026").join("app"))
        .ok_or_else(|| "Käyttäjän paikallista sovelluskansiota ei löydy".into())
}

fn executable(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

fn validate_version(version: &str) -> Result<(), String> {
    if version.is_empty()
        || version.len() > 80
        || !version.bytes().next().is_some_and(|b| b.is_ascii_digit())
        || !version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-_".contains(&b))
        || version.contains("..")
    {
        return Err("Päivityspaketin versionumero ei kelpaa kansioksi".into());
    }
    Ok(())
}

fn validate_token(token: &str) -> Result<(), String> {
    if token.len() != 48 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Sovelluksen käynnistyskuittauksen tunnus on virheellinen".into());
    }
    Ok(())
}

fn unique_token() -> String {
    // Tunnus erottaa tämän käynnistyksen muista. Aitous perustuu allekirjoitettuun
    // pakettiin ja käyttäjän omaan asennuskansioon, ei tämän tunnuksen salaisuuteen.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!(
        "{nanos:032x}{:08x}{:08x}",
        std::process::id(),
        NEXT_FILE.fetch_add(1, Ordering::Relaxed) as u32
    )
}

fn ensure_directory(path: &Path) -> Result<(), String> {
    match fs::create_dir(path) {
        Ok(()) => {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(path, fs::Permissions::from_mode(0o700))
                    .map_err(|e| io_error(path, &e))?;
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(io_error(path, &error)),
    }
    let metadata = fs::symlink_metadata(path).map_err(|e| io_error(path, &e))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(format!(
            "Asennuskansion pitää olla tavallinen kansio: {}",
            path.display()
        ));
    }
    Ok(())
}

fn prepare_root(root: &Path) -> Result<(), String> {
    let parent = root
        .parent()
        .ok_or_else(|| "Asennuskansion yläkansio puuttuu".to_owned())?;
    if let Some(base) = parent.parent() {
        fs::create_dir_all(base).map_err(|e| io_error(base, &e))?;
    }
    ensure_directory(parent)?;
    ensure_directory(root)?;
    ensure_directory(&root.join("versions"))?;
    ensure_directory(&root.join("health"))
}

fn regular_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| io_error(path, &e))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(format!(
            "Asennustiedoston pitää olla tavallinen tiedosto: {}",
            path.display()
        ));
    }
    Ok(())
}

fn optional_regular_file(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => {
            regular_file(path)?;
            Ok(true)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(io_error(path, &error)),
    }
}

fn installation_lock(root: &Path) -> Result<File, String> {
    let path = root.join("installation.lock");
    optional_regular_file(&path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)
        .map_err(|e| io_error(&path, &e))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => {
            Err("AS2026:n asennus tai päivitys on jo käynnissä. Odota hetki".into())
        }
        Err(TryLockError::Error(error)) => Err(io_error(&path, &error)),
    }
}

fn sync_parent(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    if let Some(parent) = path.parent() {
        File::open(parent)
            .and_then(|file| file.sync_all())
            .map_err(|e| io_error(parent, &e))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(not(windows))]
fn replace(source: &Path, destination: &Path) -> Result<(), String> {
    fs::rename(source, destination).map_err(|e| io_error(destination, &e))
}

#[cfg(windows)]
fn replace(source: &Path, destination: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination_wide: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    // REPLACE_EXISTING | WRITE_THROUGH: current.toml vaihtuu yhdellä operaatiolla.
    if unsafe { MoveFileExW(source.as_ptr(), destination_wide.as_ptr(), 1 | 8) } == 0 {
        return Err(io_error(destination, &std::io::Error::last_os_error()));
    }
    Ok(())
}

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<(), String> {
    optional_regular_file(path)?;
    let temporary = Temporary(path.with_extension(format!("tmp-{}", unique_token())));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary.0)
        .map_err(|e| io_error(&temporary.0, &e))?;
    file.write_all(contents)
        .and_then(|()| file.sync_all())
        .map_err(|e| io_error(&temporary.0, &e))?;
    drop(file);
    replace(&temporary.0, path)?;
    sync_parent(path)
}

fn read_current(root: &Path) -> Result<Option<Current>, String> {
    let path = root.join("current.toml");
    if !optional_regular_file(&path)? {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| io_error(&path, &e))?;
    if text.len() > 4096 {
        return Err("Sovellusasennuksen versiotiedosto on virheellinen".into());
    }
    let current: Current = toml::from_str(&text)
        .map_err(|_| "Sovellusasennuksen versiotiedosto on virheellinen".to_owned())?;
    current.validate()?;
    Ok(Some(current))
}

fn write_current(root: &Path, current: &Current) -> Result<(), String> {
    current.validate()?;
    let text = toml::to_string_pretty(current)
        .map_err(|_| "Sovellusasennuksen versiotiedon muodostaminen epäonnistui".to_owned())?;
    atomic_write(&root.join("current.toml"), text.as_bytes())
}

fn safe_relative(name: &str) -> Result<&Path, String> {
    let path = Path::new(name);
    if name.is_empty()
        || name.contains(['\\', ':'])
        || name
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("Päivityspaketissa on turvaton tiedostopolku".into());
    }
    Ok(path)
}

fn create_parents(root: &Path, relative: &Path) -> Result<(), String> {
    let mut parent = root.to_owned();
    if let Some(parts) = relative.parent() {
        for component in parts.components() {
            parent.push(component);
            ensure_directory(&parent)?;
        }
    }
    Ok(())
}

fn copy_regular(source: &Path, destination: &Path, executable: bool) -> Result<(), String> {
    regular_file(source)?;
    let mut input = File::open(source).map_err(|e| io_error(source, &e))?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(|e| io_error(destination, &e))?;
    std::io::copy(&mut input, &mut output).map_err(|e| io_error(destination, &e))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        output
            .set_permissions(fs::Permissions::from_mode(if executable {
                0o755
            } else {
                0o644
            }))
            .map_err(|e| io_error(destination, &e))?;
    }
    #[cfg(not(unix))]
    let _ = executable;
    output.sync_all().map_err(|e| io_error(destination, &e))?;
    sync_parent(destination)
}

fn same_bundle(first: &VerifiedBundle, second: &VerifiedBundle) -> bool {
    first.version == second.version
        && first.files.len() == second.files.len()
        && first
            .files
            .iter()
            .zip(&second.files)
            .all(|(first, second)| {
                first.name == second.name
                    && first.size == second.size
                    && first.sha256 == second.sha256
            })
}

struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        // Ainoastaan itse luotu keskeneräinen versionkansio poistetaan.
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn stage_bundle(
    root: &Path,
    bundle: &Path,
    verified: &VerifiedBundle,
    verifier: &impl Fn(&Path) -> Result<VerifiedBundle, String>,
) -> Result<PathBuf, String> {
    validate_version(&verified.version)?;
    let destination = root.join("versions").join(&verified.version);
    if destination.exists() || fs::symlink_metadata(&destination).is_ok() {
        ensure_directory(&destination)?;
        let existing = verifier(&destination)?;
        if !same_bundle(&existing, verified) {
            return Err("Samannimisen asennetun version sisältö poikkeaa päivityspaketista".into());
        }
        return Ok(destination);
    }
    let staging = Staging(
        root.join("versions")
            .join(format!(".staging-{}", unique_token())),
    );
    fs::create_dir(&staging.0).map_err(|e| io_error(&staging.0, &e))?;
    let mut paths = HashSet::new();
    for name in verified
        .files
        .iter()
        .map(|file| file.name.as_str())
        .chain(["release.json", "release.sig"])
    {
        let relative = safe_relative(name)?;
        if !paths.insert(relative.to_owned()) {
            return Err("Päivityspaketissa on päällekkäinen tiedostopolku".into());
        }
        create_parents(&staging.0, relative)?;
        let binary = ["as2026", "as2026-gui", "as2026-launcher"]
            .iter()
            .any(|base| name == executable(base));
        copy_regular(&bundle.join(relative), &staging.0.join(relative), binary)?;
    }
    let copied = verifier(&staging.0)?;
    if !same_bundle(&copied, verified) {
        return Err("Päivityspaketti muuttui kopioinnin aikana".into());
    }
    fs::rename(&staging.0, &destination).map_err(|e| io_error(&destination, &e))?;
    sync_parent(&destination)?;
    Ok(destination)
}

fn install_stable_launcher(root: &Path, version: &Path) -> Result<(), String> {
    let stable = root.join(executable("as2026-launcher"));
    if optional_regular_file(&stable)? {
        return Ok(());
    }
    let temporary = Temporary(root.join(format!(".launcher-{}", unique_token())));
    copy_regular(
        &version.join(executable("as2026-launcher")),
        &temporary.0,
        true,
    )?;
    // Julkaise ensi kerran korvaamatta mahdollista toisen asentajan tiedostoa.
    fs::hard_link(&temporary.0, &stable).map_err(|e| io_error(&stable, &e))?;
    sync_parent(&stable)
}

fn spawn_gui(root: &Path, version: &str, token: Option<&str>) -> Result<Child, String> {
    validate_version(version)?;
    let directory = root.join("versions").join(version);
    let metadata = fs::symlink_metadata(&directory).map_err(|e| io_error(&directory, &e))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Asennetun ohjelmaversion kansio on virheellinen".into());
    }
    let installed = verify_bundle(&directory)?;
    if installed.version != version {
        return Err("Asennetun ohjelmaversion tiedot eivät vastaa versiokansiota".into());
    }
    let binary = directory.join(executable("as2026-gui"));
    regular_file(&binary)?;
    let mut command = Command::new(&binary);
    if let Some(token) = token {
        command.args(["--health-check", token]);
    }
    command.spawn().map_err(|e| io_error(&binary, &e))
}

fn remember_failure(root: &Path, message: &str) -> Result<(), String> {
    atomic_write(&root.join("last-update-error.txt"), message.as_bytes())
}

fn restore_previous(root: &Path, current: &Current) -> Result<Current, String> {
    let previous = current.previous.as_ref().ok_or_else(|| {
        "Ensiasennuksen käynnistys ei onnistunut. Suorita asennus uudelleen".to_owned()
    })?;
    let restored = Current {
        format: 1,
        active: previous.clone(),
        previous: current.rollback_previous.clone(),
        rollback_previous: None,
        pending: None,
    };
    write_current(root, &restored)?;
    Ok(restored)
}

fn recover_interrupted(root: &Path, current: Current) -> Result<Current, String> {
    if current.pending.is_none() {
        return Ok(current);
    }
    let restored = restore_previous(root, &current)?;
    remember_failure(
        root,
        "Edellinen päivitys keskeytyi ennen käynnistyskuittausta. Edellinen versio palautettiin.",
    )?;
    Ok(restored)
}

fn require_upgrade(verified: &VerifiedBundle, current: Option<&Current>) -> Result<(), String> {
    let candidate = semver::Version::parse(&verified.version)
        .map_err(|_| "Päivityspaketin versionumero on virheellinen".to_owned())?;
    if let Some(current) = current {
        let active = semver::Version::parse(&current.active)
            .map_err(|_| "Asennetun ohjelman versionumero on virheellinen".to_owned())?;
        if candidate < active {
            return Err("Vanhempaa ohjelmaversiota ei voi asentaa päivityksenä. Aiemman version palautus tehdään erillisellä palautustoiminnolla".into());
        }
    }
    Ok(())
}

fn activate(
    root: &Path,
    verified: &VerifiedBundle,
    current: Option<Current>,
    start: impl FnOnce(&str, &str) -> Result<(), String>,
) -> Result<InstallOutcome, String> {
    require_upgrade(verified, current.as_ref())?;
    if current
        .as_ref()
        .is_some_and(|old| old.active == verified.version && old.pending.is_none())
    {
        return Ok(InstallOutcome {
            version: verified.version.clone(),
            previous_version: current.and_then(|old| old.previous),
            already_installed: true,
        });
    }
    let previous = current.as_ref().map(|old| old.active.clone());
    let token = unique_token();
    let candidate = Current {
        format: 1,
        active: verified.version.clone(),
        previous: previous.clone(),
        rollback_previous: current.as_ref().and_then(|old| old.previous.clone()),
        pending: Some(token.clone()),
    };
    write_current(root, &candidate)?;
    if let Err(error) = start(&candidate.active, &token) {
        let recovery = if previous.is_some() {
            restore_previous(root, &candidate).map(|_| ())
        } else {
            // Säilytä keskeneräisen ensiasennuksen osoitin uuden yrityksen tueksi.
            Ok(())
        };
        let message = match recovery {
            Ok(()) if previous.is_some() => format!(
                "Päivityksen käynnistys epäonnistui. Edellinen versio palautettiin. {error}"
            ),
            Ok(()) => format!("Ensiasennuksen käynnistys epäonnistui. {error}"),
            Err(restore) => format!(
                "Päivityksen käynnistys epäonnistui: {error}. Palautus epäonnistui: {restore}"
            ),
        };
        let _ = remember_failure(root, &message);
        return Err(message);
    }
    write_current(
        root,
        &Current {
            pending: None,
            rollback_previous: None,
            ..candidate
        },
    )?;
    Ok(InstallOutcome {
        version: verified.version.clone(),
        previous_version: previous,
        already_installed: false,
    })
}

fn wait_for_startup(root: &Path, version: &str, token: &str) -> Result<(), String> {
    let mut child = spawn_gui(root, version, Some(token))?;
    let ack = root.join("health").join(format!("{token}.ok"));
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    let result = (|| loop {
        if let Some(status) = child.try_wait().map_err(|e| {
            format!(
                "Käynnistyvän ohjelman tilaa ei voi tarkistaa: {}",
                crate::io_message(&e)
            )
        })? {
            break Err(match status.code() {
                Some(code) => {
                    format!("Ohjelma sulkeutui ennen käynnistyskuittausta (virhekoodi {code})")
                }
                None => "Ohjelman suoritus keskeytyi ennen käynnistyskuittausta".into(),
            });
        }
        if optional_regular_file(&ack)? {
            let contents = fs::read_to_string(&ack).map_err(|e| io_error(&ack, &e))?;
            if contents == token {
                break Ok(());
            }
            break Err("Käynnistyskuittauksen sisältö on virheellinen".into());
        }
        if Instant::now() >= deadline {
            break Err("Uusi ohjelmaversio ei avautunut 30 sekunnissa".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = fs::remove_file(&ack);
    result
}

/// Varmenna allekirjoitettu paketti, asenna uuteen kansioon ja käynnistä GUI.
/// Epäonnistunut käynnistys palauttaa vanhan osoittimen. Suorita helperissä,
/// sillä käynnistyskuittausta voidaan odottaa enintään 30 sekuntia.
pub fn install_bundle(bundle: &Path) -> Result<InstallOutcome, String> {
    install_bundle_inner(bundle, None)
}

/// Päivityshelperin valmiuskuittaus. Tavallinen asennus käyttää install_bundlea.
pub fn install_bundle_ready(bundle: &Path, token: &str) -> Result<InstallOutcome, String> {
    validate_token(token)?;
    install_bundle_inner(bundle, Some(token))
}

fn install_bundle_inner(
    bundle: &Path,
    ready_token: Option<&str>,
) -> Result<InstallOutcome, String> {
    let root = install_root()?;
    prepare_root(&root)?;
    let _lock = installation_lock(&root)?;
    let verified = verify_bundle(bundle)?;
    let old = match read_current(&root)? {
        Some(current) if current.pending.is_some() && current.previous.is_some() => {
            Some(recover_interrupted(&root, current)?)
        }
        // Ensiasennuksen epäonnistuminen ei estä saman paketin uutta yritystä.
        Some(current) if current.pending.is_some() => None,
        other => other,
    };
    require_upgrade(&verified, old.as_ref())?;
    let version = stage_bundle(&root, bundle, &verified, &verify_bundle)?;
    install_stable_launcher(&root, &version)?;
    install_shortcuts(&root)?;
    if let Some(token) = ready_token {
        let ready = root.join("health").join(format!("helper-{token}.ready"));
        let proceed = root.join("health").join(format!("helper-{token}.go"));
        atomic_write(&ready, token.as_bytes())?;
        let deadline = Instant::now() + HEALTH_TIMEOUT;
        loop {
            if optional_regular_file(&proceed)? {
                let contents = fs::read_to_string(&proceed).map_err(|e| io_error(&proceed, &e))?;
                if contents != token {
                    return Err("Päivityshelperin valmiuskuittaus on virheellinen".into());
                }
                break;
            }
            if Instant::now() >= deadline {
                let _ = fs::remove_file(&ready);
                return Err(
                    "Päivityksen käynnistänyt ohjelma ei vahvistanut asennusta ajoissa".into(),
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let _ = fs::remove_file(&ready);
        let _ = fs::remove_file(&proceed);
    }
    let outcome = activate(&root, &verified, old, |version, token| {
        wait_for_startup(&root, version, token)
    });
    match outcome {
        Ok(outcome) => {
            if outcome.already_installed {
                spawn_gui(&root, &outcome.version, None)?;
            }
            Ok(outcome)
        }
        Err(error) => {
            if let Ok(Some(current)) = read_current(&root)
                && current.pending.is_none()
            {
                let _ = spawn_gui(&root, &current.active, None);
            }
            Err(error)
        }
    }
}

/// Käynnistä varmennetun paketin oma helper. GUI saa sulkeutua vasta tämän
/// onnistuttua ja tallennusten valmistuttua. Helper varmentaa paketin uudestaan.
pub fn spawn_installer(bundle: &Path) -> Result<(), String> {
    let bundle = fs::canonicalize(bundle).map_err(|e| io_error(bundle, &e))?;
    verify_bundle(&bundle)?;
    let helper = bundle.join(executable("as2026-launcher"));
    regular_file(&helper)?;
    let root = install_root()?;
    prepare_root(&root)?;
    let token = unique_token();
    let ready = root.join("health").join(format!("helper-{token}.ready"));
    let proceed = root.join("health").join(format!("helper-{token}.go"));
    let mut child = Command::new(&helper)
        .arg("--apply")
        .arg(&bundle)
        .arg("--ready-token")
        .arg(&token)
        .spawn()
        .map_err(|e| io_error(&helper, &e))?;
    let deadline = Instant::now() + HEALTH_TIMEOUT;
    let result = (|| loop {
        if child
            .try_wait()
            .map_err(|e| io_error(&helper, &e))?
            .is_some()
        {
            break Err("Päivityshelperi ei valmistellut asennusta. Nykyinen ohjelma voi jatkaa normaalisti".into());
        }
        if optional_regular_file(&ready)? {
            let contents = fs::read_to_string(&ready).map_err(|e| io_error(&ready, &e))?;
            if contents != token {
                break Err("Päivityshelperin valmiuskuittaus on virheellinen".into());
            }
            atomic_write(&proceed, token.as_bytes())?;
            break Ok(());
        }
        if Instant::now() >= deadline {
            break Err("Päivityshelperin valmistelu ei valmistunut 30 sekunnissa".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
        let _ = fs::remove_file(&ready);
        let _ = fs::remove_file(&proceed);
    }
    result
}

/// Käynnistys vahvistetaan vasta GUI-ikkunan avauduttua. Vain asennetun
/// aktiivisen version GUI saa kuitata oman käynnistysyrityksensä.
pub fn acknowledge_startup(token: &str) -> Result<(), String> {
    validate_token(token)?;
    let root = install_root()?;
    let current =
        read_current(&root)?.ok_or_else(|| "Sovellusasennuksen versiotieto puuttuu".to_owned())?;
    if current.pending.as_deref() != Some(token) {
        return Err("Käynnistyskuittaus ei vastaa odotettua päivitystä".into());
    }
    let expected = root
        .join("versions")
        .join(current.active)
        .join(executable("as2026-gui"));
    let running = std::env::current_exe().map_err(|e| {
        format!(
            "Ohjelman sijaintia ei voi tunnistaa: {}",
            crate::io_message(&e)
        )
    })?;
    let expected = fs::canonicalize(&expected).map_err(|e| io_error(&expected, &e))?;
    let running = fs::canonicalize(&running).map_err(|e| io_error(&running, &e))?;
    if expected != running {
        return Err("Käynnistyskuittaus ei tullut asennetusta ohjelmaversiosta".into());
    }
    ensure_directory(&root.join("health"))?;
    atomic_write(
        &root.join("health").join(format!("{token}.ok")),
        token.as_bytes(),
    )
}

/// Pysyvä käynnistin palauttaa keskeytyneen päivityksen ennen avaamista.
pub fn launch_current() -> Result<(), String> {
    let root = install_root()?;
    prepare_root(&root)?;
    let _lock = installation_lock(&root)?;
    let current = read_current(&root)?.ok_or_else(|| {
        "AS2026:ta ei ole vielä asennettu. Avaa julkaisupaketin asennusohjelma".to_owned()
    })?;
    let current = recover_interrupted(&root, current)?;
    spawn_gui(&root, &current.active, None)?;
    Ok(())
}

/// Käytössä on sovellusasennuksen versio eikä kehitystyöpuun binääri.
pub fn is_managed() -> bool {
    let Ok(root) = install_root() else {
        return false;
    };
    let Ok(Some(current)) = read_current(&root) else {
        return false;
    };
    let Ok(running) = std::env::current_exe().and_then(fs::canonicalize) else {
        return false;
    };
    fs::canonicalize(
        root.join("versions")
            .join(current.active)
            .join(executable("as2026-gui")),
    )
    .is_ok_and(|expected| expected == running)
}

/// Päivityksen palautus säilyttää kilpailutiedot ennallaan.
pub fn rollback_to_previous() -> Result<(), String> {
    let root = install_root()?;
    prepare_root(&root)?;
    let _lock = installation_lock(&root)?;
    let current = read_current(&root)?.ok_or_else(|| "AS2026:n asennusta ei löydy".to_owned())?;
    let restored = restore_previous(&root, &current)?;
    remember_failure(
        &root,
        "Edellinen ohjelmaversio palautettiin käyttäjän pyynnöstä.",
    )?;
    spawn_gui(&root, &restored.active, None)?;
    Ok(())
}

pub fn take_startup_error() -> Result<Option<String>, String> {
    let path = install_root()?.join("last-update-error.txt");
    if !optional_regular_file(&path)? {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).map_err(|e| io_error(&path, &e))?;
    fs::remove_file(&path).map_err(|e| io_error(&path, &e))?;
    sync_parent(&path)?;
    Ok(Some(text))
}

/// Desktop Entry -muodon lainaus (myös prosenttimerkit ovat Exec-kentässä erityisiä).
fn desktop_argument(path: &Path, exec: bool) -> Result<String, String> {
    let text = path
        .to_str()
        .ok_or_else(|| "Pikakuvakkeen polku ei ole kelvollista Unicode-tekstiä".to_owned())?;
    if text.contains(['\n', '\r', '\0']) {
        return Err("Pikakuvakkeen polku sisältää kielletyn merkin".into());
    }
    let mut escaped = String::new();
    for character in text.chars() {
        match character {
            '\\' => escaped.push_str(if exec { "\\\\\\\\" } else { "\\\\" }),
            '"' | '`' | '$' if exec => {
                escaped.push_str("\\\\");
                escaped.push(character);
            }
            '%' if exec => escaped.push_str("%%"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    Ok(if exec {
        format!("\"{escaped}\"")
    } else {
        escaped
    })
}

/// Tuottaa sovellusvalikon pysyvään käynnistimeen osoittavan pikakuvakkeen.
pub fn desktop_entry(launcher: &Path, icon: &Path) -> Result<String, String> {
    if !launcher.is_absolute() || !icon.is_absolute() {
        return Err("Pikakuvake tarvitsee absoluuttiset sovellus- ja kuvakepolut".into());
    }
    Ok(format!(
        "[Desktop Entry]\nType=Application\nName=AS2026\nComment=Autosuunnistuksen kilpailunhallinta ja tuloslaskenta\nExec={}\nIcon={}\nTerminal=false\nCategories=Office;\nStartupNotify=true\nStartupWMClass=as2026-gui\n",
        desktop_argument(launcher, true)?,
        desktop_argument(icon, false)?
    ))
}

#[cfg(target_os = "linux")]
fn install_shortcuts(root: &Path) -> Result<(), String> {
    let base = directories::BaseDirs::new()
        .ok_or_else(|| "Käyttäjän sovelluskansiota ei löydy".to_owned())?;
    let applications = base.data_dir().join("applications");
    let icons = base.data_dir().join("icons/hicolor/scalable/apps");
    fs::create_dir_all(&applications).map_err(|e| io_error(&applications, &e))?;
    fs::create_dir_all(&icons).map_err(|e| io_error(&icons, &e))?;
    let icon = icons.join("as2026.svg");
    atomic_write(&icon, ICON.as_bytes())?;
    let entry = applications.join("fi.as2026.AS2026.desktop");
    let launcher = root.join(executable("as2026-launcher"));
    atomic_write(&entry, desktop_entry(&launcher, &icon)?.as_bytes())
}

#[cfg(windows)]
fn install_shortcuts(root: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    // PowerShellin ohjelma on vakio. Käyttäjän polut välitetään ympäristössä,
    // eivätkä ne päädy suoritettavan skriptin osiksi tai komentotulkkilausekkeiksi.
    let launcher = root.join(executable("as2026-launcher"));
    let icon = root.join("as2026.ico");
    atomic_write(&icon, WINDOWS_ICON)?;
    const SCRIPT: &str = "$ErrorActionPreference='Stop'; $programs=[Environment]::GetFolderPath('Programs'); if([string]::IsNullOrWhiteSpace($programs)){throw 'Programs folder missing'}; [IO.Directory]::CreateDirectory($programs) | Out-Null; $shell=New-Object -ComObject WScript.Shell; $link=$shell.CreateShortcut([IO.Path]::Combine($programs,'AS2026.lnk')); $link.TargetPath=$env:AS2026_SHORTCUT_TARGET; $link.WorkingDirectory=[IO.Path]::GetDirectoryName($env:AS2026_SHORTCUT_TARGET); $link.Description='AS2026 - Autosuunnistuksen kilpailunhallinta'; $link.IconLocation=$env:AS2026_SHORTCUT_ICON+',0'; $link.Save()";
    let powershell = std::env::var_os("SystemRoot")
        .map(PathBuf::from)
        .ok_or_else(|| "Windowsin järjestelmäkansiota ei löydy".to_owned())?
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
    let output = Command::new(&powershell)
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW: ei konsoli-ikkunaa GUI-asennuksessa.
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            SCRIPT,
        ])
        .env("AS2026_SHORTCUT_TARGET", launcher)
        .env("AS2026_SHORTCUT_ICON", icon)
        .output()
        .map_err(|e| io_error(&powershell, &e))?;
    if !output.status.success() {
        return Err("AS2026:n Käynnistä-valikon pikakuvakkeen luominen epäonnistui".into());
    }
    Ok(())
}

#[cfg(not(any(target_os = "linux", windows)))]
fn install_shortcuts(_root: &Path) -> Result<(), String> {
    Err("Sovelluksen asentamista tuetaan Linuxissa ja Windowsissa".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Folder(PathBuf);
    impl Folder {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!("as2026-installation-{}", unique_token()));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Folder {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn bundle(version: &str) -> VerifiedBundle {
        VerifiedBundle {
            version: version.into(),
            files: vec![],
        }
    }

    #[test]
    fn activation_success_keeps_previous_version() {
        let folder = Folder::new();
        let old = Current {
            format: 1,
            active: "0.4.0".into(),
            previous: None,
            rollback_previous: None,
            pending: None,
        };
        write_current(&folder.0, &old).unwrap();
        let outcome = activate(&folder.0, &bundle("0.5.0"), Some(old), |version, token| {
            assert_eq!(version, "0.5.0");
            validate_token(token)?;
            assert!(read_current(&folder.0)?.unwrap().pending.is_some());
            Ok(())
        })
        .unwrap();
        assert_eq!(outcome.previous_version.as_deref(), Some("0.4.0"));
        let current = read_current(&folder.0).unwrap().unwrap();
        assert_eq!(current.active, "0.5.0");
        assert!(current.pending.is_none());
    }

    #[test]
    fn signed_older_version_cannot_activate_or_change_saved_pointer() {
        let folder = Folder::new();
        let current = Current {
            format: 1,
            active: "0.10.0".into(),
            previous: Some("0.9.0".into()),
            rollback_previous: None,
            pending: None,
        };
        write_current(&folder.0, &current).unwrap();
        let before = fs::read(folder.0.join("current.toml")).unwrap();
        let error = activate(&folder.0, &bundle("0.9.9"), Some(current), |_, _| {
            panic!("Vanhempaa versiota ei saa käynnistää päivityksenä");
        })
        .unwrap_err();
        assert!(error.contains("Vanhempaa ohjelmaversiota"));
        assert_eq!(fs::read(folder.0.join("current.toml")).unwrap(), before);
        let current = read_current(&folder.0).unwrap().unwrap();
        assert!(require_upgrade(&bundle("0.11.0"), Some(&current)).is_ok());
        assert!(require_upgrade(&bundle("0.10.0"), Some(&current)).is_ok());
        // Erillinen palautustoiminto käyttää suoraan aiempaa varmennettua versiota.
        assert_eq!(
            restore_previous(&folder.0, &current).unwrap().active,
            "0.9.0"
        );
    }

    #[test]
    fn failed_startup_restores_pointer_and_preserves_unrelated_data() {
        let folder = Folder::new();
        let competition = folder.0.join("kilpailu.toml");
        fs::write(&competition, "alkuperäinen kilpailutieto").unwrap();
        let old = Current {
            format: 1,
            active: "0.4.0".into(),
            previous: None,
            rollback_previous: None,
            pending: None,
        };
        write_current(&folder.0, &old).unwrap();
        assert!(
            activate(&folder.0, &bundle("0.5.0"), Some(old), |_, _| Err(
                "testin käynnistysvirhe".into()
            ))
            .is_err()
        );
        assert_eq!(read_current(&folder.0).unwrap().unwrap().active, "0.4.0");
        assert_eq!(
            fs::read_to_string(competition).unwrap(),
            "alkuperäinen kilpailutieto"
        );
        assert!(
            fs::read_to_string(folder.0.join("last-update-error.txt"))
                .unwrap()
                .contains("palautettiin")
        );
    }

    #[test]
    fn interrupted_update_is_rolled_back_on_next_launch() {
        let folder = Folder::new();
        let current = Current {
            format: 1,
            active: "0.5.0".into(),
            previous: Some("0.4.0".into()),
            rollback_previous: Some("0.3.0".into()),
            pending: Some(unique_token()),
        };
        write_current(&folder.0, &current).unwrap();
        let restored = recover_interrupted(&folder.0, current).unwrap();
        assert_eq!(restored.active, "0.4.0");
        assert!(restored.pending.is_none());
        assert_eq!(restored.previous.as_deref(), Some("0.3.0"));
    }

    #[test]
    fn invalid_copied_bundle_cannot_change_current_version() {
        let folder = Folder::new();
        let root = folder.0.join("app");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("versions")).unwrap();
        let source = folder.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("release.json"), "manifest").unwrap();
        fs::write(source.join("release.sig"), "signature").unwrap();
        let old = Current {
            format: 1,
            active: "0.4.0".into(),
            previous: None,
            rollback_previous: None,
            pending: None,
        };
        write_current(&root, &old).unwrap();
        let result = stage_bundle(&root, &source, &bundle("0.5.0"), &|_| {
            Err("testin vioittunut kopio".into())
        });
        assert!(result.is_err());
        assert!(!root.join("versions/0.5.0").exists());
        assert_eq!(fs::read_dir(root.join("versions")).unwrap().count(), 0);
        assert_eq!(read_current(&root).unwrap().unwrap().active, "0.4.0");
    }

    #[test]
    fn staging_copies_manifest_files_and_nested_notices_without_overwriting() {
        let folder = Folder::new();
        let root = folder.0.join("app");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("versions")).unwrap();
        let source = folder.0.join("source");
        fs::create_dir(&source).unwrap();
        fs::create_dir(source.join("licenses")).unwrap();
        fs::write(source.join("release.json"), "manifest").unwrap();
        fs::write(source.join("release.sig"), "signature").unwrap();
        fs::write(source.join("licenses/notice.txt"), "lisenssi").unwrap();
        let verified = VerifiedBundle {
            version: "0.5.0".into(),
            files: vec![crate::updates::BundleFile {
                name: "licenses/notice.txt".into(),
                size: 8,
                sha256: "testin varmentimen tarkistama sisältö".into(),
            }],
        };
        let verifier = |copied: &Path| {
            assert_eq!(
                fs::read_to_string(copied.join("licenses/notice.txt")).unwrap(),
                "lisenssi"
            );
            assert!(copied.join("release.sig").is_file());
            Ok(verified.clone())
        };
        let version = stage_bundle(&root, &source, &verified, &verifier).unwrap();
        assert_eq!(version, root.join("versions/0.5.0"));
        assert!(source.join("licenses/notice.txt").exists());
        // Toistuva saman paketin asennus käyttää vain varmennettua aiempaa kopiota.
        assert_eq!(
            stage_bundle(&root, &source, &verified, &verifier).unwrap(),
            version
        );
        assert!(
            stage_bundle(&root, &source, &verified, &|_| Err(
                "asennettu kopio muuttui".into()
            ))
            .is_err()
        );
        assert_eq!(
            fs::read_to_string(version.join("licenses/notice.txt")).unwrap(),
            "lisenssi"
        );
    }

    #[test]
    fn installation_lock_refuses_another_writer() {
        let folder = Folder::new();
        let _lock = installation_lock(&folder.0).unwrap();
        assert!(
            installation_lock(&folder.0)
                .unwrap_err()
                .contains("jo käynnissä")
        );
    }

    #[test]
    fn bundle_paths_cannot_escape_version_folder() {
        for name in [
            "../as2026",
            "/as2026",
            "C:\\as2026.exe",
            "licenses/../../as2026",
            "a\\b",
            "a/./b",
            "a//b",
            "",
        ] {
            assert!(safe_relative(name).is_err(), "{name}");
        }
        assert!(safe_relative("licenses/serde.txt").is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_cannot_replace_pointer_or_version_folder() {
        use std::os::unix::fs::symlink;
        let folder = Folder::new();
        let untouched = folder.0.join("outside");
        fs::write(&untouched, "ei muuteta").unwrap();
        symlink(&untouched, folder.0.join("current.toml")).unwrap();
        let current = Current {
            format: 1,
            active: "0.5.0".into(),
            previous: None,
            rollback_previous: None,
            pending: None,
        };
        assert!(write_current(&folder.0, &current).is_err());
        assert_eq!(fs::read_to_string(untouched).unwrap(), "ei muuteta");
    }
}
