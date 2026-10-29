//! Aidot lapsiprosessit ja indeksöidyt Git-fixturet, myös tarkoitukselliset virheet.
#[path = "../src/lan.rs"]
mod lan;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
struct Folder(PathBuf);
impl Folder {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "as2026-checks-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
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
