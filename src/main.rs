use as2026::{
    ClockTime, CoreError, MatchKind, application, emit, io_message,
    legacy::{self, Archive, Encoding},
    live_report, matching, native, ranking, reader, report, scoring, storage, terminal,
    timing::{self, CalculationPolicy, LiveCalculationOptions, TimingMethod},
};
use chrono::Timelike;
use std::{
    ffi::OsString,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};
type Result<T> = std::result::Result<T, CoreError>;
fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}
fn io_error(error: io::Error) -> CoreError {
    invalid(io_message(&error))
}
fn load(directory: &Path, encoding: Encoding) -> Result<Archive> {
    let config = directory.join(native::FILE_NAME);
    if config.is_file() {
        native::load(&config)
    } else {
        Archive::load(directory, encoding).map_err(|e| invalid(e.to_string()))
    }
}
fn help() {
    println!(
        "AS2026 — autosuunnistuksen kilpailunhallinta\n\n\
         Käynnistä ilman lisävalintoja, niin saat käyttövalikon.\n\n\
         Komennot:\n\
           as2026 tarkista KANSIO [--utf8]\n\
           as2026 laske KANSIO [--utf8] [--vaiheet] [--raportti TIEDOSTO.html]\n\
           as2026 tulokset KANSIO [--utf8] [--raportti TIEDOSTO.html] [--csv TIEDOSTO.csv] [--xlsx TIEDOSTO.xlsx] [--live PORTTI]\n\
           as2026 pura KEHYSTIEDOSTO [HH:MM:SS]\n\
           as2026 esimerkki\n\
           as2026 lue KANSIO [SARJAPORTTI]\n\
           as2026 lue [SARJAPORTTI] (kertaluvun laitetarkistus)\n\
           as2026 tallenna-kehys KANSIO KEHYSTIEDOSTO HH:MM:SS\n\
           as2026 paivita KANSIO MUUTOKSET.toml\n\
           as2026 uusi KANSIO [KILPAILUN_NIMI]\n\n\
         Tuloslaskenta näyttää korttiaineiston laskennan esikatselun.\n\
         TOML-kilpailu käyttää tallennettuja korjauksia tai kortista laskettuja tuloksia.\n\
         Vanhan kilpailun tulosluettelot perustuvat tallennettuihin pisteisiin.\n\
         Tasatulokset säilytetään jaettuina. --ratkaise-tasatulokset ottaa käyttöön\n\
         asematietoihin perustuvan vertailun, kun tarvittavat tiedot ovat saatavilla.\n\
         Emit-luku ja paivita tallentavat TOML-kilpailun automaattisesti.\n\
         Tarkastus ja esikatselu säilyttävät lähdetiedostot. Lopeta jatkuva luku Ctrl+C:llä."
    );
}
fn inspect(directory: &Path, encoding: Encoding) -> Result<()> {
    let is_native = directory.join(native::FILE_NAME).is_file();
    let a = load(directory, encoding)?;
    println!("\nTarkastus");
    println!(
        "Kilpailun nimi: {}",
        a.basic.identity.get(1).map_or("?", String::as_str)
    );
    println!(
        "Luokkia: {}, ratoja: {}, kilpailijoita: {}",
        a.basic.classes.len(),
        a.routes.len(),
        a.competitors.len()
    );
    println!(
        "Tallennettuja asematuloksia: {}",
        a.competitors.iter().map(|c| c.results.len()).sum::<usize>()
    );
    let p = a.scoring_parameters().map_err(|e| invalid(e.to_string()))?;
    println!(
        "Parametrit: vapaa-aika {} s, ennakko {} p/s, myöhä {} p/s",
        p.tolerance_seconds, p.early_points_per_second, p.late_points_per_second
    );
    for r in &a.routes {
        println!("Rata {} {}: {} asemaa", r.index, r.name, r.stations.len());
    }
    for c in &a.competitors {
        if is_native {
            println!(
                "#{} {} | Luokka {} | Emit {}. Tulos muodostetaan tuloslaskennassa.",
                c.number, c.driver, c.class_code, c.emit_card
            );
        } else {
            println!(
                "#{} {}: tallennettu {} p, tila {}, {} riviä",
                c.number,
                c.driver,
                c.total_penalties,
                c.status,
                c.results.len()
            );
        }
    }
    for warning in &a.warnings {
        println!("Huomio: {warning}");
    }
    println!("Tämä on tallennettujen tietojen tarkastus. Kokonaistuloksia ei laskettu uudelleen.");
    Ok(())
}
fn decode_file(path: &Path, readout_clock: Option<ClockTime>) -> Result<()> {
    let metadata = std::fs::metadata(path).map_err(io_error)?;
    if !metadata.is_file() || metadata.len() != emit::FRAME_LENGTH as u64 {
        return Err(invalid("Odotettiin tavallista 217 tavun kehystiedostoa"));
    }
    let frame = emit::parse_frame(&std::fs::read(path).map_err(io_error)?)?;
    println!(
        "Emit-kortti: {}. Kehys {} tavua; 50 tietuetta.",
        frame.card_number,
        frame.raw.len()
    );
    if let Some(k) = frame.records.iter().position(|p| p.code == 250) {
        println!(
            "Ensimmäinen lukumerkki: tietue {k}. Sisäleimauksia: {}",
            k.saturating_sub(1)
        );
    } else {
        println!("Lukumerkki 250 puuttuu; lukua ei hyväksytä valmiiksi.");
    }
    if let Some(value) = readout_clock {
        let read = frame.accepted_readout(value)?;
        println!(
            "Maalikello sekunteina: {} (ei piilotettua vuorokausikorjausta)",
            read.finish_clock_seconds
        );
        for p in read.interior_punches {
            println!("  koodi {}, nollauksesta {} s", p.code, p.elapsed_seconds);
        }
    }
    Ok(())
}
fn create(directory: &Path, name: &str) -> Result<()> {
    native::create(directory, name)?;
    println!(
        "Kilpailu luotu: {}",
        directory.join(native::FILE_NAME).display()
    );
    println!(
        "Avaa kilpailu valikon valinnalla 1. Lisää tiedot paivita-komennolla tai tekstieditorissa."
    );
    Ok(())
}
fn read_card(port: Option<&str>) -> Result<emit::EmitFrame> {
    let port = reader::port_name(port).map_err(invalid)?;
    println!("Emit-lukija: {port}");
    println!("Aseta Emit-kortti lukijaan (odotus enintään 30 sekuntia).");
    let frame = reader::read_one(&port).map_err(invalid)?;
    println!(
        "Emit-kortti: {}. Kehys {} tavua; 50 tietuetta.",
        frame.card_number,
        frame.raw.len()
    );
    if let Some(index) = frame.records.iter().position(|p| p.code == 250) {
        println!(
            "Ensimmäinen lukumerkki: tietue {index}. Sisäleimauksia: {}",
            index.saturating_sub(1)
        );
        for (number, punch) in frame.records[1..index].iter().enumerate() {
            println!(
                "  {}: leima {}, nollauksesta {} s",
                number + 1,
                punch.code,
                punch.elapsed_seconds
            );
        }
    }
    Ok(frame)
}

static READING: AtomicBool = AtomicBool::new(false);
static STOP_READING: AtomicBool = AtomicBool::new(false);
static SIGNAL_HANDLER: OnceLock<std::result::Result<(), String>> = OnceLock::new();

fn save_frame(directory: &Path, frame: &emit::EmitFrame, clock: ClockTime) -> Result<()> {
    // Avaaminen joka tapahtumalla ottaa mukaan myös toisessa paikallisessa näkymässä tehdyt muutokset.
    let mut store = storage::ProjectStore::open(&directory.join(native::FILE_NAME))?;
    let outcome = store.update(|project| project.record_frame(frame, clock))?;
    println!(
        "Tallennettu: Emit {} | {} | kilpailija {}",
        outcome.card_number,
        if outcome.content_changed {
            "uudet leimaukset"
        } else {
            "samat leimaukset, uusi lukutapahtuma"
        },
        outcome.competitor_number.map_or_else(
            || "ei vielä ilmoittautunutta kilpailijaa".into(),
            |n| n.to_string()
        ),
    );
    if let Some(number) = outcome.competitor_number {
        let archive = store.project().to_archive()?;
        let report = application::calculate(&archive)?;
        if let Some(entry) = report.competitors.iter().find(|c| c.number == number) {
            match &entry.calculation {
                Ok(result) => print_calculation(result, false),
                Err(error) => println!("Lukema säilytetty; laskenta ei onnistunut: {error}"),
            }
        }
    }
    Ok(())
}

fn read_continuously(directory: &Path, requested: Option<&str>) -> Result<()> {
    storage::ProjectStore::open(&directory.join(native::FILE_NAME))?;
    let port = reader::port_name(requested).map_err(invalid)?;
    let mut reader = reader::CardReader::open(&port).map_err(invalid)?;
    SIGNAL_HANDLER
        .get_or_init(|| {
            ctrlc::set_handler(|| {
                if READING.load(Ordering::SeqCst) {
                    STOP_READING.store(true, Ordering::SeqCst);
                } else {
                    std::process::exit(130);
                }
            })
            .map_err(|_| "Keskeytyskäsittelijän asettaminen epäonnistui".to_owned())
        })
        .as_ref()
        .map_err(|e| invalid(e.clone()))?;
    STOP_READING.store(false, Ordering::SeqCst);
    READING.store(true, Ordering::SeqCst);
    println!("Emit-lukija: {port} (9600 8N2). Automaattinen tallennus käytössä.");
    println!(
        "Aseta kortti lukijaan. Seuraavaa korttia odotetaan samassa portissa. Lopeta Ctrl+C:llä."
    );
    let result = (|| {
        while !STOP_READING.load(Ordering::SeqCst) {
            if let Some(frame) = reader.poll().map_err(invalid)? {
                let now = chrono::Local::now();
                save_frame(
                    directory,
                    &frame,
                    ClockTime::new(now.hour(), now.minute(), now.second())?,
                )?;
                println!("Odotetaan seuraavaa korttia.");
            }
        }
        Ok(())
    })();
    READING.store(false, Ordering::SeqCst);
    result
}

fn read_card_menu<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    port: Option<&str>,
) -> Result<()> {
    let Some(value) = prompt(
        input,
        output,
        "Kilpailukansion polku automaattista tallennusta varten (tyhjä = takaisin): ",
    )?
    else {
        return Ok(());
    };
    let directory = terminal::clean_path(&value);
    if directory.as_os_str().is_empty() {
        return Ok(());
    }
    let Some(value) = prompt(
        input,
        output,
        "Sarjaportti (tyhjä = automaattinen tunnistus): ",
    )?
    else {
        return Ok(());
    };
    let selected = if value.trim().is_empty() {
        port
    } else {
        Some(value.trim())
    };
    read_continuously(&directory, selected)
}

fn update_project(directory: &Path, change_path: &Path) -> Result<()> {
    let text = std::fs::read_to_string(change_path).map_err(io_error)?;
    let changes: as2026::changes::Changes = toml::from_str(&text)
        .map_err(|_| invalid("Muutostiedoston TOML-rakenne tai kenttä on virheellinen"))?;
    let mut store = storage::ProjectStore::open(&directory.join(native::FILE_NAME))?;
    store.update(|project| changes.apply(project))?;
    println!("Kilpailun muutokset tallennettu. Laskenta käyttää päivitettyjä tietoja.");
    Ok(())
}

fn print_calculation(result: &timing::LiveCalculation, show_trace: bool) {
    println!(
        "Aikavirhepisteet: {} | Muut virhepisteet: {} | Virhepisteet yhteensä: {} (asemarivit)",
        result.totals.time_points, result.totals.other_points, result.totals.total_points
    );
    println!(
        "Tarkastustulos: {} {}",
        result.status.as_legacy(),
        result.reason
    );
    println!("Asema | Koodi | Tavoite s | Aika s | Aikavirhe p | Muut p | Yhteensä p");
    for row in &result.rows {
        let name = row
            .source
            .station
            .as_ref()
            .map_or("at liikaa", |s| s.label.as_str());
        println!(
            "{} | {} | {} | {} | {} | {} | {} {}",
            name,
            row.received_code,
            row.target_seconds,
            row.actual_seconds,
            row.signed_time_points,
            row.other_points,
            row.total_points,
            row.note
        );
    }
    for d in &result.diagnostics {
        if let Some(message) = report::user_notice(&d.message) {
            match d.row {
                Some(row) => println!("Huomio, rivi {row}: {message}"),
                None => println!("Huomio: {message}"),
            }
        }
    }
    if show_trace {
        println!("Laskennan vaiheet:");
        for event in &result.trace {
            match event.row {
                Some(row) => println!("  Rivi {row}: {}", event.detail),
                None => println!("  {}", event.detail),
            }
        }
    }
}
fn calculate(directory: &Path, encoding: Encoding, show_trace: bool) -> Result<String> {
    let is_native = directory.join(native::FILE_NAME).is_file();
    if is_native {
        let project = storage::ProjectStore::open(&directory.join(native::FILE_NAME))?;
        let archive = application::calculated_project(project.project())?;
        println!("\nTuloslaskenta — {}", project.project().kilpailu.nimi);
        for competitor in &archive.competitors {
            let snapshot = project
                .project()
                .kilpailijat
                .iter()
                .find(|d| d.numero == competitor.number)
                .and_then(|d| d.tallennettu_tulos.clone())
                .or_else(|| {
                    application::capture_project_result(project.project(), competitor.number).ok()
                });
            let complete = snapshot.as_ref().is_some_and(|r| r.can_approve());
            println!(
                "\nNro. {} | {} | Luokka {} | Tila {} | Syy {}",
                competitor.number,
                competitor.driver,
                competitor.class_code,
                competitor.status,
                competitor.header_fields.get(29).map_or("", String::as_str)
            );
            if archive.has_confirmed_interruption(competitor.number) {
                println!("Vahvistettu keskeytys. Maaliaikaa tai pisteitä ei ole vahvistettu.");
            } else if complete && archive.has_approved_result(competitor.number) {
                println!("Virhepisteet yhteensä: {}", competitor.total_penalties);
            } else if complete {
                println!(
                    "Hyväksymätön esikatselu, virhepisteet: {}",
                    competitor.total_penalties
                );
            } else {
                println!("Tulos odottaa käsikellojen täydentämistä tai tarkastusta.");
            }
            println!(
                "Asema | Koodi odotettu/luettu | Kello | Tavoite s | Aika s | Aikavirhe p | Muut p | Yhteensä p"
            );
            for row in &competitor.results {
                let pending = snapshot
                    .as_ref()
                    .is_some_and(|r| r.pending_manual_clocks().contains(&row.output_index));
                let clock = if pending {
                    "—".into()
                } else {
                    format!(
                        "{:02}:{:02}:{:02}",
                        row.clock[0], row.clock[1], row.clock[2]
                    )
                };
                println!(
                    "{} {} | {}/{} | {} | {} | {} | {} | {} | {}",
                    row.station_type,
                    row.name,
                    row.expected_code,
                    row.received_code,
                    clock,
                    row.target_seconds,
                    row.actual_seconds,
                    row.signed_time_penalty,
                    row.other_penalty,
                    row.total_penalty
                );
            }
        }
        for notice in archive
            .warnings
            .iter()
            .filter_map(|m| report::user_notice(m))
        {
            println!("Huomio: {notice}");
        }
        if show_trace {
            println!(
                "Vaiheet perustuvat samoihin tallennettuihin asemariveihin kuin käyttöliittymässä; tiedostoja ei muutettu."
            );
        }
        return report::project_results_html(project.project());
    }
    let a = load(directory, encoding)?;
    let result = application::calculate(&a)?;
    println!("\nTuloslaskennan esikatselu — {}", result.competition_name);
    for warning in result
        .warnings
        .iter()
        .filter_map(|m| report::user_notice(m))
    {
        println!("Huomio: {warning}");
    }
    for entry in &result.competitors {
        if is_native {
            println!(
                "\nNro. {} | {} | Luokka {}",
                entry.number, entry.driver, entry.class_code
            );
        } else {
            println!(
                "\nNro. {} | {} | Luokka {} | Tallennettu tulos {} p",
                entry.number, entry.driver, entry.class_code, entry.imported_total
            );
        }
        match &entry.calculation {
            Ok(calculation) => print_calculation(calculation, show_trace),
            Err(error) => println!("Laskentaa ei tehty: {error}"),
        }
        for note in entry.notes.iter().filter_map(|m| report::user_notice(m)) {
            println!("Huomio: {note}");
        }
    }
    let count = result
        .competitors
        .iter()
        .filter(|c| c.calculation.is_ok())
        .count();
    println!(
        "\nLaskettu {count}/{} kilpailijaa. Tallennetut tulokset säilyvät ennallaan.",
        result.competitors.len()
    );
    Ok(if is_native {
        report::project_calculation_html(&result)
    } else {
        report::calculation_html(&result)
    })
}
fn rank(directory: &Path, encoding: Encoding, break_ties: bool) -> Result<String> {
    let native_path = directory.join(native::FILE_NAME);
    let is_native = native_path.is_file();
    let a = if is_native {
        application::calculated_project(storage::ProjectStore::open(&native_path)?.project())?
    } else {
        load(directory, encoding)?
    };
    let policy = if break_ties {
        ranking::TiePolicy::BestRowsWithExplicitCounts
    } else {
        ranking::TiePolicy::KeepSharedAndFlag
    };
    let result = application::rank_stored(&a, policy)?;
    println!(
        "\nLuokkakohtaiset tulokset — {}",
        if is_native {
            "ajantasainen asemalaskenta"
        } else {
            "tallennetut pisteet"
        }
    );
    for group in &result.groups {
        println!(
            "\nLuokka {} | Lähteneitä {}{}",
            group.code,
            group.statistics.starter_count(),
            if group.provisional {
                " | Keskeneräinen: pisteitä ei vahvistettu"
            } else {
                ""
            }
        );
        println!("Sij. | Nro. | Kuljettaja | Kartanlukija | Tulos | Tila | Np | Sp");
        for entry in &group.entries {
            let c = &a.competitors[entry.source_index - 1];
            let position = if entry.primary_position < 25_000 {
                entry.primary_position.to_string()
            } else {
                "—".to_owned()
            };
            let np = if a.has_approved_result(c.number) && group.np_enabled && !group.provisional {
                entry.table_points.to_string()
            } else {
                "—".to_owned()
            };
            let sp = if a.has_approved_result(c.number)
                && group.series_method != 0
                && !group.provisional
            {
                entry.series_points.to_string()
            } else {
                "—".to_owned()
            };
            println!(
                "{position} | {} | {} | {} | {} | {} | {np} | {sp}",
                c.number,
                c.driver,
                c.navigator,
                if a.has_approved_result(c.number) {
                    c.total_penalties.to_string()
                } else {
                    "—".into()
                },
                entry.status.as_legacy()
            );
        }
    }
    println!("\nRatakohtaiset tulokset");
    for route in &result.routes {
        println!("Rata {}", route.code);
        for entry in &route.entries {
            let c = &a.competitors[entry.source_index - 1];
            let position = if entry.primary_position < 25_000 {
                entry.primary_position.to_string()
            } else {
                "—".to_owned()
            };
            println!(
                "  {position} | Nro. {} | {} | {} p | {}",
                c.number,
                c.driver,
                if a.has_approved_result(c.number) {
                    c.total_penalties.to_string()
                } else {
                    "—".into()
                },
                entry.status.as_legacy()
            );
        }
    }
    for warning in &a.warnings {
        println!("Huomio: {warning}");
    }
    for d in &result.diagnostics {
        println!("Huomio: {d}");
    }
    if !result.omitted_source_indices.is_empty() {
        println!(
            "Tulosluettelon ulkopuolelle jäi {} kilpailijaa tilan vuoksi.",
            result.omitted_source_indices.len()
        );
    }
    Ok(if is_native {
        report::competition_ranking_html(
            storage::ProjectStore::open(&native_path)?.project(),
            policy,
        )?
    } else {
        report::ranking_html(&a, &result)
    })
}
/// CSV takes exactly the same ranking inputs and tie policy as the HTML path.
fn rank_csv(directory: &Path, encoding: Encoding, break_ties: bool) -> Result<String> {
    let native_path = directory.join(native::FILE_NAME);
    let archive = if native_path.is_file() {
        application::calculated_project(storage::ProjectStore::open(&native_path)?.project())?
    } else {
        load(directory, encoding)?
    };
    let policy = if break_ties {
        ranking::TiePolicy::BestRowsWithExplicitCounts
    } else {
        ranking::TiePolicy::KeepSharedAndFlag
    };
    let ranked = application::rank_stored(&archive, policy)?;
    if native_path.is_file() {
        return report::competition_ranking_csv(
            storage::ProjectStore::open(&native_path)?.project(),
            policy,
        );
    }
    Ok(report::ranking_csv(&archive, &ranked))
}

/// Render without terminal output for the localhost live report endpoint.
fn rank_html_live(directory: &Path, encoding: Encoding, break_ties: bool) -> Result<String> {
    let native_path = directory.join(native::FILE_NAME);
    let native = native_path.is_file();
    let archive = if native {
        application::calculated_project(storage::ProjectStore::open(&native_path)?.project())?
    } else {
        load(directory, encoding)?
    };
    let policy = if break_ties {
        ranking::TiePolicy::BestRowsWithExplicitCounts
    } else {
        ranking::TiePolicy::KeepSharedAndFlag
    };
    let ranked = application::rank_stored(&archive, policy)?;
    Ok(if native {
        report::competition_ranking_html(
            storage::ProjectStore::open(&native_path)?.project(),
            policy,
        )?
    } else {
        report::ranking_html(&archive, &ranked)
    })
}

fn demo() -> Result<()> {
    println!("SYNTEETTINEN ESIMERKKI — ei alkuperäisohjelman tai lukijalaitteen tuottama tulos.");
    let document = legacy::parse_document(
        include_bytes!("../fixtures/synthetic/Radat.txt"),
        Encoding::Windows1252,
    )
    .map_err(|e| invalid(e.to_string()))?;
    let routes = legacy::parse_routes(&document).map_err(|e| invalid(e.to_string()))?;
    let route = routes
        .first()
        .ok_or_else(|| invalid("Esimerkkikilpailun rata puuttuu"))?
        .to_core()
        .map_err(|e| invalid(e.to_string()))?;
    let frame = emit::parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw"))?;
    let read = frame.accepted_readout(ClockTime::new(12, 11, 0)?)?;
    let official_start = ClockTime::new(12, 0, 0)?;
    let rows = matching::match_route(&route, &read.interior_punches, official_start)?;
    println!(
        "Kortti {}, maalikaavan tulos {} s, kohdistusrivejä {}",
        frame.card_number,
        read.finish_clock_seconds,
        rows.len()
    );
    for row in &rows {
        let kind = match row.kind {
            MatchKind::Matched => "kohdistettu",
            MatchKind::Missing => "puuttuva",
            MatchKind::Extra => "ylimääräinen",
            MatchKind::RouteOnly => "reittirivi",
            MatchKind::SharedUla => "jaettu ULA",
            MatchKind::ManualStart => "käsilähtö",
            MatchKind::ManualFinish => "käsimaali",
        };
        let code = row
            .punch
            .map_or_else(|| "—".to_owned(), |p| p.code.to_string());
        println!("  {}: {kind}, leima {code}", row.output_index);
    }
    let options = LiveCalculationOptions {
        official_start,
        reset_correction_seconds: 120,
        timing_method: TimingMethod::Emit,
        last_punch_elapsed_seconds: read.last_punch_elapsed_seconds,
        cumulative_limit_minutes: Some(60),
        parameters: scoring::ScoringParameters::default(),
        policy: CalculationPolicy::RecoveredAs2015aStaticV2,
    };
    print_calculation(&timing::calculate_live(&rows, &options)?, true);
    Ok(())
}
fn save(path: &Path, html: &str) -> Result<()> {
    report::save_new(path, html)?;
    println!("Raportti tallennettu: {}", path.display());
    Ok(())
}
fn prompt<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    text: &str,
) -> Result<Option<String>> {
    write!(output, "{text}").map_err(io_error)?;
    output.flush().map_err(io_error)?;
    let mut value = String::new();
    if input.read_line(&mut value).map_err(io_error)? == 0 {
        return Ok(None);
    }
    Ok(Some(value.trim_end_matches(['\r', '\n']).to_owned()))
}
fn offer_report<R: BufRead, W: Write>(input: &mut R, output: &mut W, html: &str) -> Result<()> {
    loop {
        let Some(value) = prompt(
            input,
            output,
            "\nTallenna raportti selaimessa avattavaksi? [k/e, oletus e]: ",
        )?
        else {
            return Ok(());
        };
        if value.trim().is_empty() || terminal::parse_yes_no(&value) == Some(false) {
            return Ok(());
        }
        if terminal::parse_yes_no(&value) == Some(true) {
            break;
        }
        writeln!(output, "Vastaa k (kyllä) tai e (ei).").map_err(io_error)?;
    }
    if let Some(value) = prompt(
        input,
        output,
        "Uuden raporttitiedoston polku (esim. tulokset.html; tyhjä = peruuta): ",
    )? {
        let path = terminal::clean_path(&value);
        if !path.as_os_str().is_empty() {
            save(&path, html)?;
        }
    }
    Ok(())
}
fn menu() -> Result<()> {
    let keyboard = io::stdin().is_terminal();
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let action = terminal::menu(&mut input, &mut output).map_err(io_error)?;
        let result = match action {
            terminal::Action::Quit => return Ok(()),
            terminal::Action::Inspect {
                directory,
                encoding,
            } => inspect(&directory, encoding).map(|()| None),
            terminal::Action::Calculate {
                directory,
                encoding,
            } => calculate(&directory, encoding, false).map(Some),
            terminal::Action::RankStored {
                directory,
                encoding,
            } => rank(&directory, encoding, false).map(Some),
            terminal::Action::Decode {
                path,
                readout_clock,
            } => decode_file(&path, readout_clock).map(|()| None),
            terminal::Action::Demo => demo().map(|()| None),
            terminal::Action::Create { directory, name } => {
                create(&directory, &name).map(|()| None)
            }
            terminal::Action::ReadLive { port } => {
                read_card_menu(&mut input, &mut output, port.as_deref()).map(|()| None)
            }
        };
        match result {
            Ok(Some(html)) => {
                if let Err(error) = offer_report(&mut input, &mut output, &html) {
                    writeln!(output, "Virhe: {error}").map_err(io_error)?;
                }
            }
            Ok(None) => {}
            Err(error) => writeln!(output, "Virhe: {error}").map_err(io_error)?,
        }
        if keyboard
            && prompt(
                &mut input,
                &mut output,
                "\nPalaa valikkoon painamalla Enter: ",
            )?
            .is_none()
        {
            return Ok(());
        }
    }
}
#[derive(Default)]
struct Options {
    utf8: bool,
    trace: bool,
    break_ties: bool,
    html: Option<PathBuf>,
    csv: Option<PathBuf>,
    xlsx: Option<PathBuf>,
    live: Option<u16>,
}
fn options(args: &[OsString], calculation: bool) -> Result<Options> {
    let mut result = Options::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].to_str() {
            Some("--utf8") if !result.utf8 => result.utf8 = true,
            Some("--vaiheet" | "--trace") if calculation && !result.trace => result.trace = true,
            Some("--ratkaise-tasatulokset" | "--break-ties")
                if !calculation && !result.break_ties =>
            {
                result.break_ties = true
            }
            Some("--xlsx") if !calculation && result.xlsx.is_none() => {
                i += 1;
                result.xlsx = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| invalid("XLSX-tiedoston polku puuttuu"))?,
                ));
            }
            Some("--live") if !calculation && result.live.is_none() => {
                i += 1;
                result.live = Some(
                    args.get(i)
                        .and_then(|value| value.to_str())
                        .ok_or_else(|| invalid("Live-HTML-portti puuttuu"))?
                        .parse::<u16>()
                        .map_err(|_| invalid("Live-HTML-portti ei kelpaa"))?,
                );
            }
            Some("--csv") if !calculation && result.csv.is_none() => {
                i += 1;
                result.csv = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| invalid("CSV-tiedoston polku puuttuu"))?,
                ));
            }
            Some("--raportti" | "--html") if result.html.is_none() => {
                i += 1;
                result.html = Some(PathBuf::from(
                    args.get(i)
                        .ok_or_else(|| invalid("Raporttitiedoston polku puuttuu"))?,
                ));
            }
            _ => return Err(invalid("Tuntematon tai toistuva lisävalinta")),
        }
        i += 1;
    }
    Ok(result)
}
fn run() -> Result<()> {
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    match args.first().and_then(|s| s.to_str()) {
        None if args.is_empty() => menu(),
        Some("ohje" | "help" | "--help" | "-h") if args.len() == 1 => {
            help();
            Ok(())
        }
        Some("esimerkki" | "demo") if args.len() == 1 => demo(),
        Some("uusi") if (2..=3).contains(&args.len()) => {
            let name = args
                .get(2)
                .map(|value| value.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Uusi kilpailu".to_owned());
            create(Path::new(&args[1]), &name)
        }
        Some("paivita") if args.len() == 3 => {
            update_project(Path::new(&args[1]), Path::new(&args[2]))
        }
        Some("tallenna-kehys") if args.len() == 4 => {
            let bytes = std::fs::read(Path::new(&args[2])).map_err(io_error)?;
            let frame = emit::parse_frame(&bytes)?;
            let clock = terminal::parse_clock(args[3].to_str().unwrap_or(""))?;
            save_frame(Path::new(&args[1]), &frame, clock)
        }
        Some("lue" | "read")
            if args.len() == 3 || (args.len() == 2 && Path::new(&args[1]).is_dir()) =>
        {
            read_continuously(Path::new(&args[1]), args.get(2).and_then(|s| s.to_str()))
        }
        Some("lue" | "read") if (1..=2).contains(&args.len()) => {
            read_card(args.get(1).and_then(|s| s.to_str())).map(|_| ())
        }
        Some("tarkista" | "inspect")
            if args.len() == 2 || (args.len() == 3 && args[2] == "--utf8") =>
        {
            inspect(
                Path::new(&args[1]),
                if args.len() == 3 {
                    Encoding::Utf8
                } else {
                    Encoding::Windows1252
                },
            )
        }
        Some("laske" | "calculate" | "tulokset" | "rank") if args.len() >= 2 => {
            let calculation = matches!(args[0].to_str(), Some("laske" | "calculate"));
            let options = options(&args[2..], calculation)?;
            let encoding = if options.utf8 {
                Encoding::Utf8
            } else {
                Encoding::Windows1252
            };
            let html = if calculation {
                calculate(Path::new(&args[1]), encoding, options.trace)?
            } else {
                rank(Path::new(&args[1]), encoding, options.break_ties)?
            };
            if let Some(path) = options.html {
                save(&path, &html)?;
            }
            if let Some(path) = options.csv {
                let csv = rank_csv(Path::new(&args[1]), encoding, options.break_ties)?;
                save(&path, &csv)?;
            }
            if let Some(path) = options.xlsx {
                let csv = rank_csv(Path::new(&args[1]), encoding, options.break_ties)?;
                let bytes = report::xlsx_from_csv(&csv)?;
                report::save_new_bytes(&path, &bytes)?;
                println!("XLSX tallennettu: {}", path.display());
            }
            if let Some(port) = options.live {
                let directory = PathBuf::from(&args[1]);
                live_report::serve(port, move || {
                    rank_html_live(&directory, encoding, options.break_ties)
                })?;
            }
            Ok(())
        }
        Some("pura" | "decode") if (2..=3).contains(&args.len()) => {
            let clock = args
                .get(2)
                .map(|s| terminal::parse_clock(s.to_str().unwrap_or("")))
                .transpose()?;
            decode_file(Path::new(&args[1]), clock)
        }
        _ if args.len() == 1 && Path::new(&args[0]).is_dir() => {
            inspect(Path::new(&args[0]), Encoding::Windows1252)
        }
        _ => {
            help();
            Err(invalid("Tuntematon komento tai virheelliset lisävalinnat"))
        }
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("Virhe: {error}");
        std::process::exit(2);
    }
}
