//! Keinotekoisen aineiston prosessitestit; AS2015A-ohjelmaa ei suoriteta.
use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

const FILES: [&str; 6] = [
    "Aspara.txt",
    "Asperus.txt",
    "Radat.txt",
    "Kilp.txt",
    "emit.txt",
    "emit-frame.raw",
];

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/synthetic")
}

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_as2026"))
}

fn run<I, S>(args: I) -> Output
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    command()
        .args(args)
        .output()
        .expect("Rust-sovellus käynnistyy")
}

fn successful_text(output: Output) -> String {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).expect("Tuloste on UTF-8-muodossa")
}

fn snapshot(directory: &Path) -> [Vec<u8>; 6] {
    FILES.map(|name| std::fs::read(directory.join(name)).unwrap())
}

struct TemporaryDirectory(PathBuf);
impl TemporaryDirectory {
    fn new() -> Self {
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let directory = std::env::temp_dir().join(format!(
            "as2015-{}-{}-{sequence} Äänekosken kilpailu",
            std::process::id(),
            timestamp.as_nanos()
        ));
        std::fs::create_dir(&directory).unwrap();
        Self(directory)
    }

    fn copy_fixture(&self) -> PathBuf {
        let directory = self.0.join("Kilpailun tiedot ääkkösillä");
        std::fs::create_dir(&directory).unwrap();
        for name in FILES {
            std::fs::copy(fixture().join(name), directory.join(name)).unwrap();
        }
        directory
    }
}
impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn inspect_reads_current_header_and_station_order_without_changing_files() {
    let directory = fixture();
    let before = snapshot(&directory);
    let text = successful_text(run([OsStr::new("inspect"), directory.as_os_str()]));
    assert!(text.contains("Luokkia: 1, ratoja: 1, kilpailijoita: 1"));
    assert!(text.contains("tallennettu 645 p, tila TRK, 5 riviä"));
    assert!(text.contains("Kokonaistuloksia ei laskettu uudelleen"));
    assert_eq!(snapshot(&directory), before);
}

#[test]
fn finnish_pura_command_uses_marker_and_signed_seconds_formula() {
    let frame = fixture().join("emit-frame.raw");
    let text = successful_text(run([
        OsStr::new("pura"),
        frame.as_os_str(),
        OsStr::new("12:11:00"),
    ]));
    assert!(text.contains("Emit-kortti: 123456"));
    assert!(text.contains("Maalikello sekunteina: 43800"));
    assert!(text.contains("koodi 90, nollauksesta 450 s"));
}

#[test]
fn example_labels_synthetic_data_and_shows_recovered_full_scoring() {
    for argument in ["esimerkki", "demo"] {
        let text = successful_text(run([argument]));
        assert!(text.contains("SYNTEETTINEN ESIMERKKI"));
        assert!(text.contains("3: ylimääräinen"));
        assert!(text.contains("Virhepisteet yhteensä: 645"));
        assert!(text.contains("Laskennan vaiheet:"));
        assert!(!text.contains("Koko kilpailun pisteitä ei laskettu"));
    }
}

#[test]
fn finnish_ohje_command_explains_supported_operations() {
    let text = successful_text(run(["ohje"]));
    for command in ["tarkista", "laske", "tulokset", "pura", "esimerkki", "lue"] {
        assert!(text.contains(&format!("as2026 {command}")));
    }
    assert!(text.contains("Tuloslaskenta näyttää korttiaineiston laskennan esikatselun"));
    assert!(text.contains("--raportti TIEDOSTO.html"));
}

#[test]
fn finnish_laske_command_reconstructs_preview_without_changing_source_files() {
    let directory = fixture();
    let before = snapshot(&directory);
    let text = successful_text(run([
        OsStr::new("laske"),
        directory.as_os_str(),
        OsStr::new("--vaiheet"),
    ]));
    assert!(text.contains("Tuloslaskennan esikatselu"));
    assert!(text.contains("Virhepisteet yhteensä: 645"));
    assert!(text.contains("Laskettu 1/1 kilpailijaa"));
    assert!(text.contains("Laskennan vaiheet:"));
    assert_eq!(snapshot(&directory), before);
}

#[test]
fn finnish_tulokset_command_ranks_imported_totals() {
    let directory = fixture();
    let before = snapshot(&directory);
    let text = successful_text(run([OsStr::new("tulokset"), directory.as_os_str()]));
    assert!(text.contains("Luokkakohtaiset tulokset — tallennetut pisteet"));
    assert!(text.contains("Ratakohtaiset tulokset"));
    assert!(text.contains("Esimerkkikuljettaja"));
    assert!(text.contains("645"));
    assert!(!text.contains("Tuloslaskennan esikatselu"));
    assert_eq!(snapshot(&directory), before);
}

#[test]
fn piped_menu_runs_example_and_exits_without_keyboard_input() {
    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Rust-sovellus käynnistyy");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"5\r\n0\r\n")
        .unwrap();
    let text = successful_text(child.wait_with_output().unwrap());
    assert!(text.contains("SYNTEETTINEN ESIMERKKI"));
    assert!(text.contains("Virhepisteet yhteensä: 645"));
    assert!(
        text.matches("AS2026 — autosuunnistuksen kilpailunhallinta")
            .count()
            >= 2
    );
    assert!(!text.contains("Palaa valikkoon painamalla Enter"));
}

#[test]
fn finnish_report_option_creates_html_and_never_overwrites_existing_bytes() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.copy_fixture();
    let before = snapshot(&directory);
    for operation in ["laske", "tulokset"] {
        let path = temporary.0.join(format!("{operation} ääkköset.html"));
        let args = [
            OsStr::new(operation),
            directory.as_os_str(),
            OsStr::new("--raportti"),
            path.as_os_str(),
        ];
        let text = successful_text(run(args));
        assert!(text.contains("Raportti tallennettu:"));
        let original = std::fs::read(&path).unwrap();
        let html = String::from_utf8(original.clone()).unwrap();
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<html lang=\"fi\">"));
        assert!(html.contains("Esimerkkikuljettaja"));
        assert!(html.contains("645"));
        assert!(!html.contains("<script"));
        assert!(!html.contains("https://"));

        // Käyttäjän jo muokkaama raportti säilyy myös uuden tallennusyrityksen jälkeen.
        let preserved = b"OMA RAPORTTI: ei saa korvata\n";
        std::fs::write(&path, preserved).unwrap();
        let output = run(args);
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("Tiedosto on jo olemassa, sitä ei korvata"));
        assert_eq!(std::fs::read(&path).unwrap(), preserved);
    }
    assert_eq!(snapshot(&directory), before);
}

#[test]
fn native_argument_paths_support_spaces_and_finnish_letters() {
    let temporary = TemporaryDirectory::new();
    let directory = temporary.copy_fixture();
    for operation in ["tarkista", "laske", "tulokset"] {
        let text = successful_text(run([OsStr::new(operation), directory.as_os_str()]));
        assert!(text.contains("Esimerkkikuljettaja"));
    }
    let frame = directory.join("emit-frame.raw");
    assert!(
        successful_text(run([OsStr::new("pura"), frame.as_os_str()]))
            .contains("Emit-kortti: 123456")
    );
}

#[test]
fn missing_files_and_directories_have_finnish_io_errors() {
    let temporary = TemporaryDirectory::new();
    let missing = temporary.0.join("Puuttuva kansio tai tiedosto");
    for operation in ["tarkista", "pura"] {
        let output = run([OsStr::new(operation), missing.as_os_str()]);
        assert_eq!(output.status.code(), Some(2));
        let error = String::from_utf8(output.stderr).unwrap();
        assert!(error.contains("Virhe: Virheellinen syöte:"));
        assert!(error.contains("Tiedostoa tai kansiota ei löydy"));
        for english in ["No such file", "cannot find", "The system", "os error"] {
            assert!(!error.contains(english), "{error}");
        }
    }
}

#[test]
fn invalid_arguments_and_invalid_clock_fail_explicitly() {
    assert_eq!(run(["tuntematon"]).status.code(), Some(2));
    assert_eq!(
        run([OsStr::new("decode"), fixture().as_os_str()])
            .status
            .code(),
        Some(2)
    );
    let frame = fixture().join("emit-frame.raw");
    let output = run([
        OsStr::new("pura"),
        frame.as_os_str(),
        OsStr::new("24:00:00"),
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Virheellinen syöte"));
}

fn native_directory(temporary: &TemporaryDirectory) -> PathBuf {
    let directory = temporary.0.join("TOML-kilpailu");
    std::fs::create_dir(&directory).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/native-demo/kilpailu.toml"),
        directory.join("kilpailu.toml"),
    )
    .unwrap();
    directory
}

#[test]
fn native_updates_autosave_and_current_results_follow_names_classes_and_station_edits() {
    let temporary = TemporaryDirectory::new();
    let directory = native_directory(&temporary);
    let changes = temporary.0.join("muutokset.toml");
    let original = std::fs::read(directory.join("kilpailu.toml")).unwrap();
    let mut project =
        as2026::native::Project::from_toml(std::str::from_utf8(&original).unwrap()).unwrap();
    project.kilpailijat[0].kuljettaja = "Korjattu kuljettaja".into();
    let mut track = project.radat[0].clone();
    track.numero = 2;
    track.nimi = "Uusi rata".into();
    track.asemat[1].tavoiteaika_sekuntia = 275;
    project.radat.push(track);
    project.luokat[0].rata = 2;
    // Changes accepts only explicit edit sections; histories stay owned by the store.
    let mut edits = toml::Value::try_from(&project).unwrap();
    let table = edits.as_table_mut().unwrap();
    table.remove("versio");
    table.remove("leimaluvut");
    std::fs::write(&changes, toml::to_string_pretty(&edits).unwrap()).unwrap();
    successful_text(run([
        OsStr::new("paivita"),
        directory.as_os_str(),
        changes.as_os_str(),
    ]));
    assert_eq!(
        std::fs::read(directory.join("kilpailu.toml.bak")).unwrap(),
        original
    );
    let reopened = as2026::storage::ProjectStore::open(&directory.join("kilpailu.toml")).unwrap();
    assert_eq!(
        reopened.project().kilpailijat[0].kuljettaja,
        "Korjattu kuljettaja"
    );
    assert_eq!(reopened.project().leimaluvut.len(), 1);
    let current = as2026::application::calculated_project(reopened.project()).unwrap();
    assert_eq!(current.competitors[0].total_penalties, 600);
    let html_path = temporary.0.join("ajantasaiset.html");
    let text = successful_text(run([
        OsStr::new("tulokset"),
        directory.as_os_str(),
        OsStr::new("--raportti"),
        html_path.as_os_str(),
    ]));
    assert!(text.contains("ajantasainen asemalaskenta"));
    assert!(text.contains("Korjattu kuljettaja"));
    assert!(text.contains("600"));
    let html = std::fs::read_to_string(html_path).unwrap();
    assert!(html.contains("kehitysversio"));
    assert!(!html.contains("tuodun Kilp.txt-tiedoston"));
}

#[test]
fn invalid_native_edit_does_not_change_saved_data() {
    let temporary = TemporaryDirectory::new();
    let directory = native_directory(&temporary);
    let changes = temporary.0.join("virhe.toml");
    std::fs::write(
        &changes,
        "[[luokat]]\ntunnus = \"A\"\nnimi = \"Virhe\"\nrata = 999\n",
    )
    .unwrap();
    let before = std::fs::read(directory.join("kilpailu.toml")).unwrap();
    let result = run([
        OsStr::new("paivita"),
        directory.as_os_str(),
        changes.as_os_str(),
    ]);
    assert_eq!(result.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&result.stderr).contains("rataa 999 ei löydy"));
    assert_eq!(
        std::fs::read(directory.join("kilpailu.toml")).unwrap(),
        before
    );
    assert!(!directory.join("kilpailu.toml.bak").exists());
}

#[test]
fn stored_frames_keep_reread_history_without_duplicate_results_and_reject_bad_readings() {
    let temporary = TemporaryDirectory::new();
    let directory = native_directory(&temporary);
    let raw_path = temporary.0.join("kortti.raw");
    let raw = include_bytes!("../fixtures/synthetic/emit-frame.raw");
    let mut decoded = raw.map(|b| b ^ 0xdf);
    for marker in [800u16, 900u16] {
        decoded[23..25].copy_from_slice(&marker.to_le_bytes());
        std::fs::write(&raw_path, decoded.map(|b| b ^ 0xdf)).unwrap();
        successful_text(run([
            OsStr::new("tallenna-kehys"),
            directory.as_os_str(),
            raw_path.as_os_str(),
            OsStr::new("12:11:00"),
        ]));
    }
    let store = as2026::storage::ProjectStore::open(&directory.join("kilpailu.toml")).unwrap();
    assert_eq!(store.project().leimaluvut.len(), 3);
    let archive = as2026::application::calculated_project(store.project()).unwrap();
    assert_eq!(archive.competitors.len(), 1);
    assert_eq!(archive.competitors[0].total_penalties, 645);
    // Change the actual AT time, then check the derived result after reopening.
    decoded[14..16].copy_from_slice(&420u16.to_le_bytes());
    std::fs::write(&raw_path, decoded.map(|b| b ^ 0xdf)).unwrap();
    successful_text(run([
        OsStr::new("tallenna-kehys"),
        directory.as_os_str(),
        raw_path.as_os_str(),
        OsStr::new("12:11:00"),
    ]));
    let store = as2026::storage::ProjectStore::open(&directory.join("kilpailu.toml")).unwrap();
    assert_eq!(
        as2026::application::calculated_project(store.project())
            .unwrap()
            .competitors[0]
            .total_penalties,
        600
    );
    let before = std::fs::read(directory.join("kilpailu.toml")).unwrap();
    decoded[22] = 0; // Remove the only reader marker.
    std::fs::write(&raw_path, decoded.map(|b| b ^ 0xdf)).unwrap();
    assert_eq!(
        run([
            OsStr::new("tallenna-kehys"),
            directory.as_os_str(),
            raw_path.as_os_str(),
            OsStr::new("12:11:00")
        ])
        .status
        .code(),
        Some(2)
    );
    assert_eq!(
        std::fs::read(directory.join("kilpailu.toml")).unwrap(),
        before
    );
}
