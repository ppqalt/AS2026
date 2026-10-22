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

fn eligible(driver: &crate::native::Driver, project: &Project) -> bool {
    driver
        .tila
        .as_ref()
        .is_none_or(|status| status.tunnus == "ILM")
        && driver.tallennettu_tulos.is_none()
        && !project
            .leimaluvut
            .iter()
            .any(|read| read.emit == driver.emit)
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
            .filter(|driver| {
                driver.luokka.trim() == class.tunnus.trim() && eligible(driver, project)
            })
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
        if !project
            .luokat
            .iter()
            .any(|class| class.osallistuu_lahtoarvontaan)
        {
            return Err(invalid("Valitse vähintään yksi luokka lähtöarvontaan"));
        }
        return Err(invalid(
            "Valituissa luokissa ei ole arvottavia lukemattomia kilpailijoita. Jo luetut ja asematuloksen saaneet ohitetaan",
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
    let mut rows = project
        .kilpailijat
        .iter()
        .map(|driver| {
            let clock = driver
                .start_clock()?
                .map(|c| c.seconds_since_midnight())
                .transpose()?
                .unwrap_or(86_400);
            Ok((clock, driver))
        })
        .collect::<Result<Vec<_>, CoreError>>()?;
    rows.sort_by_key(|(clock, driver)| (*clock, driver.numero));
    Ok(rows.into_iter().map(|(_, driver)| driver).collect())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub const START_COLUMNS: [(&str, &str); 9] = [
    ("numero", "Nro"),
    ("luokka", "Luokka"),
    ("emit", "Emit"),
    ("lahtoaika", "Lähtöaika"),
    ("kuljettaja", "Kuljettaja"),
    ("kuljettajan_seura", "Kuljettajan seura"),
    ("kartturi", "Kartturi"),
    ("kartturin_seura", "Kartturin seura"),
    ("auto", "Auto"),
];
pub fn columns(project: &Project) -> Result<Vec<(&'static str, &'static str)>, CoreError> {
    if project.virhepisteet.lahtoluettelon_kentat.is_empty() {
        return Ok(START_COLUMNS.to_vec());
    }
    let mut result = Vec::new();
    for id in &project.virhepisteet.lahtoluettelon_kentat {
        let entry = START_COLUMNS
            .iter()
            .find(|(key, _)| key == id)
            .ok_or_else(|| invalid(format!("Tuntematon lähtöluettelon kenttä: {id}")))?;
        if result.contains(entry) {
            return Err(invalid(format!("Lähtöluettelon kenttä toistuu: {id}")));
        }
        result.push(*entry);
    }
    Ok(result)
}
pub fn column_value(driver: &crate::native::Driver, id: &str) -> String {
    match id {
        "numero" => driver.numero.to_string(),
        "luokka" => driver.luokka.clone(),
        "emit" => driver
            .emit_number()
            .map(|v| v.to_string())
            .unwrap_or_default(),
        "lahtoaika" => driver.lahtoaika.clone(),
        "kuljettaja" => driver.kuljettaja.clone(),
        "kuljettajan_seura" => driver.kuljettajan_seura.clone(),
        "kartturi" => driver.kartanlukija.clone(),
        "kartturin_seura" => driver.kartanlukijan_seura.clone(),
        "auto" => driver.auto.clone(),
        _ => String::new(),
    }
}
pub fn start_list_html(project: &Project) -> Result<String, CoreError> {
    let rows = sorted_drivers(project)?;
    let columns = columns(project)?;
    let mut html = format!(
        "<!doctype html><html lang=\"fi\"><meta charset=\"utf-8\"><title>Lähtöluettelo — {}</title><style>body{{font:16px system-ui;margin:2rem}}table{{border-collapse:collapse;width:100%}}th,td{{text-align:left;padding:.65rem;border-bottom:1px solid #ddd}}@media print{{thead{{display:table-header-group}}tr{{break-inside:avoid}}}}</style><h1>{}</h1><h2>Lähtöluettelo</h2><table><thead><tr>",
        escape(&project.kilpailu.nimi),
        escape(&project.kilpailu.nimi)
    );
    for (_, label) in &columns {
        html.push_str(&format!("<th>{}</th>", escape(label)));
    }
    html.push_str("</tr></thead><tbody>");
    for driver in rows {
        html.push_str("<tr>");
        for (id, _) in &columns {
            html.push_str(&format!("<td>{}</td>", escape(&column_value(driver, id))));
        }
        html.push_str("</tr>");
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
    let columns = columns(project)?;
    let mut csv = columns
        .iter()
        .map(|(_, label)| csv_cell(label))
        .collect::<Vec<_>>()
        .join(";");
    csv.push_str("\r\n");
    for driver in sorted_drivers(project)? {
        csv.push_str(
            &columns
                .iter()
                .map(|(id, _)| csv_cell(&column_value(driver, id)))
                .collect::<Vec<_>>()
                .join(";"),
        );
        csv.push_str("\r\n");
    }
    Ok(csv)
}
