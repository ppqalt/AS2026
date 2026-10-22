//! Perusteltu leimakorjaus on laskennan syöte, ei muutos alkuperäiseen korttiin.
use crate::{
    CoreError, MatchKind, application, matching,
    native::{Project, ReadPunch, Readout},
};
use serde::{Deserialize, Serialize};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Correction {
    /// Ykkösestä alkava viittaus pysyvään alkuperäiseen lukuhistoriaan.
    pub lahdelukema: usize,
    pub kilpailija: u32,
    pub asema: String,
    pub syy: String,
    pub aika_unix_ms: u64,
    /// Kortin sisältö ankkureineen, ilman lukijan 250-merkkiä.
    pub ennen: Vec<ReadPunch>,
    pub jalkeen: Vec<ReadPunch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Edit,
    Remove,
    Restore,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub row: u32,
    pub source_row: Option<u32>,
    pub action: Action,
    pub code: u32,
    pub elapsed_seconds: u32,
    pub reason: String,
}

fn content(read: &Readout) -> Result<Vec<ReadPunch>, CoreError> {
    Ok(read
        .content()?
        .into_iter()
        .map(|p| ReadPunch {
            koodi: p.code,
            aika_sekuntia: p.elapsed_seconds,
        })
        .collect())
}
fn validate_content(values: &[ReadPunch]) -> Result<(), CoreError> {
    if values.is_empty() || values.len() > 49 || values.iter().any(|p| p.koodi == 250) {
        return Err(invalid(
            "Korjattuun korttiin mahtuu ankkuri ja enintään 48 leimaa; lukijakoodia 250 ei saa lisätä",
        ));
    }
    if values.iter().skip(1).any(|p| {
        p.koodi == 0 || (p.koodi > 255 && p.koodi != 999) || p.aika_sekuntia > u16::MAX.into()
    }) {
        return Err(invalid(
            "Emit-koodin on oltava 1–255 tai 999 (ei 250) ja korttiajan enintään 18:12:15",
        ));
    }
    if values
        .windows(2)
        .any(|p| p[0].aika_sekuntia > p[1].aika_sekuntia)
    {
        return Err(invalid(
            "Leimojen korttiaikojen on oltava aikajärjestyksessä. Korjaa aika tai kohdistus",
        ));
    }
    Ok(())
}

pub fn validate_history(project: &Project) -> Result<(), CoreError> {
    let mut chains = std::collections::BTreeMap::<Vec<(u32, u32)>, Vec<ReadPunch>>::new();
    for edit in &project.emit_korjaukset {
        let original = edit
            .lahdelukema
            .checked_sub(1)
            .and_then(|i| project.leimaluvut.get(i))
            .ok_or_else(|| invalid("Emit-korjauksen alkuperäistä lukemaa ei löydy"))?;
        let baseline = content(original)?;
        // Korttinumero erottaa samansisältöiset eri kortit.
        let key = std::iter::once((original.emit, u32::MAX))
            .chain(baseline.iter().map(|p| (p.koodi, p.aika_sekuntia)))
            .collect();
        let previous = chains.entry(key).or_insert(baseline.clone());
        if edit.syy.chars().any(char::is_control)
            || edit.ennen != *previous
            || edit.jalkeen.first() != baseline.first()
        {
            return Err(invalid(
                "Emit-korjauksen perustelu tai ennen/jälkeen-historia on virheellinen",
            ));
        }
        // Alkuperäisen virheellisen korttiaikajärjestyksen palauttaminen on sallittu.
        if edit.jalkeen != baseline {
            validate_content(&edit.jalkeen)?;
        }
        *previous = edit.jalkeen.clone();
    }
    Ok(())
}

pub fn effective_readout(project: &Project, original: &Readout) -> Result<Readout, CoreError> {
    let baseline = content(original)?;
    let mut effective = original.clone();
    for edit in &project.emit_korjaukset {
        let Some(source) = edit
            .lahdelukema
            .checked_sub(1)
            .and_then(|i| project.leimaluvut.get(i))
        else {
            return Err(invalid("Emit-korjauksen alkuperäinen lukema puuttuu"));
        };
        // Sama korttisisältö uusintaluennassa säilyttää korjauksen. Uusi muuttunut
        // sisältö ei peri aikaisemman kortin kohdistuksia hiljaisesti.
        if source.emit == original.emit && content(source)? == baseline {
            let marker = original
                .leimat
                .iter()
                .find(|p| p.koodi == 250)
                .unwrap()
                .clone();
            effective.leimat = edit.jalkeen.clone();
            effective.leimat.push(marker);
        }
    }
    effective.validate()?;
    Ok(effective)
}

#[derive(Debug, Clone)]
pub struct Row {
    pub index: u32,
    pub sequence: u32,
    pub name: String,
    pub expected_code: u32,
    pub code: u32,
    pub elapsed_seconds: u32,
    position: usize,
    consumes: bool,
    shared: bool,
}

pub fn punch_clock(project: &Project, number: u32, elapsed: i64) -> Result<i64, CoreError> {
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let start = driver
        .start_clock()?
        .ok_or_else(|| invalid("Kilpailijan lähtöaika puuttuu"))?
        .seconds_since_midnight()?;
    let correction = if project.kilpailu.ajanottotapa == 0 {
        0
    } else {
        driver.effective_emit_correction()
    };
    start
        .checked_add(elapsed)
        .and_then(|v| v.checked_sub(correction))
        .ok_or_else(|| invalid("Leimauksen kellonaika on liian suuri"))
}
pub fn clock_text(seconds: i64) -> String {
    let seconds = seconds.rem_euclid(86_400);
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        seconds % 3600 / 60,
        seconds % 60
    )
}

pub fn rows(project: &Project, number: u32) -> Result<Vec<Row>, CoreError> {
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let class = project
        .luokat
        .iter()
        .find(|c| c.tunnus == driver.luokka)
        .ok_or_else(|| invalid("Luokkaa ei löydy"))?;
    let archive = project.to_archive()?;
    let route = archive
        .routes
        .iter()
        .find(|r| r.index == class.rata)
        .ok_or_else(|| invalid("Luokan rata puuttuu"))?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    let original = project
        .active_readout(driver.emit)
        .ok_or_else(|| invalid("Lue ensin kilpailijan kortti"))?;
    let read = effective_readout(project, original)?;
    let punches = read.content()?;
    let start = driver
        .start_clock()?
        .ok_or_else(|| invalid("Kilpailijan lähtöaika puuttuu"))?;
    let mut position = 1;
    Ok(
        matching::match_operational_route(&route, &punches[1..], start)?
            .into_iter()
            .map(|r| {
                let consumes = matches!(r.kind, MatchKind::Matched | MatchKind::Extra);
                let row = Row {
                    index: r.output_index as u32,
                    sequence: r.station.as_ref().map_or(0, |s| s.sequence),
                    name: r
                        .station
                        .as_ref()
                        .map_or("Ylimääräinen leima".into(), |s| s.label.clone()),
                    expected_code: r.station.as_ref().map_or(999, |s| s.expected_code),
                    code: r.punch.map_or(0, |p| p.code),
                    elapsed_seconds: r.punch.map_or(0, |p| p.elapsed_seconds),
                    position,
                    consumes,
                    shared: r.kind == MatchKind::SharedUla,
                };
                if consumes {
                    position += 1;
                }
                row
            })
            .collect(),
    )
}

pub(crate) fn recapture(
    project: &Project,
    number: u32,
    previous: Option<&crate::corrections::StoredResult>,
) -> Result<crate::corrections::StoredResult, CoreError> {
    let mut result = application::capture_project_result(project, number)?;
    if let Some(old) = previous {
        let same_layout = old.rata.len() == result.rata.len()
            && old
                .rata
                .iter()
                .zip(&result.rata)
                .all(|(a, b)| a.tyyppi == b.tyyppi && a.emit_koodi == b.emit_koodi);
        // Käsikellot tunnistetaan rata-asemasta, eivät muuttuvasta rivinumerosta.
        for row in &old.rivit {
            if row.sequence != 0
                && row.expected_code == 0
                && old.kasin_annetut_kellot.contains(&row.output_index)
                && let Some(index) = result
                    .rivit
                    .iter()
                    .find(|r| {
                        r.name == row.name
                            && r.station_type == row.station_type
                            && r.expected_code == 0
                    })
                    .or_else(|| {
                        if same_layout {
                            result.rivit.iter().find(|r| {
                                r.sequence == row.sequence
                                    && r.station_type == row.station_type
                                    && r.expected_code == 0
                            })
                        } else {
                            None
                        }
                    })
                    .map(|r| r.output_index)
            {
                result.set_clock(index, row.clock)?;
            }
        }
        result = application::recalculate_stored_result(project, number, &result)?;
    }
    result.hyvaksytty = false;
    Ok(result)
}

/// Kutsutaan ProjectStoren atomisen muutoksen sisällä. Virhe ei tallenna mitään.
pub fn apply(project: &mut Project, number: u32, request: &Request) -> Result<(), CoreError> {
    let reason = request.reason.trim();
    if reason.chars().any(char::is_control) {
        return Err(invalid(
            "Anna korjaukselle selkeä perustelu yhdellä rivillä",
        ));
    }
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let card = driver.emit;
    let (source_index, original) = project
        .leimaluvut
        .iter()
        .enumerate()
        .rev()
        .find(|(_, r)| r.emit == card && r.validate().is_ok())
        .ok_or_else(|| invalid("Lue ensin kilpailijan kortti"))?;
    let original_content = content(original)?;
    let current = effective_readout(project, original)?;
    let before = content(&current)?;
    let mut after = before.clone();
    let mapping = rows(project, number)?;
    let row = mapping
        .iter()
        .find(|r| r.index == request.row)
        .ok_or_else(|| invalid("Valitse asemarivi uudelleen"))?;
    if request.action != Action::Restore && (row.expected_code == 0 || row.shared) {
        return Err(invalid(
            "Käsikello korjataan asematuloksen kellokentässä. Jaettu ULA-leima korjataan sitä edeltävän aseman kautta",
        ));
    }
    match request.action {
        Action::Restore => after = original_content,
        Action::Remove => {
            if !row.consumes {
                return Err(invalid("Valitulla asemalla ei ole poistettavaa leimaa"));
            }
            after.remove(row.position);
        }
        Action::Edit => {
            let punch = ReadPunch {
                koodi: request.code,
                aika_sekuntia: request.elapsed_seconds,
            };
            if let Some(source_row) = request.source_row {
                if row.consumes {
                    return Err(invalid(
                        "Valitse puuttuva asema, kun kohdistat ylimääräistä leimaa",
                    ));
                }
                let source = mapping
                    .iter()
                    .find(|r| r.index == source_row && r.sequence == 0 && r.consumes)
                    .ok_or_else(|| invalid("Valitse olemassa oleva ylimääräinen leima"))?;
                after.remove(source.position);
                let position = row.position - usize::from(source.position < row.position);
                after.insert(position, punch);
            } else if row.consumes {
                after[row.position] = punch;
            } else {
                after.insert(row.position, punch);
            }
        }
    }
    if after == before {
        return Err(invalid("Korjauksessa ei ole muutoksia"));
    }
    if request.action != Action::Restore {
        validate_content(&after)?;
    }
    let event = Correction {
        lahdelukema: source_index + 1,
        kilpailija: number,
        asema: format!("{} · {}", row.index, row.name),
        syy: reason.into(),
        aika_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| invalid("Järjestelmän kello on virheellinen"))?
            .as_millis()
            .try_into()
            .map_err(|_| invalid("Järjestelmän aika on liian suuri"))?,
        ennen: before,
        jalkeen: after,
    };
    project.emit_korjaukset.push(event);
    validate_history(project)?;
    if request.action == Action::Edit && row.sequence != 0 {
        let rematched = rows(project, number)?;
        if rematched
            .iter()
            .find(|r| r.sequence == row.sequence)
            .is_none_or(|r| r.code != request.code || r.elapsed_seconds != request.elapsed_seconds)
        {
            return Err(invalid(
                "Korjattu leima ei kohdistu valittuun asemaan. Tarkista toistuvat koodit ja leimojen järjestys",
            ));
        }
    }
    rebuild_results(project, number)
}

pub fn replace_content(
    project: &mut Project,
    number: u32,
    punches: Vec<ReadPunch>,
) -> Result<(), CoreError> {
    let mut candidate = project.clone();
    let card = candidate
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?
        .emit;
    let (index, original) = candidate
        .leimaluvut
        .iter()
        .enumerate()
        .rev()
        .find(|(_, r)| r.emit == card && r.validate().is_ok())
        .ok_or_else(|| invalid("Korttiluku puuttuu"))?;
    let before = content(&effective_readout(&candidate, original)?)?;
    let mut after = vec![before[0].clone()];
    after.extend(punches);
    validate_content(&after)?;
    if before == after {
        return Ok(());
    }
    candidate.emit_korjaukset.push(Correction {
        lahdelukema: index + 1,
        kilpailija: number,
        asema: "Kaikki leimaukset".into(),
        syy: String::new(),
        aika_unix_ms: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| invalid("Järjestelmän kello on virheellinen"))?
            .as_millis() as u64,
        ennen: before,
        jalkeen: after,
    });
    validate_history(&candidate)?;
    rebuild_results(&mut candidate, number)?;
    candidate.validate()?;
    *project = candidate;
    Ok(())
}

fn rebuild_results(project: &mut Project, number: u32) -> Result<(), CoreError> {
    let previous = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .unwrap()
        .tallennettu_tulos
        .clone();
    let result = recapture(project, number, previous.as_ref())?;
    project
        .kilpailijat
        .iter_mut()
        .find(|d| d.numero == number)
        .unwrap()
        .tallennettu_tulos = Some(result);
    if project
        .bc_osallistujat
        .iter()
        .any(|e| e.numero == number && e.osallistuu)
    {
        let projection = crate::beginners::projection(project)?;
        let previous = project
            .bc_osallistujat
            .iter()
            .find(|e| e.numero == number)
            .unwrap()
            .tallennettu_tulos
            .as_ref();
        let result = recapture(&projection, number, previous)?;
        project
            .bc_osallistujat
            .iter_mut()
            .find(|e| e.numero == number)
            .unwrap()
            .tallennettu_tulos = Some(result);
    } else if let Some(entry) = project
        .bc_osallistujat
        .iter_mut()
        .find(|e| e.numero == number)
    {
        // Myös tilapäisesti pois valittu BC-tulos on tarkistettava uudelleen.
        entry.tallennettu_tulos = None;
    }
    Ok(())
}
