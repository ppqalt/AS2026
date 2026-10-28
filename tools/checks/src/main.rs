//! Projektin Rust-tarkistukset ilman erillisiä tulkkeja tai riippuvuuksia.
mod guard;
mod lan;
use std::{path::PathBuf, time::Duration};

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("check-no-python") => {
            let root = match args.next().as_deref() {
                None => PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."),
                Some("--root") => PathBuf::from(args.next().ok_or("Anna repositorion polku")?),
                Some(_) => return Err("Tuntematon tarkistusvalinta".into()),
            };
            if args.next().is_some() { return Err("Liikaa valintoja".into()); }
            guard::check(&root)
        }
        Some("lan-gui") => {
            let (mut app, mut output) = (None, None);
            while let Some(flag) = args.next() {
                let value = args.next().ok_or("Valinnalta puuttuu arvo")?;
                match flag.as_str() {
                    "--app" => app = Some(PathBuf::from(value)),
                    "--output" => output = Some(PathBuf::from(value)),
                    _ => return Err(format!("Tuntematon valinta: {flag}")),
                }
            }
            lan::run(&app.ok_or("Anna --app")?, &output.ok_or("Anna --output")?, Duration::from_secs(70)).map(|_| ())
        }
        _ => Err("Käyttö: as2026-checks check-no-python [--root repositorio] | lan-gui --app ohjelma --output kansio".into()),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Tarkistus epäonnistui: {error}");
        std::process::exit(1);
    }
}
