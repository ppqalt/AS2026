//! Lomakkeiden kentät ja niiden pienet tallennustoiminnot. Valintalistojen
//! nimet ovat käyttäjälle, values-arvot yhteisen kilpailumallin tunnisteita.

use super::EditorField;
use as2026::{
    CoreError,
    native::Project,
    workspace::{
        ClassEdit, ClassField, CompetitionField, DriverEdit, DriverField, OverrideField,
        StationEdit, StationField, TrackEdit, TrackField, Workspace,
    },
};
use slint::{ModelRc, SharedString, VecModel};
use std::{collections::BTreeMap, rc::Rc};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Selection {
    #[default]
    None,
    Driver(u32),
    Class(String),
    Track(u32),
    Station {
        track: u32,
        index: usize,
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
    EditorField {
        key,
        label: label.into(),
        value: value.to_string().into(),
        hint: hint.into(),
        options: ModelRc::default(),
        values: ModelRc::default(),
        choice: -1,
    }
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
        options: strings(options.iter().map(|(name, _)| name.clone())),
        values: strings(options.into_iter().map(|(_, id)| id)),
        choice: selected,
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
    project
        .radat
        .iter()
        .map(|track| {
            (
                format!("{} — {}", track.numero, track.nimi),
                track.numero.to_string(),
            )
        })
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
        text(
            6,
            "Lähtöaika",
            &driver.start_time,
            "Kellonaika muodossa 15:00:00",
        ),
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
        text(
            4,
            "Luokan lähtöaika",
            &class.start_time,
            "Kellonaika muodossa 15:00:00",
        ),
        text(
            5,
            "Lähtöväli (min)",
            &class.interval_minutes,
            "Lähtöjen väli minuutteina",
        ),
        text(
            6,
            "Myöhästymisraja (min)",
            &class.lateness_minutes,
            "Kokonaismyöhästymisen raja minuutteina",
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
            "Lähtö- ja maaliasemalla voidaan käyttää nollaa",
        ),
        text(
            3,
            "Tavoiteaika (s)",
            &station.target_seconds,
            "Tavoiteaika sekunteina, esimerkiksi 300 = 5 minuuttia",
        ),
        station_rule_field(&station.kind, &station.rule),
        text(
            5,
            "Jakson pituus (min)",
            &station.section_minutes,
            "Valinnainen jaksoaika minuutteina",
        ),
    ]
}

pub fn fields(project: &Project, selection: &Selection) -> Vec<EditorField> {
    match selection {
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
                        card_number: driver.emit.to_string(),
                        car: driver.auto.clone(),
                        start_time: driver.lahtoaika.clone(),
                        correction_seconds: driver.emit_korjaus_sekuntia.to_string(),
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
                    correction_seconds: "0".into(),
                    ..Default::default()
                },
            )
        }
        Selection::Class(code) => project
            .luokat
            .iter()
            .find(|class| class.tunnus == *code)
            .map(|class| {
                class_fields(
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
                )
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
                text(
                    1,
                    "Kilpailun numero",
                    project.kilpailu.numero,
                    "Kilpailun yksilöivä numero",
                ),
                choice(
                    2,
                    "Ajanottotapa",
                    project.kilpailu.ajanottotapa,
                    "Tavat 0, 1 ja 2 ovat käytettävissä; erillinen käsiaikojen syöttö on vielä kesken",
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
                ("Vapaa-aika (s)", settings.vapaa_aika_sekunteina),
                ("Ennakon pisteet / s", settings.ennakon_pisteet_sekunnilta),
                (
                    "Myöhästymisen pisteet / s",
                    settings.myohastymisen_pisteet_sekunnilta,
                ),
                ("Puuttuvan AT:n pisteet", settings.puuttuva_at),
                ("Ylimääräisen AT:n pisteet", settings.ylimaarainen_at),
                ("Puuttuvan JAT:n pisteet", settings.puuttuva_jat),
                ("Jakson raja (min)", settings.jakson_raja_minuutteina),
                ("AT:n enimmäispisteet", settings.at_enimmaispisteet),
                ("JAT:n enimmäispisteet", settings.jat_enimmaispisteet),
                ("Puuttuvan RT:n pisteet", settings.puuttuva_rt),
                ("Ylimääräisen RT:n pisteet", settings.ylimaarainen_rt),
            ]
            .into_iter()
            .enumerate()
            {
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

pub fn apply_field(
    ws: &mut Workspace,
    selection: &Selection,
    key: i32,
    value: &str,
) -> Result<Selection, CoreError> {
    match selection {
        Selection::Driver(number) => {
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
            ws.edit_station(*track, *index, field, value)?;
            Ok(selection.clone())
        }
        Selection::Settings => {
            match key {
                0 => ws.edit_competition(CompetitionField::Name, value)?,
                1 => ws.edit_competition(CompetitionField::Number, value)?,
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
                lateness_minutes: value(6),
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
                    target_seconds: value(3),
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
