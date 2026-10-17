//! Korttiaineiston laskentapolku, joka perustuu staattisesti palautettuun logiikkaan.
//!
//! `RecoveredLiveV1` säilyttää aikaisemman Rust-esikatselun oletukset.
//! `RecoveredAs2015aStaticV2` perustuu täydennettyyn Ghidra-käskytutkimukseen.
//! Kumpaakaan ei ole validoitu suorittamalla alkuperäistä ohjelmaa. Raakaleimaukset säilyvät
//! `CalculatedRow::source`-kentässä, ja johdetut ajat ovat erillisissä kentissä.
//! Vanhan ohjelman ristiriitaiset haarat ilmoitetaan diagnostiikassa. Ajanottotapa
//! 2 ja tallennettujen tulosten uudelleenlaskenta eivät kuulu tähän rajapintaan.
//!
//! Vanhan ohjelman asemahuomiot ja tilasyyt on kopioitu sen merkkijonoista:
//! lähtö 0x004d44eb, AT 0x004d458b, RT 0x004d457f, JAT 0x004d4597 /
//! 0x004d45a8, maali 0x004d45b5 / 0x004d45c8, ylimääräinen AT 0x004d45d8,
//! pistekatto 0x004d45e2 ja myöhästymisrajat 0x004d45f1 / 0x004d456a.
//! Vain merkkijonojen täytevälilyönnit on poistettu. Uudet laskennan rajauksia
//! selittävät diagnostiikkaviestit ja laskentajäljen selitykset on kirjoitettu
//! suomeksi, koska niille ei ole vastinetta alkuperäisessä ohjelmassa.

use crate::scoring::{
    PenaltyTotals, ScoringParameters, cumulative_lateness_exceeded, extra_punch_penalty,
    finish_section_limit_exceeded, round_ula_elapsed, row_penalty_total, sum_row_penalties,
    time_penalty,
};
use crate::{ClockTime, CoreError, MatchKind, MatchedRow, Rule, StationKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculationPolicy {
    /// Aikaisemman Rust-prototyypin eksplisiittinen laskentakäytäntö.
    RecoveredLiveV1,
    /// AS2015A:n live-korttipolusta staattisesti palautettu käytäntö.
    RecoveredAs2015aStaticV2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TimingMethod {
    ManualClock = 0,
    Emit = 1,
    LegacyMode2 = 2,
}

impl TimingMethod {
    pub fn from_legacy(value: i64) -> Result<Self, CoreError> {
        match value {
            0 => Ok(Self::ManualClock),
            1 => Ok(Self::Emit),
            2 => Ok(Self::LegacyMode2),
            _ => Err(CoreError::Unsupported(format!("ajanottotapa {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCalculationOptions {
    pub official_start: ClockTime,
    pub reset_correction_seconds: i64,
    pub timing_method: TimingMethod,
    /// Viimeinen leima ennen 250-lukumerkkiä, ei lukumerkin oma aika.
    pub last_punch_elapsed_seconds: u32,
    /// Luokkakohtainen raja; None käyttää yleistä jakson myöhästymisrajaa.
    pub cumulative_limit_minutes: Option<i64>,
    pub parameters: ScoringParameters,
    pub policy: CalculationPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculationStatus {
    Checked,
    Interrupted,
}

impl CalculationStatus {
    pub const fn as_legacy(self) -> &'static str {
        match self {
            Self::Checked => "TRK",
            Self::Interrupted => "KESK",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculationDiagnostic {
    pub code: &'static str,
    pub row: Option<usize>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub row: Option<usize>,
    pub action: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatedRow {
    /// Muuttumaton kohdistusrivi, mukaan lukien alkuperäinen Emit-leima.
    pub source: MatchedRow,
    pub received_code: u32,
    pub effective_rule: Rule,
    pub derived_elapsed_seconds: i64,
    pub clock_seconds: Option<i64>,
    pub target_seconds: i64,
    pub actual_seconds: i64,
    pub signed_time_points: i64,
    pub other_points: i64,
    pub total_points: i64,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveCalculation {
    pub policy: CalculationPolicy,
    pub rows: Vec<CalculatedRow>,
    pub totals: PenaltyTotals,
    pub status: CalculationStatus,
    pub reason: String,
    /// Sekunnit virallisen lähtöpäivän keskiyöstä; vuorokautta ei piiloteta.
    pub finish_clock_seconds: i64,
    pub total_duration_seconds: i64,
    pub cumulative_schedule_difference_seconds: i64,
    pub diagnostics: Vec<CalculationDiagnostic>,
    pub trace: Vec<TraceEvent>,
}

fn add(a: i64, b: i64, what: &'static str) -> Result<i64, CoreError> {
    a.checked_add(b).ok_or(CoreError::ArithmeticOverflow(what))
}
fn sub(a: i64, b: i64, what: &'static str) -> Result<i64, CoreError> {
    a.checked_sub(b).ok_or(CoreError::ArithmeticOverflow(what))
}
/// Tallennettujen P-rivien tarkistuksen erillinen ajanvalmistelu
/// (AS2015A 0x0041ef28). Tämä EI vielä laske asemien pisteitä uudelleen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredTimingInput {
    pub official_start: ClockTime,
    /// Tallennettu maalikello sekunteina vuorokauden alusta.
    pub stored_finish_clock_seconds: i64,
    /// Tallennetun viimeisen P-rivin alkuperäinen aikakoordinaatti.
    pub last_row_elapsed_seconds: i64,
    pub finish_expected_code: u32,
    pub reset_correction_seconds: i64,
    pub timing_method: TimingMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedStoredTiming {
    pub start_elapsed_seconds: i64,
    pub last_row_elapsed_seconds: i64,
    pub effective_reset_correction_seconds: i64,
    pub finish_clock_seconds: i64,
    pub total_duration_seconds: i64,
}

/// Staattisesti palautettu valmistelujärjestys. Alustamattomaan pinoarvoon
/// riippuvia tuloksia ei arvata (ajanottotapa 1/2 ja käsimaali M0).
pub fn prepare_stored_timing(input: &StoredTimingInput) -> Result<PreparedStoredTiming, CoreError> {
    let start = input.official_start.seconds_since_midnight()?;
    if start == 0 {
        return Err(CoreError::Unsupported(
            "Tallennettujen tulosten lähtöaika 00:00:00 palauttaa AS2015A:ssa ennen ajanottotavan valmistelua".into(),
        ));
    }
    let mut finish = input.stored_finish_clock_seconds;
    let mut correction = input.reset_correction_seconds;
    let mut last_elapsed = input.last_row_elapsed_seconds;
    let start_elapsed = match input.timing_method {
        TimingMethod::ManualClock => {
            if finish == 0 {
                return Err(CoreError::Unsupported(
                    "Tallennetun ajanottotavan 0 maalikello 00:00:00 ei kelpaa AS2015A:n tarkistuspolulle".into(),
                ));
            }
            if finish < start {
                finish = add(finish, 86_400, "tallennetun maalikellon vuorokausikorjaus")?;
            }
            last_elapsed = sub(finish, start, "tallennetun maalin koordinaatti")?;
            0
        }
        TimingMethod::Emit | TimingMethod::LegacyMode2 => {
            if input.finish_expected_code == 0 {
                return Err(CoreError::Unsupported(
                    "Tallennettujen tulosten ajanottotavoilla 1 ja 2 käsimaali käyttää AS2015A:ssa alustamatonta maalikelloa; tulosta ei arvata".into(),
                ));
            }
            if input.timing_method == TimingMethod::Emit {
                let anchor = if correction == 0 { 120 } else { correction };
                finish = add(
                    start,
                    sub(last_elapsed, anchor, "tallennetun maalin aikaviite")?,
                    "tallennettujen tulosten maalikello",
                )?;
                anchor
            } else {
                correction = sub(finish, start, "ajanottotavan 2 aikakorjaus")?;
                if correction < -20_000 {
                    correction = add(correction, 86_400, "ajanottotavan 2 vuorokausikorjaus")?;
                }
                correction
            }
        }
    };
    let mut duration = sub(finish, start, "tallennettujen tulosten kesto")?;
    if duration < -20_000 {
        duration = add(
            duration,
            86_400,
            "tallennettujen tulosten vuorokausikorjaus",
        )?;
    }
    if duration < 0 {
        return Err(CoreError::Unsupported(
            "Tallennettujen tulosten laskettu kesto on negatiivinen".into(),
        ));
    }
    Ok(PreparedStoredTiming {
        start_elapsed_seconds: start_elapsed,
        last_row_elapsed_seconds: last_elapsed,
        effective_reset_correction_seconds: correction,
        finish_clock_seconds: finish,
        total_duration_seconds: duration,
    })
}

/// Tallennettujen asemarivien ajallinen tieto ennen uusintatarkistusta.
/// Pistelaskentaa, aseman lajia tai korjaussääntöä ei muuteta tässä.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredStationTime {
    pub expected_code: u32,
    pub elapsed_seconds: i64,
    pub clock: [i64; 3],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedStoredStationTimes {
    pub rows: Vec<StoredStationTime>,
    pub warnings: Vec<String>,
}

fn stored_hms(seconds: i64) -> [i64; 3] {
    // AS2015A:n tässä kohdassa tapahtuva kokonaislukujako ei moduloidu
    // vuorokauteen. Negatiiviset arvot säilytetään näkyvinä.
    [seconds / 3600, (seconds % 3600) / 60, seconds % 60]
}

fn stored_clock_seconds(parts: [i64; 3]) -> Result<i64, CoreError> {
    let hours = parts[0]
        .checked_mul(3600)
        .ok_or(CoreError::ArithmeticOverflow(
            "tallennetun käsikellon tunnit",
        ))?;
    let minutes = parts[1]
        .checked_mul(60)
        .ok_or(CoreError::ArithmeticOverflow(
            "tallennetun käsikellon minuutit",
        ))?;
    add(
        add(hours, minutes, "tallennettu käsikello")?,
        parts[2],
        "tallennettu käsikello",
    )
}

/// AS2015A 0x0041ef28: talletettujen P-rivien kellot valmistellaan riveillä
/// n−1 .. 2 käänteisessä järjestyksessä, *ennen* context=1-asemapistelaskentaa.
/// Ei muuta kutsujan rivitaulukkoa eikä arvaa vanhan ohjelman muistivirheitä.
pub fn prepare_stored_station_times(
    prepared: &PreparedStoredTiming,
    source: &[StoredStationTime],
) -> Result<PreparedStoredStationTimes, CoreError> {
    if source.len() < 2 {
        return Err(CoreError::InvalidInput(
            "Tallennettujen tulosten valmistelu vaatii lähtö- ja maalirivin".into(),
        ));
    }
    let mut rows = source.to_vec();
    let last = rows.len() - 1;
    rows[0].elapsed_seconds = prepared.start_elapsed_seconds;
    rows[last].elapsed_seconds = prepared.last_row_elapsed_seconds;
    let mut warnings = Vec::new();
    for index in (1..last).rev() {
        let next_elapsed = rows[index + 1].elapsed_seconds;
        let row = &mut rows[index];
        if row.expected_code != 0 && row.elapsed_seconds != 0 {
            let mut clock = add(
                sub(
                    prepared.finish_clock_seconds,
                    prepared.last_row_elapsed_seconds,
                    "leimatun aseman kelloviite",
                )?,
                row.elapsed_seconds,
                "leimatun aseman kello",
            )?;
            if clock < -50_000 {
                clock = add(clock, 86_400, "leimatun aseman vuorokausikorjaus")?;
            }
            row.clock = stored_hms(clock);
        } else if row.expected_code == 0 {
            // Historiallinen testi on kellokomponenttien summa, ei K != 0.
            let components_sum = row
                .clock
                .iter()
                .try_fold(0_i64, |a, value| a.checked_add(*value))
                .ok_or(CoreError::ArithmeticOverflow("käsikellon komponentit"))?;
            if components_sum == 0 {
                row.elapsed_seconds = 0;
            } else {
                let mut clock = stored_clock_seconds(row.clock)?;
                if sub(
                    clock,
                    prepared.finish_clock_seconds,
                    "käsikellon vuorokausiraja",
                )? < -50_000
                {
                    clock = add(clock, 86_400, "käsikellon vuorokausikorjaus")?;
                }
                row.elapsed_seconds = sub(
                    prepared.last_row_elapsed_seconds,
                    sub(
                        prepared.finish_clock_seconds,
                        clock,
                        "käsikellon ero maalista",
                    )?,
                    "käsikellon aikakoordinaatti",
                )?;
                if row.elapsed_seconds == 0 || row.elapsed_seconds >= next_elapsed {
                    warnings.push(format!(
                        "Tallennetun asemarivin {} käsiaika poikkeaa aikajärjestyksestä; AS2015A vain huomauttaa eikä korjaa sitä",
                        index + 1
                    ));
                }
            }
        }
    }
    Ok(PreparedStoredStationTimes { rows, warnings })
}

fn diagnostic(
    result: &mut LiveCalculation,
    row: Option<usize>,
    code: &'static str,
    message: impl Into<String>,
) {
    result.diagnostics.push(CalculationDiagnostic {
        code,
        row,
        message: message.into(),
    });
}
fn trace(
    result: &mut LiveCalculation,
    row: Option<usize>,
    action: &'static str,
    detail: impl Into<String>,
) {
    result.trace.push(TraceEvent {
        row,
        action,
        detail: detail.into(),
    });
}
fn interrupt(result: &mut LiveCalculation, row: Option<usize>, reason: &str) {
    result.status = CalculationStatus::Interrupted;
    result.reason = reason.to_owned();
    trace(result, row, "status", format!("KESK: {reason}"));
}
fn station_kind(row: &CalculatedRow) -> Option<StationKind> {
    row.source.station.as_ref().map(|s| s.kind)
}
fn expected(row: &CalculatedRow) -> u32 {
    row.source.station.as_ref().map_or(999, |s| s.expected_code)
}
fn present(row: &CalculatedRow) -> bool {
    if row.source.station.is_none() {
        return false;
    }
    if expected(row) != 0 {
        row.received_code == expected(row)
    } else {
        row.clock_seconds.is_some_and(|clock| clock != 0)
    }
}

fn validate_rows(rows: &[MatchedRow]) -> Result<(), CoreError> {
    if rows.len() < 2 {
        return Err(CoreError::InvalidInput(
            "laskenta vaatii lähtö- ja maalirivin".into(),
        ));
    }
    for (index, row) in rows.iter().enumerate() {
        if row.output_index != index + 1 {
            return Err(CoreError::InvalidInput(
                "kohdistusrivien indeksien pitää alkaa yhdestä ja olla peräkkäiset".into(),
            ));
        }
        if let Some(clock) = row.clock {
            clock.seconds_since_midnight()?;
        }
        match (&row.station, row.punch, row.kind) {
            (None, Some(punch), MatchKind::Extra) if punch.code != 0 => {}
            (Some(station), Some(punch), MatchKind::Matched)
                if station.expected_code != 0 && station.expected_code == punch.code => {}
            (Some(station), Some(punch), MatchKind::SharedUla)
                if station.kind == StationKind::Ula
                    && station.expected_code != 0
                    && station.expected_code == punch.code => {}
            (Some(station), None, MatchKind::ManualStart)
                if station.kind == StationKind::Start && station.expected_code == 0 => {}
            (Some(station), None, MatchKind::ManualFinish)
                if station.kind == StationKind::Finish && station.expected_code == 0 => {}
            (Some(_), None, MatchKind::Missing | MatchKind::RouteOnly) => {}
            _ => {
                return Err(CoreError::InvalidInput(format!(
                    "ristiriitainen kohdistusrivi {}",
                    index + 1
                )));
            }
        }
        if let Some(station) = &row.station {
            if station.target_seconds < 0 {
                return Err(CoreError::InvalidInput(format!(
                    "negatiivinen tavoiteaika rivillä {}",
                    index + 1
                )));
            }
            if matches!(
                station.rule,
                Rule::UlaNearestMinute | Rule::UlaFloorMinute | Rule::UlaCeilMinute
            ) && station.kind != StationKind::Ula
            {
                return Err(CoreError::Unsupported(format!(
                    "ULA-pyöristyssääntö muulla asemalajilla rivillä {}",
                    index + 1
                )));
            }
            if (index == 0) != (station.kind == StationKind::Start)
                || (station.kind == StationKind::Finish && index != rows.len() - 1)
            {
                return Err(CoreError::Unsupported("lähtö on oltava ensimmäinen ja maali viimeinen kohdistusrivi; jälkimaalileimaukset vaativat erillisen käytännön".into()));
            }
            if station.expected_code == 0
                && !matches!(station.kind, StationKind::Start | StationKind::Finish)
                && row.clock.is_some_and(|c| c.hour + c.minute + c.second != 0)
            {
                return Err(CoreError::Unsupported("käsin kellotettujen väliasemien muuntaminen kuuluu vielä toteuttamattomaan uudelleenlaskentapolkuun".into()));
            }
        }
    }
    if rows[0].station.as_ref().map(|s| s.kind) != Some(StationKind::Start)
        || rows.last().and_then(|r| r.station.as_ref()).map(|s| s.kind) != Some(StationKind::Finish)
    {
        return Err(CoreError::Unsupported(
            "reitin ensimmäisen rivin pitää olla L ja viimeisen M".into(),
        ));
    }
    Ok(())
}

/// Erillinen turvallinen AS2026-käytäntö maalin jälkeisille leimoille.
/// Maalikello perustuu varsinaiseen maalileimaan, ei viimeiseen ylimääräiseen.
/// Menetelmää ei ole varmennettu suorittamalla AS2015A:ta.
pub fn calculate_live_with_trailing_extras(
    rows: &[MatchedRow],
    options: &LiveCalculationOptions,
) -> Result<LiveCalculation, CoreError> {
    let Some(finish) = rows.iter().position(|row| {
        row.station
            .as_ref()
            .is_some_and(|station| station.kind == StationKind::Finish)
    }) else {
        return calculate_live(rows, options);
    };
    if finish + 1 == rows.len() {
        return calculate_live(rows, options);
    }
    if !rows[finish + 1..].iter().all(|row| {
        row.station.is_none()
            && row.kind == MatchKind::Extra
            && row.punch.is_some_and(|punch| punch.code != 0)
    }) {
        return Err(CoreError::InvalidInput(
            "Maalin jälkeiset asemarivit eivät ole pelkkiä ylimääräisiä leimoja".into(),
        ));
    }
    let finish_punch = rows[finish].punch.ok_or_else(|| {
        CoreError::Unsupported(
            "Käsimaalin jälkeisten leimojen ajanottoa ei ole vahvistettu".into(),
        )
    })?;
    if rows[finish]
        .station
        .as_ref()
        .is_none_or(|station| station.expected_code == 0)
    {
        return Err(CoreError::Unsupported(
            "Käsimaalin jälkeisiä ylimääräisiä leimoja ei pisteytetä arvaamalla".into(),
        ));
    }
    let mut finish_options = options.clone();
    finish_options.last_punch_elapsed_seconds = finish_punch.elapsed_seconds;
    let mut result = calculate_live(&rows[..=finish], &finish_options)?;
    for source in &rows[finish + 1..] {
        let punch = source.punch.expect("jälkileima tarkistettu");
        let previous = result.rows.last().map(|row| row.received_code);
        let two_back = result.rows.iter().rev().nth(1).map(|row| row.received_code);
        let penalty = extra_punch_penalty(
            punch.code,
            previous,
            two_back,
            &options.parameters,
        )?;
        result.rows.push(CalculatedRow {
            source: source.clone(),
            received_code: punch.code,
            effective_rule: Rule::Normal,
            derived_elapsed_seconds: i64::from(punch.elapsed_seconds),
            clock_seconds: None,
            target_seconds: 0,
            actual_seconds: 0,
            signed_time_points: 0,
            other_points: penalty,
            total_points: penalty,
            note: if matches!(punch.code, 99 | 999) {
                String::new()
            } else {
                "at liikaa".into()
            },
        });
    }
    result.totals = sum_row_penalties(
        &result
            .rows
            .iter()
            .map(|row| (row.signed_time_points, row.other_points))
            .collect::<Vec<_>>(),
    )?;
    diagnostic(
        &mut result,
        None,
        "post-finish-extras-as2026-policy",
        "Jälkimaalileimat säilytettiin ja maalikello sidottiin maalileimaan. Pisteytys on AS2026-käytäntö, ei vahvistettu AS2015A-tulos.",
    );
    Ok(result)
}

/// Laskee kohdistetun kortin. Tulos on tarkastettava ennen virallista käyttöä:
/// diagnostiikka kertoo vanhan ohjelman ristiriidoista ja nimetyistä käytännöistä.
/// Tallennettuja kilpailijatietoja ei muunnetta tähän APIin huomaamatta.
pub fn calculate_live(
    rows: &[MatchedRow],
    options: &LiveCalculationOptions,
) -> Result<LiveCalculation, CoreError> {
    options.parameters.validate()?;
    let official = options.official_start.seconds_since_midnight()?;
    let cumulative_limit = options
        .cumulative_limit_minutes
        .unwrap_or(options.parameters.section_limit_minutes);
    if cumulative_limit < 0 {
        return Err(CoreError::InvalidInput(
            "negatiivinen kokonaismyöhästymisraja".into(),
        ));
    }
    if options.timing_method == TimingMethod::LegacyMode2
        && options.policy == CalculationPolicy::RecoveredLiveV1
    {
        return Err(CoreError::Unsupported("ajanottotavan 2 laskenta kuuluu uuteen staattisesti palautettuun korttilaskentakäytäntöön".into()));
    }
    validate_rows(rows)?;
    let mut result = LiveCalculation {
        policy: options.policy,
        rows: rows
            .iter()
            .map(|source| CalculatedRow {
                source: source.clone(),
                received_code: source.punch.map_or(0, |p| p.code),
                effective_rule: source.station.as_ref().map_or(Rule::Normal, |s| s.rule),
                derived_elapsed_seconds: source.punch.map_or(0, |p| i64::from(p.elapsed_seconds)),
                clock_seconds: source
                    .clock
                    .map(|c| c.seconds_since_midnight())
                    .transpose()
                    .expect("kellot validoitu"),
                target_seconds: source.station.as_ref().map_or(0, |s| s.target_seconds),
                actual_seconds: 0,
                signed_time_points: 0,
                other_points: 0,
                total_points: 0,
                note: String::new(),
            })
            .collect(),
        totals: PenaltyTotals::default(),
        status: CalculationStatus::Checked,
        reason: String::new(),
        finish_clock_seconds: official,
        total_duration_seconds: 0,
        cumulative_schedule_difference_seconds: 0,
        diagnostics: Vec::new(),
        trace: Vec::new(),
    };
    trace(
        &mut result,
        None,
        "policy",
        match options.policy {
            CalculationPolicy::RecoveredLiveV1 => {
                "Aikaisemman Rust-prototyypin korttilaskenta; ei historiallisesti varmennettu."
            }
            CalculationPolicy::RecoveredAs2015aStaticV2 => {
                "AS2015A:n live-korttilaskennasta staattisesti palautettu laskentatapa; ei alkuperäisohjelmalla ajamalla varmennettu."
            }
        },
    );
    let start_is_manual = expected(&result.rows[0]) == 0;
    if result.rows[0].source.kind == MatchKind::Missing {
        if start_is_manual {
            return Err(CoreError::Unsupported("leimattomalle lähdölle ei ole hyväksyttyä lähtöaikaa; tämän tapauksen laskentaa ei ole vielä vahvistettu".into()));
        }
        if options.policy == CalculationPolicy::RecoveredAs2015aStaticV2 {
            return Err(CoreError::Unsupported(
                "Leimallinen lähtö puuttuu: AS2015A palaa ennen pistesummien alustusta. Vanhoja muistisummia ei käytetä uuden tuloksen korvikkeena".into(),
            ));
        }
        result.rows[0].other_points = 9999;
        result.rows[0].total_points = 9999;
        result.rows[0].note = "lähtö puuttuu".into();
        result.totals = sum_row_penalties(&[(0, 9999)])?;
        interrupt(&mut result, Some(1), "lähtö puuttuu");
        return Ok(result);
    }
    if start_is_manual && official == 0 {
        return Err(CoreError::Unsupported(
            "00:00:00-käsilähtö luokitellaan vanhassa ohjelmassa puuttuvaksi".into(),
        ));
    }
    let mut reference = if start_is_manual {
        match options.timing_method {
            TimingMethod::ManualClock => 0,
            TimingMethod::Emit | TimingMethod::LegacyMode2 => {
                // Molemmat menetelmät käyttävät live-korttipolussa samaa
                // lähtöankkuria; tallennettujen tulosten tarkistus on eri polku.
                let value = if options.reset_correction_seconds == 0 {
                    120
                } else {
                    options.reset_correction_seconds
                };
                result.rows[0].derived_elapsed_seconds = value;
                value
            }
        }
    } else {
        add(
            result.rows[0].derived_elapsed_seconds,
            options.reset_correction_seconds,
            "lähdön aikaviite",
        )?
    };
    result.rows[0].clock_seconds = Some(official);
    trace(
        &mut result,
        Some(1),
        "reference-start",
        format!("jakson aikaviite {reference} s"),
    );
    if start_is_manual
        && matches!(
            options.timing_method,
            TimingMethod::Emit | TimingMethod::LegacyMode2
        )
        && options.reset_correction_seconds != 0
    {
        diagnostic(
            &mut result,
            Some(1),
            "manual-start-display-correction",
            "Vanha ohjelma voi vähentää leimattoman lähdön aikakorjauksen kahdesti leimausten kellonajoista. Pisteissä ja maaliajassa korjaus tehdään kerran. Tässä leimausten kellonajat noudattavat vanhan ohjelman näyttökaavaa.",
        );
    }
    let last = result.rows.len() - 1;
    if options.policy == CalculationPolicy::RecoveredAs2015aStaticV2
        && expected(&result.rows[last]) != 0
        && !present(&result.rows[last])
    {
        return Err(CoreError::Unsupported(
            "Leimallinen maali puuttuu: AS2015A keskeyttää korttilaskennan ennen lopullisia asemien summia. Epätäydellistä tulosta ei julkaista".into(),
        ));
    }
    result.total_duration_seconds = sub(
        i64::from(options.last_punch_elapsed_seconds),
        result.rows[0].derived_elapsed_seconds,
        "kokonaiskesto",
    )?;
    if result.total_duration_seconds < 0 {
        return Err(CoreError::InvalidInput(
            "viimeisen leiman aika edeltää lähtöankkuria".into(),
        ));
    }
    result.finish_clock_seconds = add(official, result.total_duration_seconds, "maalikello")?;
    let manual_finish = expected(&result.rows[last]) == 0;
    let legacy_manual_finish =
        manual_finish && options.policy == CalculationPolicy::RecoveredAs2015aStaticV2;
    if manual_finish {
        if legacy_manual_finish {
            // M0 luokitellaan ennen maalikellon kirjoitusta. Koordinaatti ja
            // toteutunut aika jäävät nolliksi ensimmäisellä kierroksella.
            diagnostic(
                &mut result,
                Some(last + 1),
                "manual-finish-legacy-order",
                "AS2015A:n leimaamattoman maalin toteutunut aika jää nollaksi, ja maalikello kirjoitetaan vasta ennen toista sakkokierrosta. Tulos on staattinen ennuste.",
            );
        } else {
            result.rows[last].derived_elapsed_seconds =
                i64::from(options.last_punch_elapsed_seconds);
            result.rows[last].clock_seconds = Some(result.finish_clock_seconds);
            diagnostic(
                &mut result,
                Some(last + 1),
                "manual-finish-derived",
                "Aiempi Rust-käytäntö johtaa maalin ajan viimeisestä leimasta, eikä vastaa palautettua AS2015A:n live-korttipolkua.",
            );
        }
    }
    let clock_baseline = if matches!(
        options.timing_method,
        TimingMethod::Emit | TimingMethod::LegacyMode2
    ) {
        add(
            result.rows[0].derived_elapsed_seconds,
            options.reset_correction_seconds,
            "näyttökellon aikaviite",
        )?
    } else {
        result.rows[0].derived_elapsed_seconds
    };
    let mut pending_target = 0;
    let mut pending_missing = 0usize;
    for index in 1..result.rows.len() {
        let kind = station_kind(&result.rows[index]);
        if present(&result.rows[index]) {
            match kind {
                Some(StationKind::Ula) => {
                    if matches!(
                        result.rows[index].effective_rule,
                        Rule::UlaNearestMinute | Rule::UlaFloorMinute | Rule::UlaCeilMinute
                    ) {
                        let raw = u32::try_from(result.rows[index].derived_elapsed_seconds)
                            .map_err(|_| {
                                CoreError::Unsupported("negatiivinen ULA-koordinaatti".into())
                            })?;
                        let rounded = round_ula_elapsed(raw, result.rows[index].effective_rule)?;
                        result.rows[index].derived_elapsed_seconds = i64::from(rounded);
                        trace(
                            &mut result,
                            Some(index + 1),
                            "ula-round",
                            format!("{raw} → {rounded} s"),
                        );
                    }
                    reference = result.rows[index].derived_elapsed_seconds;
                    trace(
                        &mut result,
                        Some(index + 1),
                        "reference-ula",
                        format!("uusi aikaviite {reference} s"),
                    );
                }
                Some(StationKind::At | StationKind::Jat | StationKind::Rt) => {
                    result.rows[index].actual_seconds = sub(
                        result.rows[index].derived_elapsed_seconds,
                        reference,
                        "aseman toteutunut aika",
                    )?;
                    if pending_missing != 0 {
                        result.rows[index].target_seconds = add(
                            result.rows[index].target_seconds,
                            pending_target,
                            "puuttuvan JAT:n tavoitesiirto",
                        )?;
                        trace(
                            &mut result,
                            Some(index + 1),
                            "missing-jat-carry",
                            format!(
                                "{pending_missing} puuttuvaa JAT:ta: tavoite +{pending_target} s"
                            ),
                        );
                    }
                    if kind == Some(StationKind::Jat) {
                        reference = result.rows[index].derived_elapsed_seconds;
                        trace(
                            &mut result,
                            Some(index + 1),
                            "reference-jat",
                            format!("uusi aikaviite {reference} s"),
                        );
                    }
                }
                Some(StationKind::Finish) if pending_missing != 0 => {
                    diagnostic(
                        &mut result,
                        Some(index + 1),
                        "missing-jat-carry-cleared-at-finish",
                        "Palautettu ensimmäinen kierros tyhjentää puuttuvan JAT:n tavoitesiirron ennen erillistä maalilaskentaa; tavoitetta ei lisätä maaliin.",
                    );
                }
                _ => {}
            }
            pending_target = 0;
            pending_missing = 0;
        } else if kind == Some(StationKind::Jat) {
            pending_target = add(
                pending_target,
                result.rows[index].target_seconds,
                "puuttuvien JAT-tavoitteiden summa",
            )?;
            pending_missing = pending_missing
                .checked_add(1)
                .ok_or(CoreError::ArithmeticOverflow("puuttuvien JAT:ien määrä"))?;
            result.rows[index].note = "JAT puuttuu".into();
        }
        if expected(&result.rows[index]) != 0 && present(&result.rows[index]) {
            let delta = sub(
                result.rows[index].derived_elapsed_seconds,
                clock_baseline,
                "aseman kellon erotus",
            )?;
            result.rows[index].clock_seconds = Some(add(official, delta, "aseman kello")?);
        }
    }
    if present(&result.rows[last]) {
        result.rows[last].actual_seconds = sub(
            result.rows[last].derived_elapsed_seconds,
            reference,
            "maalijakson toteutunut aika",
        )?;
    } else if !legacy_manual_finish {
        diagnostic(
            &mut result,
            Some(last + 1),
            "missing-finish-explicit-result",
            "Vanhan ohjelman korttilaskenta keskeytyy puuttuvaan maalileimaan, eikä kaikkien tuloskenttien käsittely ole selvä. Tässä laskennassa asemarivit säilytetään ja puuttumissakko sekä muiden asemien virheet lasketaan. Kilpailijan tila on keskeyttänyt.",
        );
    }
    if legacy_manual_finish {
        result.rows[last].clock_seconds = Some(result.finish_clock_seconds);
        trace(
            &mut result,
            Some(last + 1),
            "manual-finish-clock-after-first-pass",
            "Maalikello asetettiin ensimmäisen kierroksen jälkeen; toteutunut aika pysyy nollassa",
        );
    }
    // Toinen kierros säilyttää alkuperäisen järjestyksen. Synteettiset muutokset
    // eivät käynnistä ensimmäistä aikaviitekierrosta uudelleen.
    for index in 1..result.rows.len() {
        let kind = station_kind(&result.rows[index]);
        let was_present = present(&result.rows[index]);
        if result.rows[index].effective_rule == Rule::Void {
            diagnostic(
                &mut result,
                Some(index + 1),
                "rule4-ordered-second-pass",
                "Mitätöity rivi säilyy aikarakenteessa. Toisen kierroksen aikamuutoksia ei levitetä uudelleen myöhempien rivien ensimmäisen kierroksen aikoihin.",
            );
            if expected(&result.rows[index]) != 0
                && result.rows[index].received_code == 0
                && kind != Some(StationKind::Ula)
            {
                synthesize_void_row(&mut result, index)?;
            }
            if kind == Some(StationKind::Jat)
                && index + 1 < result.rows.len()
                && station_kind(&result.rows[index + 1]) == Some(StationKind::Ula)
                && expected(&result.rows[index]) == expected(&result.rows[index + 1])
            {
                result.rows[index + 1].derived_elapsed_seconds =
                    result.rows[index].derived_elapsed_seconds;
                result.rows[index + 1].received_code = 0;
                result.rows[index + 1].effective_rule = Rule::Void;
                trace(
                    &mut result,
                    Some(index + 2),
                    "rule4-jat-ula",
                    "JAT:n koordinaatti siirretty ULA:lle; vastaanotettu koodi tyhjennetty ja sääntö 4 asetettu",
                );
            }
            if kind == Some(StationKind::Ula)
                && result.rows[index].received_code == 0
                && expected(&result.rows[index]) != expected(&result.rows[index - 1])
            {
                synthesize_void_row(&mut result, index)?;
            }
        } else if !was_present && expected(&result.rows[index]) != 0 {
            match kind {
                Some(StationKind::At) => {
                    result.rows[index].other_points = options.parameters.missing_at;
                    result.rows[index].note = "AT puuttuu".into();
                }
                Some(StationKind::Jat | StationKind::Ula) => {
                    result.rows[index].other_points = options.parameters.missing_jat;
                    result.rows[index].note = "JAT puuttuu".into();
                    interrupt(&mut result, Some(index + 1), "*JAT puuttuu");
                }
                Some(StationKind::Rt) => {
                    result.rows[index].other_points = options.parameters.missing_rt;
                    result.rows[index].note = "rt puuttuu".into();
                }
                Some(StationKind::Finish) => {
                    result.rows[index].other_points = 9999;
                    result.rows[index].note = "maali puuttuu".into();
                    interrupt(&mut result, Some(index + 1), "*Maali puuttuu");
                }
                _ => {}
            }
        } else if !was_present && expected(&result.rows[index]) == 0 {
            diagnostic(
                &mut result,
                Some(index + 1),
                "missing-manual-row-unscored",
                "Korttilaskenta ei määrää tavallista puuttumissakkoa asemalle, jonka aika oli tarkoitus kirjata käsin. Tallennettujen tulosten uudelleenlaskenta käsittelee nämä asemat erikseen.",
            );
        }
        if result.rows[index].source.kind == MatchKind::Extra {
            let previous = Some(result.rows[index - 1].received_code);
            let two_back = index.checked_sub(2).map(|p| result.rows[p].received_code);
            result.rows[index].other_points = extra_punch_penalty(
                result.rows[index].received_code,
                previous,
                two_back,
                &options.parameters,
            )?;
            result.rows[index].note = if matches!(result.rows[index].received_code, 99 | 999) {
                ""
            } else {
                "at liikaa"
            }
            .into();
        }
        if was_present
            && matches!(
                kind,
                Some(StationKind::At | StationKind::Jat | StationKind::Finish)
            )
        {
            let penalty = time_penalty(
                result.rows[index].target_seconds,
                result.rows[index].actual_seconds,
                kind.expect("asemalaji tarkistettu"),
                result.rows[index].effective_rule,
                &options.parameters,
            )?;
            result.rows[index].signed_time_points = penalty.signed_points;
            if penalty.capped {
                result.rows[index].note = "max myöhä".into();
                trace(
                    &mut result,
                    Some(index + 1),
                    "time-cap",
                    "Aikavirhe ylitti katon; tallennettu etumerkki on vanhan kaavan mukaan negatiivinen",
                );
            }
            if matches!(kind, Some(StationKind::Jat | StationKind::Finish)) {
                let text = result.rows[index]
                    .source
                    .station
                    .as_ref()
                    .expect("asemalaji tarkistettu")
                    .section_time_text
                    .clone();
                if let Some(difference) =
                    section_difference(&text, result.rows[index].actual_seconds, options.policy)?
                {
                    result.cumulative_schedule_difference_seconds = add(
                        result.cumulative_schedule_difference_seconds,
                        difference,
                        "kokonaismyöhästymän summa",
                    )?;
                    let detail = format!(
                        "jaksoero {difference} s; kertymä {} s",
                        result.cumulative_schedule_difference_seconds
                    );
                    trace(&mut result, Some(index + 1), "section-difference", detail);
                    if kind == Some(StationKind::Finish) {
                        if finish_section_limit_exceeded(
                            difference,
                            options.parameters.section_limit_minutes,
                        )? {
                            interrupt(&mut result, Some(index + 1), "*jaksomyöhä ylitetty");
                        }
                    } else if options.parameters.section_limit_minutes != 0 {
                        diagnostic(
                            &mut result,
                            Some(index + 1),
                            "legacy-jat-limit-comparison",
                            "Vanhan ohjelman JAT-aseman myöhästymisrajan vertailu ei voi laueta, kun raja on positiivinen. Laskenta säilyttää tämän vertailun. Kokonaismyöhästymisraja ja maalin jakson raja tarkistetaan erikseen.",
                        );
                    }
                } else if !text.trim_matches(' ').is_empty() {
                    diagnostic(
                        &mut result,
                        Some(index + 1),
                        "ignored-section-text",
                        "Jakso-kenttä ei läpäise palautettua numerotarkistusta; vanha polku lisää kertymään nollan.",
                    );
                }
            }
        }
        if index == last && !was_present {
            interrupt(&mut result, Some(index + 1), "*Maali puuttuu");
        }
        result.rows[index].total_points = row_penalty_total(
            result.rows[index].signed_time_points,
            result.rows[index].other_points,
        )?;
        let row = &result.rows[index];
        let detail = format!(
            "tavoite {} s; toteutunut {} s; aika {} p; muu {} p; yhteensä {} p",
            row.target_seconds,
            row.actual_seconds,
            row.signed_time_points,
            row.other_points,
            row.total_points
        );
        trace(&mut result, Some(index + 1), "row-result", detail);
    }
    let pairs: Vec<_> = result
        .rows
        .iter()
        .map(|row| (row.signed_time_points, row.other_points))
        .collect();
    result.totals = sum_row_penalties(&pairs)?;
    if cumulative_lateness_exceeded(
        result.cumulative_schedule_difference_seconds,
        cumulative_limit,
    )? {
        interrupt(&mut result, None, "*max myöhä ylitetty");
    }
    let detail = format!(
        "aikavirheet {} p; muut virheet {} p; yhteensä {} p",
        result.totals.time_points, result.totals.other_points, result.totals.total_points
    );
    trace(&mut result, None, "total", detail);
    Ok(result)
}

fn synthesize_void_row(result: &mut LiveCalculation, index: usize) -> Result<(), CoreError> {
    let intermediate = add(
        result.rows[index - 1].derived_elapsed_seconds,
        result.rows[index - 1].target_seconds,
        "mitätöidyn rivin edeltävä tavoite",
    )?;
    let elapsed = add(
        intermediate,
        result.rows[index].target_seconds,
        "mitätöidyn rivin synteettinen aika",
    )?;
    result.rows[index].derived_elapsed_seconds = elapsed;
    result.rows[index].actual_seconds = result.rows[index].target_seconds;
    trace(
        result,
        Some(index + 1),
        "rule4-synthetic-time",
        format!("johdettu koordinaatti {elapsed} s; raakaleima säilyy muuttumattomana"),
    );
    Ok(())
}

fn section_difference(
    text: &str,
    actual_seconds: i64,
    policy: CalculationPolicy,
) -> Result<Option<i64>, CoreError> {
    // 0x00408748: 1..9 tavua, vain numerot/välilyönnit, vähintään yksi numero.
    // Sisävälilyöntien x87-muunnosta ei arvata.
    if text.is_empty() || text.len() > 9 || !text.bytes().all(|b| b.is_ascii_digit() || b == b' ') {
        return Ok(None);
    }
    let trimmed = text.trim_matches(' ');
    if trimmed.is_empty() {
        return Ok(None);
    }
    let numeric_prefix = if trimmed.contains(' ') {
        if policy == CalculationPolicy::RecoveredLiveV1 {
            return Err(CoreError::Unsupported(
                "Jakso-kentän sisävälilyönnin vanha numeromuunnos on epäselvä".into(),
            ));
        }
        // AS2015A:n numeromuunnos pysähtyy ensimmäiseen sisävälilyöntiin.
        trimmed.split(' ').next().unwrap_or("")
    } else {
        trimmed
    };
    let minutes = numeric_prefix
        .parse::<i64>()
        .map_err(|_| CoreError::InvalidInput("Jakso-kentän lukuarvo".into()))?;
    let seconds = minutes
        .checked_mul(60)
        .ok_or(CoreError::ArithmeticOverflow("Jakso-kentän sekunnit"))?;
    Ok(Some(sub(seconds, actual_seconds, "jakson aikatauluero")?))
}
