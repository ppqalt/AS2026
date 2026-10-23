//! AS2026:n yhteinen kilpailu- ja laskentakirjasto.
//!
//! Emit-aineiston tuonti, kohdistus, rajattu jaksolaskenta ja sijoitukset.
//! Alkuperäisellä ohjelmalla tehtävää vertailua ei ole suoritettu.

pub mod application;
pub mod beginners;
pub mod changes;
pub mod corrections;
pub mod emit;
pub mod emit_corrections;
pub mod excel_import;
#[cfg(feature = "desktop")]
pub mod installation;
#[cfg(feature = "desktop")]
pub mod lan;
pub mod legacy;
#[cfg(feature = "desktop")]
pub mod live_publish;
pub mod live_reader;
pub mod matching;
#[cfg(feature = "desktop")]
pub mod preferences;
pub mod reader;
pub mod scoring;
pub mod starts;
#[cfg(feature = "desktop")]
pub mod updates;
pub mod workspace;

use std::fmt;

/// Käyttöjärjestelmän viestejä ei näytetä sen kieliasetuksen varassa.
pub fn io_message(error: &std::io::Error) -> String {
    use std::io::ErrorKind;
    let description = match error.kind() {
        ErrorKind::NotFound => "Tiedostoa tai kansiota ei löydy",
        ErrorKind::PermissionDenied => "Tiedoston tai kansion käyttöoikeus puuttuu",
        ErrorKind::AlreadyExists => "Tiedosto on jo olemassa; valitse toinen nimi",
        ErrorKind::InvalidInput => "Tiedoston polku tai syöte on virheellinen",
        ErrorKind::InvalidData => "Tiedoston tai syötteen sisältö on virheellinen",
        ErrorKind::BrokenPipe => "Tulosteen vastaanottaja sulki yhteyden",
        ErrorKind::UnexpectedEof => "Tiedosto tai syöte loppui kesken",
        ErrorKind::WriteZero => "Tiedoston kirjoittaminen ei edennyt",
        ErrorKind::OutOfMemory => "Muisti ei riitä toiminnon suorittamiseen",
        ErrorKind::StorageFull => "Tallennustila on täynnä",
        ErrorKind::IsADirectory => "Annettu polku on kansio; tarvitaan tiedosto",
        ErrorKind::NotADirectory => "Annettu polku ei ole kansio",
        _ => "Tiedoston tai syötteen käsittely epäonnistui",
    };
    match error.raw_os_error() {
        Some(code) => format!("{description} (järjestelmän virhenumero {code})"),
        None => description.to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    InvalidInput(String),
    Unsupported(String),
    ArithmeticOverflow(&'static str),
    FrameLength { expected: usize, actual: usize },
    Synchronization,
    MissingMarker,
    MarkerAtStart,
}

impl fmt::Display for CoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "Virheellinen syöte: {message}"),
            Self::Unsupported(message) => write!(f, "Ei vielä tuettu: {message}"),
            Self::ArithmeticOverflow(operation) => {
                write!(f, "Laskennan lukuarvo ylittää sallitun alueen: {operation}")
            }
            Self::FrameLength { expected, actual } => {
                write!(
                    f,
                    "Emit-kehyksen pituus on {actual}; odotettiin {expected} tavua"
                )
            }
            Self::Synchronization => {
                write!(f, "Emit-kehyksen alussa ei ole kahta synkronointitavua")
            }
            Self::MissingMarker => write!(f, "Emit-kehyksestä puuttuu lukumerkki 250"),
            Self::MarkerAtStart => write!(f, "Emit-lukumerkillä ei ole edeltävää leimausta"),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<String> for CoreError {
    fn from(message: String) -> Self {
        Self::InvalidInput(message)
    }
}

impl From<&str> for CoreError {
    fn from(message: &str) -> Self {
        Self::InvalidInput(message.to_owned())
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Punch {
    pub code: u32,
    pub elapsed_seconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockTime {
    pub hour: u32,
    pub minute: u32,
    pub second: u32,
}

impl ClockTime {
    /// Tavallinen vuorokausikello. Analyysin 24:00:00-poikkeusta ei normalisoida tähän.
    pub fn new(hour: u32, minute: u32, second: u32) -> Result<Self, CoreError> {
        let clock = Self {
            hour,
            minute,
            second,
        };
        clock.seconds_since_midnight()?;
        Ok(clock)
    }

    pub fn seconds_since_midnight(self) -> Result<i64, CoreError> {
        if self.hour > 23 || self.minute > 59 || self.second > 59 {
            return Err(CoreError::InvalidInput(format!(
                "kellonaika {:02}:{:02}:{:02}",
                self.hour, self.minute, self.second
            )));
        }
        Ok(i64::from(self.hour) * 3600 + i64::from(self.minute) * 60 + i64::from(self.second))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationKind {
    Start,
    Ula,
    Jat,
    At,
    Finish,
    Rt,
}

impl StationKind {
    /// Vain kokonaiset kanoniset tyypit; vanhan vertailurutiinin etuliitevirhettä ei jäljitellä.
    pub fn from_legacy(value: &str) -> Result<Self, CoreError> {
        match value.trim() {
            "L" => Ok(Self::Start),
            "ULA" => Ok(Self::Ula),
            "JAT" => Ok(Self::Jat),
            "AT" => Ok(Self::At),
            "M" => Ok(Self::Finish),
            "RT" => Ok(Self::Rt),
            value => Err(CoreError::Unsupported(format!("asemalaji {value:?}"))),
        }
    }

    pub const fn as_legacy(self) -> &'static str {
        match self {
            Self::Start => "L",
            Self::Ula => "ULA",
            Self::Jat => "JAT",
            Self::At => "AT",
            Self::Finish => "M",
            Self::Rt => "RT",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Rule {
    Normal = 0,
    NoEarly = 1,
    NoTiming = 2,
    NoLate = 3,
    Void = 4,
    UlaNearestMinute = 5,
    UlaFloorMinute = 6,
    UlaCeilMinute = 7,
}

impl Rule {
    pub fn from_legacy(value: i64) -> Result<Self, CoreError> {
        match value {
            0 => Ok(Self::Normal),
            1 => Ok(Self::NoEarly),
            2 => Ok(Self::NoTiming),
            3 => Ok(Self::NoLate),
            4 => Ok(Self::Void),
            5 => Ok(Self::UlaNearestMinute),
            6 => Ok(Self::UlaFloorMinute),
            7 => Ok(Self::UlaCeilMinute),
            value => Err(CoreError::Unsupported(format!("asemasääntö {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteStation {
    pub sequence: u32,
    pub kind: StationKind,
    pub label: String,
    pub expected_code: u32,
    pub target_seconds: i64,
    pub rule: Rule,
    pub section_time_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchKind {
    Matched,
    Missing,
    Extra,
    RouteOnly,
    SharedUla,
    ManualStart,
    ManualFinish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedRow {
    pub output_index: usize,
    pub station: Option<RouteStation>,
    pub punch: Option<Punch>,
    pub kind: MatchKind,
    pub clock: Option<ClockTime>,
}

pub mod native;
pub mod printing;
pub mod storage;

pub mod terminal;

pub mod ranking;

pub mod live_report;
pub mod report;

pub mod timing;
