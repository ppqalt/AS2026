//! 0x00411120:n ahne kohdistus kanonisilla asemalajeilla.
//! Dynaaminen taulukko korvaa alkuperäisen tarkistamattoman 72-rivisen muistialueen.

use crate::{ClockTime, CoreError, MatchKind, MatchedRow, Punch, RouteStation, StationKind};

pub fn match_route(
    route: &[RouteStation],
    punches: &[Punch],
    official_start: ClockTime,
) -> Result<Vec<MatchedRow>, CoreError> {
    official_start.seconds_since_midnight()?;
    if route.is_empty() {
        return Err(CoreError::Unsupported("tyhjän reitin käsittely".into()));
    }
    let capacity = route
        .len()
        .checked_add(punches.len())
        .ok_or(CoreError::ArithmeticOverflow("kohdistusrivien määrä"))?;
    let mut output = Vec::new();
    output
        .try_reserve(capacity)
        .map_err(|_| CoreError::InvalidInput("kohdistusriveille ei voida varata muistia".into()))?;
    let mut r = 0;
    let mut p = 0;
    while r < route.len() || p < punches.len() {
        let station = route.get(r);
        let shared_ula = station.is_some_and(|station| {
            r > 0
                && station.expected_code != 0
                && station.expected_code == route[r - 1].expected_code
                && station.kind == StationKind::Ula
        });
        let kind = match station {
            // Battery markers never satisfy a course control, even if its code
            // happens to be configured as 99/999. Keep them visible for editing.
            _ if punches.get(p).is_some_and(|p| matches!(p.code, 99 | 999))
                && station.is_none_or(|s| s.expected_code != 0) =>
            {
                MatchKind::Extra
            }
            None => MatchKind::Extra,
            Some(station) if p >= punches.len() => {
                if shared_ula {
                    MatchKind::SharedUla
                } else if station.expected_code == 0 && station.kind == StationKind::Finish {
                    MatchKind::ManualFinish
                } else {
                    // Myös L0 jää puuttuvaksi, jos leimaukset loppuivat jo ennen sitä.
                    MatchKind::Missing
                }
            }
            Some(station)
                if station.expected_code != 0 && station.expected_code == punches[p].code =>
            {
                MatchKind::Matched
            }
            Some(station) if station.expected_code == 0 => match station.kind {
                StationKind::Start => MatchKind::ManualStart,
                StationKind::Finish => MatchKind::ManualFinish,
                _ => MatchKind::RouteOnly,
            },
            Some(_) if shared_ula => MatchKind::SharedUla,
            Some(station) => {
                if punches[p + 1..]
                    .iter()
                    .any(|punch| punch.code == station.expected_code)
                {
                    MatchKind::Extra
                } else {
                    MatchKind::Missing
                }
            }
        };
        let mut row = MatchedRow {
            output_index: output.len() + 1,
            station: None,
            punch: None,
            kind,
            clock: None,
        };
        match kind {
            MatchKind::Matched => {
                row.station = Some(route[r].clone());
                row.punch = Some(punches[p]);
                r += 1;
                p += 1;
            }
            MatchKind::Extra => {
                row.punch = Some(punches[p]);
                p += 1;
            }
            MatchKind::SharedUla => {
                row.station = Some(route[r].clone());
                // Alkuperäinen p-1 on nollattu ankkuripaikka ennen ensimmäistä sisäleimausta.
                let elapsed_seconds = punches[..p]
                    .iter()
                    .rev()
                    .find(|p| !matches!(p.code, 99 | 999))
                    .map_or(0, |punch| punch.elapsed_seconds);
                row.punch = Some(Punch {
                    code: route[r].expected_code,
                    elapsed_seconds,
                });
                r += 1;
            }
            MatchKind::ManualStart => {
                row.station = Some(route[r].clone());
                row.clock = Some(official_start);
                r += 1;
            }
            MatchKind::ManualFinish => {
                row.station = Some(route[r].clone());
                row.clock = Some(ClockTime {
                    hour: 0,
                    minute: 0,
                    second: 0,
                });
                r += 1;
            }
            MatchKind::Missing | MatchKind::RouteOnly => {
                row.station = Some(route[r].clone());
                r += 1;
            }
        }
        output.push(row);
    }
    Ok(output)
}
