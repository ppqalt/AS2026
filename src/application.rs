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
            status: if a.can_publish_competitor(c) {
                ranking::Status::from_legacy(&c.status)
            } else {
                // Säilytä lähtijä- ja keskeneräisyystilastot, mutta älä anna
                // hyväksymättömälle tulokselle sijaa, Np:tä tai Sp:tä.
                ranking::Status::Unknown("HYVÄKSYMÄTÖN".into())
            },
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

/// Muodostaa Tarkista-toiminnon uuden asematuloksen korttiluvusta.
/// Vanhoja käsikelloja tai asemarivien pistekorjauksia ei käytetä lähtötietona.
/// Kutsuja tallentaa palautetun tuloksen vasta toiminnon onnistuttua.
pub fn capture_project_result(
    project: &crate::native::Project,
    number: u32,
) -> Result<crate::corrections::StoredResult, CoreError> {
    let archive = project.to_archive()?;
    let competitor = archive
        .competitors
        .iter()
        .find(|c| c.number == number)
        .ok_or_else(|| invalid(format!("Kilpailijaa {number} ei löydy")))?;
    capture_archive_operational(&archive, competitor)
}

fn capture_archive_operational(
    archive: &Archive,
    c: &Competitor,
) -> Result<crate::corrections::StoredResult, CoreError> {
    let card: u32 = c
        .emit_card
        .trim()
        .parse()
        .map_err(|_| invalid("Emit-numero on virheellinen"))?;
    if archive
        .competitors
        .iter()
        .filter(|d| d.emit_card.trim().parse::<u32>().ok() == Some(card))
        .count()
        != 1
    {
        return Err(invalid("Emit-numero kuuluu usealle kilpailijalle"));
    }
    let logs = parse_saved_emit(
        archive
            .documents
            .get("emit.txt")
            .ok_or_else(|| invalid("Korttilukua ei ole vielä tallennettu"))?,
    )
    .map_err(|e| invalid(e.to_string()))?;
    let read = logs
        .iter()
        .find(|l| l.card_number == card)
        .ok_or_else(|| invalid("Kilpailijan korttilukua ei ole vielä tallennettu"))?
        .accepted()?;
    let route = route_for(archive, c)?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    let matched = matching::match_route(&route, &read.interior_punches, start(c)?)?;
    let source = matched
        .into_iter()
        .map(|r| {
            let s = r.station.as_ref();
            crate::legacy::StationResult {
                output_index: r.output_index as u32,
                sequence: s.map_or(0, |s| s.sequence),
                station_type: s.map_or(StationKind::At, |s| s.kind).as_legacy().into(),
                name: s.map_or("Ylimääräinen asema", |s| s.label.as_str()).into(),
                elapsed_seconds: r.punch.map_or(0, |p| i64::from(p.elapsed_seconds)),
                expected_code: s.map_or(999, |s| s.expected_code),
                received_code: r.punch.map_or(0, |p| p.code),
                target_seconds: s.map_or(0, |s| s.target_seconds),
                actual_seconds: 0,
                clock: [0; 3],
                rule: s.map_or(0, |s| s.rule as i64),
                section_text: s.map_or("", |s| s.section_time_text.as_str()).into(),
                signed_time_penalty: 0,
                other_penalty: 0,
                total_penalty: 0,
                note: String::new(),
            }
        })
        .collect::<Vec<_>>();
    let options = operational_options(archive, c)?;
    snapshot_for_calculation(
        archive,
        c,
        timing::calculate_operational(&source, &route, &[], &options)?,
    )
}
fn operational_options(
    archive: &Archive,
    c: &Competitor,
) -> Result<LiveCalculationOptions, CoreError> {
    Ok(LiveCalculationOptions {
        official_start: start(c)?,
        reset_correction_seconds: c.correction_seconds,
        timing_method: TimingMethod::from_legacy(integer(
            archive
                .basic
                .identity
                .get(7)
                .ok_or_else(|| invalid("Ajanottotapa puuttuu"))?,
            "Ajanottotapa",
        )?)?,
        last_punch_elapsed_seconds: 0,
        cumulative_limit_minutes: optional_nonnegative(
            class_for(archive, &c.class_code)?.fields.get(7),
            "Luokan kokonaismyöhästymisraja",
        )?,
        parameters: archive
            .scoring_parameters()
            .map_err(|e| invalid(e.to_string()))?,
        policy: CalculationPolicy::As2026OperationalV1,
    })
}

fn calculation_rows(result: &LiveCalculation) -> Vec<crate::legacy::StationResult> {
    result
        .rows
        .iter()
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
                expected_code: station.map_or(999, |s| s.expected_code),
                received_code: row.received_code,
                target_seconds: row.target_seconds,
                actual_seconds: row.actual_seconds,
                clock: row.clock_seconds.map_or([0; 3], hms),
                rule: row.effective_rule as i64,
                section_text: station.map_or("", |s| s.section_time_text.as_str()).into(),
                signed_time_penalty: row.signed_time_points,
                other_penalty: row.other_points,
                total_penalty: row.total_points,
                note: row.note.clone(),
            }
        })
        .collect()
}
fn snapshot_for_calculation(
    archive: &Archive,
    competitor: &Competitor,
    result: LiveCalculation,
) -> Result<crate::corrections::StoredResult, CoreError> {
    let route = route_for(archive, competitor)?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    Ok(crate::corrections::StoredResult {
        laskentatapa: if result.policy == CalculationPolicy::As2026OperationalV1 {
            crate::corrections::StoredCalculationPolicy::As2026Operational
        } else {
            crate::corrections::StoredCalculationPolicy::As2015aStatic
        },
        hyvaksytty: false,
        kasin_annetut_kellot: Vec::new(),
        virheelliset_ajat: result
            .diagnostics
            .iter()
            .filter(|d| d.code == "operational-invalid-time")
            .map(|d| d.message.clone())
            .collect(),
        rata: route
            .iter()
            .map(crate::corrections::StoredRouteStation::from)
            .collect(),
        rivit: calculation_rows(&result)
            .iter()
            .map(crate::corrections::StoredRow::from)
            .collect(),
        aikavirheet: result.totals.time_points,
        asemien_muut_virheet: result.totals.other_points,
        tila: result.status.as_legacy().into(),
        syy: result.reason,
        maalikello_sekuntia: result.finish_clock_seconds,
        kesto_sekuntia: result.total_duration_seconds,
        korjaus_sekuntia: competitor.correction_seconds,
        kasikellot_muuttuneet: false,
        huomautukset: result.diagnostics.into_iter().map(|d| d.message).collect(),
    })
}

/// Tarkistaa tallennetun tuloksen ja nykyisen radan yhteensopivuuden laskematta pisteitä uudelleen.
pub fn validate_project_result(
    project: &crate::native::Project,
    number: u32,
    stored: &crate::corrections::StoredResult,
) -> Result<(), CoreError> {
    let archive = project.to_archive()?;
    let c = archive
        .competitors
        .iter()
        .find(|c| c.number == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let route = route_for(&archive, c)?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    stored.validate_route(&route)
}

/// Tutki käyttää tallennettuja asemarivejä ja käsikelloja. Se ei lue eikä muuta
/// raakaleimauksia. Määrittelemättömän vanhan laskentapolun kohdalla palautetaan virhe.
pub fn recalculate_stored_result(
    project: &crate::native::Project,
    number: u32,
    stored: &crate::corrections::StoredResult,
) -> Result<crate::corrections::StoredResult, CoreError> {
    let archive = project.to_archive()?;
    let c = archive
        .competitors
        .iter()
        .find(|c| c.number == number)
        .ok_or_else(|| invalid(format!("Kilpailijaa {number} ei löydy")))?;
    let route = route_for(&archive, c)?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    stored.validate_route(&route)?;
    let rows = stored.as_station_results();
    let options = operational_options(&archive, c)?;
    let calculated =
        timing::calculate_operational(&rows, &route, &stored.kasin_annetut_kellot, &options)?;
    let mut result = snapshot_for_calculation(&archive, c, calculated)?;
    result.kasin_annetut_kellot = stored.kasin_annetut_kellot.clone();
    // Syötetty käsikello säilyy sellaisenaan myös vuorokauden vaihtuessa.
    for row in &mut result.rivit {
        if result.kasin_annetut_kellot.contains(&row.output_index)
            && let Some(original) = stored
                .rivit
                .iter()
                .find(|r| r.output_index == row.output_index)
        {
            row.clock = original.clock;
        }
    }
    Ok(result)
}

fn apply_result(
    c: &mut Competitor,
    result: &crate::corrections::StoredResult,
) -> Result<(), CoreError> {
    let total = crate::scoring::competitor_total([
        result.aikavirheet,
        result.asemien_muut_virheet,
        header_number(c, 22, "Puuttuvien RT pisteet")?,
        header_number(c, 25, "Muut virhepisteet")?,
        header_number(c, 24, "Ylimääräisten RT pisteet")?,
    ])?;
    c.header_fields[19] = result.aikavirheet.to_string();
    c.header_fields[20] = result.asemien_muut_virheet.to_string();
    c.header_fields[26] = total.to_string();
    c.total_penalties = total;
    for (index, value) in hms(result.kesto_sekuntia).into_iter().enumerate() {
        c.header_fields[13 + index] = value.to_string();
    }
    for (index, value) in hms(result.maalikello_sekuntia).into_iter().enumerate() {
        c.header_fields[16 + index] = value.to_string();
    }
    c.status = result.tila.clone();
    c.header_fields[28] = c.status.clone();
    c.header_fields[29] = result.syy.clone();
    c.results = result.as_station_results();
    c.declared_row_count = u32::try_from(c.results.len())
        .map_err(|_| CoreError::ArithmeticOverflow("asematulosten määrä"))?;
    c.header_fields[27] = c.declared_row_count.to_string();
    Ok(())
}

/// Rakentaa tulosnäkymän. Käsin hyväksytty asematulos säilyy, vaikka kortin
/// lukuhistoriaan lisätään uusi luku. Tarkista korvaa sen vain käyttäjän pyynnöstä.
/// Viisi pisteosaa yhdistetään aina; käsin valittu tila asetetaan laskennan jälkeen.
pub fn calculated_project(project: &crate::native::Project) -> Result<Archive, CoreError> {
    let mut automatic = project.clone();
    for driver in &mut automatic.kilpailijat {
        driver.tila = None;
        driver.keskeytys_vahvistettu = false;
    }
    let mut archive = automatic.to_archive()?;
    let mut resolved = Vec::new();
    for competitor in &archive.competitors {
        let driver = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == competitor.number)
            .expect("projektin kilpailija");
        let (result, blocked) = if let Some(stored) = &driver.tallennettu_tulos {
            let validation = route_for(&archive, competitor)
                .and_then(|r| r.to_core().map_err(|e| invalid(e.to_string())))
                .and_then(|route| stored.validate_route(&route));
            let blocked = validation.is_err();
            (validation.map(|()| stored.clone()), blocked)
        } else {
            (capture_archive_operational(&archive, competitor), false)
        };
        resolved.push((competitor.number, result, blocked));
    }
    for (number, result, blocked) in resolved {
        let competitor = archive
            .competitors
            .iter_mut()
            .find(|c| c.number == number)
            .expect("laskennan kilpailija");
        let has_result = result.is_ok();
        let operational_incomplete = result.as_ref().is_ok_and(|r| {
            r.kasikellot_muuttuneet
                || (r.laskentatapa
                    == crate::corrections::StoredCalculationPolicy::As2026Operational
                    && !r.can_approve())
        });
        let approved = result
            .as_ref()
            .is_ok_and(|r| r.hyvaksytty && r.can_approve());
        let confirmed_interruption = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .is_some_and(|d| d.has_confirmed_interruption());
        let requested_status = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .and_then(|d| d.tila.as_ref());
        match result {
            Ok(mut result) => {
                // Ajanoton lähtötietojen muutos ei laske tallennettuja pisteitä
                // uudelleen. Vanhat rivit säilyvät, mutta TRK ei saa julkaista niitä.
                if operational_incomplete && result.tila == "TRK" {
                    result.tila = "ILM".into();
                    result.syy = if result.kasikellot_muuttuneet {
                        "Ajanoton tietoja tai käsikelloja on muutettu. Valitse Tutki.".into()
                    } else {
                        "Tulos odottaa tarkastusta. Täydennä puuttuvat tai virheelliset ajat."
                            .into()
                    };
                }
                for message in &result.huomautukset {
                    archive
                        .warnings
                        .push(format!("Kilpailija {number}: {message}"));
                }
                if result.kasikellot_muuttuneet {
                    archive.warnings.push(format!("Kilpailija {number}: käsikelloa on muutettu. Pisteet säilyvät ennallaan, kunnes valitaan Tutki."));
                }
                apply_result(competitor, &result)?;
            }
            Err(_) if confirmed_interruption => {}
            Err(error) => archive
                .warnings
                .push(format!("Kilpailijan {number} tulos puuttuu: {error}")),
        }
        let missing_classified_result = !has_result
            && !confirmed_interruption
            && requested_status.is_some_and(|s| matches!(s.tunnus.as_str(), "TRK" | "KESK"));
        if missing_classified_result {
            archive.warnings.push(format!("Kilpailija {number}: käsin valittua tarkastustilaa ei käytetä ilman laskettua asematulosta. Valittu tila säilyy kilpailijan tiedoissa."));
        }
        if blocked || missing_classified_result {
            competitor.status = "ILM".into();
            competitor.header_fields[28] = "ILM".into();
            competitor.header_fields[29] = if blocked {
                "Tallennettu tulos ei vastaa rataa tai sen pisteitä. Valitse Tarkista."
            } else {
                "Laskettu asematulos puuttuu. Tarkista kilpailijan korttiluku."
            }
            .into();
            competitor.results.clear();
            competitor.declared_row_count = 0;
            competitor.header_fields[27] = "0".into();
        } else if operational_incomplete && requested_status.is_some_and(|s| s.tunnus == "TRK") {
            archive.warnings.push(format!("Kilpailija {number}: keskeneräistä tulosta ei merkitä tarkastetuksi. Täydennä käsikellot ja valitse Tutki."));
        } else if let Some(status) = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .and_then(|d| d.tila.as_ref())
        {
            competitor.status = status.tunnus.clone();
            competitor.header_fields[28] = status.tunnus.clone();
            competitor.header_fields[29] = status.syy.clone();
        }
        let publication = if confirmed_interruption {
            competitor.status = "KESK".into();
            competitor.header_fields[28] = "KESK".into();
            competitor.header_fields[29] = requested_status.map_or("", |s| s.syy.as_str()).into();
            crate::legacy::ResultPublication::ConfirmedInterruption
        } else if approved && competitor.status != "ILM" && !blocked {
            crate::legacy::ResultPublication::Approved
        } else {
            crate::legacy::ResultPublication::Pending
        };
        archive
            .result_publication
            .as_mut()
            .expect("AS2026:n hyväksyntätila")
            .insert(number, publication);
    }
    Ok(archive)
}
