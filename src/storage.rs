//! Paikallinen kilpailutietovarasto. Yksi yhteinen malli ja atominen tallennus.
//!
//! Sivutiedoston käyttöjärjestelmälukko suojaa tämän sovelluksen paikallisia
//! kirjoittajia. Avaamisen jälkeen muuttunut sisältö havaitaan myös ulkoisen
//! editorin jäljiltä. LAN-tilassa GUI:n muutokset kulkevat isännän kautta;
//! vain LAN-isäntä kirjoittaa jaettua kilpailutiedostoa paikallisesti.

use crate::{CoreError, native::Project};
use std::{
    fs::{self, File, OpenOptions, TryLockError},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "desktop")]
pub(crate) fn durable_write(path: &Path, content: &[u8]) -> Result<(), CoreError> {
    #[cfg(not(unix))]
    let permissions = fs::metadata(path).ok().map(|m| m.permissions());
    #[cfg(unix)]
    let permissions = {
        use std::os::unix::fs::PermissionsExt;
        Some(fs::Permissions::from_mode(0o600))
    };
    Temporary::write(path, content, permissions)?.replace(path)?;
    sync_parent(path)
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

fn file_error(path: &Path, error: &std::io::Error) -> CoreError {
    invalid(format!("{}: {}", path.display(), crate::io_message(error)))
}

fn sidecar(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

pub(crate) fn lock(path: &Path) -> Result<File, CoreError> {
    let lock_path = sidecar(path, ".lukko");
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| file_error(&lock_path, &e))?;
    match file.try_lock() {
        Ok(()) => Ok(file),
        Err(TryLockError::WouldBlock) => Err(invalid(
            "Toinen paikallinen toiminto tallentaa kilpailua. Yritä uudelleen hetken kuluttua",
        )),
        Err(TryLockError::Error(error)) => Err(file_error(&lock_path, &error)),
    }
}

struct Temporary {
    path: PathBuf,
}

impl Temporary {
    fn write(
        path: &Path,
        content: &[u8],
        permissions: Option<fs::Permissions>,
    ) -> Result<Self, CoreError> {
        // create_new estää myös prosessien välisen tilapäisnimitörmäyksen.
        for _ in 0..100 {
            let suffix = format!(
                ".as2026-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            );
            let temp_path = sidecar(path, &suffix);
            let opened = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_path);
            let mut file = match opened {
                Ok(file) => file,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(file_error(&temp_path, &error)),
            };
            let temp = Self { path: temp_path };
            file.write_all(content)
                .map_err(|e| file_error(&temp.path, &e))?;
            if let Some(permissions) = permissions {
                file.set_permissions(permissions)
                    .map_err(|e| file_error(&temp.path, &e))?;
            }
            file.sync_all().map_err(|e| file_error(&temp.path, &e))?;
            drop(file);
            return Ok(temp);
        }
        Err(invalid(
            "Tallennuksen tilapäistiedostolle ei löytynyt vapaata nimeä",
        ))
    }

    fn replace(&self, destination: &Path) -> Result<(), CoreError> {
        fs::rename(&self.path, destination).map_err(|e| file_error(destination, &e))
    }
}

impl Drop for Temporary {
    fn drop(&mut self) {
        // Vain tämän toiminnon itse luoma tilapäistiedosto voidaan poistaa.
        let _ = fs::remove_file(&self.path);
    }
}

fn sync_parent(path: &Path) -> Result<(), CoreError> {
    #[cfg(unix)]
    {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|e| file_error(parent, &e))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Avattu paikallinen kilpailu ja sen viimeksi onnistuneesti tallennettu sisältö.
/// Muutokset tehdään update-kutsulla: epäonnistuminen ei muuta muistissa olevaa mallia.
pub struct ProjectStore {
    path: PathBuf,
    project: Project,
    expected: Vec<u8>,
    lan_authority: bool,
    #[cfg(feature = "desktop")]
    lan: Option<crate::lan::Client>,
    #[cfg(feature = "desktop")]
    edit_basis: std::cell::RefCell<Option<Project>>,
}

impl ProjectStore {
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let canonical = fs::canonicalize(path).map_err(|e| file_error(path, &e))?;
        let path = canonical.as_path();
        let _lock = lock(path)?;
        let expected = fs::read(path).map_err(|e| file_error(path, &e))?;
        let source = std::str::from_utf8(&expected)
            .map_err(|_| invalid("Kilpailutiedoston pitää käyttää UTF-8-merkistöä"))?;
        let project = Project::from_toml(source)?;
        Ok(Self {
            path: path.to_owned(),
            project,
            expected,
            lan_authority: false,
            #[cfg(feature = "desktop")]
            lan: None,
            #[cfg(feature = "desktop")]
            edit_basis: std::cell::RefCell::new(None),
        })
    }

    /// Luo uuden tiedoston. Olemassa olevaa kilpailua ei korvata.
    pub fn create(path: &Path, project: Project) -> Result<Self, CoreError> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let file_name = path
            .file_name()
            .ok_or_else(|| invalid("Kilpailutiedoston nimi puuttuu"))?;
        let canonical = fs::canonicalize(parent)
            .map_err(|e| file_error(parent, &e))?
            .join(file_name);
        let path = canonical.as_path();
        let expected = project.to_toml()?.into_bytes();
        let _lock = lock(path)?;
        match fs::metadata(path) {
            Ok(_) => {
                return Err(invalid(
                    "Kilpailutiedosto on jo olemassa; valitse toinen kansio",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(file_error(path, &error)),
        }
        let temp = Temporary::write(path, &expected, None)?;
        // hard_link julkaisee valmiin tiedoston atomisesti ja kieltäytyy korvaamasta
        // kohdetta, vaikka toinen ohjelma olisi luonut sen tarkistuksen jälkeen.
        fs::hard_link(&temp.path, path).map_err(|e| file_error(path, &e))?;
        sync_parent(path)?;
        Ok(Self {
            path: path.to_owned(),
            project,
            expected,
            lan_authority: false,
            #[cfg(feature = "desktop")]
            lan: None,
            #[cfg(feature = "desktop")]
            edit_basis: std::cell::RefCell::new(None),
        })
    }

    pub fn project(&self) -> &Project {
        &self.project
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn backup_path(&self) -> PathBuf {
        sidecar(&self.path, ".bak")
    }

    fn check_unchanged(&self) -> Result<(), CoreError> {
        let actual = fs::read(&self.path).map_err(|e| file_error(&self.path, &e))?;
        if actual != self.expected {
            return Err(invalid(
                "Kilpailutiedostoa on muutettu toisessa ohjelmassa tai ikkunassa. Avaa kilpailu uudelleen ennen muuttamista",
            ));
        }
        Ok(())
    }

    /// Rajapinta kilpailijan, luokan, radan, aseman ja Emit-historian muutoksille.
    /// Laskenta muodostetaan aina tästä samasta mallista, jolloin vanhentunutta
    /// erillistä tulosvälimuistia ei jää käytettäväksi muokkauksen jälkeen.
    pub fn update<T>(
        &mut self,
        edit: impl FnOnce(&mut Project) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        let mut candidate = self.project.clone();
        let result = edit(&mut candidate)?;
        #[cfg(feature = "desktop")]
        if let Some(client) = &self.lan {
            let before = self.project.clone();
            self.project =
                client.submit(&self.project, &candidate, self.edit_basis.borrow().as_ref())?;
            crate::lan::advance_basis(
                &before,
                &candidate,
                &self.project,
                &mut self.edit_basis.borrow_mut(),
            )?;
            self.expected = self.project.to_toml()?.into_bytes();
            // Isännän tiedoston kirjoittaa yksin palvelin. Asiakkaan kopio on palautumista varten.
            if !client.is_host()
                && let Err(error) = durable_write(&self.path, &self.expected)
            {
                // Verkkotapahtuma on jo pysyvä. Sitä ei saa esittää uutena tallentamattomana lukuna.
                client.cache_warning(&error);
            }
            return Ok(result);
        }
        let content = candidate.to_toml()?.into_bytes();
        let _lock = lock(&self.path)?;
        let _lan_writer = if self.lan_authority || !sidecar(&self.path, ".lan-lukko").exists() {
            None
        } else {
            let path = sidecar(&self.path, ".lan-lukko");
            let file = OpenOptions::new()
                .create(true)
                .truncate(false)
                .read(true)
                .write(true)
                .open(&path)
                .map_err(|e| file_error(&path, &e))?;
            match file.try_lock() {
                Ok(()) => Some(file),
                Err(TryLockError::WouldBlock) => {
                    return Err(invalid(
                        "Kilpailu on jaettu lähiverkossa. Liity siihen yläpalkin Lähiverkko-painikkeesta ennen muokkaamista.",
                    ));
                }
                Err(TryLockError::Error(e)) => return Err(file_error(&path, &e)),
            }
        };
        self.check_unchanged()?;
        if candidate == self.project {
            return Ok(result);
        }
        let permissions = fs::metadata(&self.path)
            .map_err(|e| file_error(&self.path, &e))?
            .permissions();
        let temp = Temporary::write(&self.path, &content, Some(permissions.clone()))?;
        let backup = Temporary::write(&self.backup_path(), &self.expected, Some(permissions))?;
        self.check_unchanged()?;
        backup.replace(&self.backup_path())?;
        sync_parent(&self.path)?;
        self.check_unchanged()?;
        temp.replace(&self.path)?;
        sync_parent(&self.path).map_err(|_| invalid(
            "Kilpailutiedosto kirjoitettiin, mutta tallennuksen pysyvyyttä ei voitu varmistaa. Avaa kilpailu uudelleen ja tarkista tiedot",
        ))?;
        self.project = candidate;
        self.expected = content;
        Ok(result)
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn enable_lan_authority(&mut self) {
        self.lan_authority = true;
    }
    #[cfg(feature = "desktop")]
    pub fn attach_lan(&mut self, client: crate::lan::Client) {
        self.lan = Some(client);
    }

    #[cfg(feature = "desktop")]
    pub fn lan_client(&self) -> Option<&crate::lan::Client> {
        self.lan.as_ref()
    }

    #[cfg(feature = "desktop")]
    pub(crate) fn take_edit_basis(&self) -> Option<Project> {
        self.edit_basis.take()
    }
    #[cfg(feature = "desktop")]
    pub(crate) fn restore_edit_basis(&self, basis: Option<Project>) {
        self.edit_basis.replace(basis);
    }
    #[cfg(feature = "desktop")]
    pub fn pin_edit_basis(&self) {
        *self.edit_basis.borrow_mut() = Some(self.project.clone());
    }

    #[cfg(feature = "desktop")]
    pub fn poll_lan(&mut self) -> Result<bool, CoreError> {
        let Some(client) = &self.lan else {
            return Ok(false);
        };
        let project = client.snapshot()?;
        if project == self.project {
            return Ok(false);
        }
        let content = project.to_toml()?.into_bytes();
        if !client.is_host() {
            durable_write(&self.path, &content)?;
        }
        self.project = project;
        self.expected = content;
        Ok(true)
    }
}
