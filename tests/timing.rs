//! Synteettisiä kokonaispolkuja; ei AS2015A:n tai fyysisen lukijan suoritusta.
use as2026::emit::parse_frame;
use as2026::legacy::{Encoding, parse_document, parse_routes};
use as2026::matching::match_route;
use as2026::scoring::ScoringParameters;
use as2026::timing::*;
use as2026::{ClockTime, CoreError, Punch, RouteStation, Rule, StationKind};

fn clock() -> ClockTime {
    ClockTime::new(12, 0, 0).unwrap()
}
fn station(sequence: u32, kind: StationKind, code: u32, target: i64) -> RouteStation {
    RouteStation {
        sequence,
        kind,
        label: format!("Asema {sequence}"),
        expected_code: code,
        target_seconds: target,
        rule: Rule::Normal,
        section_time_text: String::new(),
    }
}
fn options(last: u32) -> LiveCalculationOptions {
    LiveCalculationOptions {
        official_start: clock(),
        reset_correction_seconds: 120,
        timing_method: TimingMethod::Emit,
        last_punch_elapsed_seconds: last,
        cumulative_limit_minutes: None,
        parameters: ScoringParameters::default(),
        policy: CalculationPolicy::RecoveredLiveV1,
    }
}
fn calculate(
    route: &[RouteStation],
    punches: &[(u32, u32)],
    options: LiveCalculationOptions,
) -> LiveCalculation {
    let punches: Vec<_> = punches
        .iter()
        .map(|&(code, elapsed_seconds)| Punch {
            code,
            elapsed_seconds,
        })
        .collect();
    let rows = match_route(route, &punches, options.official_start).unwrap();
    calculate_live(&rows, &options).unwrap()
}
fn has_diagnostic(result: &LiveCalculation, code: &str) -> bool {
    result.diagnostics.iter().any(|d| d.code == code)
}

#[test]
fn imported_synthetic_route_and_decoded_card_produce_645_with_explicit_policy() {
    let route_document = parse_document(
        include_bytes!("../fixtures/synthetic/Radat.txt"),
        Encoding::Windows1252,
    )
    .unwrap();
    let route = parse_routes(&route_document)
        .unwrap()
        .remove(0)
        .to_core()
        .unwrap();
    let frame = parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    let readout = frame
        .accepted_readout(ClockTime::new(12, 11, 0).unwrap())
        .unwrap();
    let rows = match_route(&route, &readout.interior_punches, clock()).unwrap();
    let before = rows.clone();
    let result = calculate_live(&rows, &options(readout.last_punch_elapsed_seconds)).unwrap();
    assert_eq!(result.totals.time_points, 45);
    assert_eq!(result.totals.other_points, 600);
    assert_eq!(result.totals.total_points, 645);
    assert_eq!(result.rows[1].actual_seconds, 275);
    assert_eq!(result.rows[3].actual_seconds, 600);
    assert_eq!(result.rows[4].actual_seconds, 600);
    assert_eq!(result.rows[4].derived_elapsed_seconds, 720);
    assert_eq!(result.rows[4].source.punch, None);
    assert_eq!(result.finish_clock_seconds, 12 * 3600 + 600);
    assert_eq!(result.status, CalculationStatus::Checked);
    assert_eq!(rows, before, "raaka- ja kohdistustietoja ei saa muuttaa");
    assert!(has_diagnostic(&result, "manual-finish-derived"));
    assert!(has_diagnostic(&result, "manual-start-display-correction"));
    assert!(result.trace.iter().any(|e| e.action == "row-result"));
}

#[test]
fn at_keeps_reference_jat_advances_and_ula_rounds_a_derived_copy() {
    for (rule, rounded) in [
        (Rule::UlaNearestMinute, 780),
        (Rule::UlaFloorMinute, 720),
        (Rule::UlaCeilMinute, 780),
    ] {
        let mut route = vec![
            station(1, StationKind::Start, 0, 0),
            station(2, StationKind::At, 31, 300),
            station(3, StationKind::Jat, 32, 600),
            station(4, StationKind::Ula, 32, 0),
            station(5, StationKind::At, 33, 300),
            station(6, StationKind::Finish, 34, 600),
        ];
        route[3].rule = rule;
        let result = calculate(
            &route,
            &[(31, 420), (32, 750), (33, 1080), (34, 1380)],
            options(1380),
        );
        assert_eq!(result.rows[1].actual_seconds, 300);
        assert_eq!(result.rows[2].actual_seconds, 630);
        assert_eq!(result.rows[3].source.punch.unwrap().elapsed_seconds, 750);
        assert_eq!(result.rows[3].derived_elapsed_seconds, rounded);
        assert_eq!(result.rows[3].actual_seconds, 0);
        assert_eq!(result.rows[4].actual_seconds, 1080 - rounded);
        assert_eq!(result.rows[5].actual_seconds, 1380 - rounded);
    }
}

#[test]
fn missing_jat_target_carries_once_to_next_present_at_not_later_ats() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::Jat, 31, 300),
        station(3, StationKind::At, 32, 300),
        station(4, StationKind::At, 33, 600),
        station(5, StationKind::Finish, 34, 600),
    ];
    let result = calculate(&route, &[(32, 720), (33, 720), (34, 720)], options(720));
    assert_eq!(result.rows[1].other_points, 3600);
    assert_eq!(result.rows[2].target_seconds, 600);
    assert_eq!(result.rows[2].actual_seconds, 600);
    assert_eq!(result.rows[3].target_seconds, 600);
    assert_eq!(result.status, CalculationStatus::Interrupted);
    assert_eq!(result.reason, "*JAT puuttuu");
    assert!(result.trace.iter().any(|e| e.action == "missing-jat-carry"));
}

#[test]
fn present_finish_clears_missing_jat_carry_before_finish_preparation() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::Jat, 31, 300),
        station(3, StationKind::Finish, 32, 300),
    ];
    let result = calculate(&route, &[(32, 720)], options(720));
    assert_eq!(result.rows[2].target_seconds, 300);
    assert_eq!(result.rows[2].actual_seconds, 600);
    assert!(has_diagnostic(
        &result,
        "missing-jat-carry-cleared-at-finish"
    ));
}

#[test]
fn trailing_extra_after_finish_requires_an_explicit_policy() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let punches = vec![
        Punch {
            code: 31,
            elapsed_seconds: 420,
        },
        Punch {
            code: 32,
            elapsed_seconds: 720,
        },
        Punch {
            code: 90,
            elapsed_seconds: 730,
        },
    ];
    let rows = match_route(&route, &punches, clock()).unwrap();
    assert!(matches!(
        calculate_live(&rows, &options(730)),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn missing_penalties_and_terminal_finish_are_explicit() {
    let route = vec![
        station(1, StationKind::Start, 11, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Rt, 32, 0),
        station(4, StationKind::Ula, 33, 0),
        station(5, StationKind::Finish, 34, 300),
    ];
    let result = calculate(&route, &[(11, 0)], options(420));
    assert_eq!(
        result
            .rows
            .iter()
            .map(|r| r.other_points)
            .collect::<Vec<_>>(),
        vec![0, 1800, 600, 3600, 9999]
    );
    assert_eq!(result.totals.other_points, 15999);
    assert_eq!(result.rows[1].note, "AT puuttuu");
    assert_eq!(result.rows[2].note, "rt puuttuu");
    assert_eq!(result.rows[3].note, "JAT puuttuu");
    assert_eq!(result.rows[4].note, "maali puuttuu");
    assert_eq!(result.status, CalculationStatus::Interrupted);
    assert_eq!(result.reason, "*Maali puuttuu");
    assert!(has_diagnostic(&result, "missing-finish-explicit-result"));
}

#[test]
fn missing_start_aborts_with_9999_without_scoring_other_rows() {
    let route = vec![
        station(1, StationKind::Start, 11, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let result = calculate(&route, &[(31, 420), (32, 720)], options(720));
    assert_eq!(result.totals.total_points, 9999);
    assert_eq!(result.rows[1].total_points, 0);
    assert_eq!(result.reason, "lähtö puuttuu");
}

#[test]
fn caps_and_no_early_rules_apply_after_reference_preparation() {
    let mut route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 3000),
        station(3, StationKind::Finish, 32, 3600),
    ];
    let result = calculate(&route, &[(31, 420), (32, 720)], options(720));
    assert_eq!(result.rows[1].signed_time_points, -1800);
    assert_eq!(result.rows[2].signed_time_points, -3600);
    assert_eq!(result.rows[2].note, "max myöhä");
    assert_eq!(
        result.status,
        CalculationStatus::Checked,
        "katto ei muuta kilpailijan tilaa"
    );
    route[1].rule = Rule::NoEarly;
    let result = calculate(&route, &[(31, 420), (32, 720)], options(720));
    assert_eq!(result.rows[1].signed_time_points, 0);
}

#[test]
fn section_limit_and_final_cumulative_assignment_preserve_order() {
    let mut route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::Jat, 31, 300),
        station(3, StationKind::Finish, 32, 300),
    ];
    route[1].section_time_text = "5".into();
    route[2].section_time_text = "5".into();
    let mut o = options(2000);
    o.parameters.section_limit_minutes = 1;
    o.cumulative_limit_minutes = Some(60);
    let result = calculate(&route, &[(31, 1000), (32, 2000)], o.clone());
    assert_eq!(result.cumulative_schedule_difference_seconds, -1280);
    assert_eq!(result.reason, "*jaksomyöhä ylitetty");
    assert!(has_diagnostic(&result, "legacy-jat-limit-comparison"));
    o.cumulative_limit_minutes = Some(1);
    let result = calculate(&route, &[(31, 1000), (32, 2000)], o);
    assert_eq!(result.reason, "*max myöhä ylitetty");
    assert_eq!(
        result.trace.iter().filter(|e| e.action == "status").count(),
        2
    );
}

#[test]
fn cumulative_zero_is_not_disabled_and_early_time_can_cancel_late() {
    let mut route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::Jat, 31, 300),
        station(3, StationKind::Finish, 32, 300),
    ];
    route[1].section_time_text = "5".into();
    route[2].section_time_text = "5".into();
    let mut o = options(720);
    o.cumulative_limit_minutes = Some(0);
    let result = calculate(&route, &[(31, 360), (32, 720)], o.clone());
    assert_eq!(result.cumulative_schedule_difference_seconds, 0);
    assert_eq!(result.status, CalculationStatus::Checked);
    let result = calculate(&route, &[(31, 420), (32, 780)], o);
    assert_eq!(result.cumulative_schedule_difference_seconds, -60);
    assert_eq!(result.reason, "*max myöhä ylitetty");
}

#[test]
fn rule4_missing_jat_and_shared_ula_keep_the_first_pass_times() {
    let mut route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::Jat, 31, 300),
        station(3, StationKind::Ula, 31, 0),
        station(4, StationKind::At, 32, 300),
        station(5, StationKind::Finish, 33, 600),
    ];
    route[1].rule = Rule::Void;
    let result = calculate(&route, &[(32, 720), (33, 1020)], options(1020));
    assert_eq!(result.rows.len(), 5, "mitätöityä riviä ei poisteta");
    assert_eq!(result.rows[1].derived_elapsed_seconds, 420);
    assert_eq!(result.rows[1].actual_seconds, 300);
    assert_eq!(result.rows[1].total_points, 0);
    assert_eq!(result.rows[2].derived_elapsed_seconds, 420);
    assert_eq!(result.rows[2].received_code, 0);
    assert_eq!(result.rows[2].effective_rule, Rule::Void);
    assert_eq!(result.rows[2].source.punch.unwrap().elapsed_seconds, 0);
    assert_eq!(
        result.rows[3].actual_seconds, 720,
        "1. kierroksen ULA-viite oli 0; 2. kierrosta ei ajeta uudelleen"
    );
    assert!(has_diagnostic(&result, "rule4-ordered-second-pass"));
    assert!(result.trace.iter().any(|e| e.action == "rule4-jat-ula"));
}

#[test]
fn missing_manual_interiors_are_not_charged_as_missing_punches() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 0, 300),
        station(3, StationKind::Finish, 31, 600),
    ];
    let result = calculate(&route, &[(31, 720)], options(720));
    assert_eq!(result.rows[1].other_points, 0);
    assert!(has_diagnostic(&result, "missing-manual-row-unscored"));
}

#[test]
fn extra_99_and_repeat_after_99_are_exempt_with_output_row_adjacency() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let result = calculate(
        &route,
        &[(31, 420), (99, 430), (31, 440), (32, 720)],
        options(720),
    );
    assert_eq!(result.rows.len(), 5);
    assert_eq!(result.rows[2].other_points, 0);
    assert_eq!(result.rows[3].other_points, 0);
}

#[test]
fn unsupported_modes_ambiguous_section_text_and_overflow_fail_explicitly() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let punches = vec![
        Punch {
            code: 31,
            elapsed_seconds: 420,
        },
        Punch {
            code: 32,
            elapsed_seconds: 720,
        },
    ];
    let rows = match_route(&route, &punches, clock()).unwrap();
    let mut o = options(720);
    o.timing_method = TimingMethod::LegacyMode2;
    assert!(matches!(
        calculate_live(&rows, &o),
        Err(CoreError::Unsupported(_))
    ));
    o.timing_method = TimingMethod::Emit;
    o.reset_correction_seconds = i64::MIN;
    assert!(calculate_live(&rows, &o).is_err());
    let mut bad_rows = rows.clone();
    bad_rows[2].station.as_mut().unwrap().section_time_text = "1 2".into();
    assert!(matches!(
        calculate_live(&bad_rows, &options(720)),
        Err(CoreError::Unsupported(_))
    ));
    bad_rows[2].station.as_mut().unwrap().section_time_text = "1,5".into();
    let result = calculate_live(&bad_rows, &options(720)).unwrap();
    assert!(has_diagnostic(&result, "ignored-section-text"));
}

#[test]
fn manual_clock_live_method_is_distinct_from_stored_recheck_and_midnight_is_explicit() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let mut o = options(600);
    o.timing_method = TimingMethod::ManualClock;
    o.official_start = ClockTime::new(23, 55, 0).unwrap();
    let result = calculate(&route, &[(31, 300), (32, 600)], o);
    assert_eq!(result.rows[1].actual_seconds, 300);
    assert_eq!(result.finish_clock_seconds, 86700);
    assert_eq!(result.rows[2].clock_seconds, Some(86700));
    assert_eq!(result.rows[0].derived_elapsed_seconds, 0);
}

#[test]
fn static_legacy_manual_finish_keeps_zero_actual_and_calculates_2415() {
    let route_document = parse_document(
        include_bytes!("../fixtures/synthetic/Radat.txt"),
        Encoding::Windows1252,
    )
    .unwrap();
    let route = parse_routes(&route_document)
        .unwrap()
        .remove(0)
        .to_core()
        .unwrap();
    let frame = parse_frame(include_bytes!("../fixtures/synthetic/emit-frame.raw")).unwrap();
    let readout = frame
        .accepted_readout(ClockTime::new(12, 11, 0).unwrap())
        .unwrap();
    let rows = match_route(&route, &readout.interior_punches, clock()).unwrap();
    let mut o = options(readout.last_punch_elapsed_seconds);
    o.policy = CalculationPolicy::RecoveredAs2015aStaticV2;
    let result = calculate_live(&rows, &o).unwrap();
    assert_eq!(result.rows[4].derived_elapsed_seconds, 0);
    assert_eq!(result.rows[4].actual_seconds, 0);
    assert_eq!(result.rows[4].clock_seconds, Some(43_800));
    assert_eq!(result.rows[4].signed_time_points, 1770);
    assert_eq!(result.totals.time_points, 1815);
    assert_eq!(result.totals.other_points, 600);
    assert_eq!(result.totals.total_points, 2415);
    assert_eq!(result.status, CalculationStatus::Checked);
    assert!(has_diagnostic(&result, "manual-finish-legacy-order"));
}

#[test]
fn static_live_method_two_uses_emit_anchoring_but_not_stored_recheck() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let mut o = options(720);
    o.policy = CalculationPolicy::RecoveredAs2015aStaticV2;
    o.timing_method = TimingMethod::LegacyMode2;
    let result = calculate(&route, &[(31, 420), (32, 720)], o);
    assert_eq!(result.rows[0].derived_elapsed_seconds, 120);
    assert_eq!(result.rows[1].actual_seconds, 300);
    assert_eq!(result.rows[2].actual_seconds, 600);
    assert_eq!(result.totals.total_points, 0);
    assert_eq!(result.finish_clock_seconds, 43_800);
}

#[test]
fn static_section_conversion_uses_numeric_prefix_not_joined_digits() {
    let route = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 31, 300),
        station(3, StationKind::Finish, 32, 600),
    ];
    let mut rows = match_route(
        &route,
        &[
            Punch {
                code: 31,
                elapsed_seconds: 420,
            },
            Punch {
                code: 32,
                elapsed_seconds: 720,
            },
        ],
        clock(),
    )
    .unwrap();
    rows[2].station.as_mut().unwrap().section_time_text = "1 2".into();
    let mut o = options(720);
    o.policy = CalculationPolicy::RecoveredAs2015aStaticV2;
    let result = calculate_live(&rows, &o).unwrap();
    assert_eq!(result.cumulative_schedule_difference_seconds, -540);
}

#[test]
fn stored_timing_method_zero_recovers_midnight_rollover() {
    let input = StoredTimingInput {
        official_start: ClockTime::new(23, 58, 0).unwrap(),
        stored_finish_clock_seconds: 120,
        last_row_elapsed_seconds: 720,
        finish_expected_code: 0,
        reset_correction_seconds: 45,
        timing_method: TimingMethod::ManualClock,
    };
    let prepared = prepare_stored_timing(&input).unwrap();
    assert_eq!(prepared.start_elapsed_seconds, 0);
    assert_eq!(prepared.last_row_elapsed_seconds, 240);
    assert_eq!(prepared.finish_clock_seconds, 86_520);
    assert_eq!(prepared.total_duration_seconds, 240);
    assert_eq!(prepared.effective_reset_correction_seconds, 45);
    let mut invalid = input;
    invalid.stored_finish_clock_seconds = 0;
    assert!(matches!(
        prepare_stored_timing(&invalid),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn stored_timing_method_two_uses_saved_clock_and_threshold_minus_20000() {
    let mut input = StoredTimingInput {
        official_start: ClockTime::new(12, 0, 0).unwrap(),
        stored_finish_clock_seconds: 43_800,
        last_row_elapsed_seconds: 720,
        finish_expected_code: 32,
        reset_correction_seconds: 120,
        timing_method: TimingMethod::LegacyMode2,
    };
    let prepared = prepare_stored_timing(&input).unwrap();
    assert_eq!(prepared.start_elapsed_seconds, 600);
    assert_eq!(prepared.effective_reset_correction_seconds, 600);
    assert_eq!(prepared.last_row_elapsed_seconds, 720);
    assert_eq!(prepared.total_duration_seconds, 600);
    input.official_start = ClockTime::new(23, 58, 0).unwrap();
    input.stored_finish_clock_seconds = 0;
    let wrapped = prepare_stored_timing(&input).unwrap();
    assert_eq!(wrapped.start_elapsed_seconds, 120);
    assert_eq!(wrapped.total_duration_seconds, 120);
    assert_eq!(wrapped.finish_clock_seconds, 0);
}

#[test]
fn stored_timing_method_one_reanchors_and_rejects_ambiguous_manual_finish() {
    let mut input = StoredTimingInput {
        official_start: clock(),
        stored_finish_clock_seconds: 10,
        last_row_elapsed_seconds: 720,
        finish_expected_code: 32,
        reset_correction_seconds: 120,
        timing_method: TimingMethod::Emit,
    };
    let prepared = prepare_stored_timing(&input).unwrap();
    assert_eq!(prepared.start_elapsed_seconds, 120);
    assert_eq!(prepared.finish_clock_seconds, 43_800);
    assert_eq!(prepared.total_duration_seconds, 600);
    input.finish_expected_code = 0;
    assert!(matches!(
        prepare_stored_timing(&input),
        Err(CoreError::Unsupported(_))
    ));
    input.timing_method = TimingMethod::LegacyMode2;
    assert!(matches!(
        prepare_stored_timing(&input),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn static_policy_does_not_publish_stale_legacy_early_return_scores() {
    let missing_start = vec![
        station(1, StationKind::Start, 31, 0),
        station(2, StationKind::At, 32, 300),
        station(3, StationKind::Finish, 33, 600),
    ];
    let mut o = options(720);
    o.policy = CalculationPolicy::RecoveredAs2015aStaticV2;
    let punches = vec![
        Punch {
            code: 32,
            elapsed_seconds: 420,
        },
        Punch {
            code: 33,
            elapsed_seconds: 720,
        },
    ];
    let rows = match_route(&missing_start, &punches, clock()).unwrap();
    assert!(matches!(
        calculate_live(&rows, &o),
        Err(CoreError::Unsupported(_))
    ));
    let missing_finish = vec![
        station(1, StationKind::Start, 0, 0),
        station(2, StationKind::At, 32, 300),
        station(3, StationKind::Finish, 33, 600),
    ];
    let rows = match_route(
        &missing_finish,
        &[Punch {
            code: 32,
            elapsed_seconds: 420,
        }],
        clock(),
    )
    .unwrap();
    assert!(matches!(
        calculate_live(&rows, &o),
        Err(CoreError::Unsupported(_))
    ));
}

#[test]
fn stored_intermediate_clock_reconstruction_is_reverse_ordered_and_read_only() {
    let prepared = prepare_stored_timing(&StoredTimingInput {
        official_start: ClockTime::new(23, 58, 0).unwrap(),
        stored_finish_clock_seconds: 120,
        last_row_elapsed_seconds: 999,
        finish_expected_code: 0,
        reset_correction_seconds: 0,
        timing_method: TimingMethod::ManualClock,
    })
    .unwrap();
    let source = vec![
        StoredStationTime {
            expected_code: 0,
            elapsed_seconds: 7,
            clock: [23, 58, 0],
        },
        StoredStationTime {
            expected_code: 31,
            elapsed_seconds: 120,
            clock: [0, 0, 0],
        },
        StoredStationTime {
            expected_code: 0,
            elapsed_seconds: 0,
            clock: [0, 1, 0],
        },
        StoredStationTime {
            expected_code: 0,
            elapsed_seconds: 999,
            clock: [0, 2, 0],
        },
    ];
    let result = prepare_stored_station_times(&prepared, &source).unwrap();
    assert_eq!(result.rows[0].elapsed_seconds, 0);
    assert_eq!(result.rows[1].elapsed_seconds, 120);
    assert_eq!(result.rows[1].clock, [24, 0, 0]);
    assert_eq!(result.rows[2].elapsed_seconds, 180);
    assert_eq!(result.rows[2].clock, [0, 1, 0]);
    assert_eq!(result.rows[3].elapsed_seconds, 240);
    assert!(result.warnings.is_empty());
    assert_eq!(source[0].elapsed_seconds, 7);
    assert_eq!(source[3].elapsed_seconds, 999);
}

#[test]
fn stored_manual_clock_zero_and_nonmonotonic_data_are_not_silently_fixed() {
    let prepared = PreparedStoredTiming {
        start_elapsed_seconds: 120,
        last_row_elapsed_seconds: 600,
        effective_reset_correction_seconds: 120,
        finish_clock_seconds: 43_800,
        total_duration_seconds: 600,
    };
    let first = StoredStationTime {
        expected_code: 0,
        elapsed_seconds: 0,
        clock: [12, 0, 0],
    };
    let last = StoredStationTime {
        expected_code: 32,
        elapsed_seconds: 720,
        clock: [12, 10, 0],
    };
    let zero_clock = StoredStationTime {
        expected_code: 0,
        elapsed_seconds: 77,
        clock: [0, 0, 0],
    };
    let result =
        prepare_stored_station_times(&prepared, &[first.clone(), zero_clock, last.clone()])
            .unwrap();
    assert_eq!(result.rows[1].elapsed_seconds, 0);
    let wrong_order = StoredStationTime {
        expected_code: 0,
        elapsed_seconds: 10,
        clock: [12, 12, 0],
    };
    let result = prepare_stored_station_times(&prepared, &[first, wrong_order, last]).unwrap();
    assert_eq!(result.rows[1].elapsed_seconds, 720);
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn stored_recheck_rejects_zero_start_before_any_timing_method() {
    for method in [
        TimingMethod::ManualClock,
        TimingMethod::Emit,
        TimingMethod::LegacyMode2,
    ] {
        let input = StoredTimingInput {
            official_start: ClockTime::new(0, 0, 0).unwrap(),
            stored_finish_clock_seconds: 43_800,
            last_row_elapsed_seconds: 720,
            finish_expected_code: 32,
            reset_correction_seconds: 120,
            timing_method: method,
        };
        assert!(matches!(
            prepare_stored_timing(&input),
            Err(CoreError::Unsupported(_))
        ));
    }
}
