//! Uusi, ihmisen luettavaksi tarkoitettu kilpailumuoto (UTF-8 / TOML).
//! Laskentasääntöjä ei toteuteta tässä: tiedot sovitetaan nykyiseen ytimeen.

use crate::{
    CoreError,
    legacy::{
        self, Archive, BasicSettings, ClassDefinition, Competitor, Document, Encoding, Route,
        RouteStationRecord,
    },
    terminal::parse_clock,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs,
    path::Path,
};

pub const FILE_NAME: &str = "kilpailu.toml";

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub versio: u32,
    pub kilpailu: Competition,
    #[serde(default)]
    pub virhepisteet: Overrides,
    /// Sp 4: position 1 is the first value. Stored with the native competition.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sarjapisteet: Vec<i64>,
    #[serde(default)]
    pub luokat: Vec<Class>,
    /// Up to four explicit combined result classes from the G08 ranking model.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub yhdistetyt_luokat: Vec<CombinedClass>,
    #[serde(default)]
    pub radat: Vec<Track>,
    #[serde(default)]
    pub kilpailijat: Vec<Driver>,
    #[serde(default)]
    pub leimaluvut: Vec<Readout>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Competition {
    pub numero: u32,
    pub nimi: String,
    #[serde(default = "default_method")]
    pub ajanottotapa: u32,
}
fn default_method() -> u32 {
    1
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Overrides {
    pub vapaa_aika_sekunteina: Option<i64>,
    pub ennakon_pisteet_sekunnilta: Option<i64>,
    pub myohastymisen_pisteet_sekunnilta: Option<i64>,
    pub puuttuva_at: Option<i64>,
    pub ylimaarainen_at: Option<i64>,
    pub puuttuva_jat: Option<i64>,
    pub jakson_raja_minuutteina: Option<i64>,
    pub at_enimmaispisteet: Option<i64>,
    pub jat_enimmaispisteet: Option<i64>,
    pub puuttuva_rt: Option<i64>,
    pub ylimaarainen_rt: Option<i64>,
}
impl Overrides {
    fn as_legacy(&self) -> Vec<Option<i64>> {
        vec![
            self.vapaa_aika_sekunteina,
            self.ennakon_pisteet_sekunnilta,
            self.myohastymisen_pisteet_sekunnilta,
            self.puuttuva_at,
            self.ylimaarainen_at,
            self.puuttuva_jat,
            self.jakson_raja_minuutteina,
            self.at_enimmaispisteet,
            self.jat_enimmaispisteet,
            self.puuttuva_rt,
            self.ylimaarainen_rt,
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Class {
    pub tunnus: String,
    pub nimi: String,
    pub rata: u32,
    #[serde(default = "one")]
    pub ensimmainen_numero: u32,
    #[serde(default = "default_start")]
    pub lahtoaika: String,
    #[serde(default = "one")]
    pub lahtovali_minuutteina: u32,
    #[serde(default = "default_lateness")]
    pub myohastymisraja_minuutteina: u32,
    /// AS2015A:n Np-valinta (0/1).
    #[serde(default)]
    pub nousupisteet: u8,
    /// AS2015A:n Sp-menetelmä. Menetelmä 4 edellyttää sarjap.txt:tä.
    #[serde(default)]
    pub sarjapistetapa: u8,
    #[serde(default = "enabled", skip_serializing_if = "is_enabled")]
    pub osallistuu_lahtoarvontaan: bool,
}
/// A combined ranking class. Member codes are exact, not legacy prefix matches.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombinedClass {
    pub nimi: String,
    pub luokat: Vec<String>,
    #[serde(default)]
    pub nousupisteet: u8,
    #[serde(default)]
    pub sarjapistetapa: u8,
}

fn one() -> u32 {
    1
}
fn enabled() -> bool {
    true
}
fn is_enabled(value: &bool) -> bool {
    *value
}
fn default_start() -> String {
    "00:00:00".to_owned()
}
fn default_lateness() -> u32 {
    60
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub numero: u32,
    pub nimi: String,
    #[serde(default)]
    pub asemat: Vec<Station>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Station {
    pub tyyppi: String,
    pub nimi: String,
    pub emit_koodi: u32,
    pub tavoiteaika_sekuntia: i64,
    #[serde(default)]
    pub saanto: i64,
    pub jakso_minuutteina: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Driver {
    pub numero: u32,
    pub luokka: String,
    pub kuljettaja: String,
    pub kartanlukija: String,
    /// Lisenssin X- ja numeromerkit tulkitaan vanhan sarjapistelaskennan mukaan.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kuljettajan_lisenssi: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kartanlukijan_lisenssi: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kuljettajan_seura: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kartanlukijan_seura: String,
    /// Puuttuva kenttä on valmistelutieto, ei fyysinen kortti numero 0.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub emit: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub auto: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lahtoaika: String,
    #[serde(default)]
    pub emit_korjaus_sekuntia: i64,
    /// Erilliset kilpailijakohtaiset virhepisteet, eivät asemarivien pisteitä.
    #[serde(default, skip_serializing_if = "AdditionalPenalties::is_zero")]
    pub lisavirheet: AdditionalPenalties,
    /// Puuttuva valinta tarkoittaa laskennan automaattista tilaa.
    /// Myös TRK ja KESK voidaan valita nimenomaisesti käsin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tila: Option<ManualStatus>,
    /// Toimitsijan erikseen vahvistama, perusteltu KESK-päätös.
    /// Ei merkitse asemarivejä, maaliaikaa tai pisteitä lasketuiksi/hyväksytyiksi.
    #[serde(default, skip_serializing_if = "is_false")]
    pub keskeytys_vahvistettu: bool,
    /// Tarkista tallentaa asemat; Tutki laskee tallennetuista käsikorjauksista.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tallennettu_tulos: Option<crate::corrections::StoredResult>,
}

fn is_false(value: &bool) -> bool {
    !value
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

/// Vahvistettava syy on yksi ei-tyhjä tekstirivi. Keskeneräinen tyhjä
/// syykenttä on sallittu kilpailijan muokkauksessa, mutta ei vahvistuksessa.
pub fn validate_interruption_reason(reason: &str) -> Result<&str, CoreError> {
    if reason.chars().any(char::is_control) {
        return Err(invalid(
            "Keskeytyksen syy ei saa sisältää rivinvaihtoja tai ohjausmerkkejä",
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() {
        return Err(invalid("Anna keskeytyksen syy ennen vahvistamista"));
    }
    if reason.chars().any(char::is_control) {
        return Err(invalid(
            "Keskeytyksen syy ei saa sisältää rivinvaihtoja tai ohjausmerkkejä",
        ));
    }
    Ok(reason)
}
impl Driver {
    pub fn emit_number(&self) -> Option<u32> {
        (self.emit != 0).then_some(self.emit)
    }

    pub fn start_clock(&self) -> Result<Option<crate::ClockTime>, CoreError> {
        if self.lahtoaika.trim().is_empty() {
            Ok(None)
        } else {
            parse_clock(&self.lahtoaika).map(Some)
        }
    }
    pub fn can_confirm_interruption(&self) -> bool {
        self.tila.as_ref().is_some_and(|status| {
            status.tunnus == "KESK" && validate_interruption_reason(&status.syy).is_ok()
        })
    }
    pub fn has_confirmed_interruption(&self) -> bool {
        self.keskeytys_vahvistettu && self.can_confirm_interruption()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ManualStatus {
    pub tunnus: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub syy: String,
}

impl ManualStatus {
    pub fn new(tunnus: &str, syy: &str) -> Result<Self, CoreError> {
        let status = Self {
            tunnus: tunnus.to_owned(),
            syy: syy.to_owned(),
        };
        status.validate()?;
        Ok(status)
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if !matches!(
            self.tunnus.as_str(),
            "TRK" | "KESK" | "PER" | "HYL" | "SULJ" | "ILM" | "EILÄ"
        ) {
            return Err(invalid(
                "Kilpailijan tilan pitää olla TRK, KESK, PER, HYL, SULJ, ILM tai EILÄ",
            ));
        }
        Ok(())
    }
}

/// AS2015A:n Kilp.txt-L-tietueen erilliset kentät 21–25.
/// Määrät ja jo tallennetut pistesummat pidetään erillään:
/// kortin uusintaluku ei saa hiljaisesti hinnoitella aiempia RT-lisäyksiä uudestaan.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct AdditionalPenalties {
    pub puuttuvat_rt_kpl: u32,
    pub puuttuvat_rt_pisteet: i64,
    pub ylimaaraiset_rt_kpl: u32,
    pub ylimaaraiset_rt_pisteet: i64,
    pub muut_virhepisteet: i64,
}

impl AdditionalPenalties {
    pub fn is_zero(&self) -> bool {
        self.puuttuvat_rt_kpl == 0
            && self.puuttuvat_rt_pisteet == 0
            && self.ylimaaraiset_rt_kpl == 0
            && self.ylimaaraiset_rt_pisteet == 0
            && self.muut_virhepisteet == 0
    }

    fn validate(&self) -> Result<(), CoreError> {
        if self.puuttuvat_rt_pisteet < 0
            || self.ylimaaraiset_rt_pisteet < 0
            || self.muut_virhepisteet < 0
        {
            return Err(invalid(
                "Erilliset virhepisteet eivät saa olla negatiivisia",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Readout {
    pub emit: u32,
    pub lukukello: String,
    pub leimat: Vec<ReadPunch>,
    /// Fyysisen lukutapahtuman aika; puuttuu vanhoista version 1 tiedostoista.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub luettu_unix_ms: Option<u64>,
}
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadPunch {
    pub koodi: u32,
    pub aika_sekuntia: u32,
}

pub fn load(path: &Path) -> Result<Archive, CoreError> {
    let text = fs::read_to_string(path)
        .map_err(|e| invalid(format!("{}: {}", path.display(), crate::io_message(&e))))?;
    Project::from_toml(&text)?.into_archive()
}

impl Project {
    pub fn from_toml(text: &str) -> Result<Self, CoreError> {
        let project: Self = toml::from_str(text).map_err(|e: toml::de::Error| {
            let line = e
                .span()
                .map(|s| text[..s.start].bytes().filter(|b| *b == b'\n').count() + 1);
            match line {
                Some(line) => invalid(format!(
                    "kilpailu.toml: virheellinen kenttä tai arvo rivillä {line}"
                )),
                None => invalid("kilpailu.toml: virheellinen TOML-rakenne tai kenttä"),
            }
        })?;
        project.validate()?;
        Ok(project)
    }

    pub fn to_toml(&self) -> Result<String, CoreError> {
        self.validate()?;
        toml::to_string_pretty(self)
            .map_err(|_| invalid("Kilpailutietojen muodostaminen TOML-muotoon epäonnistui"))
    }

    /// Sama ajantasainen malli validoidaan ja sovitetaan laskennan syötteeksi.
    pub fn validate(&self) -> Result<(), CoreError> {
        self.to_archive().map(|_| ())
    }

    pub fn to_archive(&self) -> Result<Archive, CoreError> {
        self.clone().into_archive()
    }

    pub fn into_archive(self) -> Result<Archive, CoreError> {
        if self.versio != 1 {
            return Err(invalid("Kilpailutiedoston versiota ei tueta (tuettu: 1)"));
        }
        if self.kilpailu.numero == 0 || self.kilpailu.nimi.trim().is_empty() {
            return Err(invalid("Kilpailun numero ja nimi vaaditaan"));
        }
        crate::timing::TimingMethod::from_legacy(i64::from(self.kilpailu.ajanottotapa))?;
        if self.sarjapisteet.len() > 1000 || self.sarjapisteet.iter().any(|&points| points < 0) {
            return Err(invalid(
                "Sarjapistetaulukossa saa olla enintään 1000 ei-negatiivista pistettä",
            ));
        }
        let mut classes = Vec::new();
        let mut class_keys = HashSet::new();
        for class in &self.luokat {
            let code = class.tunnus.trim();
            if code.is_empty() || !class_keys.insert(code.to_owned()) {
                return Err(invalid("Luokan tunnus puuttuu tai toistuu"));
            }
            if !self.radat.iter().any(|r| r.numero == class.rata) {
                return Err(invalid(format!(
                    "Luokan {code} rataa {} ei löydy",
                    class.rata
                )));
            }
            if class.nousupisteet > 1 {
                return Err(invalid("Luokan nousupistevalinnan pitää olla 0 tai 1"));
            }
            if class.sarjapistetapa > 6 {
                return Err(invalid("Luokan sarjapistetapa ei voi olla suurempi kuin 6"));
            }
            if class.sarjapistetapa == 4 && self.sarjapisteet.is_empty() {
                return Err(invalid(
                    "Sarjapistetapa 4 edellyttää kilpailun sarjapistetaulukkoa",
                ));
            }
            let time = parse_clock(&class.lahtoaika)?;
            classes.push(ClassDefinition {
                code: code.to_owned(),
                name: class.nimi.trim().to_owned(),
                route: class.rata.to_string(),
                fields: vec![
                    code.to_owned(),
                    class.nimi.clone(),
                    class.rata.to_string(),
                    class.ensimmainen_numero.to_string(),
                    format!("{:02}.{:02}.{:02}", time.hour, time.minute, time.second),
                    class.lahtovali_minuutteina.to_string(),
                    "0".into(),
                    class.myohastymisraja_minuutteina.to_string(),
                    class.nousupisteet.to_string(),
                    class.sarjapistetapa.to_string(),
                ],
            });
        }
        if self.yhdistetyt_luokat.len() > 4 {
            return Err(invalid("Yhdistettyjä luokkia saa olla enintään neljä"));
        }
        let mut combined_classes = Vec::new();
        let mut combined_names = HashSet::new();
        for (index, group) in self.yhdistetyt_luokat.iter().enumerate() {
            let name = group.nimi.trim();
            if name.is_empty() || !combined_names.insert(name.to_owned()) {
                return Err(invalid("Yhdistetyn luokan nimi puuttuu tai toistuu"));
            }
            if group.luokat.is_empty() || group.nousupisteet > 1 || group.sarjapistetapa > 6 {
                return Err(invalid(format!(
                    "Yhdistetyn luokan {name} jäsenet tai Np/Sp-valinta on virheellinen"
                )));
            }
            if group.sarjapistetapa == 4 && self.sarjapisteet.is_empty() {
                return Err(invalid(
                    "Yhdistetyn luokan Sp 4 edellyttää sarjapistetaulukkoa",
                ));
            }
            let mut members = HashSet::new();
            for code in &group.luokat {
                if !class_keys.contains(code.as_str()) || !members.insert(code.as_str()) {
                    return Err(invalid(format!(
                        "Yhdistetyssä luokassa {name} on tuntematon tai toistuva jäsenluokka {code}"
                    )));
                }
            }
            let mut fields = vec![
                (index + 1).to_string(),
                name.to_owned(),
                group.nousupisteet.to_string(),
                group.sarjapistetapa.to_string(),
            ];
            fields.extend(classes.iter().map(|class| {
                if members.contains(class.code.as_str()) {
                    "1"
                } else {
                    "0"
                }
                .to_owned()
            }));
            combined_classes.push(fields);
        }
        let mut routes = Vec::new();
        let mut route_keys = HashSet::new();
        for track in &self.radat {
            if track.numero == 0 || !route_keys.insert(track.numero) {
                return Err(invalid("Radan numero puuttuu tai toistuu"));
            }
            if track.nimi.trim().is_empty() || track.asemat.is_empty() {
                return Err(invalid(format!(
                    "Rata {} tarvitsee nimen ja asemia",
                    track.numero
                )));
            }
            let mut stations = Vec::new();
            for (i, s) in track.asemat.iter().enumerate() {
                crate::StationKind::from_legacy(&s.tyyppi)?;
                crate::Rule::from_legacy(s.saanto)?;
                if s.tavoiteaika_sekuntia < 0 {
                    return Err(invalid("Aseman tavoiteaika ei voi olla negatiivinen"));
                }
                stations.push(RouteStationRecord {
                    index: (i + 1)
                        .try_into()
                        .map_err(|_| invalid("Liian monta asemaa"))?,
                    station_type: s.tyyppi.clone(),
                    expected_code: s.emit_koodi,
                    name: s.nimi.clone(),
                    rule: s.saanto,
                    minute_text: format!(
                        "{},{:02}",
                        s.tavoiteaika_sekuntia / 60,
                        s.tavoiteaika_sekuntia % 60
                    ),
                    seconds: s.tavoiteaika_sekuntia,
                    interval: 0,
                    section_text: s.jakso_minuutteina.map_or(String::new(), |n| n.to_string()),
                });
            }
            routes.push(Route {
                index: track.numero,
                name: track.nimi.clone(),
                declared_station_count: stations.len() as u32,
                stations,
            });
        }
        let mut competitors = Vec::new();
        let mut numbers = HashSet::new();
        let mut emit_cards = HashSet::new();
        for driver in &self.kilpailijat {
            if driver.numero == 0 || !numbers.insert(driver.numero) {
                return Err(invalid("Kilpailijan numero puuttuu tai toistuu"));
            }
            if driver
                .emit_number()
                .is_some_and(|card| !emit_cards.insert(card))
            {
                return Err(invalid(format!(
                    "Kilpailijan {} Emit-kortin numero puuttuu tai on käytössä",
                    driver.numero
                )));
            }
            if !class_keys.contains(driver.luokka.trim()) {
                return Err(invalid(format!(
                    "Kilpailijan {} luokkaa ei löydy",
                    driver.numero
                )));
            }
            driver.lisavirheet.validate()?;
            if let Some(status) = &driver.tila {
                status.validate()?;
            }
            if driver.keskeytys_vahvistettu && !driver.has_confirmed_interruption() {
                return Err(invalid(
                    "Vahvistettu keskeytys edellyttää KESK-tilaa ja keskeytyksen syytä",
                ));
            }
            let time = driver.start_clock()?;
            let mut fields = vec!["0".to_owned(); 31];
            fields[0] = driver.numero.to_string();
            fields[1] = driver.luokka.trim().to_owned();
            fields[2] = driver
                .emit_number()
                .map_or_else(String::new, |card| card.to_string());
            fields[3] = driver.kuljettaja.clone();
            fields[4] = driver.kartanlukija.clone();
            fields[5] = driver.kuljettajan_lisenssi.clone();
            fields[6] = driver.kartanlukijan_lisenssi.clone();
            fields[7] = driver.kuljettajan_seura.clone();
            fields[8] = driver.kartanlukijan_seura.clone();
            fields[9] = driver.auto.clone();
            fields[10] = time.map_or_else(String::new, |time| time.hour.to_string());
            fields[11] = time.map_or_else(String::new, |time| time.minute.to_string());
            fields[12] = time.map_or_else(String::new, |time| time.second.to_string());
            fields[21] = driver.lisavirheet.puuttuvat_rt_kpl.to_string();
            fields[22] = driver.lisavirheet.puuttuvat_rt_pisteet.to_string();
            fields[23] = driver.lisavirheet.ylimaaraiset_rt_kpl.to_string();
            fields[24] = driver.lisavirheet.ylimaaraiset_rt_pisteet.to_string();
            fields[25] = driver.lisavirheet.muut_virhepisteet.to_string();
            fields[28] = driver
                .tila
                .as_ref()
                .map_or("ILM", |status| status.tunnus.as_str())
                .to_owned();
            fields[29] = driver
                .tila
                .as_ref()
                .map_or("", |status| status.syy.as_str())
                .to_owned();
            fields[30] = driver.emit_korjaus_sekuntia.to_string();
            competitors.push(Competitor {
                number: driver.numero,
                class_code: fields[1].clone(),
                emit_card: fields[2].clone(),
                driver: fields[3].clone(),
                navigator: fields[4].clone(),
                status: fields[28].clone(),
                header_fields: fields,
                total_penalties: 0,
                declared_row_count: 0,
                correction_seconds: driver.emit_korjaus_sekuntia,
                results: Vec::new(),
            });
        }
        let mut documents: BTreeMap<String, Document> = BTreeMap::new();
        if !self.sarjapisteet.is_empty() {
            // Reuse the legacy ranker's first-row sarjap.txt input, without
            // writing a second persistent source of truth beside kilpailu.toml.
            let content = format!(
                "{};\n",
                self.sarjapisteet
                    .iter()
                    .map(i64::to_string)
                    .collect::<Vec<_>>()
                    .join(";")
            );
            documents.insert(
                "sarjap.txt".to_owned(),
                legacy::parse_document(content.as_bytes(), Encoding::Utf8)
                    .map_err(|error| invalid(error.to_string()))?,
            );
        }
        let mut warnings = Vec::new();
        // Fyysiset tapahtumat säilyvät Projectissa. Laskennan syötteeksi valitaan
        // jokaisen kortin viimeisin kelvollinen lukema, ei erillisiä kilpailijatietoja.
        let mut active = BTreeMap::new();
        for (index, read) in self.leimaluvut.iter().enumerate() {
            match read.validate() {
                Ok(()) => {
                    active.insert(read.emit, read);
                }
                Err(error) => warnings.push(format!(
                    "Kortin {} historian lukema {} ohitettiin: {error}",
                    read.emit,
                    index + 1
                )),
            }
        }
        if !active.is_empty() {
            let mut source = String::new();
            for read in active.values() {
                let time = parse_clock(&read.lukukello)?;
                source.push_str(&format!("999;{};{};", read.emit, read.leimat.len()));
                for record in &read.leimat {
                    source.push_str(&format!("{};{};", record.koodi, record.aika_sekuntia));
                }
                source.push_str(&format!("{};{};{};\n", time.hour, time.minute, time.second));
            }
            let doc = legacy::parse_document(source.as_bytes(), Encoding::Utf8)
                .map_err(|e| invalid(e.to_string()))?;
            documents.insert("emit.txt".to_owned(), doc);
        }
        let params = self.virhepisteet.as_legacy();
        let basic = BasicSettings {
            identity: vec![
                self.kilpailu.numero.to_string(),
                self.kilpailu.nimi,
                "0".into(),
                String::new(),
                classes.len().to_string(),
                String::new(),
                "0".into(),
                self.kilpailu.ajanottotapa.to_string(),
            ],
            output_names: vec!["tulokset".into(), "asemat".into()],
            classes,
            combined_classes,
        };
        let archive = Archive {
            result_publication: Some(BTreeMap::new()),
            documents,
            parameters: params,
            basic,
            routes,
            competitors,
            warnings,
        };
        archive
            .scoring_parameters()
            .map_err(|e| invalid(e.to_string()))?;
        Ok(archive)
    }
}

/// Luo uuden kilpailun samaan tietomalliin, jota tallennus ja laskenta käyttävät.
impl Project {
    pub fn new(name: &str) -> Result<Self, CoreError> {
        let project = Self {
            versio: 1,
            kilpailu: Competition {
                numero: 1,
                nimi: name.trim().into(),
                ajanottotapa: default_method(),
            },
            virhepisteet: Overrides::default(),
            sarjapisteet: Vec::new(),
            yhdistetyt_luokat: Vec::new(),
            luokat: vec![Class {
                tunnus: "H".into(),
                nimi: "Harjoitus".into(),
                rata: 1,
                ensimmainen_numero: 1,
                lahtoaika: "15:00:00".into(),
                lahtovali_minuutteina: 2,
                myohastymisraja_minuutteina: default_lateness(),
                nousupisteet: 0,
                sarjapistetapa: 0,
                osallistuu_lahtoarvontaan: true,
            }],
            radat: vec![Track {
                numero: 1,
                nimi: "Harjoitusrata".into(),
                asemat: vec![
                    Station {
                        tyyppi: "L".into(),
                        nimi: "Lähtö".into(),
                        emit_koodi: 0,
                        tavoiteaika_sekuntia: 0,
                        saanto: 0,
                        jakso_minuutteina: None,
                    },
                    Station {
                        tyyppi: "M".into(),
                        nimi: "Maali".into(),
                        emit_koodi: 0,
                        tavoiteaika_sekuntia: 300,
                        saanto: 0,
                        jakso_minuutteina: Some(5),
                    },
                ],
            }],
            kilpailijat: Vec::new(),
            leimaluvut: Vec::new(),
        };
        project.validate()?;
        Ok(project)
    }
}

/// Luo kilpailukansion ja tallentaa valmiin tiedoston korvaamatta olemassa olevaa.
pub fn create(directory: &Path, name: &str) -> Result<(), CoreError> {
    let project = Project::new(name)?;
    fs::create_dir_all(directory).map_err(|e| invalid(crate::io_message(&e)))?;
    crate::storage::ProjectStore::create(&directory.join(FILE_NAME), project).map(|_| ())
}

/// Yhden fyysisen lukemisen tulos. Laskentatuloksia ei kopioida historiaan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadOutcome {
    pub card_number: u32,
    pub competitor_number: Option<u32>,
    pub content_changed: bool,
}

impl Readout {
    fn punches(&self) -> Vec<crate::Punch> {
        self.leimat
            .iter()
            .map(|p| crate::Punch {
                code: p.koodi,
                elapsed_seconds: p.aika_sekuntia,
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), CoreError> {
        if self.emit == 0 {
            return Err(invalid("Emit-kortin numero ei voi olla nolla"));
        }
        if self.leimat.is_empty() || self.leimat.len() > 50 {
            return Err(invalid("Emit-lukemassa pitää olla 1–50 tietuetta"));
        }
        crate::emit::accepted_records(&self.punches(), parse_clock(&self.lukukello)?)?;
        Ok(())
    }

    /// Varsinainen korttisisältö ei sisällä lukumerkkiä eikä sen muuttuvaa aikaa.
    pub fn content(&self) -> Result<Vec<crate::Punch>, CoreError> {
        self.validate()?;
        let punches = self.punches();
        Ok(crate::emit::punches_before_reader_marker(&punches)?.to_vec())
    }
}

impl Project {
    pub fn active_readout(&self, card_number: u32) -> Option<&Readout> {
        self.leimaluvut
            .iter()
            .rev()
            .find(|read| read.emit == card_number && read.validate().is_ok())
    }

    /// Täydentää yhteistä mallia vasta, kun lukema on kokonaan kelvollinen.
    /// Kilpailijaa vailla oleva kortti kohdistuu myöhemmin Emit-numeron perusteella.
    pub fn record_readout(&mut self, read: Readout) -> Result<ReadOutcome, CoreError> {
        let content = read.content()?;
        let previous = self.active_readout(read.emit);
        let content_changed = match previous {
            Some(previous) => previous.content()? != content,
            None => true,
        };
        let competitors = self
            .kilpailijat
            .iter()
            .filter(|driver| driver.emit == read.emit)
            .collect::<Vec<_>>();
        if competitors.len() > 1 {
            return Err(invalid(format!(
                "Emit-kortti {} kuuluu usealle kilpailijalle",
                read.emit
            )));
        }
        let outcome = ReadOutcome {
            card_number: read.emit,
            competitor_number: competitors.first().map(|driver| driver.numero),
            content_changed,
        };
        self.leimaluvut.push(read);
        Ok(outcome)
    }

    pub fn record_frame(
        &mut self,
        frame: &crate::emit::EmitFrame,
        clock: crate::ClockTime,
    ) -> Result<ReadOutcome, CoreError> {
        let accepted = frame.accepted_readout(clock)?;
        let read = Readout {
            emit: frame.card_number,
            lukukello: format!("{:02}:{:02}:{:02}", clock.hour, clock.minute, clock.second),
            leimat: frame.records[..=accepted.marker_index]
                .iter()
                .map(|p| ReadPunch {
                    koodi: p.code,
                    aika_sekuntia: p.elapsed_seconds,
                })
                .collect(),
            luettu_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|duration| duration.as_millis().try_into().ok()),
        };
        self.record_readout(read)
    }
}
