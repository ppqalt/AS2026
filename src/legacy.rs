//! Read-only import of the current text formats recovered from AS2015A.
//!
//! Raw file bytes and trailing field spaces are retained; leading spaces follow legacy tokenization. Windows CRLF is normalized
//! for tokenization. Unlike the unsafe legacy tokenizer, a final unterminated
//! field is retained with a warning; overlong fields fail rather than truncate.
use std::{collections::BTreeMap, fmt, fs, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Encoding {
    Windows1252,
    Utf8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportError(pub String);
impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ImportError {}
fn error(message: impl Into<String>) -> ImportError {
    ImportError(message.into())
}

#[derive(Debug, Clone)]
pub struct Record {
    pub line: usize,
    pub fields: Vec<String>,
}
impl Record {
    pub fn tag(&self) -> &str {
        self.fields.first().map_or("", |s| s.trim())
    }
    fn data(&self, count: usize) -> Result<&[String], ImportError> {
        fields(&self.fields, count + 1, self.line).map(|all| &all[1..])
    }
}
#[derive(Debug, Clone)]
pub struct Document {
    pub raw: Vec<u8>,
    pub encoding: Encoding,
    pub records: Vec<Record>,
    pub warnings: Vec<String>,
}

pub fn decode(bytes: &[u8], encoding: Encoding) -> Result<String, ImportError> {
    if encoding == Encoding::Utf8 {
        return String::from_utf8(bytes.to_vec()).map_err(|e| {
            error(format!(
                "Virheellinen UTF-8 tavun {} kohdalla",
                e.utf8_error().valid_up_to() + 1
            ))
        });
    }
    // C1 bytes undefined by CP1252 are preserved as the corresponding controls.
    // This avoids replacement-character data loss; raw bytes remain authoritative.
    const C1: [u32; 32] = [
        0x20ac, 0x81, 0x201a, 0x192, 0x201e, 0x2026, 0x2020, 0x2021, 0x2c6, 0x2030, 0x160, 0x2039,
        0x152, 0x8d, 0x17d, 0x8f, 0x90, 0x2018, 0x2019, 0x201c, 0x201d, 0x2022, 0x2013, 0x2014,
        0x2dc, 0x2122, 0x161, 0x203a, 0x153, 0x9d, 0x17e, 0x178,
    ];
    Ok(bytes
        .iter()
        .map(|&b| {
            let n = if (0x80..=0x9f).contains(&b) {
                C1[usize::from(b - 0x80)]
            } else {
                u32::from(b)
            };
            char::from_u32(n).expect("Merkistötaulukon merkki on kelvollinen")
        })
        .collect())
}

pub fn parse_document(raw: &[u8], encoding: Encoding) -> Result<Document, ImportError> {
    if raw.contains(&0) {
        return Err(error("Tekstitiedostossa on NUL-tavu"));
    }
    let mut records = Vec::new();
    let mut warnings = Vec::new();
    let mut line = 1usize;
    let mut start = 0;
    for (index, &b) in raw.iter().enumerate() {
        if b == b'\n' || b == 12 {
            let mut record = &raw[start..index];
            if record.last() == Some(&b'\r') {
                record = &record[..record.len() - 1];
            }
            parse_record(record, line, encoding, &mut records)?;
            line += 1;
            start = index + 1;
        }
    }
    if start < raw.len() {
        warnings.push(format!("Rivi {line}: viimeinen kenttä säilytettiin ilman rivinvaihtoa; vanha lukija voi pudottaa sen"));
        parse_record(&raw[start..], line, encoding, &mut records)?;
    }
    Ok(Document {
        raw: raw.to_vec(),
        encoding,
        records,
        warnings,
    })
}
fn parse_record(
    raw: &[u8],
    line: usize,
    encoding: Encoding,
    out: &mut Vec<Record>,
) -> Result<(), ImportError> {
    if raw.iter().all(|b| b.is_ascii_whitespace()) {
        return Ok(());
    }
    let mut result = Vec::new();
    for token in raw.split(|b| *b == b';' || *b == b':') {
        let token = token.strip_prefix(b" ").map_or(token, |_| {
            &token[token.iter().position(|b| *b != b' ').unwrap_or(token.len())..]
        });
        if token.len() > 30 {
            return Err(error(format!(
                "Rivi {line}: yli 30 tavun kenttää ei typistetä"
            )));
        }
        if token.contains(&b'\r') {
            return Err(error(format!(
                "Rivi {line}: erillinen CR-tavu ei ole tuettu"
            )));
        }
        result.push(decode(token, encoding)?);
    }
    out.push(Record {
        line,
        fields: result,
    });
    Ok(())
}
fn fields(values: &[String], count: usize, line: usize) -> Result<&[String], ImportError> {
    if values.len() < count || values.iter().skip(count).any(|f| !f.trim().is_empty()) {
        return Err(error(format!(
            "Rivi {line}: odotettiin {count} kenttää, saatiin {}",
            values.len()
        )));
    }
    Ok(&values[..count])
}
fn number(s: &str, line: usize, label: &str) -> Result<i64, ImportError> {
    s.trim().parse().map_err(|_| {
        error(format!(
            "Rivi {line}: {label} ei ole yksiselitteinen kokonaisluku: {s:?}"
        ))
    })
}
fn unsigned(s: &str, line: usize, label: &str) -> Result<u32, ImportError> {
    let n = number(s, line, label)?;
    u32::try_from(n).map_err(|_| {
        error(format!(
            "Rivi {line}: {label} ei mahdu positiiviseen 32-bittiseen arvoon"
        ))
    })
}
fn text(s: &str) -> String {
    s.trim_end_matches(' ').to_owned()
}

#[derive(Debug, Clone)]
pub struct Route {
    pub index: u32,
    pub name: String,
    pub declared_station_count: u32,
    pub stations: Vec<RouteStationRecord>,
}
#[derive(Debug, Clone)]
pub struct RouteStationRecord {
    pub index: u32,
    pub station_type: String,
    pub expected_code: u32,
    pub name: String,
    pub rule: i64,
    pub minute_text: String,
    pub seconds: i64,
    pub interval: i64,
    pub section_text: String,
}
pub fn parse_routes(document: &Document) -> Result<Vec<Route>, ImportError> {
    let mut routes: BTreeMap<u32, Route> = BTreeMap::new();
    for record in &document.records {
        match record.tag() {
            "R" => {
                let f = record.data(3)?;
                let index = unsigned(&f[0], record.line, "rata")?;
                if index == 0 || routes.contains_key(&index) {
                    return Err(error(format!(
                        "Rivi {}: nolla tai toistuva rata",
                        record.line
                    )));
                }
                routes.insert(
                    index,
                    Route {
                        index,
                        name: text(&f[1]),
                        declared_station_count: unsigned(&f[2], record.line, "asemamäärä")?,
                        stations: Vec::new(),
                    },
                );
            }
            "L" => {
                let f = record.data(10)?;
                let index = unsigned(&f[0], record.line, "rata")?;
                let route = routes.get_mut(&index).ok_or_else(|| {
                    error(format!("Rivi {}: asema ennen radan otsikkoa", record.line))
                })?;
                let station_index = unsigned(&f[1], record.line, "asema")?;
                if station_index == 0 || route.stations.iter().any(|s| s.index == station_index) {
                    return Err(error(format!(
                        "Rivi {}: nolla tai toistuva asema",
                        record.line
                    )));
                }
                route.stations.push(RouteStationRecord {
                    index: station_index,
                    station_type: text(&f[2]),
                    expected_code: unsigned(&f[3], record.line, "Emit-koodi")?,
                    name: text(&f[4]),
                    rule: number(&f[5], record.line, "sääntö")?,
                    minute_text: text(&f[6]),
                    seconds: number(&f[7], record.line, "Sek")?,
                    interval: number(&f[8], record.line, "Väli")?,
                    section_text: text(&f[9]),
                });
            }
            tag => {
                return Err(error(format!(
                    "Rivi {}: tuntematon ratatietue {tag:?}",
                    record.line
                )));
            }
        }
    }
    for route in routes.values_mut() {
        route.stations.sort_by_key(|s| s.index);
    }
    Ok(routes.into_values().collect())
}

#[derive(Debug, Clone)]
pub struct StationResult {
    pub output_index: u32,
    pub sequence: u32,
    pub station_type: String,
    pub name: String,
    pub elapsed_seconds: i64,
    pub expected_code: u32,
    pub received_code: u32,
    pub target_seconds: i64,
    pub actual_seconds: i64,
    pub clock: [i64; 3],
    pub rule: i64,
    pub section_text: String,
    pub signed_time_penalty: i64,
    pub other_penalty: i64,
    pub total_penalty: i64,
    pub note: String,
}
#[derive(Debug, Clone)]
pub struct Competitor {
    pub number: u32,
    pub class_code: String,
    pub emit_card: String,
    pub driver: String,
    pub navigator: String,
    pub header_fields: Vec<String>,
    pub total_penalties: i64,
    pub declared_row_count: u32,
    pub status: String,
    pub correction_seconds: i64,
    pub results: Vec<StationResult>,
}
pub fn parse_competitors(document: &Document) -> Result<Vec<Competitor>, ImportError> {
    let mut result: Vec<Competitor> = Vec::new();
    for record in &document.records {
        match record.tag() {
            "L" => {
                let f = record.data(31)?;
                let n = unsigned(&f[0], record.line, "kilpailunumero")?;
                if result.iter().any(|c| c.number == n) {
                    return Err(error(format!(
                        "Rivi {}: toistuva kilpailunumero {n}",
                        record.line
                    )));
                }
                result.push(Competitor {
                    number: n,
                    class_code: text(&f[1]),
                    emit_card: text(&f[2]),
                    driver: text(&f[3]),
                    navigator: text(&f[4]),
                    header_fields: f.to_vec(),
                    total_penalties: number(&f[26], record.line, "virhepisteet")?,
                    declared_row_count: unsigned(&f[27], record.line, "rivimäärä")?,
                    status: text(&f[28]),
                    correction_seconds: number(&f[30], record.line, "Emit-korjaus")?,
                    results: Vec::new(),
                });
            }
            "P" => {
                let f = record.data(18)?;
                let c = result.last_mut().ok_or_else(|| {
                    error(format!("Rivi {}: P-tietue ennen kilpailijaa", record.line))
                })?;
                let n = |i: usize, label: &str| number(f[i].as_str(), record.line, label);
                c.results.push(StationResult {
                    output_index: unsigned(&f[0], record.line, "rivi")?,
                    sequence: unsigned(&f[1], record.line, "asema")?,
                    station_type: text(&f[2]),
                    name: text(&f[3]),
                    elapsed_seconds: n(4, "eaika")?,
                    expected_code: unsigned(&f[5], record.line, "okoodi")?,
                    received_code: unsigned(&f[6], record.line, "kkoodi")?,
                    target_seconds: n(7, "oaika")?,
                    actual_seconds: n(8, "kaika")?,
                    clock: [n(9, "tunti")?, n(10, "minuutti")?, n(11, "sekunti")?],
                    rule: n(12, "sääntö")?,
                    section_text: text(&f[13]),
                    signed_time_penalty: n(14, "aikavirhe")?,
                    other_penalty: n(15, "muuvirhe")?,
                    total_penalty: n(16, "yhtvirhe")?,
                    note: text(&f[17]),
                });
            }
            "K" => {
                return Err(error(format!(
                    "Rivi {}: vanha K-otsikko säilyy raakadatassa, mutta sen tulkintaa ei vielä tueta",
                    record.line
                )));
            }
            tag => {
                return Err(error(format!(
                    "Rivi {}: tuntematon kilpailijatietue {tag:?}",
                    record.line
                )));
            }
        }
    }
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct ClassDefinition {
    pub code: String,
    pub name: String,
    pub route: String,
    pub fields: Vec<String>,
}
#[derive(Debug, Clone, Default)]
pub struct BasicSettings {
    pub identity: Vec<String>,
    pub output_names: Vec<String>,
    pub classes: Vec<ClassDefinition>,
    pub combined_classes: Vec<Vec<String>>,
}
pub fn parse_basic(document: &Document) -> Result<BasicSettings, ImportError> {
    let mut result = BasicSettings::default();
    for record in &document.records {
        match record.tag() {
            "PE" => {
                if !result.identity.is_empty() {
                    return Err(error("Useita PE-otsikoita"));
                }
                result.identity = record.data(8)?.to_vec();
            }
            "TU" => result.output_names = record.data(2)?.to_vec(),
            "Y" => result.combined_classes.push(record.fields[1..].to_vec()),
            _ => {
                let f = fields(&record.fields, 10, record.line)?;
                let code = text(&f[0]);
                if result.classes.iter().any(|class| class.code == code) {
                    return Err(error(format!(
                        "Rivi {}: toistuva luokkakoodi {code:?}; luokan rataa ei voi valita yksiselitteisesti",
                        record.line
                    )));
                }
                result.classes.push(ClassDefinition {
                    code,
                    name: text(&f[1]),
                    route: text(&f[2]),
                    fields: f.to_vec(),
                });
            }
        }
    }
    if result.identity.is_empty() {
        return Err(error(
            "Nykyinen PE-otsikko puuttuu; vanhaa perustietomuotoa ei arvata",
        ));
    }
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct Archive {
    pub documents: BTreeMap<String, Document>,
    pub parameters: Vec<Option<i64>>,
    pub basic: BasicSettings,
    pub routes: Vec<Route>,
    pub competitors: Vec<Competitor>,
    pub warnings: Vec<String>,
}
impl Archive {
    /// Reads fixed filenames only. Never writes, recalculates or changes source files.
    pub fn load(directory: &Path, encoding: Encoding) -> Result<Self, ImportError> {
        let names = [
            "Aspara.txt",
            "Asperus.txt",
            "Radat.txt",
            "Kilp.txt",
            "emit.txt",
            "sarjap.txt",
        ];
        let entries = fs::read_dir(directory)
            .map_err(|e| error(format!("Hakemisto: {}", crate::io_message(&e))))?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| error(crate::io_message(&e)))?;
        let mut documents = BTreeMap::new();
        let mut warnings = Vec::new();
        for name in names {
            let matches: Vec<_> = entries
                .iter()
                .filter(|p| {
                    p.file_name()
                        .is_some_and(|n| n.to_string_lossy().eq_ignore_ascii_case(name))
                })
                .collect();
            if matches.len() > 1 {
                return Err(error(format!("Useita samannimisiä tiedostoja: {name}")));
            }
            let Some(path) = matches.first() else {
                if name == "emit.txt" || name == "sarjap.txt" {
                    if name == "emit.txt" {
                        warnings.push("emit.txt puuttuu; raakaleimauksia ei voi verrata".into());
                    }
                    continue;
                }
                return Err(error(format!("Tiedosto puuttuu: {name}")));
            };
            let metadata = fs::metadata(path).map_err(|e| error(crate::io_message(&e)))?;
            if !metadata.is_file() {
                return Err(error(format!("Ei tavallinen tekstitiedosto: {name}")));
            }
            if metadata.len() > 64 * 1024 * 1024 {
                return Err(error(format!("Yli 64 MiB tiedostoa ei lueta: {name}")));
            }
            let bytes =
                fs::read(path).map_err(|e| error(format!("{name}: {}", crate::io_message(&e))))?;
            let document = parse_document(&bytes, encoding)?;
            warnings.extend(document.warnings.iter().map(|w| format!("{name}: {w}")));
            documents.insert(name.to_owned(), document);
        }
        let doc = &documents["Aspara.txt"];
        if doc.records.len() != 1 {
            return Err(error("Aspara.txt: odotettiin yhtä parametririviä"));
        }
        let row = &doc.records[0];
        if row.fields.len() < 11 {
            return Err(error("Aspara.txt: alle 11 parametria"));
        }
        if row.fields.iter().skip(13).any(|s| !s.trim().is_empty()) {
            return Err(error("Aspara.txt: yli 13 parametria ei tulkita"));
        }
        if row.fields.iter().skip(11).any(|s| !s.trim().is_empty()) {
            warnings.push(
                "Aspara.txt: ylimääräiset parametrit säilytetään mutta niitä ei käytetä".into(),
            );
        }
        let values = &row.fields[..11];
        let parameters = values
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if s.trim().is_empty() {
                    Ok(None)
                } else {
                    number(s, row.line, &format!("parametri {}", i + 1)).map(Some)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let basic = parse_basic(&documents["Asperus.txt"])?;
        let routes = parse_routes(&documents["Radat.txt"])?;
        let competitors = parse_competitors(&documents["Kilp.txt"])?;
        for route in &routes {
            if route
                .stations
                .iter()
                .enumerate()
                .any(|(i, s)| u32::try_from(i + 1).ok() != Some(s.index))
            {
                warnings.push(format!(
                    "Rata {}: aukolliset asemaindeksit; reittimallia ei muodosteta",
                    route.index
                ));
            }
            if usize::try_from(route.declared_station_count).ok() != Some(route.stations.len()) {
                warnings.push(format!(
                    "Rata {}: ilmoitettu {} asemaa, luettu {}",
                    route.index,
                    route.declared_station_count,
                    route.stations.len()
                ));
            }
        }
        for c in &competitors {
            if usize::try_from(c.declared_row_count).ok() != Some(c.results.len()) {
                warnings.push(format!(
                    "Kilpailija {}: ilmoitettu {} tulosriviä, luettu {}",
                    c.number,
                    c.declared_row_count,
                    c.results.len()
                ));
            }
            if !basic.classes.iter().any(|class| class.code == c.class_code) {
                warnings.push(format!(
                    "Kilpailijan {} luokkaa {:?} ei löydy",
                    c.number, c.class_code
                ));
            }
            for row in &c.results {
                if let Some(expected) = row
                    .signed_time_penalty
                    .checked_abs()
                    .and_then(|p| p.checked_add(row.other_penalty))
                {
                    if expected != row.total_penalty {
                        warnings.push(format!(
                            "Kilpailija {}, rivi {}: tallennettu summa poikkeaa rivin osista",
                            c.number, row.output_index
                        ));
                    }
                } else {
                    warnings.push(format!(
                        "Kilpailija {}, rivi {}: summan ylivuoto",
                        c.number, row.output_index
                    ));
                }
            }
        }
        Ok(Self {
            documents,
            parameters,
            basic,
            routes,
            competitors,
            warnings,
        })
    }
}

impl Route {
    /// The source file addresses fixed indices. Do not silently compact gaps for matching.
    pub fn to_core(&self) -> Result<Vec<crate::RouteStation>, ImportError> {
        if usize::try_from(self.declared_station_count).ok() != Some(self.stations.len()) {
            return Err(error(
                "Radan ilmoitettu asemamäärä poikkeaa luetuista asemista",
            ));
        }
        for (i, station) in self.stations.iter().enumerate() {
            if u32::try_from(i + 1).ok() != Some(station.index) {
                return Err(error(
                    "Aukollisia asemaindeksejä ei tiivistetä reittimallissa",
                ));
            }
        }
        self.stations
            .iter()
            .map(RouteStationRecord::to_core)
            .collect()
    }
}
impl RouteStationRecord {
    pub fn to_core(&self) -> Result<crate::RouteStation, ImportError> {
        if self.seconds < 0 {
            return Err(error("Negatiivinen Sek-arvo ei ole tuettu reittimallissa"));
        }
        Ok(crate::RouteStation {
            sequence: self.index,
            kind: crate::StationKind::from_legacy(&self.station_type)
                .map_err(|e| error(e.to_string()))?,
            label: self.name.clone(),
            expected_code: self.expected_code,
            target_seconds: self.seconds,
            rule: crate::Rule::from_legacy(self.rule).map_err(|e| error(e.to_string()))?,
            section_time_text: self.section_text.clone(),
        })
    }
}
impl Archive {
    pub fn scoring_parameters(&self) -> Result<crate::scoring::ScoringParameters, ImportError> {
        let mut p = crate::scoring::ScoringParameters::default();
        let targets = [
            &mut p.tolerance_seconds,
            &mut p.early_points_per_second,
            &mut p.late_points_per_second,
            &mut p.missing_at,
            &mut p.extra_at,
            &mut p.missing_jat,
            &mut p.section_limit_minutes,
            &mut p.at_cap,
            &mut p.jat_cap,
            &mut p.missing_rt,
            &mut p.extra_rt,
        ];
        for (target, value) in targets.into_iter().zip(&self.parameters) {
            if let Some(value) = value {
                *target = *value;
            }
        }
        p.validate().map_err(|e| error(e.to_string()))?;
        Ok(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cp1252_and_raw_are_preserved() {
        let raw = b"R;1;M\xe4ki;0\r\n";
        let d = parse_document(raw, Encoding::Windows1252).unwrap();
        assert_eq!(d.raw, raw);
        assert_eq!(d.records[0].fields[2], "Mäki");
    }
    #[test]
    fn colon_is_structural_and_empty_fields_survive() {
        let d = parse_document(b"A;  1::x;\r\n", Encoding::Windows1252).unwrap();
        assert_eq!(d.records[0].fields, vec!["A", "1", "", "x", ""]);
    }
    #[test]
    fn overlong_tokens_are_rejected_not_truncated() {
        assert!(
            parse_document(
                b"A;1234567890123456789012345678901;\n",
                Encoding::Windows1252
            )
            .is_err()
        );
    }
    #[test]
    fn unterminated_final_field_is_retained_with_warning() {
        let d = parse_document(b"R;1;Name;0", Encoding::Windows1252).unwrap();
        assert_eq!(d.records[0].fields[3], "0");
        assert_eq!(d.warnings.len(), 1);
    }
    #[test]
    fn station_before_header_is_rejected() {
        let d = parse_document(b"L;1;1;AT;31;A;0;5,00;300;300;;\n", Encoding::Windows1252).unwrap();
        assert!(parse_routes(&d).is_err());
    }
    #[test]
    fn p_record_file_order_is_not_grid_order() {
        let d = parse_document(
            b"P;1;2;AT;A;420;31;32;300;310;12;0;0;0;5;-5;0;5;;\n",
            Encoding::Windows1252,
        )
        .unwrap();
        assert!(parse_competitors(&d).is_err()); // no silent attachment to an invented competitor
        let header = "L;7;A;123;Driver;Navigator;1;2;Club;Club;Car;12;0;0;0;10;0;12;10;0;5;0;0;0;0;0;0;5;1;TRK;;120;\n";
        let input = format!("{header}P;1;2;AT;A;420;31;32;300;310;12;0;0;0;5;-5;0;5;;\n");
        let c =
            parse_competitors(&parse_document(input.as_bytes(), Encoding::Windows1252).unwrap())
                .unwrap();
        let row = &c[0].results[0];
        assert_eq!(
            (
                row.elapsed_seconds,
                row.expected_code,
                row.received_code,
                row.target_seconds
            ),
            (420, 31, 32, 300)
        );
    }
    #[test]
    fn sparse_route_indices_are_retained_but_never_compacted_into_a_model() {
        let d = parse_document(
            b"R;1;DEMO;2\nL;1;1;L;0;Start;0;0,00;0;0;;\nL;1;3;M;0;Finish;0;5,00;300;0;;\n",
            Encoding::Windows1252,
        )
        .unwrap();
        let routes = parse_routes(&d).unwrap();
        assert_eq!(routes[0].stations[1].index, 3);
        assert!(routes[0].to_core().is_err());
    }
}

#[derive(Debug, Clone)]
pub struct SavedEmitReadout {
    pub line: usize,
    pub card_number: u32,
    pub records: Vec<crate::Punch>,
    pub readout_clock: crate::ClockTime,
}
impl SavedEmitReadout {
    pub fn accepted(&self) -> Result<crate::emit::EmitReadout, crate::CoreError> {
        crate::emit::accepted_records(&self.records, self.readout_clock)
    }
}
pub fn parse_saved_emit(document: &Document) -> Result<Vec<SavedEmitReadout>, ImportError> {
    let mut output = Vec::new();
    for row in &document.records {
        if row.tag() != "999" {
            return Err(error(format!(
                "emit.txt rivi {}: tuntematon tietue",
                row.line
            )));
        }
        let prefix = row
            .fields
            .get(1..3)
            .ok_or_else(|| error(format!("emit.txt rivi {}: otsikko puuttuu", row.line)))?;
        let card_number = unsigned(&prefix[0], row.line, "kortti")?;
        let count = unsigned(&prefix[1], row.line, "leimausten määrä")?;
        if !(1..=50).contains(&count) {
            return Err(error(format!(
                "emit.txt rivi {}: leimausten määrä ei ole 1–50",
                row.line
            )));
        }
        let count = count as usize;
        let f = row.data(5 + 2 * count)?;
        let mut records = Vec::with_capacity(count);
        for index in 0..count {
            records.push(crate::Punch {
                code: unsigned(&f[2 + 2 * index], row.line, "koodi")?,
                elapsed_seconds: unsigned(&f[3 + 2 * index], row.line, "leimausaika")?,
            });
        }
        let clock = 2 + 2 * count;
        let readout_clock = crate::ClockTime::new(
            unsigned(&f[clock], row.line, "tunti")?,
            unsigned(&f[clock + 1], row.line, "minuutti")?,
            unsigned(&f[clock + 2], row.line, "sekunti")?,
        )
        .map_err(|e| error(e.to_string()))?;
        output.push(SavedEmitReadout {
            line: row.line,
            card_number,
            records,
            readout_clock,
        });
    }
    Ok(output)
}
