//! Erillinen taustajulkaisija: yksi siirto kerrallaan, uusin kokonainen tulostilanne.
//! Salasanat ovat vain muistissa. Julkaisija ei muuta kilpailua eikä LAN-protokollaa.
use crate::native::Project;
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::{TcpStream, ToSocketAddrs},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ftp: bool,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub directory: String,
    pub filename: String,
    pub public_url: String,
    pub key_file: String,
    pub fingerprint: String,
    #[serde(skip)]
    pub password: String,
}
impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Julkaisuasetukset (tunnistetiedot piilotettu)")
    }
}
static TRANSFER: Mutex<()> = Mutex::new(());
impl Config {
    pub fn validate(&self) -> Result<(), String> {
        if self.host.trim().is_empty() || self.user.trim().is_empty() || self.port == 0 {
            return Err("Anna palvelin, portti ja käyttäjätunnus.".into());
        }
        if self.filename.is_empty()
            || self.filename.starts_with('.')
            || self.filename.contains(['/', '\\', '\r', '\n', '\0'])
        {
            return Err("Anna pelkkä tulostiedoston nimi, esimerkiksi tulokset.html.".into());
        }
        if [
            &self.host,
            &self.user,
            &self.password,
            &self.directory,
            &self.key_file,
        ]
        .iter()
        .any(|s| s.contains(['\r', '\n', '\0']))
        {
            return Err("Yhteysasetuksissa on kielletty rivinvaihto.".into());
        }
        if !self.public_url.is_empty()
            && !(self.public_url.starts_with("https://") || self.public_url.starts_with("http://"))
        {
            return Err("Julkisen osoitteen pitää alkaa http:// tai https://.".into());
        }
        Ok(())
    }
}
fn connect(config: &Config) -> Result<TcpStream, String> {
    let addresses = (config.host.as_str(), config.port)
        .to_socket_addrs()
        .map_err(|_| "Palvelimen osoitetta ei voitu selvittää.")?;
    for address in addresses {
        if let Ok(stream) = TcpStream::connect_timeout(&address, Duration::from_secs(8)) {
            stream
                .set_read_timeout(Some(Duration::from_secs(12)))
                .map_err(|_| "Yhteyden aikakatkaisua ei voitu asettaa.")?;
            stream
                .set_write_timeout(Some(Duration::from_secs(12)))
                .map_err(|_| "Yhteyden aikakatkaisua ei voitu asettaa.")?;
            return Ok(stream);
        }
    }
    Err("Palvelimeen ei saatu yhteyttä. Tarkista osoite, portti ja palomuuri.".into())
}
fn ssh(config: &Config) -> Result<ssh2::Session, String> {
    use sha2::{Digest, Sha256};
    let mut session = ssh2::Session::new().map_err(|_| "SFTP-istuntoa ei voitu luoda.")?;
    session.set_tcp_stream(connect(config)?);
    session.set_timeout(12000);
    session
        .handshake()
        .map_err(|_| "SFTP-kättely epäonnistui.")?;
    let (key, _) = session.host_key().ok_or("Palvelimen avain puuttuu.")?;
    let fingerprint = format!(
        "SHA256:{}",
        Sha256::digest(key)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if !config.fingerprint.trim().eq_ignore_ascii_case(&fingerprint) {
        return Err(format!(
            "Palvelimen avainta ei ole vahvistettu. Vertaa ylläpitäjän antamaan SHA256-heksasormenjälkeen ja täytä kenttä: {fingerprint}"
        ));
    }
    if config.key_file.trim().is_empty() {
        session
            .userauth_password(&config.user, &config.password)
            .map_err(|_| "SFTP-tunnistautuminen epäonnistui.")?;
    } else {
        session
            .userauth_pubkey_file(
                &config.user,
                None,
                Path::new(&config.key_file),
                Some(&config.password),
            )
            .map_err(|_| "SFTP-avaimella tunnistautuminen epäonnistui.")?;
    }
    if !session.authenticated() {
        return Err("SFTP-tunnistautumista ei hyväksytty.".into());
    }
    Ok(session)
}
struct Ftp {
    control: BufReader<TcpStream>,
}
impl Ftp {
    fn response(&mut self, expected: &[u16]) -> Result<String, String> {
        let mut first = String::new();
        self.control
            .by_ref()
            .take(8192)
            .read_line(&mut first)
            .map_err(|_| "FTP-palvelin ei vastannut.")?;
        let code = first
            .get(..3)
            .and_then(|s| s.parse::<u16>().ok())
            .ok_or("FTP-vastaus on virheellinen.")?;
        if first.as_bytes().get(3) == Some(&b'-') {
            for _ in 0..100 {
                let mut line = String::new();
                self.control
                    .by_ref()
                    .take(8192)
                    .read_line(&mut line)
                    .map_err(|_| "FTP-vastaus katkesi.")?;
                if line.starts_with(&format!("{code} ")) {
                    break;
                }
                if line.is_empty() {
                    return Err("FTP-yhteys katkesi.".into());
                }
            }
        }
        if !expected.contains(&code) {
            return Err(format!("FTP-palvelin hylkäsi toiminnon (vastaus {code})."));
        }
        Ok(first)
    }
    fn command(&mut self, command: &str, expected: &[u16]) -> Result<String, String> {
        self.control
            .get_mut()
            .write_all(format!("{command}\r\n").as_bytes())
            .map_err(|_| "FTP-komennon lähetys epäonnistui.")?;
        self.response(expected)
    }
    fn login(c: &Config) -> Result<Self, String> {
        let mut ftp = Self {
            control: BufReader::new(connect(c)?),
        };
        ftp.response(&[220])?;
        let reply = ftp.command(&format!("USER {}", c.user), &[230, 331])?;
        if reply.starts_with("331") {
            ftp.command(&format!("PASS {}", c.password), &[230])?;
        }
        ftp.command("TYPE I", &[200])?;
        if !c.directory.is_empty() {
            ftp.command(&format!("CWD {}", c.directory), &[250])?;
        }
        Ok(ftp)
    }
    fn put(
        &mut self,
        c: &Config,
        bytes: &[u8],
        cancelled: Option<&AtomicBool>,
    ) -> Result<(), String> {
        let response = self.command("PASV", &[227])?;
        let parts = response
            .split_once('(')
            .and_then(|(_, s)| s.split_once(')'))
            .map(|(s, _)| {
                s.split(',')
                    .map(str::parse::<u8>)
                    .collect::<Result<Vec<_>, _>>()
            })
            .ok_or("FTP:n passiiviosoite puuttuu.")?
            .map_err(|_| "FTP:n passiiviosoite on virheellinen.")?;
        if parts.len() != 6 {
            return Err("FTP:n passiiviosoite on virheellinen.".into());
        }
        let port = u16::from(parts[4]) * 256 + u16::from(parts[5]);
        let mut address = self
            .control
            .get_ref()
            .peer_addr()
            .map_err(|_| "FTP-yhteys katkesi.")?;
        address.set_port(port);
        let mut data = TcpStream::connect_timeout(&address, Duration::from_secs(8))
            .map_err(|_| "FTP:n datayhteys ei auennut. Tarkista passiivisen FTP:n palomuuri.")?;
        data.set_write_timeout(Some(Duration::from_secs(12)))
            .map_err(|_| "FTP-aikakatkaisun asetus epäonnistui.")?;
        let temporary = format!("{}.as2026-upload", c.filename);
        self.command(&format!("STOR {temporary}"), &[125, 150])?;
        data.write_all(bytes)
            .map_err(|_| "FTP-tiedostosiirto katkesi.")?;
        drop(data);
        self.response(&[226, 250])?;
        check_cancelled(cancelled)?;
        self.command(&format!("RNFR {temporary}"), &[350])?;
        self.command(&format!("RNTO {}", c.filename), &[250])?;
        Ok(())
    }
}
// ssh2 0.9:n turvallinen rajapinta ei vielä tarjoa OpenSSH:n atomista
// korvauslaajennusta. Sidotaan vain tämä libssh2 1.11:n julkinen funktio.
fn posix_rename(session: &ssh2::Session, source: &str, destination: &str) -> Result<(), String> {
    use std::ffi::{CString, c_char, c_int};
    unsafe extern "C" {
        fn libssh2_sftp_posix_rename_ex(
            sftp: *mut libssh2_sys::LIBSSH2_SFTP,
            source: *const c_char,
            source_len: usize,
            destination: *const c_char,
            destination_len: usize,
        ) -> c_int;
    }
    let source = CString::new(source).map_err(|_| "SFTP-tiedostopolku on virheellinen.")?;
    let destination =
        CString::new(destination).map_err(|_| "SFTP-tiedostopolku on virheellinen.")?;
    let mut locked = session.raw();
    // SAFETY: Sessionin lukko estää rinnakkaisen käytön. Uusi SFTP-kahva
    // suljetaan täsmälleen kerran ennen lukon vapauttamista. CStringit elävät
    // kaikkien kutsujen ajan; pituudet ovat tavumääriä ilman nollapäätettä.
    let code = unsafe {
        let handle = libssh2_sys::libssh2_sftp_init(&mut *locked);
        if handle.is_null() {
            return Err("SFTP-tiedostonvaihdon yhteys ei auennut.".into());
        }
        let code = libssh2_sftp_posix_rename_ex(
            handle,
            source.as_ptr(),
            source.as_bytes().len(),
            destination.as_ptr(),
            destination.as_bytes().len(),
        );
        libssh2_sys::libssh2_sftp_shutdown(handle);
        code
    };
    if code != 0 {
        return Err(
            "SFTP-palvelin ei tue atomista posix-rename-tiedostonvaihtoa. Aiempi tulos säilyy."
                .into(),
        );
    }
    Ok(())
}

fn check_cancelled(cancelled: Option<&AtomicBool>) -> Result<(), String> {
    if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
        return Err("Julkaisu pysäytettiin.".into());
    }
    Ok(())
}
pub fn transfer(config: &Config, content: Option<&[u8]>) -> Result<(), String> {
    transfer_with_cancel(config, content, None)
}
fn transfer_with_cancel(
    config: &Config,
    content: Option<&[u8]>,
    cancelled: Option<&AtomicBool>,
) -> Result<(), String> {
    config.validate()?;
    let _serial = TRANSFER
        .lock()
        .map_err(|_| "Edellinen tiedostosiirto keskeytyi odottamattomasti.")?;
    check_cancelled(cancelled)?;
    if config.ftp {
        let mut ftp = Ftp::login(config)?;
        if let Some(bytes) = content {
            ftp.put(config, bytes, cancelled)?;
        }
        let _ = ftp.command("QUIT", &[221]);
    } else {
        let session = ssh(config)?;
        let sftp = session
            .sftp()
            .map_err(|_| "SFTP-tiedostopalvelu ei auennut.")?;
        let dir = if config.directory.is_empty() {
            Path::new(".")
        } else {
            Path::new(&config.directory)
        };
        sftp.opendir(dir)
            .map_err(|_| "SFTP-kohdehakemisto ei avaudu.")?;
        if let Some(bytes) = content {
            let temporary = dir.join(format!("{}.as2026-upload", config.filename));
            let mut file = sftp
                .create(&temporary)
                .map_err(|_| "SFTP-väliaikaistiedostoa ei voitu luoda.")?;
            file.write_all(bytes)
                .map_err(|_| "SFTP-tiedostosiirto katkesi.")?;
            file.close()
                .map_err(|_| "SFTP-tiedostoa ei voitu sulkea.")?;
            let remote =
                |name: &str| format!("{}/{}", config.directory.trim_end_matches('/'), name);
            drop(file);
            drop(sftp);
            let remote = |name: &str| {
                if config.directory.is_empty() {
                    name.to_owned()
                } else {
                    remote(name)
                }
            };
            check_cancelled(cancelled)?;
            posix_rename(
                &session,
                &remote(&format!("{}.as2026-upload", config.filename)),
                &remote(&config.filename),
            )?;
        }
    }
    Ok(())
}
pub fn html(project: &Project) -> Result<String, String> {
    let html =
        crate::report::competition_ranking_html(project, crate::ranking::TiePolicy::default())
            .map_err(|_| "Tulossivun muodostaminen epäonnistui. Tarkista kilpailun tiedot.")?;
    Ok(html.replacen("<head>","<head><meta http-equiv=\"refresh\" content=\"60\"><meta http-equiv=\"Cache-Control\" content=\"no-store, max-age=0\"><meta http-equiv=\"Pragma\" content=\"no-cache\">",1)
        .replacen("<body>",&format!("<body><p>AS2026 · Epäviralliset tulokset · Päivitetty {}</p>",chrono::Local::now().format("%d.%m.%Y %H:%M:%S %:z")),1))
}
struct Latest {
    value: Option<(u64, Project)>,
    generation: u64,
    final_result: bool,
}
pub struct Publisher {
    latest: Arc<Mutex<Latest>>,
    wake: mpsc::Sender<()>,
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<String>>,
    finished: Arc<AtomicBool>,
    pub path: PathBuf,
}
impl Publisher {
    pub fn start(config: Config, path: PathBuf) -> Result<Self, String> {
        config.validate()?;
        let latest = Arc::new(Mutex::new(Latest {
            value: None,
            generation: 0,
            final_result: false,
        }));
        let shared = latest.clone();
        let status = Arc::new(Mutex::new(
            "Julkaisu käynnissä; odottaa tulostilannetta.".into(),
        ));
        let state = status.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let cancelled = stop.clone();
        let finished = Arc::new(AtomicBool::new(false));
        let completed = finished.clone();
        let (wake, rx) = mpsc::channel();
        thread::Builder::new()
            .name("as2026-live-julkaisu".into())
            .spawn(move || {
                let mut sent = 0;
                let mut next = Instant::now();
                while !cancelled.load(Ordering::Relaxed) {
                    let _ = rx.recv_timeout(Duration::from_millis(300));
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }
                    if Instant::now() < next {
                        continue;
                    }
                    let (job, final_result) = { let latest = shared.lock().unwrap(); (latest.value.clone(), latest.final_result) };
                    if let Some((generation, project)) = job {
                        if generation == sent {
                            continue;
                        }
                        // Odotukset yhdistetään: vain viimeisin tilanne lasketaan taustalla.
                        let outcome = html(&project).and_then(|body| {
                            let body = if final_result { body.replace("Epäviralliset tulokset", "Lopulliset tulokset") } else { body };
                            transfer_with_cancel(&config, Some(body.as_bytes()), Some(&cancelled))
                        });
                        match outcome {
                            Ok(()) => {
                                if final_result {
                                    *state.lock().unwrap() = format!("Lopulliset tulokset julkaistu {}. Automaattijulkaisu päättyi.", chrono::Local::now().format("%H:%M:%S"));
                                    completed.store(true, Ordering::Relaxed);
                                    break;
                                }
                                sent = generation;
                                *state.lock().unwrap() = format!(
                                    "Viimeisin lähetys {}. Automaattijulkaisu käynnissä.",
                                    chrono::Local::now().format("%H:%M:%S")
                                );
                                next = Instant::now();
                            }
                            Err(error) => {
                                *state.lock().unwrap() =
                                    format!("{error} Uusi yritys 10 sekunnin kuluttua.");
                                next = Instant::now() + Duration::from_secs(10);
                            }
                        }
                    }
                }
            })
            .map_err(|_| "Julkaisun taustatyötä ei voitu käynnistää.")?;
        Ok(Self {
            latest,
            wake,
            stop,
            status,
            finished,
            path,
        })
    }
    pub fn offer(&self, project: &Project) {
        let mut latest = self.latest.lock().unwrap();
        if latest.final_result || latest.value.as_ref().is_some_and(|(_, p)| p == project) {
            return;
        }
        latest.generation += 1;
        let generation = latest.generation;
        latest.value = Some((generation, project.clone()));
        let _ = self.wake.send(());
    }
    pub fn finish(&self, project: &Project) -> Result<(), String> {
        let archive = crate::application::calculated_project(project)
            .map_err(|_| "Tarkista kilpailun tulokset ennen lopullista julkaisua.")?;
        if archive
            .competitors
            .iter()
            .any(|driver| !archive.can_publish_competitor(driver))
        {
            return Err("Kilpailussa on keskeneräisiä tuloksia. Hyväksy tulokset ennen lopullista julkaisua.".into());
        }
        if project.beginners_cup.is_some() {
            let bc = crate::beginners::results(project)
                .map_err(|_| "Tarkista Beginners Cupin tulokset.")?;
            if bc
                .competitors
                .iter()
                .any(|driver| !bc.can_publish_competitor(driver))
            {
                return Err("Beginners Cupissa on keskeneräisiä tuloksia.".into());
            }
        }
        let mut latest = self.latest.lock().unwrap();
        if latest.final_result {
            return Err("Lopullisen tulostilanteen lähetys on jo käynnissä.".into());
        }
        latest.final_result = true;
        latest.generation += 1;
        let generation = latest.generation;
        latest.value = Some((generation, project.clone()));
        let _ = self.wake.send(());
        Ok(())
    }
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }
    pub fn status(&self) -> String {
        self.status.lock().unwrap().clone()
    }
}
impl Drop for Publisher {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.wake.send(());
    }
}
