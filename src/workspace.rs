//! Käyttöliittymien yhteinen avattu kilpailu. Muokkaus tallentuu vain yhtenä,
//! validoituna muutoksena; näkymät johdetaan samasta onnistuneesta tilasta.

use crate::{
    ClockTime, CoreError, application,
    emit::EmitFrame,
    legacy::{Archive, Encoding},
    native::{self, Class, Driver, Project, ReadOutcome, Station, Track},
    ranking::{RankingReport, TiePolicy},
    storage::ProjectStore,
};
use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

fn number<T: FromStr>(value: &str, label: &str) -> Result<T, CoreError> {
    value
        .trim()
        .parse()
        .map_err(|_| invalid(format!("{label}: anna kelvollinen kokonaisluku")))
}

fn required(value: &str, label: &str) -> Result<String, CoreError> {
    let text = value.trim();
    if text.is_empty() {
        Err(invalid(format!("{label} ei voi olla tyhjä")))
    } else {
        Ok(text.into())
    }
}

fn optional_number<T: FromStr>(value: &str, label: &str) -> Result<Option<T>, CoreError> {
    if value.trim().is_empty() {
        Ok(None)
    } else {
        number(value, label).map(Some)
    }
}

enum Source {
    Native(ProjectStore),
    Legacy(Archive),
}

/// Ei käyttöliittymäkirjaston, tapahtumasilmukan tai verkkopalvelun riippuvuuksia.
pub struct Workspace {
    path: PathBuf,
    source: Source,
}

impl Workspace {
    /// Kansiossa kilpailu.toml on ensisijainen. Muu kansio avataan AS2015A-
    /// tuontina, jota tämä rajapinta ei kirjoita eikä automaattisesti muunna.
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let path = fs::canonicalize(path).map_err(|e| invalid(crate::io_message(&e)))?;
        let path = if path.is_dir() && path.join(native::FILE_NAME).is_file() {
            path.join(native::FILE_NAME)
        } else {
            path
        };
        let source = if path.is_dir() {
            Source::Legacy(
                Archive::load(&path, Encoding::Windows1252).map_err(|e| invalid(e.to_string()))?,
            )
        } else {
            Source::Native(ProjectStore::open(&path)?)
        };
        Ok(Self { path, source })
    }

    pub fn create(directory: &Path, name: &str) -> Result<Self, CoreError> {
        // Nimi validoidaan ennen kansion tai tiedoston luontia.
        let project = Project::new(name)?;
        fs::create_dir_all(directory).map_err(|e| invalid(crate::io_message(&e)))?;
        let store = ProjectStore::create(&directory.join(native::FILE_NAME), project)?;
        Ok(Self {
            path: store.path().to_owned(),
            source: Source::Native(store),
        })
    }

    /// Epäonnistunut uudelleenavaus jättää nykyisen onnistuneen tilan näkyviin.
    pub fn reload(&mut self) -> Result<(), CoreError> {
        let reopened = Self::open(&self.path)?;
        *self = reopened;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn is_editable(&self) -> bool {
        matches!(self.source, Source::Native(_))
    }

    pub fn project(&self) -> Option<&Project> {
        match &self.source {
            Source::Native(store) => Some(store.project()),
            Source::Legacy(_) => None,
        }
    }

    pub fn name(&self) -> &str {
        match &self.source {
            Source::Native(store) => &store.project().kilpailu.nimi,
            Source::Legacy(archive) => archive
                .basic
                .identity
                .get(1)
                .map_or("Nimetön kilpailu", String::as_str),
        }
    }

    pub fn archive(&self) -> Result<Archive, CoreError> {
        match &self.source {
            Source::Native(store) => store.project().to_archive(),
            Source::Legacy(archive) => Ok(archive.clone()),
        }
    }

    pub fn calculation(&self) -> Result<application::CalculationReport, CoreError> {
        application::calculate(&self.archive()?)
    }

    /// Vanhan aineiston tallennetut tulokset ja uuden kilpailun johdetut
    /// asematulokset pidetään erillään nykyisen sovellusytimen mukaisesti.
    pub fn results(&self) -> Result<Archive, CoreError> {
        match &self.source {
            Source::Native(store) => application::calculated_project(store.project()),
            Source::Legacy(archive) => Ok(archive.clone()),
        }
    }

    pub fn ranking(&self) -> Result<RankingReport, CoreError> {
        application::rank_stored(&self.results()?, TiePolicy::KeepSharedAndFlag)
    }

    fn update<T>(
        &mut self,
        edit: impl FnOnce(&mut Project) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        match &mut self.source {
            Source::Native(store) => store.update(edit),
            Source::Legacy(_) => Err(invalid(
                "AS2015A-aineisto on avattu vain katselua varten. Luo AS2026-kilpailu muokkaamista varten",
            )),
        }
    }

    pub fn record_frame(
        &mut self,
        frame: &EmitFrame,
        clock: ClockTime,
    ) -> Result<ReadOutcome, CoreError> {
        self.update(|project| project.record_frame(frame, clock))
    }

    pub fn add_driver(&mut self, edit: DriverEdit) -> Result<(), CoreError> {
        self.update(|project| {
            project.kilpailijat.push(edit.parse()?);
            Ok(())
        })
    }

    /// Yhden kentän muutos säilyttää myös lomakkeessa näkymättömät tiedot.
    pub fn edit_driver(
        &mut self,
        id: u32,
        field: DriverField,
        value: &str,
    ) -> Result<(), CoreError> {
        self.update(|project| {
            let defaults = crate::scoring::ScoringParameters::default();
            let missing_rate = project
                .virhepisteet
                .puuttuva_rt
                .unwrap_or(defaults.missing_rt);
            let extra_rate = project
                .virhepisteet
                .ylimaarainen_rt
                .unwrap_or(defaults.extra_rt);
            let driver = project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == id)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy; päivitä näkymä"))?;
            match field {
                DriverField::Number => driver.numero = number(value, "Kilpailijan numero")?,
                DriverField::Class => driver.luokka = required(value, "Luokka")?,
                DriverField::Driver => driver.kuljettaja = required(value, "Kuljettaja")?,
                DriverField::Navigator => driver.kartanlukija = value.trim().into(),
                DriverField::Card => driver.emit = number(value, "Emit-numero")?,
                DriverField::Car => driver.auto = value.trim().into(),
                DriverField::Start => driver.lahtoaika = value.trim().into(),
                DriverField::Correction => {
                    driver.emit_korjaus_sekuntia = number(value, "Emit-korjaus")?
                }
                DriverField::MissingRtCount => {
                    let count: u32 = number(value, "Puuttuvien RT-asemien määrä")?;
                    let points = i64::from(count).checked_mul(missing_rate).ok_or(
                        CoreError::ArithmeticOverflow("puuttuvien RT-asemien pisteet"),
                    )?;
                    driver.lisavirheet.puuttuvat_rt_kpl = count;
                    driver.lisavirheet.puuttuvat_rt_pisteet = points;
                }
                DriverField::ExtraRtCount => {
                    let count: u32 = number(value, "Ylimääräisten RT-asemien määrä")?;
                    let points = i64::from(count).checked_mul(extra_rate).ok_or(
                        CoreError::ArithmeticOverflow("ylimääräisten RT-asemien pisteet"),
                    )?;
                    driver.lisavirheet.ylimaaraiset_rt_kpl = count;
                    driver.lisavirheet.ylimaaraiset_rt_pisteet = points;
                }
                DriverField::OtherPenalties => {
                    let points: i64 = number(value, "Muut virhepisteet")?;
                    driver.lisavirheet.muut_virhepisteet = points;
                }
            }
            Ok(())
        })
    }

    pub fn add_class(&mut self, edit: ClassEdit) -> Result<(), CoreError> {
        self.update(|project| {
            project.luokat.push(edit.parse()?);
            Ok(())
        })
    }

    pub fn edit_class(
        &mut self,
        id: &str,
        field: ClassField,
        value: &str,
    ) -> Result<(), CoreError> {
        self.update(|project| {
            let class = project
                .luokat
                .iter_mut()
                .find(|c| c.tunnus.trim() == id.trim())
                .ok_or_else(|| invalid("Luokkaa ei löydy; päivitä näkymä"))?;
            match field {
                ClassField::Code => class.tunnus = required(value, "Luokan tunnus")?,
                ClassField::Name => class.nimi = required(value, "Luokan nimi")?,
                ClassField::Route => class.rata = number(value, "Radan numero")?,
                ClassField::FirstNumber => {
                    class.ensimmainen_numero = number(value, "Ensimmäinen numero")?
                }
                ClassField::Start => class.lahtoaika = value.trim().into(),
                ClassField::Interval => class.lahtovali_minuutteina = number(value, "Lähtöväli")?,
                ClassField::Lateness => {
                    class.myohastymisraja_minuutteina = number(value, "Myöhästymisraja")?
                }
                ClassField::Np => class.nousupisteet = number(value, "Nousupistevalinta")?,
                ClassField::Sp => class.sarjapistetapa = number(value, "Sarjapistemenetelmä")?,
            }
            if field == ClassField::Code {
                let new_code = class.tunnus.clone();
                for driver in &mut project.kilpailijat {
                    if driver.luokka.trim() == id.trim() {
                        driver.luokka = new_code.clone();
                    }
                }
            }
            Ok(())
        })
    }

    /// Uusi rata sisältää muokattavat lähtö- ja maaliasemat kuten uusi kilpailu.
    pub fn add_track(&mut self, edit: TrackEdit) -> Result<(), CoreError> {
        self.update(|project| {
            let mut track = Project::new("Uusi rata")?.radat.remove(0);
            track.numero = number(&edit.number, "Radan numero")?;
            track.nimi = required(&edit.name, "Radan nimi")?;
            project.radat.push(track);
            Ok(())
        })
    }

    pub fn edit_track(&mut self, id: u32, field: TrackField, value: &str) -> Result<(), CoreError> {
        self.update(|project| {
            let track = find_track(project, id)?;
            match field {
                TrackField::Number => track.numero = number(value, "Radan numero")?,
                TrackField::Name => track.nimi = required(value, "Radan nimi")?,
            }
            let new_number = track.numero;
            if field == TrackField::Number {
                for class in &mut project.luokat {
                    if class.rata == id {
                        class.rata = new_number;
                    }
                }
            }
            Ok(())
        })
    }

    /// Palauttaa nollapohjaisen indeksin. Väliasema sijoitetaan ennen viimeistä
    /// maalia, jotta tavallinen lisäys säilyttää maalin radan lopussa.
    pub fn add_station(&mut self, track: u32, edit: StationEdit) -> Result<usize, CoreError> {
        self.update(|project| {
            let station = edit.parse()?;
            validate_station_rule(&station)?;
            let stations = &mut find_track(project, track)?.asemat;
            let index = if station.tyyppi.trim() == "M" {
                stations.len()
            } else {
                stations
                    .iter()
                    .rposition(|existing| existing.tyyppi.trim() == "M")
                    .unwrap_or(stations.len())
            };
            stations.insert(index, station);
            Ok(index)
        })
    }

    pub fn edit_station(
        &mut self,
        track: u32,
        index: usize,
        field: StationField,
        value: &str,
    ) -> Result<(), CoreError> {
        self.update(|project| {
            let station = find_track(project, track)?
                .asemat
                .get_mut(index)
                .ok_or_else(|| invalid("Asemaa ei löydy; päivitä näkymä"))?;
            match field {
                StationField::Kind => station.tyyppi = required(value, "Aseman tyyppi")?,
                StationField::Name => station.nimi = required(value, "Aseman nimi")?,
                StationField::Code => station.emit_koodi = number(value, "Emit-koodi")?,
                StationField::Target => {
                    station.tavoiteaika_sekuntia = number(value, "Tavoiteaika")?
                }
                StationField::Rule => station.saanto = number(value, "Laskentasääntö")?,
                StationField::Section => {
                    station.jakso_minuutteina = optional_number(value, "Jakson minuutit")?
                }
            }
            // Vanhoja tiedostoja voidaan edelleen avata ja niiden muita tietoja
            // korjata. Uutta yhteensopimatonta tyyppi/sääntö-yhdistelmää ei luoda.
            if matches!(field, StationField::Kind | StationField::Rule) {
                validate_station_rule(station)?;
            }
            Ok(())
        })
    }

    pub fn move_station(&mut self, track: u32, from: usize, to: usize) -> Result<(), CoreError> {
        self.update(|project| {
            let stations = &mut find_track(project, track)?.asemat;
            if from >= stations.len() || to >= stations.len() {
                return Err(invalid("Aseman järjestysnumero on virheellinen"));
            }
            let station = stations.remove(from);
            stations.insert(to, station);
            Ok(())
        })
    }

    pub fn edit_competition(
        &mut self,
        field: CompetitionField,
        value: &str,
    ) -> Result<(), CoreError> {
        self.update(|project| {
            match field {
                CompetitionField::Name => {
                    project.kilpailu.nimi = required(value, "Kilpailun nimi")?
                }
                CompetitionField::Number => {
                    project.kilpailu.numero = number(value, "Kilpailun numero")?
                }
                CompetitionField::Timing => {
                    let method = number(value, "Ajanottotapa")?;
                    if method > 2 {
                        return Err(invalid("Valitse ajanottotapa 0, 1 tai 2"));
                    }
                    project.kilpailu.ajanottotapa = method;
                }
                CompetitionField::CustomSeries => {
                    project.sarjapisteet = if value.trim().is_empty() {
                        Vec::new()
                    } else {
                        value
                            .split(',')
                            .map(|part| number(part, "Sarjapistetaulukon pisteet"))
                            .collect::<Result<Vec<i64>, _>>()?
                    };
                }
            }
            Ok(())
        })
    }

    /// Tyhjä arvo palauttaa vain valitun virhepisteasetuksen ytimen oletukseen.
    pub fn edit_override(&mut self, field: OverrideField, value: &str) -> Result<(), CoreError> {
        self.update(|project| {
            let value = optional_number(value, "Virhepisteasetus")?;
            let settings = &mut project.virhepisteet;
            let target = match field {
                OverrideField::FreeTime => &mut settings.vapaa_aika_sekunteina,
                OverrideField::EarlyRate => &mut settings.ennakon_pisteet_sekunnilta,
                OverrideField::LateRate => &mut settings.myohastymisen_pisteet_sekunnilta,
                OverrideField::MissingAt => &mut settings.puuttuva_at,
                OverrideField::ExtraAt => &mut settings.ylimaarainen_at,
                OverrideField::MissingJat => &mut settings.puuttuva_jat,
                OverrideField::SectionLimit => &mut settings.jakson_raja_minuutteina,
                OverrideField::AtMaximum => &mut settings.at_enimmaispisteet,
                OverrideField::JatMaximum => &mut settings.jat_enimmaispisteet,
                OverrideField::MissingRt => &mut settings.puuttuva_rt,
                OverrideField::ExtraRt => &mut settings.ylimaarainen_rt,
            };
            *target = value;
            Ok(())
        })
    }
}

fn validate_station_rule(station: &Station) -> Result<(), CoreError> {
    if matches!(station.saanto, 5..=7) && station.tyyppi.trim() != "ULA" {
        return Err(invalid(
            "ULA-pyöristyssääntö sopii vain ULA-asemalle. Vaihda laskentasääntö ensin normaaliksi ennen aseman tyypin vaihtamista",
        ));
    }
    Ok(())
}

fn find_track(project: &mut Project, number: u32) -> Result<&mut Track, CoreError> {
    project
        .radat
        .iter_mut()
        .find(|track| track.numero == number)
        .ok_or_else(|| invalid("Rataa ei löydy; päivitä näkymä"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverField {
    Number,
    Class,
    Driver,
    Navigator,
    Card,
    Car,
    Start,
    Correction,
    MissingRtCount,
    ExtraRtCount,
    OtherPenalties,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassField {
    Code,
    Name,
    Route,
    FirstNumber,
    Start,
    Interval,
    Lateness,
    Np,
    Sp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackField {
    Number,
    Name,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StationField {
    Kind,
    Name,
    Code,
    Target,
    Rule,
    Section,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompetitionField {
    Number,
    Name,
    Timing,
    CustomSeries,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideField {
    FreeTime,
    EarlyRate,
    LateRate,
    MissingAt,
    ExtraAt,
    MissingJat,
    SectionLimit,
    AtMaximum,
    JatMaximum,
    MissingRt,
    ExtraRt,
}

/// Lomakkeen keskeneräisiä tekstiarvoja ei viedä kilpailumalliin ennen hyväksyntää.
#[derive(Debug, Clone, Default)]
pub struct DriverEdit {
    pub number: String,
    pub class_code: String,
    pub driver: String,
    pub navigator: String,
    pub card_number: String,
    pub car: String,
    pub start_time: String,
    pub correction_seconds: String,
}

impl DriverEdit {
    fn parse(self) -> Result<Driver, CoreError> {
        Ok(Driver {
            numero: number(&self.number, "Kilpailijan numero")?,
            luokka: required(&self.class_code, "Luokka")?,
            kuljettaja: required(&self.driver, "Kuljettaja")?,
            kartanlukija: self.navigator.trim().into(),
            emit: number(&self.card_number, "Emit-numero")?,
            auto: self.car.trim().into(),
            lahtoaika: self.start_time.trim().into(),
            emit_korjaus_sekuntia: optional_number(&self.correction_seconds, "Emit-korjaus")?
                .unwrap_or(0),
            lisavirheet: Default::default(),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct ClassEdit {
    pub code: String,
    pub name: String,
    pub route: String,
    pub first_number: String,
    pub start_time: String,
    pub interval_minutes: String,
    pub lateness_minutes: String,
    pub np_enabled: String,
    pub series_method: String,
}

impl ClassEdit {
    fn parse(self) -> Result<Class, CoreError> {
        Ok(Class {
            tunnus: required(&self.code, "Luokan tunnus")?,
            nimi: required(&self.name, "Luokan nimi")?,
            rata: number(&self.route, "Radan numero")?,
            ensimmainen_numero: number(&self.first_number, "Ensimmäinen numero")?,
            lahtoaika: self.start_time.trim().into(),
            lahtovali_minuutteina: number(&self.interval_minutes, "Lähtöväli")?,
            myohastymisraja_minuutteina: number(&self.lateness_minutes, "Myöhästymisraja")?,
            nousupisteet: optional_number(&self.np_enabled, "Nousupistevalinta")?.unwrap_or(0),
            sarjapistetapa: optional_number(&self.series_method, "Sarjapistemenetelmä")?
                .unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct TrackEdit {
    pub number: String,
    pub name: String,
}

#[derive(Debug, Clone, Default)]
pub struct StationEdit {
    pub kind: String,
    pub name: String,
    pub code: String,
    pub target_seconds: String,
    pub rule: String,
    pub section_minutes: String,
}

impl StationEdit {
    fn parse(self) -> Result<Station, CoreError> {
        Ok(Station {
            tyyppi: required(&self.kind, "Aseman tyyppi")?,
            nimi: required(&self.name, "Aseman nimi")?,
            emit_koodi: number(&self.code, "Emit-koodi")?,
            tavoiteaika_sekuntia: number(&self.target_seconds, "Tavoiteaika")?,
            saanto: optional_number(&self.rule, "Laskentasääntö")?.unwrap_or(0),
            jakso_minuutteina: optional_number(&self.section_minutes, "Jakson minuutit")?,
        })
    }
}
