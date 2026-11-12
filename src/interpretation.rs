//! Paperikortin oikean rivin esikatselu. Ei muuta laskennan asemalajeja.
//! Erityisesti tämän mallin L on leima, ei laskentamallin lähtöasema.
use crate::CoreError;
use calamine::{Data, Reader, open_workbook_auto};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRtSummaryRow {
    pub route: u32,
    pub route_name: String,
    pub section: Option<u32>,
    pub station: String,
    pub occurrences: u64,
    pub pairs: Vec<u32>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingRtSummary {
    pub rows: Vec<MissingRtSummaryRow>,
    pub received_pairs: usize,
    pub total_pairs: usize,
    pub unidentified_pairs: usize,
}
/// Koko kilpailun vastaanotetut puutteet. Toistuva tunnus lasketaan esiintyminä,
/// mutta sama kilpailupari kuuluu tunnuksen parimäärään vain kerran.
pub fn missing_rt_summary(project: &crate::native::Project) -> MissingRtSummary {
    let mut groups: BTreeMap<(u32, u32, String), (u64, BTreeSet<u32>)> = BTreeMap::new();
    let mut received = 0;
    let mut unidentified = 0;
    for driver in &project.kilpailijat {
        let Some(class) = project.luokat.iter().find(|c| c.tunnus == driver.luokka) else {
            continue;
        };
        let source = driver.lisavirheet.tulkki_siirto.as_ref();
        received += usize::from(source.is_some());
        let known = source.map_or(0, |s| {
            s.puuttuvat_rt.iter().map(|j| j.kilvet.len()).sum::<usize>()
        });
        if known != driver.lisavirheet.puuttuvat_rt_kpl as usize {
            unidentified += 1;
            if driver.lisavirheet.puuttuvat_rt_kpl > 0 {
                let group = groups
                    .entry((class.rata, u32::MAX, "Yksilöimättä".into()))
                    .or_default();
                group.0 += u64::from(driver.lisavirheet.puuttuvat_rt_kpl);
                group.1.insert(driver.numero);
            }
            continue;
        }
        if let Some(source) = source {
            for section in &source.puuttuvat_rt {
                for label in &section.kilvet {
                    let group = groups
                        .entry((class.rata, section.jakso, label.clone()))
                        .or_default();
                    group.0 += 1;
                    group.1.insert(driver.numero);
                }
            }
        }
    }
    MissingRtSummary {
        rows: groups
            .into_iter()
            .map(
                |((route, section, station), (occurrences, pairs))| MissingRtSummaryRow {
                    route,
                    route_name: project
                        .radat
                        .iter()
                        .find(|r| r.numero == route)
                        .map_or(String::new(), |r| r.nimi.clone()),
                    section: (section != u32::MAX).then_some(section),
                    station,
                    occurrences,
                    pairs: pairs.into_iter().collect(),
                },
            )
            .collect(),
        received_pairs: received,
        total_pairs: project.kilpailijat.len(),
        unidentified_pairs: unidentified,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    Rt,
    Mrt,
    Stamp,
    Jrt,
    At,
    Jat,
    Ula,
    Finish,
}

impl EntryKind {
    pub fn is_clock(self) -> bool {
        matches!(self, Self::At | Self::Jat | Self::Ula | Self::Finish)
    }
    pub fn requires_manual_time(self, emit_enabled: bool) -> bool {
        matches!(self, Self::At | Self::Jat)
            || (!emit_enabled && matches!(self, Self::Ula | Self::Finish))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Lähderivi esikatselua varten, ei pysyvä asematunniste.
    pub source_row: u32,
    pub section: u32,
    pub label: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredEntry {
    pub tunniste: String,
    pub merkinta: Entry,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kasiaika: Option<bool>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub mitatoity: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}

impl StoredEntry {
    pub fn requires_manual_time(&self, emit_enabled: bool) -> bool {
        self.merkinta.kind.is_clock()
            && self
                .kasiaika
                .unwrap_or_else(|| self.merkinta.kind.requires_manual_time(emit_enabled))
    }
}

pub fn set_time_input(
    project: &mut crate::native::Project,
    route: u32,
    entry_id: &str,
    manual: Option<bool>,
) -> Result<(), CoreError> {
    let entry = project
        .oikeat_rivit
        .iter_mut()
        .find(|r| r.rata == route)
        .and_then(|r| r.merkinnat.iter_mut().find(|e| e.tunniste == entry_id))
        .ok_or_else(|| invalid("Valitse tallennettu aikamerkintä"))?;
    if !entry.merkinta.kind.is_clock() {
        return Err(invalid(
            "Ajan syöttötapa voidaan valita vain AT-, JAT-, ULA- tai maaliriville",
        ));
    }
    entry.kasiaika = manual;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredRoute {
    pub rata: u32,
    pub lahde: String,
    pub merkinnat: Vec<StoredEntry>,
    #[serde(default)]
    pub seuraava_tunniste: u64,
}

/// Ensimmäinen tuonti luo tunnisteet. Sama tuonti säilyttää ne; erilaista
/// aineistoa ei vielä korvata ilman tulevaa tulkkausten sovitusmenettelyä.
pub fn import_route(
    project: &mut crate::native::Project,
    route: u32,
    preview: &Preview,
) -> Result<(), CoreError> {
    if !preview.can_publish() {
        return Err(invalid(
            "Korjaa oikean rivin huomautukset ennen tallennusta",
        ));
    }
    if !project.radat.iter().any(|r| r.numero == route) {
        return Err(invalid("Valittua rataa ei löydy kilpailusta"));
    }
    if let Some(existing) = project.oikeat_rivit.iter().find(|r| r.rata == route) {
        if existing
            .merkinnat
            .iter()
            .map(|e| &e.merkinta)
            .eq(preview.entries.iter())
        {
            return Ok(());
        }
        return Err(invalid(
            "Radalla on jo erilainen oikea rivi. Sen korvaamista ei vielä tueta",
        ));
    }
    let stored = StoredRoute {
        rata: route,
        lahde: preview.sheet.clone(),
        seuraava_tunniste: preview.entries.len() as u64 + 1,
        merkinnat: preview
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| StoredEntry {
                tunniste: format!("r{route}-e{}", i + 1),
                merkinta: entry.clone(),
                kasiaika: None,
                mitatoity: false,
            })
            .collect(),
    };
    let mut candidate = project.clone();
    candidate.oikeat_rivit.push(stored);
    candidate.validate()?;
    *project = candidate;
    Ok(())
}

/// Sama kilpi kohdistetaan jakson, lajin, tunnuksen ja esiintymäjärjestyksen mukaan.
/// Uusi tunniste ei koskaan käytä uudelleen aiemmin poistettua esiintymää.
pub fn replace_route(
    project: &mut crate::native::Project,
    route: u32,
    preview: &Preview,
) -> Result<(), CoreError> {
    if !preview.can_publish() {
        return Err(invalid(
            "Korjaa oikean rivin huomautukset ennen tallennusta",
        ));
    }
    let Some(index) = project.oikeat_rivit.iter().position(|r| r.rata == route) else {
        return import_route(project, route, preview);
    };
    let old = &project.oikeat_rivit[index];
    let mut used = BTreeSet::new();
    let mut next = old.seuraava_tunniste.max(
        old.merkinnat
            .iter()
            .filter_map(|e| {
                e.tunniste
                    .rsplit_once("-e")
                    .and_then(|(_, n)| n.parse::<u64>().ok())
            })
            .max()
            .unwrap_or(0)
            + 1,
    );
    let mut entries = Vec::new();
    for entry in &preview.entries {
        if let Some((i, saved)) = old.merkinnat.iter().enumerate().find(|(i, s)| {
            !used.contains(i)
                && s.merkinta.section == entry.section
                && s.merkinta.kind == entry.kind
                && s.merkinta.label == entry.label
        }) {
            used.insert(i);
            let mut saved = saved.clone();
            saved.merkinta = entry.clone();
            entries.push(saved);
        } else {
            entries.push(StoredEntry {
                tunniste: format!("r{route}-e{next}"),
                merkinta: entry.clone(),
                kasiaika: None,
                mitatoity: false,
            });
            next = next
                .checked_add(1)
                .ok_or_else(|| invalid("Asematunnisteet loppuivat"))?;
        }
    }
    if old
        .merkinnat
        .iter()
        .enumerate()
        .any(|(i, e)| e.mitatoity && !used.contains(&i))
    {
        return Err(invalid(
            "Mitätöity kilpi on säilytettävä oikeassa rivissä. Sitä ei voi poistaa tai ottaa takaisin käyttöön.",
        ));
    }
    let mut candidate = project.clone();
    candidate.oikeat_rivit[index] = StoredRoute {
        rata: route,
        lahde: preview.sheet.clone(),
        merkinnat: entries,
        seuraava_tunniste: next,
    };
    candidate.validate()?;
    *project = candidate;
    Ok(())
}

pub fn validate_stored(project: &crate::native::Project) -> Result<(), CoreError> {
    let mut routes = std::collections::HashSet::new();
    let mut ids = std::collections::HashSet::new();
    for stored in &project.oikeat_rivit {
        if !routes.insert(stored.rata) || !project.radat.iter().any(|r| r.numero == stored.rata) {
            return Err(invalid("Oikean rivin rata puuttuu tai toistuu"));
        }
        if stored.merkinnat.len() > 10_000 || stored.lahde.trim().is_empty() {
            return Err(invalid("Oikean rivin lähde tai pituus on virheellinen"));
        }
        let rows: Vec<_> = stored
            .merkinnat
            .iter()
            .map(|e| (e.merkinta.source_row, e.merkinta.label.clone()))
            .collect();
        let parsed = preview_rows(&stored.lahde, &rows);
        if !parsed.can_publish()
            || !parsed
                .entries
                .iter()
                .eq(stored.merkinnat.iter().map(|e| &e.merkinta))
        {
            return Err(invalid(
                "Tallennetun oikean rivin jaksot tai merkinnät ovat virheellisiä",
            ));
        }
        for entry in &stored.merkinnat {
            if entry.kasiaika.is_some() && !entry.merkinta.kind.is_clock() {
                return Err(invalid("Kilvelle ei voi määrittää ajan syöttötapaa"));
            }
            if entry.mitatoity && entry.merkinta.kind.is_clock() {
                return Err(invalid("Aika-asemaa ei voi mitätöidä RT-kilpenä"));
            }
            if entry.tunniste.trim().is_empty() || !ids.insert(&entry.tunniste) {
                return Err(invalid(
                    "Oikean rivin esiintymätunniste puuttuu tai toistuu",
                ));
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Issue {
    pub source_row: u32,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub sheet: String,
    pub entries: Vec<Entry>,
    pub issues: Vec<Issue>,
}

impl Preview {
    pub fn can_publish(&self) -> bool {
        !self.entries.is_empty() && self.issues.is_empty()
    }
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

/// Tunnistaa vain sovitut tunnukset. ?? jää korjattavaksi esikatseluun.
fn parse_label(value: &str) -> Option<(String, EntryKind)> {
    let text = value.trim().to_uppercase();
    if !text.is_empty() && text.len() <= 2 && text.bytes().all(|b| b.is_ascii_digit()) {
        let number: u8 = text.parse().ok()?;
        return (number <= 99).then(|| (format!("{number:02}"), EntryKind::Rt));
    }
    for (prefix, kind) in [
        ("AT", EntryKind::At),
        ("JAT", EntryKind::Jat),
        ("ULA", EntryKind::Ula),
    ] {
        if let Some(suffix) = text.strip_prefix(prefix) {
            let suffix = suffix.trim();
            if !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()) {
                return Some((format!("{prefix}{suffix}"), kind));
            }
        }
    }
    let kind = match text.as_str() {
        "MRT" => EntryKind::Mrt,
        "L" => EntryKind::Stamp,
        "JRT" => EntryKind::Jrt,
        "AT" => EntryKind::At,
        "JAT" => EntryKind::Jat,
        "ULA" => EntryKind::Ula,
        "M" => EntryKind::Finish,
        _ => return None,
    };
    Some((text, kind))
}

/// Lukee rivikohtaiset arvot. Tyhjät välirivit ilmoitetaan, ei tulkita jaksoiksi.
/// JAT ja välittömästi seuraava ULA kuuluvat päättyvään jaksoon.
pub fn preview_rows(sheet: &str, rows: &[(u32, String)]) -> Preview {
    let mut preview = Preview {
        sheet: sheet.into(),
        entries: Vec::new(),
        issues: Vec::new(),
    };
    let mut section = 1;
    let mut pending_boundary = false;
    let mut finished = false;
    for (source_row, value) in rows {
        let Some((label, kind)) = parse_label(value) else {
            preview.issues.push(Issue {
                source_row: *source_row,
                message: if value.trim() == "??" {
                    "Oikean rivin tunnus on keskeneräinen".into()
                } else {
                    format!("Tuntematon tai tyhjä oikean rivin tunnus: {value:?}")
                },
            });
            continue;
        };
        if finished {
            preview.issues.push(Issue {
                source_row: *source_row,
                message: "Maalin jälkeen on merkintöjä".into(),
            });
        }
        let follows_jat = pending_boundary;
        if pending_boundary {
            if kind != EntryKind::Ula {
                section += 1;
            }
            pending_boundary = false;
        }
        if kind == EntryKind::Ula && !follows_jat {
            preview.issues.push(Issue {
                source_row: *source_row,
                message: "ULA ei seuraa JAT-merkintää".into(),
            });
        }
        preview.entries.push(Entry {
            source_row: *source_row,
            section,
            label,
            kind,
        });
        match kind {
            EntryKind::Jat => pending_boundary = true,
            EntryKind::Ula if follows_jat => section += 1,
            EntryKind::Finish => finished = true,
            _ => {}
        }
    }
    if preview.entries.is_empty() {
        preview.issues.push(Issue {
            source_row: 1,
            message: "Oikea rivi on tyhjä".into(),
        });
    }
    preview
}

/// Lukee Syotto-välilehtien A-sarakkeet ilman Exceliä tai makrojen suoritusta.
/// Rataan kohdistaminen ja pysyvät tunnisteet annetaan vasta tuonnin vahvistuksessa.
pub fn preview_workbook(path: &Path) -> Result<Vec<Preview>, CoreError> {
    let metadata = std::fs::metadata(path)
        .map_err(|_| invalid("Oikean rivin Excel-tiedostoa ei löydy tai sitä ei voi lukea"))?;
    if !metadata.is_file() || metadata.len() > 30 * 1024 * 1024 {
        return Err(invalid(
            "Excel-tiedoston pitää olla tavallinen, enintään 30 megatavun tiedosto",
        ));
    }
    let mut workbook = open_workbook_auto(path)
        .map_err(|_| invalid("Oikean rivin Excel-tiedostoa ei voi lukea"))?;
    let sheets: Vec<_> = workbook
        .sheet_names()
        .iter()
        .filter(|name| name.starts_with("Syotto_"))
        .cloned()
        .collect();
    if sheets.is_empty() {
        return Err(invalid("Tiedostossa ei ole Syotto_-alkuisia välilehtiä"));
    }
    let mut previews = Vec::new();
    for sheet in sheets {
        let range = workbook
            .worksheet_range(&sheet)
            .map_err(|_| invalid(format!("Välilehteä {sheet} ei voi lukea")))?;
        let last = range
            .used_cells()
            .filter(|(_, col, cell)| *col == 0 && !matches!(cell, Data::Empty))
            .map(|(row, _, _)| row)
            .max();
        // used_cells-indeksit ovat alueen suhteellisia, get_value käyttää absoluuttisia.
        let start = range.start().unwrap_or((0, 0));
        let mut rows = Vec::new();
        if start.1 == 0
            && let Some(last) = last
        {
            if u64::from(start.0) + last as u64 >= 10_000 {
                return Err(invalid("Oikean rivin enimmäispituus on 10 000 riviä"));
            }
            for row in 0..=start.0 + last as u32 {
                let value = match range.get_value((row, 0)).unwrap_or(&Data::Empty) {
                    Data::String(text) => text.trim().to_owned(),
                    Data::Int(value) => value.to_string(),
                    Data::Float(value) if value.is_finite() && value.fract() == 0.0 => {
                        format!("{value:.0}")
                    }
                    Data::Empty => String::new(),
                    other => format!("{other}"),
                };
                rows.push((row + 1, value));
            }
        }
        previews.push(preview_rows(&sheet, &rows));
    }
    Ok(previews)
}
