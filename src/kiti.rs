//! Fill an existing KITI workbook without rebuilding or exposing its identity data.
use crate::{
    CoreError, application,
    native::Project,
    ranking::{self, TiePolicy},
};
use calamine::{Data, Reader};
use quick_xml::{
    Reader as XmlReader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event},
};
use std::{
    collections::BTreeMap,
    io::{Cursor, Read, Write},
    path::Path,
};
fn invalid(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn xml_error(e: impl std::fmt::Display) -> CoreError {
    invalid(format!("KITI-pohjan XML: {e}"))
}
fn attr(e: &BytesStart<'_>, name: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.as_ref() == name)
        .and_then(|a| {
            std::str::from_utf8(&a.value)
                .ok()
                .and_then(|v| quick_xml::escape::unescape(v).ok())
                .map(|v| v.into_owned())
        })
}
fn text(cell: Option<&Data>) -> String {
    cell.map(ToString::to_string).unwrap_or_default()
}
fn read_zip(zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>, name: &str) -> Result<Vec<u8>, CoreError> {
    let file = zip
        .by_name(name)
        .map_err(|_| invalid("KITI-pohjan Excel-rakenne on puutteellinen"))?;
    if file.size() > 32 * 1024 * 1024 {
        return Err(invalid("KITI-pohjan osa on liian suuri"));
    }
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(xml_error)?;
    Ok(bytes)
}
fn sheet_path(zip: &mut zip::ZipArchive<Cursor<Vec<u8>>>) -> Result<String, CoreError> {
    let workbook = read_zip(zip, "xl/workbook.xml")?;
    let mut reader = XmlReader::from_reader(workbook.as_slice());
    let mut id = None;
    loop {
        match reader.read_event().map_err(xml_error)? {
            Event::Empty(e) | Event::Start(e)
                if e.local_name().as_ref() == b"sheet"
                    && attr(&e, b"name").as_deref() == Some("Results") =>
            {
                id = attr(&e, b"r:id");
                break;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    let id = id.ok_or_else(|| invalid("KITI-pohjasta puuttuu Results-työlehti"))?;
    let rels = read_zip(zip, "xl/_rels/workbook.xml.rels")?;
    let mut reader = XmlReader::from_reader(rels.as_slice());
    loop {
        match reader.read_event().map_err(xml_error)? {
            Event::Empty(e) | Event::Start(e)
                if e.local_name().as_ref() == b"Relationship"
                    && attr(&e, b"Id") == Some(id.clone()) =>
            {
                let target = attr(&e, b"Target")
                    .ok_or_else(|| invalid("Results-työlehden polku puuttuu"))?;
                if target.contains("..") || target.contains('\\') {
                    return Err(invalid("Virheellinen työlehtipolku"));
                }
                return Ok(if target.starts_with('/') {
                    target.trim_start_matches('/').into()
                } else {
                    format!("xl/{target}")
                });
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Err(invalid("Results-työlehden kohdetta ei löydy"))
}
/// Codes documented in the KITI template's Result definition cell comment.
pub const DEFAULT_CODES: &str = "TRK=1,KESK=2,HYL=3,SULJ=4,PER=0,EILÄ=0";

/// Read class identities without importing competitor identity data.
pub fn template_classes(path: &Path) -> Result<Vec<String>, CoreError> {
    let mut workbook: calamine::Xlsx<_> = calamine::open_workbook(path)
        .map_err(|_| invalid("Valitse kelvollinen KITI-XLSX-pohja"))?;
    let range = workbook
        .worksheet_range("Results")
        .map_err(|_| invalid("KITI-pohjasta puuttuu Results-työlehti"))?;
    let mut classes = Vec::new();
    for row in range.rows() {
        if text(row.first()) == "Luokka / Class" {
            let name = text(row.get(1));
            if !name.is_empty() && !classes.contains(&name) {
                classes.push(name);
            }
        }
    }
    if classes.is_empty() {
        return Err(invalid("KITI-pohjasta puuttuvat luokkalohkot"));
    }
    Ok(classes)
}

/// Display names and exact ranking identifiers, including combined classes.
pub fn group_choices(project: &Project) -> Vec<(String, String)> {
    project
        .luokat
        .iter()
        .map(|c| (c.nimi.clone(), c.tunnus.clone()))
        .chain(project.yhdistetyt_luokat.iter().enumerate().map(|(i, c)| {
            (
                format!("{} (yhdistetty)", c.nimi),
                format!("Y{}: {}", i + 1, c.nimi.trim()),
            )
        }))
        .chain(
            project
                .beginners_cup
                .iter()
                .map(|c| (c.nimi.clone(), c.tunnus.clone())),
        )
        .collect()
}

pub fn suggested_group(project: &Project, source: &str) -> String {
    if let Some(cup) = &project.beginners_cup
        && (cup.tunnus == source || cup.nimi == source)
    {
        return cup.tunnus.clone();
    }
    let combined = project
        .yhdistetyt_luokat
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            (source == c.nimi || source.starts_with(&format!("{}+", c.nimi)))
                && project.luokat.iter().any(|class| {
                    (class.tunnus == source || class.nimi == source)
                        && c.luokat.contains(&class.tunnus)
                })
        })
        .collect::<Vec<_>>();
    if let [(i, c)] = combined.as_slice() {
        return format!("Y{}: {}", i + 1, c.nimi.trim());
    }
    project
        .luokat
        .iter()
        .find(|c| c.tunnus == source || c.nimi == source)
        .map_or_else(String::new, |c| c.tunnus.clone())
}

/// Explicit overrides remain available for another template revision.
pub fn parse_codes(input: &str) -> Result<BTreeMap<String, i64>, CoreError> {
    let mut codes = BTreeMap::new();
    let input = if input.trim().is_empty() {
        DEFAULT_CODES
    } else {
        input
    };
    for part in input.split(',').filter(|p| !p.trim().is_empty()) {
        let (status, code) = part
            .trim()
            .split_once('=')
            .ok_or_else(|| invalid("Anna KITI-tilakoodit muodossa TRK=numero,KESK=numero"))?;
        let code = code
            .trim()
            .parse::<i64>()
            .ok()
            .filter(|c| *c >= 0)
            .ok_or_else(|| invalid("KITI-tilakoodin pitää olla ei-negatiivinen kokonaisluku"))?;
        if !matches!(
            status.trim(),
            "TRK" | "KESK" | "HYL" | "SULJ" | "PER" | "EILÄ"
        ) || codes.insert(status.trim().into(), code).is_some()
        {
            return Err(invalid("Tuntematon tai toistuva KITI-tila"));
        }
    }
    Ok(codes)
}
pub fn fill(
    project: &Project,
    path: &Path,
    codes: &BTreeMap<String, i64>,
) -> Result<Vec<u8>, CoreError> {
    fill_with_groups(project, path, codes, &BTreeMap::new())
}
pub fn fill_with_groups(
    project: &Project,
    path: &Path,
    codes: &BTreeMap<String, i64>,
    groups: &BTreeMap<String, String>,
) -> Result<Vec<u8>, CoreError> {
    let bytes = std::fs::read(path).map_err(|_| invalid("Valitse luettava KITI-Excel-pohja"))?;
    let mut workbook = calamine::Xlsx::new(Cursor::new(bytes.clone()))
        .map_err(|_| invalid("Valitse kelvollinen XLSX-pohja"))?;
    let range = workbook
        .worksheet_range("Results")
        .map_err(|_| invalid("KITI-pohjasta puuttuu Results-työlehti"))?;
    let mut archives = vec![application::calculated_project(project)?];
    if project.beginners_cup.is_some() {
        archives.push(crate::beginners::results(project)?);
    }
    let ranks = archives
        .iter()
        .map(|a| application::rank_stored(a, TiePolicy::PairwiseControlComparison))
        .collect::<Result<Vec<_>, _>>()?;
    let mut updates = BTreeMap::<u32, BTreeMap<String, Option<i64>>>::new();
    let mut group = String::new();
    let mut columns = BTreeMap::new();
    let mut filled = 0;
    for r in 0..range.end().map_or(0, |v| v.0) + 1 {
        let first = text(range.get_value((r, 0)));
        if first == "Luokka / Class" {
            group = text(range.get_value((r, 1)));
            columns.clear();
            continue;
        }
        if first == "EnrollID" {
            for c in 0..range.end().unwrap().1 + 1 {
                columns.insert(text(range.get_value((r, c))), c);
            }
            for label in [
                "Kilpailunumero / Competition number",
                "Tulosselite / Result definition",
                "Sij. / Position",
                "Lkasij. / Class position",
                "AT-pisteet / Time Control points",
                "RT-pisteet / Route Control points",
                "Kilpailun loppupisteet / Total points",
                "Nousup. / Rating points",
            ] {
                if !columns.contains_key(label) {
                    return Err(invalid(format!("KITI-pohjasta puuttuu sarake {label}")));
                }
            }
            continue;
        }
        if columns.is_empty() || group.is_empty() {
            continue;
        }
        let value = text(range.get_value((r, columns["Kilpailunumero / Competition number"])));
        if value.trim().is_empty() {
            continue;
        }
        let number = value.parse::<u32>().map_err(|_| {
            invalid(format!(
                "KITI-rivillä {} on virheellinen kilpailunumero",
                r + 1
            ))
        })?;
        let mut matches = Vec::new();
        for (archive, ranked) in archives.iter().zip(&ranks) {
            for g in &ranked.groups {
                let target = groups.get(&group).unwrap_or(&group);
                let label = crate::printing::class_label(&g.code);
                if &g.code != target
                    && label != target
                    && !archive
                        .basic
                        .classes
                        .iter()
                        .any(|c| c.code == g.code && &c.name == target)
                {
                    continue;
                }
                for (index, driver) in archive
                    .competitors
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.number == number)
                {
                    let member = if g.kind == ranking::GroupKind::Ordinary {
                        driver.class_code == g.code
                    } else {
                        archive.basic.combined_classes.iter().any(|combined| {
                            combined
                                .first()
                                .zip(combined.get(1))
                                .is_some_and(|(i, name)| format!("Y{i}: {}", name.trim()) == g.code)
                                && archive.basic.classes.iter().enumerate().any(|(i, c)| {
                                    c.code == driver.class_code
                                        && combined.get(4 + i).is_some_and(|v| v.trim() == "1")
                                })
                        })
                    };
                    if member {
                        let entry = g.entries.iter().find(|e| e.source_index == index + 1);
                        matches.push((archive, ranked, g, entry, driver, index + 1));
                    }
                }
            }
        }
        if matches.len() != 1 {
            return Err(invalid(format!(
                "KITI-rivi {}: luokan ja kilpailunumeron kohdistus puuttuu tai on moniselitteinen",
                r + 1
            )));
        }
        let (archive, ranked, g, entry, driver, source_index) = matches[0];
        let status = entry.map_or(driver.status.as_str(), |e| e.status.as_legacy());
        let code = *codes.get(status).ok_or_else(|| {
            invalid(format!(
                "Anna KITI-tilakoodi tilalle {status}; koodeja ei arvata"
            ))
        })?;
        let classified = entry.is_some_and(|e| e.status == ranking::Status::Classified);
        let source_class = archive
            .basic
            .classes
            .iter()
            .find(|c| c.code == driver.class_code)
            .ok_or_else(|| invalid("KITI-luokan rata puuttuu"))?;
        let overall = ranked
            .routes
            .iter()
            .filter(|r| r.code == source_class.route)
            .flat_map(|r| &r.entries)
            .find(|e| e.source_index == source_index && e.number == i64::from(number))
            .map(|e| i64::from(e.primary_position));
        let header = |i: usize| {
            driver
                .header_fields
                .get(i)
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(0)
        };
        let route_points = driver
            .results
            .iter()
            .filter(|r| r.station_type == "RT")
            .map(|r| r.other_penalty)
            .sum::<i64>()
            + header(22)
            + header(24);
        let time_points = driver
            .results
            .iter()
            .filter(|r| matches!(r.station_type.as_str(), "AT" | "JAT" | "M") || r.sequence == 0)
            .map(|r| r.signed_time_penalty.abs() + r.other_penalty)
            .sum::<i64>();
        let values = [
            Some(code),
            if classified { overall } else { None },
            entry
                .filter(|_| classified)
                .map(|e| i64::from(e.primary_position)),
            classified.then_some(time_points),
            classified.then_some(route_points),
            classified.then_some(driver.total_penalties),
            entry
                .filter(|_| classified && g.np_enabled && !g.provisional)
                .map(|e| i64::from(e.table_points)),
        ];
        let labels = [
            "Tulosselite / Result definition",
            "Sij. / Position",
            "Lkasij. / Class position",
            "AT-pisteet / Time Control points",
            "RT-pisteet / Route Control points",
            "Kilpailun loppupisteet / Total points",
            "Nousup. / Rating points",
        ];
        let mut cells = BTreeMap::new();
        for (label, value) in labels.into_iter().zip(values) {
            let col = columns[label];
            if !(18..=24).contains(&col) {
                return Err(invalid("KITI-tulossarakkeiden pitää olla alueella S–Y"));
            }
            cells.insert(format!("{}{}", char::from(b'A' + col as u8), r + 1), value);
        }
        updates.insert(r + 1, cells);
        filled += 1;
    }
    if filled == 0 {
        return Err(invalid("KITI-pohjassa ei ole kohdistettavia tulosrivejä"));
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(xml_error)?;
    let sheet = sheet_path(&mut zip)?;
    let source = read_zip(&mut zip, &sheet)?;
    let rewritten = rewrite(&source, &mut updates)?;
    let mut output = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(xml_error)?;
        let name = file.name().to_owned();
        let options =
            zip::write::SimpleFileOptions::default().compression_method(file.compression());
        if file.is_dir() {
            output.add_directory(name, options).map_err(xml_error)?;
            continue;
        }
        output.start_file(&name, options).map_err(xml_error)?;
        if name == sheet {
            output.write_all(&rewritten).map_err(xml_error)?;
        } else {
            std::io::copy(&mut file, &mut output).map_err(xml_error)?;
        }
    }
    Ok(output.finish().map_err(xml_error)?.into_inner())
}
fn write_cell(
    writer: &mut Writer<Vec<u8>>,
    reference: &str,
    value: Option<i64>,
    style: Option<&str>,
) -> Result<(), CoreError> {
    let mut cell = BytesStart::new("c");
    cell.push_attribute(("r", reference));
    if let Some(style) = style {
        cell.push_attribute(("s", style));
    }
    writer.write_event(Event::Start(cell)).map_err(xml_error)?;
    if let Some(value) = value {
        writer
            .write_event(Event::Start(BytesStart::new("v")))
            .map_err(xml_error)?;
        writer
            .write_event(Event::Text(BytesText::new(&value.to_string())))
            .map_err(xml_error)?;
        writer
            .write_event(Event::End(BytesEnd::new("v")))
            .map_err(xml_error)?;
    }
    writer
        .write_event(Event::End(BytesEnd::new("c")))
        .map_err(xml_error)?;
    Ok(())
}
fn rewrite(
    source: &[u8],
    updates: &mut BTreeMap<u32, BTreeMap<String, Option<i64>>>,
) -> Result<Vec<u8>, CoreError> {
    let mut reader = XmlReader::from_reader(source);
    let mut writer = Writer::new(Vec::new());
    let mut row = 0;
    loop {
        let event = reader.read_event().map_err(xml_error)?;
        match &event {
            Event::Start(e) if e.local_name().as_ref() == b"row" => {
                row = attr(e, b"r").and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            Event::Start(e) | Event::Empty(e) if e.local_name().as_ref() == b"c" => {
                if let Some(reference) = attr(e, b"r")
                    && let Some(value) = updates.get_mut(&row).and_then(|u| u.remove(&reference))
                {
                    let style = attr(e, b"s");
                    if matches!(event, Event::Start(_)) {
                        reader.read_to_end(e.name()).map_err(xml_error)?;
                    }
                    write_cell(&mut writer, &reference, value, style.as_deref())?;
                    continue;
                }
            }
            Event::End(e) if e.local_name().as_ref() == b"row" => {
                if let Some(cells) = updates.remove(&row) {
                    for (reference, value) in cells {
                        write_cell(&mut writer, &reference, value, None)?;
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
        writer.write_event(event).map_err(xml_error)?;
    }
    if !updates.is_empty() {
        return Err(invalid("KITI-tulosrivejä puuttuu työlehden XML:stä"));
    }
    Ok(writer.into_inner())
}
