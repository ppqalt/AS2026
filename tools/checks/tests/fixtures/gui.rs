//! Rustilla rakennettu lapsiprosessi koordinaattorin virhepolkujen kokeisiin.
use std::{env, fs, io::{self, Write}, path::PathBuf, thread, time::Duration};
fn main() {
    assert_eq!(env::args().nth(1).as_deref(), Some("--demo"));
    assert_eq!(env::var("SLINT_BACKEND").unwrap(), "winit-software");
    assert_eq!(env::var("SLINT_SCALE_FACTOR").unwrap(), "1");
    assert_eq!(env::var("AS2026_LAN_GUI_RECOVERY").unwrap(), "1");
    let folder = PathBuf::from(env::var_os("AS2026_LAN_GUI_DIR").unwrap());
    let role = env::var("AS2026_LAN_GUI_ROLE").unwrap();
    fs::write(folder.join(format!("{role}.pid")), std::process::id().to_string()).unwrap();
    let mode = fs::read_to_string(folder.parent().unwrap().join("mode")).unwrap();
    match mode.as_str() {
        "exit" => std::process::exit(7),
        "error" => fs::write(folder.join(format!("{role}-virhe.txt")), "Tahallinen virhe").unwrap(),
        "timeout" => (),
        _ => {
            fs::write(folder.join(format!("{role}-palautunut.toml")), if mode == "mismatch" { role.as_bytes() } else { b"sama kilpailu" }).unwrap();
            println!("automaattinen palautuminen läpäisivät"); io::stdout().flush().unwrap();
        }
    }
    loop { thread::sleep(Duration::from_millis(100)); }
}
