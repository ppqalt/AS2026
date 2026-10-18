//! Pure ranking and points rules recovered from AS2015A.
//!
//! Class codes are matched exactly. Legacy prefix comparison and uninitialized
//! tie scratch memory are deliberately not reproduced. The default tie policy
//! keeps equal totals shared and reports why. An explicit opt-in policy can use
//! complete, aligned rows and explicit route-error counts. Neither policy claims
//! historical-result parity without real competition fixtures.
//!
//! Entries are displayed by position, total penalties, competitor number, then
//! source index. This deterministic display order is a new policy; it never
//! changes a shared sporting position or series-eligible position.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use crate::{CoreError, StationKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Classified,
    Interrupted,
    Cancelled,
    Disqualified,
    Excluded,
    Registered,
    DidNotStart,
    Unknown(String),
}

impl Status {
    /// Full-string matching, as in prepared-result creation. Trim at an import
    /// boundary only when that format explicitly permits trailing padding.
    pub fn from_legacy(value: &str) -> Self {
        match value {
            "TRK" => Self::Classified,
            "KESK" => Self::Interrupted,
            "PER" => Self::Cancelled,
            "HYL" => Self::Disqualified,
            "SULJ" => Self::Excluded,
            "ILM" => Self::Registered,
            "EILÄ" => Self::DidNotStart,
            other => Self::Unknown(other.to_owned()),
        }
    }

    pub fn as_legacy(&self) -> &str {
        match self {
            Self::Classified => "TRK",
            Self::Interrupted => "KESK",
            Self::Cancelled => "PER",
            Self::Disqualified => "HYL",
            Self::Excluded => "SULJ",
            Self::Registered => "ILM",
            Self::DidNotStart => "EILÄ",
            Self::Unknown(value) => value,
        }
    }

    /// None means omitted from prepared results; zero means numeric position.
    pub const fn sentinel(&self) -> Option<u32> {
        match self {
            Self::Classified => Some(0),
            Self::Interrupted => Some(25_000),
            Self::Cancelled => Some(25_001),
            Self::Disqualified | Self::Excluded => Some(25_002),
            Self::Registered => Some(25_003),
            Self::DidNotStart | Self::Unknown(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TieRow {
    /// Stored one-based output row, not a newly aligned route position.
    pub position: u32,
    /// Stable station identity (for example route sequence and station code).
    pub station_key: String,
    pub kind: StationKind,
    pub signed_time_penalty: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteErrors {
    pub missing_count: u32,
    pub missing_sum: i64,
    pub extra_count: u32,
    pub extra_sum: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompetitorInput {
    /// Unique identity within this input, including entries omitted by status.
    pub source_index: usize,
    pub number: i64,
    pub class_code: String,
    pub route_code: String,
    pub status: Status,
    pub total_penalty: i64,
    pub start_hour: i64,
    pub driver_licence: String,
    pub navigator_licence: String,
    /// None means unavailable; Some(empty) explicitly means zero stored rows.
    pub tie_rows: Option<Vec<TieRow>>,
    pub route_errors: Option<RouteErrors>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassDefinition {
    pub code: String,
    pub np_enabled: bool,
    pub series_method: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedClassDefinition {
    pub code: String,
    pub class_codes: Vec<String>,
    pub np_enabled: bool,
    pub series_method: u8,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TiePolicy {
    #[default]
    KeepSharedAndFlag,
    /// Requires 2..=19 tied entries, identical row identities and explicit error
    /// counts. A known zero missing count replaces the legacy retained scratch.
    /// Primary positions change; series-eligible positions remain shared.
    BestRowsWithExplicitCounts,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RankingInput {
    pub competitors: Vec<CompetitorInput>,
    pub classes: Vec<ClassDefinition>,
    pub combined_classes: Vec<CombinedClassDefinition>,
    /// Position 1 is element 0. Numeric parsing is the caller's responsibility.
    pub custom_series_points: Vec<i64>,
    pub tie_policy: TiePolicy,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ClassStatistics {
    pub all_entries: usize,
    pub nonpositive_numbers: usize,
    pub start_hour_zero: usize,
    pub registered: usize,
    pub classified: usize,
    pub disqualified: usize,
    pub interrupted_or_excluded: usize,
    pub cancelled: usize,
    pub did_not_start: usize,
    /// Legacy column 9: ILM plus any unknown/empty status.
    pub unresolved: usize,
}

impl ClassStatistics {
    pub fn starter_count(&self) -> usize {
        self.all_entries - self.cancelled - self.did_not_start
    }

    fn count(&mut self, entry: &CompetitorInput) {
        self.all_entries += 1;
        self.nonpositive_numbers += usize::from(entry.number <= 0);
        self.start_hour_zero += usize::from(entry.start_hour == 0);
        match entry.status {
            Status::Classified => self.classified += 1,
            Status::Interrupted | Status::Excluded => self.interrupted_or_excluded += 1,
            Status::Cancelled => self.cancelled += 1,
            Status::Disqualified => self.disqualified += 1,
            Status::Registered => {
                self.registered += 1;
                self.unresolved += 1;
            }
            Status::DidNotStart => self.did_not_start += 1,
            Status::Unknown(_) => self.unresolved += 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupKind {
    Ordinary,
    Combined,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SuppressedPoints {
    pub table: u8,
    pub series: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryResult {
    pub source_index: usize,
    pub number: i64,
    pub status: Status,
    /// 1-based sporting position, or the documented 25000..25003 sentinel.
    pub primary_position: u32,
    pub series_eligible_position: u32,
    pub series_excluded: bool,
    /// Computed even if np_enabled is false, except provisional suppression.
    pub table_points: u8,
    pub series_points: i64,
    pub suppressed_points: Option<SuppressedPoints>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupResult {
    pub code: String,
    pub kind: GroupKind,
    pub statistics: ClassStatistics,
    /// Effective output selector, false while provisional.
    pub np_enabled: bool,
    /// Effective method, zero while provisional.
    pub series_method: u8,
    pub provisional: bool,
    pub entries: Vec<EntryResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteEntryResult {
    pub source_index: usize,
    pub number: i64,
    pub status: Status,
    pub primary_position: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteResult {
    pub code: String,
    pub entries: Vec<RouteEntryResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TieReason {
    DefaultPolicy,
    UnsupportedSize,
    MissingRows,
    MissingRouteErrorCounts,
    UnequalRowCounts,
    MisalignedRows,
    InconsistentRouteErrorCounts,
}

impl fmt::Display for TieReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::DefaultPolicy => "oletuksena tasapisteet säilyvät jaettuna sijana",
            Self::UnsupportedSize => {
                "asemakohtainen vertailu tukee vain 2–19 kilpailijan tasapisteitä"
            }
            Self::MissingRows => "asemarivit puuttuvat",
            Self::MissingRouteErrorCounts => "reittivirheiden määrät puuttuvat",
            Self::UnequalRowCounts => "kilpailijoiden asemarivimäärät eroavat toisistaan",
            Self::MisalignedRows => "asemarivit eivät vastaa toisiaan samassa järjestyksessä",
            Self::InconsistentRouteErrorCounts => {
                "reittivirheiden määrät ja pisteet ovat ristiriidassa"
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticKind {
    UnknownClass {
        code: String,
    },
    UnknownStatus {
        status: String,
    },
    /// Exact codes are used; legacy prefix bugs are not emulated.
    PrefixClassCollision {
        other: String,
    },
    ProvisionalPointsSuppressed,
    UnknownSeriesMethod {
        method: u8,
    },
    MissingCustomSeriesSchedule,
    SharedTie {
        reason: TieReason,
    },
    /// Explicit zero/error counts replace the legacy uninitialized scratch.
    ExplicitTiePolicyApplied,
}

impl fmt::Display for DiagnosticKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownClass { code } => write!(f, "Tuntematon luokka {code:?}; kilpailija ei kuulu määriteltyihin luokkiin"),
            Self::UnknownStatus { status } => write!(f, "Tuntematon tarkastustulos {status:?}; tietue jätettiin tulosluettelon ulkopuolelle"),
            Self::PrefixClassCollision { other } => write!(f, "Luokkakoodi on luokan {other:?} koodin etuliite tai päinvastoin; luokat erotetaan koko koodin perusteella"),
            Self::ProvisionalPointsSuppressed => f.write_str("Tulokset ovat alustavat: ILM-tiloja tai tuntemattomia tarkastustuloksia on jäljellä. Nousu- ja sarjapisteitä ei vielä julkaista"),
            Self::UnknownSeriesMethod { method } => write!(f, "Tuntematon sarjapistevalinta {method}; sarjapisteet ovat nolla"),
            Self::MissingCustomSeriesSchedule => f.write_str("Oma sarjapistetaulukko puuttuu; sarjapisteet ovat nolla"),
            Self::SharedTie { reason } => write!(f, "Tasapisteissä olevat jakavat sijan: {reason}"),
            Self::ExplicitTiePolicyApplied => f.write_str("Tasapisteet vertailtiin tarkistetuilla asemariveillä ja reittivirhemäärillä; sarjapistesijat säilyvät ennallaan"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Class/combined group code or "rata:<code>"; None is input-wide.
    pub group: Option<String>,
    pub source_indices: Vec<usize>,
    pub kind: DiagnosticKind,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(group) = &self.group {
            write!(f, "Ryhmä {group}: ")?;
        }
        write!(f, "{}", self.kind)?;
        if !self.source_indices.is_empty() {
            f.write_str(" (lähdetietueet: ")?;
            for (index, source_index) in self.source_indices.iter().enumerate() {
                if index != 0 {
                    f.write_str(", ")?;
                }
                write!(f, "{source_index}")?;
            }
            f.write_str(")")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RankingReport {
    pub tie_policy: TiePolicy,
    pub groups: Vec<GroupResult>,
    pub routes: Vec<RouteResult>,
    pub omitted_source_indices: Vec<usize>,
    pub diagnostics: Vec<Diagnostic>,
}

/// Literal recovered Np threshold function. 99 is a threshold, not infinity.
pub fn table_points(position: u32, starter_count: usize) -> u8 {
    const TABLE: [[usize; 5]; 25] = [
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
    TABLE
        .get(position as usize)
        .and_then(|row| row.iter().position(|&threshold| starter_count <= threshold))
        .map_or(0, |index| index as u8)
}

/// Direct series points only; group inheritance is performed by rank().
pub fn series_points(position: u32, method: u8, custom: &[i64]) -> i64 {
    if !(1..=1000).contains(&position) {
        return 0;
    }
    let place = i64::from(position);
    match method {
        1 | 5 => match position {
            1 => 100,
            2 => 96,
            3 => 93,
            4..=94 => 95 - place,
            _ => 0,
        },
        2 | 6 => match position {
            1 => 60,
            2..=59 => 60 - place,
            _ => 0,
        },
        3 => match position {
            1 => 16,
            2..=15 => 16 - place,
            _ => 0,
        },
        4 => custom.get(position as usize - 1).copied().unwrap_or(0),
        _ => 0,
    }
}

/// Recovered left-to-right scan: X sets exclusion, a later digit clears it.
/// ASCII licence characters are the confirmed domain; other chars are ignored.
pub fn licence_excludes_series(licence: &str) -> bool {
    let mut excluded = false;
    for character in licence.trim().chars() {
        if character.eq_ignore_ascii_case(&'X') {
            excluded = true;
        } else if character.is_ascii_digit() {
            excluded = false;
        }
    }
    excluded
}

fn excluded(entry: &CompetitorInput) -> bool {
    licence_excludes_series(&entry.driver_licence)
        || licence_excludes_series(&entry.navigator_licence)
}

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

/// Computes ordinary, combined and route ranks from all source competitors.
/// Stored totals are accepted as input; this does not silently rescore them.
pub fn rank(input: &RankingInput) -> Result<RankingReport, CoreError> {
    // Numeric positions must never collide with reserved legacy sentinels.
    if input.competitors.len() >= 25_000 {
        return Err(invalid("Tuloslaskennan enimmäismäärä on 24999 kilpailijaa"));
    }
    let mut source_ids = BTreeSet::new();
    for entry in &input.competitors {
        if !source_ids.insert(entry.source_index) {
            return Err(invalid(format!(
                "Toistuva lähdetunniste {}",
                entry.source_index
            )));
        }
    }
    let mut class_codes = BTreeSet::new();
    let mut group_codes = BTreeSet::new();
    for class in &input.classes {
        validate_code(&class.code)?;
        if !class_codes.insert(class.code.as_str()) || !group_codes.insert(class.code.as_str()) {
            return Err(invalid(format!("Toistuva luokka {:?}", class.code)));
        }
    }
    for group in &input.combined_classes {
        validate_code(&group.code)?;
        if !group_codes.insert(group.code.as_str()) {
            return Err(invalid(format!("Toistuva tulosryhmä {:?}", group.code)));
        }
        let mut seen = BTreeSet::new();
        for code in &group.class_codes {
            if !class_codes.contains(code.as_str()) {
                return Err(invalid(format!(
                    "Yhdistetyssä luokassa {} on tuntematon luokka {code}",
                    group.code
                )));
            }
            if !seen.insert(code.as_str()) {
                return Err(invalid(format!(
                    "Luokka {code} toistuu yhdistetyssä luokassa {}",
                    group.code
                )));
            }
        }
    }

    let mut report = RankingReport {
        tie_policy: input.tie_policy,
        ..Default::default()
    };
    let codes: Vec<_> = class_codes.iter().copied().collect();
    for (index, code) in codes.iter().enumerate() {
        for other in &codes[index + 1..] {
            if code.starts_with(other) || other.starts_with(code) {
                report.diagnostics.push(Diagnostic {
                    group: Some((*code).to_owned()),
                    source_indices: Vec::new(),
                    kind: DiagnosticKind::PrefixClassCollision {
                        other: (*other).to_owned(),
                    },
                });
            }
        }
    }
    for entry in &input.competitors {
        if !class_codes.contains(entry.class_code.as_str()) {
            report.diagnostics.push(Diagnostic {
                group: None,
                source_indices: vec![entry.source_index],
                kind: DiagnosticKind::UnknownClass {
                    code: entry.class_code.clone(),
                },
            });
        }
        if let Status::Unknown(ref status) = entry.status {
            report.diagnostics.push(Diagnostic {
                group: None,
                source_indices: vec![entry.source_index],
                kind: DiagnosticKind::UnknownStatus {
                    status: status.clone(),
                },
            });
        }
        if entry.status.sentinel().is_none() {
            report.omitted_source_indices.push(entry.source_index);
        }
    }
    for class in &input.classes {
        let members: Vec<_> = input
            .competitors
            .iter()
            .filter(|entry| entry.class_code == class.code)
            .collect();
        report.groups.push(rank_group(
            &class.code,
            GroupKind::Ordinary,
            class.np_enabled,
            class.series_method,
            &members,
            input,
            &mut report.diagnostics,
        ));
    }
    for group in &input.combined_classes {
        let members: Vec<_> = input
            .competitors
            .iter()
            .filter(|entry| group.class_codes.contains(&entry.class_code))
            .collect();
        report.groups.push(rank_group(
            &group.code,
            GroupKind::Combined,
            group.np_enabled,
            group.series_method,
            &members,
            input,
            &mut report.diagnostics,
        ));
    }
    // Ordinary class values are snapshotted before inheritance. Canonical exact
    // membership gives each competitor one ordinary class, so method 5 in an
    // ordinary class has no extra effect. Provisional groups cannot inherit.
    let ordinary_points: BTreeMap<usize, i64> = report
        .groups
        .iter()
        .filter(|group| group.kind == GroupKind::Ordinary)
        .flat_map(|group| {
            group
                .entries
                .iter()
                .map(|entry| (entry.source_index, entry.series_points))
        })
        .collect();
    for group in &mut report.groups {
        if group.provisional
            || !(group.series_method == 5
                || (group.series_method == 6 && group.kind == GroupKind::Combined))
        {
            continue;
        }
        for entry in &mut group.entries {
            entry.series_points = entry.series_points.max(
                ordinary_points
                    .get(&entry.source_index)
                    .copied()
                    .unwrap_or(0),
            );
        }
    }

    let mut routes: BTreeMap<&str, Vec<&CompetitorInput>> = BTreeMap::new();
    for entry in &input.competitors {
        if entry.status.sentinel().is_some() {
            routes.entry(&entry.route_code).or_default().push(entry);
        }
    }
    for (code, members) in routes {
        let group_label = format!("rata:{code}");
        let mut primary = initial_positions(&members, false);
        apply_ties(
            &group_label,
            &members,
            &mut primary,
            input.tie_policy,
            &mut report.diagnostics,
        );
        let mut entries: Vec<_> = members
            .iter()
            .zip(primary)
            .map(|(entry, primary_position)| RouteEntryResult {
                source_index: entry.source_index,
                number: entry.number,
                status: entry.status.clone(),
                primary_position,
            })
            .collect();
        entries.sort_by_key(|entry| display_key(entry.primary_position, entry.source_index, input));
        report.routes.push(RouteResult {
            code: code.to_owned(),
            entries,
        });
    }
    report.omitted_source_indices.sort_unstable();
    Ok(report)
}

fn validate_code(code: &str) -> Result<(), CoreError> {
    if code.is_empty() || code.trim() != code {
        return Err(invalid(format!(
            "Luokkakoodin on oltava tyhjästä poikkeava ilman reunavälilyöntejä: {code:?}"
        )));
    }
    Ok(())
}

fn rank_group(
    code: &str,
    kind: GroupKind,
    np_enabled: bool,
    method: u8,
    all_members: &[&CompetitorInput],
    input: &RankingInput,
    diagnostics: &mut Vec<Diagnostic>,
) -> GroupResult {
    let mut statistics = ClassStatistics::default();
    for member in all_members {
        statistics.count(member);
    }
    let provisional = statistics.unresolved != 0;
    let members: Vec<_> = all_members
        .iter()
        .copied()
        .filter(|entry| entry.status.sentinel().is_some())
        .collect();
    let mut primary = initial_positions(&members, false);
    let eligible = initial_positions(&members, true);
    apply_ties(code, &members, &mut primary, input.tie_policy, diagnostics);
    if method > 6 {
        diagnostics.push(Diagnostic {
            group: Some(code.to_owned()),
            source_indices: Vec::new(),
            kind: DiagnosticKind::UnknownSeriesMethod { method },
        });
    }
    if method == 4 && input.custom_series_points.is_empty() {
        diagnostics.push(Diagnostic {
            group: Some(code.to_owned()),
            source_indices: Vec::new(),
            kind: DiagnosticKind::MissingCustomSeriesSchedule,
        });
    }
    if provisional {
        diagnostics.push(Diagnostic {
            group: Some(code.to_owned()),
            source_indices: all_members
                .iter()
                .filter(|entry| matches!(entry.status, Status::Registered | Status::Unknown(_)))
                .map(|entry| entry.source_index)
                .collect(),
            kind: DiagnosticKind::ProvisionalPointsSuppressed,
        });
    }
    let mut entries: Vec<_> = members
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let classified = entry.status == Status::Classified;
            let series_excluded = excluded(entry);
            let table = if classified {
                table_points(primary[index], statistics.starter_count())
            } else {
                0
            };
            let series = if classified && !series_excluded {
                series_points(eligible[index], method, &input.custom_series_points)
            } else {
                0
            };
            EntryResult {
                source_index: entry.source_index,
                number: entry.number,
                status: entry.status.clone(),
                primary_position: primary[index],
                series_eligible_position: eligible[index],
                series_excluded,
                table_points: if provisional { 0 } else { table },
                series_points: if provisional { 0 } else { series },
                suppressed_points: provisional.then_some(SuppressedPoints { table, series }),
            }
        })
        .collect();
    entries.sort_by_key(|entry| display_key(entry.primary_position, entry.source_index, input));
    GroupResult {
        code: code.to_owned(),
        kind,
        statistics,
        np_enabled: np_enabled && !provisional,
        series_method: if provisional { 0 } else { method },
        provisional,
        entries,
    }
}

fn display_key(position: u32, source_index: usize, input: &RankingInput) -> (u32, i64, i64, usize) {
    let entry = input
        .competitors
        .iter()
        .find(|entry| entry.source_index == source_index)
        .expect("tulosten lähdetietue on tarkistetussa syötteessä");
    (position, entry.total_penalty, entry.number, source_index)
}

fn initial_positions(members: &[&CompetitorInput], eligible: bool) -> Vec<u32> {
    members
        .iter()
        .map(|entry| {
            let sentinel = entry
                .status
                .sentinel()
                .expect("tietue kuuluu valmisteltuihin tuloksiin");
            if sentinel != 0 {
                return if eligible { 0 } else { sentinel };
            }
            if eligible && excluded(entry) {
                return 0;
            }
            1 + members
                .iter()
                .filter(|peer| {
                    peer.status == Status::Classified
                        && peer.total_penalty < entry.total_penalty
                        && (!eligible || !excluded(peer))
                })
                .count() as u32
        })
        .collect()
}

fn apply_ties(
    group: &str,
    members: &[&CompetitorInput],
    positions: &mut [u32],
    policy: TiePolicy,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut ties: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, entry) in members.iter().enumerate() {
        if entry.status == Status::Classified {
            ties.entry(entry.total_penalty).or_default().push(index);
        }
    }
    for indices in ties.values().filter(|indices| indices.len() > 1) {
        let tied: Vec<_> = indices.iter().map(|&index| members[index]).collect();
        let scores = tie_scores(&tied, policy);
        let kind = match scores {
            Ok(scores) => {
                for (index, score) in indices.iter().zip(&scores) {
                    positions[*index] += scores.iter().filter(|peer| *peer > score).count() as u32;
                }
                DiagnosticKind::ExplicitTiePolicyApplied
            }
            Err(reason) => DiagnosticKind::SharedTie { reason },
        };
        diagnostics.push(Diagnostic {
            group: Some(group.to_owned()),
            source_indices: tied.iter().map(|entry| entry.source_index).collect(),
            kind,
        });
    }
}

fn tie_scores(tied: &[&CompetitorInput], policy: TiePolicy) -> Result<Vec<i64>, TieReason> {
    if policy == TiePolicy::KeepSharedAndFlag {
        return Err(TieReason::DefaultPolicy);
    }
    if !(2..=19).contains(&tied.len()) {
        return Err(TieReason::UnsupportedSize);
    }
    let rows: Vec<_> = tied
        .iter()
        .map(|entry| entry.tie_rows.as_ref().ok_or(TieReason::MissingRows))
        .collect::<Result<_, _>>()?;
    let errors: Vec<_> = tied
        .iter()
        .map(|entry| entry.route_errors.ok_or(TieReason::MissingRouteErrorCounts))
        .collect::<Result<_, _>>()?;
    let first = rows[0];
    if rows.iter().any(|rows| rows.len() != first.len()) {
        return Err(TieReason::UnequalRowCounts);
    }
    if first
        .iter()
        .enumerate()
        .any(|(index, row)| row.position as usize != index + 1 || row.station_key.is_empty())
        || rows.iter().any(|rows| {
            rows.iter().zip(first).any(|(row, reference)| {
                row.position != reference.position
                    || row.station_key != reference.station_key
                    || row.kind != reference.kind
            })
        })
    {
        return Err(TieReason::MisalignedRows);
    }
    if errors.iter().any(|error| {
        (error.missing_sum == 0 && error.missing_count != 0)
            || (error.missing_sum != 0 && error.missing_count == 0)
            || (error.extra_sum != 0 && error.extra_count == 0)
    }) {
        return Err(TieReason::InconsistentRouteErrorCounts);
    }
    Ok(rows
        .iter()
        .zip(errors)
        .map(|(candidate, errors)| {
            let best = candidate
                .iter()
                .enumerate()
                .filter(|(index, row)| {
                    let penalty = row.signed_time_penalty.unsigned_abs();
                    rows.iter().all(|peer| {
                        matches!(
                            peer[*index].kind,
                            StationKind::At | StationKind::Jat | StationKind::Finish
                        ) && penalty <= peer[*index].signed_time_penalty.unsigned_abs()
                    })
                })
                .count() as i64;
            let missing = if errors.missing_sum == 0 {
                0
            } else {
                i64::from(errors.missing_count)
            };
            let extra = if errors.extra_sum == 0 {
                0
            } else {
                i64::from(errors.extra_count)
            };
            best - missing - extra
        })
        .collect())
}
