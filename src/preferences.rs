//! Käyttäjäkohtaiset sovellusasetukset ja pysyvät kilpailukansiot.
//!
//! Asetukset ovat kilpailutiedostoista erillään. Yhden paikallisen kirjoittajan
//! lukko ja sisällön vertailu estävät toisen ikkunan asetusten ylikirjoittamisen.

use crate::{native::Project, workspace::Workspace};
use directories::{ProjectDirs, UserDirs};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions, TryLockError},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    #[serde(
        rename = "kilpailukansio",
        alias = "competition_root",
        skip_serializing_if = "Option::is_none"
    )]
    pub competition_root: Option<PathBuf>,
    #[serde(
        rename = "viimeisin_kilpailu",
        alias = "last_competition",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_competition: Option<PathBuf>,
    /// Tyhjä arvo käyttää käyttöjärjestelmän oletustulostinta.
    pub tulostin: String,
    /// Fluent 2 -betan vapaaehtoiset animaatiot ja tehosteet. Oletus: pois.
    pub effects_enabled: bool,
    /// Paikalliset yhteysasetukset; salasanaa ei koskaan tallenneta.
    pub live_julkaisut: std::collections::BTreeMap<PathBuf, crate::live_publish::Config>,
}

pub struct PreferencesStore {
    path: PathBuf,
    default_root: PathBuf,
    preferences: Preferences,
    expected: Option<Vec<u8>>,
    allow_temporary_root: bool,
}

impl PreferencesStore {
    /// Käyttää käyttöjärjestelmän käyttäjäkohtaisia asetuksia ja Asiakirjat-
    /// kansiota. Ilman Asiakirjat-kansiota käytetään pysyvää sovellusdataa.
    pub fn open() -> Result<Self, String> {
        let project = ProjectDirs::from("fi", "AS2026", "AS2026")
            .ok_or("Käyttäjän pysyvää asetuskansiota ei löydy")?;
        let default_root = UserDirs::new()
            .and_then(|user| user.document_dir().map(Path::to_owned))
            .map(|documents| documents.join("AS2026").join("Kilpailut"))
            .unwrap_or_else(|| project.data_dir().join("Kilpailut"));
        let mut store = Self::open_at(&project.config_dir().join("asetukset.toml"), &default_root)?;
        store.allow_temporary_root = false;
        Ok(store)
    }

    /// Nimenomaiset polut mahdollistavat myös eristetyt testit ilman käyttäjän
    /// omien asetusten tai kilpailujen muuttamista.
    pub fn open_at(path: &Path, default_competition_root: &Path) -> Result<Self, String> {
        let path = resolve_existing_ancestors(path)?;
        let default_root = absolute(default_competition_root)?;
        let expected = read_optional(&path)?;
        let preferences = match &expected {
            Some(bytes) => {
                let source = std::str::from_utf8(bytes)
                    .map_err(|_| "Asetustiedoston pitää käyttää UTF-8-merkistöä".to_owned())?;
                toml::from_str(source).map_err(|error| {
                    let position = error.span().map_or(String::new(), |span| {
                        let line = source[..span.start.min(source.len())]
                            .lines()
                            .count()
                            .max(1);
                        format!(" rivillä {line}")
                    });
                    format!(
                        "Asetustiedoston TOML-rakenne on virheellinen{position}: {}",
                        path.display()
                    )
                })?
            }
            None => Preferences::default(),
        };
        Ok(Self {
            path,
            default_root,
            preferences,
            expected,
            allow_temporary_root: true,
        })
    }

    pub fn get(&self) -> &Preferences {
        &self.preferences
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn save_publish(
        &mut self,
        path: PathBuf,
        mut config: crate::live_publish::Config,
    ) -> Result<(), String> {
        let mut candidate = self.preferences.clone();
        config.password.clear();
        candidate.live_julkaisut.insert(path, config);
        self.save(candidate)
    }

    /// Käyttäjäkohtainen ja kilpailusta riippumaton ulkoasuasetus.
    pub fn save_effects_enabled(&mut self, enabled: bool) -> Result<(), String> {
        let mut candidate = self.preferences.clone();
        candidate.effects_enabled = enabled;
        self.save(candidate)
    }

    pub fn save_printer(&mut self, printer: &str) -> Result<(), String> {
        if printer.contains(['\r', '\n', '\0']) {
            return Err("Tulostimen nimi on virheellinen".into());
        }
        let mut candidate = self.preferences.clone();
        candidate.tulostin = printer.to_owned();
        self.save(candidate)
    }

    /// Palauttaa sijainnin luomatta kilpailua tai siirtämättä vanhoja tietoja.
    pub fn competition_root(&self) -> Result<PathBuf, String> {
        let root = self
            .preferences
            .competition_root
            .as_ref()
            .unwrap_or(&self.default_root);
        let root = absolute(root)?;
        self.ensure_persistent(&root)?;
        Ok(root)
    }

    /// None palauttaa käyttöjärjestelmän mukaisen oletuskansion. Aiemmat
    /// kilpailut pysyvät paikallaan eikä niiden tiedostoja siirretä.
    pub fn save_root(&mut self, root: Option<PathBuf>) -> Result<(), String> {
        let root = root.map(|path| absolute(&path)).transpose()?;
        self.ensure_persistent(root.as_deref().unwrap_or(&self.default_root))?;
        let mut candidate = self.preferences.clone();
        candidate.competition_root = root;
        self.save(candidate)
    }

    /// Kutsuja jättää väliaikaiset esimerkkikilpailut tämän ulkopuolelle.
    /// Olemassaolo tarkistetaan, jotta rikkinäistä kilpailupolkua ei muisteta.
    pub fn remember_competition(&mut self, path: &Path) -> Result<(), String> {
        let canonical = fs::canonicalize(path).map_err(|error| file_error(path, &error))?;
        self.ensure_persistent(&canonical)?;
        Workspace::open(&canonical).map_err(|error| error.to_string())?;
        let mut candidate = self.preferences.clone();
        candidate.last_competition = Some(canonical);
        self.save(candidate)
    }

    /// Luo aina uuden alikansion. create_dir on prosessien välillä atominen,
    /// joten samannimiset tai samanaikaisesti luodut kilpailut eivät korvaudu.
    /// Muistettavan viimeisimmän kilpailun päivitys tehdään erikseen, jotta sen
    /// epäonnistuminen ei piilota jo onnistuneesti luotua kilpailutiedostoa.
    pub fn create_competition(&self, name: &str) -> Result<Workspace, String> {
        self.create_competition_in(name, &self.competition_root()?)
    }

    /// Kilpailukohtainen sijainti ei muuta seuraavien kilpailujen oletusta.
    pub fn create_competition_in(&self, name: &str, root: &Path) -> Result<Workspace, String> {
        let project = Project::new(name).map_err(|error| error.to_string())?;
        self.create_imported_in(project, root)
    }

    pub fn create_imported_in(&self, project: Project, root: &Path) -> Result<Workspace, String> {
        project.validate().map_err(|error| error.to_string())?;
        let name = &project.kilpailu.nimi;
        let root = absolute(root)?;
        self.ensure_persistent(&root)?;
        fs::create_dir_all(&root).map_err(|error| file_error(&root, &error))?;
        let root = fs::canonicalize(&root).map_err(|error| file_error(&root, &error))?;
        self.ensure_persistent(&root)?;
        let base = directory_name(name);
        for index in 1..=10_000 {
            let folder = if index == 1 {
                base.clone()
            } else {
                format!("{base} ({index})")
            };
            let directory = root.join(folder);
            match fs::create_dir(&directory) {
                Ok(()) => {
                    return Workspace::create_imported(&directory, project).map_err(|error| {
                        format!(
                            "Kilpailua ei voitu luoda kansioon {}: {error}",
                            directory.display()
                        )
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(file_error(&directory, &error)),
            }
        }
        Err("Kilpailulle ei löytynyt vapaata kansiota; valitse toinen nimi tai sijainti".into())
    }

    fn ensure_persistent(&self, path: &Path) -> Result<(), String> {
        if self.allow_temporary_root {
            return Ok(());
        }
        let resolved = resolve_existing_ancestors(path)?;
        let temporary_roots = [
            std::env::temp_dir(),
            #[cfg(unix)]
            PathBuf::from("/tmp"),
            #[cfg(unix)]
            PathBuf::from("/var/tmp"),
        ];
        for temporary in temporary_roots {
            if resolved.starts_with(resolve_existing_ancestors(&temporary)?) {
                return Err("Kilpailut tarvitsevat pysyvän tallennuspaikan. Valitse kansio tilapäiskansion ulkopuolelta".into());
            }
        }
        Ok(())
    }

    fn save(&mut self, candidate: Preferences) -> Result<(), String> {
        let content = toml::to_string_pretty(&candidate)
            .map_err(|_| "Asetuksia ei voitu muuntaa TOML-muotoon".to_owned())?
            .into_bytes();
        let parent = self.path.parent().ok_or("Asetuskansion polku puuttuu")?;
        fs::create_dir_all(parent).map_err(|error| file_error(parent, &error))?;
        let lock_path = sidecar(&self.path, ".lukko");
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|error| file_error(&lock_path, &error))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(TryLockError::WouldBlock) => return Err(
                "Toinen paikallinen toiminto tallentaa asetuksia. Yritä uudelleen hetken kuluttua"
                    .into(),
            ),
            Err(TryLockError::Error(error)) => return Err(file_error(&lock_path, &error)),
        }
        self.check_unchanged()?;
        if candidate == self.preferences {
            return Ok(());
        }
        let permissions = if self.expected.is_some() {
            Some(
                fs::metadata(&self.path)
                    .map_err(|error| file_error(&self.path, &error))?
                    .permissions(),
            )
        } else {
            None
        };
        let temp = Temporary::write(&self.path, &content, permissions.clone())?;
        if let Some(previous) = &self.expected {
            let backup_path = sidecar(&self.path, ".bak");
            let backup = Temporary::write(&backup_path, previous, permissions)?;
            self.check_unchanged()?;
            backup.replace(&backup_path)?;
            sync_parent(&self.path)?;
        }
        self.check_unchanged()?;
        if self.expected.is_some() {
            temp.replace(&self.path)?;
        } else {
            // Ensimmäistä asetustiedostoa ei korvata, vaikka muu ohjelma olisi
            // luonut sellaisen viimeisen sisältötarkistuksen jälkeen.
            crate::storage::file_operation(|| fs::hard_link(&temp.path, &self.path))
                .map_err(|error| file_error(&self.path, &error))?;
        }
        sync_parent(&self.path).map_err(|_| "Asetukset kirjoitettiin, mutta tallennuksen pysyvyyttä ei voitu varmistaa. Avaa sovellus uudelleen ja tarkista asetukset".to_owned())?;
        self.preferences = candidate;
        self.expected = Some(content);
        Ok(())
    }

    fn check_unchanged(&self) -> Result<(), String> {
        if read_optional(&self.path)? != self.expected {
            return Err("Asetuksia on muutettu toisessa ikkunassa tai ohjelmassa. Käynnistä sovellus uudelleen ennen asetusten muuttamista".into());
        }
        Ok(())
    }
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if path.as_os_str().is_empty() {
        return Err("Kansiopolku ei voi olla tyhjä".into());
    }
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        std::env::current_dir()
            .map(|directory| directory.join(path))
            .map_err(|error| crate::io_message(&error))
    }
}

// Myös vielä luomattoman kansion olemassa oleva yläpolku voi olla linkki
// tilapäiskansioon. Tarkistus ei luo eikä siirrä tarkistettavia kansioita.
fn resolve_existing_ancestors(path: &Path) -> Result<PathBuf, String> {
    let mut ancestor = absolute(path)?;
    let mut missing = Vec::new();
    loop {
        match fs::canonicalize(&ancestor) {
            Ok(mut resolved) => {
                for component in missing.into_iter().rev() {
                    resolved.push(component);
                }
                return Ok(resolved);
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let name = ancestor
                    .file_name()
                    .ok_or_else(|| file_error(path, &error))?
                    .to_owned();
                missing.push(name);
                if !ancestor.pop() {
                    return Err(file_error(path, &error));
                }
            }
            Err(error) => return Err(file_error(path, &error)),
        }
    }
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>, String> {
    match crate::storage::file_operation(|| fs::read(path)) {
        Ok(content) => Ok(Some(content)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(file_error(path, &error)),
    }
}

fn file_error(path: &Path, error: &io::Error) -> String {
    format!("{}: {}", path.display(), crate::io_message(error))
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Säilyttää luettavan suomalaisen nimen myös Windowsissa. Polkuerottimista ja
/// ohjausmerkeistä tulee välilyöntejä, joten syöte ei koskaan luo alihakemistoja.
fn directory_name(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|character| {
            if character.is_control() || "<>:\"/\\|?*".contains(character) {
                ' '
            } else {
                character
            }
        })
        .collect();
    let compact = safe.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = compact.trim_matches(['.', ' ']);
    let mut result = String::new();
    // Jättää tilaa törmäystunnukselle sekä kilpailutiedoston nimelle. Katkaisu
    // tapahtuu UTF-8-merkin rajalta eikä riko ääkkösiä.
    for character in trimmed.chars() {
        if result.len() + character.len_utf8() > 100 {
            break;
        }
        result.push(character);
    }
    result = result.trim_end_matches(['.', ' ']).to_owned();
    if result.is_empty() {
        result = "Kilpailu".into();
    }
    let stem = result
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end()
        .to_uppercase();
    let reserved = matches!(
        stem.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) || ["COM", "LPT"].iter().any(|prefix| {
        stem.strip_prefix(*prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
            )
        })
    });
    if reserved {
        result.insert(0, '_');
    }
    result
}

struct Temporary {
    path: PathBuf,
}

impl Temporary {
    fn write(
        path: &Path,
        content: &[u8],
        permissions: Option<fs::Permissions>,
    ) -> Result<Self, String> {
        for _ in 0..100 {
            let suffix = format!(
                ".as2026-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            );
            let temp_path = sidecar(path, &suffix);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = match options.open(&temp_path) {
                Ok(file) => file,
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(file_error(&temp_path, &error)),
            };
            let temporary = Self { path: temp_path };
            file.write_all(content)
                .map_err(|error| file_error(&temporary.path, &error))?;
            if let Some(permissions) = permissions {
                file.set_permissions(permissions)
                    .map_err(|error| file_error(&temporary.path, &error))?;
            }
            file.sync_all()
                .map_err(|error| file_error(&temporary.path, &error))?;
            drop(file);
            return Ok(temporary);
        }
        Err("Asetustallennuksen tilapäistiedostolle ei löytynyt vapaata nimeä".into())
    }

    fn replace(&self, destination: &Path) -> Result<(), String> {
        crate::storage::file_operation(|| fs::rename(&self.path, destination))
            .map_err(|error| file_error(destination, &error))
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn sync_parent(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let parent = path.parent().ok_or("Asetuskansion polku puuttuu")?;
        fs::File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| file_error(parent, &error))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_mode_rejects_temporary_roots_before_creation() {
        let root = std::env::temp_dir().join(format!("as2026-no-create-{}", std::process::id()));
        let mut store =
            PreferencesStore::open_at(&root.join("asetukset.toml"), &root.join("Kilpailut"))
                .unwrap();
        store.allow_temporary_root = false;
        assert!(
            store
                .competition_root()
                .unwrap_err()
                .contains("pysyvän tallennuspaikan")
        );
        assert!(store.save_root(Some(root.clone())).is_err());
        assert!(store.create_competition("Ei tilapäiskansioon").is_err());
        assert!(!root.exists());
    }

    #[test]
    fn effects_are_off_by_default_and_persist() {
        let root = std::env::temp_dir().join(format!(
            "as2026-effects-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        let path = root.join("asetukset.toml");
        let default_root = root.join("Kilpailut");
        let mut store = PreferencesStore::open_at(&path, &default_root).unwrap();
        assert!(!store.get().effects_enabled);
        store.save_effects_enabled(true).unwrap();
        drop(store);
        let mut reopened = PreferencesStore::open_at(&path, &default_root).unwrap();
        assert!(reopened.get().effects_enabled);
        reopened.save_effects_enabled(false).unwrap();
        drop(reopened);
        assert!(!PreferencesStore::open_at(&path, &default_root).unwrap().get().effects_enabled);
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn temporary_root_cannot_be_hidden_behind_a_symlink() {
        use std::os::unix::fs::symlink;
        let root = std::env::temp_dir().join(format!(
            "as2026-temp-link-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        let link = root.join("Linkki");
        symlink(std::env::temp_dir(), &link).unwrap();
        let mut store = PreferencesStore::open_at(
            &root.join("asetukset.toml"),
            &link.join("Luomaton kilpailukansio"),
        )
        .unwrap();
        store.allow_temporary_root = false;
        assert!(store.competition_root().is_err());
        assert!(!link.join("Luomaton kilpailukansio").exists());
        fs::remove_file(&link).unwrap();
        fs::remove_dir(&root).unwrap();
    }
}
