//! Käyttöliittymien yhteinen avattu kilpailu. Muokkaus tallentuu vain yhtenä,
//! validoituna muutoksena; näkymät johdetaan samasta onnistuneesta tilasta.

use crate::{
    ClockTime, CoreError, application,
    emit::EmitFrame,
    legacy::{Archive, Encoding},
    native::{self, Class, CombinedClass, Driver, Project, ReadOutcome, Station, Track},
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

fn card_number(value: &str) -> Result<u32, CoreError> {
    match optional_number(value, "Emit-numero")? {
        Some(0) => Err(invalid("Anna Emit-numero tai jätä kenttä tyhjäksi")),
        Some(number) => Ok(number),
        None => Ok(0),
    }
}

enum Source {
    Native(Box<ProjectStore>),
    Legacy(Box<Archive>),
}

/// Käyttöliittymästä riippumaton kilpailu. Työpöytäominaisuus voi liittää LAN-asiakkaan.
pub struct Workspace {
    path: PathBuf,
    source: Source,
    bc_view: bool,
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
            Source::Legacy(Box::new(
                Archive::load(&path, Encoding::Windows1252).map_err(|e| invalid(e.to_string()))?,
            ))
        } else {
            Source::Native(Box::new(ProjectStore::open(&path)?))
        };
        Ok(Self {
            path,
            source,
            bc_view: false,
        })
    }

    pub fn create(directory: &Path, name: &str) -> Result<Self, CoreError> {
        // Nimi validoidaan ennen kansion tai tiedoston luontia.
        let project = Project::new(name)?;
        fs::create_dir_all(directory).map_err(|e| invalid(crate::io_message(&e)))?;
        let store = ProjectStore::create(&directory.join(native::FILE_NAME), project)?;
        Ok(Self {
            path: store.path().to_owned(),
            source: Source::Native(Box::new(store)),
            bc_view: false,
        })
    }

    /// Epäonnistunut uudelleenavaus jättää nykyisen onnistuneen tilan näkyviin.
    pub fn reload(&mut self) -> Result<(), CoreError> {
        #[cfg(feature = "desktop")]
        if self.lan_client().is_some() {
            self.poll_lan()?;
            return Ok(());
        }
        let mut reopened = Self::open(&self.path)?;
        reopened.bc_view = self.bc_view
            && reopened
                .project()
                .is_some_and(|p| p.beginners_cup.is_some());
        *self = reopened;
        Ok(())
    }

    #[cfg(feature = "desktop")]
    pub fn attach_lan(&mut self, client: crate::lan::Client) -> Result<(), CoreError> {
        match &mut self.source {
            Source::Native(store) => {
                store.attach_lan(client);
                store.poll_lan()?;
                Ok(())
            }
            _ => Err(invalid("LAN-yhteiskäyttö tarvitsee AS2026-kilpailun")),
        }
    }
    #[cfg(feature = "desktop")]
    pub fn lan_client(&self) -> Option<&crate::lan::Client> {
        match &self.source {
            Source::Native(store) => store.lan_client(),
            _ => None,
        }
    }
    #[cfg(feature = "desktop")]
    pub fn pin_lan_basis(&self) {
        if let Source::Native(store) = &self.source {
            store.pin_edit_basis();
        }
    }
    #[cfg(feature = "desktop")]
    pub fn synchronize_lan(&mut self) -> Result<(), CoreError> {
        if let Some(client) = self.lan_client() {
            client.refresh()?;
        }
        self.poll_lan()?;
        Ok(())
    }
    #[cfg(feature = "desktop")]
    pub fn poll_lan(&mut self) -> Result<bool, CoreError> {
        match &mut self.source {
            Source::Native(store) => store.poll_lan(),
            _ => Ok(false),
        }
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

    pub fn bc_view(&self) -> bool {
        self.bc_view
    }

    pub fn set_bc_view(&mut self, selected: bool) -> Result<(), CoreError> {
        if selected {
            crate::beginners::projection(
                self.project()
                    .ok_or_else(|| invalid("Avaa AS2026-kilpailu"))?,
            )?;
        }
        self.bc_view = selected;
        Ok(())
    }

    pub fn result_project(&self) -> Result<Option<Project>, CoreError> {
        self.project()
            .map(|p| {
                if self.bc_view {
                    crate::beginners::projection(p)
                } else {
                    Ok(p.clone())
                }
            })
            .transpose()
    }

    fn update_result<T>(
        &mut self,
        edit: impl FnOnce(&mut Project) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        if !self.bc_view {
            return self.update(edit);
        }
        self.update(|project| {
            let mut view = crate::beginners::projection(project)?;
            let before = view.clone();
            let value = edit(&mut view)?;
            crate::corrections::invalidate_input_changes(&before, &mut view);
            view.validate()?;
            crate::beginners::store_projection(project, &view)?;
            Ok(value)
        })
    }

    pub fn save_beginners_cup(
        &mut self,
        config: crate::beginners::BeginnersCup,
        participants: &[u32],
    ) -> Result<(), CoreError> {
        self.update(|project| {
            let mut selected = std::collections::BTreeSet::new();
            for &number in participants {
                if !selected.insert(number)
                    || !crate::beginners::eligible(project, number, config.lahderata)
                {
                    return Err(invalid("BC-osallistujavalinta on virheellinen"));
                }
            }
            for entry in &mut project.bc_osallistujat {
                entry.osallistuu = selected.contains(&entry.numero);
            }
            for &number in participants {
                if !project.bc_osallistujat.iter().any(|e| e.numero == number) {
                    project.bc_osallistujat.push(crate::beginners::Entry {
                        numero: number,
                        osallistuu: true,
                        ..Default::default()
                    });
                }
            }
            project.beginners_cup = Some(Box::new(config));
            Ok(())
        })
    }

    /// Tulospaneelin erilliset pisteet ja tila; NAC-kilpailijatietoja ei muuteta BC-näkymässä.
    pub fn edit_result_driver(
        &mut self,
        id: u32,
        field: DriverField,
        value: &str,
    ) -> Result<(), CoreError> {
        if !matches!(
            field,
            DriverField::MissingRtCount
                | DriverField::ExtraRtCount
                | DriverField::OtherPenalties
                | DriverField::Status
                | DriverField::StatusReason
        ) {
            return Err(invalid("Valitse tuloskohtainen piste- tai tilakenttä"));
        }
        self.update_result(|project| {
            let parameters = project
                .to_archive()?
                .scoring_parameters()
                .map_err(|e| invalid(e.to_string()))?;
            let driver = project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == id)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
            match field {
                DriverField::MissingRtCount => {
                    let count: u32 = number(value, "Puuttuvat RT-asemat")?;
                    driver.lisavirheet.puuttuvat_rt_kpl = count;
                    driver.lisavirheet.puuttuvat_rt_pisteet = i64::from(count)
                        .checked_mul(parameters.missing_rt)
                        .ok_or(CoreError::ArithmeticOverflow("RT-pisteet"))?;
                }
                DriverField::ExtraRtCount => {
                    let count: u32 = number(value, "Ylimääräiset RT-asemat")?;
                    driver.lisavirheet.ylimaaraiset_rt_kpl = count;
                    driver.lisavirheet.ylimaaraiset_rt_pisteet = i64::from(count)
                        .checked_mul(parameters.extra_rt)
                        .ok_or(CoreError::ArithmeticOverflow("RT-pisteet"))?;
                }
                DriverField::OtherPenalties => {
                    driver.lisavirheet.muut_virhepisteet = number(value, "Muut virhepisteet")?
                }
                DriverField::Status => {
                    driver.tila = if value.trim().is_empty() {
                        None
                    } else {
                        Some(native::ManualStatus::new(
                            value.trim(),
                            driver.tila.as_ref().map_or("", |s| s.syy.as_str()),
                        )?)
                    };
                    driver.keskeytys_vahvistettu = false;
                }
                DriverField::StatusReason => {
                    driver
                        .tila
                        .as_mut()
                        .ok_or_else(|| invalid("Valitse ensin tila"))?
                        .syy = value.trim().into();
                    driver.keskeytys_vahvistettu = false;
                }
                _ => unreachable!(),
            }
            Ok(())
        })
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
            Source::Legacy(archive) => Ok((**archive).clone()),
        }
    }

    pub fn calculation(&self) -> Result<application::CalculationReport, CoreError> {
        application::calculate(&self.archive()?)
    }

    /// Vanhan aineiston tallennetut tulokset ja uuden kilpailun johdetut
    /// asematulokset pidetään erillään nykyisen sovellusytimen mukaisesti.
    pub fn results(&self) -> Result<Archive, CoreError> {
        match &self.source {
            Source::Native(store) if self.bc_view => crate::beginners::results(store.project()),
            Source::Native(store) => application::calculated_project(store.project()),
            Source::Legacy(archive) => Ok((**archive).clone()),
        }
    }

    /// Tallennettu korjaus on ensisijainen; lukemattomalle kortille ei keksitä tulosta.
    pub fn result_snapshot(
        &self,
        number: u32,
    ) -> Result<Option<crate::corrections::StoredResult>, CoreError> {
        let result_project = self.result_project()?;
        let Some(project) = result_project.as_ref() else {
            return Ok(None);
        };
        let driver = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
        if let Some(stored) = &driver.tallennettu_tulos {
            return Ok(Some(stored.clone()));
        }
        if project.active_readout(driver.emit).is_none() {
            return Ok(None);
        }
        application::capture_project_result(project, number).map(Some)
    }

    /// Keskeytyspäätös voidaan vahvistaa ilman korttia tai asematulosta.
    pub fn can_confirm_interruption(&self, number: u32) -> bool {
        self.result_project()
            .ok()
            .flatten()
            .as_ref()
            .and_then(|p| p.kilpailijat.iter().find(|d| d.numero == number))
            .is_some_and(|driver| {
                driver.can_confirm_interruption() && !driver.has_confirmed_interruption()
            })
    }
    pub fn interruption_confirmed(&self, number: u32) -> bool {
        self.result_project()
            .ok()
            .flatten()
            .as_ref()
            .and_then(|p| p.kilpailijat.iter().find(|d| d.numero == number))
            .is_some_and(|driver| driver.has_confirmed_interruption())
    }
    /// Tallentaa vain toimitsijan KESK-päätöksen ja perustelun. Raakaleimat,
    /// asemarivit, kellot ja pisteet säilyvät täsmälleen ennallaan.
    pub fn confirm_interruption(&mut self, number: u32, reason: &str) -> Result<(), CoreError> {
        let reason = native::validate_interruption_reason(reason)?.to_owned();
        self.update_result(|project| {
            let driver = project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
            driver.tila = Some(native::ManualStatus::new("KESK", &reason)?);
            driver.keskeytys_vahvistettu = true;
            Ok(())
        })
    }

    pub fn approve_result(&mut self, number: u32) -> Result<(), CoreError> {
        if let Some(reason) = self
            .result_project()?
            .as_ref()
            .and_then(|p| p.kilpailijat.iter().find(|d| d.numero == number))
            .and_then(|d| d.tila.as_ref())
            .filter(|status| status.tunnus == "KESK")
            .map(|status| status.syy.clone())
        {
            return self.confirm_interruption(number, &reason);
        }
        let mut snapshot = self
            .result_snapshot(number)?
            .ok_or_else(|| invalid("Lue ensin kilpailijan kortti"))?;
        if !snapshot.can_approve() {
            return Err(invalid(
                "Täydennä puuttuvat käsikellot ja valitse Tutki ennen hyväksymistä",
            ));
        }
        snapshot.hyvaksytty = true;
        self.update_result(|project| {
            application::validate_project_result(project, number, &snapshot)?;
            project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?
                .tallennettu_tulos = Some(snapshot);
            Ok(())
        })
    }

    /// Ensimmäinen lukema saa asemarivit. Uusintaluku säilyttää aiemman tarkastuksen.
    pub fn ensure_readout_result(&mut self, number: u32) -> Result<(), CoreError> {
        #[cfg(feature = "desktop")]
        if let Source::Native(store) = &self.source {
            let basis = store.take_edit_basis();
            let result = self.ensure_readout_result_current(number);
            if let Source::Native(store) = &self.source {
                store.restore_edit_basis(basis);
            }
            return result;
        }
        self.ensure_readout_result_current(number)
    }
    fn ensure_readout_result_current(&mut self, number: u32) -> Result<(), CoreError> {
        self.update(|project| {
            if project
                .kilpailijat
                .iter()
                .any(|d| d.numero == number && d.tallennettu_tulos.is_none())
            {
                let result = application::capture_project_result(project, number)?;
                project
                    .kilpailijat
                    .iter_mut()
                    .find(|d| d.numero == number)
                    .unwrap()
                    .tallennettu_tulos = Some(result);
            }
            Ok(())
        })?;
        // BC:n epäselvä kohdistus ei saa perua onnistunutta NAC-tarkastusta.
        self.update(|project| {
            if project
                .bc_osallistujat
                .iter()
                .any(|e| e.numero == number && e.osallistuu && e.tallennettu_tulos.is_none())
            {
                let view = crate::beginners::projection(project)?;
                let result = application::capture_project_result(&view, number)?;
                project
                    .bc_osallistujat
                    .iter_mut()
                    .find(|e| e.numero == number)
                    .unwrap()
                    .tallennettu_tulos = Some(result);
            }
            Ok(())
        })
    }

    pub fn create_imported(directory: &Path, project: Project) -> Result<Self, CoreError> {
        project.validate()?;
        fs::create_dir_all(directory).map_err(|e| invalid(crate::io_message(&e)))?;
        let store = ProjectStore::create(&directory.join(native::FILE_NAME), project)?;
        Ok(Self {
            path: store.path().to_owned(),
            source: Source::Native(Box::new(store)),
            bc_view: false,
        })
    }

    pub fn ranking(&self) -> Result<RankingReport, CoreError> {
        application::rank_stored(&self.results()?, TiePolicy::PairwiseControlComparison)
    }

    fn update<T>(
        &mut self,
        edit: impl FnOnce(&mut Project) -> Result<T, CoreError>,
    ) -> Result<T, CoreError> {
        match &mut self.source {
            Source::Native(store) => store.update(|project| {
                let before = project.clone();
                let value = edit(project)?;
                crate::corrections::invalidate_input_changes(&before, project);
                Ok(value)
            }),
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
            let use_default = edit.correction_seconds.trim().is_empty();
            let mut driver = edit.parse()?;
            if use_default {
                driver.emit_korjaus_sekuntia = project
                    .virhepisteet
                    .oletus_emit_korjaus_sekuntia
                    .unwrap_or(120);
            }
            driver.emit_korjaus_annettu = true;
            project.kilpailijat.push(driver);
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
                DriverField::Card => driver.emit = card_number(value)?,
                DriverField::DriverClub => driver.kuljettajan_seura = value.trim().into(),
                DriverField::NavigatorClub => driver.kartanlukijan_seura = value.trim().into(),
                DriverField::DriverLicence => driver.kuljettajan_lisenssi = value.trim().into(),
                DriverField::NavigatorLicence => driver.kartanlukijan_lisenssi = value.trim().into(),
                DriverField::Status => {
                    driver.tila = if value.trim().is_empty() { None } else {
                        Some(native::ManualStatus::new(value.trim(), driver.tila.as_ref().map_or("", |s| s.syy.as_str()))?)
                    };
                }
                DriverField::StatusReason => {
                    let state = driver.tila.as_mut().ok_or_else(|| invalid("Valitse ensin kilpailijan tila; automaattisen tilan syy määräytyy laskennasta"))?;
                    state.syy = value.trim().into();
                }
                DriverField::Car => driver.auto = value.trim().into(),
                DriverField::Start => driver.lahtoaika = value.trim().into(),
                DriverField::Correction => {
                    driver.emit_korjaus_sekuntia = number(value, "Emit-korjaus")?;
                    driver.emit_korjaus_annettu = true;
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
            if matches!(field, DriverField::Status | DriverField::StatusReason) {
                driver.keskeytys_vahvistettu = false;
            }
            if field == DriverField::Number {
                for entry in &mut project.bc_osallistujat {
                    if entry.numero == id { entry.numero = driver.numero; }
                }
            }
            if matches!(field, DriverField::Class | DriverField::Card | DriverField::Start | DriverField::Correction | DriverField::MissingRtCount | DriverField::ExtraRtCount | DriverField::OtherPenalties | DriverField::Status | DriverField::StatusReason)
                && let Some(result) = &mut driver.tallennettu_tulos {
                result.hyvaksytty = false;
                if matches!(field, DriverField::Class | DriverField::Start | DriverField::Correction) { result.kasikellot_muuttuneet = true; }
            }
            Ok(())
        })
    }

    /// Tarkista muodostaa asematuloksen uudelleen kortinlukuhistoriasta.
    pub fn review_results(&mut self, number: u32) -> Result<(), CoreError> {
        self.update_result(|project| {
            let result = application::capture_project_result(project, number)?;
            project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?
                .tallennettu_tulos = Some(result);
            Ok(())
        })
    }

    /// Tutki käyttää tallennettuja asemarivejä ja mahdollisia käsikelloja.
    /// Kortinluvun jälkeinen syöttö: käsikellot ja lisävirheet yhtenä muutoksena,
    /// laskenta automaattisesti. Tulostaminen ei tarvitse erillistä hyväksymistä.
    pub fn complete_result_entry(
        &mut self,
        number: u32,
        clocks: &[(u32, [i64; 3])],
        missing_rt: u32,
        other: i64,
    ) -> Result<(), CoreError> {
        if other < 0 {
            return Err(invalid("Muut virhepisteet eivät saa olla negatiivisia"));
        }
        let terminal_status = self
            .result_project()?
            .as_ref()
            .and_then(|p| p.kilpailijat.iter().find(|d| d.numero == number))
            .and_then(|d| d.tila.as_ref())
            .map(|s| s.tunnus.clone());
        if terminal_status
            .as_deref()
            .is_some_and(|s| matches!(s, "KESK" | "PER" | "HYL" | "SULJ" | "EILÄ"))
        {
            self.edit_result_driver(number, DriverField::MissingRtCount, &missing_rt.to_string())?;
            self.edit_result_driver(number, DriverField::OtherPenalties, &other.to_string())?;
            return if terminal_status.as_deref() == Some("KESK") {
                self.approve_result(number)
            } else {
                Ok(())
            };
        }
        self.update_result(|project| {
            let driver = project
                .kilpailijat
                .iter()
                .find(|d| d.numero == number)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
            let mut result = driver
                .tallennettu_tulos
                .clone()
                .map_or_else(|| application::capture_project_result(project, number), Ok)?;
            let expected = result
                .rivit
                .iter()
                .filter(|r| r.expected_code == 0 && r.station_type != "L")
                .map(|r| r.output_index)
                .collect::<std::collections::BTreeSet<_>>();
            let supplied = clocks
                .iter()
                .map(|(r, _)| *r)
                .collect::<std::collections::BTreeSet<_>>();
            if expected != supplied || supplied.len() != clocks.len() {
                return Err(invalid(
                    "Täydennä kaikki käsikellot; asemavalinta on muuttunut",
                ));
            }
            let mut changed = result.kasikellot_muuttuneet;
            for &(row, clock) in clocks {
                let old = result.rivit.iter().find(|r| r.output_index == row).unwrap();
                if old.clock != clock || !result.kasin_annetut_kellot.contains(&row) {
                    result.set_clock(row, clock)?;
                    changed = true;
                }
            }
            if changed {
                result = application::recalculate_stored_result(project, number, &result)?;
            }
            let rate = project
                .virhepisteet
                .puuttuva_rt
                .unwrap_or(crate::scoring::ScoringParameters::default().missing_rt);
            let points = i64::from(missing_rt)
                .checked_mul(rate)
                .ok_or(CoreError::ArithmeticOverflow("RT-pisteet"))?;
            let driver = project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .unwrap();
            driver.lisavirheet.puuttuvat_rt_kpl = missing_rt;
            driver.lisavirheet.puuttuvat_rt_pisteet = points;
            driver.lisavirheet.muut_virhepisteet = other;
            driver.tallennettu_tulos = Some(result);
            Ok(())
        })?;
        self.approve_result(number)
    }

    pub fn recalculate_stored(&mut self, number: u32) -> Result<(), CoreError> {
        self.update_result(|project| {
            let stored = project
                .kilpailijat
                .iter()
                .find(|d| d.numero == number)
                .and_then(|d| d.tallennettu_tulos.as_ref())
                .ok_or_else(|| invalid("Valitse ensin Tarkista tai tallenna asemakorjaus"))?;
            let result = application::recalculate_stored_result(project, number, stored)?;
            project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .unwrap()
                .tallennettu_tulos = Some(result);
            Ok(())
        })
    }

    /// Yhteinen korttikorjaus koskee NAC- ja BC-laskentaa. Kumpikin tarkistetaan
    /// uudelleen; alkuperäiset leimat ja erilliset lisävirheet säilyvät.
    pub fn correct_emit_row(
        &mut self,
        number: u32,
        request: &crate::emit_corrections::Request,
    ) -> Result<(), CoreError> {
        if self.bc_view {
            return Err(invalid(
                "Korjaa yhteinen Emit-leima NAC / perusluokat -näkymässä",
            ));
        }
        self.update(|project| crate::emit_corrections::apply(project, number, request))
    }

    /// Muutos ja ensimmäisen tuloskopion luonti tallentuvat yhtenä tapahtumana.
    pub fn edit_station_result(
        &mut self,
        competitor_number: u32,
        row: u32,
        field: ResultField,
        value: &str,
    ) -> Result<(), CoreError> {
        self.update_result(|project| {
            let mut result = match project
                .kilpailijat
                .iter()
                .find(|d| d.numero == competitor_number)
                .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?
                .tallennettu_tulos
                .clone()
            {
                Some(result) => result,
                None => application::capture_project_result(project, competitor_number)?,
            };
            match field {
                ResultField::Clock => {
                    let parsed = crate::terminal::parse_clock(value)?;
                    let clock = [
                        i64::from(parsed.hour),
                        i64::from(parsed.minute),
                        i64::from(parsed.second),
                    ];
                    result.set_clock(row, clock)?;
                }
                ResultField::TimePenalty | ResultField::OtherPenalty => {
                    let old = result
                        .rivit
                        .iter()
                        .find(|r| r.output_index == row)
                        .ok_or_else(|| invalid("Asemariviä ei löydy"))?;
                    let (signed, other) = if field == ResultField::TimePenalty {
                        (number(value, "Aikavirhe")?, old.other_penalty)
                    } else {
                        (old.signed_time_penalty, number(value, "Muu virhe")?)
                    };
                    result.set_row_penalties(row, signed, other)?;
                }
            }
            project
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == competitor_number)
                .unwrap()
                .tallennettu_tulos = Some(result);
            Ok(())
        })
    }

    /// Tallentaa arvonnan luokkavalinnat yhdessä; muiden tietojen arvot säilyvät.
    pub fn set_draw_classes(&mut self, codes: &[String]) -> Result<(), CoreError> {
        self.update(|project| {
            let mut selected = std::collections::HashSet::new();
            for code in codes {
                let code = code.trim();
                if !project
                    .luokat
                    .iter()
                    .any(|class| class.tunnus.trim() == code)
                {
                    return Err(invalid(format!(
                        "Luokkaa {code:?} ei löydy lähtöarvonnan valinnoista"
                    )));
                }
                if !selected.insert(code) {
                    return Err(invalid(format!(
                        "Luokka {code} on valittu useammin kuin kerran"
                    )));
                }
            }
            for class in &mut project.luokat {
                class.osallistuu_lahtoarvontaan = selected.contains(class.tunnus.trim());
            }
            Ok(())
        })
    }

    pub fn apply_start_draw(&mut self, draw: &crate::starts::StartDraw) -> Result<(), CoreError> {
        self.update(|project| draw.apply(project))
    }

    pub fn replace_emit_content(
        &mut self,
        number: u32,
        punches: Vec<native::ReadPunch>,
    ) -> Result<(), CoreError> {
        match &mut self.source {
            Source::Native(store) => {
                store.update(|p| crate::emit_corrections::replace_content(p, number, punches))
            }
            _ => Err(invalid("Avaa muokattava kilpailu")),
        }
    }

    pub fn save_emit_card_from(
        &mut self,
        number: u32,
        source_card: u32,
        card: u32,
        punches: Vec<native::ReadPunch>,
        new: bool,
    ) -> Result<(), CoreError> {
        match &mut self.source {
            Source::Native(store) => store.update(|p| {
                if !new {
                    if p.kilpailijat
                        .iter()
                        .any(|d| d.numero != number && d.emit == source_card)
                    {
                        return Err(invalid("Korttiluku kuuluu toiselle kilpailijalle"));
                    }
                    p.kilpailijat
                        .iter_mut()
                        .find(|d| d.numero == number)
                        .ok_or_else(|| invalid("Kilpailija puuttuu"))?
                        .emit = source_card;
                }
                crate::emit_corrections::save_card(p, number, card, punches, new)
            }),
            _ => Err(invalid("Avaa muokattava kilpailu")),
        }
    }
    pub fn save_emit_card(
        &mut self,
        number: u32,
        card: u32,
        punches: Vec<native::ReadPunch>,
        new: bool,
    ) -> Result<(), CoreError> {
        match &mut self.source {
            Source::Native(store) => {
                store.update(|p| crate::emit_corrections::save_card(p, number, card, punches, new))
            }
            _ => Err(invalid("Avaa muokattava kilpailu")),
        }
    }

    pub fn delete_class(&mut self, code: &str) -> Result<(), CoreError> {
        self.update(|project| {
            if project
                .kilpailijat
                .iter()
                .any(|d| d.luokka.trim() == code.trim())
            {
                return Err(invalid("Siirrä luokan kilpailijat ensin toiseen luokkaan"));
            }
            let index = project
                .luokat
                .iter()
                .position(|c| c.tunnus == code)
                .ok_or_else(|| invalid("Luokkaa ei löydy"))?;
            project.luokat.remove(index);
            for group in &mut project.yhdistetyt_luokat {
                group.luokat.retain(|c| c != code);
            }
            project.yhdistetyt_luokat.retain(|g| !g.luokat.is_empty());
            Ok(())
        })
    }

    pub fn rebuild_route_results(&mut self, tracks: &[u32]) -> Result<(), CoreError> {
        let Source::Native(store) = &mut self.source else {
            return Err(invalid("Avaa muokattava kilpailu"));
        };
        store.update(|project| {
            let update_bc = project
                .beginners_cup
                .as_ref()
                .is_some_and(|c| tracks.contains(&c.lahderata));
            if update_bc {
                let config = project.beginners_cup.as_mut().unwrap();
                let stations = &project
                    .radat
                    .iter()
                    .find(|r| r.numero == config.lahderata)
                    .unwrap()
                    .asemat;
                let mut selected = Vec::new();
                for (i, station) in stations.iter().enumerate() {
                    let old = config
                        .lahdeasemat
                        .iter()
                        .position(|s| s.tyyppi == station.tyyppi && s.nimi == station.nimi)
                        .or_else(|| {
                            (stations.len() == config.lahdeasemat.len()
                                && config.lahdeasemat[i].tyyppi == station.tyyppi)
                                .then_some(i)
                        });
                    if matches!(station.tyyppi.as_str(), "L" | "ULA")
                        || old.is_some_and(|i| config.mukana_asemat.contains(&((i + 1) as u32)))
                    {
                        selected.push((i + 1) as u32);
                    }
                }
                config.lahdeasemat = stations.clone();
                config.mukana_asemat = selected;
            }
            let numbers = project
                .kilpailijat
                .iter()
                .filter(|d| {
                    d.tallennettu_tulos.is_some()
                        && project
                            .luokat
                            .iter()
                            .any(|c| c.tunnus == d.luokka && tracks.contains(&c.rata))
                })
                .map(|d| d.numero)
                .collect::<Vec<_>>();
            for number in numbers {
                let driver = project
                    .kilpailijat
                    .iter()
                    .find(|d| d.numero == number)
                    .unwrap();
                let result = crate::emit_corrections::recapture(
                    project,
                    number,
                    driver.tallennettu_tulos.as_ref(),
                )?;
                project
                    .kilpailijat
                    .iter_mut()
                    .find(|d| d.numero == number)
                    .unwrap()
                    .tallennettu_tulos = Some(result);
            }
            if update_bc {
                let projection = crate::beginners::projection(project)?;
                for entry in &mut project.bc_osallistujat {
                    if entry.tallennettu_tulos.is_none() {
                        continue;
                    }
                    entry.tallennettu_tulos = if entry.osallistuu {
                        Some(crate::emit_corrections::recapture(
                            &projection,
                            entry.numero,
                            entry.tallennettu_tulos.as_ref(),
                        )?)
                    } else {
                        None
                    };
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
                ClassField::Interval => {
                    class.lahtovali_sekuntia = Some(number(value, "Lähtöväli")?)
                }
                ClassField::Lateness => {
                    class.myohastymisraja_minuutteina = number(value, "Myöhästymisraja")?
                }
                ClassField::Np => class.nousupisteet = number(value, "Nousupistevalinta")?,
                ClassField::Draw => {
                    class.osallistuu_lahtoarvontaan = match value.trim() {
                        "0" => false,
                        "1" => true,
                        _ => return Err(invalid("Lähtöarvonnan valinnan pitää olla Kyllä tai Ei")),
                    }
                }
                ClassField::Sp => class.sarjapistetapa = number(value, "Sarjapistemenetelmä")?,
            }
            if field == ClassField::Code {
                let new_code = class.tunnus.clone();
                for driver in &mut project.kilpailijat {
                    if driver.luokka.trim() == id.trim() {
                        driver.luokka = new_code.clone();
                    }
                }
                for group in &mut project.yhdistetyt_luokat {
                    for member in &mut group.luokat {
                        if member == id.trim() {
                            *member = new_code.clone();
                        }
                    }
                }
            }
            Ok(())
        })
    }

    /// Tallentaa yhdistetyn luokan ja jäsenvalinnat yhtenä validoituna muutoksena.
    pub fn save_combined_class(
        &mut self,
        index: usize,
        mut group: CombinedClass,
    ) -> Result<(), CoreError> {
        self.update(|project| {
            if index > project.yhdistetyt_luokat.len() || index >= 4 {
                return Err(invalid("Yhdistetyn luokan järjestysnumero ei kelpaa"));
            }
            group.nimi = required(&group.nimi, "Yhdistetyn luokan nimi")?;
            if group.luokat.is_empty() {
                return Err(invalid("Valitse vähintään yksi yhdistettävä luokka"));
            }
            if index == project.yhdistetyt_luokat.len() {
                project.yhdistetyt_luokat.push(group);
            } else {
                project.yhdistetyt_luokat[index] = group;
            }
            Ok(())
        })
    }

    /// Atominen yhdistetyn luokan muokkaus: nimi | luokat | Np | Sp.
    /// Tyhjä arvo poistaa olemassa olevan ryhmän.
    pub fn edit_combined_class(&mut self, index: usize, value: &str) -> Result<(), CoreError> {
        self.update(|project| {
            if index > project.yhdistetyt_luokat.len() || index >= 4 {
                return Err(invalid("Yhdistetyn luokan järjestysnumero ei kelpaa"));
            }
            if value.trim().is_empty() {
                if index == project.yhdistetyt_luokat.len() {
                    return Ok(());
                }
                project.yhdistetyt_luokat.remove(index);
                return Ok(());
            }
            let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
            if parts.len() != 4 {
                return Err(invalid(
                    "Anna yhdistetty luokka muodossa Nimi | A,B | Np | Sp",
                ));
            }
            let members = parts[1]
                .split(',')
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>();
            let group = CombinedClass {
                nimi: required(parts[0], "Yhdistetyn luokan nimi")?,
                luokat: members,
                nousupisteet: number(parts[2], "Yhdistetyn luokan Np")?,
                sarjapistetapa: number(parts[3], "Yhdistetyn luokan Sp")?,
            };
            if index == project.yhdistetyt_luokat.len() {
                project.yhdistetyt_luokat.push(group);
            } else {
                project.yhdistetyt_luokat[index] = group;
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
                if let Some(bc) = &mut project.beginners_cup
                    && bc.lahderata == id
                {
                    bc.lahderata = new_number;
                }
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
                CompetitionField::TotalLateness => {
                    project.virhepisteet.kokonaismyohastymisraja_minuutteina =
                        optional_number(value, "Kokonaismyöhästymisraja")?;
                }
                CompetitionField::DefaultEmitCorrection => {
                    project.virhepisteet.oletus_emit_korjaus_sekuntia =
                        optional_number(value, "Oletus Emit-korjaus")?;
                }
                CompetitionField::StartColumns => {
                    project.virhepisteet.lahtoluettelon_kentat = value
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                    crate::starts::columns(project)?;
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
pub enum ResultField {
    Clock,
    TimePenalty,
    OtherPenalty,
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
    DriverLicence,
    NavigatorLicence,
    Status,
    StatusReason,
    DriverClub,
    NavigatorClub,
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
    Draw,
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
    TotalLateness,
    DefaultEmitCorrection,
    StartColumns,
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
            emit: card_number(&self.card_number)?,
            auto: self.car.trim().into(),
            lahtoaika: self.start_time.trim().into(),
            emit_korjaus_annettu: false,
            emit_korjaus_sekuntia: optional_number(&self.correction_seconds, "Emit-korjaus")?
                .unwrap_or(0),
            lisavirheet: Default::default(),
            kuljettajan_seura: String::new(),
            kartanlukijan_seura: String::new(),
            kuljettajan_lisenssi: String::new(),
            kartanlukijan_lisenssi: String::new(),
            tila: None,
            keskeytys_vahvistettu: false,
            tallennettu_tulos: None,
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
    pub interval_seconds: String,
    pub lateness_minutes: String,
    pub np_enabled: String,
    pub series_method: String,
}

impl ClassEdit {
    fn parse(self) -> Result<Class, CoreError> {
        Ok(Class {
            osallistuu_lahtoarvontaan: true,
            tunnus: required(&self.code, "Luokan tunnus")?,
            nimi: required(&self.name, "Luokan nimi")?,
            rata: number(&self.route, "Radan numero")?,
            ensimmainen_numero: number(&self.first_number, "Ensimmäinen numero")?,
            lahtoaika: self.start_time.trim().into(),
            lahtovali_minuutteina: 0,
            lahtovali_sekuntia: Some(number(&self.interval_seconds, "Lähtöväli")?),
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
