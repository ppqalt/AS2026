//! Kaikki tapaukset ovat synteettisiä, raporteista johdettuja odotusarvoja.
//! AS2015A:ta tai sarjaporttia ei käynnistetä eikä tuloksia väitetä historialliseksi vertailuksi.

use as2026::emit::{FRAME_LENGTH, XOR_MASK, calculated_reset_advance, parse_frame};
use as2026::matching::match_route;
use as2026::scoring::*;
use as2026::{ClockTime, CoreError, MatchKind, Punch, RouteStation, Rule, StationKind};

fn clock(hour: u32, minute: u32, second: u32) -> ClockTime {
    ClockTime::new(hour, minute, second).unwrap()
}

fn station(sequence: u32, kind: StationKind, expected_code: u32) -> RouteStation {
    RouteStation {
        sequence,
        kind,
        label: format!("Asema {sequence}"),
        expected_code,
        target_seconds: 300,
        rule: Rule::Normal,
        section_time_text: String::new(),
    }
}

fn punches(codes: &[u32]) -> Vec<Punch> {
    codes
        .iter()
        .enumerate()
        .map(|(index, code)| Punch {
            code: *code,
            elapsed_seconds: (index as u32 + 1) * 100,
        })
        .collect()
}

fn raw_frame(card: u32, records: &[(u8, u16)]) -> [u8; FRAME_LENGTH] {
    let mut decoded = [0_u8; FRAME_LENGTH];
    decoded[0..2].copy_from_slice(&[0xff, 0xff]);
    decoded[2..5].copy_from_slice(&card.to_le_bytes()[..3]);
    // Käyttämättömät tavut säilyvät: niiden merkitystä ei oleteta.
    decoded[216] = 0xa7;
    for (index, (code, elapsed)) in records.iter().enumerate() {
        decoded[10 + index * 3] = *code;
        decoded[11 + index * 3..13 + index * 3].copy_from_slice(&elapsed.to_le_bytes());
    }
    decoded.map(|byte| byte ^ XOR_MASK)
}

#[test]
fn emit_preserves_frame_and_decodes_u24_card_and_u16_seconds() {
    let raw = raw_frame(0x12ab34, &[(0, 0), (31, 65535), (250, 65535)]);
    let frame = parse_frame(&raw).unwrap();
    assert_eq!(frame.raw, raw);
    assert_eq!(frame.decoded[216], 0xa7);
    assert_eq!(frame.card_number, 0x12ab34);
    assert_eq!(frame.records.len(), 50);
    assert_eq!(
        frame.records[1],
        Punch {
            code: 31,
            elapsed_seconds: 65535
        }
    );
    assert_eq!(frame.records[49], Punch::default());
}

#[test]
fn emit_rejects_wrong_lengths_and_sync_without_scanning_or_truncating() {
    let raw = raw_frame(42, &[]);
    for size in [0, 1, 216, 218, 434] {
        assert!(
            matches!(parse_frame(&vec![0x20; size]), Err(CoreError::FrameLength { actual, .. }) if actual == size)
        );
    }
    for index in [0, 1] {
        let mut malformed = raw;
        malformed[index] ^= 1;
        assert!(matches!(
            parse_frame(&malformed),
            Err(CoreError::Synchronization)
        ));
    }
}

#[test]
fn emit_first_marker_omits_anchor_and_marker_from_interior() {
    let frame = parse_frame(&raw_frame(
        42,
        &[
            (0, 120),
            (31, 420),
            (32, 720),
            (250, 725),
            (99, 800),
            (250, 900),
        ],
    ))
    .unwrap();
    let readout = frame.accepted_readout(clock(12, 0, 10)).unwrap();
    assert_eq!(readout.marker_index, 3);
    assert_eq!(readout.anchor.elapsed_seconds, 120);
    assert_eq!(
        readout.interior_punches,
        vec![
            Punch {
                code: 31,
                elapsed_seconds: 420
            },
            Punch {
                code: 32,
                elapsed_seconds: 720
            },
        ]
    );
    assert_eq!(readout.last_punch_elapsed_seconds, 720);
    assert_eq!(readout.reader_elapsed_seconds, 725);
    assert_eq!(readout.finish_clock_seconds, 12 * 3600 + 5);
}

#[test]
fn emit_rejects_missing_or_initial_marker_but_keeps_signed_finish() {
    let missing = parse_frame(&raw_frame(42, &[(0, 0), (31, 10)])).unwrap();
    assert!(matches!(
        missing.accepted_readout(clock(0, 0, 0)),
        Err(CoreError::MissingMarker)
    ));
    let initial = parse_frame(&raw_frame(42, &[(250, 10), (31, 0)])).unwrap();
    assert!(matches!(
        initial.accepted_readout(clock(0, 0, 0)),
        Err(CoreError::MarkerAtStart)
    ));
    let midnight = parse_frame(&raw_frame(42, &[(0, 120), (31, 3600), (250, 3610)])).unwrap();
    assert_eq!(
        midnight
            .accepted_readout(clock(0, 0, 5))
            .unwrap()
            .finish_clock_seconds,
        -5
    );
    assert_eq!(
        calculated_reset_advance(-5, clock(23, 1, 55), 3600).unwrap(),
        120
    );
    assert!(calculated_reset_advance(i64::MIN, clock(1, 0, 0), 0).is_err());
}

#[test]
fn clock_rejects_invalid_time_even_when_fields_constructed_directly() {
    for (hour, minute, second) in [(24, 0, 0), (0, 60, 0), (0, 0, 60), (u32::MAX, 0, 0)] {
        assert!(ClockTime::new(hour, minute, second).is_err());
        assert!(
            ClockTime {
                hour,
                minute,
                second
            }
            .seconds_since_midnight()
            .is_err()
        );
    }
}

#[test]
fn time_penalty_tolerance_and_signed_direction_tables() {
    let params = ScoringParameters::default();
    for (actual, expected) in [
        (275, 45),
        (289, 3),
        (290, 0),
        (299, 0),
        (300, 0),
        (301, 0),
        (310, 0),
        (311, -1),
        (325, -15),
    ] {
        let result = time_penalty(300, actual, StationKind::At, Rule::Normal, &params).unwrap();
        assert_eq!(result.signed_points, expected, "toteutunut {actual}");
        assert!(!result.capped);
    }
}

#[test]
fn cap_boundary_and_legacy_negative_cap_sign_are_preserved() {
    let params = ScoringParameters::default();
    for (kind, actual, expected, capped) in [
        (StationKind::At, 390, 1800, false),
        (StationKind::At, 389, -1800, true),
        (StationKind::At, 2810, -1800, false),
        (StationKind::At, 2811, -1800, true),
        (StationKind::Jat, -210, 3600, false),
        (StationKind::Jat, -211, -3600, true),
        (StationKind::Finish, 4611, -3600, true),
    ] {
        assert_eq!(
            time_penalty(1000, actual, kind, Rule::Normal, &params).unwrap(),
            TimePenalty {
                signed_points: expected,
                capped
            }
        );
    }
}

#[test]
fn time_rules_suppress_only_the_documented_sides() {
    let params = ScoringParameters::default();
    for (rule, early, late) in [
        (Rule::Normal, 45, -15),
        (Rule::NoEarly, 0, -15),
        (Rule::NoTiming, 0, 0),
        (Rule::NoLate, 45, 0),
        (Rule::Void, 0, 0),
    ] {
        assert_eq!(
            time_penalty(300, 275, StationKind::Jat, rule, &params)
                .unwrap()
                .signed_points,
            early
        );
        assert_eq!(
            time_penalty(300, 325, StationKind::Jat, rule, &params)
                .unwrap()
                .signed_points,
            late
        );
    }
    for kind in [StationKind::Start, StationKind::Ula, StationKind::Rt] {
        assert!(matches!(
            time_penalty(300, 300, kind, Rule::Normal, &params),
            Err(CoreError::Unsupported(_))
        ));
    }
    assert!(matches!(
        time_penalty(300, 300, StationKind::At, Rule::UlaNearestMinute, &params),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn scoring_arithmetic_rejects_overflow_and_negative_parameters() {
    let params = ScoringParameters::default();
    assert!(time_penalty(i64::MAX, -1, StationKind::At, Rule::Normal, &params).is_err());
    assert!(time_penalty(i64::MAX, 0, StationKind::At, Rule::Normal, &params).is_err());
    let invalid = ScoringParameters {
        tolerance_seconds: -1,
        ..params
    };
    assert!(invalid.validate().is_err());
    assert!(row_penalty_total(i64::MIN, 0).is_err());
    assert!(row_penalty_total(0, -1).is_err());
    assert!(sum_row_penalties(&[(i64::MAX, 0), (1, 0)]).is_err());
    assert!(competitor_total([i64::MAX, 1, 0, 0, 0]).is_err());
}

#[test]
fn time_totals_sum_magnitudes_and_five_competitor_fields() {
    assert_eq!(row_penalty_total(-15, 600).unwrap(), 615);
    assert_eq!(
        sum_row_penalties(&[(45, 0), (-15, 600), (-1800, 0)]).unwrap(),
        PenaltyTotals {
            time_points: 1860,
            other_points: 600,
            total_points: 2460
        }
    );
    assert_eq!(competitor_total([1860, 600, 1200, 50, 0]).unwrap(), 3710);
}

#[test]
fn greedy_alignment_and_duplicate_station_tables() {
    use MatchKind::{Extra as E, Matched as M, Missing as X};
    for (expected, received, kinds) in [
        (vec![31, 32], vec![31, 32], vec![M, M]),
        (vec![31, 32], vec![31, 90, 32], vec![M, E, M]),
        (vec![31, 32, 33], vec![31, 33], vec![M, X, M]),
        (vec![31, 32, 33], vec![32, 33, 31], vec![E, E, M, X, X]),
        (vec![31, 31, 32], vec![31, 32], vec![M, X, M]),
        (vec![31], vec![31, 99], vec![M, E]),
        (vec![99], vec![99], vec![M]),
    ] {
        let route: Vec<_> = expected
            .iter()
            .enumerate()
            .map(|(index, code)| station(index as u32 + 1, StationKind::At, *code))
            .collect();
        let rows = match_route(&route, &punches(&received), clock(10, 0, 0)).unwrap();
        assert_eq!(rows.iter().map(|row| row.kind).collect::<Vec<_>>(), kinds);
        assert!(
            rows.iter()
                .enumerate()
                .all(|(index, row)| row.output_index == index + 1)
        );
        for row in rows.iter().filter(|row| row.kind == E) {
            assert!(row.station.is_none());
            assert!(row.punch.is_some());
        }
    }
}

#[test]
fn ula_reuses_previous_received_time_but_direct_match_wins() {
    let route = vec![
        station(1, StationKind::Jat, 31),
        station(2, StationKind::Ula, 31),
        station(3, StationKind::Finish, 32),
    ];
    let reused = match_route(&route, &punches(&[31, 32]), clock(10, 0, 0)).unwrap();
    assert_eq!(reused[1].kind, MatchKind::SharedUla);
    assert_eq!(reused[1].punch.unwrap().elapsed_seconds, 100);
    assert_eq!(reused[2].punch.unwrap().code, 32);
    let direct = match_route(&route, &punches(&[31, 31, 32]), clock(10, 0, 0)).unwrap();
    assert_eq!(direct[1].kind, MatchKind::Matched);
    assert_eq!(direct[1].punch.unwrap().elapsed_seconds, 200);
    let exhausted = match_route(&route[..2], &punches(&[31]), clock(10, 0, 0)).unwrap();
    assert_eq!(exhausted[1].kind, MatchKind::SharedUla);
    assert_eq!(exhausted[1].punch.unwrap().elapsed_seconds, 100);
}

#[test]
fn ula_condition_uses_route_adjacency_without_checking_previous_received_code() {
    let route = vec![
        station(1, StationKind::At, 31),
        station(2, StationKind::Ula, 31),
    ];
    let rows = match_route(&route, &punches(&[90]), clock(10, 0, 0)).unwrap();
    assert_eq!(rows[0].kind, MatchKind::Missing);
    assert_eq!(rows[1].kind, MatchKind::SharedUla);
    assert_eq!(
        rows[1].punch,
        Some(Punch {
            code: 31,
            elapsed_seconds: 0
        })
    );
    assert_eq!(rows[2].kind, MatchKind::Extra);
}

#[test]
fn manual_start_finish_and_exhausted_start_asymmetry() {
    let official_start = clock(10, 0, 0);
    let route = vec![
        station(1, StationKind::Start, 0),
        station(2, StationKind::At, 31),
        station(3, StationKind::Finish, 0),
    ];
    let rows = match_route(&route, &punches(&[31]), official_start).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
        vec![
            MatchKind::ManualStart,
            MatchKind::Matched,
            MatchKind::ManualFinish
        ]
    );
    assert_eq!(rows[0].clock, Some(official_start));
    assert_eq!(rows[2].clock, Some(clock(0, 0, 0)));
    let empty_punches =
        match_route(&[route[0].clone(), route[2].clone()], &[], official_start).unwrap();
    assert_eq!(empty_punches[0].kind, MatchKind::Missing);
    assert_eq!(empty_punches[0].clock, None);
    assert_eq!(empty_punches[1].kind, MatchKind::ManualFinish);
    assert!(matches!(
        match_route(&[], &[], official_start),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn ula_rounding_boundary_tables_preserve_exact_minutes() {
    for (seconds, nearest, floor, ceil) in [
        (0, 0, 0, 0),
        (60, 60, 60, 60),
        (89, 60, 60, 120),
        (90, 120, 60, 120),
        (119, 120, 60, 120),
        (120, 120, 120, 120),
    ] {
        assert_eq!(
            round_ula_elapsed(seconds, Rule::UlaNearestMinute).unwrap(),
            nearest
        );
        assert_eq!(
            round_ula_elapsed(seconds, Rule::UlaFloorMinute).unwrap(),
            floor
        );
        assert_eq!(
            round_ula_elapsed(seconds, Rule::UlaCeilMinute).unwrap(),
            ceil
        );
    }
    assert!(round_ula_elapsed(u32::MAX, Rule::UlaCeilMinute).is_err());
    assert_eq!(round_ula_elapsed(99, Rule::Normal).unwrap(), 99);
}

#[test]
fn extra_control_exemptions_use_received_codes_and_two_rows_back() {
    let params = ScoringParameters::default();
    for (current, previous, two_back, expected) in [
        (31, None, None, 600),
        (99, None, None, 0),
        (999, None, None, 0),
        (31, Some(99), Some(31), 0),
        (31, Some(999), Some(31), 0),
        (31, Some(99), Some(32), 600),
        (31, Some(90), Some(31), 600),
    ] {
        assert_eq!(
            extra_punch_penalty(current, previous, two_back, &params).unwrap(),
            expected
        );
    }
}

#[test]
fn lateness_signed_division_and_literal_jat_anomaly() {
    for (difference, expected) in [(-3600, false), (-3659, false), (-3660, true), (3660, false)] {
        assert_eq!(
            cumulative_lateness_exceeded(difference, 60).unwrap(),
            expected
        );
    }
    assert!(!cumulative_lateness_exceeded(-59, 0).unwrap());
    assert!(cumulative_lateness_exceeded(-60, 0).unwrap());
    for difference in [-3660, 3660] {
        assert!(finish_section_limit_exceeded(difference, 60).unwrap());
        assert!(!jat_section_limit_exceeded_literal(difference, 60).unwrap());
    }
    assert!(!finish_section_limit_exceeded(i64::MIN, 0).unwrap());
    assert!(!jat_section_limit_exceeded_literal(i64::MIN, 60).unwrap());
    assert!(cumulative_lateness_exceeded(0, -1).is_err());
}

#[test]
fn unsupported_station_names_and_rules_are_errors() {
    assert_eq!(StationKind::from_legacy(" JAT ").unwrap(), StationKind::Jat);
    assert!(StationKind::from_legacy("JA").is_err());
    assert!(StationKind::from_legacy("JAT-erikois").is_err());
    assert!(Rule::from_legacy(8).is_err());
    assert!(Rule::from_legacy(-1).is_err());
}

#[test]
fn legacy_duplicate_class_codes_do_not_choose_a_route_by_file_order() {
    use as2026::legacy::{Encoding, parse_basic, parse_document};

    let input = b"PE;1;Harjoitus;0;Seura;2;15.10.2026;0;1;\r\n\
H;Harjoitus;1;1;15.00.00;1;0;60;0;0;\r\n\
H   ;Toinen rata;2;1;15.00.00;1;0;60;0;0;\r\n";
    let document = parse_document(input, Encoding::Windows1252).unwrap();
    let error = parse_basic(&document).unwrap_err().to_string();
    assert!(error.contains("toistuva luokkakoodi"));
    assert!(error.contains("Rivi 3"));
    assert_eq!(document.raw, input, "ristiriita ei muuta lähdeaineistoa");
}

#[test]
fn legacy_class_codes_with_a_shared_prefix_stay_distinct() {
    use as2026::legacy::{Encoding, parse_basic, parse_document};

    let input = b"PE;1;Harjoitus;0;Seura;2;15.10.2026;0;1;\n\
H;Harjoitus;1;1;15.00.00;1;0;60;0;0;\n\
HH;Toinen rata;2;1;15.00.00;1;0;60;0;0;\n";
    let settings = parse_basic(&parse_document(input, Encoding::Windows1252).unwrap()).unwrap();
    assert_eq!(settings.classes[0].code, "H");
    assert_eq!(settings.classes[0].route, "1");
    assert_eq!(settings.classes[1].code, "HH");
    assert_eq!(settings.classes[1].route, "2");
}

#[test]
fn ghidra_m02_m03_shared_ula_keeps_greedy_match_precedence() {
    let route = [
        station(1, StationKind::Jat, 31),
        station(2, StationKind::Ula, 31),
    ];
    let result = match_route(&route, &punches(&[31, 31]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        result.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::Matched, MatchKind::Matched]
    );
    assert_eq!(result[1].punch.unwrap().elapsed_seconds, 200);
    let shared = match_route(&route, &punches(&[31]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        shared.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::Matched, MatchKind::SharedUla]
    );
    assert_eq!(shared[1].punch.unwrap().elapsed_seconds, 100);
}

#[test]
fn ghidra_m04_shared_ula_can_reuse_empty_anchor_after_missing_predecessor() {
    let route = [
        station(1, StationKind::At, 31),
        station(2, StationKind::Ula, 31),
    ];
    let result = match_route(&route, &punches(&[41]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        result.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::Missing, MatchKind::SharedUla, MatchKind::Extra]
    );
    assert_eq!(result[1].punch.unwrap().elapsed_seconds, 0);
    assert_eq!(result[2].punch.unwrap().code, 41);
}

#[test]
fn ghidra_m05_lookahead_is_greedy_not_optimal_alignment() {
    let route = [
        station(1, StationKind::At, 31),
        station(2, StationKind::At, 32),
    ];
    let result = match_route(&route, &punches(&[32, 31]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        result.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::Extra, MatchKind::Matched, MatchKind::Missing]
    );
    assert_eq!(result[0].punch.unwrap().elapsed_seconds, 100);
    assert_eq!(result[1].punch.unwrap().elapsed_seconds, 200);
}

#[test]
fn ghidra_m06_m07_finish_does_not_stop_matching_remaining_punches() {
    let manual = [
        station(1, StationKind::Start, 0),
        station(2, StationKind::Finish, 0),
    ];
    let rows = match_route(&manual, &punches(&[31]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [
            MatchKind::ManualStart,
            MatchKind::ManualFinish,
            MatchKind::Extra
        ]
    );
    assert_eq!(rows[1].clock.unwrap(), clock(0, 0, 0));
    assert!(rows[1].punch.is_none());
    let stamped = [
        station(1, StationKind::Start, 10),
        station(2, StationKind::Finish, 20),
    ];
    let rows = match_route(&stamped, &punches(&[10, 20, 77]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::Matched, MatchKind::Matched, MatchKind::Extra]
    );
    assert_eq!(rows[2].punch.unwrap().code, 77);
}

#[test]
fn ghidra_m09_code_zero_never_matches_an_expected_zero() {
    let route = [station(1, StationKind::Rt, 0)];
    let rows = match_route(&route, &punches(&[0]), clock(12, 0, 0)).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.kind).collect::<Vec<_>>(),
        [MatchKind::RouteOnly, MatchKind::Extra]
    );
    assert_eq!(rows[1].punch.unwrap().code, 0);
}
