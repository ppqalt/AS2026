//! Lomakkeiden kentät ja niiden pienet tallennustoiminnot. Valintalistojen
//! nimet ovat käyttäjälle, values-arvot yhteisen kilpailumallin tunnisteita.

use super::EditorField;
use as2026::{
    CoreError,
    native::Project,
    workspace::{
        ClassEdit, ClassField, CompetitionField, DriverEdit, DriverField, OverrideField,
        ResultField, StationEdit, StationField, TrackEdit, TrackField, Workspace,
    },
};
use slint::{ModelRc, SharedString, VecModel};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Driver(u32),
    ResultDriver(u32),
    Class(String),
    Track(u32),
    Station {
        track: u32,
        index: usize,
    },
    ResultStation {
        number: u32,
        row: u32,
    },
    Settings,
    NewDriver,
    NewClass,
    NewTrack,
    NewStation {
        track: u32,
    },
}

fn strings(values: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    Rc::new(VecModel::from(
        values
            .into_iter()
            .map(SharedString::from)
            .collect::<Vec<_>>(),
    ))
    .into()
}

fn text(key: i32, label: &str, value: impl ToString, hint: &str) -> EditorField {
    let mut field = EditorField {
        key,
        label: label.into(),
        value: value.to_string().into(),
        hint: hint.into(),
        error: "".into(),
        options: ModelRc::default(),
        values: ModelRc::default(),
        choice: -1,
        duration: false,
        hours: "".into(),
        minutes: "".into(),
        seconds: "".into(),
    };
    let unit = match label {
        "Lähtöväli (min)"
        | "Myöhästymisraja (min)"
        | "Jakson pituus (min)"
        | "Jakson raja (min)" => 60,
        "Vapaa-aika (s)" => 1,
        _ => return field,
    };
    field.duration = true;
    field.label = format!(
        "{} (h : min : s)",
        label.rsplit_once(" (").map_or(label, |(name, _)| name)
    )
    .into();
    if let Ok(value) = field.value.parse::<i64>() {
        let total = value.saturating_mul(unit);
        let magnitude = total.unsigned_abs();
        field.hours = format!(
            "{}{:02}",
            if total < 0 { "-" } else { "" },
            magnitude / 3600
        )
        .into();
        field.minutes = format!("{:02}", magnitude % 3600 / 60).into();
        field.seconds = format!("{:02}", magnitude % 60).into();
    }
    field
}

fn choice(
    key: i32,
    label: &str,
    value: impl ToString,
    hint: &str,
    options: Vec<(String, String)>,
) -> EditorField {
    let value = value.to_string();
    let selected = options
        .iter()
        .position(|(_, id)| id == &value)
        .map_or(-1, |i| i as i32);
    EditorField {
        key,
        label: label.into(),
        value: value.into(),
        hint: hint.into(),
        error: "".into(),
        options: strings(options.iter().map(|(name, _)| name.clone())),
        values: strings(options.into_iter().map(|(_, id)| id)),
        choice: selected,
        duration: false,
        hours: "".into(),
        minutes: "".into(),
        seconds: "".into(),
    }
}

fn named(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(name, id)| ((*name).into(), (*id).into()))
        .collect()
}

fn classes(project: &Project) -> Vec<(String, String)> {
    project
        .luokat
        .iter()
        .map(|class| {
            (
                format!("{} — {}", class.tunnus, class.nimi),
                class.tunnus.clone(),
            )
        })
        .collect()
}

fn tracks(project: &Project) -> Vec<(String, String)> {
    std::iter::once(("-".to_owned(), "0".to_owned()))
        .chain(project.radat.iter().map(|track| {
            (
                format!("{} — {}", track.numero, track.nimi),
                track.numero.to_string(),
            )
        }))
        .collect()
}

fn driver_fields(project: &Project, driver: &DriverEdit) -> Vec<EditorField> {
    vec![
        text(
            0,
            "Kilpailunumero",
            &driver.number,
            "Yksilöllinen positiivinen numero",
        ),
        text(1, "Kuljettaja", &driver.driver, "Kuljettajan nimi"),
        text(2, "Kartanlukija", &driver.navigator, "Kartanlukijan nimi"),
        choice(
            3,
            "Luokka",
            &driver.class_code,
            "Luokan rata otetaan käyttöön automaattisesti",
            classes(project),
        ),
        text(
            4,
            "Emit-kortti",
            &driver.card_number,
            "Kortin numero kohdistaa lukemat kilpailijaan",
        ),
        text(5, "Auto", &driver.car, "Auton tiedot, valinnainen"),
        clock_field(6, "Lähtöaika (h : min : s)", &driver.start_time),
        text(
            7,
            "Emit-korjaus (s)",
            &driver.correction_seconds,
            "Kortin nollauskorjaus sekunteina; myös negatiivinen arvo sallitaan",
        ),
    ]
}

fn class_fields(project: &Project, class: &ClassEdit) -> Vec<EditorField> {
    vec![
        text(0, "Luokan tunnus", &class.code, "Esimerkiksi H tai A"),
        text(1, "Luokan nimi", &class.name, "Luokan kuvaava nimi"),
        choice(
            2,
            "Rata",
            &class.route,
            "Luokan kaikki kilpailijat käyttävät valittua rataa",
            tracks(project),
        ),
        text(
            3,
            "Ensimmäinen kilpailunumero",
            &class.first_number,
            "Luokan numeroinnin aloitus",
        ),
        clock_field(4, "Luokan lähtöaika (h : min : s)", &class.start_time),
        text(
            5,
            "Lähtöväli (min)",
            &class.interval_minutes,
            "Lähtöjen väli minuutteina",
        ),
        choice(
            7,
            "Nousupisteet (Np)",
            &class.np_enabled,
            "Luokan nousupisteiden julkaisu",
            named(&[("Ei", "0"), ("Kyllä", "1")]),
        ),
        choice(
            8,
            "Sarjapisteet (Sp)",
            &class.series_method,
            "Sarjapisteiden laskentatapa; tavan 4 pistetaulukko asetetaan kilpailun asetuksissa",
            named(&[
                ("Ei", "0"),
                ("100 pisteen taulukko", "1"),
                ("60 pisteen taulukko", "2"),
                ("16 pisteen taulukko", "3"),
                ("Oma pistetaulukko", "4"),
                ("100 pisteen perintä", "5"),
                ("60 pisteen perintä", "6"),
            ]),
        ),
    ]
}

/// Mahdollistaa pelkän sääntövalinnan päivityksen tyypin muuttuessa ilman
/// muiden lomakekenttien tai niiden keskeneräisten tekstien korvaamista.
pub fn station_rule_field(kind: &str, rule: &str) -> EditorField {
    let mut options = named(&[
        ("Normaali", "0"),
        ("Ei ennakkopisteitä", "1"),
        ("Ei aikavirhepisteitä", "2"),
        ("Ei myöhästymispisteitä", "3"),
        ("Mitätöity asema", "4"),
    ]);
    if kind.trim() == "ULA" {
        options.extend(named(&[
            ("ULA: lähin minuutti", "5"),
            ("ULA: alaspäin minuuttiin", "6"),
            ("ULA: ylöspäin minuuttiin", "7"),
        ]));
    }
    choice(
        4,
        "Laskentasääntö",
        rule,
        "ULA-pyöristyksiä käytetään vain ULA-asemilla",
        options,
    )
}

// Aseman tavoite näytetään kestona; alkuperäinen malli tallentaa sekunnit.
// Target durations are allowed to exceed 24 hours.
fn format_target_duration(raw: &str) -> String {
    match raw.parse::<i64>() {
        Ok(seconds) if seconds >= 0 => format!(
            "{:02}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60,
        ),
        _ => raw.to_owned(),
    }
}

fn parse_target_duration(raw: &str) -> Result<String, CoreError> {
    let invalid_time =
        || CoreError::InvalidInput("Anna tavoiteaika muodossa hh:mm:ss (esim. 00:05:00)".into());
    let parts = raw.trim().split(':').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|byte| byte.is_ascii_digit()))
    {
        return Err(invalid_time());
    }
    let hours = parts[0].parse::<i64>().map_err(|_| invalid_time())?;
    let minutes = parts[1].parse::<i64>().map_err(|_| invalid_time())?;
    let seconds = parts[2].parse::<i64>().map_err(|_| invalid_time())?;
    if minutes >= 60 || seconds >= 60 {
        return Err(invalid_time());
    }
    let total = hours
        .checked_mul(3600)
        .and_then(|hours| {
            minutes
                .checked_mul(60)
                .and_then(|minutes| hours.checked_add(minutes))
        })
        .and_then(|value| value.checked_add(seconds))
        .ok_or_else(invalid_time)?;
    Ok(total.to_string())
}

fn station_target_field(seconds: &str) -> EditorField {
    let formatted = format_target_duration(seconds);
    clock_field(3, "Tavoiteaika (h : min : s)", &formatted)
}

fn clock_field(key: i32, label: &str, formatted: &str) -> EditorField {
    let parts = formatted.split(':').collect::<Vec<_>>();
    let mut field = text(
        key,
        label,
        formatted,
        "Tunnit, minuutit ja sekunnit erikseen",
    );
    field.duration = true;
    if parts.len() == 3 {
        field.hours = parts[0].into();
        field.minutes = parts[1].into();
        field.seconds = parts[2].into();
    }
    field
}

pub fn ready_to_add(
    project: &Project,
    selection: &Selection,
    draft: &BTreeMap<i32, String>,
) -> bool {
    let defaults = fields(project, selection);
    let required: &[i32] = match selection {
        Selection::NewStation { .. } => &[0, 1, 2, 3],
        Selection::NewDriver => &[0, 1, 3],
        Selection::NewClass => &[0, 1, 2, 3, 4, 5],
        Selection::NewTrack => &[0, 1],
        _ => return false,
    };
    required.iter().all(|key| {
        draft
            .get(key)
            .map(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                defaults
                    .iter()
                    .any(|field| field.key == *key && !field.value.trim().is_empty())
            })
    })
}

fn station_fields(station: &StationEdit) -> Vec<EditorField> {
    vec![
        choice(
            0,
            "Aseman tyyppi",
            &station.kind,
            "Valitse aseman tehtävä radalla",
            named(&[
                ("Lähtö", "L"),
                ("ULA", "ULA"),
                ("JAT", "JAT"),
                ("AT", "AT"),
                ("Maali", "M"),
                ("RT", "RT"),
            ]),
        ),
        text(1, "Aseman nimi", &station.name, "Esimerkiksi AT 47"),
        text(
            2,
            "Emit-koodi",
            &station.code,
            "Emit-koodi tai 0 käsin kellotettavalle asemalle",
        ),
        station_target_field(&station.target_seconds),
        station_rule_field(&station.kind, &station.rule),
        text(
            5,
            "Jakson pituus (min)",
            &station.section_minutes,
            "Valinnainen jaksoaika minuutteina",
        ),
    ]
}

pub(super) fn emit_correction_fields(
    project: &Project,
    number: u32,
    row: u32,
) -> Result<Vec<EditorField>, CoreError> {
    let rows = as2026::emit_corrections::rows(project, number)?;
    let selected = rows.iter().find(|r| r.index == row).ok_or_else(invalid)?;
    let code = if selected.code == 0 {
        selected.expected_code
    } else {
        selected.code
    };
    let elapsed = selected.elapsed_seconds;
    let sources = std::iter::once(("Valittu leima / uusi puuttuva leima".into(), "0".into()))
        .chain(
            rows.iter()
                .filter(|r| r.sequence == 0 && r.index != row)
                .map(|r| {
                    (
                        format!(
                            "Rivi {} · koodi {} · {:02}:{:02}:{:02}",
                            r.index,
                            r.code,
                            r.elapsed_seconds / 3600,
                            r.elapsed_seconds % 3600 / 60,
                            r.elapsed_seconds % 60
                        ),
                        r.index.to_string(),
                    )
                }),
        )
        .collect();
    Ok(vec![
        choice(
            0,
            "Korjauksen tapa",
            "0",
            "",
            named(&[
                ("Muokkaa / lisää leima", "0"),
                ("Poista leima laskennasta", "1"),
                ("Palauta alkuperäiset leimat", "2"),
            ]),
        ),
        choice(
            1,
            "Kohdistettava leima",
            "0",
            "Puuttuvalle asemalle voi kohdistaa ylimääräisen leiman",
            sources,
        ),
        text(2, "Leimakoodi", code, "1–255, ei lukijakoodia 250"),
        text(3, "Aika (s)", elapsed, "Korttiaika nollauksesta sekunteina"),
        clock_field(
            5,
            "Aika (hh:mm:ss)",
            &as2026::emit_corrections::clock_text(as2026::emit_corrections::punch_clock(
                project,
                number,
                i64::from(elapsed),
            )?),
        ),
        text(
            4,
            "Korjauksen perustelu",
            "",
            "Esimerkiksi väärä asemakoodi tai varakellon mukainen puuttuva leima",
        ),
    ])
}

pub fn fields(project: &Project, selection: &Selection) -> Vec<EditorField> {
    match selection {
        Selection::ResultDriver(number) => fields(project, &Selection::Driver(*number))
            .into_iter().filter(|field| matches!(field.key, 8 | 9 | 10 | 13 | 14)).collect(),
        Selection::Driver(number) => project
            .kilpailijat
            .iter()
            .find(|driver| driver.numero == *number)
            .map(|driver| {
                let mut fields = driver_fields(
                    project,
                    &DriverEdit {
                        number: driver.numero.to_string(),
                        class_code: driver.luokka.clone(),
                        driver: driver.kuljettaja.clone(),
                        navigator: driver.kartanlukija.clone(),
                        card_number: driver.emit_number().map(|n| n.to_string()).unwrap_or_default(),
                        car: driver.auto.clone(),
                        start_time: driver.lahtoaika.clone(),
                        correction_seconds: driver.effective_emit_correction().to_string(),
                    },
                );
                fields.extend([
                    text(
                        8,
                        "Puuttuvat RT-asemat (kpl)",
                        driver.lisavirheet.puuttuvat_rt_kpl,
                        &format!(
                            "Erillinen pistesumma {} p. Määrän muutos laskee summan nykyisillä asetuksilla.",
                            driver.lisavirheet.puuttuvat_rt_pisteet
                        ),
                    ),
                    text(
                        9,
                        "Ylimääräiset RT-asemat (kpl)",
                        driver.lisavirheet.ylimaaraiset_rt_kpl,
                        &format!(
                            "Erillinen pistesumma {} p. Määrän muutos laskee summan nykyisillä asetuksilla.",
                            driver.lisavirheet.ylimaaraiset_rt_pisteet
                        ),
                    ),
                    text(
                        10,
                        "Muut virhepisteet",
                        driver.lisavirheet.muut_virhepisteet,
                        "Erilliset kilpailijakohtaiset virhepisteet",
                    ),
                ]);
                fields.extend([
                    text(11, "Kuljettajan lisenssi", &driver.kuljettajan_lisenssi, "Lisenssitieto sarjapisteiden tarkastelua varten"),
                    text(12, "Kartanlukijan lisenssi", &driver.kartanlukijan_lisenssi, "Vanhan laskennan X-merkintä vaikuttaa sarjapistekelpoisuuteen"),
                    choice(13, "Tila", driver.tila.as_ref().map_or("", |s| s.tunnus.as_str()), "Automaattinen tila määräytyy laskennasta; käsivalinta säilyy kortin uudelleenluvussa", named(&[
                        ("Automaattinen", ""), ("Tarkastettu", "TRK"), ("Keskeyttänyt", "KESK"), ("Peruuttanut", "PER"), ("Hylätty", "HYL"), ("Suljettu", "SULJ"), ("Ilmoittautunut", "ILM"), ("Ei lähtenyt", "EILÄ")
                    ])),
                    text(14, "Tilan syy", driver.tila.as_ref().map_or("", |s| s.syy.as_str()), if driver.tila.as_ref().is_some_and(|s| s.tunnus == "KESK") { "Keskeytyksen syy. Tulosta keskeytystieto Tulokset-paneelista." } else { "Käsin valitun tilan perustelu näkyy tulosraportissa" }),
                    text(15, "Kuljettajan seura", &driver.kuljettajan_seura, "Kuljettajan seuratieto"),
                    text(16, "Kartanlukijan seura", &driver.kartanlukijan_seura, "Kartanlukijan seuratieto"),
                ]);
                if let Some(config) = &project.beginners_cup
                    && as2026::beginners::eligible(project, *number, config.lahderata) {
                    fields.push(choice(17, "Osallistuu Beginners Cupiin",
                        if project.bc_osallistujat.iter().any(|e| e.numero == *number && e.osallistuu) { "1" } else { "0" },
                        "BC käyttää samaa Emit-korttia ja omia virhepisteitään",
                        named(&[("Ei", "0"), ("Kyllä", "1")])));
                }
                fields
            })
            .unwrap_or_default(),
        Selection::NewDriver => {
            let class = project.luokat.first();
            driver_fields(
                project,
                &DriverEdit {
                    number: project
                        .kilpailijat
                        .iter()
                        .map(|driver| driver.numero)
                        .max()
                        .unwrap_or(0)
                        .saturating_add(1)
                        .to_string(),
                    class_code: class.map_or_else(String::new, |class| class.tunnus.clone()),
                    start_time: class
                        .map_or_else(|| "15:00:00".into(), |class| class.lahtoaika.clone()),
                    correction_seconds: project.virhepisteet.oletus_emit_korjaus_sekuntia.unwrap_or(120).to_string(),
                    ..Default::default()
                },
            )
        }
        Selection::Class(code) => project
            .luokat
            .iter()
            .find(|class| class.tunnus == *code)
            .map(|class| {
                let mut result = class_fields(
                    project,
                    &ClassEdit {
                        code: class.tunnus.clone(),
                        name: class.nimi.clone(),
                        route: class.rata.to_string(),
                        first_number: class.ensimmainen_numero.to_string(),
                        start_time: class.lahtoaika.clone(),
                        interval_minutes: class.lahtovali_minuutteina.to_string(),
                        lateness_minutes: class.myohastymisraja_minuutteina.to_string(),
                        np_enabled: class.nousupisteet.to_string(),
                        series_method: class.sarjapistetapa.to_string(),
                    },
                );
                result.push(choice(9, "Osallistuu lähtöarvontaan", if class.osallistuu_lahtoarvontaan { "1" } else { "0" }, "Vain valittujen luokkien lähtöajat arvotaan", named(&[("Ei", "0"), ("Kyllä", "1")])));
                result
            })
            .unwrap_or_default(),
        Selection::NewClass => class_fields(
            project,
            &ClassEdit {
                route: project
                    .radat
                    .first()
                    .map_or_else(String::new, |track| track.numero.to_string()),
                first_number: project
                    .kilpailijat
                    .iter()
                    .map(|driver| driver.numero)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1)
                    .to_string(),
                start_time: "15:00:00".into(),
                interval_minutes: "2".into(),
                lateness_minutes: "60".into(),
                np_enabled: "0".into(),
                series_method: "0".into(),
                ..Default::default()
            },
        ),
        Selection::Track(number) => project
            .radat
            .iter()
            .find(|track| track.numero == *number)
            .map(|track| {
                vec![
                    text(0, "Radan numero", track.numero, "Radan yksilöllinen numero"),
                    text(1, "Radan nimi", &track.nimi, "Radan kuvaava nimi"),
                ]
            })
            .unwrap_or_default(),
        Selection::NewTrack => vec![
            text(
                0,
                "Radan numero",
                project
                    .radat
                    .iter()
                    .map(|track| track.numero)
                    .max()
                    .unwrap_or(0)
                    .saturating_add(1),
                "Radan yksilöllinen numero",
            ),
            text(
                1,
                "Radan nimi",
                "",
                "Uudelle radalle lisätään lähtö ja maali",
            ),
        ],
        Selection::Station { track, index } => project
            .radat
            .iter()
            .find(|route| route.numero == *track)
            .and_then(|track| track.asemat.get(*index))
            .map(|station| {
                station_fields(&StationEdit {
                    kind: station.tyyppi.clone(),
                    name: station.nimi.clone(),
                    code: station.emit_koodi.to_string(),
                    target_seconds: station.tavoiteaika_sekuntia.to_string(),
                    rule: station.saanto.to_string(),
                    section_minutes: station
                        .jakso_minuutteina
                        .map_or_else(String::new, |minutes| minutes.to_string()),
                })
            })
            .unwrap_or_default(),
        Selection::NewStation { .. } => station_fields(&StationEdit {
            kind: "AT".into(),
            target_seconds: "0".into(),
            rule: "0".into(),
            ..Default::default()
        }),
        Selection::Settings => {
            let mut fields = vec![
                text(
                    0,
                    "Kilpailun nimi",
                    &project.kilpailu.nimi,
                    "Nimi näkyy tulosluetteloissa",
                ),
                text(4, "Maksimi kokonaismyöhästymisraja (min)",
                    project.virhepisteet.kokonaismyohastymisraja_minuutteina.map_or_else(String::new, |v| v.to_string()),
                    "Tyhjä kenttä käyttää oletusta 60 min; raja koskee kaikkia luokkia"),
                text(5, "Oletus Emit-korjaus (s)",
                    project.virhepisteet.oletus_emit_korjaus_sekuntia.map_or_else(String::new, |v| v.to_string()),
                    "Tyhjä = 120 s. Uusien kilpailijoiden nollaus on tämän verran ennen lähtöä."),
                choice(
                    2,
                    "Ajanottotapa",
                    project.kilpailu.ajanottotapa,
                    "Tavat 0, 1 ja 2 ovat käytettävissä; käsikellot korjataan Tulokset-näkymässä",
                    named(&[
                        ("Kellonaikapohjainen", "0"),
                        ("Emit", "1"),
                        ("Emit / tapa 2", "2"),
                    ]),
                ),
                text(
                    3,
                    "Oma sarjapistetaulukko (Sp 4)",
                    project.sarjapisteet.iter().map(i64::to_string).collect::<Vec<_>>().join(", "),
                    "Pisteet sijoille 1, 2, 3 jne. pilkuilla erotettuina; enintään 1000 ei-negatiivista lukua",
                ),
            ];
            let settings = &project.virhepisteet;
            let defaults = as2026::scoring::ScoringParameters::default();
            let default_values = [
                defaults.tolerance_seconds,
                defaults.early_points_per_second,
                defaults.late_points_per_second,
                defaults.missing_at,
                defaults.extra_at,
                defaults.missing_jat,
                defaults.section_limit_minutes,
                defaults.at_cap,
                defaults.jat_cap,
                defaults.missing_rt,
                defaults.extra_rt,
            ];
            for (index, (label, value)) in [
                ("Aikavirheen toleranssi (s)", settings.vapaa_aika_sekunteina),
                ("Ennakon pisteet / s", settings.ennakon_pisteet_sekunnilta),
                (
                    "Myöhästymisen pisteet / s",
                    settings.myohastymisen_pisteet_sekunnilta,
                ),
                ("Puuttuvan AT:n pisteet", settings.puuttuva_at),
                ("Ylimääräisen AT:n pisteet", settings.ylimaarainen_at),
                ("Puuttuvan JAT:n pisteet", settings.puuttuva_jat),
                ("Maksimi jaksomyöhästymisraja (min)", settings.jakson_raja_minuutteina),
                ("AT:n enimmäispisteet", settings.at_enimmaispisteet),
                ("JAT:n enimmäispisteet", settings.jat_enimmaispisteet),
                ("Puuttuvan RT:n pisteet", settings.puuttuva_rt),
                ("Ylimääräisen RT:n pisteet", settings.ylimaarainen_rt),
            ]
            .into_iter()
            .enumerate()
            {
                if matches!(index, 5 | 8) { continue; }
                fields.push(text(
                    10 + index as i32,
                    label,
                    value.map_or_else(String::new, |value| value.to_string()),
                    &format!("Tyhjä kenttä käyttää oletusta {}", default_values[index]),
                ));
            }
            for (index, group) in project.yhdistetyt_luokat.iter().enumerate() {
                fields.push(text(
                    30 + index as i32,
                    &format!("Yhdistetty luokka {}", index + 1),
                    format!(
                        "{} | {} | {} | {}",
                        group.nimi,
                        group.luokat.join(","),
                        group.nousupisteet,
                        group.sarjapistetapa
                    ),
                    "Nimi | A,B | Np (0/1) | Sp (0–6); tyhjä arvo poistaa ryhmän",
                ));
            }
            if project.yhdistetyt_luokat.len() < 4 {
                fields.push(text(
                    30 + project.yhdistetyt_luokat.len() as i32,
                    "Lisää yhdistetty luokka",
                    "",
                    "Anna koko määritelmä: Nimi | A,B | 1 | 5",
                ));
            }
            fields
        }
        Selection::ResultStation { number, row } => as2026::application::calculated_project(project).ok()
            .and_then(|archive| archive.competitors.into_iter().find(|d| d.number == *number))
            .and_then(|driver| driver.results.into_iter().find(|r| r.output_index == *row))
            .map_or_else(Vec::new, |station| {
                let mut fields = Vec::new();
                if station.expected_code == 0 && station.station_type != "L" {
                    let pending = project.kilpailijat.iter().find(|d| d.numero == *number)
                        .and_then(|d| d.tallennettu_tulos.as_ref()).is_none_or(|r| r.pending_manual_clocks().contains(row));
                    let value = if pending { String::new() } else { format!("{:02}:{:02}:{:02}", station.clock[0], station.clock[1], station.clock[2]) };
                    fields.push(clock_field(100, "Käsikello (h : min : s)", &value));
                }
                fields.extend([
                text(101, "Aikavirhe (p)", station.signed_time_penalty, "Ennakko positiivisena, myöhä negatiivisena; summaan tulee itseisarvo"),
                text(102, "Muu virhe (p)", station.other_penalty, "Ei-negatiivinen asemarivin virhe. Tutki korvaa pistekorjaukset laskennalla."),
                ]);
                fields
            }),
        Selection::None => Vec::new(),
    }
}

fn invalid() -> CoreError {
    CoreError::InvalidInput("Valittua kenttää tai tietuetta ei löydy. Päivitä näkymä".into())
}

fn identifier(value: &str) -> Result<u32, CoreError> {
    value
        .trim()
        .parse()
        .map_err(|_| CoreError::InvalidInput("Anna kelvollinen positiivinen numero".into()))
}

// Säilytä mallin alkuperäiset sekunti- ja kokonaisminuuttiyksiköt.
fn stored_duration(selection: &Selection, key: i32, value: &str) -> Result<String, CoreError> {
    let unit = match (selection, key) {
        (Selection::Driver(_) | Selection::NewDriver, 7) => 1,
        (Selection::Class(_) | Selection::NewClass, 5 | 6) => 60,
        (Selection::Station { .. } | Selection::NewStation { .. }, 5) => 60,
        (Selection::Settings, 10) => 1,
        (Selection::Settings, 16) => 60,
        _ => return Ok(value.into()),
    };
    if !value.contains(':') {
        return Ok(value.into());
    }
    if value == "::" {
        return Ok(String::new());
    }
    let negative = value.starts_with('-');
    let seconds: i64 = parse_target_duration(value.trim_start_matches('-'))?
        .parse()
        .map_err(|_| invalid())?;
    if seconds % unit != 0 {
        return Err(CoreError::InvalidInput(
            "Anna kokonaiset minuutit; sekuntien on oltava 00".into(),
        ));
    }
    Ok(((if negative { -seconds } else { seconds }) / unit).to_string())
}

pub fn apply_field(
    ws: &mut Workspace,
    selection: &Selection,
    key: i32,
    value: &str,
) -> Result<Selection, CoreError> {
    let stored = stored_duration(selection, key, value)?;
    let value = stored.as_str();
    match selection {
        Selection::ResultDriver(number) => {
            let field = match key {
                8 => DriverField::MissingRtCount,
                9 => DriverField::ExtraRtCount,
                10 => DriverField::OtherPenalties,
                13 => DriverField::Status,
                14 => DriverField::StatusReason,
                _ => return Err(invalid()),
            };
            ws.edit_result_driver(*number, field, value)?;
            Ok(selection.clone())
        }
        Selection::Driver(number) => {
            if key == 17 {
                let project = ws.project().ok_or_else(invalid)?;
                let config = project.beginners_cup.clone().ok_or_else(invalid)?;
                let mut selected = project
                    .bc_osallistujat
                    .iter()
                    .filter(|e| e.osallistuu && e.numero != *number)
                    .map(|e| e.numero)
                    .collect::<Vec<_>>();
                match value {
                    "1" => selected.push(*number),
                    "0" => {}
                    _ => return Err(invalid()),
                }
                ws.save_beginners_cup(*config, &selected)?;
                return Ok(selection.clone());
            }
            let field = match key {
                0 => DriverField::Number,
                1 => DriverField::Driver,
                2 => DriverField::Navigator,
                3 => DriverField::Class,
                4 => DriverField::Card,
                5 => DriverField::Car,
                6 => DriverField::Start,
                7 => DriverField::Correction,
                8 => DriverField::MissingRtCount,
                9 => DriverField::ExtraRtCount,
                10 => DriverField::OtherPenalties,
                11 => DriverField::DriverLicence,
                12 => DriverField::NavigatorLicence,
                13 => DriverField::Status,
                14 => DriverField::StatusReason,
                15 => DriverField::DriverClub,
                16 => DriverField::NavigatorClub,
                _ => return Err(invalid()),
            };
            let new_number = if key == 0 {
                identifier(value)?
            } else {
                *number
            };
            ws.edit_driver(*number, field, value)?;
            Ok(Selection::Driver(new_number))
        }
        Selection::Class(code) => {
            let field = match key {
                0 => ClassField::Code,
                1 => ClassField::Name,
                2 => ClassField::Route,
                3 => ClassField::FirstNumber,
                4 => ClassField::Start,
                5 => ClassField::Interval,
                6 => ClassField::Lateness,
                7 => ClassField::Np,
                8 => ClassField::Sp,
                9 => ClassField::Draw,
                _ => return Err(invalid()),
            };
            ws.edit_class(code, field, value)?;
            Ok(Selection::Class(if key == 0 {
                value.trim().into()
            } else {
                code.clone()
            }))
        }
        Selection::Track(number) => {
            let field = match key {
                0 => TrackField::Number,
                1 => TrackField::Name,
                _ => return Err(invalid()),
            };
            let new_number = if key == 0 {
                identifier(value)?
            } else {
                *number
            };
            ws.edit_track(*number, field, value)?;
            Ok(Selection::Track(new_number))
        }
        Selection::Station { track, index } => {
            let field = match key {
                0 => StationField::Kind,
                1 => StationField::Name,
                2 => StationField::Code,
                3 => StationField::Target,
                4 => StationField::Rule,
                5 => StationField::Section,
                _ => return Err(invalid()),
            };
            let stored = if key == 3 {
                parse_target_duration(value)?
            } else {
                value.to_owned()
            };
            ws.edit_station(*track, *index, field, &stored)?;
            Ok(selection.clone())
        }
        Selection::ResultStation { number, row } => {
            let field = match key {
                100 => ResultField::Clock,
                101 => ResultField::TimePenalty,
                102 => ResultField::OtherPenalty,
                _ => return Err(invalid()),
            };
            ws.edit_station_result(*number, *row, field, value)?;
            Ok(selection.clone())
        }
        Selection::Settings => {
            match key {
                0 => ws.edit_competition(CompetitionField::Name, value)?,
                4 => ws.edit_competition(CompetitionField::TotalLateness, value)?,
                5 => ws.edit_competition(CompetitionField::DefaultEmitCorrection, value)?,
                6 => ws.edit_competition(CompetitionField::StartColumns, value)?,
                2 => ws.edit_competition(CompetitionField::Timing, value)?,
                3 => ws.edit_competition(CompetitionField::CustomSeries, value)?,
                30..=33 => ws.edit_combined_class((key - 30) as usize, value)?,
                10..=20 => {
                    let field = [
                        OverrideField::FreeTime,
                        OverrideField::EarlyRate,
                        OverrideField::LateRate,
                        OverrideField::MissingAt,
                        OverrideField::ExtraAt,
                        OverrideField::MissingJat,
                        OverrideField::SectionLimit,
                        OverrideField::AtMaximum,
                        OverrideField::JatMaximum,
                        OverrideField::MissingRt,
                        OverrideField::ExtraRt,
                    ][(key - 10) as usize];
                    ws.edit_override(field, value)?;
                }
                _ => return Err(invalid()),
            }
            Ok(selection.clone())
        }
        _ => Err(invalid()),
    }
}

pub fn add(
    ws: &mut Workspace,
    selection: &Selection,
    values: &BTreeMap<i32, String>,
) -> Result<Selection, CoreError> {
    let project = ws.project().ok_or_else(invalid)?;
    let mut row: BTreeMap<i32, String> = fields(project, selection)
        .into_iter()
        .map(|field| (field.key, field.value.to_string()))
        .collect();
    row.extend(values.iter().map(|(key, value)| (*key, value.clone())));
    for (key, value) in &mut row {
        *value = stored_duration(selection, *key, value)?;
    }
    let value = |key| row.get(&key).cloned().unwrap_or_default();
    match selection {
        Selection::NewDriver => {
            let number = identifier(&value(0))?;
            ws.add_driver(DriverEdit {
                number: value(0),
                driver: value(1),
                navigator: value(2),
                class_code: value(3),
                card_number: value(4),
                car: value(5),
                start_time: value(6),
                correction_seconds: value(7),
            })?;
            Ok(Selection::Driver(number))
        }
        Selection::NewClass => {
            let code = value(0).trim().to_owned();
            ws.add_class(ClassEdit {
                code: value(0),
                name: value(1),
                route: value(2),
                first_number: value(3),
                start_time: value(4),
                interval_minutes: value(5),
                lateness_minutes: "60".into(),
                np_enabled: value(7),
                series_method: value(8),
            })?;
            Ok(Selection::Class(code))
        }
        Selection::NewTrack => {
            let number = identifier(&value(0))?;
            ws.add_track(TrackEdit {
                number: value(0),
                name: value(1),
            })?;
            Ok(Selection::Track(number))
        }
        Selection::NewStation { track } => {
            let index = ws.add_station(
                *track,
                StationEdit {
                    kind: value(0),
                    name: value(1),
                    code: value(2),
                    target_seconds: parse_target_duration(&value(3))?,
                    rule: value(4),
                    section_minutes: value(5),
                },
            )?;
            Ok(Selection::Station {
                track: *track,
                index,
            })
        }
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod target_duration_tests {
    use super::{Selection, format_target_duration, parse_target_duration, stored_duration, text};

    #[test]
    fn duration_controls_preserve_units_sign_and_optional_values() {
        assert_eq!(
            stored_duration(&Selection::NewClass, 5, "01:02:00").unwrap(),
            "62"
        );
        assert!(stored_duration(&Selection::NewClass, 5, "00:01:01").is_err());
        assert_eq!(
            stored_duration(&Selection::Driver(7), 7, "-00:00:12").unwrap(),
            "-12"
        );
        assert_eq!(stored_duration(&Selection::Settings, 10, "::").unwrap(), "");
        assert_eq!(
            stored_duration(&Selection::Settings, 10, "00:00:00").unwrap(),
            "0"
        );
        let field = text(7, "Emit-korjaus (s)", "-12", "");
        assert!(!field.duration);
        assert_eq!(field.value.as_str(), "-12");
    }

    #[test]
    fn targets_display_as_hms() {
        assert_eq!(format_target_duration("0"), "00:00:00");
        assert_eq!(format_target_duration("30"), "00:00:30");
        assert_eq!(format_target_duration("300"), "00:05:00");
        assert_eq!(format_target_duration("90061"), "25:01:01");
    }

    #[test]
    fn only_valid_hms_durations_become_seconds() {
        assert_eq!(parse_target_duration("00:05:00").unwrap(), "300");
        assert_eq!(parse_target_duration("25:01:01").unwrap(), "90061");
        for invalid in [
            "300",
            "00:60:00",
            "00:00:60",
            "1:2",
            "-1:00:00",
            "999999999999999999999:00:00",
        ] {
            assert!(parse_target_duration(invalid).is_err(), "{invalid}");
        }
    }
}
