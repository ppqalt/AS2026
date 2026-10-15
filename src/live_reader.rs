//! GUI:sta riippumaton, jatkuva Emit-lukija taustasäikeessä.
//!
//! Säie ei muuta kilpailua eikä kirjoita tiedostoja. Vastaanottaja tallentaa kehyksen
//! yhteisen kilpailumallin kautta ja ilmoittaa käyttäjälle tallennuksen tuloksen.

use crate::{ClockTime, emit::EmitFrame, reader::CardReader};
use chrono::Timelike;
use std::{
    io::Read,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::Duration,
};

/// Täysi jono keskeyttää lisälukemisen, kunnes vastaanottaja ehtii käsitellä tapahtumat.
/// Tämä rajaa sovelluksen muistin; käyttöjärjestelmän sarjapuskurin koko on silti rajallinen.
pub const EVENT_QUEUE_CAPACITY: usize = 32;
const CONTROL_WAIT: Duration = Duration::from_millis(5);

#[derive(Debug)]
pub enum ReaderEvent {
    Connected {
        port: String,
    },
    Frame {
        frame: Box<EmitFrame>,
        read_clock: ClockTime,
    },
    Error(String),
    Stopped,
}

/// Yksi avoin sarjaportti ja sen hallittu pysäytys.
///
/// Kutsuja säilyttää vastaanottimen, käsittelee sen tapahtumat pääsäikeessään ja
/// kutsuu `stop` ennen kilpailun vaihtamista. Pysäytyksen jälkeen myös jonoon jääneet
/// tapahtumat sekä `stop`-paluuarvon mahdollinen tapahtuma on käsiteltävä.
pub struct ReaderWorker {
    stop_requested: Arc<AtomicBool>,
    handle: Option<JoinHandle<Vec<ReaderEvent>>>,
}

impl ReaderWorker {
    /// Tunnistus ja avaus tehdään säikeessä. Avausvirhe tulee `Error`-tapahtumana.
    pub fn start(requested: Option<String>) -> Result<(Self, Receiver<ReaderEvent>), String> {
        Self::spawn(move || {
            let port = crate::reader::port_name(requested.as_deref())?;
            let reader = CardReader::open(&port)?;
            Ok((port, reader))
        })
    }

    /// Sama lukupolku testivirralle; ei avaa eikä etsi fyysisiä sarjaportteja.
    /// Virran `read`-kutsun on palattava viimeistään 250 millisekunnissa, kuten
    /// varsinaisen `CardReader::open`-lukijan, jotta pysäytys pysyy nopeana.
    pub fn from_reader<R: Read + Send + 'static>(
        port: String,
        device: R,
    ) -> Result<(Self, Receiver<ReaderEvent>), String> {
        Self::spawn(move || Ok((port, CardReader::from_reader(device))))
    }

    fn spawn<R, F>(open: F) -> Result<(Self, Receiver<ReaderEvent>), String>
    where
        R: Read + Send + 'static,
        F: FnOnce() -> Result<(String, CardReader<R>), String> + Send + 'static,
    {
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let stop_requested = Arc::new(AtomicBool::new(false));
        let worker_stop = stop_requested.clone();
        let handle = thread::Builder::new()
            .name("as2026-emit".into())
            .spawn(move || run(open, &sender, &worker_stop))
            .map_err(|error| {
                format!(
                    "Emit-lukijan taustasäikeen käynnistys epäonnistui: {}",
                    crate::io_message(&error)
                )
            })?;
        Ok((
            Self {
                stop_requested,
                handle: Some(handle),
            },
            receiver,
        ))
    }

    /// Lopettaa lukemisen ja odottaa säikeen päättymisen. Toistaminen on turvallista.
    ///
    /// Jos jono oli täynnä, palauttaa jo luetut tapahtumat, joita ei voitu siirtää
    /// vastaanottimelle. Mukana ovat myös saman laiteluvun valmiit puskuroidut kehykset.
    /// Käsittele ensin vastaanottimen jono ja sitten tämä lista, jotta järjestys
    /// säilyy eikä luettu kehys häviä huomaamatta.
    /// Fyysinen luku voi odottaa nykyisen, enintään 250 ms:n laiteluvun loppuun.
    pub fn stop(&mut self) -> Result<Vec<ReaderEvent>, String> {
        self.stop_requested.store(true, Ordering::Release);
        let Some(handle) = self.handle.take() else {
            return Ok(Vec::new());
        };
        handle.thread().unpark();
        handle
            .join()
            .map_err(|_| "Emit-lukijan taustasäie päättyi odottamattomasti".into())
    }
}

impl Drop for ReaderWorker {
    fn drop(&mut self) {
        // Tavallinen sulkeminen käyttää stop-paluuarvoa. Varapolku kertoo myös
        // ohjelmointivirheestä, jossa omistaja unohti käsitellä viimeisen kehyksen.
        match self.stop() {
            Ok(events) => {
                for event in events {
                    match event {
                        ReaderEvent::Frame { .. } => eprintln!(
                            "Emit-lukija suljettiin ennen viimeisen luetun kehyksen käsittelyä. Kehystä ei tallennettu."
                        ),
                        ReaderEvent::Error(error) => eprintln!("{error}"),
                        _ => {}
                    }
                }
            }
            Err(error) => eprintln!("{error}"),
        }
    }
}

fn run<R, F>(
    open: F,
    sender: &SyncSender<ReaderEvent>,
    stop_requested: &AtomicBool,
) -> Vec<ReaderEvent>
where
    R: Read,
    F: FnOnce() -> Result<(String, CardReader<R>), String>,
{
    let (port, mut reader) = match open() {
        Ok(reader) => reader,
        Err(error) => {
            let mut pending = Vec::new();
            if let Err(pending) = send(sender, stop_requested, ReaderEvent::Error(error)) {
                return vec![pending, ReaderEvent::Stopped];
            }
            dispatch_or_collect(sender, stop_requested, ReaderEvent::Stopped, &mut pending);
            return pending;
        }
    };
    if let Err(pending) = send(sender, stop_requested, ReaderEvent::Connected { port }) {
        return vec![pending, ReaderEvent::Stopped];
    }
    while !stop_requested.load(Ordering::Acquire) {
        let event = match reader.poll() {
            Ok(Some(frame)) => frame_event(frame),
            Ok(None) => {
                // Myös heti tyhjänä palaava testivirta tai ajuri saa levätä.
                thread::park_timeout(CONTROL_WAIT);
                continue;
            }
            Err(error) => {
                if let Err(pending) = send(sender, stop_requested, ReaderEvent::Error(error)) {
                    return finish(&mut reader, sender, stop_requested, Some(pending));
                }
                break;
            }
        };
        if let Err(pending) = send(sender, stop_requested, event) {
            return finish(&mut reader, sender, stop_requested, Some(pending));
        }
    }
    finish(&mut reader, sender, stop_requested, None)
}

fn frame_event(frame: EmitFrame) -> ReaderEvent {
    let now = chrono::Local::now();
    ReaderEvent::Frame {
        frame: Box::new(frame),
        read_clock: ClockTime {
            hour: now.hour(),
            minute: now.minute(),
            second: now.second(),
        },
    }
}

fn finish<R: Read>(
    reader: &mut CardReader<R>,
    sender: &SyncSender<ReaderEvent>,
    stop_requested: &AtomicBool,
    pending: Option<ReaderEvent>,
) -> Vec<ReaderEvent> {
    let mut pending: Vec<_> = pending.into_iter().collect();
    while let Some(frame) = reader.buffered_frame() {
        dispatch_or_collect(sender, stop_requested, frame_event(frame), &mut pending);
    }
    dispatch_or_collect(sender, stop_requested, ReaderEvent::Stopped, &mut pending);
    pending
}

fn dispatch_or_collect(
    sender: &SyncSender<ReaderEvent>,
    stop_requested: &AtomicBool,
    event: ReaderEvent,
    pending: &mut Vec<ReaderEvent>,
) {
    // Kun yksi tapahtuma jäi jonon ulkopuolelle, myöhemmätkään eivät saa ohittaa sitä.
    if !pending.is_empty() {
        pending.push(event);
    } else if let Err(event) = send(sender, stop_requested, event) {
        pending.push(event);
    }
}

fn send(
    sender: &SyncSender<ReaderEvent>,
    stop_requested: &AtomicBool,
    mut event: ReaderEvent,
) -> Result<(), ReaderEvent> {
    loop {
        // Ensimmäinen yritys tehdään myös pysäytyksessä, jotta viimeinen kehys
        // pääsee vapaaseen jonopaikkaan. Täysi jono ei saa estää pysäytystä.
        match sender.try_send(event) {
            Ok(()) => return Ok(()),
            Err(TrySendError::Disconnected(pending)) => return Err(pending),
            Err(TrySendError::Full(pending)) => event = pending,
        }
        if stop_requested.load(Ordering::Acquire) {
            return Err(event);
        }
        thread::park_timeout(CONTROL_WAIT);
    }
}
