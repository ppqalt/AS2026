//! Varmennetut pistelaskennan perusosat. Ei koko asema-, jakso- tai tilakonetta.
//! Erityisesti sääntö 4 säilyy mallissa: sen puuttuvien asemien aikamuutoksia ei tehdä tässä.

use crate::{CoreError, Rule, StationKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringParameters {
    pub tolerance_seconds: i64,
    pub early_points_per_second: i64,
    pub late_points_per_second: i64,
    pub missing_at: i64,
    pub extra_at: i64,
    pub missing_jat: i64,
    pub missing_rt: i64,
    pub extra_rt: i64,
    pub at_cap: i64,
    pub jat_cap: i64,
    pub section_limit_minutes: i64,
}

impl Default for ScoringParameters {
    fn default() -> Self {
        Self {
            tolerance_seconds: 10,
            early_points_per_second: 3,
            late_points_per_second: 1,
            missing_at: 1800,
            extra_at: 600,
            missing_jat: 3600,
            missing_rt: 600,
            extra_rt: 0,
            at_cap: 1800,
            jat_cap: 3600,
            section_limit_minutes: 60,
        }
    }
}

impl ScoringParameters {
    pub fn validate(&self) -> Result<(), CoreError> {
        let values = [
            self.tolerance_seconds,
            self.early_points_per_second,
            self.late_points_per_second,
            self.missing_at,
            self.extra_at,
            self.missing_jat,
            self.missing_rt,
            self.extra_rt,
            self.at_cap,
            self.jat_cap,
            self.section_limit_minutes,
        ];
        if values.iter().any(|value| *value < 0) {
            return Err(CoreError::InvalidInput(
                "pistelaskennan parametrit eivät saa olla negatiivisia".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimePenalty {
    pub signed_points: i64,
    pub capped: bool,
}

pub fn time_penalty(
    target_seconds: i64,
    actual_seconds: i64,
    kind: StationKind,
    rule: Rule,
    parameters: &ScoringParameters,
) -> Result<TimePenalty, CoreError> {
    parameters.validate()?;
    let cap = match kind {
        StationKind::At => parameters.at_cap,
        StationKind::Jat | StationKind::Finish => parameters.jat_cap,
        _ => {
            return Err(CoreError::Unsupported(
                "aikavirhe lasketaan tässä vain AT/JAT/maali-riveille".into(),
            ));
        }
    };
    if matches!(
        rule,
        Rule::UlaNearestMinute | Rule::UlaFloorMinute | Rule::UlaCeilMinute
    ) {
        return Err(CoreError::Unsupported(
            "ULA-pyöristyssääntö aikavirheasemalla".into(),
        ));
    }
    if matches!(rule, Rule::NoTiming | Rule::Void) {
        return Ok(TimePenalty {
            signed_points: 0,
            capped: false,
        });
    }
    let difference = target_seconds
        .checked_sub(actual_seconds)
        .ok_or(CoreError::ArithmeticOverflow("tavoiteajan erotus"))?;
    let signed_points = if difference > parameters.tolerance_seconds && rule != Rule::NoEarly {
        difference
            .checked_sub(parameters.tolerance_seconds)
            .and_then(|seconds| seconds.checked_mul(parameters.early_points_per_second))
            .ok_or(CoreError::ArithmeticOverflow("ennakkovirhepisteet"))?
    } else if difference < -parameters.tolerance_seconds && rule != Rule::NoLate {
        difference
            .checked_add(parameters.tolerance_seconds)
            .and_then(|seconds| seconds.checked_mul(parameters.late_points_per_second))
            .ok_or(CoreError::ArithmeticOverflow("myöhästymisvirhepisteet"))?
    } else {
        0
    };
    let magnitude = signed_points
        .checked_abs()
        .ok_or(CoreError::ArithmeticOverflow("aikavirheen itseisarvo"))?;
    // Myös liian suuren ennakkosakon etumerkki muuttuu alkuperäisessä miinukseksi.
    if magnitude > cap {
        Ok(TimePenalty {
            signed_points: -cap,
            capped: true,
        })
    } else {
        Ok(TimePenalty {
            signed_points,
            capped: false,
        })
    }
}

pub fn round_ula_elapsed(elapsed_seconds: u32, rule: Rule) -> Result<u32, CoreError> {
    let remainder = elapsed_seconds % 60;
    match rule {
        Rule::UlaNearestMinute if remainder >= 30 => elapsed_seconds.checked_add(60 - remainder),
        Rule::UlaNearestMinute | Rule::UlaFloorMinute => elapsed_seconds.checked_sub(remainder),
        Rule::UlaCeilMinute if remainder != 0 => elapsed_seconds.checked_add(60 - remainder),
        _ => return Ok(elapsed_seconds),
    }
    .ok_or(CoreError::ArithmeticOverflow("ULA-pyöristys"))
}

/// Vastaanotetut 99/999 ovat eri asia kuin ylimääräisen rivin odotetun koodin 999-merkki.
pub fn extra_punch_penalty(
    current_received: u32,
    previous_received: Option<u32>,
    two_back_received: Option<u32>,
    parameters: &ScoringParameters,
) -> Result<i64, CoreError> {
    parameters.validate()?;
    if matches!(current_received, 99 | 999)
        || (matches!(previous_received, Some(99 | 999))
            && two_back_received == Some(current_received))
    {
        Ok(0)
    } else {
        Ok(parameters.extra_at)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PenaltyTotals {
    pub time_points: i64,
    pub other_points: i64,
    pub total_points: i64,
}

pub fn row_penalty_total(signed_time_points: i64, other_points: i64) -> Result<i64, CoreError> {
    if other_points < 0 {
        return Err(CoreError::InvalidInput(
            "muu virhepistemäärä ei saa olla negatiivinen".into(),
        ));
    }
    signed_time_points
        .checked_abs()
        .and_then(|time| time.checked_add(other_points))
        .ok_or(CoreError::ArithmeticOverflow("rivin virhepistesumma"))
}

pub fn sum_row_penalties(rows: &[(i64, i64)]) -> Result<PenaltyTotals, CoreError> {
    let mut total = PenaltyTotals::default();
    for &(time, other) in rows {
        let row_total = row_penalty_total(time, other)?;
        total.time_points = total
            .time_points
            .checked_add(
                time.checked_abs()
                    .ok_or(CoreError::ArithmeticOverflow("aikavirheen itseisarvo"))?,
            )
            .ok_or(CoreError::ArithmeticOverflow("aikavirheiden summa"))?;
        total.other_points = total
            .other_points
            .checked_add(other)
            .ok_or(CoreError::ArithmeticOverflow("muiden virheiden summa"))?;
        total.total_points = total
            .total_points
            .checked_add(row_total)
            .ok_or(CoreError::ArithmeticOverflow("virheiden yhteissumma"))?;
    }
    Ok(total)
}

/// Aikavirheet, yksilöidyt reittivirheet, puuttuvat RT:t, muut virheet, ylimääräiset RT:t.
pub fn competitor_total(parts: [i64; 5]) -> Result<i64, CoreError> {
    if parts.iter().any(|part| *part < 0) {
        return Err(CoreError::InvalidInput(
            "kokonaistuloksen osat eivät saa olla negatiivisia".into(),
        ));
    }
    parts.into_iter().try_fold(0_i64, |sum, part| {
        sum.checked_add(part).ok_or(CoreError::ArithmeticOverflow(
            "kilpailijan kokonaisvirhepisteet",
        ))
    })
}

fn validate_limit(limit_minutes: i64) -> Result<(), CoreError> {
    if limit_minutes < 0 {
        return Err(CoreError::Unsupported(
            "negatiivinen myöhästymisraja".into(),
        ));
    }
    Ok(())
}

/// Alkuperäisen signed division: katkaisee kohti nollaa; etuajo voi kumota myöhästymistä.
pub fn cumulative_lateness_exceeded(
    difference_seconds: i64,
    limit_minutes: i64,
) -> Result<bool, CoreError> {
    validate_limit(limit_minutes)?;
    Ok(difference_seconds / 60 < -limit_minutes)
}

/// Maalin kirjaimellinen tarkistus koskee itseisarvon vuoksi myös etuajassa oloa.
pub fn finish_section_limit_exceeded(
    difference_seconds: i64,
    limit_minutes: i64,
) -> Result<bool, CoreError> {
    validate_limit(limit_minutes)?;
    Ok(limit_minutes != 0 && (difference_seconds / 60).abs() > limit_minutes)
}

/// Dokumentoitu JAT-ristiriita: abs(erotus/60) < -raja. Positiivisella rajalla epätosi.
/// Tarkoitettua urheilusääntöä ei päätellä eikä korjata tässä.
pub fn jat_section_limit_exceeded_literal(
    difference_seconds: i64,
    limit_minutes: i64,
) -> Result<bool, CoreError> {
    validate_limit(limit_minutes)?;
    Ok(limit_minutes != 0 && (difference_seconds / 60).abs() < -limit_minutes)
}
