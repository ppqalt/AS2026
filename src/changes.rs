//! Pienet kilpailumuutokset yhteiseen malliin; tallennus tehdään ProjectStore::update-kutsulla.
use crate::{CoreError, native};
use serde::{Deserialize, Deserializer, de::Error};

#[derive(Debug, Default)]
pub struct Changes {
    pub kilpailu: Option<native::Competition>,
    pub virhepisteet: Option<native::Overrides>,
    pub kilpailijat: Vec<native::Driver>,
    pub luokat: Vec<native::Class>,
    pub radat: Vec<native::Track>,
    provided_driver_fields: Vec<ProvidedDriverFields>,
    provided_class_draw: Vec<bool>,
}

/// Vanha muutostiedosto ei tunne myöhemmin lisättyjä kilpailijakenttiä.
/// Läsnäolo pidetään erillään arvosta, jotta tyhjän lisenssin voi myös tallentaa.
#[derive(Debug, Clone, Copy, Default)]
struct ProvidedDriverFields {
    driver_licence: bool,
    navigator_licence: bool,
    driver_club: bool,
    navigator_club: bool,
    status: bool,
    confirmed_interruption: bool,
    penalties: bool,
    stored_result: bool,
}

impl<'de> Deserialize<'de> for Changes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Default, Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Input {
            kilpailu: Option<native::Competition>,
            virhepisteet: Option<native::Overrides>,
            kilpailijat: Vec<toml::Value>,
            luokat: Vec<toml::Value>,
            radat: Vec<native::Track>,
        }
        let input = Input::deserialize(deserializer)?;
        let mut competitors = Vec::new();
        let mut provided = Vec::new();
        for value in input.kilpailijat {
            let fields = value
                .as_table()
                .ok_or_else(|| D::Error::custom("Kilpailijan muutoksen pitää olla tietue"))?;
            provided.push(ProvidedDriverFields {
                driver_licence: fields.contains_key("kuljettajan_lisenssi"),
                navigator_licence: fields.contains_key("kartanlukijan_lisenssi"),
                driver_club: fields.contains_key("kuljettajan_seura"),
                navigator_club: fields.contains_key("kartanlukijan_seura"),
                status: fields.contains_key("tila"),
                confirmed_interruption: fields.contains_key("keskeytys_vahvistettu"),
                penalties: fields.contains_key("lisavirheet"),
                stored_result: fields.contains_key("tallennettu_tulos"),
            });
            let driver = value
                .try_into()
                .map_err(|_| D::Error::custom("Kilpailijan muutoksen kenttä on virheellinen"))?;
            competitors.push(driver);
        }
        let mut classes = Vec::new();
        let mut class_draw = Vec::new();
        for value in input.luokat {
            class_draw.push(
                value
                    .as_table()
                    .ok_or_else(|| D::Error::custom("Luokan muutoksen pitää olla tietue"))?
                    .contains_key("osallistuu_lahtoarvontaan"),
            );
            classes.push(
                value
                    .try_into()
                    .map_err(|_| D::Error::custom("Luokan muutoksen kenttä on virheellinen"))?,
            );
        }
        Ok(Self {
            kilpailu: input.kilpailu,
            virhepisteet: input.virhepisteet,
            kilpailijat: competitors,
            luokat: classes,
            radat: input.radat,
            provided_driver_fields: provided,
            provided_class_draw: class_draw,
        })
    }
}

impl Changes {
    /// Korvaa saman tunnisteen tietueen tai lisää uuden. Radan asemat korvataan yhdessä.
    /// Koko muutos validoidaan ennen kuin kutsuja voi tallentaa sen.
    pub fn apply(self, project: &mut native::Project) -> Result<(), CoreError> {
        let mut candidate = project.clone();
        self.apply_inner(&mut candidate)?;
        crate::corrections::invalidate_input_changes(project, &mut candidate);
        candidate.validate()?;
        *project = candidate;
        Ok(())
    }

    fn apply_inner(self, project: &mut native::Project) -> Result<(), CoreError> {
        if let Some(value) = self.kilpailu {
            project.kilpailu = value;
        }
        if let Some(value) = self.virhepisteet {
            project.virhepisteet = value;
        }
        // Toistuvia tunnisteita ei tulkita hiljaisesti viimeisen arvon hyväksi.
        let mut numbers = std::collections::HashSet::new();
        for (index, mut value) in self.kilpailijat.into_iter().enumerate() {
            if !numbers.insert(value.numero) {
                return Err(CoreError::InvalidInput(
                    "Muutoksessa on toistuva kilpailijan numero".into(),
                ));
            }
            if let Some(old) = project
                .kilpailijat
                .iter_mut()
                .find(|c| c.numero == value.numero)
            {
                if let Some(provided) = self.provided_driver_fields.get(index) {
                    if !provided.driver_licence {
                        value.kuljettajan_lisenssi = old.kuljettajan_lisenssi.clone();
                    }
                    if !provided.navigator_licence {
                        value.kartanlukijan_lisenssi = old.kartanlukijan_lisenssi.clone();
                    }
                    if !provided.driver_club {
                        value.kuljettajan_seura = old.kuljettajan_seura.clone();
                    }
                    if !provided.navigator_club {
                        value.kartanlukijan_seura = old.kartanlukijan_seura.clone();
                    }
                    if !provided.status {
                        value.tila = old.tila.clone();
                    }
                    if !provided.penalties {
                        value.lisavirheet = old.lisavirheet.clone();
                    }
                    if !provided.stored_result {
                        value.tallennettu_tulos = old.tallennettu_tulos.clone();
                    }
                }
                // Muutostiedoston tilamuutos ei ole toimitsijan erillinen
                // vahvistus. Vanha, tilaa muuttamaton tiedosto säilyttää päätöksen.
                let same_status = self
                    .provided_driver_fields
                    .get(index)
                    .map_or(value.tila == old.tila, |provided| !provided.status);
                let explicitly_revoked =
                    self.provided_driver_fields
                        .get(index)
                        .is_some_and(|provided| {
                            provided.confirmed_interruption && !value.keskeytys_vahvistettu
                        });
                value.keskeytys_vahvistettu =
                    old.keskeytys_vahvistettu && same_status && !explicitly_revoked;
                // TOMLissa ei ole null-arvoa: tyhjä tila palauttaa automatiikan.
                if value
                    .tila
                    .as_ref()
                    .is_some_and(|status| status.tunnus.is_empty() && status.syy.is_empty())
                {
                    value.tila = None;
                }
                *old = value;
            } else {
                value.keskeytys_vahvistettu = false;
                project.kilpailijat.push(value);
            }
        }
        let mut codes = std::collections::HashSet::new();
        for (index, mut value) in self.luokat.into_iter().enumerate() {
            if !codes.insert(value.tunnus.trim().to_owned()) {
                return Err(CoreError::InvalidInput(
                    "Muutoksessa on toistuva luokan tunnus".into(),
                ));
            }
            if let Some(old) = project
                .luokat
                .iter_mut()
                .find(|c| c.tunnus.trim() == value.tunnus.trim())
            {
                if self.provided_class_draw.get(index) == Some(&false) {
                    value.osallistuu_lahtoarvontaan = old.osallistuu_lahtoarvontaan;
                }
                *old = value;
            } else {
                project.luokat.push(value);
            }
        }
        numbers.clear();
        for value in self.radat {
            if !numbers.insert(value.numero) {
                return Err(CoreError::InvalidInput(
                    "Muutoksessa on toistuva radan numero".into(),
                ));
            }
            if let Some(old) = project.radat.iter_mut().find(|c| c.numero == value.numero) {
                *old = value;
            } else {
                project.radat.push(value);
            }
        }
        project.validate()
    }
}
