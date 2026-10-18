//! AS2014A-pohjan RATA/KILP-tuonti. Lukeminen ei muuta työkirjaa tai kilpailua.
//! Pohjautuu käyttäjän AS2026-RATA-KILP-prototype.zip-paketin kenttäkarttaan.
use crate::{CoreError, native};
use calamine::{Data, Range, Reader, open_workbook_auto};
use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    path::Path,
};

const MAX_ROWS: u32 = 10_000;
const MAX_COLUMNS: u32 = 200;
const MAX_STATIONS: usize = 400;

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportIssue {
    pub estaa_tuonnin: bool,
    pub sijainti: String,
    pub viesti: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedTrack {
    pub luokka: String,
    pub alkuperainen_sarake: u32,
    pub asemat: Vec<native::Station>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportedDriver {
    pub alkuperainen_rivi: u32,
    pub tiedot: native::Driver,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ImportPreview {
    pub radat: Vec<ImportedTrack>,
    pub kilpailijat: Vec<ImportedDriver>,
    pub huomautukset: Vec<ImportIssue>,
    pub luokkien_radat: BTreeMap<String, String>,
}
impl ImportPreview {
    pub fn unresolved_classes(&self) -> Vec<String> {
        self.kilpailijat
            .iter()
            .map(|d| d.tiedot.luokka.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .filter(|code| self.route_for_class(code).is_none())
            .collect()
    }
    pub fn route_for_class(&self, code: &str) -> Option<&str> {
        self.luokkien_radat
            .get(code)
            .map(String::as_str)
            .or_else(|| {
                self.radat
                    .iter()
                    .find(|r| r.luokka == code)
                    .map(|r| r.luokka.as_str())
            })
    }
    pub fn set_class_route(&mut self, code: &str, route: &str) -> Result<(), CoreError> {
        if !self.kilpailijat.iter().any(|d| d.tiedot.luokka == code)
            && !self.radat.iter().any(|r| r.luokka == code)
        {
            return Err(invalid("Luokkaa ei löydy tuontiaineistosta"));
        }
        if !self.radat.iter().any(|r| r.luokka == route) {
            return Err(invalid("Valitse tuotava rata"));
        }
        self.luokkien_radat.insert(code.into(), route.into());
        Ok(())
    }
    pub fn has_importable_data(&self) -> bool {
        !self.radat.is_empty() && !self.huomautukset.iter().any(|issue| issue.estaa_tuonnin)
    }
    pub fn has_importable_tracks(&self) -> bool {
        !self.radat.is_empty()
            && !self
                .huomautukset
                .iter()
                .any(|issue| issue.estaa_tuonnin && !issue.sijainti.starts_with("KILP"))
    }
    pub fn tracks_only(&self) -> Self {
        let mut preview = self.clone();
        preview.kilpailijat.clear();
        preview.luokkien_radat.clear();
        preview
            .huomautukset
            .retain(|issue| !issue.sijainti.starts_with("KILP"));
        preview
    }
    fn issue(&mut self, blocking: bool, location: impl Into<String>, message: impl Into<String>) {
        self.huomautukset.push(ImportIssue {
            estaa_tuonnin: blocking,
            sijainti: location.into(),
            viesti: message.into(),
        });
    }
    pub fn can_import(&self) -> bool {
        self.has_importable_data() && self.unresolved_classes().is_empty()
    }
    pub fn summary(&self) -> String {
        let stations: usize = self.radat.iter().map(|track| track.asemat.len()).sum();
        let mut text = format!(
            "{} kilpailijaparia · {} rataa · {stations} asemaa",
            self.kilpailijat.len(),
            self.radat.len()
        );
        let cards = self
            .kilpailijat
            .iter()
            .filter(|d| d.tiedot.emit_number().is_none())
            .count();
        let starts = self
            .kilpailijat
            .iter()
            .filter(|d| d.tiedot.lahtoaika.is_empty())
            .count();
        if cards + starts > 0 {
            text.push_str(&format!(
                "\nTäydennettävää: {cards} Emit-numeroa, {starts} lähtöaikaa."
            ));
        }
        let unresolved = self.unresolved_classes();
        if !unresolved.is_empty() {
            text.push_str(&format!(
                "\nValitse rata luokille: {}.",
                unresolved.join(", ")
            ));
        }
        if !self.huomautukset.is_empty() {
            text.push_str(&format!(
                "\nTarkistettavaa: {} kohtaa.",
                self.huomautukset.len()
            ));
        }
        text
    }
    pub fn issue_details(&self) -> String {
        self.huomautukset
            .iter()
            .map(|i| format!("{}: {}", i.sijainti, i.viesti))
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn to_project(&self, name: &str) -> Result<native::Project, CoreError> {
        if !self.can_import() {
            return Err(invalid(
                "Excel-tuonnissa on ratkaisemattomia esteitä tai ei yhtään kelvollista rataa",
            ));
        }
        let mut project = native::Project::new(name)?;
        project.luokat.clear();
        project.radat.clear();
        for (index, source) in self.radat.iter().enumerate() {
            let number = u32::try_from(index + 1).map_err(|_| invalid("Ratoja on liikaa"))?;
            project.radat.push(native::Track {
                numero: number,
                nimi: source.luokka.clone(),
                asemat: source.asemat.clone(),
            });
            project.luokat.push(native::Class {
                tunnus: source.luokka.clone(),
                nimi: source.luokka.clone(),
                rata: number,
                ensimmainen_numero: self
                    .kilpailijat
                    .iter()
                    .filter(|driver| driver.tiedot.luokka == source.luokka)
                    .map(|driver| driver.tiedot.numero)
                    .min()
                    .unwrap_or(1),
                lahtoaika: "00:00:00".into(),
                lahtovali_minuutteina: 1,
                myohastymisraja_minuutteina: 60,
                nousupisteet: 0,
                sarjapistetapa: 0,
                osallistuu_lahtoarvontaan: false,
            });
        }
        for code in self
            .kilpailijat
            .iter()
            .map(|d| d.tiedot.luokka.clone())
            .collect::<BTreeSet<_>>()
        {
            let route = self
                .route_for_class(&code)
                .ok_or_else(|| invalid("Luokan ratavalinta puuttuu"))?;
            let number = self
                .radat
                .iter()
                .position(|r| r.luokka == route)
                .ok_or_else(|| invalid("Rataa ei löydy"))? as u32
                + 1;
            if let Some(class) = project.luokat.iter_mut().find(|c| c.tunnus == code) {
                class.rata = number;
            } else {
                let mut class = project.luokat[0].clone();
                class.tunnus = code.clone();
                class.nimi = code.clone();
                class.rata = number;
                class.ensimmainen_numero = self
                    .kilpailijat
                    .iter()
                    .filter(|d| d.tiedot.luokka == code)
                    .map(|d| d.tiedot.numero)
                    .min()
                    .unwrap_or(1);
                project.luokat.push(class);
            }
        }
        project.kilpailijat = self
            .kilpailijat
            .iter()
            .map(|driver| driver.tiedot.clone())
            .collect();
        project.validate()?;
        Ok(project)
    }
}

/// Lukee vain tuettujen välilehtien soluarvot; VBA- tai kaavojen suoritusrajapintaa ei kutsuta.
pub fn read(path: &Path) -> Result<ImportPreview, CoreError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if !extension.eq_ignore_ascii_case("xls") && !extension.eq_ignore_ascii_case("xlsx") {
        return Err(invalid("Tuetut Excel-tiedostot ovat .xls ja .xlsx"));
    }
    let metadata = std::fs::metadata(path).map_err(|error| {
        invalid(format!(
            "Excel-tiedostoa ei voi lukea: {}",
            crate::io_message(&error)
        ))
    })?;
    if !metadata.is_file() || metadata.len() > 30 * 1024 * 1024 {
        return Err(invalid(
            "Excel-tiedoston pitää olla tavallinen, enintään 30 megatavun tiedosto",
        ));
    }
    let mut workbook = open_workbook_auto(path).map_err(|_| {
        invalid("Excel-tiedostoa ei voi avata. Tarkista tiedostomuoto ja tiedoston eheys")
    })?;
    let names = workbook.sheet_names();
    let sheet = |expected: &str| -> Result<String, CoreError> {
        let matches = names
            .iter()
            .filter(|name| name.eq_ignore_ascii_case(expected))
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid(format!(
                "Työkirjassa pitää olla yksi {expected}-välilehti"
            )));
        }
        Ok(matches[0].to_string())
    };
    let track = workbook
        .worksheet_range(&sheet("RATA")?)
        .map_err(|_| invalid("RATA-välilehden lukeminen epäonnistui"))?;
    let drivers = workbook
        .worksheet_range(&sheet("KILP")?)
        .map_err(|_| invalid("KILP-välilehden lukeminen epäonnistui"))?;
    preview_sheets(&track, &drivers)
}
fn cell(sheet: &Range<Data>, row: u32, column: u32) -> Data {
    sheet
        .get_value((row, column))
        .cloned()
        .unwrap_or(Data::Empty)
}
fn text(cell: &Data) -> Result<String, String> {
    match cell {
        Data::Empty => Ok(String::new()),
        Data::String(value) => Ok(value.trim().to_owned()),
        Data::Int(value) => Ok(value.to_string()),
        Data::Float(value) if value.is_finite() => Ok(if value.fract() == 0.0 {
            format!("{value:.0}")
        } else {
            value.to_string()
        }),
        _ => Err("Solussa on virhe tai tunnistamaton arvo".into()),
    }
}
fn empty(cell: &Data) -> bool {
    matches!(cell, Data::Empty) || matches!(cell, Data::String(value) if value.trim().is_empty())
}
fn integer(cell: &Data) -> Result<Option<u32>, String> {
    if empty(cell) {
        return Ok(None);
    }
    match cell {
        Data::Float(value)
            if value.is_finite()
                && value.fract() == 0.0
                && *value >= 0.0
                && *value <= f64::from(u32::MAX) =>
        {
            Ok(Some(*value as u32))
        }
        _ => text(cell)?
            .parse::<u32>()
            .map(Some)
            .map_err(|_| "Odotettiin ei-negatiivista kokonaislukua".into()),
    }
}
fn time_text(seconds: u32) -> Result<String, String> {
    if seconds >= 86_400 {
        return Err("Kellonaika ylittää vuorokauden".into());
    }
    Ok(format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    ))
}
fn clock(value: &Data, minutes: &Data) -> Result<String, String> {
    if empty(value) && empty(minutes) {
        return Ok(String::new());
    }
    let excel = match value {
        Data::DateTime(v) => Some(v.as_f64()),
        Data::Float(v) if *v > 0.0 && *v < 1.0 => Some(*v),
        _ => None,
    };
    if let Some(days) = excel {
        if !days.is_finite() || !(0.0..1.0).contains(&days) {
            return Err("Excel-kellonaika ei ole kelvollinen".into());
        }
        let seconds = (days * 86400.0).round() as u32;
        if !empty(minutes) {
            let m = integer(minutes)?.ok_or("Lähtöminuutti puuttuu")?;
            if m != 0 && m != (seconds / 60) % 60 {
                return Err("Lähtöajan D- ja E-solut ovat ristiriidassa".into());
            }
        }
        return time_text(seconds);
    }
    if !empty(minutes) {
        let hour = integer(value)?.ok_or("Lähtötunti puuttuu")?;
        let minute = integer(minutes)?.ok_or("Lähtöminuutti puuttuu")?;
        if hour > 23 || minute > 59 {
            return Err("Lähtöaika ei ole kelvollinen".into());
        }
        return time_text(hour * 3600 + minute * 60);
    }
    let numeric = match value {
        Data::Float(value) => Some(*value),
        Data::DateTime(value) => Some(value.as_f64()),
        _ => None,
    };
    if let Some(value) = numeric {
        if value.is_finite() && (0.0..1.0).contains(&value) {
            return time_text((value * 86_400.0).round() as u32);
        }
        return Err("Lähtöaika on epäselvä: anna Excel-kellonaika, hh:mm:ss-teksti tai tunnit ja minuutit D/E-sarakkeisiin".into());
    }
    let value = text(value)?.replace('.', ":");
    let values = value
        .split(':')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Lähtöajan muoto pitää olla hh:mm tai hh:mm:ss")?;
    match values.as_slice() {
        [h, m] if *h < 24 && *m < 60 => time_text(h * 3600 + m * 60),
        [h, m, s] if *h < 24 && *m < 60 && *s < 60 => time_text(h * 3600 + m * 60 + s),
        _ => Err("Lähtöajan muoto pitää olla hh:mm tai hh:mm:ss".into()),
    }
}
fn target(value: &Data, kind: &str) -> Result<i64, String> {
    if empty(value) {
        return if matches!(kind, "L" | "ULA") {
            Ok(0)
        } else {
            Err("Aseman ihanneaika puuttuu".into())
        };
    }
    if let Data::DateTime(value) = value {
        let days = value.as_f64();
        return if days.is_finite() && (0.0..1.0).contains(&days) {
            Ok((days * 86_400.0).round() as i64)
        } else {
            Err("Excel-ihanneaika ei ole kelvollinen kesto".into())
        };
    }
    if let Data::Float(number) = value {
        if !number.is_finite() || *number < 0.0 || *number > i64::MAX as f64 / 100.0 {
            return Err("Ihanneaika ei ole kelvollinen".into());
        }
        let hundredths = number * 100.0;
        if (hundredths - hundredths.round()).abs() > 0.00001 {
            return Err("Min.sek-ihanneajassa saa olla enintään kaksi sekuntinumeroa".into());
        }
        let units = hundredths.round() as i64;
        return if units % 100 < 60 {
            (units / 100)
                .checked_mul(60)
                .and_then(|minutes| minutes.checked_add(units % 100))
                .ok_or_else(|| "Ihanneaika on liian suuri".into())
        } else {
            Err("Ihanneajan sekuntien pitää olla välillä 00–59".into())
        };
    }
    if let Data::Int(minutes) = value {
        return minutes
            .checked_mul(60)
            .filter(|seconds| *seconds >= 0)
            .ok_or_else(|| "Ihanneaika on virheellinen".into());
    }
    let value = text(value)?;
    let values = value
        .split([',', ':', '.'])
        .map(str::parse::<i64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Ihanneajan pitää olla Min.sek- tai hh:mm:ss-muodossa")?;
    match values.as_slice() {
        [m, s] if *m >= 0 && (0..60).contains(s) => {
            m.checked_mul(60).and_then(|value| value.checked_add(*s))
        }
        [h, m, s] if *h >= 0 && (0..60).contains(m) && (0..60).contains(s) => h
            .checked_mul(3600)
            .and_then(|value| value.checked_add(m * 60 + s)),
        _ => None,
    }
    .ok_or_else(|| "Ihanneajan minuutit tai sekunnit ovat virheelliset".into())
}
fn station_kind(name: &str) -> Option<&'static str> {
    let upper = name.trim().to_uppercase();
    if upper == "L" || upper.starts_with("LÄHTÖ") || upper.starts_with("LAHTO") {
        return Some("L");
    }
    if upper == "M" || upper.starts_with("MAALI") {
        return Some("M");
    }
    for kind in ["ULA", "JAT", "AT", "RT"] {
        if let Some(rest) = upper.strip_prefix(kind)
            && (rest.is_empty()
                || rest.starts_with(|character: char| {
                    character.is_ascii_digit() || character.is_whitespace() || character == '-'
                }))
        {
            return Some(kind);
        }
    }
    None
}
fn check_size(sheet: &Range<Data>) -> Result<(), CoreError> {
    if sheet
        .end()
        .is_some_and(|(row, column)| row >= MAX_ROWS || column >= MAX_COLUMNS)
    {
        return Err(invalid(
            "Excel-taulukon koko ylittää tuontirajan (10000 riviä, 200 saraketta)",
        ));
    }
    Ok(())
}
fn column_name(mut value: u32) -> String {
    let mut name = String::new();
    value += 1;
    while value > 0 {
        value -= 1;
        name.insert(0, (b'A' + (value % 26) as u8) as char);
        value /= 26;
    }
    name
}

/// Kenttäkartan tarkka tulkinta on erillinen levystä ja käyttöliittymästä.
/// Viallinen rivi tai rata ohitetaan selitteineen; tunnistekonflikti estää tallennuksen.
pub fn preview_sheets(
    track: &Range<Data>,
    drivers: &Range<Data>,
) -> Result<ImportPreview, CoreError> {
    check_size(track)?;
    check_size(drivers)?;
    let mut preview = ImportPreview::default();
    let mut classes = HashSet::new();
    let mut groups = 0;
    for column in (0..MAX_COLUMNS).step_by(5) {
        if text(&cell(track, 0, column))
            .unwrap_or_default()
            .to_lowercase()
            != "luokka"
        {
            continue;
        }
        groups += 1;
        let code = text(&cell(track, 0, column + 1)).unwrap_or_default();
        if code.is_empty()
            && !(3..=track.end().map_or(0, |(row, _)| row))
                .any(|row| (0..5).any(|offset| !empty(&cell(track, row, column + offset))))
        {
            continue;
        }
        let location = format!("RATA {}1", column_name(column));
        if code.is_empty() || !classes.insert(code.clone()) {
            preview.issue(true, location, "Luokan tunnus puuttuu tai esiintyy useasti");
            continue;
        }
        let headings = [
            (0, &["emit no"][..]),
            (1, &["asemanimi"][..]),
            (2, &["ihanneaika", "ihannea"][..]),
            (3, &["kdi"][..]),
            (4, &["jakso"][..]),
        ];
        if headings.iter().any(|(offset, expected)| {
            !expected.contains(
                &text(&cell(track, 2, column + offset))
                    .unwrap_or_default()
                    .to_lowercase()
                    .as_str(),
            )
        }) {
            preview.issue(
                true,
                location,
                format!("Luokan {code} sarakeotsikot eivät vastaa RATA-pohjaa"),
            );
            continue;
        }
        let mut stations = Vec::new();
        let mut rejected = false;
        for row in 3..=track.end().map_or(0, |(row, _)| row) {
            let name_cell = cell(track, row, column + 1);
            let emit_cell = cell(track, row, column);
            if empty(&name_cell) && empty(&emit_cell) {
                continue;
            }
            let location = format!("RATA {}{}", column_name(column), row + 1);
            let parsed = (|| -> Result<native::Station, String> {
                let name = text(&name_cell)?;
                let kind = station_kind(&name)
                    .ok_or_else(|| format!("Asematyyppiä ei tunnisteta nimestä {name:?}"))?;
                let emit = integer(&emit_cell)?
                    .or_else(|| matches!(kind, "L" | "M").then_some(0))
                    .ok_or("Aseman Emit-numero puuttuu")?;
                let target = target(&cell(track, row, column + 2), kind)?;
                let rule = integer(&cell(track, row, column + 3))?.unwrap_or(0);
                if rule > 7 {
                    return Err("KDI-arvon pitää olla välillä 0–7".into());
                }
                let section = integer(&cell(track, row, column + 4))?;
                Ok(native::Station {
                    tyyppi: kind.into(),
                    nimi: name,
                    emit_koodi: emit,
                    tavoiteaika_sekuntia: target,
                    saanto: i64::from(rule),
                    jakso_minuutteina: section,
                })
            })();
            match parsed {
                Ok(station) => stations.push(station),
                Err(error) => {
                    rejected = true;
                    preview.issue(
                        false,
                        location,
                        format!("Luokan {code} rataa ei tuoda: {error}"),
                    );
                }
            }
            if stations.len() > MAX_STATIONS {
                rejected = true;
                preview.issue(
                    false,
                    "RATA",
                    format!("Luokan {code} radassa on yli {MAX_STATIONS} asemaa"),
                );
                break;
            }
        }
        if stations.is_empty() {
            preview.issue(
                false,
                "RATA",
                format!("Luokan {code} rata on tyhjä eikä sitä tuoda"),
            );
            continue;
        }
        if stations.first().is_none_or(|station| station.tyyppi != "L")
            || stations.last().is_none_or(|station| station.tyyppi != "M")
        {
            rejected = true;
            preview.issue(false, "RATA", format!("Luokan {code} rataa ei tuoda: lähdön pitää olla ensimmäinen ja maalin viimeinen asema"));
        }
        if !rejected {
            preview.radat.push(ImportedTrack {
                luokka: code,
                alkuperainen_sarake: column + 1,
                asemat: stations,
            });
        }
    }
    if groups == 0 {
        preview.issue(true, "RATA", "Ryhmiteltyjä Luokka-otsikoita ei löytynyt");
    }
    let headings = [
        (0, "nro"),
        (1, "lka"),
        (2, "emit"),
        (3, "l-aika"),
        (5, "kuski"),
        (6, "seura"),
        (7, "kartturi"),
        (8, "seura"),
        (9, "auto"),
    ];
    if headings.iter().any(|(column, expected)| {
        text(&cell(drivers, 0, *column))
            .unwrap_or_default()
            .to_lowercase()
            != *expected
    }) {
        preview.issue(
            true,
            "KILP A1:J1",
            "Kilpailijataulukon otsikot eivät vastaa KILP-pohjaa",
        );
        return Ok(preview);
    }
    let mut numbers = HashSet::new();
    let mut cards = HashSet::new();
    for row in 1..=drivers.end().map_or(0, |(row, _)| row) {
        if empty(&cell(drivers, row, 0))
            && empty(&cell(drivers, row, 2))
            && empty(&cell(drivers, row, 5))
        {
            continue;
        }
        let location = format!("KILP rivi {}", row + 1);
        let parsed = (|| -> Result<native::Driver, String> {
            let number = integer(&cell(drivers, row, 0))?
                .filter(|number| *number > 0)
                .ok_or("Kilpailijan numero puuttuu tai on virheellinen")?;
            let card = match integer(&cell(drivers, row, 2))? {
                None => 0,
                Some(0) => return Err("Emit-numero ei voi olla 0".into()),
                Some(card) => card,
            };
            let class = text(&cell(drivers, row, 1))?;
            if class.is_empty() {
                return Err("Luokka puuttuu".into());
            }
            let driver = text(&cell(drivers, row, 5))?;
            if driver.is_empty() {
                return Err("Kuljettajan nimi puuttuu".into());
            }
            Ok(native::Driver {
                numero: number,
                luokka: class,
                emit: card,
                kuljettaja: driver,
                kartanlukija: text(&cell(drivers, row, 7))?,
                kuljettajan_seura: text(&cell(drivers, row, 6))?,
                kartanlukijan_seura: text(&cell(drivers, row, 8))?,
                kuljettajan_lisenssi: String::new(),
                kartanlukijan_lisenssi: String::new(),
                auto: text(&cell(drivers, row, 9))?,
                lahtoaika: clock(&cell(drivers, row, 3), &cell(drivers, row, 4))?,
                emit_korjaus_sekuntia: 0,
                lisavirheet: native::AdditionalPenalties::default(),
                tila: None,
                keskeytys_vahvistettu: false,
                tallennettu_tulos: None,
            })
        })();
        match parsed {
            Ok(driver) => {
                let duplicate_number = !numbers.insert(driver.numero);
                let duplicate_card = driver.emit_number().is_some_and(|card| !cards.insert(card));
                if duplicate_number || duplicate_card {
                    preview.issue(true, location, "Kilpailijan numero tai Emit-numero esiintyy useasti; korjaa tunnisteet ennen tuontia");
                } else {
                    preview.kilpailijat.push(ImportedDriver {
                        alkuperainen_rivi: row + 1,
                        tiedot: driver,
                    });
                }
            }
            Err(error) => preview.issue(false, location, format!("Riviä ei tuoda: {error}")),
        }
    }
    Ok(preview)
}
