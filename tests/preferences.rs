#![cfg(feature = "desktop")]

use as2026::{native, preferences::PreferencesStore, workspace::Workspace};
use std::{
    fs,
    path::PathBuf,
    sync::{
        Arc, Barrier,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT: AtomicU64 = AtomicU64::new(1);
struct Directory(PathBuf);
impl Directory {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "as2026-preferences-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn settings(&self) -> PathBuf {
        self.0.join("asetukset").join("asetukset.toml")
    }
    fn root(&self) -> PathBuf {
        self.0.join("Kilpailut")
    }
    fn open(&self) -> PreferencesStore {
        PreferencesStore::open_at(&self.settings(), &self.root()).unwrap()
    }
}
impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn default_root_requires_no_settings_file_or_folder_selection() {
    let directory = Directory::new();
    let store = directory.open();
    assert_eq!(store.competition_root().unwrap(), directory.root());
    assert!(store.get().competition_root.is_none());
    assert!(store.get().last_competition.is_none());
    assert!(!directory.settings().exists());
    let competition = store
        .create_competition("  Yöajo Äänekoski 2026  ")
        .unwrap();
    assert_eq!(competition.name(), "Yöajo Äänekoski 2026");
    assert_eq!(
        competition.path(),
        fs::canonicalize(
            directory
                .root()
                .join("Yöajo Äänekoski 2026")
                .join(native::FILE_NAME)
        )
        .unwrap()
    );
    assert_eq!(
        Workspace::open(competition.path()).unwrap().name(),
        competition.name()
    );
}

#[test]
fn settings_and_last_competition_survive_reopening_without_moving_data() {
    let directory = Directory::new();
    let mut store = directory.open();
    let competition = store.create_competition("Ensimmäinen kilpailu").unwrap();
    store.remember_competition(competition.path()).unwrap();
    let source = fs::read(competition.path()).unwrap();
    let changed_root = directory.0.join("Toinen tallennuspaikka");
    store.save_root(Some(changed_root.clone())).unwrap();
    let reopened = directory.open();
    assert_eq!(reopened.competition_root().unwrap(), changed_root);
    assert_eq!(
        reopened.get().last_competition.as_deref(),
        Some(competition.path())
    );
    assert_eq!(fs::read(competition.path()).unwrap(), source);
    assert!(!changed_root.exists());
    let settings = fs::read_to_string(directory.settings()).unwrap();
    assert!(settings.contains("kilpailukansio"));
    assert!(settings.contains("viimeisin_kilpailu"));
    store.save_root(None).unwrap();
    assert_eq!(
        directory.open().competition_root().unwrap(),
        directory.root()
    );
}

#[test]
fn same_name_and_existing_files_are_never_replaced() {
    let directory = Directory::new();
    let store = directory.open();
    let first = store.create_competition("Harjoitus").unwrap();
    let original = fs::read(first.path()).unwrap();
    fs::write(
        directory.root().join("Harjoitus (2)"),
        "Säilytettävä tiedosto",
    )
    .unwrap();
    let second = store.create_competition("Harjoitus").unwrap();
    assert_eq!(
        second.path().parent().unwrap().file_name().unwrap(),
        "Harjoitus (3)"
    );
    assert_eq!(fs::read(first.path()).unwrap(), original);
    assert_eq!(
        fs::read_to_string(directory.root().join("Harjoitus (2)")).unwrap(),
        "Säilytettävä tiedosto"
    );
}

#[test]
fn excel_import_uses_unique_default_directory_and_preserves_preparation_fields() {
    let directory = Directory::new();
    let store = directory.open();
    let mut project =
        native::Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    project.kilpailu.nimi = "Excel-kilpailu".into();
    project.kilpailijat[0].emit = 0;
    project.kilpailijat[0].lahtoaika.clear();
    project.leimaluvut.clear();
    let first = store
        .create_imported_in(project.clone(), &directory.root())
        .unwrap();
    let before = fs::read(first.path()).unwrap();
    let second = store
        .create_imported_in(project.clone(), &directory.root())
        .unwrap();
    assert_ne!(first.path(), second.path());
    assert_eq!(first.project().unwrap(), &project);
    assert_eq!(
        Workspace::open(second.path()).unwrap().project().unwrap(),
        &project
    );
    assert_eq!(fs::read(first.path()).unwrap(), before);
    assert!(!directory.settings().exists());
}

#[test]
fn path_separators_controls_and_windows_device_names_are_safe_on_both_platforms() {
    let directory = Directory::new();
    let store = directory.open();
    for (name, expected) in [
        ("../Yöajo\\2026: <A>|B?*", "Yöajo 2026 A B"),
        ("CON", "_CON"),
        ("nul.txt", "_nul.txt"),
        ("lpt9", "_lpt9"),
        ("COM¹", "_COM¹"),
        ("CONOUT$", "_CONOUT$"),
        ("..", "Kilpailu"),
        ("Sade\najo. ", "Sade ajo"),
    ] {
        let competition = store.create_competition(name).unwrap();
        let folder = competition.path().parent().unwrap();
        assert_eq!(
            folder.parent(),
            Some(fs::canonicalize(directory.root()).unwrap().as_path())
        );
        assert_eq!(folder.file_name().unwrap(), expected);
    }
    let competition = store.create_competition(&"ä".repeat(100)).unwrap();
    assert!(
        competition
            .path()
            .parent()
            .unwrap()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .len()
            <= 100
    );
}

#[test]
fn invalid_name_does_not_even_create_the_root_folder() {
    let directory = Directory::new();
    let store = directory.open();
    assert!(store.create_competition(" \n\t ").is_err());
    assert!(!directory.root().exists());
}

#[test]
fn concurrent_competition_creation_allocates_separate_folders() {
    let directory = Directory::new();
    let barrier = Arc::new(Barrier::new(2));
    let handles = (0..2)
        .map(|_| {
            let path = directory.settings();
            let root = directory.root();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let store = PreferencesStore::open_at(&path, &root).unwrap();
                barrier.wait();
                store
                    .create_competition("Yhteinen nimi")
                    .unwrap()
                    .path()
                    .to_owned()
            })
        })
        .collect::<Vec<_>>();
    let paths = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect::<Vec<_>>();
    assert_ne!(paths[0], paths[1]);
    assert!(paths.iter().all(|path| path.is_file()));
}

#[test]
fn stale_settings_and_external_edits_are_not_overwritten() {
    let directory = Directory::new();
    let mut first = directory.open();
    let mut stale = directory.open();
    first
        .save_root(Some(directory.0.join("Ensimmäinen")))
        .unwrap();
    let saved = fs::read(directory.settings()).unwrap();
    let error = stale
        .save_root(Some(directory.0.join("Toinen")))
        .unwrap_err();
    assert!(error.contains("Käynnistä sovellus uudelleen"));
    assert!(stale.get().competition_root.is_none());
    assert_eq!(fs::read(directory.settings()).unwrap(), saved);
    fs::write(directory.settings(), "ulkoinen = true\n").unwrap();
    assert!(first.save_root(None).is_err());
    assert_eq!(
        fs::read_to_string(directory.settings()).unwrap(),
        "ulkoinen = true\n"
    );
}

#[test]
fn failed_backup_keeps_previous_settings_in_memory_and_on_disk() {
    let directory = Directory::new();
    let mut store = directory.open();
    store.save_root(Some(directory.0.join("Säilytä"))).unwrap();
    let saved = fs::read(directory.settings()).unwrap();
    let old = store.get().clone();
    fs::create_dir(directory.settings().with_extension("toml.bak")).unwrap();
    assert!(
        store
            .save_root(Some(directory.0.join("Ei tallentunut")))
            .is_err()
    );
    assert_eq!(store.get(), &old);
    assert_eq!(fs::read(directory.settings()).unwrap(), saved);
    assert_eq!(
        fs::read_dir(directory.settings().parent().unwrap())
            .unwrap()
            .count(),
        3
    );
}

#[test]
fn failed_first_save_and_invalid_remember_do_not_claim_success() {
    let directory = Directory::new();
    let mut store = directory.open();
    fs::write(
        directory.0.join("asetukset"),
        "Kansion paikalla oleva tiedosto",
    )
    .unwrap();
    assert!(store.save_root(Some(directory.0.join("Uusi"))).is_err());
    assert!(store.get().competition_root.is_none());
    assert!(
        store
            .remember_competition(&directory.0.join("puuttuva.toml"))
            .is_err()
    );
    assert!(store.get().last_competition.is_none());
}

#[test]
fn operating_system_lock_blocks_other_settings_writer() {
    let directory = Directory::new();
    let mut store = directory.open();
    store.save_root(Some(directory.root())).unwrap();
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(directory.settings().with_extension("toml.lukko"))
        .unwrap();
    lock.try_lock().unwrap();
    let previous = fs::read(directory.settings()).unwrap();
    assert!(
        store
            .save_root(None)
            .unwrap_err()
            .contains("Toinen paikallinen toiminto")
    );
    assert_eq!(fs::read(directory.settings()).unwrap(), previous);
    drop(lock);
    store.save_root(None).unwrap();
}

#[test]
fn corrupt_settings_are_reported_without_replacing_the_file() {
    let directory = Directory::new();
    fs::create_dir_all(directory.settings().parent().unwrap()).unwrap();
    fs::write(directory.settings(), "kilpailukansio = [\n").unwrap();
    assert!(
        PreferencesStore::open_at(&directory.settings(), &directory.root())
            .err()
            .unwrap()
            .contains("TOML-rakenne")
    );
    assert_eq!(
        fs::read_to_string(directory.settings()).unwrap(),
        "kilpailukansio = [\n"
    );
}

#[test]
fn new_dark_themes_persist_without_changing_other_preferences() {
    use as2026::preferences::Appearance;
    let directory = Directory::new();
    directory.open().save_printer("Testitulostin").unwrap();
    for (appearance, name) in [
        (Appearance::Yomeri, "yomeri"),
        (Appearance::Kanerva, "kanerva"),
        (Appearance::Ruusu, "ruusu"),
        (Appearance::Revontuli, "revontuli"),
    ] {
        let mut store = directory.open();
        store.save_appearance(appearance).unwrap();
        drop(store);
        let reopened = directory.open();
        assert_eq!(reopened.get().appearance, appearance);
        assert_eq!(reopened.get().tulostin, "Testitulostin");
        assert!(!reopened.get().effects_enabled);
        let saved: toml::Value =
            toml::from_str(&fs::read_to_string(directory.settings()).unwrap()).unwrap();
        assert_eq!(saved["appearance"].as_str(), Some(name));
    }
}

#[test]
fn unknown_settings_survive_known_setting_changes_and_reopening() {
    let directory = Directory::new();
    fs::create_dir_all(directory.settings().parent().unwrap()).unwrap();
    let source = r#"
competition_root = "Tallennetut kilpailut"
last_competition = "aiempi/kilpailu.toml"
appearance = "dark"
uusi_asetus = "Mikky100:n tuleva asetus säilyy"
uusi_lista = ["Å", "Ä", "Ö"]

[uusi_taulu]
kerroin = 1.25
omat = { "käytössä" = true, tunnus = 42 }

[[uudet_kohteet]]
nimi = "Ensimmäinen"

[[uudet_kohteet]]
nimi = "Toinen"
"#;
    fs::write(directory.settings(), source).unwrap();
    let original: toml::Value = toml::from_str(source).unwrap();
    let mut store = directory.open();
    assert_eq!(
        store.get().competition_root,
        Some(PathBuf::from("Tallennetut kilpailut"))
    );
    assert_eq!(
        store.get().last_competition,
        Some(PathBuf::from("aiempi/kilpailu.toml"))
    );
    store.save_printer("Testitulostin").unwrap();
    let mut reopened = directory.open();
    assert_eq!(reopened.get().tulostin, "Testitulostin");
    reopened
        .save_appearance(as2026::preferences::Appearance::Spruce)
        .unwrap();
    let saved: toml::Value =
        toml::from_str(&fs::read_to_string(directory.settings()).unwrap()).unwrap();
    for key in ["uusi_asetus", "uusi_lista", "uusi_taulu", "uudet_kohteet"] {
        assert_eq!(saved.get(key), original.get(key), "{key}");
    }
    assert_eq!(saved["appearance"].as_str(), Some("spruce"));
    assert_eq!(saved["tulostin"].as_str(), Some("Testitulostin"));
    assert_eq!(
        saved["kilpailukansio"].as_str(),
        Some("Tallennetut kilpailut")
    );
    assert!(saved.get("competition_root").is_none());
    assert_eq!(
        saved["viimeisin_kilpailu"].as_str(),
        Some("aiempi/kilpailu.toml")
    );
    assert!(saved.get("last_competition").is_none());
}

#[test]
fn duplicate_known_setting_alias_does_not_become_an_unknown_setting() {
    let directory = Directory::new();
    fs::create_dir_all(directory.settings().parent().unwrap()).unwrap();
    let source = "competition_root = 'Vanha'\nkilpailukansio = 'Uusi'\nuusi_asetus = 'säilytä'\n";
    fs::write(directory.settings(), source).unwrap();
    assert!(PreferencesStore::open_at(&directory.settings(), &directory.root()).is_err());
    assert_eq!(fs::read_to_string(directory.settings()).unwrap(), source);
}
