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
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub const FILE_NAME: &str = "kilpailu.toml";

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub versio: u32,
    pub kilpailu: Competition,
    #[serde(default)]
    pub virhepisteet: Overrides,
    #[serde(default)]
    pub luokat: Vec<Class>,
    #[serde(default)]
    pub radat: Vec<Track>,
    #[serde(default)]
    pub kilpailijat: Vec<Driver>,
    #[serde(default)]
    pub leimaluvut: Vec<Readout>,
}

#[derive(Debug, Deserialize)]
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

#[derive(Debug, Default, Deserialize)]
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

#[derive(Debug, Deserialize)]
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
}
fn one() -> u32 {
    1
}
fn default_start() -> String {
    "00:00:00".to_owned()
}
fn default_lateness() -> u32 {
    60
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub numero: u32,
    pub nimi: String,
    #[serde(default)]
    pub asemat: Vec<Station>,
}
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Driver {
    pub numero: u32,
    pub luokka: String,
    pub kuljettaja: String,
    pub kartanlukija: String,
    pub emit: u32,
    pub lahtoaika: String,
    #[serde(default)]
    pub emit_korjaus_sekuntia: i64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Readout {
    pub emit: u32,
    pub lukukello: String,
    pub leimat: Vec<ReadPunch>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadPunch {
    pub koodi: u32,
    pub aika_sekuntia: u32,
}

pub fn load(path: &Path) -> Result<Archive, CoreError> {
    let text = fs::read_to_string(path)
        .map_err(|e| invalid(format!("{}: {}", path.display(), crate::io_message(&e))))?;
    let project: Project = toml::from_str(&text).map_err(|e: toml::de::Error| {
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
    project.into_archive()
}

impl Project {
    pub fn into_archive(self) -> Result<Archive, CoreError> {
        if self.versio != 1 {
            return Err(invalid("Kilpailutiedoston versiota ei tueta (tuettu: 1)"));
        }
        if self.kilpailu.numero == 0 || self.kilpailu.nimi.trim().is_empty() {
            return Err(invalid("Kilpailun numero ja nimi vaaditaan"));
        }
        crate::timing::TimingMethod::from_legacy(i64::from(self.kilpailu.ajanottotapa))?;
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
                    "0".into(),
                    "0".into(),
                ],
            });
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
            if driver.emit == 0 || !emit_cards.insert(driver.emit) {
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
            let time = parse_clock(&driver.lahtoaika)?;
            let mut fields = vec!["0".to_owned(); 31];
            fields[0] = driver.numero.to_string();
            fields[1] = driver.luokka.trim().to_owned();
            fields[2] = driver.emit.to_string();
            fields[3] = driver.kuljettaja.clone();
            fields[4] = driver.kartanlukija.clone();
            fields[5] = String::new();
            fields[6] = String::new();
            fields[10] = time.hour.to_string();
            fields[11] = time.minute.to_string();
            fields[12] = time.second.to_string();
            fields[28] = "ILM".to_owned();
            fields[29] = String::new();
            fields[30] = driver.emit_korjaus_sekuntia.to_string();
            competitors.push(Competitor {
                number: driver.numero,
                class_code: fields[1].clone(),
                emit_card: fields[2].clone(),
                driver: fields[3].clone(),
                navigator: fields[4].clone(),
                header_fields: fields,
                total_penalties: 0,
                declared_row_count: 0,
                status: "ILM".into(),
                correction_seconds: driver.emit_korjaus_sekuntia,
                results: Vec::new(),
            });
        }
        let mut documents: BTreeMap<String, Document> = BTreeMap::new();
        if !self.leimaluvut.is_empty() {
            let mut source = String::new();
            for read in &self.leimaluvut {
                let time = parse_clock(&read.lukukello)?;
                if read.leimat.is_empty() || read.leimat.len() > 50 {
                    return Err(invalid("Emit-lukemassa pitää olla 1–50 tietuetta"));
                }
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
            combined_classes: Vec::new(),
        };
        let archive = Archive {
            documents,
            parameters: params,
            basic,
            routes,
            competitors,
            warnings: Vec::new(),
        };
        archive
            .scoring_parameters()
            .map_err(|e| invalid(e.to_string()))?;
        Ok(archive)
    }
}

/// Luo selkeän kilpailupohjan. Ei koskaan korvaa olemassa olevaa tiedostoa.
pub fn create(directory: &Path, name: &str) -> Result<(), CoreError> {
    if name.trim().is_empty() {
        return Err(invalid("Kilpailun nimi ei voi olla tyhjä"));
    }
    fs::create_dir_all(directory).map_err(|e| invalid(crate::io_message(&e)))?;
    let quoted_name = toml::Value::String(name.to_owned()).to_string();
    let content = format!(
        r#"# AS2015 — kilpailun tiedot. Tallennusmuoto UTF-8, versio 1.
# Muokkaa Zedissä tai muussa tekstieditorissa.
versio = 1

[kilpailu]
numero = 1
nimi = {quoted_name}
ajanottotapa = 1 # 0 = manuaalinen, 1 = Emit, 2 = vanha ajanottotapa 2

# Kilpailukohtaiset virhepisteet ovat vapaaehtoisia.
# Tyhjä osio käyttää vanhasta ohjelmasta palautettuja oletusarvoja.
[virhepisteet]
# vapaa_aika_sekunteina = 10
# ennakon_pisteet_sekunnilta = 3
# myohastymisen_pisteet_sekunnilta = 1

[[luokat]]
tunnus = "H"
nimi = "Harjoitus"
rata = 1
ensimmainen_numero = 1
lahtoaika = "15:00:00"
lahtovali_minuutteina = 2
myohastymisraja_minuutteina = 60

[[radat]]
numero = 1
nimi = "Harjoitusrata"

[[radat.asemat]]
tyyppi = "L"
nimi = "lähtö"
emit_koodi = 0
tavoiteaika_sekuntia = 0

[[radat.asemat]]
tyyppi = "M"
nimi = "maali"
emit_koodi = 0
tavoiteaika_sekuntia = 300
jakso_minuutteina = 5

# Kilpailijan voi lisätä toistamalla tämän rakenteen:
# [[kilpailijat]]
# numero = 1
# luokka = "H"
# kuljettaja = "Koe"
# kartanlukija = "Testi"
# emit = 538747
# lahtoaika = "15:00:00"

# Kortin lukutiedot voidaan myöhemmin tallentaa automaattisesti.
# [[leimaluvut]]
# emit = 538747
# lukukello = "21:10:17"
# leimat = [{{ koodi = 0, aika_sekuntia = 0 }}, {{ koodi = 250, aika_sekuntia = 5122 }}]
"#
    );
    let path = directory.join(FILE_NAME);
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| invalid(format!("{}: {}", path.display(), crate::io_message(&e))))?;
    file.write_all(content.as_bytes())
        .map_err(|e| invalid(crate::io_message(&e)))?;
    Ok(())
}
