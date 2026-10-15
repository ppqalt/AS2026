use as2026::{StationKind, ranking::*};

fn competitor(id: usize, total: i64) -> CompetitorInput {
    CompetitorInput {
        source_index: id,
        number: id as i64 + 1,
        class_code: "A".into(),
        route_code: "1".into(),
        status: Status::Classified,
        total_penalty: total,
        start_hour: 10,
        driver_licence: "12345".into(),
        navigator_licence: "67890".into(),
        tie_rows: None,
        route_errors: None,
    }
}

fn input(competitors: Vec<CompetitorInput>) -> RankingInput {
    RankingInput {
        competitors,
        classes: vec![ClassDefinition {
            code: "A".into(),
            np_enabled: true,
            series_method: 1,
        }],
        ..RankingInput::default()
    }
}

fn entry(group: &GroupResult, id: usize) -> &EntryResult {
    group
        .entries
        .iter()
        .find(|entry| entry.source_index == id)
        .unwrap()
}

fn rows(values: &[i64]) -> Vec<TieRow> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| TieRow {
            position: index as u32 + 1,
            station_key: format!("AT:{}", index + 1),
            kind: StationKind::At,
            signed_time_penalty: *value,
        })
        .collect()
}

fn tie_data(competitor: &mut CompetitorInput, values: &[i64]) {
    competitor.tie_rows = Some(rows(values));
    competitor.route_errors = Some(RouteErrors {
        missing_count: 0,
        missing_sum: 0,
        extra_count: 0,
        extra_sum: 0,
    });
}

#[test]
fn default_ties_are_shared_with_gaps_and_deterministic_display() {
    let mut first = competitor(0, 10);
    first.number = 20;
    let mut second = competitor(1, 10);
    second.number = 10;
    let result = rank(&input(vec![first, second, competitor(2, 20)])).unwrap();
    let group = &result.groups[0];
    assert_eq!(
        group
            .entries
            .iter()
            .map(|entry| entry.primary_position)
            .collect::<Vec<_>>(),
        [1, 1, 3]
    );
    assert_eq!(
        group
            .entries
            .iter()
            .map(|entry| entry.number)
            .collect::<Vec<_>>(),
        [10, 20, 3]
    );
    assert_eq!(entry(group, 0).series_points, 100);
    assert_eq!(entry(group, 1).series_points, 100);
    assert_eq!(entry(group, 2).series_points, 93);
    assert!(result.diagnostics.iter().any(|d| d.kind
        == DiagnosticKind::SharedTie {
            reason: TieReason::DefaultPolicy
        }));
}

#[test]
fn status_filtering_sentinels_and_starter_count_include_all_source_entries() {
    let statuses = [
        "TRK", "KESK", "PER", "HYL", "SULJ", "ILM", "EILÄ", "", "TRK ",
    ];
    let mut entries: Vec<_> = statuses
        .iter()
        .enumerate()
        .map(|(id, status)| {
            let mut entry = competitor(id, -(id as i64));
            entry.status = Status::from_legacy(status);
            entry
        })
        .collect();
    entries[0].number = 0;
    entries[1].start_hour = 0;
    let report = rank(&input(entries)).unwrap();
    let group = &report.groups[0];
    assert_eq!(group.entries.len(), 6);
    assert_eq!(report.omitted_source_indices, [6, 7, 8]);
    assert_eq!(
        group.statistics,
        ClassStatistics {
            all_entries: 9,
            nonpositive_numbers: 1,
            start_hour_zero: 1,
            registered: 1,
            classified: 1,
            disqualified: 1,
            interrupted_or_excluded: 2,
            cancelled: 1,
            did_not_start: 1,
            unresolved: 3,
        }
    );
    assert_eq!(group.statistics.starter_count(), 7);
    for (id, sentinel) in [
        (0, 1),
        (1, 25000),
        (2, 25001),
        (3, 25002),
        (4, 25002),
        (5, 25003),
    ] {
        assert_eq!(entry(group, id).primary_position, sentinel);
    }
    assert!(
        group
            .entries
            .iter()
            .skip(1)
            .all(|entry| entry.series_points == 0)
    );
}

#[test]
fn nonfinishers_affect_np_denominator_but_not_numeric_places() {
    let mut entries = vec![competitor(0, 100)];
    for (id, status) in ["KESK", "HYL", "SULJ", "PER", "EILÄ"].iter().enumerate() {
        let mut entry = competitor(id + 1, 0);
        entry.status = Status::from_legacy(status);
        entries.push(entry);
    }
    let result = rank(&input(entries)).unwrap();
    assert!(!result.groups[0].provisional);
    assert_eq!(result.groups[0].statistics.starter_count(), 4);
    assert_eq!(entry(&result.groups[0], 0).primary_position, 1);
    assert_eq!(entry(&result.groups[0], 0).table_points, 2);
}

#[test]
fn np_threshold_boundaries_and_literal_99_cap_are_preserved() {
    assert_eq!(
        (table_points(1, 0), table_points(1, 1), table_points(1, 2)),
        (0, 1, 1)
    );
    assert_eq!((table_points(1, 3), table_points(1, 5)), (2, 2));
    assert_eq!((table_points(1, 6), table_points(1, 8)), (3, 3));
    assert_eq!(
        (
            table_points(1, 9),
            table_points(1, 99),
            table_points(1, 100)
        ),
        (4, 4, 0)
    );
    assert_eq!(table_points(10, 48), 3);
    assert_eq!(table_points(24, 59), 1);
    assert_eq!(table_points(24, 99), 1);
    assert_eq!(table_points(25, 1), 0);
    assert_eq!(table_points(25000, 1), 0);
    assert_eq!(table_points(0, 0), 0);
    assert_eq!(table_points(0, 1), 0);
}

#[test]
fn every_recovered_np_table_cell_matches_extracted_constants() {
    // Independent fixture copied from the PE extraction, not generated by the
    // implementation. Both sides of every distinct threshold are exercised.
    let thresholds = [
        [0, 0, 0, 0, 0],
        [0, 2, 5, 8, 99],
        [2, 5, 8, 15, 99],
        [5, 8, 12, 24, 99],
        [8, 10, 15, 34, 99],
        [10, 12, 22, 44, 99],
        [12, 18, 27, 54, 99],
        [15, 20, 32, 99, 99],
        [18, 24, 37, 99, 99],
        [20, 28, 42, 99, 99],
        [23, 31, 47, 99, 99],
        [26, 34, 52, 99, 99],
        [29, 38, 57, 99, 99],
        [31, 41, 99, 99, 99],
        [33, 44, 99, 99, 99],
        [35, 48, 99, 99, 99],
        [37, 51, 99, 99, 99],
        [40, 54, 99, 99, 99],
        [43, 58, 99, 99, 99],
        [46, 99, 99, 99, 99],
        [48, 99, 99, 99, 99],
        [51, 99, 99, 99, 99],
        [53, 99, 99, 99, 99],
        [56, 99, 99, 99, 99],
        [58, 99, 99, 99, 99],
    ];
    for (place, row) in thresholds.iter().enumerate() {
        for starters in 0..=101 {
            let expected = row
                .iter()
                .position(|threshold| starters <= *threshold)
                .unwrap_or(0) as u8;
            assert_eq!(
                table_points(place as u32, starters),
                expected,
                "place {place}, starters {starters}"
            );
        }
    }
}

#[test]
fn series_schedule_boundaries_and_custom_points() {
    for method in [1, 5] {
        for (place, points) in [(1, 100), (2, 96), (3, 93), (4, 91), (94, 1), (95, 0)] {
            assert_eq!(series_points(place, method, &[]), points);
        }
    }
    for method in [2, 6] {
        for (place, points) in [(1, 60), (2, 58), (59, 1), (60, 0)] {
            assert_eq!(series_points(place, method, &[]), points);
        }
    }
    for (place, points) in [(1, 16), (2, 14), (15, 1), (16, 0)] {
        assert_eq!(series_points(place, 3, &[]), points);
    }
    assert_eq!(series_points(2, 4, &[200, 170]), 170);
    assert_eq!(series_points(3, 4, &[200, 170]), 0);
    assert_eq!(series_points(0, 4, &[200]), 0);
    assert_eq!(series_points(1001, 4, &vec![1; 1001]), 0);
    assert_eq!(series_points(1, 0, &[]), 0);
    assert_eq!(series_points(1, 7, &[]), 0);
}

#[test]
fn licence_scan_order_exclusion_and_eligible_positions_are_separate() {
    assert!(licence_excludes_series(" 123x "));
    assert!(!licence_excludes_series("x123"));
    assert!(licence_excludes_series("X12X"));
    assert!(!licence_excludes_series("X12"));
    assert!(!licence_excludes_series(""));
    let mut excluded = competitor(0, 1);
    excluded.navigator_licence = "12X".into();
    let report = rank(&input(vec![excluded, competitor(1, 2), competitor(2, 3)])).unwrap();
    let group = &report.groups[0];
    assert_eq!(entry(group, 0).primary_position, 1);
    assert_eq!(entry(group, 0).series_eligible_position, 0);
    assert_eq!(entry(group, 0).series_points, 0);
    assert_eq!(entry(group, 1).primary_position, 2);
    assert_eq!(entry(group, 1).series_eligible_position, 1);
    assert_eq!(entry(group, 1).series_points, 100);
    assert_eq!(entry(group, 2).series_eligible_position, 2);
}

#[test]
fn combined_membership_statistics_route_groups_and_method_6_inheritance() {
    let mut b = competitor(1, 1);
    b.class_code = "B".into();
    let mut c = competitor(2, 0);
    c.class_code = "C".into();
    c.route_code = "2".into();
    let mut input = input(vec![competitor(0, 20), b, c]);
    input.classes.push(ClassDefinition {
        code: "B".into(),
        np_enabled: false,
        series_method: 3,
    });
    input.classes.push(ClassDefinition {
        code: "C".into(),
        np_enabled: false,
        series_method: 2,
    });
    input.combined_classes.push(CombinedClassDefinition {
        code: "AB".into(),
        class_codes: vec!["A".into(), "B".into()],
        np_enabled: true,
        series_method: 6,
    });
    let report = rank(&input).unwrap();
    let combined = &report.groups[3];
    assert_eq!(combined.statistics.all_entries, 2);
    assert_eq!(combined.entries.len(), 2);
    assert_eq!(entry(combined, 0).primary_position, 2);
    assert_eq!(entry(combined, 0).series_points, 100); // ordinary A's winner value
    assert_eq!(entry(combined, 1).series_points, 60); // combined direct exceeds B's 16
    assert_eq!(report.routes.len(), 2);
    assert_eq!(
        report.routes[0]
            .entries
            .iter()
            .map(|entry| (entry.source_index, entry.primary_position))
            .collect::<Vec<_>>(),
        [(1, 1), (0, 2)]
    );
    assert_eq!(report.routes[1].entries[0].source_index, 2);
    assert_eq!(report.routes[1].entries[0].primary_position, 1);
}

#[test]
fn method_5_inherits_custom_ordinary_points_but_method_6_ordinary_does_not() {
    let mut input = input(vec![competitor(0, 0)]);
    input.classes[0].series_method = 4;
    input.custom_series_points = vec![200];
    input.combined_classes.push(CombinedClassDefinition {
        code: "all".into(),
        class_codes: vec!["A".into()],
        np_enabled: false,
        series_method: 5,
    });
    let result = rank(&input).unwrap();
    assert_eq!(result.groups[1].entries[0].series_points, 200);
    input.classes[0].series_method = 6;
    let result = rank(&input).unwrap();
    assert_eq!(result.groups[0].entries[0].series_points, 60);
}

#[test]
fn explicit_tie_break_changes_primary_but_never_series_eligible_positions() {
    let mut a = competitor(0, 20);
    let mut b = competitor(1, 20);
    tie_data(&mut a, &[0, 0, 20]);
    tie_data(&mut b, &[10, 10, 0]);
    let mut input = input(vec![b, a, competitor(2, 30)]);
    input.tie_policy = TiePolicy::BestRowsWithExplicitCounts;
    let report = rank(&input).unwrap();
    let group = &report.groups[0];
    assert_eq!(entry(group, 0).primary_position, 1);
    assert_eq!(entry(group, 1).primary_position, 2);
    assert_eq!(entry(group, 2).primary_position, 3);
    assert_eq!(entry(group, 0).series_eligible_position, 1);
    assert_eq!(entry(group, 1).series_eligible_position, 1);
    assert_eq!(entry(group, 0).series_points, 100);
    assert_eq!(entry(group, 1).series_points, 100);
    assert_eq!(entry(group, 0).table_points, 2);
    assert_eq!(entry(group, 1).table_points, 1);
    assert_eq!(report.routes[0].entries[0].source_index, 0);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::ExplicitTiePolicyApplied)
    );
}

#[test]
fn route_errors_affect_explicit_tie_scores_and_equal_best_rows_stay_shared() {
    let mut a = competitor(0, 20);
    let mut b = competitor(1, 20);
    tie_data(&mut a, &[0, 20]);
    tie_data(&mut b, &[10, 10]);
    let mut input = input(vec![a, b]);
    input.tie_policy = TiePolicy::BestRowsWithExplicitCounts;
    let report = rank(&input).unwrap();
    assert_eq!(
        report.groups[0]
            .entries
            .iter()
            .map(|entry| entry.primary_position)
            .collect::<Vec<_>>(),
        [1, 1]
    );
    input.competitors[0].route_errors = Some(RouteErrors {
        missing_count: 1,
        missing_sum: 5000,
        extra_count: 1,
        extra_sum: 1000,
    });
    let report = rank(&input).unwrap();
    assert_eq!(entry(&report.groups[0], 1).primary_position, 1);
    assert_eq!(entry(&report.groups[0], 0).primary_position, 2);
}

#[test]
fn incomplete_unequal_or_misaligned_tie_rows_are_never_guessed() {
    let mut a = competitor(0, 20);
    let mut b = competitor(1, 20);
    tie_data(&mut a, &[0, 0]);
    tie_data(&mut b, &[10, 10]);
    let mut input = input(vec![a, b]);
    input.tie_policy = TiePolicy::BestRowsWithExplicitCounts;
    let check = |input: &RankingInput, reason: TieReason| {
        let report = rank(input).unwrap();
        assert!(
            report.groups[0]
                .entries
                .iter()
                .all(|entry| entry.primary_position == 1)
        );
        assert!(report.diagnostics.iter().any(|d| d.kind
            == DiagnosticKind::SharedTie {
                reason: reason.clone()
            }));
    };
    input.competitors[1].tie_rows.as_mut().unwrap().pop();
    check(&input, TieReason::UnequalRowCounts);
    input.competitors[1].tie_rows = Some(rows(&[10, 10]));
    input.competitors[1].tie_rows.as_mut().unwrap()[0].station_key = "a different station".into();
    check(&input, TieReason::MisalignedRows);
    input.competitors[1].tie_rows = None;
    check(&input, TieReason::MissingRows);
    input.competitors[1].tie_rows = Some(rows(&[10, 10]));
    input.competitors[1].route_errors = None;
    check(&input, TieReason::MissingRouteErrorCounts);
    input.competitors[1].route_errors = Some(RouteErrors {
        missing_count: 1,
        missing_sum: 0,
        extra_count: 0,
        extra_sum: 0,
    });
    check(&input, TieReason::InconsistentRouteErrorCounts);
}

#[test]
fn twenty_way_tie_keeps_shared_position_even_with_complete_rows() {
    let entries = (0..20)
        .map(|id| {
            let mut c = competitor(id, 0);
            tie_data(&mut c, &[id as i64]);
            c
        })
        .collect();
    let mut input = input(entries);
    input.tie_policy = TiePolicy::BestRowsWithExplicitCounts;
    let report = rank(&input).unwrap();
    assert!(
        report.groups[0]
            .entries
            .iter()
            .all(|entry| entry.primary_position == 1)
    );
    assert!(report.diagnostics.iter().any(|d| d.kind
        == DiagnosticKind::SharedTie {
            reason: TieReason::UnsupportedSize
        }));
}

#[test]
fn provisional_ilm_suppresses_publishable_points_and_inheritance() {
    let mut registered = competitor(1, 0);
    registered.status = Status::Registered;
    let mut input = input(vec![competitor(0, 10), registered]);
    input.combined_classes.push(CombinedClassDefinition {
        code: "all".into(),
        class_codes: vec!["A".into()],
        np_enabled: true,
        series_method: 5,
    });
    let report = rank(&input).unwrap();
    for group in &report.groups {
        assert!(group.provisional);
        assert!(!group.np_enabled);
        assert_eq!(group.series_method, 0);
        let winner = entry(group, 0);
        assert_eq!(winner.primary_position, 1);
        assert_eq!(winner.table_points, 0);
        assert_eq!(winner.series_points, 0);
        assert_eq!(
            winner.suppressed_points,
            Some(SuppressedPoints {
                table: 1,
                series: 100
            })
        );
    }
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::ProvisionalPointsSuppressed)
    );
}

#[test]
fn exact_codes_flag_prefix_collisions_and_unknowns_without_cross_membership() {
    let mut b = competitor(1, 0);
    b.class_code = "AB".into();
    let mut unknown = competitor(2, 0);
    unknown.class_code = "Z".into();
    let mut input = input(vec![competitor(0, 10), b, unknown]);
    input.classes.push(ClassDefinition {
        code: "AB".into(),
        np_enabled: false,
        series_method: 0,
    });
    let report = rank(&input).unwrap();
    assert_eq!(report.groups[0].entries.len(), 1);
    assert_eq!(report.groups[1].entries.len(), 1);
    assert_eq!(report.routes[0].entries.len(), 3);
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| matches!(d.kind, DiagnosticKind::PrefixClassCollision { .. }))
    );
    assert!(
        report
            .diagnostics
            .iter()
            .any(|d| d.kind == DiagnosticKind::UnknownClass { code: "Z".into() })
    );
}

#[test]
fn invalid_identities_and_combined_references_fail_explicitly() {
    let mut input = input(vec![competitor(0, 0), competitor(0, 1)]);
    assert!(rank(&input).is_err());
    input.competitors.pop();
    input.combined_classes.push(CombinedClassDefinition {
        code: "all".into(),
        class_codes: vec!["missing".into()],
        np_enabled: false,
        series_method: 0,
    });
    assert!(rank(&input).is_err());
    input.combined_classes[0].class_codes = vec!["A".into(), "A".into()];
    assert!(rank(&input).is_err());
}

#[test]
fn minimum_signed_penalty_is_handled_without_overflow() {
    let mut a = competitor(0, 10);
    let mut b = competitor(1, 10);
    tie_data(&mut a, &[i64::MIN]);
    tie_data(&mut b, &[0]);
    let mut input = input(vec![a, b]);
    input.tie_policy = TiePolicy::BestRowsWithExplicitCounts;
    let report = rank(&input).unwrap();
    assert_eq!(entry(&report.groups[0], 1).primary_position, 1);
    assert_eq!(entry(&report.groups[0], 0).primary_position, 2);
}

#[test]
fn user_visible_ranking_diagnostics_are_finnish_and_keep_legacy_status_codes() {
    let diagnostic = Diagnostic {
        group: Some("rata:1".into()),
        source_indices: vec![2, 5],
        kind: DiagnosticKind::SharedTie {
            reason: TieReason::MissingRows,
        },
    };
    assert_eq!(
        diagnostic.to_string(),
        "Ryhmä rata:1: Tasapisteissä olevat jakavat sijan: asemarivit puuttuvat (lähdetietueet: 2, 5)"
    );
    assert_eq!(Status::from_legacy("HYL").as_legacy(), "HYL");
    assert_eq!(Status::from_legacy("KESK").as_legacy(), "KESK");
    assert!(
        DiagnosticKind::ProvisionalPointsSuppressed
            .to_string()
            .contains("Nousu- ja sarjapisteitä")
    );
    assert!(
        DiagnosticKind::UnknownStatus {
            status: "ILM?".into()
        }
        .to_string()
        .contains("Tuntematon tarkastustulos")
    );
}
