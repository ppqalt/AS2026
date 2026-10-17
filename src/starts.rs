//! Lähtöaikojen arvonta ja lähtöluettelo yhteisestä kilpailumallista.
use crate::{ClockTime, CoreError, native::Project, terminal::parse_clock};

fn invalid(message: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(message.into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StartAssignment {
    pub number: u32,
    pub time: ClockTime,
}

#[derive(Debug, Clone)]
pub struct StartDraw {
    pub assignments: Vec<StartAssignment>,
    source: Project,
    seed: u64,
}

fn eligible(driver: &crate::native::Driver) -> bool {
    driver
        .tila
        .as_ref()
        .is_none_or(|status| status.tunnus == "ILM")
}

/// Esikatselu ei muuta kilpailijoita eikä kilpailunumeroita.
/// Vain ilmoittautuneet, joiden korttia ei ole luettu, osallistuvat arvontaan.
/// Siemen mahdollistaa toistettavan tarkastuksen; arvonta ei ole salausmenetelmä.
pub fn draw(project: &Project, seed: u64) -> Result<StartDraw, CoreError> {
    project.validate()?;
    let mut random = Random(seed);
    let mut assignments = Vec::new();
    for class in &project.luokat {
        if !class.osallistuu_lahtoarvontaan {
            continue;
        }
        let mut drivers = project
            .kilpailijat
            .iter()
            .filter(|driver| driver.luokka.trim() == class.tunnus.trim() && eligible(driver))
            .collect::<Vec<_>>();
        if drivers.is_empty() {
            continue;
        }
        if class.lahtovali_minuutteina == 0 {
            return Err(invalid(format!(
                "Luokan {} lähtövälin pitää olla vähintään yksi minuutti",
                class.tunnus
            )));
        }
        for driver in &drivers {
            if driver.tallennettu_tulos.is_some()
                || project
                    .leimaluvut
                    .iter()
                    .any(|read| read.emit == driver.emit)
            {
                return Err(invalid(format!(
                    "Kilpailijan {} kortti on jo luettu tai asematulos tallennettu; lähtöaikoja ei arvottu",
                    driver.numero
                )));
            }
        }
        let spacing = u64::from(class.lahtovali_minuutteina) * 60;
        let span = (drivers.len() as u64 - 1)
            .checked_mul(spacing)
            .ok_or(CoreError::ArithmeticOverflow("lähtöaikojen arvonta"))?;
        if span >= 86_400 {
            return Err(invalid(format!(
                "Luokan {} lähtöajat eivät mahdu yhden vuorokauden jaksoon",
                class.tunnus
            )));
        }
        for i in (1..drivers.len()).rev() {
            drivers.swap(i, random.below((i + 1) as u64) as usize);
        }
        let first = parse_clock(&class.lahtoaika)?.seconds_since_midnight()? as u64;
        for (index, driver) in drivers.iter().enumerate() {
            let seconds = (first + index as u64 * spacing) % 86_400;
            assignments.push(StartAssignment {
                number: driver.numero,
                time: ClockTime::new(
                    (seconds / 3600) as u32,
                    (seconds / 60 % 60) as u32,
                    (seconds % 60) as u32,
                )?,
            });
        }
    }
    if assignments.is_empty() {
        return Err(invalid(
            "Lähtöaikojen arvontaan ei ole ilmoittautuneita kilpailijoita",
        ));
    }
    Ok(StartDraw {
        assignments,
        source: project.clone(),
        seed,
    })
}

impl StartDraw {
    /// Vahvistus hyväksyy vain tämän muuttumattoman esikatselun.
    /// Kaikki muut kilpailijatiedot ja kortinlukuhistoria säilyvät.
    pub fn apply(&self, project: &mut Project) -> Result<(), CoreError> {
        if *project != self.source {
            return Err(invalid(
                "Kilpailun tiedot ovat muuttuneet; tee lähtöaikojen arvonta uudelleen",
            ));
        }
        if self.assignments != draw(project, self.seed)?.assignments {
            return Err(invalid(
                "Lähtöaikojen esikatselua on muutettu; tee arvonta uudelleen",
            ));
        }
        let mut candidate = project.clone();
        for assignment in &self.assignments {
            let driver = candidate
                .kilpailijat
                .iter_mut()
                .find(|driver| driver.numero == assignment.number)
                .ok_or_else(|| invalid("Arvonnan kilpailijaa ei löydy"))?;
            driver.lahtoaika = clock_text(assignment.time);
        }
        candidate.validate()?;
        *project = candidate;
        Ok(())
    }
}

/// SplitMix64 ja tasainen rajattu valinta; ei ulkoista satunnaiskirjastoa.
struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let value = self.next();
            if value >= threshold {
                return value % bound;
            }
        }
    }
}

fn clock_text(time: ClockTime) -> String {
    format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second)
}

fn sorted_drivers(project: &Project) -> Result<Vec<&crate::native::Driver>, CoreError> {
    project.validate()?;
    // Ilman päivämäärää luokkien välisestä vuorokauden vaihteesta ei arvata.
    // Luokan lähtökello toimii sen sisäisen lähtöjärjestyksen alkupisteenä.
    let schedules = project
        .luokat
        .iter()
        .map(|class| {
            Ok((
                class.tunnus.trim(),
                parse_clock(&class.lahtoaika)?.seconds_since_midnight()?,
            ))
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    let mut rows = project
        .kilpailijat
        .iter()
        .map(|driver| {
            let (class_index, (_, first)) = schedules
                .iter()
                .enumerate()
                .find(|(_, (code, _))| *code == driver.luokka.trim())
                .ok_or_else(|| invalid("Kilpailijan luokkaa ei löydy"))?;
            let clock = parse_clock(&driver.lahtoaika)?.seconds_since_midnight()?;
            Ok((class_index, (clock - first).rem_euclid(86_400), driver))
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    rows.sort_by_key(|(class, offset, driver)| (*class, *offset, driver.numero));
    Ok(rows.into_iter().map(|(_, _, driver)| driver).collect())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub fn start_list_html(project: &Project) -> Result<String, CoreError> {
    let rows = sorted_drivers(project)?;
    let mut html = format!(
        "<!doctype html><html lang=\"fi\"><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Lähtöluettelo — {}</title><style>body{{font:16px system-ui,sans-serif;margin:2rem;color:#242424;background:#fff}}table{{border-collapse:collapse;width:100%}}th,td{{text-align:left;padding:.65rem;border-bottom:1px solid #d1d1d1}}th{{background:#f5f5f5}}@media print{{body{{margin:0}}thead{{display:table-header-group}}tr{{break-inside:avoid}}}}</style><h1>{}</h1><h2>Lähtöluettelo</h2><table><thead><tr><th>Lähtöaika</th><th>Nro</th><th>Luokka</th><th>Kuljettaja</th><th>Kartanlukija</th><th>Auto</th><th>Emit</th><th>Tila</th></tr></thead><tbody>",
        escape(&project.kilpailu.nimi),
        escape(&project.kilpailu.nimi)
    );
    for driver in rows {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
            clock_text(parse_clock(&driver.lahtoaika)?),
            driver.numero,
            escape(&driver.luokka),
            escape(&driver.kuljettaja),
            escape(&driver.kartanlukija),
            escape(&driver.auto),
            driver.emit,
            driver.tila.as_ref().map_or("ILM", |status| status.tunnus.as_str()),
        ));
    }
    html.push_str("</tbody></table></html>\n");
    Ok(html)
}

fn csv_cell(value: &str) -> String {
    let prefix = if value.trim_start().starts_with(['=', '+', '-', '@']) {
        "'"
    } else {
        ""
    };
    format!("\"{prefix}{}\"", value.replace('"', "\"\""))
}

pub fn start_list_csv(project: &Project) -> Result<String, CoreError> {
    let mut csv = "\"Lähtöaika\";\"Nro\";\"Luokka\";\"Kuljettaja\";\"Kartanlukija\";\"Auto\";\"Emit\";\"Tila\"\r\n".to_owned();
    for driver in sorted_drivers(project)? {
        let cells = [
            clock_text(parse_clock(&driver.lahtoaika)?),
            driver.numero.to_string(),
            driver.luokka.clone(),
            driver.kuljettaja.clone(),
            driver.kartanlukija.clone(),
            driver.auto.clone(),
            driver.emit.to_string(),
            driver
                .tila
                .as_ref()
                .map_or("ILM", |status| status.tunnus.as_str())
                .to_owned(),
        ];
        csv.push_str(
            &cells
                .iter()
                .map(|cell| csv_cell(cell))
                .collect::<Vec<_>>()
                .join(";"),
        );
        csv.push_str("\r\n");
    }
    Ok(csv)
}
