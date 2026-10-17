//! Virallinen asematulos säilyy erillään muuttumattomasta kortinlukuhistoriasta.
//! Käsikellon muuttaminen ei käynnistä laskentaa. Tutki ja Tarkista ovat eri toimintoja.
use crate::{CoreError, RouteStation, legacy::StationResult, scoring};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StoredResult {
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
    pub fn as_station_results(&self) -> Vec<StationResult> {
        self.rivit.iter().map(StationResult::from).collect()
    }
    pub fn validate_route(&self, route: &[RouteStation]) -> Result<(), CoreError> {
        if self.rata
            != route
                .iter()
                .map(StoredRouteStation::from)
                .collect::<Vec<_>>()
        {
            return Err(CoreError::InvalidInput("Tallennetun tuloksen rata on muuttunut. Tarkista muodostaa asemarivit uudelleen korttiluvusta; käsikorjauksia ei kohdisteta muuttuneeseen rataan automaattisesti".into()));
        }
        if self.kesto_sekuntia < 0 || !matches!(self.tila.as_str(), "TRK" | "KESK") {
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
        let last = self.rivit.last().map(|r| r.output_index) == Some(index);
        let row = self.row_mut(index)?;
        row.clock = clock;
        if clock == [0; 3] {
            row.elapsed_seconds = 0;
        }
        if last {
            self.maalikello_sekuntia = clock[0] * 3600 + clock[1] * 60 + clock[2];
        }
        self.kasikellot_muuttuneet = true;
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
