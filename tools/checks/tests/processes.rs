//! Aidot lapsiprosessit ja indeksöidyt Git-fixturet, myös tarkoitukselliset virheet.
#[path = "../src/lan.rs"]
mod lan;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
static NEXT_FOLDER: AtomicU64 = AtomicU64::new(0);
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        Self::with_timestamp(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        )
    }

    fn with_timestamp(timestamp: u128) -> Self {
        loop {
            // Windowsin kellon tarkkuus ei takaa eri aikaleimaa rinnakkaisille testeille.
            let sequence = NEXT_FOLDER.fetch_add(1, Ordering::Relaxed);
            let p = std::env::temp_dir().join(format!(
                "as2026-checks-{}-{timestamp}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&p) {
                Ok(()) => return Self(p),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("Testikansion {} luominen epäonnistui: {error}", p.display()),
            }
        }
    }
}
impl Drop for Folder {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .status()
            .unwrap()
            .success()
    );
}
#[test]
fn parallel_test_folders_are_unique_even_with_the_same_timestamp() {
    let workers: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                (0..16)
                    .map(|_| Folder::with_timestamp(0))
                    .collect::<Vec<_>>()
            })
        })
        .collect();
    let folders: Vec<_> = workers
        .into_iter()
        .flat_map(|worker| worker.join().unwrap())
        .collect();
    let paths: std::collections::HashSet<_> = folders.iter().map(|folder| &folder.0).collect();
    assert_eq!(paths.len(), 128);
    assert!(folders.iter().all(|folder| folder.0.is_dir()));
}
#[test]
fn guard_cli_passes_clean_index_and_fails_for_files_commands_and_hidden_code() {
    let root = Folder::new();
    git(&root.0, &["init", "-q"]);
    let tool = env!("CARGO_BIN_EXE_as2026-checks");
    let run = || {
        Command::new(tool)
            .args(["check-no-python", "--root"])
            .arg(&root.0)
            .output()
            .unwrap()
    };
    fs::write(
        root.0.join("README.md"),
        "Python on kielletty; historiallinen python3 old.py",
    )
    .unwrap();
    git(&root.0, &["add", "."]);
    assert!(run().status.success());
    for (path, text) in [
        ("bad.py", "pass"),
        ("run.sh", "python3 -c 'print(1)'"),
        ("hidden", "import os\ndef run():\n    pass"),
    ] {
        fs::write(root.0.join(path), text).unwrap();
        git(&root.0, &["add", path]);
        let result = run();
        assert!(!result.status.success());
        assert!(String::from_utf8_lossy(&result.stderr).contains(path));
        git(&root.0, &["rm", "-f", "--", path]);
    }
    assert!(run().status.success());
}
#[test]
fn two_process_coordinator_detects_success_mismatch_error_early_exit_and_timeout() {
    let root = Folder::new();
    let exe = root.0.join(if cfg!(windows) {
        "fixture.exe"
    } else {
        "fixture"
    });
    assert!(
        Command::new("rustc")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/gui.rs"))
            .args(["--edition=2024", "-o"])
            .arg(&exe)
            .status()
            .unwrap()
            .success()
    );
    for mode in ["success", "mismatch", "error", "exit", "timeout"] {
        let output = root.0.join(mode);
        fs::create_dir(&output).unwrap();
        fs::write(output.join("mode"), mode).unwrap();
        let result = lan::run(&exe, &output, Duration::from_secs(2));
        assert_eq!(result.is_ok(), mode == "success", "{mode}: {result:?}");
        let folder = fs::read_dir(&output)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.path())
            .find(|p| p.is_dir())
            .unwrap();
        for role in ["isanta", "asiakas"] {
            assert!(folder.join(format!("{role}.log")).exists());
            if let Ok(pid) = fs::read_to_string(folder.join(format!("{role}.pid"))) {
                #[cfg(unix)]
                assert!(
                    !Command::new("kill")
                        .args(["-0", pid.trim()])
                        .status()
                        .unwrap()
                        .success(),
                    "Orpo lapsiprosessi"
                );
                #[cfg(windows)]
                {
                    let result = Command::new("tasklist")
                        .args([
                            "/FI",
                            &format!("PID eq {}", pid.trim()),
                            "/FO",
                            "CSV",
                            "/NH",
                        ])
                        .output()
                        .unwrap();
                    assert!(
                        !String::from_utf8_lossy(&result.stdout)
                            .contains(&format!("\"{}\"", pid.trim()))
                    );
                }
            }
        }
    }
}
