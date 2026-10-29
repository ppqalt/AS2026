//! Kahden erillisen demo-GUI:n LAN-palautumiskoe. Lokit säilytetään myös virheessä.
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
const ROLES: [&str; 2] = ["isanta", "asiakas"];
const SUCCESS: &str = "automaattinen palautuminen läpäisivät";

#[derive(Default)]
struct Children(Vec<Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in &mut self.0 {
            if !matches!(child.try_wait(), Ok(Some(_))) {
                let _ = child.kill();
            }
        }
        // Kill/TerminateProcess on tehty kaikille ennen odottamista: kumpikaan ei jää taustalle.
        for child in &mut self.0 {
            let _ = child.wait();
        }
    }
}
fn io(error: std::io::Error) -> String {
    error.to_string()
}
fn new_folder(output: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(output).map_err(io)?;
    let tick = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    for index in 0..100 {
        let folder = output.join(format!("lan-{}-{tick}-{index}", std::process::id()));
        match fs::create_dir(&folder) {
            Ok(()) => return folder.canonicalize().map_err(io),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io(error)),
        }
    }
    Err("LAN-koekansiota ei voitu luoda".into())
}
pub fn run(app: &Path, output: &Path, timeout: Duration) -> Result<PathBuf, String> {
    let app = app.canonicalize().map_err(io)?;
    let folder = new_folder(output)?;
    println!("LAN-kokeen lokit ja kuvat: {}", folder.display());
    let mut children = Children::default();
    for role in ROLES {
        let log = File::create(folder.join(format!("{role}.log"))).map_err(io)?;
        let error_log = log.try_clone().map_err(io)?;
        let child = Command::new(&app)
            .arg("--demo")
            .env("SLINT_BACKEND", "winit-software")
            .env("SLINT_SCALE_FACTOR", "1")
            .env("AS2026_LAN_GUI_DIR", &folder)
            .env("AS2026_LAN_GUI_RECOVERY", "1")
            .env("AS2026_LAN_GUI_ROLE", role)
            .env(
                "AS2026_PRINT_PREVIEW_DIR",
                folder.join("tulostusesikatselu"),
            )
            .stdin(Stdio::null())
            .stdout(log)
            .stderr(error_log)
            .spawn()
            .map_err(io)?;
        children.0.push(child);
    }
    let deadline = Instant::now() + timeout;
    loop {
        for entry in fs::read_dir(&folder).map_err(io)? {
            let path = entry.map_err(io)?.path();
            if path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().ends_with("-virhe.txt"))
            {
                return Err(format!(
                    "{}: {}",
                    path.display(),
                    fs::read_to_string(&path).map_err(io)?
                ));
            }
        }
        for (index, child) in children.0.iter_mut().enumerate() {
            if let Some(status) = child.try_wait().map_err(io)? {
                return Err(format!(
                    "GUI {} päättyi ennen LAN-kokeen valmistumista: {status}",
                    ROLES[index]
                ));
            }
        }
        let completed = ROLES.iter().all(|role| {
            fs::read(folder.join(format!("{role}.log")))
                .is_ok_and(|bytes| String::from_utf8_lossy(&bytes).contains(SUCCESS))
        });
        if completed {
            break;
        }
        if Instant::now() >= deadline {
            return Err("Kahden GUI:n LAN-koe aikakatkaistiin".into());
        }
        thread::sleep(Duration::from_millis(25));
    }
    let host = fs::read(folder.join("isanta-palautunut.toml")).map_err(io)?;
    let client = fs::read(folder.join("asiakas-palautunut.toml")).map_err(io)?;
    if host.is_empty() || host != client {
        return Err("Isännän ja asiakkaan palautuneet kilpailutiedot eroavat tai puuttuvat".into());
    }
    println!(
        "Kahden GUI:n LAN-koe läpäisty: Emit, käsikellot, pisteet, hyväksyntä, tulostusesikatselu, yhteyskatko, pysyvä jono ja palautuminen."
    );
    Ok(folder)
}
