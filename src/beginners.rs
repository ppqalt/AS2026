//! BC on erillinen laskenta samasta kilpailijasta ja muuttamattomasta Emit-historiasta.
use crate::{
    CoreError, MatchKind, StationKind, application,
    corrections::StoredResult,
    legacy::Archive,
    native::{AdditionalPenalties, ManualStatus, Project, Readout, Station},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BeginnersCup {
    pub nimi: String,
    pub tunnus: String,
    pub lahderata: u32,
    /// Tarkastettu lähderata: indeksit eivät saa siirtyä hiljaisesti ratamuutoksessa.
    pub lahdeasemat: Vec<Station>,
    /// Lähderadan yhden perusteiset asemakohdat. Ilman maalia viimeinen JAT päättää BC:n.
    pub mukana_asemat: Vec<u32>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub numero: u32,
    #[serde(default = "enabled")]
    pub osallistuu: bool,
    #[serde(default)]
    pub lisavirheet: AdditionalPenalties,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tila: Option<ManualStatus>,
    #[serde(default)]
    pub keskeytys_vahvistettu: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tallennettu_tulos: Option<StoredResult>,
}

fn enabled() -> bool {
    true
}

fn invalid(message: &str) -> CoreError {
    CoreError::InvalidInput(message.into())
}

pub fn validate(project: &Project) -> Result<(), CoreError> {
    let Some(config) = &project.beginners_cup else {
        return if project.bc_osallistujat.is_empty() {
            Ok(())
        } else {
            Err(invalid(
                "Määritä Beginners Cup ennen osallistujien valitsemista",
            ))
        };
    };
    if config.nimi.trim().is_empty()
        || config.tunnus.trim().is_empty()
        || project
            .luokat
            .iter()
            .any(|c| c.tunnus.trim() == config.tunnus.trim())
    {
        return Err(invalid("BC:n nimi ja erillinen luokkatunnus vaaditaan"));
    }
    if !project.radat.iter().any(|r| r.numero == config.lahderata) {
        return Err(invalid("BC:n NAC-lähderataa ei löydy"));
    }
    if !project.luokat.iter().any(|c| c.rata == config.lahderata) {
        return Err(invalid("Valitse BC:n lähteeksi NAC-luokan rata"));
    }
    let mut positions = BTreeSet::new();
    for &position in &config.mukana_asemat {
        if position == 0
            || position as usize > config.lahdeasemat.len()
            || !positions.insert(position)
        {
            return Err(invalid("BC:n asemavalinta on virheellinen tai toistuva"));
        }
    }
    if config.lahdeasemat.len() < 2 || positions.len() < 2 {
        return Err(invalid("BC-radalla pitää olla lähtö ja maali"));
    }
    if config
        .lahdeasemat
        .first()
        .is_none_or(|s| s.tyyppi.trim() != "L")
        || config
            .lahdeasemat
            .last()
            .is_none_or(|s| s.tyyppi.trim() != "M")
    {
        return Err(invalid(
            "BC:n lähderadan pitää alkaa lähdöstä ja päättyä maaliin",
        ));
    }
    let finish = finish_index(config)?;
    for (index, station) in config.lahdeasemat.iter().enumerate().take(finish + 1) {
        let kind = StationKind::from_legacy(&station.tyyppi)?;
        if matches!(kind, StationKind::Start | StationKind::Ula)
            && !positions.contains(&((index + 1) as u32))
        {
            return Err(invalid(
                "BC-radan lähtöä ja ULA-aikaviitteitä ei voi jättää pois",
            ));
        }
    }
    let mut numbers = BTreeSet::new();
    for entry in &project.bc_osallistujat {
        if !numbers.insert(entry.numero)
            || !project.kilpailijat.iter().any(|d| d.numero == entry.numero)
            || (entry.osallistuu && !eligible(project, entry.numero, config.lahderata))
        {
            return Err(invalid(
                "BC-osallistuja puuttuu NAC-lähderadalta tai toistuu",
            ));
        }
        entry.lisavirheet.validate()?;
        if let Some(status) = &entry.tila {
            status.validate()?;
        }
        if entry.keskeytys_vahvistettu
            && !entry.tila.as_ref().is_some_and(|s| {
                s.tunnus == "KESK" && crate::native::validate_interruption_reason(&s.syy).is_ok()
            })
        {
            return Err(invalid(
                "BC:n vahvistettu keskeytys edellyttää KESK-tilaa ja syytä",
            ));
        }
    }
    // Tarkista myös johdetun radan aritmetiikka ennen tallentamista.
    derived_stations(config)?;
    Ok(())
}

pub fn eligible(project: &Project, number: u32, route: u32) -> bool {
    project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .is_some_and(|d| {
            project
                .luokat
                .iter()
                .any(|c| c.tunnus == d.luokka && c.rata == route)
        })
}

fn finish_index(config: &BeginnersCup) -> Result<usize, CoreError> {
    let last = config
        .lahdeasemat
        .len()
        .checked_sub(1)
        .ok_or_else(|| invalid("BC:n lähderata puuttuu"))?;
    if config.mukana_asemat.contains(&((last + 1) as u32)) {
        return Ok(last);
    }
    let finish = config
        .lahdeasemat
        .iter()
        .rposition(|s| s.tyyppi.trim() == "JAT")
        .ok_or_else(|| invalid("Ilman maalileimausta BC tarvitsee viimeisen JAT-aseman"))?;
    if !config.mukana_asemat.contains(&((finish + 1) as u32)) {
        return Err(invalid("Valitse viimeinen JAT BC:n maaliksi"));
    }
    Ok(finish)
}

pub fn derived_stations(config: &BeginnersCup) -> Result<Vec<Station>, CoreError> {
    let mut stations = Vec::new();
    let mut omitted_target = 0_i64;
    let mut omitted_minutes = 0_u32;
    let finish = finish_index(config)?;
    for (index, station) in config.lahdeasemat.iter().enumerate().take(finish + 1) {
        let kind = StationKind::from_legacy(&station.tyyppi)?;
        if !config.mukana_asemat.contains(&((index + 1) as u32)) {
            if kind == StationKind::Jat {
                omitted_target = omitted_target
                    .checked_add(station.tavoiteaika_sekuntia)
                    .ok_or(CoreError::ArithmeticOverflow("BC:n JAT-tavoiteajat"))?;
                omitted_minutes = omitted_minutes
                    .checked_add(station.jakso_minuutteina.unwrap_or(0))
                    .ok_or(CoreError::ArithmeticOverflow("BC:n jaksoajat"))?;
            }
            continue;
        }
        let mut station = station.clone();
        if kind == StationKind::Ula {
            omitted_target = 0;
            omitted_minutes = 0;
        }
        station.tavoiteaika_sekuntia = station
            .tavoiteaika_sekuntia
            .checked_add(omitted_target)
            .ok_or(CoreError::ArithmeticOverflow("BC:n tavoiteaika"))?;
        if matches!(kind, StationKind::Jat | StationKind::Finish) {
            if let Some(minutes) = station.jakso_minuutteina {
                station.jakso_minuutteina = Some(
                    minutes
                        .checked_add(omitted_minutes)
                        .ok_or(CoreError::ArithmeticOverflow("BC:n jaksoaika"))?,
                );
            }
            omitted_target = 0;
            omitted_minutes = 0;
        }
        if index == finish {
            station.tyyppi = "M".into();
        }
        stations.push(station);
    }
    Ok(stations)
}

/// Poistaa vain pois valittuun lähdeasemakohtaan kohdistetun leiman.
/// Oikeasti ylimääräiset leimat ja toistuvan koodin muut esiintymät säilyvät.
fn filtered_readout(
    config: &BeginnersCup,
    route: &[crate::RouteStation],
    start: crate::ClockTime,
    read: &Readout,
) -> Result<Readout, CoreError> {
    let content = read.content()?;
    let interior = &content[1..];
    let rows = crate::matching::match_route(route, interior, start)?;
    let finish = finish_index(config)?;
    let shortened = finish + 1 < config.lahdeasemat.len();
    let included =
        |sequence: u32| sequence as usize <= finish + 1 && config.mukana_asemat.contains(&sequence);
    for row in &rows {
        if row.kind == MatchKind::Missing
            && let Some(missing) = &row.station
        {
            let same_code = route
                .iter()
                .filter(|s| s.expected_code != 0 && s.expected_code == missing.expected_code)
                .collect::<Vec<_>>();
            if same_code.iter().any(|s| included(s.sequence))
                && same_code.iter().any(|s| !included(s.sequence))
            {
                return Err(invalid(
                    "Toistuvan Emit-koodin BC-kohdistus on epäselvä puuttuvan leiman vuoksi. Tarkista alkuperäinen korttiluku",
                ));
            }
        }
    }
    let mut omitted = BTreeSet::new();
    let mut needed = BTreeSet::new();
    let mut punch = 0_usize;
    let mut finish_punch = None;
    for row in rows {
        let at_finish = row
            .station
            .as_ref()
            .is_some_and(|s| s.sequence as usize == finish + 1);
        let included = row.station.as_ref().is_none_or(|s| included(s.sequence));
        match row.kind {
            MatchKind::Matched | MatchKind::Extra => {
                if !included {
                    omitted.insert(punch + 1);
                }
                punch += 1;
                if at_finish {
                    finish_punch = Some(punch);
                }
            }
            MatchKind::SharedUla if included && punch > 0 => {
                needed.insert(punch);
            }
            _ => {}
        }
        if at_finish {
            finish_punch = Some(punch);
        }
    }
    let mut filtered = read.clone();
    filtered.leimat = read
        .leimat
        .iter()
        .enumerate()
        .filter(|(index, value)| {
            // Myös siirtymän ylimääräiset leimat rajataan BC:n ulkopuolelle.
            let before_finish = !shortened
                || *index == 0
                || value.koodi == 250
                || finish_punch.is_some_and(|end| *index <= end);
            before_finish && (!omitted.contains(index) || needed.contains(index))
        })
        .map(|(_, value)| value.clone())
        .collect();
    filtered.validate()?;
    Ok(filtered)
}

/// Väliaikainen laskentanäkymä; tätä ei koskaan tallenneta kilpailun tilalle.
pub fn projection(project: &Project) -> Result<Project, CoreError> {
    validate(project)?;
    let config = project
        .beginners_cup
        .as_ref()
        .ok_or_else(|| invalid("Beginners Cupia ei ole määritetty"))?;
    let source = project
        .radat
        .iter()
        .find(|r| r.numero == config.lahderata)
        .ok_or_else(|| invalid("BC:n lähderataa ei löydy"))?;
    if source.asemat != config.lahdeasemat {
        return Err(invalid(
            "NAC-rata on muuttunut. Tarkista ja tallenna BC:n asemavalinnat uudelleen",
        ));
    }
    let source_archive = project.to_archive()?;
    let route = source_archive
        .routes
        .iter()
        .find(|r| r.index == config.lahderata)
        .ok_or_else(|| invalid("BC:n lähderataa ei löydy"))?
        .to_core()
        .map_err(|e| CoreError::InvalidInput(e.to_string()))?;
    let mut view = project.clone();
    view.beginners_cup = None;
    view.bc_osallistujat.clear();
    view.yhdistetyt_luokat.clear();
    view.kilpailijat.retain(|d| {
        project
            .bc_osallistujat
            .iter()
            .any(|e| e.osallistuu && e.numero == d.numero)
    });
    for driver in &mut view.kilpailijat {
        let entry = project
            .bc_osallistujat
            .iter()
            .find(|e| e.numero == driver.numero)
            .unwrap();
        driver.lisavirheet = entry.lisavirheet.clone();
        driver.tila = entry.tila.clone();
        driver.keskeytys_vahvistettu = entry.keskeytys_vahvistettu;
        driver.tallennettu_tulos = entry.tallennettu_tulos.clone();
    }
    view.leimaluvut.clear();
    for driver in &view.kilpailijat {
        // Vain aktiivinen lukema tarvitaan projektioon. Alkuperäinen historia säilyy projektissa.
        if let Some(read) = project.active_readout(driver.emit)
            && let Some(start) = driver.start_clock()?
        {
            view.leimaluvut
                .push(filtered_readout(config, &route, start, read)?);
        }
    }
    let track = view
        .radat
        .iter_mut()
        .find(|r| r.numero == config.lahderata)
        .unwrap();
    track.asemat = derived_stations(config)?;
    track.nimi = format!("{} · {}", config.nimi, source.nimi);
    // Lähdeluokat säilyttävät omat myöhästymisrajansa laskennassa.
    view.validate()?;
    Ok(view)
}

pub fn store_projection(project: &mut Project, view: &Project) -> Result<(), CoreError> {
    for entry in project.bc_osallistujat.iter_mut().filter(|e| e.osallistuu) {
        let driver = view
            .kilpailijat
            .iter()
            .find(|d| d.numero == entry.numero)
            .ok_or_else(|| invalid("BC-tuloksen kilpailijaa ei löydy"))?;
        entry.lisavirheet = driver.lisavirheet.clone();
        entry.tila = driver.tila.clone();
        entry.keskeytys_vahvistettu = driver.keskeytys_vahvistettu;
        entry.tallennettu_tulos = driver.tallennettu_tulos.clone();
    }
    Ok(())
}

/// BC:n omat pisteet eivät muuta NAC:n hyväksyntää ja päinvastoin.
pub(crate) fn invalidate_input_changes(before: &Project, after: &mut Project) {
    let route_changed = before
        .beginners_cup
        .as_ref()
        .map(|c| (&c.lahdeasemat, &c.mukana_asemat, c.lahderata))
        != after
            .beginners_cup
            .as_ref()
            .map(|c| (&c.lahdeasemat, &c.mukana_asemat, c.lahderata));
    for entry in &mut after.bc_osallistujat {
        let Some(driver) = after.kilpailijat.iter().find(|d| d.numero == entry.numero) else {
            continue;
        };
        let Some(old_driver) = before
            .kilpailijat
            .iter()
            .find(|d| d.numero == entry.numero)
            .or_else(|| {
                before
                    .kilpailijat
                    .iter()
                    .find(|d| d.emit != 0 && d.emit == driver.emit)
            })
        else {
            continue;
        };
        let old_entry = before
            .bc_osallistujat
            .iter()
            .find(|e| e.numero == old_driver.numero);
        if driver.emit != old_driver.emit {
            entry.tallennettu_tulos = None;
            continue;
        }
        let old_class = before.luokat.iter().find(|c| c.tunnus == old_driver.luokka);
        let new_class = after.luokat.iter().find(|c| c.tunnus == driver.luokka);
        let class_changed = old_class.map(|c| (c.rata, c.myohastymisraja_minuutteina))
            != new_class.map(|c| (c.rata, c.myohastymisraja_minuutteina));
        let source_changed = after.beginners_cup.as_ref().is_some_and(|c| {
            before
                .radat
                .iter()
                .find(|r| r.numero == c.lahderata)
                .map(|r| &r.asemat)
                != after
                    .radat
                    .iter()
                    .find(|r| r.numero == c.lahderata)
                    .map(|r| &r.asemat)
        });
        let timing_changed = route_changed
            || source_changed
            || class_changed
            || before.kilpailu.ajanottotapa != after.kilpailu.ajanottotapa
            || before.virhepisteet != after.virhepisteet
            || driver.lahtoaika != old_driver.lahtoaika
            || driver.emit_korjaus_sekuntia != old_driver.emit_korjaus_sekuntia;
        if let Some(result) = &mut entry.tallennettu_tulos {
            if timing_changed {
                result.hyvaksytty = false;
                result.kasikellot_muuttuneet = true;
            } else if old_entry
                .is_some_and(|old| old.lisavirheet != entry.lisavirheet || old.tila != entry.tila)
            {
                result.hyvaksytty = false;
            }
        }
    }
}

pub fn results(project: &Project) -> Result<Archive, CoreError> {
    let view = projection(project)?;
    let mut archive = application::calculated_project(&view)?;
    let config = project.beginners_cup.as_ref().unwrap();
    let mut class = archive
        .basic
        .classes
        .iter()
        .find(|c| c.route == config.lahderata.to_string())
        .cloned()
        .ok_or_else(|| invalid("BC:n lähderadalle ei ole luokkaa"))?;
    class.code = config.tunnus.clone();
    class.name = config.nimi.clone();
    class.fields[0] = class.code.clone();
    class.fields[1] = class.name.clone();
    class.fields[8] = "0".into();
    class.fields[9] = "0".into();
    archive.basic.classes = vec![class];
    for driver in &mut archive.competitors {
        driver.class_code = config.tunnus.clone();
        driver.header_fields[1] = config.tunnus.clone();
    }
    Ok(archive)
}
