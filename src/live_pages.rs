//! Compact class pages shared by local preview and authenticated live publication.
use crate::{CoreError, application, native::Project, ranking::TiePolicy};
use serde::{Deserialize, Serialize};
use std::{
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};

static LOCAL_WRITE: Mutex<()> = Mutex::new(());
static NEXT_TEMP: AtomicU64 = AtomicU64::new(1);

struct TemporaryPage(PathBuf);
impl Drop for TemporaryPage {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

fn invalid(s: &str) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
fn url(s: &str) -> String {
    s.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    pub base: PathBuf,
    pub classes: Vec<String>,
}

fn archives(project: &Project) -> Result<Vec<crate::legacy::Archive>, CoreError> {
    let mut result = vec![application::calculated_project(project)?];
    if project.beginners_cup.is_some() {
        result.push(crate::beginners::results(project)?);
    }
    Ok(result)
}
pub fn class_choices(project: &Project) -> Result<Vec<String>, CoreError> {
    let mut choices = Vec::new();
    for archive in archives(project)? {
        choices.extend(
            application::rank_stored(&archive, TiePolicy::PairwiseControlComparison)?
                .groups
                .into_iter()
                .map(|g| g.code),
        );
    }
    Ok(choices)
}
pub fn pages(project: &Project, settings: &Settings) -> Result<Vec<(PathBuf, String)>, CoreError> {
    pages_with_interval(project, settings, 60)
}
pub fn pages_with_interval(
    project: &Project,
    settings: &Settings,
    refresh_seconds: u32,
) -> Result<Vec<(PathBuf, String)>, CoreError> {
    if refresh_seconds == 0 {
        return Err(invalid("Päivitysvälin pitää olla positiivinen"));
    }
    let filename = settings
        .base
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| invalid("Anna HTML-tiedoston nimi"))?;
    if settings.base.extension().and_then(|s| s.to_str()) != Some("html") {
        return Err(invalid("Anna tiedostolle .html-pääte"));
    }
    let stem = filename.strip_suffix(".html").unwrap();
    if stem.is_empty() || filename.chars().any(char::is_control) {
        return Err(invalid("Virheellinen HTML-tiedoston nimi"));
    }
    let mut selected = Vec::new();
    for (archive_index, archive) in archives(project)?.into_iter().enumerate() {
        let mut states = std::collections::BTreeMap::new();
        let mut snapshot = archive.clone();
        let mut provisional = false;
        for c in &mut snapshot.competitors {
            let state = live_state(project, &archive, c, archive_index == 0)?;
            if let Some(points) = state.points {
                provisional |= !archive.has_approved_result(c.number);
                c.total_penalties = points;
                c.status = "TRK".into();
            } else if !state.terminal {
                c.status = "ILM".into();
            }
            states.insert(c.number, state);
        }
        // Vain yleisön välitulossnapshot: virallisia hyväksyntöjä tai pisteitä ei muuteta.
        snapshot.result_publication = None;
        let ranked = application::rank_stored(&snapshot, TiePolicy::PairwiseControlComparison)?;
        let shared = if provisional {
            Some(application::rank_stored(
                &snapshot,
                TiePolicy::KeepSharedAndFlag,
            )?)
        } else {
            None
        };
        for mut group in ranked.groups {
            let group_provisional = group.entries.iter().any(|e| {
                states
                    .get(&(e.number as u32))
                    .is_some_and(|s| s.points.is_some())
                    && !archive.has_approved_result(e.number as u32)
            });
            if group_provisional
                && let Some(other) = shared
                    .as_ref()
                    .and_then(|r| r.groups.iter().find(|g| g.code == group.code))
            {
                group = other.clone();
            }
            if !settings.classes.contains(&group.code) {
                continue;
            }
            let mut rows = String::new();
            let mut entries = group.entries.iter().collect::<Vec<_>>();
            entries.sort_by_key(|e| {
                (
                    e.status != crate::ranking::Status::Classified,
                    e.primary_position,
                    e.number,
                )
            });
            for entry in entries {
                let Some(driver) = archive
                    .competitors
                    .iter()
                    .find(|c| i64::from(c.number) == entry.number)
                else {
                    continue;
                };
                let state = &states[&driver.number];
                let place = if state.points.is_some() && !state.terminal {
                    entry.primary_position.to_string()
                } else {
                    String::new()
                };
                let total = state.points.map(|p| p.to_string()).unwrap_or_default();
                let (rt, time, other) = state
                    .breakdown
                    .map(|p| (p.rt.to_string(), p.time.to_string(), p.other.to_string()))
                    .unwrap_or_default();
                rows.push_str(&format!("<tr><td>{place}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">{rt}</td><td class=\"numeric\">{time}</td><td class=\"numeric\">{other}</td><td class=\"numeric total\">{total}</td></tr>",driver.number,escape(&driver.driver),escape(&driver.navigator),escape(&state.driving)));
            }
            selected.push((group.code, rows, group_provisional));
        }
    }
    if selected.is_empty() {
        return Err(invalid("Valitse vähintään yksi julkaistava luokka"));
    }
    let names: Vec<_> = (0..selected.len())
        .map(|i| {
            if i == 0 {
                filename.to_owned()
            } else {
                format!("{stem}{i}.html")
            }
        })
        .collect();
    let updated = chrono::Local::now().format("%d.%m.%Y %H:%M:%S").to_string();
    Ok(selected.iter().enumerate().map(|(i, (class, rows, provisional))| {
        let next = url(&names[(i + 1) % names.len()]);
        let html = format!(r#"<!doctype html><html lang="fi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="refresh" content="{refresh_seconds};url={next}"><meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"><title>{competition} · {class}</title><style>
:root{{color-scheme:light;font:16px/1.2 system-ui,sans-serif;color:#152433;background:#f2f5f8}}body{{margin:0}}header,main{{max-width:1400px;margin:auto;padding:.5rem 1rem}}h1{{font-size:1.6rem;margin:.15rem 0}}h2{{font-size:1.25rem;margin:0 0 .4rem}}p{{margin:.2rem 0}}.brand{{font-weight:750;letter-spacing:.06em;color:#245779;font-size:.8rem}}.subtitle{{color:#4d6375;font-size:.85rem}}section{{background:#fff;border:1px solid #d7e0e8;border-radius:10px;padding:.6rem}}.table-wrap{{overflow-x:auto}}table{{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}}th,td{{text-align:left;padding:.18rem .5rem;border-bottom:1px solid #dce4eb}}th{{background:#edf2f6;font-size:.9rem}}.numeric{{text-align:right;white-space:nowrap}}tbody tr:nth-child(even){{background:#f8fafc}} .table-hint{{display:none}}@media(max-width:600px){{:root{{font-size:13px}}header,main{{padding:.4rem}}th,td{{padding:.18rem .25rem}}.table-hint{{display:block;font-size:.85rem;color:#4d6375}}}}
</style></head><body><header><p class="brand">AS2026</p><h1>{competition}</h1><p class="subtitle">Epäviralliset tulokset{interim_notice} · Päivitetty {updated} · Sivu {page}/{count}</p></header><main><section><h2>Luokka: {class}</h2>{progress_notice}<p class="table-hint">Taulukkoa voi vierittää sivusuunnassa.</p><div class="table-wrap"><table><thead><tr><th>Sij.</th><th>Nro</th><th>Kuljettaja</th><th>Kartanlukija</th><th>Tilanne</th><th class="numeric">RT</th><th class="numeric">Aika</th><th class="numeric">Muut</th><th class="numeric">Yhteensä</th></tr></thead><tbody>{rows}</tbody></table></div></section></main></body></html>"#,
            progress_notice=if *provisional{"<p class=\"subtitle\">Välituloksessa ovat mukana vain tulkatut jaksot ja käytettävissä olevat ajat. Parit voivat olla eri vaiheissa kilpailua; sijoitus voi muuttua. Maalissa ei vielä tarkoita hyväksyttyä lopputulosta.</p>"}else{""}, interim_notice=if *provisional{" · Välisijoitukset kertyneiden pisteiden mukaan"}else{""}, competition=escape(&project.kilpailu.nimi), class=escape(crate::printing::class_label(class)), page=i+1, count=selected.len());
        (settings.base.with_file_name(&names[i]), html)
    }).collect())
}

pub fn write_pages(pages: &[(PathBuf, String)]) -> Result<(), CoreError> {
    write_pages_with_cancel(pages, None)
}

pub(crate) fn write_pages_with_cancel(
    pages: &[(PathBuf, String)],
    cancelled: Option<&AtomicBool>,
) -> Result<(), CoreError> {
    // Koko sivusarja kirjoitetaan peräkkäin. Pysäytetty vanha julkaisija ei
    // voi palata korvaamaan uuden julkaisijan sivuja lukon vapautumisen jälkeen.
    let _serial = LOCAL_WRITE
        .lock()
        .map_err(|_| invalid("Edellinen livetulosten tallennus keskeytyi odottamattomasti"))?;
    let check_cancelled = || {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            Err(invalid("Julkaisu pysäytettiin."))
        } else {
            Ok(())
        }
    };
    check_cancelled()?;
    for (path, html) in pages.iter().rev() {
        check_cancelled()?;
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|_| invalid("Livetulosten kansiota ei voitu luoda"))?;
        }
        let mut file_and_temp = None;
        for _ in 0..100 {
            let mut name = path.as_os_str().to_owned();
            name.push(format!(
                ".as2026-{}-{}.tmp",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            let temporary = PathBuf::from(name);
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temporary)
            {
                Ok(file) => {
                    file_and_temp = Some((file, TemporaryPage(temporary)));
                    break;
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return Err(invalid("Livetulossivun väliaikaistiedostoa ei voitu luoda")),
            }
        }
        let (mut file, temporary) = file_and_temp
            .ok_or_else(|| invalid("Livetulossivun väliaikaistiedostoa ei voitu luoda"))?;
        let written = file.write_all(html.as_bytes());
        drop(file);
        written.map_err(|_| invalid("Livetulossivun tallennus epäonnistui"))?;
        check_cancelled()?;
        crate::storage::file_operation(|| {
            // Windowsin jakovirheen uusintayrityskin kunnioittaa pysäytystä.
            if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "Julkaisu pysäytettiin.",
                ));
            }
            std::fs::rename(&temporary.0, path)
        })
        .map_err(|_| invalid("Livetulossivun tallennus epäonnistui"))?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct PointBreakdown {
    rt: i64,
    time: i64,
    other: i64,
}
fn scored_state(parts: PointBreakdown, driving: String) -> Result<LiveState, CoreError> {
    let points = parts
        .rt
        .checked_add(parts.time)
        .and_then(|n| n.checked_add(parts.other))
        .ok_or(CoreError::ArithmeticOverflow("Livetuloksen pisteet"))?;
    Ok(LiveState {
        points: Some(points),
        breakdown: Some(parts),
        driving,
        terminal: false,
    })
}
fn full_state(c: &crate::legacy::Competitor, driving: String) -> Result<LiveState, CoreError> {
    let field = |index: usize| {
        c.header_fields
            .get(index)
            .and_then(|s| s.parse::<i64>().ok())
            .ok_or_else(|| invalid("Tuloksen piste-erittely puuttuu"))
    };
    let add = |a: i64, b: i64| {
        a.checked_add(b)
            .ok_or(CoreError::ArithmeticOverflow("Piste-erittely"))
    };
    let parts = PointBreakdown {
        rt: add(field(22)?, field(24)?)?,
        time: field(19)?,
        other: add(field(20)?, field(25)?)?,
    };
    let state = scored_state(parts, driving)?;
    if state.points != Some(c.total_penalties) {
        return Err(invalid("Piste-erittely ei vastaa kokonaispisteitä"));
    }
    Ok(state)
}
struct LiveState {
    points: Option<i64>,
    breakdown: Option<PointBreakdown>,
    driving: String,
    terminal: bool,
}
fn live_state(
    project: &Project,
    archive: &crate::legacy::Archive,
    c: &crate::legacy::Competitor,
    interim: bool,
) -> Result<LiveState, CoreError> {
    if archive.can_publish_competitor(c) && c.status != "TRK" {
        return Ok(LiveState {
            points: None,
            breakdown: None,
            driving: c.status.clone(),
            terminal: true,
        });
    }
    if archive.has_approved_result(c.number) && c.status == "TRK" {
        return full_state(c, "Maalissa".into());
    }
    let native = project.kilpailijat.iter().find(|d| d.numero == c.number);
    let source = native.and_then(|d| d.lisavirheet.tulkki_siirto.as_ref());
    let (completed, total) =
        source.map_or((&[][..], 0), |s| (s.tulkatut_jaksot.as_slice(), s.jaksoja));
    let next = (1..=total).find(|s| !completed.contains(s));
    let stored = native.and_then(|d| d.tallennettu_tulos.as_ref());
    let finished =
        (total > 0 && next.is_none()) || (total == 0 && stored.is_some_and(|r| r.can_approve()));
    let driving = if finished {
        "Maalissa".into()
    } else {
        format!("Jakso {}", next.unwrap_or(1))
    };
    let none = || LiveState {
        points: None,
        breakdown: None,
        driving: driving.clone(),
        terminal: false,
    };
    if !interim || (total > 0 && completed.is_empty()) {
        return Ok(none());
    }
    let Some(driver) = native else {
        return Ok(none());
    };
    if finished && stored.is_some_and(|r| r.can_approve()) {
        return full_state(c, driving);
    }
    if completed.is_empty() {
        return Ok(none());
    }
    let add = |a: i64, b: i64| {
        a.checked_add(b)
            .ok_or(CoreError::ArithmeticOverflow("Livetuloksen pisteet"))
    };
    let rt = add(
        driver.lisavirheet.puuttuvat_rt_pisteet,
        driver.lisavirheet.ylimaaraiset_rt_pisteet,
    )?;
    let mut other = driver.lisavirheet.muut_virhepisteet;
    let mut time = 0i64;
    if let Some(result) = stored {
        // Sama aika-asemien tyyppi- ja järjestyskohdistus kuin puhelinaikojen vastaanotossa.
        if c.results.is_empty()
            || result.kasikellot_muuttuneet
            || !result.virheelliset_ajat.is_empty()
        {
            return Ok(none());
        }
        let route = project
            .luokat
            .iter()
            .find(|class| class.tunnus == driver.luokka)
            .and_then(|class| project.oikeat_rivit.iter().find(|r| r.rata == class.rata));
        let Some(route) = route else {
            return Ok(none());
        };
        let clocks = route
            .merkinnat
            .iter()
            .filter(|e| e.merkinta.kind.is_clock())
            .collect::<Vec<_>>();
        let rows = result
            .rivit
            .iter()
            .filter(|r| matches!(r.station_type.as_str(), "AT" | "JAT" | "ULA" | "M"))
            .collect::<Vec<_>>();
        if clocks.len() != rows.len() {
            return Ok(none());
        }
        for (entry, row) in clocks.iter().zip(rows) {
            let kind = match row.station_type.as_str() {
                "AT" => crate::interpretation::EntryKind::At,
                "JAT" => crate::interpretation::EntryKind::Jat,
                "ULA" => crate::interpretation::EntryKind::Ula,
                _ => crate::interpretation::EntryKind::Finish,
            };
            if entry.merkinta.kind != kind {
                return Ok(none());
            }
            if completed.contains(&entry.merkinta.section) {
                time = add(
                    time,
                    row.signed_time_penalty
                        .checked_abs()
                        .ok_or(CoreError::ArithmeticOverflow("Aikapisteet"))?,
                )?;
                other = add(other, row.other_penalty)?;
            }
        }
    }
    scored_state(PointBreakdown { rt, time, other }, driving)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_local_writer_waiting_for_lock_cannot_replace_restarted_publication() {
        let directory = std::env::temp_dir().join(format!(
            "as2026-live-peruutus-{}-{}",
            std::process::id(),
            NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let old = vec![(directory.join("tulos.html"), "vanha tilanne".into())];
        let new = vec![(directory.join("tulos.html"), "uusi tilanne".into())];
        let stopped = AtomicBool::new(false);
        let serial = LOCAL_WRITE.lock().unwrap();
        std::thread::scope(|scope| {
            let (ready, waiting) = std::sync::mpsc::channel();
            let old_pages = &old;
            let stop = &stopped;
            let old_writer = scope.spawn(move || {
                ready.send(()).unwrap();
                write_pages_with_cancel(old_pages, Some(stop))
            });
            waiting.recv().unwrap();
            stopped.store(true, Ordering::Relaxed);
            let new_writer = scope.spawn(|| write_pages(&new));
            drop(serial);
            assert!(old_writer.join().unwrap().is_err());
            new_writer.join().unwrap().unwrap();
        });
        assert_eq!(std::fs::read_to_string(&new[0].0).unwrap(), "uusi tilanne");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
