//! Isäntä järjestää validoidut muutokset. Alkuperäinen kilpailu säilyy TOML-muodossa.
//! Noise-yhteys salaa liikenteen ja todentaa molemmat päät parituskoodilla.
use crate::{
    CoreError,
    native::{Project, Readout},
    storage::{ProjectStore, durable_write},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    net::{SocketAddr, TcpListener, TcpStream, UdpSocket},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

pub const PORT: u16 = 42165;
const DISCOVERY_PORT: u16 = 42166;
const LIMIT: usize = 32 * 1024 * 1024;
const PROTOCOL: u32 = 1;
fn err(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn message(e: &CoreError) -> String {
    match e {
        CoreError::InvalidInput(text) => text.clone(),
        _ => e.to_string(),
    }
}
fn io(e: std::io::Error) -> CoreError {
    err(crate::io_message(&e))
}
fn json<T: Serialize>(value: &T) -> Result<Vec<u8>, CoreError> {
    serde_json::to_vec(value).map_err(|_| err("LAN-tiedon muodostaminen epäonnistui"))
}
fn side(path: &Path, suffix: &str) -> PathBuf {
    let mut p = path.as_os_str().to_owned();
    p.push(suffix);
    p.into()
}
fn valid_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}
fn random_id() -> Result<String, CoreError> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b)
        .map_err(|_| err("Turvallisen tapahtumatunnuksen luonti epäonnistui"))?;
    Ok(b.iter().map(|b| format!("{b:02x}")).collect())
}
fn new_code() -> Result<String, CoreError> {
    let mut b = [0u8; 12];
    getrandom::fill(&mut b).map_err(|_| err("Parituskoodin luonti epäonnistui"))?;
    let alphabet = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    Ok(b.into_iter()
        .map(|b| alphabet[usize::from(b) % alphabet.len()] as char)
        .collect())
}
fn key(code: &str) -> [u8; 32] {
    Sha256::digest(
        code.chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .flat_map(char::to_uppercase)
            .collect::<String>(),
    )
    .into()
}

#[derive(Clone, Serialize, Deserialize, Debug)]
struct Change {
    section: String,
    identity: Option<String>,
    before: Option<Value>,
    after: Option<Value>,
}
#[derive(Clone, Serialize, Deserialize, Debug)]
struct Operation {
    id: String,
    changes: Vec<Change>,
    reads: Vec<Readout>,
    context: Option<Value>,
}
fn identity(section: &str, value: &Value) -> Option<String> {
    let field = match section {
        "kilpailijat" | "radat" | "bc_osallistujat" => "numero",
        "luokat" => "tunnus",
        _ => return None,
    };
    value.get(field).map(ToString::to_string)
}
fn entries(section: &str, value: Option<&Value>) -> BTreeMap<String, Value> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|v| identity(section, v).map(|k| (k, v.clone())))
        .collect()
}
fn context(project: &Project, numbers: &BTreeSet<u32>) -> Result<Value, CoreError> {
    let cards: BTreeSet<_> = project
        .kilpailijat
        .iter()
        .filter(|d| numbers.contains(&d.numero))
        .map(|d| d.emit)
        .collect();
    let drivers = project
        .kilpailijat
        .iter()
        .filter(|d| numbers.contains(&d.numero))
        .map(|d| {
            let mut d = d.clone();
            d.tallennettu_tulos = None;
            d
        })
        .collect::<Vec<_>>();
    // Myös BC:n hyväksyntä riippuu yhteisistä kilpailijatiedoista, ei vain BC-tietueesta.
    // Tuloksen hyväksyntä/korjaus ei saa käyttää väliin muuttunutta rataa tai korttia.
    Ok(
        serde_json::json!({"kilpailijat":drivers,"kilpailu":project.kilpailu,"virhepisteet":project.virhepisteet,"radat":project.radat,"luokat":project.luokat,"yhdistetyt_luokat":project.yhdistetyt_luokat,"beginners_cup":project.beginners_cup,"bc_osallistujat":project.bc_osallistujat.iter().filter(|entry|numbers.contains(&entry.numero)).collect::<Vec<_>>(),"sarjapisteet":project.sarjapisteet,"leimaluvut":project.leimaluvut.iter().filter(|r| cards.contains(&r.emit)).collect::<Vec<_>>(),"emit_korjaukset":project.emit_korjaukset.iter().filter(|c| numbers.contains(&c.kilpailija)).collect::<Vec<_>>() }),
    )
}

fn affected_numbers(changes: &[Change]) -> BTreeSet<u32> {
    let mut numbers = BTreeSet::new();
    for c in changes {
        if matches!(c.section.as_str(), "kilpailijat" | "bc_osallistujat") {
            if let Some(n) = c
                .after
                .as_ref()
                .or(c.before.as_ref())
                .and_then(|v| v.get("numero"))
                .and_then(Value::as_u64)
            {
                numbers.insert(n as u32);
            }
        } else if c.section == "emit_korjaukset" {
            for v in c
                .after
                .as_ref()
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                if let Some(n) = v.get("kilpailija").and_then(Value::as_u64) {
                    numbers.insert(n as u32);
                }
            }
        }
    }
    numbers
}

fn append_read(project: &mut Project, read: Readout) -> Result<(), CoreError> {
    let outcome = project.record_readout(read)?;
    if outcome.content_changed
        && let Some(number) = outcome.competitor_number
    {
        for result in project
            .kilpailijat
            .iter_mut()
            .filter(|d| d.numero == number)
            .filter_map(|d| d.tallennettu_tulos.as_mut())
        {
            result.hyvaksytty = false;
            result.kasikellot_muuttuneet = true;
        }
        for result in project
            .bc_osallistujat
            .iter_mut()
            .filter(|d| d.numero == number)
            .filter_map(|d| d.tallennettu_tulos.as_mut())
        {
            result.hyvaksytty = false;
            result.kasikellot_muuttuneet = true;
        }
    }
    Ok(())
}

fn validate_approvals(project: &Project, changes: &[Change]) -> Result<(), CoreError> {
    fn check(
        project: &Project,
        number: u32,
        stored: &crate::corrections::StoredResult,
    ) -> Result<(), CoreError> {
        let fresh = crate::application::capture_project_result(project, number)?;
        let same = stored.rivit.len() == fresh.rivit.len()
            && stored.rivit.iter().zip(&fresh.rivit).all(|(a, b)| {
                a.sequence == b.sequence
                    && a.expected_code == b.expected_code
                    && a.received_code == b.received_code
                    && ((a.expected_code == 0 && a.sequence != 0)
                        || stored.kasin_annetut_kellot.contains(&a.output_index)
                        || a.elapsed_seconds == b.elapsed_seconds)
            });
        if !same {
            return Err(err(
                "Emit-lukema on muuttunut. Valitse Tarkista ja varmista käsikellot ennen tuloksen hyväksymistä. Aiemmat tiedot ovat edelleen näkyvissä.",
            ));
        }
        Ok(())
    }
    for number in affected_numbers(changes) {
        if let Some(result) = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .and_then(|d| d.tallennettu_tulos.as_ref())
            .filter(|r| r.hyvaksytty)
        {
            check(project, number, result)?;
        }
        if let Some(result) = project
            .bc_osallistujat
            .iter()
            .find(|e| e.numero == number && e.osallistuu)
            .and_then(|e| e.tallennettu_tulos.as_ref())
            .filter(|r| r.hyvaksytty)
        {
            check(&crate::beginners::projection(project)?, number, result)?;
        }
    }
    Ok(())
}

impl Operation {
    fn create(
        base: &Project,
        candidate: &Project,
        pinned: Option<&Project>,
    ) -> Result<Self, CoreError> {
        let mut only_reads = base.clone();
        let appended = candidate
            .leimaluvut
            .get(base.leimaluvut.len()..)
            .unwrap_or(&[]);
        if candidate.leimaluvut.starts_with(&base.leimaluvut) && !appended.is_empty() {
            only_reads.leimaluvut.extend_from_slice(appended);
            crate::corrections::invalidate_input_changes(base, &mut only_reads);
            if &only_reads == candidate {
                return Ok(Self {
                    id: random_id()?,
                    changes: vec![],
                    reads: appended.to_vec(),
                    context: None,
                });
            }
        }
        if base.leimaluvut != candidate.leimaluvut {
            return Err(err(
                "Alkuperäistä Emit-historiaa ei voi korvata LAN-yhteydessä",
            ));
        }
        let b = serde_json::to_value(base).map_err(|_| err("Kilpailun lukeminen epäonnistui"))?;
        let a =
            serde_json::to_value(candidate).map_err(|_| err("Kilpailun lukeminen epäonnistui"))?;
        let basis = pinned.unwrap_or(base);
        let pinned = serde_json::to_value(basis).map_err(|_| err("Muokkauspohja puuttuu"))?;
        let mut changes = vec![];
        for section in b
            .as_object()
            .unwrap()
            .keys()
            .chain(a.as_object().unwrap().keys())
            .collect::<BTreeSet<_>>()
        {
            let before = b.get(section);
            let after = a.get(section);
            if before == after {
                continue;
            }
            if matches!(
                section.as_str(),
                "kilpailijat" | "radat" | "luokat" | "bc_osallistujat"
            ) {
                let old = entries(section, before);
                let new = entries(section, after);
                let pin = entries(section, pinned.get(section));
                let ids = after
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .chain(before.and_then(Value::as_array).into_iter().flatten())
                    .filter_map(|v| identity(section, v));
                let mut seen = BTreeSet::new();
                for id in ids {
                    if !seen.insert(id.clone()) {
                        continue;
                    }
                    if old.get(&id) != new.get(&id) {
                        changes.push(Change {
                            section: section.clone(),
                            identity: Some(id.clone()),
                            before: pin.get(&id).cloned(),
                            after: new.get(&id).cloned(),
                        });
                    }
                }
            } else {
                changes.push(Change {
                    section: section.clone(),
                    identity: None,
                    before: pinned.get(section).cloned(),
                    after: after.cloned(),
                });
            }
        }
        let numbers = affected_numbers(&changes);
        let ctx = if numbers.is_empty() {
            None
        } else {
            Some(context(basis, &numbers)?)
        };
        Ok(Self {
            id: random_id()?,
            changes,
            reads: vec![],
            context: ctx,
        })
    }
    fn apply(&self, current: &Project) -> Result<Project, CoreError> {
        if self.id.len() != 32
            || !self.id.bytes().all(|b| b.is_ascii_hexdigit())
            || self.reads.len() > 100
            || self.changes.len() > 4096
        {
            return Err(err("LAN-tapahtuman tunnus tai koko on virheellinen"));
        }
        let mut candidate = current.clone();
        if !self.reads.is_empty() {
            if !self.changes.is_empty() {
                return Err(err("Virheellinen LAN-lukutapahtuma"));
            }
            for read in &self.reads {
                append_read(&mut candidate, read.clone())?;
            }
        } else {
            let numbers = affected_numbers(&self.changes);
            if let Some(expected) = &self.context
                && expected != &context(current, &numbers)?
            {
                return Err(err(
                    "Kilpailun laskentatiedot muuttuivat toisella koneella. Avaa tiedot uudelleen ennen tallentamista",
                ));
            }
            let mut value = serde_json::to_value(current)
                .map_err(|_| err("Kilpailun lukeminen epäonnistui"))?;
            for change in &self.changes {
                if matches!(change.section.as_str(), "leimaluvut" | "versio") {
                    return Err(err("Suojattua kilpailutietoa ei voi korvata"));
                }
                if let Some(id) = &change.identity {
                    if value.get(&change.section).is_none() {
                        value[&change.section] = Value::Array(vec![]);
                    }
                    let array = value
                        .get_mut(&change.section)
                        .and_then(Value::as_array_mut)
                        .ok_or_else(|| err("LAN-tietueryhmä on virheellinen"))?;
                    let pos = array
                        .iter()
                        .position(|v| identity(&change.section, v).as_ref() == Some(id));
                    if pos.map(|i| &array[i]) != change.before.as_ref() {
                        return Err(err(format!(
                            "Ristiriita: {} {} muuttui toisella koneella. Oma syöte säilyy lomakkeella; avaa tietue uudelleen ja tarkista muutos",
                            change.section, id
                        )));
                    }
                    match (pos, &change.after) {
                        (Some(i), Some(v)) => array[i] = v.clone(),
                        (Some(i), None) => {
                            array.remove(i);
                        }
                        (None, Some(v)) => array.push(v.clone()),
                        _ => {}
                    }
                } else {
                    if value.get(&change.section) != change.before.as_ref() {
                        return Err(err(format!(
                            "Ristiriita: {} muuttui toisella koneella. Avaa tiedot uudelleen",
                            change.section
                        )));
                    }
                    match &change.after {
                        Some(v) => {
                            value[&change.section] = v.clone();
                        }
                        None => {
                            value.as_object_mut().unwrap().remove(&change.section);
                        }
                    }
                }
            }
            candidate = serde_json::from_value(value)
                .map_err(|_| err("Verkkomuutoksen kilpailumuoto on virheellinen"))?;
            // Korjaushistoria on vain kasvava; aiempaa perustelua tai lähdelukemaa ei saa vaihtaa.
            if !candidate
                .emit_korjaukset
                .starts_with(&current.emit_korjaukset)
            {
                return Err(err("Emit-korjaushistoriaa ei voi korvata"));
            }
        }
        crate::corrections::invalidate_input_changes(current, &mut candidate);
        validate_approvals(&candidate, &self.changes)?;
        candidate.validate()?;
        Ok(candidate)
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Journal {
    version: u32,
    code: String,
    competition: String,
    revision: u64,
    previous: Project,
    project: Project,
    accepted: BTreeSet<String>,
}
struct Authority {
    store: ProjectStore,
    journal: Journal,
    path: PathBuf,
}
impl Authority {
    fn open(path: &Path) -> Result<Self, CoreError> {
        let mut store = ProjectStore::open(path)?;
        store.enable_lan_authority();
        let path = side(store.path(), ".lan-isanta.json");
        let journal = if path.exists() {
            let mut j: Journal = serde_json::from_slice(&fs::read(&path).map_err(io)?)
                .map_err(|_| err("LAN-palautumistiedosto on vaurioitunut"))?;
            if j.version != PROTOCOL {
                return Err(err("LAN-palautumistiedoston versio ei sovi"));
            }
            if store.project() != &j.project {
                if store.project() == &j.previous {
                    store.update(|p| {
                        *p = j.project.clone();
                        Ok(())
                    })?;
                } else if j.previous == j.project {
                    j.project = store.project().clone();
                    j.previous = j.project.clone();
                    j.revision += 1;
                } else {
                    return Err(err(
                        "LAN-tallennus keskeytyi ja kilpailua muutettiin sen jälkeen. Säilytä kilpailu ja LAN-palautumistiedosto ennen palautusta",
                    ));
                }
            }
            j
        } else {
            Journal {
                version: PROTOCOL,
                code: new_code()?,
                competition: random_id()?,
                revision: 0,
                previous: store.project().clone(),
                project: store.project().clone(),
                accepted: BTreeSet::new(),
            }
        };
        let mut journal = journal;
        journal.previous = journal.project.clone();
        durable_write(&path, &json(&journal)?)?;
        Ok(Self {
            store,
            journal,
            path,
        })
    }
    fn commit(&mut self, op: &Operation) -> Result<(), CoreError> {
        if self.store.project() != &self.journal.project {
            let project = self.journal.project.clone();
            self.store.update(|p| {
                *p = project;
                Ok(())
            })?;
        }
        if self.journal.accepted.contains(&op.id) {
            self.journal.previous = self.journal.project.clone();
            durable_write(&self.path, &json(&self.journal)?)?;
            return Ok(());
        }
        let candidate = op.apply(&self.journal.project)?;
        let mut next = self.journal.clone();
        next.previous = next.project.clone();
        next.project = candidate.clone();
        next.revision += 1;
        next.accepted.insert(op.id.clone());
        durable_write(&self.path, &json(&next)?)?;
        // Lokimerkintä on pysyvä ennen kilpailun julkaisua: kaatuminen voidaan toistaa.
        self.journal = next;
        self.store.update(|p| {
            *p = candidate;
            Ok(())
        })?;
        self.journal.previous = self.journal.project.clone();
        durable_write(&self.path, &json(&self.journal)?)?;
        Ok(())
    }
}
#[derive(Serialize, Deserialize)]
struct Request {
    version: u32,
    competition: Option<String>,
    revision: Option<u64>,
    operation: Option<Operation>,
}
#[derive(Serialize, Deserialize)]
struct Response {
    version: u32,
    competition: String,
    revision: u64,
    project: Option<Project>,
    error: Option<String>,
    retry: bool,
}

// Noise-viestien koko on enintään 65535 tavua. Sovellussanoma pilkotaan rajatuiksi paloiksi.
struct Wire {
    stream: TcpStream,
    crypto: snow::TransportState,
}
fn raw_write(s: &mut TcpStream, data: &[u8]) -> Result<(), CoreError> {
    s.write_all(&(data.len() as u32).to_be_bytes())
        .map_err(io)?;
    s.write_all(data).map_err(io)
}
fn raw_read(s: &mut TcpStream, max: usize) -> Result<Vec<u8>, CoreError> {
    let mut n = [0; 4];
    s.read_exact(&mut n).map_err(io)?;
    let n = u32::from_be_bytes(n) as usize;
    if n > max {
        return Err(err("LAN-viesti on liian suuri"));
    }
    let mut data = vec![0; n];
    s.read_exact(&mut data).map_err(io)?;
    Ok(data)
}
impl Wire {
    fn open(mut stream: TcpStream, code: &str, initiator: bool) -> Result<Self, CoreError> {
        // Windowsin accept perii kuuntelijan ei-estävän tilan. Kättely ja
        // kehystetty read_exact/write_all tarvitsevat estävän yhteyden aikarajoilla.
        stream.set_nonblocking(false).map_err(io)?;
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(io)?;
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .map_err(io)?;
        stream.set_nodelay(true).map_err(io)?;
        let psk = key(code);
        let builder = snow::Builder::new(
            "Noise_NNpsk0_25519_ChaChaPoly_SHA256"
                .parse()
                .map_err(|_| err("LAN-salausasetukset ovat virheelliset"))?,
        )
        .psk(0, &psk)
        .map_err(|_| err("LAN-avain on virheellinen"))?;
        let mut h = if initiator {
            builder.build_initiator()
        } else {
            builder.build_responder()
        }
        .map_err(|_| err("LAN-salauksen alustus epäonnistui"))?;
        let mut buf = vec![0; 65535];
        if initiator {
            let n = h
                .write_message(&[], &mut buf)
                .map_err(|_| err("LAN-kättely epäonnistui"))?;
            raw_write(&mut stream, &buf[..n])?;
            let msg = raw_read(&mut stream, 65535)
                .map_err(|_| err("Parituskoodi ei täsmää tai isäntäkone katkaisi yhteyden"))?;
            h.read_message(&msg, &mut buf)
                .map_err(|_| err("Parituskoodi ei täsmää tai isäntäkone ei vastaa"))?;
        } else {
            let msg = raw_read(&mut stream, 65535)?;
            h.read_message(&msg, &mut buf)
                .map_err(|_| err("Parituskoodi ei täsmää"))?;
            let n = h
                .write_message(&[], &mut buf)
                .map_err(|_| err("LAN-kättely epäonnistui"))?;
            raw_write(&mut stream, &buf[..n])?;
        }
        Ok(Self {
            stream,
            crypto: h
                .into_transport_mode()
                .map_err(|_| err("LAN-salauksen avaus epäonnistui"))?,
        })
    }
    fn send<T: Serialize>(&mut self, value: &T) -> Result<(), CoreError> {
        let data = json(value)?;
        if data.len() > LIMIT {
            return Err(err("Kilpailuaineisto ylittää LAN-viestin kokorajan"));
        }
        let mut payload = (data.len() as u32).to_be_bytes().to_vec();
        payload.extend(data);
        for chunk in payload.chunks(60000) {
            let mut out = vec![0; chunk.len() + 16];
            let n = self
                .crypto
                .write_message(chunk, &mut out)
                .map_err(|_| err("LAN-viestin salaus epäonnistui"))?;
            raw_write(&mut self.stream, &out[..n])?;
        }
        Ok(())
    }
    fn receive<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, CoreError> {
        let mut data = vec![];
        let mut total = None;
        loop {
            let msg = raw_read(&mut self.stream, 65535)?;
            let mut plain = vec![0; msg.len()];
            let n = self
                .crypto
                .read_message(&msg, &mut plain)
                .map_err(|_| err("LAN-viestin todennus epäonnistui"))?;
            data.extend_from_slice(&plain[..n]);
            if data.len() >= 4 && total.is_none() {
                let n = u32::from_be_bytes(data[..4].try_into().unwrap()) as usize;
                if n > LIMIT {
                    return Err(err("LAN-viesti on liian suuri"));
                }
                total = Some(n + 4);
            }
            if let Some(total) = total {
                if data.len() > total {
                    return Err(err("LAN-viestin pituus on virheellinen"));
                }
                if data.len() == total {
                    return serde_json::from_slice(&data[4..])
                        .map_err(|_| err("LAN-viestin muoto on virheellinen"));
                }
            }
        }
    }
}

pub struct Host {
    pub address: SocketAddr,
    pub code: String,
    stop: Arc<AtomicBool>,
    authority: Arc<Mutex<Authority>>,
    _lock: Arc<File>,
}
impl Host {
    pub fn start(path: &Path, bind: SocketAddr) -> Result<Self, CoreError> {
        let canonical = fs::canonicalize(path).map_err(io)?;
        let path = canonical.as_path();
        // Sama paikallinen lukko suojaa LAN-lukon ensimmäistä julkaisemista.
        let startup_lock = crate::storage::lock(path)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(side(path, ".lan-lukko"))
            .map_err(io)?;
        lock.try_lock()
            .map_err(|_| err("Kilpailulla on jo LAN-isäntä tässä koneessa"))?;
        let lock = Arc::new(lock);
        drop(startup_lock);
        let authority = Arc::new(Mutex::new(Authority::open(path)?));
        let listener = TcpListener::bind(bind).map_err(|error|if error.kind()==std::io::ErrorKind::AddrInUse {err(format!("LAN-portti {} on jo käytössä. Sulje toinen jakava AS2026-ikkuna tai valitse toinen isäntäkone.",bind.port()))}else{io(error)})?;
        listener.set_nonblocking(true).map_err(io)?;
        let address = listener.local_addr().map_err(io)?;
        let code = authority.lock().unwrap().journal.code.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let listener_lock = lock.clone();
        let a = authority.clone();
        let c = code.clone();
        let s = stop.clone();
        thread::Builder::new()
            .name("as2026-lan-isanta".into())
            .spawn(move || {
                // Rajattu määrä kättelyjä; huono asiakas ei saa luoda rajattomasti säikeitä.
                let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                while !s.load(Ordering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            if active.load(Ordering::Relaxed) >= 16 {
                                continue;
                            }
                            active.fetch_add(1, Ordering::Relaxed);
                            let active = active.clone();
                            let connection_lock = listener_lock.clone();
                            let a = a.clone();
                            let c = c.clone();
                            thread::spawn(move || {
                                let _lock = connection_lock;
                                let _ = serve(stream, &c, &a);
                                active.fetch_sub(1, Ordering::Relaxed);
                            });
                        }
                        Err(_) => thread::sleep(Duration::from_millis(20)),
                    }
                }
            })
            .map_err(io)?;
        if let Ok(socket) = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)) {
            socket
                .set_read_timeout(Some(Duration::from_millis(200)))
                .map_err(io)?;
            let s = stop.clone();
            let a = authority.clone();
            thread::spawn(move || {
                let mut buf = [0; 128];
                while !s.load(Ordering::Relaxed) {
                    if let Ok((n, peer)) = socket.recv_from(&mut buf)
                        && &buf[..n] == b"AS2026-LAN-1"
                    {
                        let a = a.lock().unwrap();
                        let info = Discovery {
                            name: a.journal.project.kilpailu.nimi.clone(),
                            address: SocketAddr::new(peer.ip(), address.port()),
                            competition: a.journal.competition.clone(),
                        };
                        if let Ok(data) = json(&info) {
                            let _ = socket.send_to(&data, peer);
                        }
                    }
                }
            });
        }
        Ok(Self {
            address,
            code,
            stop,
            authority,
            _lock: lock,
        })
    }
    pub fn connection_address(&self) -> Option<SocketAddr> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.set_broadcast(true).ok()?;
        socket.connect(("255.255.255.255", DISCOVERY_PORT)).ok()?;
        let ip = socket.local_addr().ok()?.ip();
        if ip.is_unspecified() {
            None
        } else {
            Some(SocketAddr::new(ip, self.address.port()))
        }
    }
    pub fn competition_id(&self) -> String {
        self.authority.lock().unwrap().journal.competition.clone()
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        self.stop();
    }
}
fn serve(
    stream: TcpStream,
    code: &str,
    authority: &Arc<Mutex<Authority>>,
) -> Result<(), CoreError> {
    let mut wire = Wire::open(stream, code, false)?;
    let request: Request = wire.receive()?;
    let mut a = authority.lock().map_err(|_| err("LAN-isäntä pysähtyi"))?;
    let mut retry = false;
    let error = if request.version != PROTOCOL {
        Some("LAN-versiot eivät sovi yhteen".into())
    } else if request
        .competition
        .as_ref()
        .is_some_and(|id| id != &a.journal.competition)
    {
        Some("Isännällä on eri kilpailu. Automaattinen uudelleenkohdistus estettiin".into())
    } else if let Some(op) = request.operation {
        let validation = if a.journal.accepted.contains(&op.id) {
            Ok(())
        } else {
            op.apply(&a.journal.project).map(|_| ())
        };
        match validation {
            Err(e) => Some(message(&e)),
            Ok(()) => a.commit(&op).err().map(|e| {
                retry = true;
                e.to_string()
            }),
        }
    } else {
        None
    };
    let response = Response {
        version: PROTOCOL,
        competition: a.journal.competition.clone(),
        revision: a.journal.revision,
        project: if !retry && request.revision != Some(a.journal.revision) {
            Some(a.journal.project.clone())
        } else {
            None
        },
        error,
        retry,
    };
    drop(a);
    wire.send(&response)
}
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Discovery {
    pub name: String,
    pub address: SocketAddr,
    pub competition: String,
}
pub fn discover() -> Result<Vec<Discovery>, CoreError> {
    let socket = UdpSocket::bind("0.0.0.0:0").map_err(io)?;
    socket.set_broadcast(true).map_err(io)?;
    socket
        .set_read_timeout(Some(Duration::from_millis(180)))
        .map_err(io)?;
    let _ = socket.send_to(b"AS2026-LAN-1", ("255.255.255.255", DISCOVERY_PORT));
    let _ = socket.send_to(b"AS2026-LAN-1", ("127.0.0.1", DISCOVERY_PORT));
    let until = Instant::now() + Duration::from_millis(650);
    let mut found = BTreeMap::new();
    let mut buf = [0; 2048];
    while Instant::now() < until {
        if let Ok((n, peer)) = socket.recv_from(&mut buf)
            && let Ok(mut d) = serde_json::from_slice::<Discovery>(&buf[..n])
        {
            d.address.set_ip(peer.ip());
            found.insert(d.competition.clone(), d);
        }
    }
    Ok(found.into_values().collect())
}

#[derive(Clone, Serialize, Deserialize)]
struct Outbox {
    competition: String,
    pending: Vec<Operation>,
}
struct Shared {
    project: Project,
    revision: u64,
    connected: bool,
    notice: String,
    outbox: Outbox,
}
#[derive(Clone)]
pub struct Client {
    shared: Arc<Mutex<Shared>>,
    endpoint: SocketAddr,
    code: String,
    path: PathBuf,
    stop: Arc<AtomicBool>,
    host: bool,
    lifetime: Arc<()>,
    _queue_lock: Arc<File>,
    wake: Arc<(Mutex<bool>, Condvar)>,
}
impl Client {
    pub fn identify(endpoint: SocketAddr, code: &str) -> Result<String, CoreError> {
        let r = rpc(
            endpoint,
            code,
            &Request {
                version: PROTOCOL,
                competition: None,
                revision: None,
                operation: None,
            },
        )?;
        if let Some(e) = r.error {
            return Err(err(e));
        }
        if !valid_id(&r.competition) {
            return Err(err("Isännän kilpailutunnus on virheellinen"));
        }
        Ok(r.competition)
    }
    pub fn join(
        endpoint: SocketAddr,
        code: &str,
        path: &Path,
        host: bool,
    ) -> Result<Self, CoreError> {
        let response = rpc(
            endpoint,
            code,
            &Request {
                version: PROTOCOL,
                competition: None,
                revision: None,
                operation: None,
            },
        )?;
        if let Some(e) = response.error {
            return Err(err(e));
        }
        if response.version != PROTOCOL {
            return Err(err("LAN-versiot eivät sovi yhteen"));
        }
        if !valid_id(&response.competition) {
            return Err(err("Isännän kilpailutunnus on virheellinen"));
        }
        let project = response
            .project
            .ok_or_else(|| err("Isännän kilpailuaineisto puuttuu"))?;
        project.validate()?;
        let queue_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(side(path, ".lukko"))
            .map_err(io)?;
        queue_lock.try_lock().map_err(|_|err("Tämän kilpailun LAN-kopio on jo auki toisessa AS2026-ikkunassa. Sulje aiempi yhteys ja yritä uudelleen."))?;
        let outbox = if path.exists() {
            let q: Outbox = serde_json::from_slice(&fs::read(path).map_err(io)?).map_err(|_| {
                err("Paikallinen LAN-jono on vaurioitunut; säilytä tiedosto palautusta varten")
            })?;
            if q.pending.iter().any(|op| !valid_id(&op.id)) {
                return Err(err("Paikallisen LAN-jonon tapahtumatunnus on virheellinen"));
            }
            if q.competition != response.competition {
                return Err(err("Lähetysjonossa on toisen kilpailun tietoja"));
            }
            q
        } else {
            Outbox {
                competition: response.competition,
                pending: vec![],
            }
        };
        let client = Self {
            shared: Arc::new(Mutex::new(Shared {
                project,
                revision: response.revision,
                connected: true,
                notice: String::new(),
                outbox,
            })),
            endpoint,
            code: code.into(),
            path: path.into(),
            stop: Arc::new(AtomicBool::new(false)),
            host,
            lifetime: Arc::new(()),
            _queue_lock: Arc::new(queue_lock),
            wake: Arc::new((Mutex::new(false), Condvar::new())),
        };
        client.persist()?;
        let worker = client.clone();
        thread::Builder::new()
            .name("as2026-lan-asiakas".into())
            .spawn(move || {
                while !worker.stop.load(Ordering::Relaxed)
                    && Arc::strong_count(&worker.lifetime) > 1
                {
                    worker.tick();
                    let (pending, signal) = &*worker.wake;
                    let wake = pending.lock().unwrap();
                    let (mut wake, _) = signal
                        .wait_timeout_while(wake, Duration::from_millis(180), |wake| !*wake)
                        .unwrap();
                    *wake = false;
                }
            })
            .map_err(io)?;
        Ok(client)
    }
    fn persist(&self) -> Result<(), CoreError> {
        durable_write(&self.path, &json(&self.shared.lock().unwrap().outbox)?)
    }
    pub fn is_host(&self) -> bool {
        self.host
    }
    pub fn endpoint(&self) -> SocketAddr {
        self.endpoint
    }
    pub fn competition_id(&self) -> String {
        self.shared.lock().unwrap().outbox.competition.clone()
    }
    pub fn ready(&self) -> bool {
        let s = self.shared.lock().unwrap();
        s.connected && s.outbox.pending.is_empty()
    }
    pub fn pending(&self) -> usize {
        self.shared.lock().unwrap().outbox.pending.len()
    }
    pub(crate) fn cache_warning(&self, error: &CoreError) {
        self.shared.lock().unwrap().notice = format!(
            "Muutos on tallessa isännällä tai lähetysjonossa, mutta tämän koneen kilpailukopiota ei voitu tallentaa: {}",
            message(error)
        );
    }
    pub fn acknowledge_notice(&self) {
        self.shared.lock().unwrap().notice.clear();
    }
    pub fn notice(&self) -> String {
        self.shared.lock().unwrap().notice.clone()
    }
    pub fn status(&self) -> String {
        let s = self.shared.lock().unwrap();
        if !s.connected {
            return if s.outbox.pending.is_empty() {
                "Yhteys katkennut · yhdistetään".into()
            } else {
                format!("Yhteys katkennut · {} jonossa", s.outbox.pending.len())
            };
        }
        let state = if self.host { "Isäntä" } else { "Yhdistetty" };
        if !s.outbox.pending.is_empty() {
            format!("{state} · {} jonossa", s.outbox.pending.len())
        } else if !s.notice.is_empty() {
            format!("{state} · tarkistettavaa")
        } else {
            format!("{state} · ajan tasalla")
        }
    }
    /// Vienti ja tulostus tarvitsevat isännältä vahvistetun tilannekuvan.
    pub fn refresh(&self) -> Result<(), CoreError> {
        let id = {
            let s = self.shared.lock().unwrap();
            if !s.outbox.pending.is_empty() {
                return Err(err(
                    "Odota LAN-jonon kuittausta ennen vientiä tai tulostusta",
                ));
            }
            s.outbox.competition.clone()
        };
        let response = rpc(
            self.endpoint,
            &self.code,
            &Request {
                version: PROTOCOL,
                competition: Some(id.clone()),
                revision: None,
                operation: None,
            },
        );
        let mut s = self.shared.lock().unwrap();
        let r = match response {
            Ok(r) => r,
            Err(e) => {
                s.connected = false;
                return Err(e);
            }
        };
        if r.competition != id || r.version != PROTOCOL {
            return Err(err("Isäntäkoneen kilpailu tai versio vaihtui"));
        }
        if let Some(e) = r.error {
            return Err(err(e));
        }
        let p = r
            .project
            .ok_or_else(|| err("Isännän vahvistama kilpailu puuttuu"))?;
        p.validate()?;
        if r.revision >= s.revision {
            s.project = p;
            s.revision = r.revision;
        }
        s.connected = true;
        Ok(())
    }
    pub fn snapshot(&self) -> Result<Project, CoreError> {
        let s = self.shared.lock().unwrap();
        let mut p = s.project.clone();
        for op in &s.outbox.pending {
            if !op.reads.is_empty() {
                for read in &op.reads {
                    append_read(&mut p, read.clone())?;
                }
            }
        }
        crate::corrections::invalidate_input_changes(&s.project, &mut p);
        Ok(p)
    }
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
        self.wake_worker();
    }
    fn wake_worker(&self) {
        let (pending, signal) = &*self.wake;
        *pending.lock().unwrap() = true;
        signal.notify_one();
    }
    fn tick(&self) {
        let request = {
            let s = self.shared.lock().unwrap();
            Request {
                version: PROTOCOL,
                competition: Some(s.outbox.competition.clone()),
                revision: Some(s.revision),
                operation: s.outbox.pending.first().cloned(),
            }
        };
        let response = rpc(self.endpoint, &self.code, &request);
        let mut s = self.shared.lock().unwrap();
        match response {
            Ok(r) => {
                let pending_before = s.outbox.pending.len();
                if r.competition != s.outbox.competition || r.version != PROTOCOL {
                    s.connected = false;
                    s.notice = "Isännän kilpailu tai protokolla vaihtui".into();
                    return;
                }
                s.connected = true;
                if let Some(e) = r.error {
                    s.notice = e;
                    if !r.retry
                        && let Some(op) = request.operation
                    {
                        let rejected = side(&self.path, &format!(".hylatty-{}.json", op.id));
                        if durable_write(
                            &rejected,
                            &json(&serde_json::json!({"tapahtuma":op,"syy":s.notice}))
                                .unwrap_or_default(),
                        )
                        .is_ok()
                        {
                            s.outbox.pending.retain(|p| p.id != op.id);
                        }
                    }
                } else if let Some(op) = request.operation {
                    s.outbox.pending.retain(|p| p.id != op.id);
                }
                if let Some(p) = r.project {
                    if r.revision < s.revision {
                        return;
                    }
                    if p.validate().is_err() {
                        s.connected = false;
                        s.notice = "Isännän aineisto ei läpäissyt tarkistusta".into();
                        return;
                    }
                    s.project = p;
                    s.revision = r.revision;
                }
                // Tilakysely ei muuta lähetysjonoa. Turha sync_all jokaisella
                // kierroksella hidasti Windowsissa myös seuraavan muutoksen kuittausta.
                if s.outbox.pending.len() != pending_before {
                    let _ = durable_write(&self.path, &json(&s.outbox).unwrap_or_default());
                }
            }
            Err(_) => {
                s.connected = false;
            }
        }
    }
    pub fn submit(
        &self,
        base: &Project,
        candidate: &Project,
        pinned: Option<&Project>,
    ) -> Result<Project, CoreError> {
        let op = Operation::create(base, candidate, pinned)?;
        if op.changes.is_empty() && op.reads.is_empty() {
            return self.snapshot();
        }
        let reads = !op.reads.is_empty();
        {
            let mut s = self.shared.lock().unwrap();
            if !reads && !s.connected {
                return Err(err(
                    "Yhteys isäntään on katkennut. Emit-luvut tallentuvat jonoon; muu muokkaus jatkuu yhteyden palattua",
                ));
            }
            if !reads && !s.outbox.pending.is_empty() {
                return Err(err(
                    "Odota LAN-jonon valmistumista ennen muuta muokkausta. Emit-lukuja voi jatkaa",
                ));
            }
            s.notice.clear();
            s.outbox.pending.push(op.clone());
            if let Err(e) = durable_write(&self.path, &json(&s.outbox)?) {
                s.outbox.pending.pop();
                return Err(e);
            }
        }
        if reads && !self.shared.lock().unwrap().connected {
            return self.snapshot();
        }
        self.wake_worker();
        // Tavallisen muokkauksen kuittaus on lyhyt. Epäselvä toimitus jää pysyvään jonoon.
        let until = Instant::now() + Duration::from_millis(750);
        while Instant::now() < until {
            let s = self.shared.lock().unwrap();
            if !s.outbox.pending.iter().any(|p| p.id == op.id) {
                if !s.notice.is_empty() {
                    return Err(err(s.notice.clone()));
                }
                return Ok(s.project.clone());
            }
            drop(s);
            thread::sleep(Duration::from_millis(10));
        }
        if reads {
            return self.snapshot();
        }
        Err(err(
            "Muutos odottaa isännän kuittausta. Syöte säilyy; älä tee samaa muutosta uudelleen ennen jonon valmistumista",
        ))
    }
}
fn rpc(endpoint: SocketAddr, code: &str, request: &Request) -> Result<Response, CoreError> {
    let stream = TcpStream::connect_timeout(&endpoint, Duration::from_millis(600)).map_err(|_|err(format!("Isäntäkoneeseen {endpoint} ei saada yhteyttä. Tarkista osoite, lähiverkko ja palomuuri.")))?;
    let mut wire = Wire::open(stream, code, true)?;
    wire.send(request)?;
    wire.receive()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample() -> Project {
        let mut p =
            Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
        let mut d = p.kilpailijat[0].clone();
        d.numero = 8;
        d.emit = 234567;
        p.kilpailijat.push(d);
        p
    }
    fn folder() -> PathBuf {
        let p = std::env::temp_dir().join(format!("as2026-lan-koe-{}", random_id().unwrap()));
        fs::create_dir(&p).unwrap();
        p
    }
    fn wait(mut done: impl FnMut() -> bool) {
        let until = Instant::now() + Duration::from_secs(8);
        while !done() {
            assert!(Instant::now() < until, "LAN-koe ei valmistunut ajoissa");
            thread::sleep(Duration::from_millis(25));
        }
    }
    #[test]
    fn noise_normalizes_nonblocking_sockets_and_keeps_final_response() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            // Pakota Windowsin periytyvä tila myös Linuxissa. Ennen korjausta
            // read_exact epäonnistuu heti, kun asiakas ei ole vielä lähettänyt.
            stream.set_nonblocking(true).unwrap();
            let mut wire = Wire::open(stream, "KOKEILU23456", false).unwrap();
            let request: String = wire.receive().unwrap();
            assert_eq!(request.len(), 180_000);
            wire.send(&request).unwrap();
            // Pudota palvelinpää heti viimeisen vastauksen jälkeen kuten RPC:ssä.
        });
        let stream = TcpStream::connect(address).unwrap();
        thread::sleep(Duration::from_millis(60));
        stream.set_nonblocking(true).unwrap();
        let mut wire = Wire::open(stream, "KOKEILU23456", true).unwrap();
        let request = "koe".repeat(60_000);
        wire.send(&request).unwrap();
        let response: String = wire.receive().unwrap();
        assert_eq!(response, request);
        assert!(
            wire.receive::<String>().is_err(),
            "Suljettu yhteys ei tuota näennäistä vastausta"
        );
        server.join().unwrap();
    }
    #[test]
    fn unchanged_polls_do_not_rewrite_the_durable_outbox() {
        let root = folder();
        let path = root.join("kilpailu.toml");
        ProjectStore::create(&path, sample()).unwrap();
        let host = Host::start(&path, "127.0.0.1:0".parse().unwrap()).unwrap();
        let queue = root.join("jono.json");
        let client = Client::join(host.address, &host.code, &queue, false).unwrap();
        client.stop();
        wait(|| Arc::strong_count(&client.lifetime) == 1);
        let modified = fs::metadata(&queue).unwrap().modified().unwrap();
        let bytes = fs::read(&queue).unwrap();
        thread::sleep(Duration::from_millis(60));
        client.tick();
        client.tick();
        assert_eq!(fs::read(&queue).unwrap(), bytes);
        assert_eq!(
            fs::metadata(&queue).unwrap().modified().unwrap(),
            modified,
            "Muuttumaton tilakysely ei tee turhaa pysyvää levykirjoitusta"
        );
    }
    #[test]
    fn independent_edits_merge_but_same_driver_conflicts() {
        let p = sample();
        let mut a = p.clone();
        a.kilpailijat[0].auto = "Auto A".into();
        let mut b = p.clone();
        b.kilpailijat[1].auto = "Auto B".into();
        let a = Operation::create(&p, &a, None).unwrap();
        let b = Operation::create(&p, &b, None).unwrap();
        let current = a.apply(&p).unwrap();
        let merged = b.apply(&current).unwrap();
        assert_eq!(merged.kilpailijat[0].auto, "Auto A");
        assert_eq!(merged.kilpailijat[1].auto, "Auto B");
        assert!(a.apply(&merged).is_err());
    }
    #[test]
    fn unrelated_bc_edit_does_not_block_other_driver() {
        let mut p = sample();
        p.beginners_cup = Some(Box::new(crate::beginners::BeginnersCup {
            nimi: "BC-koe".into(),
            tunnus: "BC".into(),
            lahderata: 1,
            lahdeasemat: p.radat[0].asemat.clone(),
            mukana_asemat: vec![1, 2, 3, 4],
        }));
        p.bc_osallistujat.push(crate::beginners::Entry {
            numero: 7,
            osallistuu: true,
            ..Default::default()
        });
        let mut bc = p.clone();
        bc.bc_osallistujat[0].lisavirheet.muut_virhepisteet = 10;
        let mut other = p.clone();
        other.kilpailijat[1].auto = "Toisen kilpailijan auto".into();
        let current = Operation::create(&p, &bc, None).unwrap().apply(&p).unwrap();
        let merged = Operation::create(&p, &other, None)
            .unwrap()
            .apply(&current)
            .unwrap();
        assert_eq!(merged.bc_osallistujat[0].lisavirheet.muut_virhepisteet, 10);
        assert_eq!(merged.kilpailijat[1].auto, "Toisen kilpailijan auto");
    }
    #[test]
    fn pinned_editor_cannot_overwrite_remote_changes() {
        let p = sample();
        let mut remote = p.clone();
        remote.kilpailijat[0].auto = "Etämuutos".into();
        let mut local = remote.clone();
        local.kilpailijat[0].kuljettaja = "Oma keskeneräinen nimi".into();
        assert!(
            Operation::create(&remote, &local, Some(&p))
                .unwrap()
                .apply(&remote)
                .is_err()
        );
    }
    #[test]
    fn optional_sections_and_correction_history_are_not_lost() {
        let p = sample();
        let mut a = p.clone();
        a.yhdistetyt_luokat.push(crate::native::CombinedClass {
            nimi: "Yhteinen".into(),
            luokat: vec!["A".into()],
            nousupisteet: 0,
            sarjapistetapa: 0,
        });
        for number in [99, 2] {
            let mut driver = a.kilpailijat[0].clone();
            driver.numero = number;
            driver.emit = 900000 + number;
            a.kilpailijat.push(driver);
        }
        let op = Operation::create(&p, &a, None).unwrap();
        assert_eq!(op.apply(&p).unwrap(), a);
    }
    #[test]
    fn real_tcp_clients_concurrent_reads_replay_and_host_restart() {
        let root = folder();
        let path = root.join("kilpailu.toml");
        let project = sample();
        ProjectStore::create(&path, project.clone()).unwrap();
        let host = Host::start(&path, "127.0.0.1:0".parse().unwrap()).unwrap();
        let address = host.address;
        let code = host.code.clone();
        let a = Client::join(address, &code, &root.join("a.json"), false).unwrap();
        let b = Client::join(address, &code, &root.join("b.json"), false).unwrap();
        assert!(Client::join(address, &code, &root.join("a.json"), false).is_err());
        let mut unrelated = ProjectStore::open(&path).unwrap();
        assert!(
            unrelated
                .update(|p| {
                    p.kilpailu.nimi = "Ohittava paikallisikkuna".into();
                    Ok(())
                })
                .is_err()
        );
        assert!(Client::join(address, "VAARAPARITUS", &root.join("wrong.json"), false).is_err());
        let mut pa = project.clone();
        let mut ra = project.leimaluvut[0].clone();
        ra.emit = 345678;
        ra.luettu_unix_ms = Some(100);
        pa.record_readout(ra).unwrap();
        let mut pb = project.clone();
        let mut rb = project.leimaluvut[0].clone();
        rb.emit = 456789;
        rb.luettu_unix_ms = Some(101);
        pb.record_readout(rb).unwrap();
        let aa = a.clone();
        let bb = b.clone();
        let pp = project.clone();
        let ta = thread::spawn(move || aa.submit(&pp, &pa, None).unwrap());
        let pp = project.clone();
        let tb = thread::spawn(move || bb.submit(&pp, &pb, None).unwrap());
        ta.join().unwrap();
        tb.join().unwrap();
        wait(|| {
            a.pending() == 0
                && b.pending() == 0
                && a.snapshot().unwrap().leimaluvut.len() == 3
                && b.snapshot().unwrap().leimaluvut.len() == 3
        });
        // Kadonnut kuittaus: täsmälleen sama verkko-operaatio toimitetaan kahdesti.
        let base = a.snapshot().unwrap();
        let mut next = base.clone();
        let mut read = base.leimaluvut[0].clone();
        read.emit = 567890;
        next.record_readout(read).unwrap();
        let op = Operation::create(&base, &next, None).unwrap();
        let request = Request {
            version: PROTOCOL,
            competition: Some(a.competition_id()),
            revision: None,
            operation: Some(op),
        };
        assert!(rpc(address, &code, &request).unwrap().error.is_none());
        assert!(rpc(address, &code, &request).unwrap().error.is_none());
        wait(|| a.snapshot().unwrap().leimaluvut.len() == 4);
        // Isäntä katoaa: lukema jonoon, asiakaskin suljetaan, jono palautetaan levyltä.
        host.stop();
        drop(host);
        thread::sleep(Duration::from_millis(250));
        assert!(a.refresh().is_err());
        let base = a.snapshot().unwrap();
        let mut next = base.clone();
        let mut read = base.leimaluvut[0].clone();
        read.emit = 678901;
        next.record_readout(read).unwrap();
        a.submit(&base, &next, None).unwrap();
        assert_eq!(a.pending(), 1);
        a.stop();
        b.stop();
        drop(a);
        drop(b);
        thread::sleep(Duration::from_millis(250));
        let host = Host::start(&path, address).unwrap();
        assert_eq!(host.code, code);
        let restored = Client::join(address, &code, &root.join("a.json"), false).unwrap();
        wait(|| restored.pending() == 0 && restored.snapshot().unwrap().leimaluvut.len() == 5);
        assert_eq!(
            ProjectStore::open(&path)
                .unwrap()
                .project()
                .leimaluvut
                .len(),
            5
        );
        restored.stop();
        host.stop();
        drop(host);
        thread::sleep(Duration::from_millis(250));
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn crash_between_journal_and_competition_recovers_once() {
        let root = folder();
        let path = root.join("kilpailu.toml");
        let p = sample();
        ProjectStore::create(&path, p.clone()).unwrap();
        let a = Authority::open(&path).unwrap();
        let mut j = a.journal.clone();
        let mut candidate = p.clone();
        candidate.kilpailijat[0].auto = "Ennen kaatumista".into();
        let op = Operation::create(&p, &candidate, None).unwrap();
        j.project = candidate.clone();
        j.accepted.insert(op.id.clone());
        j.revision += 1;
        durable_write(&a.path, &json(&j).unwrap()).unwrap();
        drop(a);
        let mut reopened = Authority::open(&path).unwrap();
        reopened.commit(&op).unwrap();
        assert_eq!(reopened.store.project(), &candidate);
        assert_eq!(reopened.journal.revision, 1);
        drop(reopened);
        fs::remove_dir_all(root).unwrap();
    }
}

/// Vain toimitsijan omat onnistuneet muutokset siirtävät avoimen lomakkeen pohjaa.
/// Toisen kilpailijan automaattinen käsittely ei hyväksy samalla etämuutoksia.
pub(crate) fn advance_basis(
    before: &Project,
    candidate: &Project,
    accepted: &Project,
    basis: &mut Option<Project>,
) -> Result<(), CoreError> {
    let Some(pinned) = basis.as_ref() else {
        return Ok(());
    };
    let op = Operation::create(before, candidate, None)?;
    if !op.reads.is_empty() {
        return Ok(());
    }
    let mut value =
        serde_json::to_value(pinned).map_err(|_| err("Muokkauspohjan päivitys epäonnistui"))?;
    let fresh =
        serde_json::to_value(accepted).map_err(|_| err("Muokkauspohjan päivitys epäonnistui"))?;
    for change in op.changes {
        if let Some(id) = change.identity {
            if value.get(&change.section).is_none() {
                value[&change.section] = Value::Array(vec![]);
            }
            let array = value
                .get_mut(&change.section)
                .and_then(Value::as_array_mut)
                .ok_or_else(|| err("Muokkauspohjan tietueryhmä puuttuu"))?;
            let pos = array
                .iter()
                .position(|v| identity(&change.section, v).as_ref() == Some(&id));
            let new = entries(&change.section, fresh.get(&change.section)).remove(&id);
            match (pos, new) {
                (Some(i), Some(v)) => array[i] = v,
                (Some(i), None) => {
                    array.remove(i);
                }
                (None, Some(v)) => array.push(v),
                _ => {}
            }
        } else if let Some(v) = fresh.get(&change.section) {
            value[&change.section] = v.clone();
        } else {
            value.as_object_mut().unwrap().remove(&change.section);
        }
    }
    *basis = Some(
        serde_json::from_value(value).map_err(|_| err("Muokkauspohjan päivitys epäonnistui"))?,
    );
    Ok(())
}

#[cfg(test)]
mod workflow_tests {
    use super::*;
    use crate::{
        beginners::{BeginnersCup, Entry},
        emit_corrections::{Action, Request},
        workspace::Workspace,
    };
    #[test]
    fn two_workspaces_share_manual_clocks_approvals_bc_and_emit_corrections() {
        let dir =
            std::env::temp_dir().join(format!("as2026-lan-tuloskoe-{}", random_id().unwrap()));
        fs::create_dir(&dir).unwrap();
        let mut p =
            Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
        p.beginners_cup = Some(Box::new(BeginnersCup {
            nimi: "Synteettinen Beginners Cup".into(),
            tunnus: "BC".into(),
            lahderata: 1,
            lahdeasemat: p.radat[0].asemat.clone(),
            mukana_asemat: vec![1, 2, 3, 4],
        }));
        p.bc_osallistujat.push(Entry {
            numero: 7,
            osallistuu: true,
            ..Default::default()
        });
        p.yhdistetyt_luokat.push(crate::native::CombinedClass {
            nimi: "Yhteinen".into(),
            luokat: vec!["A".into()],
            nousupisteet: 0,
            sarjapistetapa: 0,
        });
        let mut bc_change = p.clone();
        bc_change.bc_osallistujat[0].lisavirheet.muut_virhepisteet = 1;
        let mut shared_change = p.clone();
        shared_change.kilpailijat[0].emit_korjaus_sekuntia += 1;
        assert!(
            Operation::create(&p, &bc_change, None)
                .unwrap()
                .apply(&shared_change)
                .is_err()
        );
        let host_path = dir.join("isanta.toml");
        ProjectStore::create(&host_path, p.clone()).unwrap();
        let host = Host::start(&host_path, "127.0.0.1:0".parse().unwrap()).unwrap();
        let a = Client::join(host.address, &host.code, &dir.join("a-jono.json"), false).unwrap();
        let b = Client::join(host.address, &host.code, &dir.join("b-jono.json"), false).unwrap();
        let ap = dir.join("a.toml");
        let bp = dir.join("b.toml");
        ProjectStore::create(&ap, p.clone()).unwrap();
        ProjectStore::create(&bp, p.clone()).unwrap();
        let mut wa = Workspace::open(&ap).unwrap();
        let mut wb = Workspace::open(&bp).unwrap();
        wa.attach_lan(a.clone()).unwrap();
        wb.attach_lan(b.clone()).unwrap();
        wa.pin_lan_basis();
        wa.review_results(7).unwrap();
        let finish = wa
            .result_snapshot(7)
            .unwrap()
            .unwrap()
            .rivit
            .iter()
            .find(|r| r.station_type == "M")
            .unwrap()
            .output_index;
        wa.complete_result_entry(7, &[(finish, [12, 10, 0])], 0, 25)
            .unwrap();
        assert!(wa.result_snapshot(7).unwrap().unwrap().hyvaksytty);
        let until = Instant::now() + Duration::from_secs(4);
        while wb.project() != wa.project() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(30));
            wb.poll_lan().unwrap();
        }
        wb.set_bc_view(true).unwrap();
        wb.pin_lan_basis();
        wb.review_results(7).unwrap();
        wb.complete_result_entry(7, &[(finish, [12, 10, 0])], 0, 30)
            .unwrap();
        assert!(wb.result_snapshot(7).unwrap().unwrap().hyvaksytty);
        wb.set_bc_view(false).unwrap();
        wb.pin_lan_basis();
        let raw = wb.project().unwrap().leimaluvut.clone();
        wb.correct_emit_row(
            7,
            &Request {
                row: 2,
                source_row: None,
                action: Action::Edit,
                code: 31,
                elapsed_seconds: 420,
                reason: "LAN-kokeen synteettinen varakello".into(),
            },
        )
        .unwrap();
        let until = Instant::now() + Duration::from_secs(4);
        while wa.project() != wb.project() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(30));
            wa.poll_lan().unwrap();
        }
        assert_eq!(wa.project().unwrap().leimaluvut, raw);
        assert_eq!(wa.project().unwrap().emit_korjaukset.len(), 1);
        assert!(!wa.result_snapshot(7).unwrap().unwrap().hyvaksytty);
        assert_eq!(wa.result_snapshot(7).unwrap().unwrap().aikavirheet, 0);
        wa.set_bc_view(true).unwrap();
        assert!(!wa.result_snapshot(7).unwrap().unwrap().hyvaksytty);
        wa.set_bc_view(false).unwrap();
        assert!(crate::printing::competitor(wa.project().unwrap(), 7, None).is_err());
        wa.pin_lan_basis();
        wa.complete_result_entry(7, &[(finish, [12, 10, 0])], 0, 25)
            .unwrap();
        let until = Instant::now() + Duration::from_secs(4);
        while wb.project() != wa.project() {
            assert!(Instant::now() < until);
            thread::sleep(Duration::from_millis(30));
            wb.poll_lan().unwrap();
        }
        assert_eq!(
            crate::printing::competitor(wa.project().unwrap(), 7, None)
                .unwrap()
                .pages
                .len(),
            crate::printing::competitor(wb.project().unwrap(), 7, None)
                .unwrap()
                .pages
                .len()
        );
        let before = wa.project().unwrap().clone();
        let mut changed = before.clone();
        let mut read = changed.leimaluvut[0].clone();
        read.leimat[1].aika_sekuntia += 10;
        changed.record_readout(read).unwrap();
        let merged = Operation::create(&before, &changed, None)
            .unwrap()
            .apply(&before)
            .unwrap();
        assert!(merged.leimaluvut.starts_with(&before.leimaluvut));
        assert_eq!(merged.leimaluvut.len(), before.leimaluvut.len() + 1);
        assert!(
            !merged.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty
        );
        let mut stale = merged.clone();
        stale.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        assert!(
            Operation::create(&merged, &stale, None)
                .unwrap()
                .apply(&merged)
                .is_err()
        );
        // Välimuistin levyvika ei saa muuttaa jo jonotettua Emit-lukua uudeksi lukuyritykseksi.
        fs::remove_file(&ap).unwrap();
        fs::create_dir(&ap).unwrap();
        let frame =
            crate::emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw"))
                .unwrap();
        wa.record_frame(&frame, crate::ClockTime::new(12, 12, 0).unwrap())
            .unwrap();
        assert!(a.notice().contains("kilpailukopiota"));
        assert_eq!(
            ProjectStore::open(&host_path)
                .unwrap()
                .project()
                .leimaluvut
                .len(),
            raw.len() + 1
        );
        a.stop();
        b.stop();
        host.stop();
        drop(host);
        thread::sleep(Duration::from_millis(250));
        assert_eq!(Workspace::open(&host_path).unwrap().project(), wa.project());
        fs::remove_dir_all(dir).unwrap();
    }
}
