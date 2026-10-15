//! Emit-kortin luku suoraan sarjaportista (Linux ja Windows).
//! Laitteesta luetaan vain tavudataa; asetuksia tai kilpailutiedostoja ei kirjoiteta.

use crate::emit::{self, EmitFrame, FRAME_LENGTH};
use std::{
    io::{self, Read},
    time::{Duration, Instant},
};

const FRAME_START: [u8; 2] = [0x20, 0x20];
const READ_WAIT: Duration = Duration::from_secs(30);

/// Sarjavirta voi alkaa mistä tahansa kehyksen kohdasta tai tulla pienissä osissa.
#[derive(Default)]
pub struct FrameScanner {
    buffer: Vec<u8>,
}

impl FrameScanner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, bytes: &[u8]) -> Option<EmitFrame> {
        self.buffer.extend_from_slice(bytes);
        self.next_frame()
    }

    /// Palauttaa seuraavan jo puskuroidun kehyksen ilman uutta laitelukua.
    /// Sisäinen synkkatavupari voi kuulua myös hyötydataan, joten se ei katkaise kehystä.
    /// Varmennettua tarkistussummaa ei tunneta: kaikkia tavuhäviöitä ei voida tunnistaa.
    pub fn next_frame(&mut self) -> Option<EmitFrame> {
        loop {
            let start = self
                .buffer
                .windows(2)
                .position(|pair| pair == FRAME_START.as_slice());
            match start {
                Some(offset) if offset > 0 => {
                    self.buffer.drain(..offset);
                }
                None => {
                    // Jos viimeinen tavu on 0x20, seuraava pala voi aloittaa kehyksen.
                    let keep = usize::from(self.buffer.last() == Some(&FRAME_START[0]));
                    self.buffer.drain(..self.buffer.len().saturating_sub(keep));
                    return None;
                }
                _ => {}
            }
            if self.buffer.len() < FRAME_LENGTH {
                return None;
            }
            if let Ok(frame) = emit::parse_frame(&self.buffer[..FRAME_LENGTH]) {
                // Pelkkä 0x20 0x20 ei riitä: hyväksytään vain lukumerkin sisältävä kehys.
                if frame.card_number != 0 && frame.punches_before_reader_marker().is_ok() {
                    self.buffer.drain(..FRAME_LENGTH);
                    return Some(frame);
                }
            }
            // Virheellinen kehys: etsitään seuraavaa synkronointia yhden tavun päästä.
            self.buffer.drain(..1);
        }
    }
}

/// Jos porttia ei anneta, valitaan ensisijaisesti FTDI-lukija.
/// Käyttäjä voi aina ohittaa oletuksen itse antamallaan portilla.
pub fn port_name(requested: Option<&str>) -> Result<String, String> {
    if let Some(value) = requested.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(value.to_owned());
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(entries) = std::fs::read_dir("/dev/serial/by-id") {
            let mut ftdi: Vec<_> = entries
                .flatten()
                .filter_map(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    (name.to_ascii_lowercase().contains("ftdi"))
                        .then(|| entry.path().to_string_lossy().to_string())
                })
                .collect();
            ftdi.sort();
            if let Some(path) = ftdi.into_iter().next() {
                return Ok(path);
            }
        }
    }
    if let Ok(ports) = serialport::available_ports()
        && let Some(port) = ports.iter().find(|p| {
            matches!(&p.port_type,
            serialport::SerialPortType::UsbPort(info) if info.vid == 0x0403 && info.pid == 0x6001)
        })
    {
        return Ok(port.port_name.clone());
    }
    #[cfg(target_os = "linux")]
    if std::path::Path::new("/dev/ttyUSB0").exists() {
        return Ok("/dev/ttyUSB0".to_owned());
    }
    Err("Emit-lukijaa ei löytynyt automaattisesti. Anna sarjaportti itse (esimerkiksi /dev/ttyUSB0 tai COM3).".into())
}

/// Jatkuva lukija säilyttää saman portin ja keskeneräisen kehyksen lukukertojen välillä.
/// Sisältöjen uusintalukujen vertailu kuuluu kilpailumallille, ei sarjaportille.
pub struct CardReader<R = Box<dyn serialport::SerialPort>> {
    device: R,
    scanner: FrameScanner,
}

impl CardReader {
    pub fn open(port: &str) -> Result<Self, String> {
        let device = serialport::new(port, 9600)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::Two)
        .flow_control(serialport::FlowControl::None)
        .timeout(Duration::from_millis(250))
        .open()
        .map_err(|error| {
            match error.kind() {
                serialport::ErrorKind::NoDevice => "Lukijaa ei löydy annetusta portista".to_owned(),
                serialport::ErrorKind::Io(io::ErrorKind::PermissionDenied) =>
                    "Sarjaportin käyttöoikeus puuttuu. Tarkista käyttäjän ryhmät (Linuxissa yleensä uucp)".to_owned(),
                _ => "Sarjaportin avaaminen epäonnistui. Tarkista portin nimi ja ettei toinen ohjelma käytä lukijaa".to_owned(),
            }
        })?;
        Ok(Self::from_reader(device))
    }
}

impl<R: Read> CardReader<R> {
    /// Mahdollistaa saman virtapolun testaamisen ilman fyysistä sarjaporttia.
    pub fn from_reader(device: R) -> Self {
        Self {
            device,
            scanner: FrameScanner::new(),
        }
    }

    /// Palauttaa yhden kehyksen tai None, kun kokonainen kehys ei ole vielä saapunut.
    /// Fyysisen portin yksittäinen odotus on enintään 250 millisekuntia.
    pub fn poll(&mut self) -> Result<Option<EmitFrame>, String> {
        if let Some(frame) = self.scanner.next_frame() {
            return Ok(Some(frame));
        }
        let mut buffer = [0u8; 512];
        match self.device.read(&mut buffer) {
            Ok(count) if count > 0 => Ok(self.scanner.push(&buffer[..count])),
            Ok(_) => Ok(None),
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
                ) =>
            {
                Ok(None)
            }
            Err(error) => Err(format!(
                "Sarjaportin lukeminen epäonnistui: {}",
                crate::io_message(&error)
            )),
        }
    }
}

/// Kertalukukomennon yhteensopivuusrajapinta. Jatkuva toiminto käyttää CardReaderia.
pub fn read_one(port: &str) -> Result<EmitFrame, String> {
    let mut reader = CardReader::open(port)?;
    let started = Instant::now();
    while started.elapsed() < READ_WAIT {
        if let Some(frame) = reader.poll()? {
            return Ok(frame);
        }
    }
    Err("Korttia ei saatu luettua 30 sekunnissa. Tarkista lukija ja kokeile uudelleen".into())
}
