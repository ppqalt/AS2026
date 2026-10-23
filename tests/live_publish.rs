#![cfg(feature = "desktop")]
use as2026::{
    live_publish::{self, Config, Publisher},
    native::Project,
};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};
fn fixture() -> Project {
    Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap()
}
type Published = Arc<Mutex<Vec<Vec<u8>>>>;
type TestServer = (Config, Published, Arc<AtomicBool>, thread::JoinHandle<()>);
fn server(count: usize, fail_first: bool, delay_first_store: bool) -> TestServer {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let config = Config {
        ftp: true,
        host: "127.0.0.1".into(),
        port: listener.local_addr().unwrap().port(),
        user: "testaaja".into(),
        password: "synteettinen-salasana".into(),
        directory: "/tulokset".into(),
        filename: "kilpailu.html".into(),
        ..Default::default()
    };
    let published = Arc::new(Mutex::new(Vec::new()));
    let result = published.clone();
    let started = Arc::new(AtomicBool::new(false));
    let stored = started.clone();
    let handle = thread::spawn(move || {
        for i in 0..count {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(20)))
                .unwrap();
            if i == 0 && fail_first {
                socket.write_all(b"421 Katko\r\n").unwrap();
                continue;
            }
            socket
                .write_all(b"220-Testipalvelin\r\n220 Valmis\r\n")
                .unwrap();
            let mut input = BufReader::new(socket.try_clone().unwrap());
            let mut data = None;
            let mut temporary = None;
            let mut rename = false;
            loop {
                let mut line = String::new();
                if input.read_line(&mut line).unwrap_or(0) == 0 {
                    break;
                }
                let response = if line.starts_with("USER ") {
                    "331 Salasana\r\n"
                } else if line.starts_with("PASS ") {
                    "230 OK\r\n"
                } else if line.starts_with("TYPE ") {
                    "200 OK\r\n"
                } else if line.starts_with("CWD ") {
                    "250 OK\r\n"
                } else if line == "PASV\r\n" {
                    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
                    let port = listener.local_addr().unwrap().port();
                    socket
                        .write_all(
                            format!(
                                "227 Passive (203,0,113,1,{},{})\r\n",
                                port / 256,
                                port % 256
                            )
                            .as_bytes(),
                        )
                        .unwrap();
                    data = Some(listener);
                    continue;
                } else if line.starts_with("STOR ") {
                    assert_eq!(line, "STOR kilpailu.html.as2026-upload\r\n");
                    socket.write_all(b"150 Data\r\n").unwrap();
                    let (mut stream, _) = data.take().unwrap().accept().unwrap();
                    let mut bytes = Vec::new();
                    stream.read_to_end(&mut bytes).unwrap();
                    temporary = Some(bytes);
                    stored.store(true, Ordering::Relaxed);
                    if i == 0 && delay_first_store {
                        thread::sleep(Duration::from_millis(500));
                    }
                    "226 OK\r\n"
                } else if line.starts_with("RNFR ") {
                    rename = true;
                    "350 OK\r\n"
                } else if line.starts_with("RNTO ") {
                    assert!(rename);
                    assert_eq!(line, "RNTO kilpailu.html\r\n");
                    result.lock().unwrap().push(temporary.take().unwrap());
                    "250 OK\r\n"
                } else if line == "QUIT\r\n" {
                    socket.write_all(b"221 Hei\r\n").unwrap();
                    break;
                } else {
                    panic!("Odottamaton testikomento")
                };
                socket.write_all(response.as_bytes()).unwrap();
            }
        }
    });
    (config, published, started, handle)
}
#[test]
fn ftp_test_does_not_publish_and_transfer_renames_complete_file() {
    let (config, published, _, handle) = server(2, false, false);
    live_publish::transfer(&config, None).unwrap();
    assert!(published.lock().unwrap().is_empty());
    live_publish::transfer(&config, Some(b"valmis synteettinen tulossivu")).unwrap();
    handle.join().unwrap();
    assert_eq!(
        *published.lock().unwrap(),
        vec![b"valmis synteettinen tulossivu".to_vec()]
    );
}
#[test]
fn background_retry_publishes_latest_snapshot_after_disconnect() {
    let (config, published, _, handle) = server(2, true, false);
    let publisher = Publisher::start(config, "synteettinen.toml".into()).unwrap();
    let mut project = fixture();
    publisher.offer(&project);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !publisher.status().contains("Uusi yritys") {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(30));
    }
    project.kilpailu.nimi = "Viimeisin tilanne & korjaus".into();
    publisher.offer(&project);
    let deadline = Instant::now() + Duration::from_secs(15);
    while published.lock().unwrap().is_empty() {
        assert!(Instant::now() < deadline, "{}", publisher.status());
        thread::sleep(Duration::from_millis(30));
    }
    drop(publisher);
    handle.join().unwrap();
    let values = published.lock().unwrap();
    assert_eq!(values.len(), 1);
    let html = String::from_utf8(values[0].clone()).unwrap();
    assert!(html.contains("Viimeisin tilanne &amp; korjaus"));
    assert!(html.contains("content=\"60;url="));
    assert!(html.contains("Epäviralliset tulokset"));
    assert!(html.contains("Päivitetty"));
}
#[test]
fn credentials_are_not_serialized_and_commands_reject_injection() {
    let config = Config {
        host: "testi".into(),
        port: 22,
        user: "testaaja".into(),
        password: "yksityinen-salasana".into(),
        filename: "tulokset.html".into(),
        ..Default::default()
    };
    assert!(
        !toml::to_string(&config)
            .unwrap()
            .contains("yksityinen-salasana")
    );
    assert!(!format!("{config:?}").contains("yksityinen-salasana"));
    let mut unsafe_config = config;
    unsafe_config.directory = "/tulokset\r\nDELE kilpailu.html".into();
    assert!(unsafe_config.validate().is_err());
}
#[test]
fn sftp_real_isolated_server_when_configured() {
    // Erillinen paikallinen sshd-koe Linuxissa; ei käyttäjän palvelimeen eikä fyysiseen laitteeseen.
    let Ok(file) = std::env::var("AS2026_SFTP_TEST_CONFIG") else {
        return;
    };
    let config: Config = toml::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
    let mut wrong = config.clone();
    wrong.fingerprint = "väärä avain".into();
    assert!(
        live_publish::transfer(&wrong, None)
            .unwrap_err()
            .contains("avainta ei ole vahvistettu")
    );
    let mut uppercase = config.clone();
    uppercase.fingerprint = uppercase.fingerprint.to_ascii_uppercase();
    live_publish::transfer(&uppercase, None).unwrap();
    live_publish::transfer(&config, Some(b"ensimmainen")).unwrap();
    live_publish::transfer(&config, Some(b"korjattu")).unwrap();
    assert_eq!(
        std::fs::read(std::path::Path::new(&config.directory).join(config.filename)).unwrap(),
        b"korjattu"
    );
}

#[test]
fn final_publication_requires_complete_results_and_stops_only_after_success() {
    let (config, published, _, handle) = server(1, false, false);
    let publisher = Publisher::start(config, "synteettinen.toml".into()).unwrap();
    let mut project = fixture();
    assert!(publisher.finish(&project).is_err());
    project.kilpailijat[0].tila = Some(as2026::native::ManualStatus {
        tunnus: "PER".into(),
        syy: "Synteettinen peruutus".into(),
    });
    publisher.finish(&project).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !publisher.is_finished() {
        assert!(Instant::now() < deadline, "{}", publisher.status());
        thread::sleep(Duration::from_millis(30));
    }
    handle.join().unwrap();
    let values = published.lock().unwrap();
    assert_eq!(values.len(), 1);
    assert!(
        String::from_utf8(values[0].clone())
            .unwrap()
            .contains("Lopulliset tulokset")
    );
}

#[test]
fn stopped_publisher_cannot_commit_over_the_replacement_publisher() {
    let (config, published, started, handle) = server(2, false, true);
    let first = Publisher::start(config.clone(), "ensimmainen.toml".into()).unwrap();
    let mut project = fixture();
    first.offer(&project);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !started.load(Ordering::Relaxed) {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(20));
    }
    drop(first);
    let second = Publisher::start(config, "toinen.toml".into()).unwrap();
    project.kilpailu.nimi = "Vain uusi julkaisija".into();
    second.offer(&project);
    let deadline = Instant::now() + Duration::from_secs(5);
    while published.lock().unwrap().is_empty() {
        assert!(Instant::now() < deadline, "{}", second.status());
        thread::sleep(Duration::from_millis(20));
    }
    drop(second);
    handle.join().unwrap();
    let values = published.lock().unwrap();
    assert_eq!(
        values.len(),
        1,
        "Pysäytetty julkaisija ei saa vahvistaa vanhaa tiedostoa"
    );
    assert!(
        String::from_utf8(values[0].clone())
            .unwrap()
            .contains("Vain uusi julkaisija")
    );
}
