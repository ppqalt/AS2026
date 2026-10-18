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
}
impl ImportPreview {
    fn issue(&mut self, blocking: bool, location: impl Into<String>, message: impl Into<String>) {
        self.huomautukset.push(ImportIssue {
            estaa_tuonnin: blocking,
            sijainti: location.into(),
            viesti: message.into(),
        });
    }
    pub fn can_import(&self) -> bool {
        (!self.radat.is_empty() || !self.kilpailijat.is_empty())
            && !self.huomautukset.iter().any(|issue| issue.estaa_tuonnin)
    }
    pub fn summary(&self) -> String {
        let stations: usize = self.radat.iter().map(|track| track.asemat.len()).sum();
        let mut text = format!(
            "Tuotavat tiedot: {} luokkaa ja rataa, {stations} asemaa, {} kilpailijaparia.\n",
            self.radat.len(),
            self.kilpailijat.len()
        );
        for track in &self.radat {
            text.push_str(&format!(
                "Luokka {}: {} asemaa, {} kilpailijaparia.\n",
                track.luokka,
                track.asemat.len(),
                self.kilpailijat
                    .iter()
                    .filter(|driver| driver.tiedot.luokka == track.luokka)
                    .count()
            ));
        }
        text.push_str("\nRATA- ja KILP-välilehtien tunnistetut tiedot tuodaan riippumatta siitä, onko toinen välilehti tyhjä tai puuttuuko se. Puuttuva Emit, lähtöaika tai rata jää täydennettäväksi (ei arvata). Apulehdet ohitetaan. Työkirjaa ei muuteta. Luokat eivät osallistu lähtöarvontaan ennen erillistä valintaa.\n");
        let mut groups: BTreeMap<(bool, String), (usize, String)> = BTreeMap::new();
        for issue in &self.huomautukset {
            let item = groups
                .entry((issue.estaa_tuonnin, issue.viesti.clone()))
                .or_insert((0, issue.sijainti.clone()));
            item.0 += 1;
        }
        for ((blocking, message), (count, example)) in groups {
            text.push_str(&format!(
                "\n{}{} ({} kpl; esim. {}): {}",
                if blocking {
                    "Tuonnin este — "
                } else {
                    "Huomautus — "
                },
                if count > 1 {
                    "toistuva"
                } else {
                    "yksittäinen"
                },
                count,
                example,
                message
            ));
        }
        if !self.can_import() {
            text.push_str("\n\nTuontia ei voi vahvistaa. Tarkista lähdetyökirjan tiedot.");
        }
        text
    }
    pub fn to_project(&self, name: &str) -> Result<native::Project, CoreError> {
        if !self.can_import() {
            return Err(invalid(
                "Excel-tuonnissa on ratkaisemattomia esteitä tai ei yhtään kelvollista rataa tai kilpailijaa",
            ));
        }
        let mut project = native::Project::new(name)?;
        project.luokat.clear();
        project.radat.clear();
        let mut route_for_class = BTreeMap::new();
        for (index, source) in self.radat.iter().enumerate() {
            let number = u32::try_from(index + 1).map_err(|_| invalid("Ratoja on liikaa"))?;
            project.radat.push(native::Track {
                numero: number,
                nimi: source.luokka.clone(),
                asemat: source.asemat.clone(),
            });
            route_for_class.insert(source.luokka.clone(), number);
        }
        // Säilytä RATA-välilehden luokkajärjestys ja lisää vasta sitten
        // KILP-välilehdessä esiintyvät uudet luokat.
        let mut codes = self
            .radat
            .iter()
            .map(|track| track.luokka.clone())
            .collect::<Vec<_>>();
        for driver in &self.kilpailijat {
            if !codes.contains(&driver.tiedot.luokka) {
                codes.push(driver.tiedot.luokka.clone());
            }
        }
        for code in codes {
            let route = route_for_class
                .get(&code)
                .copied()
                .or_else(|| {
                    code.strip_prefix("SM-")
                        .filter(|suffix| !suffix.contains('+'))
                        .and_then(|suffix| route_for_class.get(suffix).copied())
                })
                .unwrap_or(0);
            project.luokat.push(native::Class {
                tunnus: code.clone(),
                nimi: code.clone(),
                rata: route, // 0 = puuttuva rata, ei keksittyä asemaluetteloa.
                ensimmainen_numero: self
                    .kilpailijat
                    .iter()
                    .filter(|driver| driver.tiedot.luokka == code)
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
    let find_sheet = |expected: &str| -> Option<String> {
        names
            .iter()
            .find(|name| name.trim().eq_ignore_ascii_case(expected))
            .cloned()
    };
    if find_sheet("RATA").is_none() && find_sheet("KILP").is_none() {
        return Err(invalid("Excelistä ei löydy RATA- eikä KILP-välilehteä"));
    }
    let track = match find_sheet("RATA") {
        Some(name) => workbook
            .worksheet_range(&name)
            .map_err(|_| invalid("RATA-välilehden lukeminen epäonnistui"))?,
        None => Range::<Data>::new((0, 0), (0, 0)),
    };
    let drivers = match find_sheet("KILP") {
        Some(name) => workbook
            .worksheet_range(&name)
            .map_err(|_| invalid("KILP-välilehden lukeminen epäonnistui"))?,
        None => Range::<Data>::new((0, 0), (0, 0)),
    };
    let mut preview = preview_sheets(&track, &drivers)?;
    if find_sheet("RATA").is_none() {
        preview.issue(
            false,
            "RATA",
            "RATA-välilehti puuttuu. Kilpailijat tuodaan ilman arvattuja ratoja.",
        );
    }
    if find_sheet("KILP").is_none() {
        preview.issue(
            false,
            "KILP",
            "KILP-välilehti puuttuu. Radat tuodaan ilman kilpailijoita.",
        );
    }
    Ok(preview)
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
    for column in 0..MAX_COLUMNS {
        if text(&cell(track, 0, column))
            .unwrap_or_default()
            .to_lowercase()
            != "luokka"
        {
            continue;
        }
        groups += 1;
        let code = text(&cell(track, 0, column + 1)).unwrap_or_default();
        let location = format!("RATA {}1", column_name(column));
        if code.is_empty() {
            // AS2014A:n tyhjä neljäs Luokka-lohko ei estä tuontia.
            if (3..=track.end().map_or(0, |(row, _)| row)).any(|row| {
                !empty(&cell(track, row, column)) || !empty(&cell(track, row, column + 1))
            }) {
                preview.issue(
                    false,
                    location,
                    "Luokan tunnus puuttuu; keskeneräinen ratalohko ohitettiin",
                );
            }
            continue;
        }
        if !classes.insert(code.clone()) {
            preview.issue(true, location, "Luokan tunnus esiintyy useasti");
            continue;
        }
        let headings = [
            (0, &["emit no"][..]),
            (1, &["asemanimi"][..]),
            (2, &["ihanneaika", "ihannea"][..]),
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
        preview.issue(
            false,
            "RATA",
            "Ratoja ei löytynyt. Puuttuvat radat pitää täydentää ennen tuloslaskentaa.",
        );
    }
    // Otsikot tunnistetaan nimen perusteella, eivät kiinteistä sarakkeista.
    let mut header = None;
    for row in 0..=drivers.end().map_or(0, |(row, _)| row).min(9) {
        let headings = (0..=drivers.end().map_or(0, |(_, col)| col))
            .map(|col| {
                text(&cell(drivers, row, col))
                    .unwrap_or_default()
                    .to_lowercase()
            })
            .collect::<Vec<_>>();
        if headings
            .iter()
            .any(|h| matches!(h.as_str(), "nro" | "numero"))
        {
            header = Some((row, headings));
            break;
        }
    }
    let Some((header_row, headings)) = header else {
        if drivers
            .rows()
            .any(|row| row.iter().any(|value| !empty(value)))
        {
            preview.issue(true, "KILP", "Kilpailijataulukon nro-otsikkoa ei löytynyt");
        } else {
            preview.issue(false, "KILP", "Kilpailijoita ei löytynyt");
        }
        return Ok(preview);
    };
    let find = |aliases: &[&str]| -> Option<u32> {
        headings
            .iter()
            .position(|value| aliases.contains(&value.as_str()))
            .map(|index| index as u32)
    };
    let num_col = find(&["nro", "numero"]);
    let class_col = find(&["lka", "luokka"]);
    let driver_col = find(&["kuski", "kuljettaja"]);
    let emit_col = find(&["emit", "emit no", "emit numero", "emit-nro"]);
    let time_col = find(&["l-aika", "lähtöaika", "lahtoaika"]);
    let minute_col = time_col
        .filter(|index| {
            headings
                .get((*index + 1) as usize)
                .is_some_and(|h| h.is_empty() || h == "min")
        })
        .map(|index| index + 1);
    let navigator_col = find(&["kartturi", "kartanlukija"]);
    let car_col = find(&["auto", "ajoneuvo"]);
    let club_cols = headings
        .iter()
        .enumerate()
        .filter(|(_, h)| h.as_str() == "seura")
        .map(|(i, _)| i as u32)
        .collect::<Vec<_>>();
    if num_col.is_none() || class_col.is_none() || driver_col.is_none() {
        preview.issue(
            true,
            "KILP",
            "Kilpailijataulukosta puuttuu nro-, Lka- tai Kuski-otsikko",
        );
        return Ok(preview);
    }
    let field = |row, col: Option<u32>| -> Data {
        col.map_or(Data::Empty, |index| cell(drivers, row, index))
    };
    let mut numbers = HashSet::new();
    let mut cards: BTreeMap<u32, usize> = BTreeMap::new();
    for row in header_row + 1..=drivers.end().map_or(0, |(row, _)| row) {
        if empty(&field(row, num_col)) && empty(&field(row, driver_col)) {
            continue;
        }
        let location = format!("KILP rivi {}", row + 1);
        let parsed = (|| -> Result<(native::Driver, Vec<String>), String> {
            let number = integer(&field(row, num_col))?
                .filter(|value| *value > 0)
                .ok_or("Kilpailijan numero puuttuu tai on virheellinen")?;
            let class = text(&field(row, class_col))?;
            if class.is_empty() {
                return Err("Kilpailijan luokka puuttuu".into());
            }
            let name = text(&field(row, driver_col))?;
            if name.is_empty() {
                return Err("Kuljettajan nimi puuttuu".into());
            }
            // Viallisetkin valinnaiset kentät jätetään täydennettäviksi:
            // ilmoittautunutta ei saa hukata puuttuvan kellominuutin vuoksi.
            let mut optional_issues = Vec::new();
            let card = match integer(&field(row, emit_col)) {
                Ok(number) => number.unwrap_or(0),
                Err(error) => {
                    optional_issues.push(format!(
                        "Emit-numero on virheellinen ({error}); kortti jätettiin täydennettäväksi"
                    ));
                    0
                }
            };
            let time = if empty(&field(row, time_col)) && empty(&field(row, minute_col)) {
                String::new()
            } else {
                match clock(&field(row, time_col), &field(row, minute_col)) {
                    Ok(time) => time,
                    Err(error) => {
                        optional_issues.push(format!(
                            "Lähtöaika on puutteellinen tai virheellinen ({error}); aika jätettiin täydennettäväksi"
                        ));
                        String::new()
                    }
                }
            };
            let imported = native::Driver {
                numero: number,
                luokka: class,
                emit: card,
                kuljettaja: name,
                kartanlukija: text(&field(row, navigator_col))?,
                kuljettajan_seura: text(&field(row, club_cols.first().copied()))?,
                kartanlukijan_seura: text(&field(row, club_cols.get(1).copied()))?,
                kuljettajan_lisenssi: String::new(),
                kartanlukijan_lisenssi: String::new(),
                auto: text(&field(row, car_col))?,
                lahtoaika: time,
                emit_korjaus_sekuntia: 0,
                lisavirheet: native::AdditionalPenalties::default(),
                tila: None,
                keskeytys_vahvistettu: false,
                tallennettu_tulos: None,
            };
            Ok((imported, optional_issues))
        })();
        match parsed {
            Ok((driver, optional_issues)) => {
                if !numbers.insert(driver.numero) {
                    preview.issue(
                        true,
                        location,
                        "Kilpailijan numero esiintyy useasti. Korjaa tunniste ennen tuontia.",
                    );
                    continue;
                }
                for issue in optional_issues {
                    preview.issue(false, &location, issue);
                }
                if driver.emit != 0 {
                    *cards.entry(driver.emit).or_default() += 1;
                } else {
                    preview.issue(
                        false,
                        &location,
                        "Emit-numero puuttuu: täydennä kortti ennen lukemista",
                    );
                }
                if driver.lahtoaika.is_empty() {
                    preview.issue(
                        false,
                        &location,
                        "Lähtöaika puuttuu: täydennä aika tai arvo lähtöajat",
                    );
                }
                preview.kilpailijat.push(ImportedDriver {
                    alkuperainen_rivi: row + 1,
                    tiedot: driver,
                });
            }
            Err(error) => preview.issue(false, location, format!("Riviä ei tuoda: {error}")),
        }
    }
    // Toistuvia kortteja ei koskaan kohdisteta kilpailijoihin arvaamalla.
    for driver in &mut preview.kilpailijat {
        if driver.tiedot.emit != 0
            && cards
                .get(&driver.tiedot.emit)
                .is_some_and(|count| *count > 1)
        {
            preview.huomautukset.push(ImportIssue {
                estaa_tuonnin: false,
                sijainti: format!("KILP rivi {}", driver.alkuperainen_rivi),
                viesti: format!(
                    "Emit-numero {} esiintyy useasti; kortti jätettiin täydennettäväksi",
                    driver.tiedot.emit
                ),
            });
            driver.tiedot.emit = 0;
        }
    }
    // Puuttuvia ratoja ei päätellä yhdistetyistä luokkatunnuksista.
    for code in preview
        .kilpailijat
        .iter()
        .map(|driver| driver.tiedot.luokka.clone())
        .collect::<BTreeSet<_>>()
    {
        if !preview.radat.iter().any(|track| track.luokka == code) {
            let alias = code
                .strip_prefix("SM-")
                .filter(|suffix| !suffix.contains('+'));
            if alias.is_none_or(|suffix| !preview.radat.iter().any(|track| track.luokka == suffix))
            {
                preview.issue(false, "KILP", format!("Luokan {code} rata puuttuu tai vastaa eri tunnusta. Kilpailijat tuodaan, mutta rata pitää valita ennen tuloslaskentaa."));
            } else {
                preview.issue(false, "KILP", format!("Luokan {code} radaksi tunnistettiin RATA-välilehden {}. Tarkista kohdistus.", alias.unwrap()));
            }
        }
    }
    Ok(preview)
}
