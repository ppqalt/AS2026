#![cfg(feature = "desktop")]

use as2026::installation::desktop_entry;
use std::path::Path;

#[test]
fn application_menu_uses_stable_launcher_and_handles_spaces() {
    #[cfg(unix)]
    let (launcher, icon) = (
        Path::new("/home/test user/.local/share/AS2026/app/as2026-launcher"),
        Path::new("/home/test user/.local/share/icons/as2026.svg"),
    );
    #[cfg(windows)]
    let (launcher, icon) = (
        Path::new("C:\\Users\\test user\\AS2026\\as2026-launcher.exe"),
        Path::new("C:\\Users\\test user\\AS2026\\as2026.svg"),
    );
    let entry = desktop_entry(launcher, icon).unwrap();
    assert!(entry.contains("Name=AS2026\n"));
    assert!(entry.contains("Exec=\""));
    assert!(entry.contains("as2026-launcher"));
    assert!(!entry.contains("versions/"));
    assert!(!entry.contains("cargo"));
    assert!(entry.contains("Terminal=false\n"));
}

#[test]
fn application_menu_refuses_relative_or_multiline_paths() {
    assert!(desktop_entry(Path::new("as2026-launcher"), Path::new("icon.svg")).is_err());
    #[cfg(unix)]
    assert!(
        desktop_entry(
            Path::new("/app/launcher\nExec=other"),
            Path::new("/app/icon.svg")
        )
        .is_err()
    );
}

#[cfg(unix)]
#[test]
fn desktop_exec_escapes_field_codes_and_shell_characters() {
    let entry = desktop_entry(
        Path::new("/a % $ ` \" \\/as2026-launcher"),
        Path::new("/a/icon.svg"),
    )
    .unwrap();
    assert!(entry.contains("%%"));
    assert!(entry.contains("\\\\$"));
    assert!(entry.contains("\\\\`"));
    assert!(entry.contains("\\\\\""));
}
