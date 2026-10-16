//! Pysyvä käynnistin ja allekirjoitetun julkaisupaketin asennusohjelma.
#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use as2026::installation;
use std::path::Path;

fn run() -> Result<(), String> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    match arguments.as_slice() {
        [] => {
            let binary = std::env::current_exe()
                .map_err(|_| "Asennusohjelman sijaintia ei voi tunnistaa".to_owned())?;
            let directory = binary
                .parent()
                .ok_or_else(|| "Asennusohjelman kansio puuttuu".to_owned())?;
            if directory.join("release.json").is_file() || directory.join("release.sig").is_file() {
                installation::install_bundle(directory).map(|_| ())
            } else {
                installation::launch_current()
            }
        }
        [option, bundle] if option == "--install" || option == "--apply" => {
            let outcome = installation::install_bundle(Path::new(bundle))?;
            println!("AS2026 {} asennettu ja käynnistetty.", outcome.version);
            Ok(())
        }
        [option, bundle, ready, token] if option == "--apply" && ready == "--ready-token" => {
            let token = token
                .to_str()
                .ok_or_else(|| "Päivityshelperin tunnus on virheellinen".to_owned())?;
            installation::install_bundle_ready(Path::new(bundle), token).map(|_| ())
        }
        [option] if option == "--rollback" => installation::rollback_to_previous(),
        [option] if option == "--help" || option == "-h" => {
            println!(
                "AS2026-käynnistin\n\nIlman valitsimia avataan asennettu ohjelma.\n--install KANSIO  Asenna varmennettu julkaisupaketti.\n--rollback        Palauta edellinen ohjelmaversio."
            );
            Ok(())
        }
        _ => Err("Tuntematon käynnistysvalitsin. Ohje: as2026-launcher --help".into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("AS2026: {error}");
        show_error(&error);
        std::process::exit(1);
    }
}

#[cfg(windows)]
fn show_error(error: &str) {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn MessageBoxW(
            window: *mut std::ffi::c_void,
            text: *const u16,
            title: *const u16,
            flags: u32,
        ) -> i32;
    }
    let text: Vec<_> = error.encode_utf16().chain(Some(0)).collect();
    let title: Vec<_> = "AS2026".encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(std::ptr::null_mut(), text.as_ptr(), title.as_ptr(), 0x10);
    }
}

#[cfg(target_os = "linux")]
fn show_error(error: &str) {
    let _ = std::process::Command::new("notify-send")
        .args([
            "--app-name=AS2026",
            "--icon=as2026",
            "--",
            "AS2026:n käynnistys epäonnistui",
            error,
        ])
        .spawn();
}

#[cfg(not(any(windows, target_os = "linux")))]
fn show_error(_error: &str) {}
