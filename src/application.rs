//! Portable application operations. Imported records remain separate from previews.
use crate::{
    ClockTime, CoreError, Punch, StationKind,
    emit::EmitFrame,
    legacy::{Archive, Competitor, parse_saved_emit},
    matching, ranking,
    timing::{self, CalculationPolicy, LiveCalculation, LiveCalculationOptions, TimingMethod},
};

#[derive(Debug)]
pub struct CompetitorPreview {
    pub number: u32,
    pub driver: String,
    pub class_code: String,
    pub imported_total: i64,
    pub calculation: Result<LiveCalculation, CoreError>,
    pub notes: Vec<String>,
}
#[derive(Debug)]
pub struct CalculationReport {
    pub competition_name: String,
    pub competitors: Vec<CompetitorPreview>,
    pub warnings: Vec<String>,
}
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn integer(value: &str, label: &str) -> Result<i64, CoreError> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("{label}: kokonaisluku puuttuu tai on virheellinen")))
}
fn header_number(c: &Competitor, index: usize, label: &str) -> Result<i64, CoreError> {
    integer(
        c.header_fields
            .get(index)
            .ok_or_else(|| invalid(format!("Kilpailijan {} kenttä {label} puuttuu", c.number)))?,
        label,
    )
}
fn header_count(c: &Competitor, index: usize, label: &str) -> Result<u32, CoreError> {
    u32::try_from(header_number(c, index, label)?)
        .map_err(|_| invalid(format!("{label}: negatiivinen tai liian suuri määrä")))
}
fn start(c: &Competitor) -> Result<ClockTime, CoreError> {
    ClockTime::new(
        header_count(c, 10, "lähtötunti")?,
        header_count(c, 11, "lähtöminuutti")?,
        header_count(c, 12, "lähtösekunti")?,
    )
}
fn competition_name(a: &Archive) -> String {
    a.basic
        .identity
        .get(1)
        .map_or("Nimetön kilpailu", String::as_str)
        .trim()
        .to_owned()
}
fn class_for<'a>(
    a: &'a Archive,
    code: &str,
) -> Result<&'a crate::legacy::ClassDefinition, CoreError> {
    let found = a
        .basic
        .classes
        .iter()
        .filter(|c| c.code == code)
        .collect::<Vec<_>>();
    if found.len() != 1 {
        return Err(invalid(format!(
            "Luokka {code:?} puuttuu tai on moniselitteinen"
        )));
    }
    Ok(found[0])
}
fn optional_nonnegative(value: Option<&String>, label: &str) -> Result<Option<i64>, CoreError> {
    let Some(value) = value else {
        return Err(invalid(format!("{label} puuttuu")));
    };
    if value.trim().is_empty() {
        return Ok(None);
    }
    let n = integer(value, label)?;
    if n < 0 {
        return Err(invalid(format!("{label} ei saa olla negatiivinen")));
    }
    Ok(Some(n))
}

/// Rebuild supported live-card calculations from the saved raw log. Never uses stored
/// station penalties as new calculation inputs and never overwrites imported totals.
pub fn calculate(a: &Archive) -> Result<CalculationReport, CoreError> {
    let params = a.scoring_parameters().map_err(|e| invalid(e.to_string()))?;
    let method = TimingMethod::from_legacy(integer(
        a.basic
            .identity
            .get(7)
            .ok_or_else(|| invalid("Ajanottotapa puuttuu"))?,
        "ajanottotapa",
    )?)?;
    let logs = match a.documents.get("emit.txt") {
        Some(doc) => parse_saved_emit(doc).map_err(|e| invalid(e.to_string()))?,
        None => Vec::new(),
    };
    let mut competitors = Vec::new();
    for c in &a.competitors {
        let mut notes=vec!["Esikatselu sisältää asemarivien virhepisteet. Erillisiä RT-lisäyksiä ja muita kilpailijakohtaisia virhepisteitä ei vielä yhdistetä siihen.".to_owned()];
        let calculation = (|| {
            if c.status != "TRK" && c.status != "KESK" && c.status != "ILM" {
                return Err(CoreError::Unsupported(format!(
                    "Tilaa {} ei lasketa korttiesikatselussa",
                    c.status
                )));
            }
            let class = class_for(a, &c.class_code)?;
            let routes = a
                .routes
                .iter()
                .filter(|r| {
                    if let Ok(number) = class.route.trim().parse::<u32>() {
                        r.index == number
                    } else {
                        r.name == class.route
                    }
                })
                .collect::<Vec<_>>();
            if routes.len() != 1 {
                return Err(invalid(format!(
                    "Luokan {} rata {:?} puuttuu tai on moniselitteinen",
                    c.class_code, class.route
                )));
            }
            let card: u32 = c.emit_card.trim().parse().map_err(|_| {
                invalid(format!(
                    "Kilpailijan {} Emit-numero on virheellinen",
                    c.number
                ))
            })?;
            if a.competitors
                .iter()
                .filter(|other| other.emit_card.trim().parse::<u32>().ok() == Some(card))
                .count()
                != 1
            {
                return Err(invalid(format!(
                    "Emit-kortti {card} kuuluu usealle kilpailijalle; lukua ei kohdisteta arvaamalla"
                )));
            }
            let matching = logs
                .iter()
                .filter(|l| l.card_number == card)
                .collect::<Vec<_>>();
            let log = matching.last().ok_or_else(|| {
                invalid(format!(
                    "Kortin {card} raakaleimauksia ei löydy emit.txt-tiedostosta"
                ))
            })?;
            if matching.len() > 1 {
                notes.push(format!(
                    "Kortille löytyi {} lukua; käytettiin tiedoston viimeistä lukua riviltä {}",
                    matching.len(),
                    log.line
                ));
            }
            let read = log.accepted()?;
            let official = start(c)?;
            let rows = matching::match_route(
                &routes[0].to_core().map_err(|e| invalid(e.to_string()))?,
                &read.interior_punches,
                official,
            )?;
            let cumulative_limit =
                optional_nonnegative(class.fields.get(7), "Luokan kokonaismyöhästymisraja")?;
            let options = LiveCalculationOptions {
                official_start: official,
                reset_correction_seconds: c.correction_seconds,
                timing_method: method,
                last_punch_elapsed_seconds: read.last_punch_elapsed_seconds,
                cumulative_limit_minutes: cumulative_limit,
                parameters: params.clone(),
                policy: CalculationPolicy::RecoveredLiveV1,
            };
            let result = timing::calculate_live(&rows, &options)?;
            if result.finish_clock_seconds != read.finish_clock_seconds {
                notes.push(format!("Nollauskorjaukseen perustuva maalikello {} s ja lukukelloon perustuva arvio {} s eroavat",result.finish_clock_seconds,read.finish_clock_seconds));
            }
            if result.totals.total_points != c.total_penalties {
                notes.push(format!(
                    "Asemarivien summa {} p poikkeaa tallennetusta kokonaistuloksesta {} p",
                    result.totals.total_points, c.total_penalties
                ));
            }
            Ok(result)
        })();
        competitors.push(CompetitorPreview {
            number: c.number,
            driver: c.driver.clone(),
            class_code: c.class_code.clone(),
            imported_total: c.total_penalties,
            calculation,
            notes,
        });
    }
    Ok(CalculationReport {
        competition_name: competition_name(a),
        competitors,
        warnings: a.warnings.clone(),
    })
}

/// Yhden suoraan luetun Emit-kortin laskennan esikatselu.
/// Kilpailutiedostoihin tai kilpailijan tallennettuihin tuloksiin ei kirjoiteta.
#[derive(Debug)]
pub struct LiveCardPreview {
    pub card_number: u32,
    pub competitor_number: u32,
    pub driver: String,
    pub navigator: String,
    pub class_code: String,
    pub route_name: String,
    pub calculation: LiveCalculation,
}

/// Kohdistaa kortin yksiselitteiseen kilpailijaan ja laskee tämän radan.
/// Lukukelloa ei arvata: käytetään Kilp.txt:n lähtöaikaa ja nollauskorjausta.
pub fn calculate_live_frame(
    archive: &Archive,
    frame: &EmitFrame,
) -> Result<LiveCardPreview, CoreError> {
    let candidates: Vec<_> = archive
        .competitors
        .iter()
        .filter(|c| c.emit_card.trim().parse::<u32>().ok() == Some(frame.card_number))
        .collect();
    let competitor = match candidates.as_slice() {
        [] => {
            return Err(invalid(format!(
                "Emit-korttia {} ei löydy kilpailijoista",
                frame.card_number
            )));
        }
        [competitor] => *competitor,
        _ => {
            return Err(invalid(format!(
                "Emit-kortti {} kuuluu usealle kilpailijalle; kohdistusta ei arvata",
                frame.card_number
            )));
        }
    };

    let marker = frame
        .records
        .iter()
        .position(|p| p.code == 250)
        .ok_or(CoreError::MissingMarker)?;
    if marker == 0 {
        return Err(CoreError::MarkerAtStart);
    }
    let punches: &[Punch] = &frame.records[1..marker];
    let class = class_for(archive, &competitor.class_code)?;
    let routes: Vec<_> = archive
        .routes
        .iter()
        .filter(|route| {
            if let Ok(number) = class.route.trim().parse::<u32>() {
                route.index == number
            } else {
                route.name == class.route
            }
        })
        .collect();
    let route = match routes.as_slice() {
        [route] => *route,
        _ => {
            return Err(invalid(format!(
                "Luokan {} rata {:?} puuttuu tai on moniselitteinen",
                competitor.class_code, class.route
            )));
        }
    };
    let official_start = start(competitor)?;
    let rows = matching::match_route(
        &route.to_core().map_err(|e| invalid(e.to_string()))?,
        punches,
        official_start,
    )?;
    let method = TimingMethod::from_legacy(integer(
        archive
            .basic
            .identity
            .get(7)
            .ok_or_else(|| invalid("Ajanottotapa puuttuu"))?,
        "ajanottotapa",
    )?)?;
    let options = LiveCalculationOptions {
        official_start,
        reset_correction_seconds: competitor.correction_seconds,
        timing_method: method,
        last_punch_elapsed_seconds: frame.records[marker - 1].elapsed_seconds,
        cumulative_limit_minutes: optional_nonnegative(
            class.fields.get(7),
            "Luokan kokonaismyöhästymisraja",
        )?,
        parameters: archive
            .scoring_parameters()
            .map_err(|e| invalid(e.to_string()))?,
        policy: CalculationPolicy::RecoveredLiveV1,
    };
    let calculation = timing::calculate_live(&rows, &options)?;
    Ok(LiveCardPreview {
        card_number: frame.card_number,
        competitor_number: competitor.number,
        driver: competitor.driver.clone(),
        navigator: competitor.navigator.clone(),
        class_code: competitor.class_code.clone(),
        route_name: route.name.clone(),
        calculation,
    })
}

fn selector(value: Option<&String>, label: &str) -> Result<u8, CoreError> {
    let n = optional_nonnegative(value, label)?.unwrap_or(0);
    u8::try_from(n).map_err(|_| invalid(format!("{label}: liian suuri valinta")))
}
/// Rank stored, imported totals. This operation does not silently substitute preview totals.
pub fn rank_stored(
    a: &Archive,
    tie_policy: ranking::TiePolicy,
) -> Result<ranking::RankingReport, CoreError> {
    let mut input = ranking::RankingInput {
        tie_policy,
        ..Default::default()
    };
    for class in &a.basic.classes {
        input.classes.push(ranking::ClassDefinition {
            code: class.code.clone(),
            np_enabled: selector(class.fields.get(8), "Np")? != 0,
            series_method: selector(class.fields.get(9), "Sp")?,
        });
    }
    for group in &a.basic.combined_classes {
        let name = group
            .get(1)
            .ok_or_else(|| invalid("Yhdistetyn luokan nimi puuttuu"))?
            .trim();
        if name.is_empty() {
            continue;
        }
        let index = group
            .first()
            .ok_or_else(|| invalid("Yhdistetyn luokan numero puuttuu"))?
            .trim();
        let mut class_codes = Vec::new();
        for (i, class) in a.basic.classes.iter().enumerate() {
            let membership = group
                .get(4 + i)
                .ok_or_else(|| invalid(format!("Yhdistetyn luokan {name} jäsenyys puuttuu")))?
                .trim();
            match membership {
                "1" => class_codes.push(class.code.clone()),
                "0" | "" => {}
                _ => {
                    return Err(invalid(format!(
                        "Yhdistetyn luokan {name} jäsenyys on virheellinen"
                    )));
                }
            }
        }
        input
            .combined_classes
            .push(ranking::CombinedClassDefinition {
                code: format!("Y{index}: {name}"),
                class_codes,
                np_enabled: selector(group.get(2), "Yhdistetyn luokan Np")? != 0,
                series_method: selector(group.get(3), "Yhdistetyn luokan Sp")?,
            });
    }
    let needs_custom = input.classes.iter().any(|c| c.series_method == 4)
        || input.combined_classes.iter().any(|c| c.series_method == 4);
    if let Some(doc) = a.documents.get("sarjap.txt") {
        let row = doc
            .records
            .first()
            .ok_or_else(|| invalid("sarjap.txt on tyhjä"))?;
        let mut tokens = row.fields.as_slice();
        while tokens.last().is_some_and(|s| s.trim().is_empty()) {
            tokens = &tokens[..tokens.len() - 1];
        }
        for value in tokens {
            let n = integer(value, "Sarjapiste")?;
            if n < 0 {
                return Err(invalid("Sarjapiste ei saa olla negatiivinen"));
            }
            input.custom_series_points.push(n);
        }
    } else if needs_custom {
        return Err(invalid("Sp 4 edellyttää sarjap.txt-pistetaulukkoa"));
    }
    for (i, c) in a.competitors.iter().enumerate() {
        let class = class_for(a, &c.class_code)?;
        let tie_rows = c
            .results
            .iter()
            .map(|row| {
                Ok(ranking::TieRow {
                    position: row.output_index,
                    station_key: format!(
                        "{}:{}:{}",
                        row.sequence, row.expected_code, row.station_type
                    ),
                    kind: StationKind::from_legacy(&row.station_type)?,
                    signed_time_penalty: row.signed_time_penalty,
                })
            })
            .collect::<Result<Vec<_>, CoreError>>()
            .ok();
        input.competitors.push(ranking::CompetitorInput {
            source_index: i + 1,
            number: i64::from(c.number),
            class_code: c.class_code.clone(),
            route_code: class.route.clone(),
            status: ranking::Status::from_legacy(&c.status),
            total_penalty: c.total_penalties,
            start_hour: header_number(c, 10, "Lähtötunti")?,
            driver_licence: c
                .header_fields
                .get(5)
                .cloned()
                .ok_or_else(|| invalid("Kuljettajan lisenssi puuttuu"))?,
            navigator_licence: c
                .header_fields
                .get(6)
                .cloned()
                .ok_or_else(|| invalid("Kartanlukijan lisenssi puuttuu"))?,
            tie_rows,
            route_errors: Some(ranking::RouteErrors {
                missing_count: header_count(c, 21, "Puuttuvien RT määrä")?,
                missing_sum: header_number(c, 22, "Puuttuvien RT pisteet")?,
                extra_count: header_count(c, 23, "Ylimääräisten RT määrä")?,
                extra_sum: header_number(c, 24, "Ylimääräisten RT pisteet")?,
            }),
        });
    }
    ranking::rank(&input)
}
