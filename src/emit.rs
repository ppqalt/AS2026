//! 0x004385e4:n kehysrakenne ja 0x0040f448:n ensimmäisen 250-merkin käsittely.
//! Ainoastaan valmiit tavut: ei sarjaporttia, laitekyselyitä tai suoritettavaa PE-koodia.

use crate::{ClockTime, CoreError, Punch};

pub const FRAME_LENGTH: usize = 217;
pub const RECORD_COUNT: usize = 50;
pub const XOR_MASK: u8 = 0xdf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitFrame {
    /// Alkuperäinen kehys säilytetään myös käyttämättömien kenttien tutkimista varten.
    pub raw: [u8; FRAME_LENGTH],
    pub decoded: [u8; FRAME_LENGTH],
    pub card_number: u32,
    pub records: [Punch; RECORD_COUNT],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitReadout {
    pub marker_index: usize,
    pub anchor: Punch,
    pub interior_punches: Vec<Punch>,
    pub last_punch_elapsed_seconds: u32,
    pub reader_elapsed_seconds: u32,
    /// Kirjaimellinen vähennystulos; negatiivista tai vuorokauden ylitystä ei piiloteta.
    pub finish_clock_seconds: i64,
}

pub fn parse_frame(raw: &[u8]) -> Result<EmitFrame, CoreError> {
    let raw: [u8; FRAME_LENGTH] = raw.try_into().map_err(|_| CoreError::FrameLength {
        expected: FRAME_LENGTH,
        actual: raw.len(),
    })?;
    let decoded = raw.map(|byte| byte ^ XOR_MASK);
    if decoded[..2] != [0xff, 0xff] {
        return Err(CoreError::Synchronization);
    }
    let card_number =
        u32::from(decoded[2]) | (u32::from(decoded[3]) << 8) | (u32::from(decoded[4]) << 16);
    let records = std::array::from_fn(|index| {
        let offset = 10 + index * 3;
        Punch {
            code: u32::from(decoded[offset]),
            elapsed_seconds: u32::from(u16::from_le_bytes([
                decoded[offset + 1],
                decoded[offset + 2],
            ])),
        }
    });
    Ok(EmitFrame {
        raw,
        decoded,
        card_number,
        records,
    })
}

impl EmitFrame {
    /// Kortin sisältö ennen lukijan 250-merkkiä, ensimmäinen ankkurileima mukaan lukien.
    /// Lukumerkin vaihtuva aika ja kehyksen loppupehmusteet eivät kuulu tähän sisältöön.
    pub fn punches_before_reader_marker(&self) -> Result<&[Punch], CoreError> {
        punches_before_reader_marker(&self.records)
    }

    /// Kehys voi olla rakenteellisesti kelvollinen mutta vailla hyväksyttävää lukumerkkiä.
    pub fn accepted_readout(&self, readout_clock: ClockTime) -> Result<EmitReadout, CoreError> {
        accepted_records(&self.records, readout_clock)
    }
}

/// Kehysten ja tallennettujen leimalistojen yhteinen sisältövertailu.
/// Lukumerkin vaihtuva aika ja sen jälkeiset kentät eivät kuulu kortin sisältöön.
pub fn punches_before_reader_marker(records: &[Punch]) -> Result<&[Punch], CoreError> {
    let marker_index = reader_marker_index(records)?;
    Ok(&records[..marker_index])
}

/// Dekoodatun kehyksen ja tuodun emit.txt-rivin yhteinen tulkinta.
/// Kutsuja säilyttää alkuperäisen aineiston; tekstitiedoston koodi voi ylittää tavun arvon.
pub fn accepted_records(
    records: &[Punch],
    readout_clock: ClockTime,
) -> Result<EmitReadout, CoreError> {
    let readout_seconds = readout_clock.seconds_since_midnight()?;
    let marker_index = reader_marker_index(records)?;
    let last_punch_elapsed_seconds = records[..marker_index]
        .iter()
        .rev()
        .find(|p| !matches!(p.code, 99 | 999))
        .map_or(records[0].elapsed_seconds, |p| p.elapsed_seconds);
    let reader_elapsed_seconds = records[marker_index].elapsed_seconds;
    let lag = i64::from(reader_elapsed_seconds) - i64::from(last_punch_elapsed_seconds);
    let finish_clock_seconds = readout_seconds
        .checked_sub(lag)
        .ok_or(CoreError::ArithmeticOverflow("Emit-maalikello"))?;
    Ok(EmitReadout {
        marker_index,
        anchor: records[0],
        interior_punches: records[1..marker_index].to_vec(),
        last_punch_elapsed_seconds,
        reader_elapsed_seconds,
        finish_clock_seconds,
    })
}

fn reader_marker_index(records: &[Punch]) -> Result<usize, CoreError> {
    let marker_index = records
        .iter()
        .position(|record| record.code == 250)
        .ok_or(CoreError::MissingMarker)?;
    if marker_index == 0 {
        return Err(CoreError::MarkerAtStart);
    }
    Ok(marker_index)
}

/// Raportin yhden keskiyön korjaus. Ei tulkitse ajanottotapaa eikä muuta syötettyä korjausta.
pub fn calculated_reset_advance(
    finish_clock_seconds: i64,
    official_start: ClockTime,
    last_punch_elapsed_seconds: u32,
) -> Result<i64, CoreError> {
    let mut race_elapsed = finish_clock_seconds
        .checked_sub(official_start.seconds_since_midnight()?)
        .ok_or(CoreError::ArithmeticOverflow("suoritusaika"))?;
    if race_elapsed < 0 {
        race_elapsed = race_elapsed
            .checked_add(86400)
            .ok_or(CoreError::ArithmeticOverflow("keskiyön korjaus"))?;
    }
    i64::from(last_punch_elapsed_seconds)
        .checked_sub(race_elapsed)
        .ok_or(CoreError::ArithmeticOverflow("nollausennakko"))
}
