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
            // Jos ennen kehyksen loppua alkaa uusi kehys, ensimmäinen jäi kesken.
            if let Some(next) = self.buffer[2..FRAME_LENGTH]
                .windows(2)
                .position(|pair| pair == FRAME_START.as_slice())
            {
                self.buffer.drain(..next + 2);
                continue;
            }
            if let Ok(frame) = emit::parse_frame(&self.buffer[..FRAME_LENGTH]) {
                // Pelkkä 0x20 0x20 ei riitä: hyväksytään vain lukumerkin sisältävä kehys.
                if frame.card_number != 0
                    && frame
                        .records
                        .iter()
                        .skip(1)
                        .any(|record| record.code == 250)
                {
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
    if let Ok(ports) = serialport::available_ports() {
        if let Some(port) = ports.iter().find(|p| {
            matches!(&p.port_type,
            serialport::SerialPortType::UsbPort(info) if info.vid == 0x0403 && info.pid == 0x6001)
        }) {
            return Ok(port.port_name.clone());
        }
    }
    #[cfg(target_os = "linux")]
    if std::path::Path::new("/dev/ttyUSB0").exists() {
        return Ok("/dev/ttyUSB0".to_owned());
    }
    Err("Emit-lukijaa ei löytynyt automaattisesti. Anna sarjaportti itse (esimerkiksi /dev/ttyUSB0 tai COM3).".into())
}

/// Lukee yhden kortin ja palauttaa heti ensimmäisen kokonaisen hyväksyttävän kehyksen.
/// Näin saman paikallaan olevan kortin jatkuvat kehykset eivät näy monena lukuna.
pub fn read_one(port: &str) -> Result<EmitFrame, String> {
    let mut device = serialport::new(port, 9600)
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
    let mut scanner = FrameScanner::new();
    let mut buffer = [0u8; 512];
    let started = Instant::now();
    while started.elapsed() < READ_WAIT {
        match device.read(&mut buffer) {
            Ok(count) if count > 0 => {
                if let Some(frame) = scanner.push(&buffer[..count]) {
                    return Ok(frame);
                }
            }
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => {
                return Err(format!(
                    "Sarjaportin lukeminen epäonnistui: {}",
                    crate::io_message(&error)
                ));
            }
        }
    }
    Err("Korttia ei saatu luettua 30 sekunnissa. Tarkista lukija ja kokeile uudelleen".into())
}
