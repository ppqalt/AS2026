use as2026::{CoreError, native, storage::ProjectStore};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

static NEXT: AtomicU64 = AtomicU64::new(1);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "as2026-storage-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self) -> PathBuf {
        self.0.join(native::FILE_NAME)
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn project() -> native::Project {
    native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}

#[test]
fn saving_is_immediate_and_backup_is_previous_complete_version() {
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    let original = fs::read(&path).unwrap();
    store
        .update(|p| {
            p.kilpailijat[0].kuljettaja = "Korjattu".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        ProjectStore::open(&path).unwrap().project().kilpailijat[0].kuljettaja,
        "Korjattu"
    );
    assert_eq!(fs::read(store.backup_path()).unwrap(), original);
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 3); // pää, varmuuskopio, pysyvä lukon nimi
}

#[test]
fn stale_store_and_external_edit_are_detected_without_overwrite() {
    let directory = Directory::new();
    let path = directory.file();
    let mut first = ProjectStore::create(&path, project()).unwrap();
    let mut stale = ProjectStore::open(&path).unwrap();
    first
        .update(|p| {
            p.kilpailu.nimi = "Ensimmäinen muutos".into();
            Ok(())
        })
        .unwrap();
    let current = fs::read(&path).unwrap();
    let error = stale
        .update(|p| {
            p.kilpailu.nimi = "Vanhan ikkunan muutos".into();
            Ok(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("Avaa kilpailu uudelleen"));
    assert_eq!(fs::read(&path).unwrap(), current);
    assert_eq!(stale.project().kilpailu.nimi, "Esimerkkikilpailu");
    fs::write(&path, b"ulkoinen muutos").unwrap();
    assert!(
        first
            .update(|p| {
                p.kilpailu.nimi = "Kolmas".into();
                Ok(())
            })
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), b"ulkoinen muutos");
}

#[test]
fn failed_backup_leaves_memory_and_original_intact_and_cleans_temps() {
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    let original = fs::read(&path).unwrap();
    fs::create_dir(store.backup_path()).unwrap(); // Varma kirjoitusvirhe myös pääkäyttäjänä.
    assert!(
        store
            .update(|p| {
                p.kilpailu.nimi = "Ei tallentunut".into();
                Ok(())
            })
            .is_err()
    );
    assert_eq!(store.project().kilpailu.nimi, "Esimerkkikilpailu");
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 3);
}

#[test]
fn invalid_edit_and_edit_error_do_not_change_either_state() {
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(
        store
            .update(|p| {
                p.luokat.clear();
                Ok(())
            })
            .is_err()
    );
    assert!(
        store
            .update::<()>(|p| {
                p.kilpailu.nimi = "Kesken".into();
                Err(CoreError::InvalidInput("Peruutettu".into()))
            })
            .is_err()
    );
    assert_eq!(store.project(), &project());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn operating_system_lock_prevents_other_local_writer() {
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.0.join("kilpailu.toml.lukko"))
        .unwrap();
    lock.try_lock().unwrap();
    let error = store
        .update(|p| {
            p.kilpailu.nimi = "Lukittu".into();
            Ok(())
        })
        .unwrap_err();
    assert!(error.to_string().contains("Toinen paikallinen toiminto"));
    drop(lock);
    store
        .update(|p| {
            p.kilpailu.nimi = "Vapaa".into();
            Ok(())
        })
        .unwrap();
}

#[test]
fn create_never_replaces_existing_competition() {
    let directory = Directory::new();
    let path = directory.file();
    ProjectStore::create(&path, project()).unwrap();
    let original = fs::read(&path).unwrap();
    assert!(ProjectStore::create(&path, project()).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[test]
fn card_history_and_result_survive_reopening_and_failed_read_is_not_saved() {
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    let frame =
        as2026::emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    let clock = as2026::ClockTime::new(12, 11, 0).unwrap();
    store.update(|p| p.record_frame(&frame, clock)).unwrap();
    let opened = ProjectStore::open(&path).unwrap();
    assert_eq!(opened.project().leimaluvut.len(), 2);
    assert_eq!(opened.project(), store.project());
    let report = as2026::application::calculate(&opened.project().to_archive().unwrap()).unwrap();
    assert!(report.competitors[0].calculation.is_ok());
    let original = fs::read(&path).unwrap();
    let mut invalid = frame;
    invalid.records.fill(as2026::Punch::default());
    assert!(store.update(|p| p.record_frame(&invalid, clock)).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
}

#[cfg(unix)]
#[test]
fn symlink_alias_uses_the_same_file_and_lock() {
    use std::os::unix::fs::symlink;
    let directory = Directory::new();
    let path = directory.file();
    let store = ProjectStore::create(&path, project()).unwrap();
    let alias = directory.0.join("kilpailun-linkki.toml");
    symlink(&path, &alias).unwrap();
    let mut aliased = ProjectStore::open(&alias).unwrap();
    assert_eq!(aliased.path(), store.path());
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.0.join("kilpailu.toml.lukko"))
        .unwrap();
    lock.try_lock().unwrap();
    assert!(
        aliased
            .update(|p| {
                p.kilpailu.nimi = "Lukon ohitus".into();
                Ok(())
            })
            .is_err()
    );
    drop(lock);
    aliased
        .update(|p| {
            p.kilpailu.nimi = "Sama tiedosto".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fs::read_to_string(&alias).unwrap(),
        fs::read_to_string(&path).unwrap()
    );
    assert!(alias.is_symlink());
}

#[cfg(unix)]
#[test]
fn replacement_and_backup_preserve_restricted_file_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let directory = Directory::new();
    let path = directory.file();
    let mut store = ProjectStore::create(&path, project()).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    store
        .update(|p| {
            p.kilpailu.nimi = "Yksityinen kilpailu".into();
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(store.backup_path())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}
