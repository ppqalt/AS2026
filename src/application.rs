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
fn route_for<'a>(a: &'a Archive, c: &Competitor) -> Result<&'a crate::legacy::Route, CoreError> {
    let class = class_for(a, &c.class_code)?;
    // AS2015A:n luokan sarake 2 vertautuu R-tietueen nimeen (0x41553e).
    // Uuden TOML-muodon luokan rata on sen sijaan eksplisiittinen numero.
    let imported = a.documents.contains_key("Radat.txt");
    let routes: Vec<_> = a
        .routes
        .iter()
        .filter(|route| {
            if imported {
                route.name == class.route
            } else {
                class.route.trim().parse::<u32>().ok() == Some(route.index)
            }
        })
        .collect();
    match routes.as_slice() {
        [route] => Ok(route),
        _ => Err(invalid(format!(
            "Luokan {} rata {:?} puuttuu tai on moniselitteinen",
            c.class_code, class.route
        ))),
    }
}

fn calculate_punches(
    a: &Archive,
    c: &Competitor,
    punches: &[Punch],
    last_punch_elapsed_seconds: u32,
) -> Result<LiveCalculation, CoreError> {
    let class = class_for(a, &c.class_code)?;
    let official_start = start(c)?;
    let rows = matching::match_route(
        &route_for(a, c)?
            .to_core()
            .map_err(|e| invalid(e.to_string()))?,
        punches,
        official_start,
    )?;
    let options = LiveCalculationOptions {
        official_start,
        reset_correction_seconds: c.correction_seconds,
        timing_method: TimingMethod::from_legacy(integer(
            a.basic
                .identity
                .get(7)
                .ok_or_else(|| invalid("Ajanottotapa puuttuu"))?,
            "ajanottotapa",
        )?)?,
        last_punch_elapsed_seconds,
        cumulative_limit_minutes: optional_nonnegative(
            class.fields.get(7),
            "Luokan kokonaismyöhästymisraja",
        )?,
        parameters: a.scoring_parameters().map_err(|e| invalid(e.to_string()))?,
        policy: CalculationPolicy::RecoveredAs2015aStaticV2,
    };
    timing::calculate_live_with_trailing_extras(&rows, &options)
}

/// Johdetun tulosarkiston HMS-kentät säilyttävät myös keskiyön ylittävän
/// kokonaissekuntiarvon: esimerkiksi 24:05:00, eivät piilota vuorokauden vaihtoa.
fn hms(seconds: i64) -> [i64; 3] {
    let hours = seconds.div_euclid(3600);
    let within_hour = seconds.rem_euclid(3600);
    [hours, within_hour / 60, within_hour % 60]
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
    a.scoring_parameters().map_err(|e| invalid(e.to_string()))?;
    TimingMethod::from_legacy(integer(
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
            // AS2015A:n emit.txt-uusintahaku pysähtyy ensimmäiseen osuvaan
            // tietueeseen (0x00414f5c). Uuden TOML-kilpailun sovitin valitsee
            // erikseen uusimman kelvollisen fyysisen lukeman jo ennen tätä.
            let log = matching.first().ok_or_else(|| {
                invalid(format!(
                    "Kortin {card} raakaleimauksia ei löydy emit.txt-tiedostosta"
                ))
            })?;
            if matching.len() > 1 {
                notes.push(format!(
                    "Kortille löytyi {} lukua; käytettiin tiedoston ensimmäistä lukua riviltä {}",
                    matching.len(),
                    log.line
                ));
            }
            let read = log.accepted()?;
            let result = calculate_punches(
                a,
                c,
                &read.interior_punches,
                read.last_punch_elapsed_seconds,
            )?;
            if result.finish_clock_seconds != read.finish_clock_seconds {
                notes.push(format!("Nollauskorjaukseen perustuva maalikello {} s ja lukukelloon perustuva arvio {} s eroavat",result.finish_clock_seconds,read.finish_clock_seconds));
            }
            if a.documents.contains_key("Kilp.txt")
                && result.totals.total_points != c.total_penalties
            {
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

    let records = frame.punches_before_reader_marker()?;
    let route = route_for(archive, competitor)?;
    let calculation = calculate_punches(
        archive,
        competitor,
        &records[1..],
        records[records.len() - 1].elapsed_seconds,
    )?;
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
/// Valmistelee vanhojen tallennettujen P-rivien ajanottokontekstin, muuttamatta
/// kilpailijaa tai laskematta asemien pisteitä uudelleen. Varsinainen context-1
/// asemalaskenta on erillinen toteuttamaton vaihe.
pub fn prepare_stored_competitor_timing(
    archive: &Archive,
    number: u32,
) -> Result<timing::PreparedStoredTiming, CoreError> {
    let c = archive
        .competitors
        .iter()
        .find(|c| c.number == number)
        .ok_or_else(|| invalid(format!("Kilpailijaa {number} ei löydy")))?;
    let last = c
        .results
        .last()
        .ok_or_else(|| invalid("Kilpailijalla ei ole tallennettuja asematuloksia"))?;
    let hours = header_number(c, 16, "Maalin tunti")?;
    let minutes = header_number(c, 17, "Maalin minuutti")?;
    let seconds = header_number(c, 18, "Maalin sekunti")?;
    if hours < 0 || !(0..60).contains(&minutes) || !(0..60).contains(&seconds) {
        return Err(invalid("Tallennettu maalikello on virheellinen"));
    }
    let finish_clock = hours
        .checked_mul(3600)
        .and_then(|v| minutes.checked_mul(60).and_then(|m| v.checked_add(m)))
        .and_then(|v| v.checked_add(seconds))
        .ok_or(CoreError::ArithmeticOverflow("tallennettu maalikello"))?;
    let method = integer(
        archive
            .basic
            .identity
            .get(7)
            .ok_or_else(|| invalid("Ajanottotapa puuttuu"))?,
        "ajanottotapa",
    )?;
    timing::prepare_stored_timing(&timing::StoredTimingInput {
        official_start: start(c)?,
        stored_finish_clock_seconds: finish_clock,
        last_row_elapsed_seconds: last.elapsed_seconds,
        finish_expected_code: last.expected_code,
        reset_correction_seconds: c.correction_seconds,
        timing_method: TimingMethod::from_legacy(method)?,
    })
}

/// Valmistelee tallennetut P-kellot erillisellä vanhan ohjelman
/// uudelleenlaskentapolulla. Tämä ei vielä muuta asemien virhepisteitä.
pub fn prepare_stored_competitor_station_times(
    archive: &Archive,
    number: u32,
) -> Result<timing::PreparedStoredStationTimes, CoreError> {
    let prepared = prepare_stored_competitor_timing(archive, number)?;
    let c = archive
        .competitors
        .iter()
        .find(|c| c.number == number)
        .ok_or_else(|| invalid(format!("Kilpailijaa {number} ei löydy")))?;
    let rows = c
        .results
        .iter()
        .map(|row| timing::StoredStationTime {
            expected_code: row.expected_code,
            elapsed_seconds: row.elapsed_seconds,
            clock: row.clock,
        })
        .collect::<Vec<_>>();
    timing::prepare_stored_station_times(&prepared, &rows)
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

/// Rakentaa uuden kilpailun tulosnäkymän nykyisistä lähtötiedoista.
/// Johdettuja pisteitä ei tallenneta toiseksi totuudeksi TOML-tiedostoon.
/// Tämä kehitysversion asemalaskenta ei vahvista AS2015A-vastaavuutta.
pub fn calculated_project(project: &crate::native::Project) -> Result<Archive, CoreError> {
    let mut archive = project.to_archive()?;
    let calculations = calculate(&archive)?;
    for (competitor, preview) in archive.competitors.iter_mut().zip(calculations.competitors) {
        match preview.calculation {
            Ok(result) => {
                for diagnostic in &result.diagnostics {
                    archive.warnings.push(format!(
                        "Kilpailija {}: {}",
                        competitor.number, diagnostic.message
                    ));
                }
                // Asemarivien summat ovat vain kaksi viidestä kokonaispisteosasta.
                // Erilliset RT-pistesummat säilyvät tallennettuina, vaikka
                // kilpailun parametrien arvoja muutettaisiin myöhemmin.
                let missing_rt = header_number(competitor, 22, "Puuttuvien RT pisteet")?;
                let extra_rt = header_number(competitor, 24, "Ylimääräisten RT pisteet")?;
                let other = header_number(competitor, 25, "Muut virhepisteet")?;
                let total = crate::scoring::competitor_total([
                    result.totals.time_points,
                    result.totals.other_points,
                    missing_rt,
                    other,
                    extra_rt,
                ])?;
                competitor.header_fields[19] = result.totals.time_points.to_string();
                competitor.header_fields[20] = result.totals.other_points.to_string();
                competitor.header_fields[26] = total.to_string();
                let elapsed = hms(result.total_duration_seconds);
                let finish = hms(result.finish_clock_seconds);
                for (index, value) in elapsed.into_iter().enumerate() {
                    competitor.header_fields[13 + index] = value.to_string();
                }
                for (index, value) in finish.into_iter().enumerate() {
                    competitor.header_fields[16 + index] = value.to_string();
                }
                competitor.total_penalties = total;
                competitor.status = result.status.as_legacy().into();
                competitor.header_fields[28] = competitor.status.clone();
                competitor.header_fields[29] = result.reason.clone();
                competitor.results = result
                    .rows
                    .into_iter()
                    .map(|row| {
                        let station = row.source.station.as_ref();
                        crate::legacy::StationResult {
                            output_index: row.source.output_index as u32,
                            sequence: station.map_or(0, |s| s.sequence),
                            station_type: station
                                .map_or(StationKind::At, |s| s.kind)
                                .as_legacy()
                                .into(),
                            name: station
                                .map_or("Ylimääräinen asema", |s| s.label.as_str())
                                .into(),
                            elapsed_seconds: row.derived_elapsed_seconds,
                            expected_code: station.map_or(0, |s| s.expected_code),
                            received_code: row.received_code,
                            target_seconds: row.target_seconds,
                            actual_seconds: row.actual_seconds,
                            clock: row.clock_seconds.map_or([0; 3], hms),
                            rule: row.effective_rule as i64,
                            section_text: station
                                .map_or("", |s| s.section_time_text.as_str())
                                .into(),
                            signed_time_penalty: row.signed_time_points,
                            other_penalty: row.other_points,
                            total_penalty: row.total_points,
                            note: row.note,
                        }
                    })
                    .collect();
                competitor.declared_row_count = u32::try_from(competitor.results.len())
                    .map_err(|_| CoreError::ArithmeticOverflow("asematulosten määrä"))?;
                competitor.header_fields[27] = competitor.declared_row_count.to_string();
            }
            Err(error) => archive.warnings.push(format!(
                "Kilpailijan {} tulos puuttuu: {error}",
                competitor.number
            )),
        }
    }
    Ok(archive)
}
