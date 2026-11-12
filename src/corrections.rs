//! Virallinen asematulos säilyy erillään muuttumattomasta kortinlukuhistoriasta.
//! Manuaalisen ajan muuttaminen ei käynnistä laskentaa. Tutki ja Tarkista ovat eri toimintoja.
use crate::{CoreError, RouteStation, legacy::StationResult, scoring};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredResult {
    #[serde(default)]
    pub ilman_emit: bool,
    #[serde(default)]
    pub puuttuvien_at_pisteet: i64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub puuttuvat_aikamerkinnat: Vec<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emit_kortti: Option<u32>,
    #[serde(default)]
    pub laskentatapa: StoredCalculationPolicy,
    #[serde(default)]
    pub hyvaksytty: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kasin_annetut_kellot: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub virheelliset_ajat: Vec<String>,
    pub rata: Vec<StoredRouteStation>,
    pub rivit: Vec<StoredRow>,
    pub aikavirheet: i64,
    pub asemien_muut_virheet: i64,
    pub tila: String,
    pub syy: String,
    pub maalikello_sekuntia: i64,
    pub kesto_sekuntia: i64,
    pub korjaus_sekuntia: i64,
    #[serde(default)]
    pub kasikellot_muuttuneet: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub huomautukset: Vec<String>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredCalculationPolicy {
    #[default]
    #[serde(rename = "as2015a_staattinen")]
    As2015aStatic,
    #[serde(rename = "as2026")]
    As2026Operational,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredRouteStation {
    pub jarjestys: u32,
    pub tyyppi: String,
    pub nimi: String,
    pub emit_koodi: u32,
    pub tavoiteaika_sekuntia: i64,
    pub saanto: i64,
    pub jakso: String,
}
impl From<&RouteStation> for StoredRouteStation {
    fn from(s: &RouteStation) -> Self {
        Self {
            jarjestys: s.sequence,
            tyyppi: s.kind.as_legacy().into(),
            nimi: s.label.clone(),
            emit_koodi: s.expected_code,
            tavoiteaika_sekuntia: s.target_seconds,
            saanto: s.rule as i64,
            jakso: s.section_time_text.clone(),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredRow {
    #[serde(rename = "rivi")]
    pub output_index: u32,
    #[serde(rename = "ratasekvenssi")]
    pub sequence: u32,
    #[serde(rename = "tyyppi")]
    pub station_type: String,
    #[serde(rename = "nimi")]
    pub name: String,
    #[serde(rename = "aikakoordinaatti")]
    pub elapsed_seconds: i64,
    #[serde(rename = "odotettu_koodi")]
    pub expected_code: u32,
    #[serde(rename = "saatu_koodi")]
    pub received_code: u32,
    #[serde(rename = "tavoiteaika")]
    pub target_seconds: i64,
    #[serde(rename = "toteutunut_aika")]
    pub actual_seconds: i64,
    #[serde(rename = "kello")]
    pub clock: [i64; 3],
    #[serde(rename = "saanto")]
    pub rule: i64,
    #[serde(rename = "jakso")]
    pub section_text: String,
    #[serde(rename = "aikavirhe")]
    pub signed_time_penalty: i64,
    #[serde(rename = "muu_virhe")]
    pub other_penalty: i64,
    #[serde(rename = "yhteensa")]
    pub total_penalty: i64,
    #[serde(rename = "huomautus")]
    pub note: String,
}
impl From<&StationResult> for StoredRow {
    fn from(r: &StationResult) -> Self {
        Self {
            output_index: r.output_index,
            sequence: r.sequence,
            station_type: r.station_type.clone(),
            name: r.name.clone(),
            elapsed_seconds: r.elapsed_seconds,
            expected_code: r.expected_code,
            received_code: r.received_code,
            target_seconds: r.target_seconds,
            actual_seconds: r.actual_seconds,
            clock: r.clock,
            rule: r.rule,
            section_text: r.section_text.clone(),
            signed_time_penalty: r.signed_time_penalty,
            other_penalty: r.other_penalty,
            total_penalty: r.total_penalty,
            note: r.note.clone(),
        }
    }
}
impl From<&StoredRow> for StationResult {
    fn from(r: &StoredRow) -> Self {
        Self {
            output_index: r.output_index,
            sequence: r.sequence,
            station_type: r.station_type.clone(),
            name: r.name.clone(),
            elapsed_seconds: r.elapsed_seconds,
            expected_code: r.expected_code,
            received_code: r.received_code,
            target_seconds: r.target_seconds,
            actual_seconds: r.actual_seconds,
            clock: r.clock,
            rule: r.rule,
            section_text: r.section_text.clone(),
            signed_time_penalty: r.signed_time_penalty,
            other_penalty: r.other_penalty,
            total_penalty: r.total_penalty,
            note: r.note.clone(),
        }
    }
}
impl StoredResult {
    pub fn section_difference(&self, row: &StoredRow) -> Option<i64> {
        if !matches!(row.station_type.as_str(), "JAT" | "M")
            || self.puuttuvat_aikamerkinnat.contains(&row.output_index)
            || (row.expected_code != 0 && row.received_code != row.expected_code)
            || self.pending_manual_clocks().contains(&row.output_index)
        {
            return None;
        }
        let operational = self.laskentatapa == StoredCalculationPolicy::As2026Operational;
        crate::timing::section_difference(
            &row.section_text,
            row.actual_seconds,
            if operational {
                crate::timing::CalculationPolicy::As2026OperationalV1
            } else {
                crate::timing::CalculationPolicy::RecoveredAs2015aStaticV2
            },
        )
        .ok()
        .flatten()
        .or_else(|| operational.then_some(row.target_seconds - row.actual_seconds))
    }
    pub fn cumulative_schedule_difference(&self) -> i64 {
        self.rivit
            .iter()
            .filter_map(|r| self.section_difference(r))
            .sum()
    }

    /// Puuttuva manuaalinen aika tunnistetaan syötön alkuperästä: myös 00:00:00 on kelvollinen syöte.
    pub fn pending_manual_clocks(&self) -> Vec<u32> {
        self.rivit
            .iter()
            .filter(|r| {
                r.sequence != 0
                    && r.expected_code == 0
                    && r.station_type != "L"
                    && !self.kasin_annetut_kellot.contains(&r.output_index)
                    && !self.puuttuvat_aikamerkinnat.contains(&r.output_index)
            })
            .map(|r| r.output_index)
            .collect()
    }
    pub fn can_approve(&self) -> bool {
        if self.rivit.is_empty()
            || !matches!(self.tila.as_str(), "TRK" | "KESK")
            || !self.pending_manual_clocks().is_empty()
            || self.kasikellot_muuttuneet
            || !self.virheelliset_ajat.is_empty()
        {
            return false;
        }
        let route = self
            .rata
            .iter()
            .map(|s| {
                Ok(crate::RouteStation {
                    sequence: s.jarjestys,
                    kind: crate::StationKind::from_legacy(&s.tyyppi)?,
                    label: s.nimi.clone(),
                    expected_code: s.emit_koodi,
                    target_seconds: s.tavoiteaika_sekuntia,
                    rule: crate::Rule::from_legacy(s.saanto)?,
                    section_time_text: s.jakso.clone(),
                })
            })
            .collect::<Result<Vec<_>, CoreError>>();
        route.is_ok_and(|route| self.validate_route(&route).is_ok())
    }

    pub fn as_station_results(&self) -> Vec<StationResult> {
        self.rivit.iter().map(StationResult::from).collect()
    }
    pub fn validate_route(&self, route: &[RouteStation]) -> Result<(), CoreError> {
        if self.puuttuvien_at_pisteet < 0 || (!self.ilman_emit && self.puuttuvien_at_pisteet != 0) {
            return Err(CoreError::InvalidInput(
                "Puuttuvan AT:n pisteosuus on virheellinen".into(),
            ));
        }
        let missing = self
            .puuttuvat_aikamerkinnat
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if missing.len() != self.puuttuvat_aikamerkinnat.len()
            || (!self.ilman_emit && !missing.is_empty())
            || missing.iter().any(|id| {
                self.kasin_annetut_kellot.contains(id)
                    || !self.rivit.iter().any(|r| {
                        r.output_index == *id
                            && r.expected_code == 0
                            && matches!(r.station_type.as_str(), "AT" | "JAT" | "ULA" | "M")
                    })
            })
        {
            return Err(CoreError::InvalidInput(
                "Puuttuvien aikamerkintöjen tiedot ovat ristiriitaiset".into(),
            ));
        }
        if self.rata
            != route
                .iter()
                .map(StoredRouteStation::from)
                .collect::<Vec<_>>()
        {
            return Err(CoreError::InvalidInput("Tallennetun tuloksen rata on muuttunut. Tarkista muodostaa asemarivit uudelleen korttiluvusta; käsikorjauksia ei kohdisteta muuttuneeseen rataan automaattisesti".into()));
        }
        if self.kesto_sekuntia < 0 || !matches!(self.tila.as_str(), "TRK" | "KESK" | "ILM") {
            return Err(CoreError::InvalidInput(
                "Tallennetun laskentatuloksen kesto tai tila on virheellinen".into(),
            ));
        }
        let totals = scoring::sum_row_penalties(
            &self
                .rivit
                .iter()
                .map(|r| (r.signed_time_penalty, r.other_penalty))
                .collect::<Vec<_>>(),
        )?;
        if totals.time_points != self.aikavirheet
            || totals.other_points != self.asemien_muut_virheet
        {
            return Err(CoreError::InvalidInput(
                "Tallennetun tuloksen pistesummat eivät vastaa asemarivejä".into(),
            ));
        }
        for row in &self.rivit {
            if row.other_penalty < 0
                || scoring::row_penalty_total(row.signed_time_penalty, row.other_penalty)?
                    != row.total_penalty
            {
                return Err(CoreError::InvalidInput(
                    "Tallennetun asemarivin virhepisteet ovat ristiriitaiset".into(),
                ));
            }
        }
        let mut previous_sequence = 0;
        for (index, row) in self.rivit.iter().enumerate() {
            if row.output_index as usize != index + 1 {
                return Err(CoreError::InvalidInput(
                    "Tallennetun tuloksen rivinumerot ovat virheelliset".into(),
                ));
            }
            if row.sequence == 0 {
                continue;
            }
            let s = route
                .iter()
                .find(|s| s.sequence == row.sequence)
                .ok_or_else(|| {
                    CoreError::InvalidInput("Tallennetun tuloksen asemaa ei löydy radalta".into())
                })?;
            if row.sequence <= previous_sequence
                || row.expected_code != s.expected_code
                || row.station_type != s.kind.as_legacy()
            {
                return Err(CoreError::InvalidInput(
                    "Tallennetun tuloksen asemat eivät vastaa radan järjestystä".into(),
                ));
            }
            previous_sequence = row.sequence;
        }
        if previous_sequence as usize != route.len()
            || self.rivit.iter().filter(|r| r.sequence != 0).count() != route.len()
        {
            return Err(CoreError::InvalidInput(
                "Tallennetusta tuloksesta puuttuu rata-asemia".into(),
            ));
        }
        Ok(())
    }
    fn row_mut(&mut self, index: u32) -> Result<&mut StoredRow, CoreError> {
        self.rivit
            .iter_mut()
            .find(|r| r.output_index == index)
            .ok_or_else(|| CoreError::InvalidInput(format!("Asemariviä {index} ei löydy")))
    }
    /// Muuttaa kellokenttiä; pisteet ja toteutunut aika säilyvät Tutki-toimintoon asti.
    pub fn set_clock(&mut self, index: u32, clock: [i64; 3]) -> Result<(), CoreError> {
        if !(0..=24).contains(&clock[0])
            || !(0..60).contains(&clock[1])
            || !(0..60).contains(&clock[2])
        {
            return Err(CoreError::InvalidInput(
                "Kellon tunnin on oltava 0–24 sekä minuuttien ja sekuntien 0–59".into(),
            ));
        }
        let position = self
            .rivit
            .iter()
            .position(|r| r.output_index == index)
            .ok_or_else(|| CoreError::InvalidInput("Asemariviä ei löydy".into()))?;
        if self.rivit[position].expected_code == 0 {
            let seconds = |c: [i64; 3]| c[0] * 3600 + c[1] * 60 + c[2];
            let origin = self.rivit.first().map_or(0, |r| seconds(r.clock));
            let relative = |c| (seconds(c) - origin).rem_euclid(86_400);
            let known = |r: &&StoredRow| {
                r.sequence != 0
                    && (r.station_type == "L"
                        || self.kasin_annetut_kellot.contains(&r.output_index)
                        || (r.expected_code != 0 && r.received_code == r.expected_code))
            };
            let before = self.rivit[..position].iter().rev().find(known);
            let after = self.rivit[position + 1..].iter().find(known);
            let entered = relative(clock);
            if before.is_some_and(|r| entered < relative(r.clock))
                || after.is_some_and(|r| entered > relative(r.clock))
            {
                let describe = |r: Option<&StoredRow>| {
                    r.map_or("ei asemaa".into(), |r| {
                        format!(
                            "{} {:02}:{:02}:{:02}",
                            r.name, r.clock[0], r.clock[1], r.clock[2]
                        )
                    })
                };
                return Err(CoreError::InvalidInput(format!(
                    "Ajan pitää olla edellisen ja seuraavan aseman välissä: {} – {}",
                    describe(before),
                    describe(after)
                )));
            }
        }
        let last = self.rivit.last().map(|r| r.output_index) == Some(index);
        self.puuttuvat_aikamerkinnat.retain(|r| *r != index);
        let row = self.row_mut(index)?;
        row.clock = clock;
        if clock == [0; 3] {
            row.elapsed_seconds = 0;
        }
        if !self.kasin_annetut_kellot.contains(&index) {
            self.kasin_annetut_kellot.push(index);
        }
        if last {
            self.maalikello_sekuntia = clock[0] * 3600 + clock[1] * 60 + clock[2];
        }
        self.kasikellot_muuttuneet = true;
        self.hyvaksytty = false;
        if self.tila == "TRK" {
            self.tila = "ILM".into();
        }
        Ok(())
    }
    /// Asemarivin pistekorjaus päivittää rivien summan, ei laske aikoja uudelleen.
    pub fn set_row_penalties(
        &mut self,
        index: u32,
        signed: i64,
        other: i64,
    ) -> Result<(), CoreError> {
        if other < 0 {
            return Err(CoreError::InvalidInput(
                "Muu virhe ei saa olla negatiivinen".into(),
            ));
        }
        // Kaikki tarkistukset tehdään kopioon: ylivuoto ei jätä puolikasta korjausta.
        let mut edited = self.clone();
        edited.hyvaksytty = false;
        let row = edited.row_mut(index)?;
        row.signed_time_penalty = signed;
        row.other_penalty = other;
        row.total_penalty = scoring::row_penalty_total(signed, other)?;
        let totals = scoring::sum_row_penalties(
            &edited
                .rivit
                .iter()
                .map(|r| (r.signed_time_penalty, r.other_penalty))
                .collect::<Vec<_>>(),
        )?;
        edited.aikavirheet = totals.time_points;
        edited.asemien_muut_virheet = totals.other_points;
        *self = edited;
        Ok(())
    }
}

/// Samat laskennan lähtötietomuutokset mitätöivät hyväksynnän sekä lomakkeissa
/// että muutostiedostossa. Pelkät sijoitusasetukset eivät muuta asemalaskentaa.
pub(crate) fn invalidate_input_changes(
    before: &crate::native::Project,
    after: &mut crate::native::Project,
) {
    // Samassa tapahtumassa uudelleen laskettu ja nimenomaisesti hyväksytty
    // tulos voidaan säilyttää. Pelkkä vanhan hyväksynnän kopio ei riitä.
    let approvals = after
        .kilpailijat
        .iter()
        .filter_map(|driver| {
            let result = driver.tallennettu_tulos.as_ref()?;
            let old = before
                .kilpailijat
                .iter()
                .find(|old| old.numero == driver.numero)?;
            (result.hyvaksytty && old.tallennettu_tulos.as_ref() != Some(result))
                .then_some(driver.numero)
        })
        .collect::<Vec<_>>();
    let bc_approvals = after
        .bc_osallistujat
        .iter()
        .filter_map(|entry| {
            let result = entry.tallennettu_tulos.as_ref()?;
            let old = before
                .bc_osallistujat
                .iter()
                .find(|old| old.numero == entry.numero)?;
            (result.hyvaksytty && old.tallennettu_tulos.as_ref() != Some(result))
                .then_some(entry.numero)
        })
        .collect::<Vec<_>>();
    crate::beginners::invalidate_input_changes(before, after);
    let global_timing_changed = before.kilpailu.ajanottotapa != after.kilpailu.ajanottotapa
        || !before.virhepisteet.same_scoring(&after.virhepisteet);
    for driver in &mut after.kilpailijat {
        let Some(old) = before
            .kilpailijat
            .iter()
            .find(|d| d.numero == driver.numero)
            .or_else(|| before.kilpailijat.iter().find(|d| d.emit == driver.emit))
        else {
            continue;
        };
        // Preserve only a snapshot explicitly recaptured for the newly recorded
        // card. A prior card's snapshot is never transferred by a number edit.
        if driver.emit != old.emit {
            let new_read = after
                .leimaluvut
                .iter()
                .skip(before.leimaluvut.len())
                .any(|read| read.emit == driver.emit && read.validate().is_ok());
            if new_read
                && driver
                    .tallennettu_tulos
                    .as_ref()
                    .is_some_and(|r| r.emit_kortti == Some(driver.emit))
            {
                if let Some(result) = &mut driver.tallennettu_tulos {
                    result.hyvaksytty = false;
                }
                continue;
            }
            driver.tallennettu_tulos = None;
            continue;
        }
        let old_class = before
            .luokat
            .iter()
            .find(|c| c.tunnus.trim() == old.luokka.trim());
        let new_class = after
            .luokat
            .iter()
            .find(|c| c.tunnus.trim() == driver.luokka.trim());
        let class_timing_changed = match (old_class, new_class) {
            (Some(old_class), Some(new_class)) => {
                old_class.myohastymisraja_minuutteina != new_class.myohastymisraja_minuutteina
                    || old_class.rata != new_class.rata
                    || before
                        .radat
                        .iter()
                        .find(|r| r.numero == old_class.rata)
                        .map(|r| &r.asemat)
                        != after
                            .radat
                            .iter()
                            .find(|r| r.numero == new_class.rata)
                            .map(|r| &r.asemat)
            }
            _ => true,
        };
        let timing_changed = global_timing_changed
            || class_timing_changed
            || old.lahtoaika != driver.lahtoaika
            || old.effective_emit_correction() != driver.effective_emit_correction();
        if let Some(result) = &mut driver.tallennettu_tulos {
            if timing_changed {
                result.hyvaksytty = false;
                result.kasikellot_muuttuneet = true;
            } else if !old.lisavirheet.same_scoring(&driver.lisavirheet) || old.tila != driver.tila
            {
                result.hyvaksytty = false;
            }
        }
    }
    for number in approvals {
        let valid = after
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .and_then(|d| d.tallennettu_tulos.as_ref())
            .is_some_and(|r| {
                r.can_approve()
                    && crate::application::validate_project_result(after, number, r).is_ok()
            });
        if valid {
            after
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .unwrap()
                .tallennettu_tulos
                .as_mut()
                .unwrap()
                .hyvaksytty = true;
        }
    }
    for number in bc_approvals {
        let valid = crate::beginners::projection(after).is_ok_and(|view| {
            view.kilpailijat
                .iter()
                .find(|d| d.numero == number)
                .and_then(|d| d.tallennettu_tulos.as_ref())
                .is_some_and(|r| {
                    r.can_approve()
                        && crate::application::validate_project_result(&view, number, r).is_ok()
                })
        });
        if valid {
            after
                .bc_osallistujat
                .iter_mut()
                .find(|e| e.numero == number)
                .unwrap()
                .tallennettu_tulos
                .as_mut()
                .unwrap()
                .hyvaksytty = true;
        }
    }
}

#[cfg(test)]
mod card_identity_tests {
    use super::*;
    fn project() -> crate::native::Project {
        let mut p = crate::native::Project::from_toml(include_str!(
            "../fixtures/native-demo/kilpailu.toml"
        ))
        .unwrap();
        p.kilpailijat[0].tallennettu_tulos =
            Some(crate::application::capture_project_result(&p, 7).unwrap());
        p
    }
    #[test]
    fn explicit_completed_result_and_points_keep_approval_in_one_transaction() {
        let before = project();
        let mut after = before.clone();
        let mut result = before.kilpailijat[0].tallennettu_tulos.clone().unwrap();
        result.set_clock(5, [12, 10, 0]).unwrap();
        result = crate::application::recalculate_stored_result(&after, 7, &result).unwrap();
        assert!(result.can_approve(), "{result:?}");
        result.hyvaksytty = true;
        after.kilpailijat[0].lisavirheet.muut_virhepisteet = 25;
        after.kilpailijat[0].tallennettu_tulos = Some(result);
        invalidate_input_changes(&before, &mut after);
        assert!(
            after.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty,
            "{:?}",
            after.kilpailijat[0].tallennettu_tulos
        );
        let before = after.clone();
        after.kilpailijat[0].lisavirheet.muut_virhepisteet = 26;
        invalidate_input_changes(&before, &mut after);
        assert!(
            !after.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty,
            "Pelkkä pistekorjaus ei hyväksy vanhaa tulosta"
        );
    }
    #[test]
    fn timing_change_still_invalidates_an_explicit_approval() {
        let mut before = project();
        let mut result = before.kilpailijat[0].tallennettu_tulos.clone().unwrap();
        result.set_clock(5, [12, 10, 0]).unwrap();
        result = crate::application::recalculate_stored_result(&before, 7, &result).unwrap();
        before.kilpailijat[0].tallennettu_tulos = Some(result);
        let mut after = before.clone();
        after.kilpailijat[0].lahtoaika = "12:01:00".into();
        after.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        invalidate_input_changes(&before, &mut after);
        let result = after.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert!(!result.hyvaksytty);
        assert!(result.kasikellot_muuttuneet);
    }
    #[test]
    fn newly_recaptured_card_survives_lan_input_invalidation() {
        let before = project();
        let mut after = before.clone();
        let punches = after.leimaluvut[0]
            .leimat
            .iter()
            .skip(1)
            .take_while(|p| p.koodi != 250)
            .cloned()
            .collect();
        crate::emit_corrections::save_card(&mut after, 7, 7002, punches, false).unwrap();
        let snapshot = after.kilpailijat[0].tallennettu_tulos.clone();
        assert_eq!(snapshot.as_ref().unwrap().emit_kortti, Some(7002));
        invalidate_input_changes(&before, &mut after);
        assert_eq!(after.kilpailijat[0].tallennettu_tulos, snapshot);
    }
    #[test]
    fn tulkki_metadata_preserves_approval_but_points_require_new_approval() {
        let mut before = project();
        before.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        let mut after = before.clone();
        after.kilpailijat[0].lisavirheet.tulkki_siirto = Some(crate::native::TulkkiPenaltySource {
            puuttuvat_rt: Vec::new(),
            tunniste: "a".repeat(64),
            tapahtumatunniste: None,
            tulkatut_jaksot: Vec::new(),
            jaksoja: 0,
            muut_virhepisteet: 0,
        });
        invalidate_input_changes(&before, &mut after);
        assert!(
            after.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty
        );
        let before = after.clone();
        after.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 1;
        after.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet = 600;
        invalidate_input_changes(&before, &mut after);
        assert!(
            !after.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty
        );
    }
    #[test]
    fn new_read_does_not_transfer_an_old_cards_snapshot() {
        let before = project();
        let mut after = before.clone();
        let mut read = after.leimaluvut[0].clone();
        read.emit = 7002;
        after.leimaluvut.push(read);
        after.kilpailijat[0].emit = 7002;
        invalidate_input_changes(&before, &mut after);
        assert!(after.kilpailijat[0].tallennettu_tulos.is_none());
    }
}
