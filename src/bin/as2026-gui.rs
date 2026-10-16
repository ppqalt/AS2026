#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
#[path = "../gui/mod.rs"]
mod gui;

fn main() {
    if let Err(error) = gui::run() {
        eprintln!("AS2026: {error}");
        std::process::exit(2);
    }
}
