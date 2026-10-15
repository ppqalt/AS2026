//! Pienet kilpailumuutokset yhteiseen malliin; tallennus tehdään ProjectStore::update-kutsulla.
use crate::{CoreError, native};
use serde::Deserialize;

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Changes {
    pub kilpailu: Option<native::Competition>,
    pub virhepisteet: Option<native::Overrides>,
    pub kilpailijat: Vec<native::Driver>,
    pub luokat: Vec<native::Class>,
    pub radat: Vec<native::Track>,
}

impl Changes {
    /// Korvaa saman tunnisteen tietueen tai lisää uuden. Radan asemat korvataan yhdessä.
    /// Koko muutos validoidaan ennen kuin kutsuja voi tallentaa sen.
    pub fn apply(self, project: &mut native::Project) -> Result<(), CoreError> {
        let mut candidate = project.clone();
        self.apply_inner(&mut candidate)?;
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
        for value in self.kilpailijat {
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
                *old = value;
            } else {
                project.kilpailijat.push(value);
            }
        }
        let mut codes = std::collections::HashSet::new();
        for value in self.luokat {
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
