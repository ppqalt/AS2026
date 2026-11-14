//! WordPressin AS tulkki -rajapinnan työpöytäasiakas. Ei muuta laskentatuloksia.
use crate::{interpretation::EntryKind, native::Project};
use reqwest::{Url, blocking::Client, redirect::Policy};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    time::Duration,
};

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub site: String,
    pub user: String,
    pub competition_id: u64,
    pub request_id: String,
    pub name: String,
    pub join_code: String,
    pub key: String,
    pub revision: u64,
    pub cursor: u64,
    /// JSON-teksti säilyttää myös manualTime:null-arvot TOML-asetuksissa.
    pub published_manifest: String,
    pub received: Vec<ReceivedSection>,
    pub closed: bool,
    pub pending_join_code: String,
    pub scoring_revision: u64,
    pub route_scoring_revisions: BTreeMap<String, u64>,
    pub automatic_results: bool,
    /// Ennen tulostusjonoon siirtoa tallennettu tuloksen tunniste estää toistot.
    pub automatic_prints: BTreeMap<String, String>,
    pub history: Vec<InterpretationEvent>,
    pub last_received_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SectionInterpretation {
    pub id: String,
    pub missing_entry_ids: Vec<String>,
    #[serde(deserialize_with = "deserialize_times")]
    pub times: BTreeMap<String, String>,
    pub note: String,
    pub other_errors: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub other_penalty_points: Option<u32>,
    pub missing_signs: u32,
    pub missing_times: u32,
    pub interpreted: bool,
    #[serde(default)]
    pub pending_time_ids: Vec<String>,
}

fn deserialize_times<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum WireTimes {
        Object(BTreeMap<String, String>),
        EmptyList(Vec<Value>),
    }
    match WireTimes::deserialize(deserializer)? {
        WireTimes::Object(times) => Ok(times),
        // WordPressin nykyinen muutossyöte muuttaa tyhjän PHP-olion tyhjäksi luetteloksi.
        WireTimes::EmptyList(items) if items.is_empty() => Ok(BTreeMap::new()),
        WireTimes::EmptyList(_) => Err(serde::de::Error::custom(
            "Aikaluettelo ei saa olla ei-tyhjä taulukko",
        )),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterpretationEvent {
    pub event_id: u64,
    pub pair_number: u32,
    pub revision: u64,
    pub reviewer: String,
    pub saved_at: String,
    pub sections: Vec<SectionInterpretation>,
    #[serde(default)]
    pub original_sections: Vec<Value>,
    #[serde(default)]
    pub original_labels: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePage {
    pub events: Vec<InterpretationEvent>,
    pub next_cursor: u64,
    pub has_more: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceivedSection {
    pub event_id: u64,
    pub pair_number: u32,
    pub reviewer: String,
    pub saved_at: String,
    pub section: SectionInterpretation,
}

/// Muodostaa päivitykset nykyisiin laskentakenttiin. Alkuperäisiä leimoja tai käsiaikoja ei muuteta.
pub fn penalty_changes(
    project: &Project,
    config: &Config,
) -> Result<Vec<(u32, crate::native::AdditionalPenalties)>, crate::CoreError> {
    let mut effective = config.clone();
    if !effective.published_manifest.is_empty() {
        let mut value: Value = serde_json::from_str(&effective.published_manifest)
            .map_err(|_| crate::CoreError::InvalidInput("Julkaistu oikea rivi puuttuu".into()))?;
        // Tarkista alkuperäinen vastaanotto ennen mitätöityjen kilpien sovitusta.
        // Normalisointi ei saa peittää virheellisiä määriä välimuistissa.
        for receipt in &config.received {
            validate_event(
                &InterpretationEvent {
                    event_id: receipt.event_id,
                    pair_number: receipt.pair_number,
                    revision: config.revision,
                    reviewer: receipt.reviewer.clone(),
                    saved_at: receipt.saved_at.clone(),
                    sections: vec![receipt.section.clone()],
                    original_sections: Vec::new(),
                    original_labels: BTreeMap::new(),
                },
                &value,
                config.revision,
            )
            .map_err(crate::CoreError::InvalidInput)?;
        }
        for route in &project.oikeat_rivit {
            for entry in route.merkinnat.iter().filter(|e| e.mitatoity) {
                for r in value["routes"].as_array_mut().into_iter().flatten() {
                    for s in r["sections"].as_array_mut().into_iter().flatten() {
                        for e in s["entries"].as_array_mut().into_iter().flatten() {
                            if e["id"].as_str() == Some(&entry.tunniste) {
                                e["voided"] = json!(true);
                            }
                        }
                    }
                }
            }
        }
        effective.published_manifest = value.to_string();
        normalize_received(&mut effective).map_err(crate::CoreError::InvalidInput)?;
    }
    penalty_changes_inner(project, &effective)
}

fn penalty_changes_inner(
    project: &Project,
    config: &Config,
) -> Result<Vec<(u32, crate::native::AdditionalPenalties)>, crate::CoreError> {
    use crate::{CoreError, native::TulkkiPenaltySource};
    use sha2::{Digest, Sha256};
    let invalid = |message: &str| CoreError::InvalidInput(message.into());
    if config.received.is_empty() {
        return Ok(Vec::new());
    }
    let manifest: Value = serde_json::from_str(&config.published_manifest)
        .map_err(|_| invalid("Julkaistu oikea rivi puuttuu"))?;
    let parameters = project
        .to_archive()?
        .scoring_parameters()
        .map_err(|e| invalid(&e.to_string()))?;
    let mut groups: BTreeMap<u32, Vec<&ReceivedSection>> = BTreeMap::new();
    for receipt in &config.received {
        groups.entry(receipt.pair_number).or_default().push(receipt);
    }
    let mut changes = Vec::new();
    for (number, mut receipts) in groups {
        let driver = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == number)
            .ok_or_else(|| invalid("Tulkinnan kilpailunumeroa ei löydy kilpailusta"))?;
        let route = project
            .luokat
            .iter()
            .find(|c| c.tunnus == driver.luokka)
            .map(|c| format!("r{}", c.rata));
        let published = route_for_pair(&manifest, number)
            .ok_or_else(|| invalid("Tulkinnan julkaistu rata puuttuu"))?;
        if published["id"].as_str() != route.as_deref() {
            return Err(invalid(
                "Parin rata on muuttunut julkaisemisen jälkeen; tarkista tulkinta",
            ));
        }
        receipts.sort_by(|a, b| a.section.id.cmp(&b.section.id));
        // Myös aiemmin käsitellyn tapahtuman sisältö tarkistetaan ennen
        // siirtotunnisteen pikaoikotietä; vioittunut välimuisti ei saa ohittaa validointia.
        let mut sections = BTreeSet::new();
        for receipt in &receipts {
            if !sections.insert(&receipt.section.id) {
                return Err(invalid("Tulkinnassa on sama jakso kahdesti"));
            }
            validate_event(
                &InterpretationEvent {
                    event_id: receipt.event_id,
                    pair_number: number,
                    revision: config.revision,
                    reviewer: receipt.reviewer.clone(),
                    saved_at: receipt.saved_at.clone(),
                    sections: vec![receipt.section.clone()],
                    original_sections: Vec::new(),
                    original_labels: BTreeMap::new(),
                },
                &manifest,
                config.revision,
            )
            .map_err(|e| invalid(&e))?;
        }
        let published_sections = list(published, "sections").map_err(|e| invalid(&e))?;
        let completed = published_sections
            .iter()
            .enumerate()
            .filter(|(_, s)| {
                receipts
                    .iter()
                    .any(|r| s["id"].as_str() == Some(&r.section.id) && r.section.interpreted)
            })
            .map(|(i, _)| i as u32 + 1)
            .collect::<Vec<_>>();
        let section_count = published_sections.len() as u32;
        let ids = receipts
            .iter()
            .map(|r| (&r.section.id, r.event_id))
            .collect::<Vec<_>>();
        let scoring_revision = config
            .route_scoring_revisions
            .get(published["id"].as_str().unwrap_or(""))
            .copied()
            .unwrap_or(if config.scoring_revision == 0 {
                config.revision
            } else {
                config.scoring_revision
            });
        let stamp = if project.kilpailu.ajanottotapa == 0 {
            serde_json::to_vec(&(
                "paperikortti-v2",
                &config.site,
                config.competition_id,
                scoring_revision,
                &ids,
            ))
        } else {
            serde_json::to_vec(&(&config.site, config.competition_id, scoring_revision, &ids))
        }
        .map_err(|_| invalid("Siirtotunnistetta ei voitu muodostaa"))?;
        let base_token = format!("{:x}", Sha256::digest(stamp));
        let missing_details =
            missing_rt_sections(config, number, published).map_err(|e| invalid(&e))?;
        let section_other_points = |rate: i64| {
            published_sections
                .iter()
                .map(|s| {
                    let section = receipts
                        .iter()
                        .find(|r| s["id"].as_str() == Some(&r.section.id))
                        .map(|r| &r.section);
                    match section {
                        Some(s) => s.other_penalty_points.map_or_else(
                            || {
                                i64::from(s.other_errors).checked_mul(rate).ok_or(
                                    CoreError::ArithmeticOverflow("Jakson muut virhepisteet"),
                                )
                            },
                            |points| Ok(i64::from(points)),
                        ),
                        None => Ok(0),
                    }
                })
                .collect::<Result<Vec<_>, _>>()
        };
        let has_voids = list(published, "sections").is_ok_and(|ss| {
            ss.iter()
                .any(|s| list(s, "entries").is_ok_and(|es| es.iter().any(|e| e["voided"] == true)))
        });
        let token = if has_voids {
            format!(
                "{:x}",
                Sha256::digest(
                    serde_json::to_vec(&(&base_token, &missing_details))
                        .map_err(|_| invalid("Siirtotunniste puuttuu"))?
                )
            )
        } else {
            base_token.clone()
        };
        if let Some(source) = driver.lisavirheet.tulkki_siirto.as_ref().filter(|s| {
            (s.tapahtumatunniste.as_ref() == Some(&base_token) || s.tunniste == base_token)
                && (!has_voids
                    || (s.puuttuvat_rt == missing_details
                        && (s.tapahtumatunniste.is_some() || !s.puuttuvat_rt.is_empty())))
        }) {
            let other_points = if source.jaksojen_muut_pisteet.len() == section_count as usize {
                source.jaksojen_muut_pisteet.clone()
            } else {
                // Päivitys 0.7.6:sta säilyttää vastaanottohetken hinnan ja käsin korjatut pisteet.
                let missing_at_points = driver
                    .tallennettu_tulos
                    .as_ref()
                    .map_or(0, |r| r.puuttuvien_at_pisteet);
                let count: i64 = receipts
                    .iter()
                    .map(|r| i64::from(r.section.other_errors))
                    .sum();
                let direct: i64 = receipts
                    .iter()
                    .filter_map(|r| r.section.other_penalty_points)
                    .map(i64::from)
                    .sum();
                let total = source.muut_virhepisteet - missing_at_points - direct;
                if count > 0 && total >= 0 && total % count == 0 {
                    section_other_points(total / count)?
                } else {
                    section_other_points(0)?
                }
            };
            if source.puuttuvat_rt != missing_details
                || source.tulkatut_jaksot != completed
                || source.jaksoja != section_count
                || source.jaksojen_muut_pisteet != other_points
            {
                let mut penalties = driver.lisavirheet.clone();
                let source = penalties.tulkki_siirto.as_mut().unwrap();
                source.puuttuvat_rt = missing_details;
                source.tulkatut_jaksot = completed;
                source.jaksoja = section_count;
                source.jaksojen_muut_pisteet = other_points;
                changes.push((number, penalties));
            }
            continue;
        }
        let mut missing = 0u32;
        for receipt in &receipts {
            missing = missing
                .checked_add(receipt.section.missing_signs)
                .ok_or(CoreError::ArithmeticOverflow("RT-määrä"))?;
        }
        let rt_points = i64::from(missing)
            .checked_mul(parameters.missing_rt)
            .ok_or(CoreError::ArithmeticOverflow("RT-pisteet"))?;
        let points_by_section = section_other_points(parameters.extra_at)?;
        let mut other_points = points_by_section.iter().try_fold(0i64, |sum, p| {
            sum.checked_add(*p)
                .ok_or(CoreError::ArithmeticOverflow("Muut virhepisteet"))
        })?;
        if project.kilpailu.ajanottotapa == 0 {
            let result = manual_result(project, config, number)?;
            other_points = other_points
                .checked_add(crate::manual_timing::missing_at_points(project, &result)?)
                .ok_or(CoreError::ArithmeticOverflow("Puuttuvan AT:n pisteet"))?;
        }
        let previous = driver.lisavirheet.tulkki_siirto.as_ref().map_or_else(
            || {
                driver
                    .tallennettu_tulos
                    .as_ref()
                    .filter(|r| r.ilman_emit)
                    .map_or(0, |r| r.puuttuvien_at_pisteet)
            },
            |s| s.muut_virhepisteet,
        );
        let manual = driver
            .lisavirheet
            .muut_virhepisteet
            .checked_sub(previous)
            .unwrap_or(0)
            .max(0);
        let mut penalties = driver.lisavirheet.clone();
        penalties.puuttuvat_rt_kpl = missing;
        penalties.puuttuvat_rt_pisteet = rt_points;
        penalties.muut_virhepisteet = manual
            .checked_add(other_points)
            .ok_or(CoreError::ArithmeticOverflow("Muut virhepisteet"))?;
        penalties.tulkki_siirto = Some(TulkkiPenaltySource {
            tunniste: token,
            tapahtumatunniste: Some(base_token),
            tulkatut_jaksot: completed,
            jaksoja: section_count,
            muut_virhepisteet: other_points,
            puuttuvat_rt: missing_details,
            jaksojen_muut_pisteet: points_by_section,
        });
        changes.push((number, penalties));
    }
    Ok(changes)
}

/// Mitätöinti ei poista alkuperäisiä tulkintoja eikä muuta muita saman numeron esiintymiä.
pub fn void_sign(
    project: &mut Project,
    route: u32,
    id: &str,
    config: &Config,
) -> Result<Vec<u32>, crate::CoreError> {
    let entry = project
        .oikeat_rivit
        .iter_mut()
        .find(|r| r.rata == route)
        .and_then(|r| r.merkinnat.iter_mut().find(|e| e.tunniste == id))
        .ok_or_else(|| crate::CoreError::InvalidInput("Kilpeä ei löydy".into()))?;
    if entry.merkinta.kind.is_clock() {
        return Err(crate::CoreError::InvalidInput(
            "Aika-asemaa ei mitätöidä RT-kilpenä".into(),
        ));
    }
    if entry.mitatoity {
        return Err(crate::CoreError::InvalidInput(
            "Kilpi on jo mitätöity. Mitätöintiä ei voi perua.".into(),
        ));
    }
    entry.mitatoity = true;
    let changes = penalty_changes(project, config)?;
    let mut affected = Vec::new();
    for (number, penalties) in changes {
        let d = project
            .kilpailijat
            .iter_mut()
            .find(|d| d.numero == number)
            .unwrap();
        if !d.lisavirheet.same_scoring(&penalties)
            || d.lisavirheet
                .tulkki_siirto
                .as_ref()
                .map(|s| &s.puuttuvat_rt)
                != penalties.tulkki_siirto.as_ref().map(|s| &s.puuttuvat_rt)
        {
            affected.push(number);
        }
        d.lisavirheet = penalties;
    }
    Ok(affected)
}

/// Kohdistus on järjestyksen ja asematyypin mukainen; tunnisteiden nimet saavat erota.
pub(crate) fn manual_result(
    project: &Project,
    config: &Config,
    number: u32,
) -> Result<crate::corrections::StoredResult, crate::CoreError> {
    use crate::{CoreError, application, manual_timing};
    let invalid = |s: &str| CoreError::InvalidInput(s.into());
    let manifest: Value = serde_json::from_str(&config.published_manifest)
        .map_err(|_| invalid("Julkaistu oikea rivi puuttuu"))?;
    let route =
        route_for_pair(&manifest, number).ok_or_else(|| invalid("Parin julkaistu rata puuttuu"))?;
    let entries = route["sections"]
        .as_array()
        .ok_or_else(|| invalid("Julkaistut jaksot puuttuvat"))?
        .iter()
        .flat_map(|s| {
            s["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .map(move |e| (s, e))
        })
        .filter(|(_, e)| matches!(e["kind"].as_str(), Some("at" | "jat" | "ula" | "maali")))
        .collect::<Vec<_>>();
    let driver = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    let mut result = match driver.tallennettu_tulos.as_ref().filter(|r| r.ilman_emit) {
        Some(r) => {
            application::validate_project_result(project, number, r)?;
            r.clone()
        }
        None => application::capture_manual_result(project, number)?,
    };
    let rows = result
        .rivit
        .iter()
        .filter(|r| r.station_type != "L")
        .collect::<Vec<_>>();
    let mut entries = entries.into_iter().peekable();
    let mut updates = Vec::new();
    for row in &rows {
        if row.station_type == "ULA" && entries.peek().is_none_or(|(_, e)| e["kind"] != "ula") {
            continue;
        }
        let Some((section, entry)) = entries.next() else {
            return Err(invalid(
                "Oikean rivin aika-asemien määrä ei vastaa laskentarataa. Tarkista Radat ja asemat sekä Kilvet.",
            ));
        };
        let expected = match row.station_type.as_str() {
            "AT" => "at",
            "JAT" => "jat",
            "ULA" => "ula",
            "M" => "maali",
            _ => return Err(invalid("Tuntematon laskentaradan aika-asema")),
        };
        if entry["kind"].as_str() != Some(expected) {
            return Err(invalid(
                "Oikean rivin aika-asemien järjestys tai tyyppi eroaa laskentaradasta",
            ));
        }
        let Some(receipt) = config.received.iter().find(|r| {
            r.pair_number == number && Some(r.section.id.as_str()) == section["id"].as_str()
        }) else {
            continue;
        };
        let id = entry["id"]
            .as_str()
            .ok_or_else(|| invalid("Asematunniste puuttuu"))?;
        if receipt
            .section
            .pending_time_ids
            .iter()
            .any(|pending| pending == id)
        {
            continue;
        }
        let missing = receipt.section.missing_entry_ids.iter().any(|v| v == id);
        let clock = receipt
            .section
            .times
            .get(id)
            .map(|v| {
                crate::terminal::parse_clock(v)
                    .map(|t| [i64::from(t.hour), i64::from(t.minute), i64::from(t.second)])
            })
            .transpose()?;
        let same = if missing {
            result.puuttuvat_aikamerkinnat.contains(&row.output_index)
        } else if let Some(clock) = clock {
            result.kasin_annetut_kellot.contains(&row.output_index) && row.clock == clock
        } else {
            !result.kasin_annetut_kellot.contains(&row.output_index)
                && !result.puuttuvat_aikamerkinnat.contains(&row.output_index)
        };
        if !same {
            updates.push((row.output_index, missing, clock));
        }
    }
    if entries.next().is_some() {
        return Err(invalid(
            "Oikean rivin aika-asemien määrä ei vastaa laskentarataa. Tarkista Radat ja asemat sekä Kilvet.",
        ));
    }
    if !updates.is_empty() {
        // Kaikki muuttuneet ajat vapautetaan ensin, jotta vanha seuraava aika ei estä kelvollista korjausta.
        for (row, _, _) in &updates {
            result.kasin_annetut_kellot.retain(|r| r != row);
            result.puuttuvat_aikamerkinnat.retain(|r| r != row);
            result
                .rivit
                .iter_mut()
                .find(|r| r.output_index == *row)
                .unwrap()
                .clock = [0; 3];
        }
        for (row, missing, clock) in updates {
            if missing {
                manual_timing::set_missing(&mut result, row)?;
            } else if let Some(clock) = clock {
                result.set_clock(row, clock)?;
            }
        }
        result = application::recalculate_stored_result(project, number, &result)?;
    }
    result.puuttuvien_at_pisteet = manual_timing::missing_at_points(project, &result)?;
    Ok(result)
}

fn missing_rt_sections(
    config: &Config,
    number: u32,
    route: &Value,
) -> Result<Vec<crate::native::MissingRtSection>, String> {
    let manifest: Value = serde_json::from_str(&config.published_manifest)
        .map_err(|_| "Julkaistu oikea rivi puuttuu")?;
    let mut missing = Vec::new();
    for (index, section) in list(route, "sections")?.iter().enumerate() {
        let Some(receipt) = config.received.iter().find(|r| {
            r.pair_number == number && Some(r.section.id.as_str()) == section["id"].as_str()
        }) else {
            continue;
        };
        validate_event(
            &InterpretationEvent {
                event_id: receipt.event_id,
                pair_number: number,
                revision: config.revision,
                reviewer: receipt.reviewer.clone(),
                saved_at: receipt.saved_at.clone(),
                sections: vec![receipt.section.clone()],
                original_sections: Vec::new(),
                original_labels: BTreeMap::new(),
            },
            &manifest,
            config.revision,
        )?;
        let labels = list(section, "entries")?
            .iter()
            .filter(|e| {
                matches!(e["kind"].as_str(), Some("rt" | "mrt" | "leima" | "jrt"))
                    && e["id"].as_str().is_some_and(|id| {
                        receipt
                            .section
                            .missing_entry_ids
                            .iter()
                            .any(|missing| missing == id)
                    })
            })
            .filter_map(|e| e["label"].as_str().map(str::to_owned))
            .collect::<Vec<_>>();
        if !labels.is_empty() {
            missing.push(crate::native::MissingRtSection {
                jakso: index as u32 + 1,
                kilvet: labels,
            });
        }
    }
    Ok(missing)
}

fn list<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value[key]
        .as_array()
        .ok_or_else(|| "Julkaistut kilpailutiedot ovat puutteelliset.".into())
}

pub fn route_for_pair(manifest: &Value, number: u32) -> Option<&Value> {
    let pair = manifest["pairs"]
        .as_array()?
        .iter()
        .find(|p| p["number"].as_u64() == Some(u64::from(number)))?;
    manifest["routes"]
        .as_array()?
        .iter()
        .find(|r| r["id"] == pair["routeId"])
}

fn validate_event(
    event: &InterpretationEvent,
    manifest: &Value,
    revision: u64,
) -> Result<(), String> {
    if event.revision != revision
        || event.sections.is_empty()
        || event.sections.len() > 200
        || event.reviewer.len() > 120
        || chrono::DateTime::parse_from_rfc3339(&event.saved_at).is_err()
    {
        return Err("Vastaanotetun tulkinnan tietoversio tai perustiedot eivät kelpaa.".into());
    }
    let pair = list(manifest, "pairs")?
        .iter()
        .find(|p| p["number"].as_u64() == Some(u64::from(event.pair_number)))
        .ok_or("Vastaanotettua kilpailuparia ei löydy julkaistuista tiedoista.")?;
    let route =
        route_for_pair(manifest, event.pair_number).ok_or("Vastaanotetun parin rata puuttuu.")?;
    let mut seen = BTreeSet::new();
    for section in &event.sections {
        if !seen.insert(&section.id)
            || !section.interpreted
            || section.note.len() > 4000
            || section.other_errors > 10000
            || section.other_penalty_points.is_some_and(|p| {
                section.other_errors != 0 || p > 6_000_000 || !p.is_multiple_of(100)
            })
        {
            return Err("Vastaanotetun jakson tiedot eivät kelpaa.".into());
        }
        let definition = list(route, "sections")?
            .iter()
            .find(|s| s["id"].as_str() == Some(&section.id))
            .ok_or("Vastaanotettu jakso ei kuulu parin radalle.")?;
        let entries = list(definition, "entries")?;
        let missing = section.missing_entry_ids.iter().collect::<BTreeSet<_>>();
        if missing.len() != section.missing_entry_ids.len() {
            return Err("Puuttuva asema toistuu tulkinnassa.".into());
        }
        let mut signs = 0;
        let mut clocks = 0;
        for id in section.missing_entry_ids.iter().chain(section.times.keys()) {
            if !entries
                .iter()
                .any(|e| e["id"].as_str() == Some(id) && e["voided"] != true)
            {
                return Err("Tulkinnassa on tuntematon tai mitätöity asema.".into());
            }
        }
        for entry in entries {
            if entry["voided"] == true {
                continue;
            }
            let id = entry["id"].as_str().ok_or("Aseman tunniste puuttuu.")?;
            let absent = section.missing_entry_ids.iter().any(|s| s == id);
            let clock = matches!(entry["kind"].as_str(), Some("at" | "jat" | "ula" | "maali"));
            let manual = clock
                && entry["manualTime"].as_bool().unwrap_or(
                    matches!(entry["kind"].as_str(), Some("at" | "jat")) || pair["emit"] == false,
                );
            if absent {
                if clock {
                    clocks += 1;
                } else {
                    signs += 1;
                }
            }
            if let Some(time) = section.times.get(id) {
                let bytes = time.as_bytes();
                if absent
                    || !manual
                    || bytes.len() != 8
                    || bytes[2] != b':'
                    || bytes[5] != b':'
                    || !bytes
                        .iter()
                        .enumerate()
                        .all(|(i, c)| i == 2 || i == 5 || c.is_ascii_digit())
                    || crate::terminal::parse_clock(time).is_err()
                {
                    return Err(
                        "Tulkinnassa on virheellinen tai väärälle asemalle kirjattu aika.".into(),
                    );
                }
            } else if manual
                && !absent
                && !section.pending_time_ids.iter().any(|pending| pending == id)
            {
                return Err("Vastaanotetusta tulkinnasta puuttuu vaadittu aika.".into());
            }
        }
        let pending = section.pending_time_ids.iter().collect::<BTreeSet<_>>();
        if pending.len() != section.pending_time_ids.len()
            || pending.iter().any(|id| {
                missing.contains(*id)
                    || section.times.contains_key(*id)
                    || !entries.iter().any(|e| {
                        e["id"].as_str() == Some(id.as_str())
                            && e["kind"] == "ula"
                            && e["manualTime"].as_bool().unwrap_or(pair["emit"] == false)
                    })
            })
        {
            return Err("Täydennettävien ULA-aikojen luettelo ei kelpaa.".into());
        }
        if signs != section.missing_signs || clocks != section.missing_times {
            return Err("Tulkinnan puuttuvien asemien määrät eivät vastaa asemaluetteloa.".into());
        }
    }
    Ok(())
}

/// Yksi sivu muuttuu kokonaan tai ei lainkaan. Kohdistin ja viimeisimmät jaksot tallennetaan yhdessä.
pub fn apply_page(config: &mut Config, page: ChangePage) -> Result<usize, String> {
    let manifest: Value = serde_json::from_str(&config.published_manifest)
        .map_err(|_| "Julkaistun oikean rivin tiedot puuttuvat.")?;
    if page.events.len() > 100 || (page.has_more && page.events.is_empty()) {
        return Err("Muutossyötteen sivutus ei kelpaa.".into());
    }
    let mut cursor = config.cursor;
    for event in &page.events {
        if event.event_id <= cursor {
            return Err("Muutossyötteen tapahtumajärjestys ei kelpaa.".into());
        }
        validate_event(event, &manifest, config.revision)?;
        cursor = event.event_id;
    }
    if cursor != page.next_cursor {
        return Err("Muutossyötteen kuittaus ei vastaa tapahtumia.".into());
    }
    let count = page.events.len();
    let mut received = config
        .received
        .iter()
        .cloned()
        .map(|r| ((r.pair_number, r.section.id.clone()), r))
        .collect::<BTreeMap<_, _>>();
    config.history.extend(page.events.iter().cloned());
    for event in page.events {
        for section in event.sections {
            let key = (event.pair_number, section.id.clone());
            if received
                .get(&key)
                .is_some_and(|old| old.event_id >= event.event_id)
            {
                continue;
            }
            received.insert(
                key,
                ReceivedSection {
                    event_id: event.event_id,
                    pair_number: event.pair_number,
                    reviewer: event.reviewer.clone(),
                    saved_at: event.saved_at.clone(),
                    section,
                },
            );
        }
    }
    config.received = received.into_values().collect();
    config.cursor = cursor;
    Ok(count)
}

impl std::fmt::Debug for Config {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TulkkiConfig")
            .field("site", &self.site)
            .field("competition_id", &self.competition_id)
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

pub fn api_url(site: &str) -> Result<Url, String> {
    let mut url = Url::parse(site.trim()).map_err(|_| "Anna sivuston HTTPS-osoite.")?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Käytä HTTPS-osoitetta ilman käyttäjätunnusta, kyselyä tai ankkuria.".into());
    }
    let path = url.path().trim_end_matches('/');
    let path = if path.ends_with("/wp-json/as-tulkki/v1") {
        format!("{path}/")
    } else {
        format!("{path}/wp-json/as-tulkki/v1/")
    };
    url.set_path(&path);
    Ok(url)
}

fn random_hex(bytes: usize) -> Result<String, String> {
    let mut data = vec![0u8; bytes];
    getrandom::fill(&mut data).map_err(|_| "Tunnusten turvallinen muodostaminen epäonnistui.")?;
    Ok(data.iter().map(|b| format!("{b:02x}")).collect())
}

impl Config {
    pub fn stage_join_code(&mut self) -> Result<(), String> {
        if self.pending_join_code.is_empty() {
            self.pending_join_code = random_hex(6)?.to_uppercase();
        }
        Ok(())
    }
    /// Vain puhelimen liittymiskoodi; hallinta-avainta tai salasanaa ei sisällytetä.
    pub fn join_url(&self) -> Result<String, String> {
        if self.closed
            || self.competition_id == 0
            || self.revision == 0
            || self.join_code.len() != 12
            || !self.join_code.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err("Luo verkkokilpailu ja lähetä kilpailutiedot ensin".into());
        }
        let mut url = api_url(&self.site)?;
        let prefix = url
            .path()
            .strip_suffix("wp-json/as-tulkki/v1/")
            .ok_or("Sivuston osoite on virheellinen")?
            .to_owned();
        url.set_path(&prefix);
        let site = url.to_string();
        url.set_path(&format!("{prefix}as-tulkki/"));
        url.query_pairs_mut()
            .append_pair("join", &self.join_code)
            .append_pair("site", &site);
        let fragment = url.query().unwrap().to_owned();
        url.set_query(None);
        url.set_fragment(Some(&fragment));
        Ok(url.to_string())
    }
    /// Tallenna palautunut asetus ennen ensimmäistä verkkopyyntöä.
    pub fn prepare(site: &str, user: &str, name: &str) -> Result<Self, String> {
        let url = api_url(site)?;
        if user.trim().is_empty() || name.trim().is_empty() {
            return Err("Anna WordPress-käyttäjätunnus ja avaa nimetty kilpailu.".into());
        }
        Ok(Self {
            site: url.to_string(),
            user: user.trim().into(),
            name: name.into(),
            request_id: format!("create-{}", random_hex(16)?),
            join_code: random_hex(6)?.to_uppercase(),
            key: random_hex(24)?,
            ..Self::default()
        })
    }
}

/// Esiintymän pysyvä tunniste erottaa saman jakson kaksi samannumeroista kilpeä.
pub fn manifest(project: &Project) -> Result<Value, String> {
    if project.kilpailijat.is_empty() {
        return Err("Lisää kilpailuparit ennen lähettämistä.".into());
    }
    let mut routes = Vec::new();
    for stored in &project.oikeat_rivit {
        let route = project
            .radat
            .iter()
            .find(|r| r.numero == stored.rata)
            .ok_or("Oikean rivin rata puuttuu.")?;
        let mut sections: BTreeMap<u32, Vec<Value>> = BTreeMap::new();
        for entry in &stored.merkinnat {
            let kind = match entry.merkinta.kind {
                EntryKind::Rt => "rt",
                EntryKind::Mrt => "mrt",
                EntryKind::Stamp => "leima",
                EntryKind::Jrt => "jrt",
                EntryKind::At => "at",
                EntryKind::Jat => "jat",
                EntryKind::Ula => "ula",
                EntryKind::Finish => "maali",
            };
            sections
                .entry(entry.merkinta.section)
                .or_default()
                .push(json!({
                    "id": entry.tunniste, "label": entry.merkinta.label, "kind": kind,
                    "manualTime": if project.kilpailu.ajanottotapa==0 && entry.merkinta.kind.is_clock(){Some(true)}else{entry.kasiaika}, "voided": entry.mitatoity,
                }));
        }
        if sections.is_empty() {
            return Err(format!("Radalla {} ei ole kilpiä.", stored.rata));
        }
        routes.push(json!({"id":format!("r{}",stored.rata), "name":route.nimi,
            "sections":sections.into_iter().map(|(number,entries)| json!({
                "id":format!("r{}-j{number}",stored.rata),"name":format!("Jakso {number}"),"entries":entries,
            })).collect::<Vec<_>>() }));
    }
    let mut pairs = Vec::new();
    for pair in &project.kilpailijat {
        let class = project
            .luokat
            .iter()
            .find(|c| c.tunnus == pair.luokka)
            .ok_or("Kilpailuparin luokka puuttuu.")?;
        if !project
            .oikeat_rivit
            .iter()
            .any(|r| r.rata == class.rata && !r.merkinnat.is_empty())
        {
            return Err(format!(
                "Parin {} radalta {} puuttuu oikea rivi. Tuo se Kilvet-välilehdellä.",
                pair.numero, class.rata
            ));
        }
        pairs.push(json!({"number":pair.numero,"driver":pair.kuljettaja,"navigator":pair.kartanlukija,
            "className":pair.luokka,"routeId":format!("r{}",class.rata),"emit":project.kilpailu.ajanottotapa != 0 && pair.emit_number().is_some()}));
    }
    Ok(json!({"routes":routes,"pairs":pairs}))
}

pub struct Api {
    base: Url,
    client: Client,
}

/// Vain kokonaan tulkattu, laskennallisesti valmis normaalitulos saa automaattisen hyväksynnän.
pub fn automatic_result_key(project: &Project, config: &Config, number: u32) -> Option<String> {
    use sha2::{Digest, Sha256};
    if !config.automatic_results {
        return None;
    }
    let manifest: Value = serde_json::from_str(&config.published_manifest).ok()?;
    if self::manifest(project).ok().as_ref() != Some(&manifest) {
        return None;
    }
    let sections = route_for_pair(&manifest, number)?["sections"].as_array()?;
    if sections.is_empty()
        || !sections.iter().all(|s| {
            config.received.iter().any(|r| {
                r.pair_number == number
                    && Some(r.section.id.as_str()) == s["id"].as_str()
                    && r.section.interpreted
            })
        })
    {
        return None;
    }
    let driver = project.kilpailijat.iter().find(|d| d.numero == number)?;
    if driver.tila.is_some() {
        return None;
    }
    let mut result = driver.tallennettu_tulos.clone()?;
    if result.tila != "TRK"
        || !result.can_approve()
        || crate::application::validate_project_result(project, number, &result).is_err()
    {
        return None;
    }
    result.hyvaksytty = false;
    let bytes = serde_json::to_vec(&(result, &driver.lisavirheet)).ok()?;
    let key = format!("{:x}", Sha256::digest(bytes));
    (config.automatic_prints.get(&number.to_string()) != Some(&key)).then_some(key)
}
pub fn void_only(old: &Value, new: &Value) -> bool {
    let mut expected = old.clone();
    // Luokan näyttönimi on metatietoa: sen lisääminen ei muuta laskentarataa.
    if let Some(pairs) = expected["pairs"].as_array_mut() {
        for (i, pair) in pairs.iter_mut().enumerate() {
            if let Some(name) = new["pairs"][i].get("className") {
                pair["className"] = name.clone();
            }
        }
    }
    let Some(routes) = expected["routes"].as_array_mut() else {
        return false;
    };
    for (ri, r) in routes.iter_mut().enumerate() {
        let Some(sections) = r["sections"].as_array_mut() else {
            return false;
        };
        for (si, s) in sections.iter_mut().enumerate() {
            let Some(entries) = s["entries"].as_array_mut() else {
                return false;
            };
            for (ei, e) in entries.iter_mut().enumerate() {
                let n = &new["routes"][ri]["sections"][si]["entries"][ei];
                if e["voided"] != n["voided"] {
                    if e["voided"] == true
                        || n["voided"] != true
                        || !matches!(e["kind"].as_str(), Some("rt" | "mrt" | "leima" | "jrt"))
                    {
                        return false;
                    }
                    e["voided"] = json!(true);
                }
            }
        }
    }
    expected == *new
}
/// Kilpikorjaus säilyttää parit, jaksot ja aika-asemien pysyvät tunnisteet.
pub fn compatible_row_change(old: &Value, new: &Value) -> bool {
    let mut expected = old.clone();
    if let Some(pairs) = expected["pairs"].as_array_mut() {
        for (i, p) in pairs.iter_mut().enumerate() {
            if let Some(name) = new["pairs"][i].get("className") {
                p["className"] = name.clone();
            }
        }
    } else {
        return false;
    }
    let Some(routes) = expected["routes"].as_array_mut() else {
        return false;
    };
    for (ri, r) in routes.iter_mut().enumerate() {
        let Some(sections) = r["sections"].as_array_mut() else {
            return false;
        };
        for (si, s) in sections.iter_mut().enumerate() {
            let Some(old_entries) = s["entries"].as_array() else {
                return false;
            };
            let Some(new_entries) = new["routes"][ri]["sections"][si]["entries"].as_array() else {
                return false;
            };
            let clock =
                |e: &&Value| matches!(e["kind"].as_str(), Some("at" | "jat" | "ula" | "maali"));
            let old_clocks = old_entries.iter().filter(clock).collect::<Vec<_>>();
            let retained = new_entries
                .iter()
                .filter(clock)
                .filter(|e| e["kind"] != "ula" || old_clocks.iter().any(|old| old["id"] == e["id"]))
                .collect::<Vec<_>>();
            if old_clocks != retained {
                return false;
            }
            for (i, e) in new_entries.iter().enumerate() {
                if e["kind"] == "ula"
                    && !old_clocks.iter().any(|old| old["id"] == e["id"])
                    && (i == 0
                        || new_entries[i - 1]["kind"] != "jat"
                        || new_entries.get(i + 1).is_some_and(|n| n["kind"] == "ula"))
                {
                    return false;
                }
            }
            for entry in old_entries {
                let next = new_entries.iter().find(|e| e["id"] == entry["id"]);
                if entry["voided"] == true && next.is_none_or(|e| e["voided"] != true) {
                    return false;
                }
                if let Some(next) = next
                    && (entry["kind"] != next["kind"] || entry["label"] != next["label"])
                {
                    return false;
                }
            }
            s["entries"] = Value::Array(new_entries.clone());
        }
    }
    expected == *new
}
pub fn reconcile_manifest(config: &mut Config, manifest: &Value) -> Result<(), String> {
    let old: Value = serde_json::from_str(&config.published_manifest).unwrap_or(Value::Null);
    if config.competition_id > 0 && old != *manifest && !compatible_row_change(&old, manifest) {
        return Err("Julkaistussa kilpailussa voi korjata kilpiä ja lisätä ULA:n JAT:n jälkeen. Vanhoja aika-asemia tai jaksorajoja ei voi vaihtaa tallennettujen aikojen alta.".into());
    }
    if old != *manifest && !void_only(&old, manifest) {
        note_route_revision(
            config,
            &old,
            manifest,
            config
                .revision
                .checked_add(1)
                .ok_or("Tietoversiot loppuivat")?,
        );
    }
    freeze_history(config);
    config.published_manifest = manifest.to_string();
    normalize_received(config)
}
fn note_route_revision(config: &mut Config, old: &Value, new: &Value, revision: u64) {
    if config.scoring_revision == 0 {
        config.scoring_revision = config.revision;
    }
    for route in new["routes"].as_array().into_iter().flatten() {
        let previous = old["routes"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|r| r["id"] == route["id"]);
        if previous.is_some_and(|p| {
            p != route
                && !void_only(
                    &json!({"routes":[p],"pairs":[]}),
                    &json!({"routes":[route],"pairs":[]}),
                )
        }) && let Some(id) = route["id"].as_str()
        {
            config
                .route_scoring_revisions
                .insert(id.to_owned(), revision);
        }
    }
}
fn freeze_history(config: &mut Config) {
    let old: Value = serde_json::from_str(&config.published_manifest).unwrap_or(Value::Null);
    let mut labels = BTreeMap::new();
    for route in old["routes"].as_array().into_iter().flatten() {
        for section in route["sections"].as_array().into_iter().flatten() {
            for (i, e) in section["entries"]
                .as_array()
                .into_iter()
                .flatten()
                .enumerate()
            {
                if let (Some(id), Some(label)) = (e["id"].as_str(), e["label"].as_str()) {
                    labels.insert(id.to_owned(), format!("{label} (rivi {})", i + 1));
                }
            }
        }
    }
    for event in &mut config.history {
        if event.original_sections.is_empty() {
            event.original_sections = event
                .sections
                .iter()
                .filter_map(|s| serde_json::to_value(s).ok())
                .collect();
        }
        if event.original_labels.is_empty() {
            let ids = event
                .original_sections
                .iter()
                .flat_map(|s| {
                    s["missingEntryIds"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|v| v.as_str())
                        .chain(
                            s["times"]
                                .as_object()
                                .into_iter()
                                .flat_map(|m| m.keys().map(String::as_str)),
                        )
                })
                .collect::<BTreeSet<_>>();
            event.original_labels = labels
                .iter()
                .filter(|(id, _)| ids.contains(id.as_str()))
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
        }
    }
}
pub fn correct_route(
    project: &mut Project,
    config: &Config,
    route: u32,
    preview: &crate::interpretation::Preview,
) -> Result<(), crate::CoreError> {
    let mut candidate = project.clone();
    crate::interpretation::replace_route(&mut candidate, route, preview)?;
    if !config.published_manifest.is_empty() {
        let mut staged = config.clone();
        let published = manifest(&candidate).map_err(crate::CoreError::InvalidInput)?;
        reconcile_manifest(&mut staged, &published).map_err(crate::CoreError::InvalidInput)?;
        for (number, penalties) in penalty_changes(&candidate, &staged)? {
            let result = if candidate.kilpailu.ajanottotapa == 0 {
                Some(manual_result(&candidate, &staged, number)?)
            } else {
                None
            };
            let driver = candidate
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .unwrap();
            let changed = !driver.lisavirheet.same_scoring(&penalties);
            driver.lisavirheet = penalties;
            if let Some(result) = result {
                driver.tallennettu_tulos = Some(result);
            }
            if changed && let Some(result) = &mut driver.tallennettu_tulos {
                result.hyvaksytty = false;
            }
        }
    }
    candidate.validate()?;
    *project = candidate;
    Ok(())
}
fn normalize_received(config: &mut Config) -> Result<(), String> {
    let manifest: Value =
        serde_json::from_str(&config.published_manifest).map_err(|_| "Julkaistu rivi puuttuu")?;
    for receipt in &mut config.received {
        let section = route_for_pair(&manifest, receipt.pair_number)
            .and_then(|r| r["sections"].as_array())
            .and_then(|ss| {
                ss.iter()
                    .find(|s| s["id"].as_str() == Some(&receipt.section.id))
            })
            .ok_or("Tulkinnan jakso puuttuu")?;
        let entries = list(section, "entries")?;
        let active = entries
            .iter()
            .filter(|e| e["voided"] != true)
            .filter_map(|e| e["id"].as_str())
            .collect::<BTreeSet<_>>();
        receipt
            .section
            .missing_entry_ids
            .retain(|id| active.contains(id.as_str()));
        receipt
            .section
            .times
            .retain(|id, _| active.contains(id.as_str()));
        receipt.section.missing_signs = entries
            .iter()
            .filter(|e| {
                receipt
                    .section
                    .missing_entry_ids
                    .iter()
                    .any(|id| e["id"].as_str() == Some(id))
                    && !matches!(e["kind"].as_str(), Some("at" | "jat" | "ula" | "maali"))
            })
            .count() as u32;
        receipt.section.missing_times =
            receipt.section.missing_entry_ids.len() as u32 - receipt.section.missing_signs;
        let emit = manifest["pairs"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p["number"].as_u64() == Some(u64::from(receipt.pair_number)))
            .and_then(|p| p["emit"].as_bool())
            .unwrap_or(false);
        receipt.section.pending_time_ids = entries
            .iter()
            .filter(|e| e["kind"] == "ula" && e["manualTime"].as_bool().unwrap_or(!emit))
            .filter_map(|e| e["id"].as_str())
            .filter(|id| {
                !receipt.section.missing_entry_ids.iter().any(|v| v == id)
                    && !receipt.section.times.contains_key(*id)
            })
            .map(str::to_owned)
            .collect();
    }
    Ok(())
}
fn adopt_manifest(config: &mut Config, manifest: &Value, revision: u64) -> Result<(), String> {
    if config.published_manifest.is_empty() {
        config.published_manifest = manifest.to_string();
        config.revision = revision;
        return Ok(());
    }
    let old: Value =
        serde_json::from_str(&config.published_manifest).map_err(|_| "Julkaistu rivi puuttuu")?;
    if old != *manifest && !config.received.is_empty() && !compatible_row_change(&old, manifest) {
        return Err(
            "Palvelimen aika-asemien tai jaksojen rakenne on muuttunut. Tarkista kilpailutiedot."
                .into(),
        );
    }
    if revision != config.revision {
        if !void_only(&old, manifest) {
            note_route_revision(config, &old, manifest, revision);
        } else if config.scoring_revision == 0 {
            config.scoring_revision = config.revision;
        }
    }
    freeze_history(config);
    config.published_manifest = manifest.to_string();
    config.revision = revision;
    normalize_received(config)
}
impl Api {
    pub fn manage(&self, config: &mut Config, password: &str, rotate: bool) -> Result<(), String> {
        if password.trim().is_empty() || config.user.trim().is_empty() {
            return Err("Anna ylläpitäjän käyttäjätunnus ja WordPressin sovellussalasana.".into());
        }
        let (suffix, body) = if rotate {
            if config.pending_join_code.is_empty() {
                return Err("Uutta liittymiskoodia ei ole tallennettu".into());
            }
            (
                "credentials",
                json!({"joinCode":config.pending_join_code,"key":config.key}),
            )
        } else {
            ("connection", json!({"active":config.closed}))
        };
        let data = self.send(
            self.client
                .put(self.url(&format!(
                    "management/competitions/{}/{suffix}",
                    config.competition_id
                ))?)
                .basic_auth(&config.user, Some(password))
                .json(&body),
        )?;
        if data["id"].as_u64() != Some(config.competition_id) {
            return Err("Hallintapyynnön kuittaus puuttuu. Uusi sama toiminto.".into());
        }
        config.closed = !data["active"]
            .as_bool()
            .ok_or("Yhteystilan kuittaus puuttuu")?;
        if rotate {
            config.join_code = std::mem::take(&mut config.pending_join_code);
        }
        Ok(())
    }
    pub fn new(site: &str) -> Result<Self, String> {
        Ok(Self {
            base: api_url(site)?,
            client: Client::builder()
                .timeout(Duration::from_secs(20))
                .redirect(Policy::none())
                .build()
                .map_err(|_| "Verkkoasiakasta ei voitu käynnistää.")?,
        })
    }
    fn send(&self, request: reqwest::blocking::RequestBuilder) -> Result<Value, String> {
        let response = request.send().map_err(|_| "Yhteys sivustoon epäonnistui. Tarkista osoite ja verkkoyhteys; lähetyksen voi uusia.")?;
        let status = response.status();
        let mut bytes = Vec::new();
        response
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Vastauksen lukeminen keskeytyi. Uusi lähetys.")?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err("Palvelimen vastaus on liian suuri.".into());
        }
        let data: Value = serde_json::from_slice(&bytes).map_err(
            |_| "Sivusto ei palauttanut rajapinnan JSON-vastausta. Tarkista lisäosa ja osoite.",
        )?;
        if !status.is_success() {
            return Err(match status.as_u16() {
                401 | 403 => "Kirjautuminen ei kelpaa tai oikeudet puuttuvat. Käytä WordPressin ylläpitäjän käyttäjätunnusta ja sovellussalasanaa.",
                404 => "Rajapintaa tai avointa verkkokilpailua ei löytynyt. Tarkista lisäosan versio ja sivuston osoite.",
                409 => "Kilpailutiedot tai tunnukset ovat ristiriidassa. Tulkkausten alettua sallitaan vain RT-kilpien mitätöinti.",
                400 => "Palvelin hylkäsi kilpailutiedot. Tarkista kilvet, kilpailuparit ja aikavalinnat.",
                _ => "Palvelin ilmoitti virheestä. Tiedonsiirtoa ei kuitattu onnistuneeksi; uusi lähetys myöhemmin.",
            }.into());
        }
        Ok(data)
    }
    fn url(&self, path: &str) -> Result<Url, String> {
        self.base
            .join(path)
            .map_err(|_| "Rajapinnan osoite on virheellinen.".into())
    }
    pub fn create(&self, config: &mut Config, password: &str) -> Result<(), String> {
        if password.trim().is_empty() {
            return Err(
                "Anna WordPressin sovellussalasana. Tavallista kirjautumissalasanaa ei käytetä."
                    .into(),
            );
        }
        let data = self.send(self.client.post(self.url("management/competitions")?)
            .basic_auth(&config.user, Some(password.trim()))
            .json(&json!({"requestId":config.request_id,"name":config.name,"joinCode":config.join_code,"key":config.key})))?;
        config.competition_id = data["id"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("Vastauksesta puuttuu kilpailun tunniste.")?;
        config.revision = data["revision"]
            .as_u64()
            .ok_or("Vastauksesta puuttuu tietoversio.")?;
        Ok(())
    }
    pub fn publish(&self, config: &mut Config, manifest: &Value) -> Result<(), String> {
        let path = format!("competitions/{}", config.competition_id);
        let current = self.send(
            self.client
                .get(self.url(&path)?)
                .header("X-AS-Key", &config.key),
        )?;
        let revision = current["revision"]
            .as_u64()
            .ok_or("Palvelimen tietoversio puuttuu.")?;
        let data = self.send(
            self.client
                .put(self.url(&format!("{path}/manifest"))?)
                .header("X-AS-Key", &config.key)
                .json(&json!({"baseRevision":revision,"manifest":manifest})),
        )?;
        let revision = data["revision"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("Tallennuskuittaus puuttuu.")?;
        let old: Value = serde_json::from_str(&config.published_manifest).unwrap_or(Value::Null);
        if config.revision != revision && !compatible_row_change(&old, manifest) {
            config.cursor = 0;
            config.received.clear();
            config.scoring_revision = 0;
            config.route_scoring_revisions.clear();
        } else if config.revision != revision && !void_only(&old, manifest) {
            note_route_revision(config, &old, manifest, revision);
        } else if config.revision != revision && config.scoring_revision == 0 {
            config.scoring_revision = config.revision;
        }
        freeze_history(config);
        config.revision = revision;
        config.published_manifest = manifest.to_string();
        normalize_received(config)?;
        Ok(())
    }

    pub fn receive(&self, config: &mut Config) -> Result<usize, String> {
        if config.competition_id == 0 || config.revision == 0 {
            return Err("Lähetä ensin kilpailutiedot sivustolle.".into());
        }
        if config.published_manifest.is_empty() {
            // Myös aiemman kehitysversion jo julkaisema kilpailu toimii ilman uudelleenluontia.
            let data = self.send(
                self.client
                    .get(self.url(&format!("competitions/{}", config.competition_id))?)
                    .header("X-AS-Key", &config.key),
            )?;
            if data["revision"].as_u64() != Some(config.revision) || !data["manifest"].is_object() {
                return Err(
                    "Palvelimen kilpailutiedot eivät vastaa tallennettua tietoversiota.".into(),
                );
            }
            config.published_manifest = data["manifest"].to_string();
        }
        let mut total = 0;
        for _ in 0..10 {
            let data = self.send(
                self.client
                    .get(self.url(&format!(
                        "competitions/{}/interpretations?after={}",
                        config.competition_id, config.cursor
                    ))?)
                    .header("X-AS-Key", &config.key)
                    .header("X-AS-Other-Penalty-Points", "1"),
            )?;
            if let Some(revision) = data["revision"].as_u64()
                && data["manifest"].is_object()
            {
                adopt_manifest(config, &data["manifest"], revision)?;
            }
            let page: ChangePage = serde_json::from_value(data)
                .map_err(|_| "Palvelimen tulkintavastausta ei tunnistettu.")?;
            let more = page.has_more;
            total += apply_page(config, page)?;
            if !more {
                break;
            }
        }
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receiving_config() -> Config {
        let mut config = Config::prepare("https://example.fi", "user", "Koekilpailu").unwrap();
        config.competition_id = 7;
        config.revision = 1;
        config.published_manifest=json!({"routes":[{"id":"r1","name":"M","sections":[
            {"id":"r1-j1","name":"Jakso 1","entries":[
                {"id":"a","label":"15","kind":"rt","voided":false,"manualTime":null},
                {"id":"b","label":"15","kind":"rt","voided":false,"manualTime":null},
                {"id":"at","label":"AT","kind":"at","voided":false,"manualTime":true},
                {"id":"jat","label":"JAT","kind":"jat","voided":false,"manualTime":false}]},
            {"id":"r1-j2","name":"Maali","entries":[{"id":"m","label":"M","kind":"maali","voided":false,"manualTime":false}]}]}],
            "pairs":[{"number":101,"driver":"Aino","navigator":"Eero","routeId":"r1","emit":true}]}).to_string();
        config
    }
    #[test]
    fn qr_join_link_contains_only_public_join_details_and_keeps_subdirectory() {
        let mut c = Config::prepare("https://example.fi/kerho/", "admin", "Kilpailu").unwrap();
        assert!(c.join_url().is_err());
        c.competition_id = 7;
        c.revision = 1;
        let text = c.join_url().unwrap();
        let url = Url::parse(&text).unwrap();
        assert_eq!(url.path(), "/kerho/as-tulkki/");
        assert!(url.query().is_none());
        assert!(!text.contains(&c.key));
        assert!(!text.contains("admin"));
        let mut params = Url::parse("https://example.fi/").unwrap();
        params.set_query(url.fragment());
        let fields = params.query_pairs().collect::<BTreeMap<_, _>>();
        assert_eq!(fields["join"], c.join_code);
        assert_eq!(fields["site"], "https://example.fi/kerho/");
        assert_eq!(fields.len(), 2);
    }

    fn paper_fixture() -> (Project, Config) {
        let mut p =
            Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
        p.kilpailu.ajanottotapa = 0;
        p.leimaluvut.clear();
        p.kilpailijat[0].emit = 0;
        let rows = ["08", "AT1", "AT2", "M"]
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i as u32 + 1, s.to_owned()))
            .collect::<Vec<_>>();
        crate::interpretation::import_route(
            &mut p,
            1,
            &crate::interpretation::preview_rows("Syotto_A", &rows),
        )
        .unwrap();
        let mut c = Config {
            competition_id: 7,
            revision: 1,
            ..Config::default()
        };
        c.published_manifest = manifest(&p).unwrap().to_string();
        (p, c)
    }
    #[test]
    fn voiding_one_duplicate_sign_recalculates_only_that_occurrence_and_keeps_original_receipt() {
        let (mut p, mut c) = paper_fixture();
        let mut extra = p.oikeat_rivit[0].merkinnat[0].clone();
        extra.tunniste = "r1-extra".into();
        p.oikeat_rivit[0].merkinnat.insert(1, extra);
        c.published_manifest = manifest(&p).unwrap().to_string();
        let mut event = paper_event(1, vec!["r1-e1"], "12:10:00");
        event.sections[0].missing_entry_ids.push("r1-extra".into());
        event.sections[0].missing_signs = 2;
        apply_page(&mut c, page(vec![event], 1, false)).unwrap();
        apply_paper(&mut p, &c);
        assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl, 2);
        let old: Value = serde_json::from_str(&c.published_manifest).unwrap();
        assert_eq!(void_sign(&mut p, 1, "r1-e1", &c).unwrap(), vec![7]);
        assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl, 1);
        assert_eq!(
            p.kilpailijat[0]
                .lisavirheet
                .tulkki_siirto
                .as_ref()
                .unwrap()
                .puuttuvat_rt[0]
                .kilvet,
            vec!["08"]
        );
        assert_eq!(c.received[0].section.missing_entry_ids.len(), 2);
        assert!(void_sign(&mut p, 1, "r1-e1", &c).is_err());
        assert!(void_sign(&mut p, 1, "r1-e2", &c).is_err());
        let new = manifest(&p).unwrap();
        assert!(void_only(&old, &new));
        assert!(!void_only(&new, &old));
        adopt_manifest(&mut c, &new, 2).unwrap();
        assert_eq!(c.cursor, 1);
        assert_eq!(c.received[0].section.missing_signs, 1);
        assert_eq!(c.received[0].section.missing_entry_ids, vec!["r1-extra"]);
        let mut bad = new.clone();
        bad["pairs"][0]["driver"] = json!("changed");
        assert!(!void_only(&new, &bad));
    }
    #[test]
    fn management_closes_reopens_and_retries_same_staged_join_code_without_changing_owner_key() {
        let (api, task) = server(vec![
            (200, json!({"id":7,"active":false})),
            (200, json!({"id":7,"active":true})),
            (500, json!({"message":"retry"})),
            (200, json!({"id":7,"active":true})),
        ]);
        let mut c = Config {
            competition_id: 7,
            revision: 1,
            user: "admin".into(),
            key: "a".repeat(48),
            join_code: "ABCDEF123456".into(),
            ..Config::default()
        };
        api.manage(&mut c, "pass", false).unwrap();
        assert!(c.closed);
        assert!(c.join_url().is_err());
        api.manage(&mut c, "pass", false).unwrap();
        assert!(!c.closed);
        c.stage_join_code().unwrap();
        let code = c.pending_join_code.clone();
        let key = c.key.clone();
        assert!(api.manage(&mut c, "pass", true).is_err());
        assert_eq!(c.join_code, "ABCDEF123456");
        c.stage_join_code().unwrap();
        assert_eq!(c.pending_join_code, code);
        api.manage(&mut c, "pass", true).unwrap();
        assert_eq!(c.join_code, code);
        assert_eq!(c.key, key);
        assert!(c.pending_join_code.is_empty());
        let requests = task.join().unwrap();
        assert!(requests[0].contains("\"active\":false"));
        assert!(requests[1].contains("\"active\":true"));
        assert!(requests[2].contains(&code));
        assert!(requests[3].contains(&code));
    }
    fn paper_event(id: u64, missing: Vec<&str>, at2: &str) -> InterpretationEvent {
        let mut times = json!({"r1-e2":"12:04:35","r1-e3":at2,"r1-e4":"12:11:00"});
        for id in &missing {
            times.as_object_mut().unwrap().remove(*id);
        }
        let signs = missing.iter().filter(|id| **id == "r1-e1").count();
        serde_json::from_value(json!({"eventId":id,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":missing,"times":times,"note":"Koe","otherErrors":1,"missingSigns":signs,"missingTimes":missing.len()-signs,"interpreted":true}]})).unwrap()
    }
    #[test]
    fn corrected_row_keeps_interpretation_clocks_history_and_recalculates_removed_missing_rt() {
        let (mut p, mut c) = paper_fixture();
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec!["r1-e1"], "12:10:00")], 1, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        p.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        let preview = crate::interpretation::preview_rows(
            "Korjattu",
            &[
                (1, "09".into()),
                (2, "AT1".into()),
                (3, "AT2".into()),
                (4, "M".into()),
            ],
        );
        correct_route(&mut p, &c, 1, &preview).unwrap();
        assert_eq!(p.oikeat_rivit[0].merkinnat[0].tunniste, "r1-e5");
        assert_eq!(p.oikeat_rivit[0].merkinnat[1].tunniste, "r1-e2");
        let d = &p.kilpailijat[0];
        assert_eq!(d.lisavirheet.puuttuvat_rt_kpl, 0);
        assert_eq!(d.lisavirheet.muut_virhepisteet, 600);
        assert!(!d.tallennettu_tulos.as_ref().unwrap().hyvaksytty);
        assert_eq!(
            d.tallennettu_tulos.as_ref().unwrap().rivit[2].clock,
            [12, 10, 0]
        );
        let updated = manifest(&p).unwrap();
        adopt_manifest(&mut c, &updated, 2).unwrap();
        assert!(c.received[0].section.interpreted);
        assert!(c.received[0].section.missing_entry_ids.is_empty());
        assert_eq!(
            c.history[0].original_sections[0]["missingEntryIds"],
            json!(["r1-e1"])
        );
        assert!(c.history[0].original_labels["r1-e1"].contains("08"));
        assert!(penalty_changes(&p, &c).unwrap().is_empty());
        let mut bad = preview.clone();
        bad.entries[1].label = "AT9".into();
        let before = p.clone();
        assert!(correct_route(&mut p, &c, 1, &bad).is_err());
        assert_eq!(p, before);
        let again = crate::interpretation::preview_rows(
            "Korjattu",
            &[
                (1, "10".into()),
                (2, "AT1".into()),
                (3, "AT2".into()),
                (4, "M".into()),
            ],
        );
        correct_route(&mut p, &c, 1, &again).unwrap();
        assert_eq!(p.oikeat_rivit[0].merkinnat[0].tunniste, "r1-e6");
    }
    #[test]
    fn row_correction_preserves_manual_overrides_on_another_route() {
        let (mut p, mut c) = paper_fixture();
        let mut route = p.radat[0].clone();
        route.numero = 2;
        p.radat.push(route);
        let mut class = p.luokat[0].clone();
        class.tunnus = "B".into();
        class.rata = 2;
        p.luokat.push(class);
        let mut driver = p.kilpailijat[0].clone();
        driver.numero = 8;
        driver.luokka = "B".into();
        p.kilpailijat.push(driver);
        crate::interpretation::import_route(
            &mut p,
            2,
            &crate::interpretation::preview_rows(
                "B",
                &[
                    (1, "08".into()),
                    (2, "AT1".into()),
                    (3, "AT2".into()),
                    (4, "M".into()),
                ],
            ),
        )
        .unwrap();
        c.published_manifest = manifest(&p).unwrap().to_string();
        let first = paper_event(1, vec!["r1-e1"], "12:10:00");
        let mut second = paper_event(2, vec!["r1-e1"], "12:10:00");
        second.pair_number = 8;
        second = serde_json::from_str(
            &serde_json::to_string(&second)
                .unwrap()
                .replace("r1-", "r2-"),
        )
        .unwrap();
        apply_page(&mut c, page(vec![first, second], 2, false)).unwrap();
        apply_paper(&mut p, &c);
        p.kilpailijat[1].lisavirheet.puuttuvat_rt_kpl = 9;
        p.kilpailijat[1].lisavirheet.puuttuvat_rt_pisteet = 5400;
        p.kilpailijat[1].lisavirheet.muut_virhepisteet = 77;
        let unchanged = p.kilpailijat[1].clone();
        correct_route(
            &mut p,
            &c,
            1,
            &crate::interpretation::preview_rows(
                "A",
                &[
                    (1, "09".into()),
                    (2, "AT1".into()),
                    (3, "AT2".into()),
                    (4, "M".into()),
                ],
            ),
        )
        .unwrap();
        assert_eq!(p.kilpailijat[1], unchanged);
        adopt_manifest(&mut c, &manifest(&p).unwrap(), 2).unwrap();
        assert!(penalty_changes(&p, &c).unwrap().is_empty());
        assert_eq!(p.kilpailijat[1], unchanged);
    }
    #[test]
    fn automatic_result_requires_complete_valid_normal_result_and_persists_print_key() {
        let (mut p, mut c) = paper_fixture();
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec!["r1-e1"], "12:10:00")], 1, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        assert!(automatic_result_key(&p, &c, 7).is_none());
        c.automatic_results = true;
        let key = automatic_result_key(&p, &c, 7).expect("valmis kokonainen tulos");
        c.automatic_prints.insert("7".into(), key.clone());
        p.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        assert!(
            automatic_result_key(&p, &c, 7).is_none(),
            "hyväksyntä ei tuplaa tulostetta"
        );
        let restored: Config = toml::from_str(&toml::to_string(&c).unwrap()).unwrap();
        assert!(automatic_result_key(&p, &restored, 7).is_none());
        apply_page(
            &mut c,
            page(vec![paper_event(2, vec![], "12:10:30")], 2, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        assert_ne!(automatic_result_key(&p, &c, 7).unwrap(), key);
        assert_eq!(c.history.len(), 2);
        assert_eq!(c.history[0].sections[0].missing_entry_ids, vec!["r1-e1"]);
        c.received.clear();
        assert!(automatic_result_key(&p, &c, 7).is_none());
    }
    #[test]
    fn class_metadata_can_be_added_without_changing_scoring_contract() {
        let (p, _) = paper_fixture();
        let new = manifest(&p).unwrap();
        let mut old = new.clone();
        for pair in old["pairs"].as_array_mut().unwrap() {
            pair.as_object_mut().unwrap().remove("className");
        }
        assert!(void_only(&old, &new));
        let mut changed = new.clone();
        changed["pairs"][0]["routeId"] = json!("other");
        assert!(!void_only(&new, &changed));
    }
    #[test]
    fn upgrading_existing_transfer_adds_progress_without_replacing_manual_points() {
        let (mut p, mut c) = paper_fixture();
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec!["r1-e1"], "12:10:00")], 1, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        let source = p.kilpailijat[0].lisavirheet.tulkki_siirto.as_mut().unwrap();
        source.tulkatut_jaksot.clear();
        source.jaksoja = 0;
        source.jaksojen_muut_pisteet.clear();
        p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 9;
        p.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet = 5400;
        let changes = penalty_changes(&p, &c).unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].1.puuttuvat_rt_kpl, 9);
        assert_eq!(changes[0].1.puuttuvat_rt_pisteet, 5400);
        let source = changes[0].1.tulkki_siirto.as_ref().unwrap();
        assert_eq!(source.tulkatut_jaksot, vec![1]);
        assert_eq!(source.jaksoja, 1);
    }
    #[test]
    fn legacy_transfer_stamp_and_manual_overrides_survive_upgrade_and_unrelated_void() {
        let (mut p, mut c) = paper_fixture();
        let mut unused = p.oikeat_rivit[0].merkinnat[0].clone();
        unused.tunniste = "r1-unused".into();
        unused.merkinta.label = "09".into();
        p.oikeat_rivit[0].merkinnat.insert(1, unused);
        c.published_manifest = manifest(&p).unwrap().to_string();
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec!["r1-e1"], "12:10:00")], 1, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        let source = p.kilpailijat[0].lisavirheet.tulkki_siirto.as_mut().unwrap();
        assert_eq!(
            source.tunniste,
            "67b8b0cedd746598d3baf15b9df9aa86749357f322cf74ef53476cace342dca0"
        );
        source.tapahtumatunniste = None;
        p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl = 9;
        p.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet = 5400;
        assert!(penalty_changes(&p, &c).unwrap().is_empty());
        assert!(void_sign(&mut p, 1, "r1-unused", &c).unwrap().is_empty());
        assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl, 9);
        let changed = manifest(&p).unwrap();
        adopt_manifest(&mut c, &changed, 2).unwrap();
        assert_eq!(c.scoring_revision, 1);
        assert!(penalty_changes(&p, &c).unwrap().is_empty());
        assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl, 9);
    }
    fn apply_paper(p: &mut Project, c: &Config) {
        for (number, penalties) in penalty_changes(p, c).unwrap() {
            let result = manual_result(p, c, number).unwrap();
            let driver = p
                .kilpailijat
                .iter_mut()
                .find(|d| d.numero == number)
                .unwrap();
            driver.lisavirheet = penalties;
            driver.tallennettu_tulos = Some(result);
        }
    }
    #[test]
    fn paper_phone_clocks_and_corrections_update_existing_result_without_duplicate_points() {
        let (mut p, mut c) = paper_fixture();
        apply_page(
            &mut c,
            page(
                vec![paper_event(1, vec!["r1-e1", "r1-e2"], "12:10:00")],
                1,
                false,
            ),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        let d = &p.kilpailijat[0];
        let r = d.tallennettu_tulos.as_ref().unwrap();
        assert!(r.ilman_emit);
        assert_eq!(r.rivit[2].clock, [12, 10, 0]);
        assert_eq!(d.lisavirheet.muut_virhepisteet, 2400);
        assert_eq!(d.lisavirheet.puuttuvat_rt_kpl, 1);
        assert!(r.can_approve());
        assert!(penalty_changes(&p, &c).unwrap().is_empty());
        let p2 = Project::from_toml(&p.to_toml().unwrap()).unwrap();
        assert!(penalty_changes(&p2, &c).unwrap().is_empty());
        apply_page(
            &mut c,
            page(vec![paper_event(2, vec![], "12:10:30")], 2, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        assert_eq!(p.kilpailijat[0].lisavirheet.muut_virhepisteet, 600);
        assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl, 0);
        assert_eq!(
            p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().rivit[2].clock,
            [12, 10, 30]
        );
        p.kilpailijat[0]
            .tallennettu_tulos
            .as_mut()
            .unwrap()
            .hyvaksytty = true;
        apply_page(
            &mut c,
            page(vec![paper_event(3, vec![], "12:10:30")], 3, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        assert!(
            p.kilpailijat[0]
                .tallennettu_tulos
                .as_ref()
                .unwrap()
                .hyvaksytty
        );
        apply_page(
            &mut c,
            page(vec![paper_event(4, vec![], "12:12:00")], 4, false),
        )
        .unwrap();
        let before = p.clone();
        assert!(penalty_changes(&p, &c).is_err());
        assert_eq!(p, before);
    }
    #[test]
    fn paper_phone_requires_matching_clock_order_and_exports_all_clocks_as_manual() {
        let (mut p, mut c) = paper_fixture();
        let data = manifest(&p).unwrap();
        assert_eq!(data["pairs"][0]["emit"], false);
        assert_eq!(
            data["routes"][0]["sections"][0]["entries"][3]["manualTime"],
            true
        );
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec![], "12:10:00")], 1, false),
        )
        .unwrap();
        p.radat[0].asemat[2].tyyppi = "JAT".into();
        assert!(penalty_changes(&p, &c).is_err());
    }
    #[test]
    fn missing_sign_metadata_migration_preserves_manual_scoring_changes() {
        let (mut p, mut c) = paper_fixture();
        apply_page(
            &mut c,
            page(vec![paper_event(1, vec!["r1-e1"], "12:10:00")], 1, false),
        )
        .unwrap();
        apply_paper(&mut p, &c);
        assert_eq!(
            p.kilpailijat[0]
                .lisavirheet
                .tulkki_siirto
                .as_ref()
                .unwrap()
                .puuttuvat_rt[0]
                .kilvet,
            vec!["08"]
        );
        let d = &mut p.kilpailijat[0];
        d.lisavirheet
            .tulkki_siirto
            .as_mut()
            .unwrap()
            .puuttuvat_rt
            .clear();
        d.lisavirheet.puuttuvat_rt_kpl = 7;
        d.lisavirheet.puuttuvat_rt_pisteet = 4200;
        d.lisavirheet.muut_virhepisteet = 99;
        let planned = penalty_changes(&p, &c).unwrap();
        assert_eq!(planned.len(), 1);
        let updated = &planned[0].1;
        assert_eq!(updated.puuttuvat_rt_kpl, 7);
        assert_eq!(updated.puuttuvat_rt_pisteet, 4200);
        assert_eq!(updated.muut_virhepisteet, 99);
        assert_eq!(
            updated.tulkki_siirto.as_ref().unwrap().puuttuvat_rt[0].kilvet,
            vec!["08"]
        );
    }
    #[test]
    fn partial_paper_interpretation_keeps_future_sections_pending() {
        let (mut p, mut c) = paper_fixture();
        let old = p.radat[0].asemat.clone();
        let mut jat = old[2].clone();
        jat.tyyppi = "JAT".into();
        jat.nimi = "JAT laskenta".into();
        let mut ula = jat.clone();
        ula.tyyppi = "ULA".into();
        ula.tavoiteaika_sekuntia = 0;
        p.radat[0].asemat = vec![
            old[0].clone(),
            old[1].clone(),
            jat,
            ula,
            old[2].clone(),
            old[3].clone(),
        ];
        p.oikeat_rivit.clear();
        let rows = ["AT1", "JAT3", "ULA3", "AT2", "M"]
            .into_iter()
            .enumerate()
            .map(|(i, s)| (i as u32 + 1, s.to_owned()))
            .collect::<Vec<_>>();
        crate::interpretation::import_route(
            &mut p,
            1,
            &crate::interpretation::preview_rows("Syotto_A", &rows),
        )
        .unwrap();
        c.published_manifest = manifest(&p).unwrap().to_string();
        let event=serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e1":"12:05:00","r1-e2":"12:10:00","r1-e3":"12:12:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
        apply_page(&mut c, page(vec![event], 1, false)).unwrap();
        apply_paper(&mut p, &c);
        let r = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert_eq!(r.pending_manual_clocks(), vec![5, 6]);
        assert_eq!(r.tila, "ILM");
        assert_eq!(p.kilpailijat[0].lisavirheet.muut_virhepisteet, 0);
        let event=serde_json::from_value(json!({"eventId":2,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j2","missingEntryIds":[],"times":{"r1-e4":"12:22:00","r1-e5":"12:22:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
        apply_page(&mut c, page(vec![event], 2, false)).unwrap();
        apply_paper(&mut p, &c);
        let r = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert!(r.can_approve());
        assert_eq!(r.rivit[1].clock, [12, 5, 0]);
        assert_eq!(r.kesto_sekuntia, 1320);
    }
    #[test]
    fn adding_ula_to_published_row_preserves_clocks_and_requests_only_the_new_time() {
        let (mut p, mut c) = paper_fixture();
        let old = p.radat[0].asemat.clone();
        let mut jat = old[2].clone();
        jat.tyyppi = "JAT".into();
        jat.nimi = "JAT3".into();
        let mut ula = jat.clone();
        ula.tyyppi = "ULA".into();
        ula.nimi = "ULA3".into();
        ula.tavoiteaika_sekuntia = 0;
        p.radat[0].asemat = vec![
            old[0].clone(),
            old[1].clone(),
            jat,
            ula,
            old[2].clone(),
            old[3].clone(),
        ];
        p.oikeat_rivit.clear();
        let preview = crate::interpretation::preview_rows(
            "Koe",
            &[
                (1, "AT1".into()),
                (2, "JAT3".into()),
                (3, "AT2".into()),
                (4, "M".into()),
            ],
        );
        crate::interpretation::import_route(&mut p, 1, &preview).unwrap();
        c.published_manifest = manifest(&p).unwrap().to_string();
        let event=serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-12T12:00:00Z","sections":[
            {"id":"r1-j1","missingEntryIds":[],"times":{"r1-e1":"12:05:00","r1-e2":"12:10:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true},
            {"id":"r1-j2","missingEntryIds":[],"times":{"r1-e3":"12:22:00","r1-e4":"12:22:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
        apply_page(&mut c, page(vec![event], 1, false)).unwrap();
        apply_paper(&mut p, &c);
        let corrected = crate::interpretation::preview_rows(
            "Koe",
            &[
                (1, "AT1".into()),
                (2, "JAT3".into()),
                (3, "ULA3".into()),
                (4, "AT2".into()),
                (5, "M".into()),
            ],
        );
        correct_route(&mut p, &c, 1, &corrected).unwrap();
        let revised = manifest(&p).unwrap();
        adopt_manifest(&mut c, &revised, 2).unwrap();
        assert!(c.received.iter().all(|r| r.section.interpreted));
        assert_eq!(c.received[0].section.pending_time_ids, vec!["r1-e5"]);
        assert!(c.received[0].section.missing_entry_ids.is_empty());
        let result = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert_eq!(result.rivit[1].clock, [12, 5, 0]);
        assert_eq!(result.rivit[4].clock, [12, 22, 0]);
        assert_eq!(result.pending_manual_clocks(), vec![4]);
        assert!(!result.can_approve());
        assert!(result.puuttuvat_aikamerkinnat.is_empty());
        let event=serde_json::from_value(json!({"eventId":2,"pairNumber":7,"revision":2,"reviewer":"Koe","savedAt":"2026-11-12T12:05:00Z","sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e1":"12:05:00","r1-e2":"12:10:00","r1-e5":"12:12:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
        apply_page(&mut c, page(vec![event], 2, false)).unwrap();
        apply_paper(&mut p, &c);
        let result = p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();
        assert!(result.can_approve());
        assert_eq!(result.rivit[3].clock, [12, 12, 0]);
        assert_eq!(result.rivit[4].clock, [12, 22, 0]);
        let mut emit_manifest: Value = serde_json::from_str(&c.published_manifest).unwrap();
        emit_manifest["pairs"][0]["emit"] = json!(true);
        emit_manifest["routes"][0]["sections"][0]["entries"][2]["manualTime"] = json!(false);
        c.published_manifest = emit_manifest.to_string();
        c.received[0].section.times.remove("r1-e5");
        normalize_received(&mut c).unwrap();
        assert!(c.received[0].section.pending_time_ids.is_empty());
    }
    fn event(id: u64, section: &str) -> InterpretationEvent {
        serde_json::from_value(json!({"eventId":id,"pairNumber":101,"revision":1,"reviewer":"Tarkistaja",
            "savedAt":"2026-11-10T18:00:00Z","sections":[if section=="r1-j1"{json!({"id":section,"missingEntryIds":["b","jat"],"times":{"at":"12:34:03"},"note":"Toinen 15 puuttuu","otherErrors":2,"missingSigns":1,"missingTimes":1,"interpreted":true})}
            else {json!({"id":section,"missingEntryIds":[],"times":{},"note":"Maali tarkistettu","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true})}]})).unwrap()
    }
    fn page(events: Vec<InterpretationEvent>, cursor: u64, more: bool) -> ChangePage {
        ChangePage {
            events,
            next_cursor: cursor,
            has_more: more,
        }
    }

    #[test]
    fn reception_replaces_only_corrected_section_without_double_counting() {
        let mut config = receiving_config();
        apply_page(
            &mut config,
            page(vec![event(10, "r1-j1"), event(11, "r1-j2")], 11, false),
        )
        .unwrap();
        assert_eq!(config.received.len(), 2);
        assert_eq!(
            config.received[0].section.missing_entry_ids,
            vec!["b", "jat"]
        );
        let mut correction = event(12, "r1-j1");
        correction.sections[0].missing_entry_ids = vec!["a".into()];
        correction.sections[0].missing_times = 0;
        correction.reviewer = "Toinen tarkistaja".into();
        apply_page(&mut config, page(vec![correction], 12, false)).unwrap();
        assert_eq!(config.received.len(), 2);
        assert_eq!(config.cursor, 12);
        assert_eq!(config.received[0].section.missing_entry_ids, vec!["a"]);
        assert_eq!(config.received[0].reviewer, "Toinen tarkistaja");
        assert_eq!(config.received[1].event_id, 11);
        assert_eq!(
            config
                .received
                .iter()
                .map(|r| r.section.missing_signs)
                .sum::<u32>(),
            1
        );
        let encoded = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&encoded).unwrap(), config);
        apply_page(&mut config, page(vec![], 12, false)).unwrap();
        assert_eq!(config.received.len(), 2);
    }

    #[test]
    fn penalties_use_existing_fields_and_replace_only_received_contribution() {
        let mut project = Project::new("Laskentakoe").unwrap();
        project.kilpailijat.push(serde_json::from_value(json!({"numero":101,"luokka":"H","kuljettaja":"Aino","kartanlukija":"Eero","emit":123})).unwrap());
        project.kilpailijat[0].lisavirheet.muut_virhepisteet = 250;
        project.kilpailijat[0].lisavirheet.ylimaaraiset_rt_kpl = 2;
        project.kilpailijat[0].lisavirheet.ylimaaraiset_rt_pisteet = 1200;
        let mut config = receiving_config();
        apply_page(&mut config, page(vec![event(1, "r1-j1")], 1, false)).unwrap();
        let changes = penalty_changes(&project, &config).unwrap();
        let p = &changes[0].1;
        assert_eq!(
            (
                p.puuttuvat_rt_kpl,
                p.puuttuvat_rt_pisteet,
                p.muut_virhepisteet
            ),
            (1, 600, 1450)
        );
        assert_eq!(
            (p.ylimaaraiset_rt_kpl, p.ylimaaraiset_rt_pisteet),
            (2, 1200)
        );
        project.kilpailijat[0].lisavirheet = p.clone();
        let mut project: Project =
            serde_json::from_str(&serde_json::to_string(&project).unwrap()).unwrap();
        assert!(penalty_changes(&project, &config).unwrap().is_empty());
        // Manuaalinen lisäys säilyy myös seuraavan puhelinkorjauksen jälkeen.
        project.kilpailijat[0].lisavirheet.muut_virhepisteet += 50;
        assert!(penalty_changes(&project, &config).unwrap().is_empty());
        let mut corrected = event(2, "r1-j1");
        corrected.sections[0].missing_entry_ids = vec!["a".into(), "b".into()];
        corrected.sections[0].missing_signs = 2;
        corrected.sections[0].missing_times = 0;
        corrected.sections[0].other_errors = 1;
        apply_page(&mut config, page(vec![corrected], 2, false)).unwrap();
        let p = penalty_changes(&project, &config).unwrap().remove(0).1;
        assert_eq!(
            (
                p.puuttuvat_rt_kpl,
                p.puuttuvat_rt_pisteet,
                p.muut_virhepisteet
            ),
            (2, 1200, 900)
        );
        project.kilpailijat[0].lisavirheet = p;
        let mut cleared = event(3, "r1-j1");
        cleared.sections[0].missing_entry_ids.clear();
        cleared.sections[0].missing_signs = 0;
        cleared.sections[0].missing_times = 0;
        cleared.sections[0].other_errors = 0;
        apply_page(&mut config, page(vec![cleared], 3, false)).unwrap();
        let p = penalty_changes(&project, &config).unwrap().remove(0).1;
        assert_eq!(
            (
                p.puuttuvat_rt_kpl,
                p.puuttuvat_rt_pisteet,
                p.muut_virhepisteet
            ),
            (0, 0, 300)
        );
        config.received[0].section.missing_signs = 99;
        assert!(penalty_changes(&project, &config).is_err());
    }

    #[test]
    fn invalid_last_event_keeps_entire_page_and_cursor_unchanged() {
        let original = receiving_config();
        let mut invalid = event(2, "r1-j1");
        invalid.sections[0].missing_entry_ids.push("unknown".into());
        let mut config = original.clone();
        assert!(
            apply_page(
                &mut config,
                page(vec![event(1, "r1-j1"), invalid], 2, false)
            )
            .is_err()
        );
        assert_eq!(config, original);
        for invalid_time in ["24:00:00", "12:60:00", "1:02:03", "12:00:99"] {
            let mut invalid = event(1, "r1-j1");
            invalid.sections[0]
                .times
                .insert("at".into(), invalid_time.into());
            assert!(apply_page(&mut config, page(vec![invalid], 1, false)).is_err());
        }
        let mut invalid = event(1, "r1-j1");
        invalid.revision = 2;
        assert!(apply_page(&mut config, page(vec![invalid], 1, false)).is_err());
        let mut invalid = event(1, "r1-j1");
        invalid.sections[0].missing_signs = 2;
        assert!(apply_page(&mut config, page(vec![invalid], 1, false)).is_err());
        let mut invalid = event(1, "r1-j1");
        invalid.sections[0].missing_entry_ids = vec!["m".into()];
        assert!(apply_page(&mut config, page(vec![invalid], 1, false)).is_err());
        assert_eq!(config, original);
    }

    #[test]
    fn invalid_order_and_cursor_do_not_hide_unprocessed_events() {
        let mut config = receiving_config();
        assert!(
            apply_page(
                &mut config,
                page(vec![event(2, "r1-j1"), event(1, "r1-j2")], 2, false)
            )
            .is_err()
        );
        assert!(apply_page(&mut config, page(vec![event(1, "r1-j1")], 99, false)).is_err());
        assert!(apply_page(&mut config, page(vec![], 0, true)).is_err());
        assert_eq!(config.cursor, 0);
        assert!(config.received.is_empty());
    }

    #[test]
    fn paging_resumes_after_acknowledged_page_and_keeps_legacy_connections() {
        let mut config = receiving_config();
        let manifest: Value = serde_json::from_str(&config.published_manifest).unwrap();
        config.published_manifest.clear();
        let (api, task) = server(vec![
            (200, json!({"revision":1,"manifest":manifest})),
            (
                200,
                serde_json::to_value(page(vec![event(10, "r1-j1")], 10, true)).unwrap(),
            ),
            (
                200,
                serde_json::to_value(page(vec![event(11, "r1-j2")], 11, false)).unwrap(),
            ),
        ]);
        assert_eq!(api.receive(&mut config).unwrap(), 2);
        assert_eq!(config.cursor, 11);
        assert_eq!(config.received.len(), 2);
        let requests = task.join().unwrap();
        assert!(requests[1].contains("after=0"));
        assert!(requests[2].contains("after=10"));
        assert!(requests.iter().all(|r| r.contains(&config.key)));
        assert!(
            requests
                .iter()
                .all(|r| !r.to_lowercase().contains("authorization:"))
        );
    }

    #[test]
    fn replay_from_reset_cursor_does_not_replace_newer_received_correction() {
        let mut config = receiving_config();
        apply_page(&mut config, page(vec![event(20, "r1-j1")], 20, false)).unwrap();
        config.cursor = 0;
        let mut older = event(10, "r1-j1");
        older.sections[0].note = "Vanha".into();
        apply_page(&mut config, page(vec![older], 10, false)).unwrap();
        assert_eq!(config.received[0].event_id, 20);
        assert_eq!(config.received[0].section.note, "Toinen 15 puuttuu");
    }

    #[test]
    fn wordpress_empty_time_array_is_accepted_but_nonempty_arrays_are_rejected() {
        let mut data = serde_json::to_value(event(1, "r1-j2")).unwrap();
        data["sections"][0]["times"] = json!([]);
        let decoded: InterpretationEvent = serde_json::from_value(data.clone()).unwrap();
        assert!(decoded.sections[0].times.is_empty());
        let mut config = receiving_config();
        apply_page(&mut config, page(vec![decoded], 1, false)).unwrap();
        data["sections"][0]["times"] = json!(["12:00:00"]);
        assert!(serde_json::from_value::<InterpretationEvent>(data).is_err());
    }

    #[test]
    fn failed_refresh_keeps_received_data_and_cursor_for_retry() {
        let mut config = receiving_config();
        apply_page(&mut config, page(vec![event(10, "r1-j1")], 10, false)).unwrap();
        let original = config.clone();
        let (api, task) = server(vec![(503, json!({"message":"Tilapäinen katko"}))]);
        assert!(api.receive(&mut config).is_err());
        assert_eq!(config, original);
        let requests = task.join().unwrap();
        assert!(requests[0].contains("after=10"));
        let (api, task) = server(vec![(
            200,
            serde_json::to_value(page(vec![event(11, "r1-j2")], 11, false)).unwrap(),
        )]);
        api.receive(&mut config).unwrap();
        task.join().unwrap();
        assert_eq!(config.cursor, 11);
        assert_eq!(config.received.len(), 2);
    }
    use std::{
        io::{BufRead, BufReader, Write},
        net::TcpListener,
        thread,
    };

    fn server(responses: Vec<(u16, Value)>) -> (Api, thread::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let task = thread::spawn(move || {
            let mut requests = Vec::new();
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut headers = String::new();
                let mut length = 0;
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    if line == "\r\n" {
                        break;
                    }
                    if let Some(value) = line.to_lowercase().strip_prefix("content-length:") {
                        length = value.trim().parse::<usize>().unwrap();
                    }
                    headers.push_str(&line);
                }
                let mut bytes = vec![0; length];
                reader.read_exact(&mut bytes).unwrap();
                requests.push(headers + &String::from_utf8(bytes).unwrap());
                let body = body.to_string();
                write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
            }
            requests
        });
        let mut api = Api::new("https://example.fi").unwrap();
        // Vain testipalvelin käyttää HTTP:tä. Julkinen rakentaja hylkää sen.
        api.base = Url::parse(&format!("http://{address}/wp-json/as-tulkki/v1/")).unwrap();
        (api, task)
    }

    #[test]
    fn export_keeps_occurrences_sections_time_choices_and_original_project() {
        let mut project = Project::new("Tulkki-koe").unwrap();
        let rows = ["15", "15", "AT", "JAT", "ULA", "M"]
            .into_iter()
            .enumerate()
            .map(|(i, label)| (i as u32 + 1, label.to_string()))
            .collect::<Vec<_>>();
        let preview = crate::interpretation::preview_rows("Syotto_H", &rows);
        crate::interpretation::import_route(&mut project, 1, &preview).unwrap();
        project.oikeat_rivit[0].merkinnat[2].kasiaika = Some(true);
        project.oikeat_rivit[0].merkinnat[3].kasiaika = Some(false);
        project.kilpailijat.push(serde_json::from_value(json!({"numero":101,"luokka":"H","kuljettaja":"Aino","kartanlukija":"Eero","emit":123})).unwrap());
        let before = project.clone();
        let data = manifest(&project).unwrap();
        let sections = data["routes"][0]["sections"].as_array().unwrap();
        assert_eq!(sections.len(), 2);
        let entries = &sections[0]["entries"];
        assert_eq!(entries[0]["label"], entries[1]["label"]);
        assert_ne!(entries[0]["id"], entries[1]["id"]);
        assert_eq!(entries[2]["manualTime"], true);
        assert_eq!(entries[3]["manualTime"], false);
        assert_eq!(data["pairs"][0]["routeId"], "r1");
        assert_eq!(project, before);
        project.oikeat_rivit.clear();
        assert!(
            manifest(&project)
                .unwrap_err()
                .contains("puuttuu oikea rivi")
        );
    }

    #[test]
    fn creation_and_publish_use_separate_authentication_and_acknowledgements() {
        let (api, task) = server(vec![
            (200, json!({"id":7,"revision":0})),
            (200, json!({"revision":2})),
            (200, json!({"revision":3})),
        ]);
        let mut config = Config::prepare("https://example.fi", "user", "Koekilpailu").unwrap();
        let original = config.clone();
        api.create(&mut config, "pass").unwrap();
        assert_eq!(config.competition_id, 7);
        api.publish(&mut config, &json!({"routes":[],"pairs":[]}))
            .unwrap();
        assert_eq!(config.revision, 3);
        let requests = task.join().unwrap();
        assert!(
            requests[0]
                .to_lowercase()
                .contains("authorization: basic dxnlcjpwyxnz")
        );
        assert!(requests[0].contains(&original.request_id));
        assert!(requests[0].contains(&original.join_code));
        assert!(requests[1].contains(&original.key));
        assert!(!requests[1].to_lowercase().contains("authorization:"));
        assert!(requests[2].contains("\"baseRevision\":2"));
    }

    #[test]
    fn failed_publish_preserves_created_identity_for_retry() {
        let (api, task) = server(vec![
            (200, json!({"id":7,"revision":0})),
            (200, json!({"revision":1})),
            (409, json!({"message":"Ristiriita"})),
        ]);
        let mut config = Config::prepare("https://example.fi", "user", "Koekilpailu").unwrap();
        let key = config.key.clone();
        api.create(&mut config, "pass").unwrap();
        assert!(api.publish(&mut config, &json!({})).is_err());
        assert_eq!(config.competition_id, 7);
        assert_eq!(config.key, key);
        assert_eq!(config.revision, 0);
        task.join().unwrap();
    }
    #[test]
    fn site_paths_and_credentials_are_checked() {
        assert_eq!(
            api_url("https://example.fi/kerho/").unwrap().as_str(),
            "https://example.fi/kerho/wp-json/as-tulkki/v1/"
        );
        assert_eq!(
            api_url("https://example.fi/wp-json/as-tulkki/v1")
                .unwrap()
                .as_str(),
            "https://example.fi/wp-json/as-tulkki/v1/"
        );
        for url in [
            "http://example.fi",
            "https://user:secret@example.fi",
            "https://example.fi/?key=x",
            "https://example.fi/#x",
        ] {
            assert!(api_url(url).is_err());
        }
    }
    #[test]
    fn credentials_survive_retry_and_debug_does_not_disclose_them() {
        let config = Config::prepare("https://example.fi", "admin", "Kisa").unwrap();
        assert_eq!(config.key.len(), 48);
        assert_eq!(config.join_code.len(), 12);
        let encoded = toml::to_string(&config).unwrap();
        assert_eq!(toml::from_str::<Config>(&encoded).unwrap(), config);
        let debug = format!("{config:?}");
        assert!(!debug.contains(&config.key));
        assert!(!debug.contains(&config.join_code));
        assert_ne!(
            Config::prepare("https://example.fi", "admin", "Kisa")
                .unwrap()
                .key,
            config.key
        );
    }
}
