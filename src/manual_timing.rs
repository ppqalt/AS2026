//! Paperikortin ajanotto käyttää samaa laskentaydintä ilman synteettisiä Emit-lukuja.
use crate::{
    CoreError, StationKind, application, corrections::StoredResult, legacy::Archive,
    native::Project,
};

pub(crate) fn normalize(archive: &mut Archive) {
    for route in &mut archive.routes {
        route.stations.retain(|s| s.station_type != "RT");
        for (i, s) in route.stations.iter_mut().enumerate() {
            s.index = i as u32 + 1;
            s.expected_code = 0;
        }
        route.declared_station_count = route.stations.len() as u32;
    }
}

/// Merkinnän puuttuminen on eri asia kuin vielä syöttämätön aika.
pub fn set_missing(result: &mut StoredResult, row: u32) -> Result<(), CoreError> {
    if !result.ilman_emit
        || !result.rivit.iter().any(|r| {
            r.output_index == row && matches!(r.station_type.as_str(), "AT" | "JAT" | "ULA" | "M")
        })
    {
        return Err(CoreError::InvalidInput(
            "Valitse paperikortin aika-asema".into(),
        ));
    }
    result.kasin_annetut_kellot.retain(|r| *r != row);
    if !result.puuttuvat_aikamerkinnat.contains(&row) {
        result.puuttuvat_aikamerkinnat.push(row);
    }
    let r = result
        .rivit
        .iter_mut()
        .find(|r| r.output_index == row)
        .unwrap();
    r.clock = [0; 3];
    result.hyvaksytty = false;
    result.kasikellot_muuttuneet = true;
    Ok(())
}

pub(crate) fn resolve_missing(result: &mut StoredResult) {
    if !result.ilman_emit {
        return;
    }
    let missing = &result.puuttuvat_aikamerkinnat;
    let mut terminal = None;
    let mut unresolved = false;
    for r in &mut result.rivit {
        if missing.contains(&r.output_index) {
            let pending = format!("Aseman {} {} manuaalinen aika puuttuu", r.sequence, r.name);
            result.huomautukset.retain(|m| m != &pending);
            r.note = format!("{}-merkintä puuttuu kortista", r.station_type);
            match StationKind::from_legacy(&r.station_type) {
                Ok(StationKind::Jat | StationKind::Finish) => terminal = Some(r.note.clone()),
                Ok(StationKind::Ula) => unresolved = true,
                _ => {}
            }
        }
    }
    if let Some(reason) = terminal {
        result.tila = "KESK".into();
        result.syy = reason;
    } else if unresolved {
        result.tila = "ILM".into();
        result.syy = "Puuttuva ULA-aika on selvitettävä ennen laskennan hyväksymistä".into();
    } else if result.tila == "ILM"
        && result.pending_manual_clocks().is_empty()
        && result.virheelliset_ajat.is_empty()
    {
        result.tila = "TRK".into();
        result.syy.clear();
    }
}

pub fn missing_at_points(project: &Project, result: &StoredResult) -> Result<i64, CoreError> {
    let count = result
        .rivit
        .iter()
        .filter(|r| {
            r.station_type == "AT" && result.puuttuvat_aikamerkinnat.contains(&r.output_index)
        })
        .count() as i64;
    count
        .checked_mul(
            project
                .to_archive()?
                .scoring_parameters()
                .map_err(|e| CoreError::InvalidInput(e.to_string()))?
                .missing_at,
        )
        .ok_or(CoreError::ArithmeticOverflow(
            "Puuttuvien AT-asemien pisteet",
        ))
}

pub fn capture(project: &Project, number: u32) -> Result<StoredResult, CoreError> {
    application::capture_manual_result(project, number)
}

/// Korvaa nykyiseen Muu-kenttään sisältyvän AT-osuuden myös paikallisissa korjauksissa.
pub(crate) fn sync_points(
    project: &mut Project,
    number: u32,
    result: &mut StoredResult,
    input: Option<i64>,
) -> Result<(), CoreError> {
    if !result.ilman_emit {
        return Ok(());
    }
    let current = missing_at_points(project, result)?;
    let driver = project
        .kilpailijat
        .iter_mut()
        .find(|d| d.numero == number)
        .ok_or_else(|| CoreError::InvalidInput("Kilpailijaa ei löydy".into()))?;
    let previous = driver
        .tallennettu_tulos
        .as_ref()
        .map_or(0, |r| r.puuttuvien_at_pisteet);
    let replace = |total: i64| {
        total
            .checked_sub(previous)
            .unwrap_or(0)
            .max(0)
            .checked_add(current)
            .ok_or(CoreError::ArithmeticOverflow("Puuttuvan AT:n pistekorjaus"))
    };
    driver.lisavirheet.muut_virhepisteet =
        replace(input.unwrap_or(driver.lisavirheet.muut_virhepisteet))?;
    if let Some(source) = &mut driver.lisavirheet.tulkki_siirto {
        source.muut_virhepisteet = replace(source.muut_virhepisteet)?;
    }
    result.puuttuvien_at_pisteet = current;
    Ok(())
}
