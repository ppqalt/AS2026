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
    for archive in archives(project)? {
        let ranked = application::rank_stored(&archive, TiePolicy::PairwiseControlComparison)?;
        for group in ranked.groups {
            if !settings.classes.contains(&group.code) {
                continue;
            }
            let mut rows = String::new();
            let mut entries = group.entries.iter().collect::<Vec<_>>();
            entries.sort_by_key(|e| {
                (
                    e.status != crate::ranking::Status::Classified,
                    e.primary_position,
                )
            });
            let pending = archive.competitors.iter().filter(|c| {
                let member = if group.kind == crate::ranking::GroupKind::Ordinary { c.class_code == group.code } else {
                    archive.basic.combined_classes.iter().any(|g| group.code == format!("Y{}: {}", g.first().map(String::as_str).unwrap_or(""), g.get(1).map(String::as_str).unwrap_or("")) && archive.basic.classes.iter().enumerate().any(|(i, class)| class.code == c.class_code && g.get(4 + i).is_some_and(|v| v == "1")))
                };
                member && !archive.can_publish_competitor(c) && !matches!(c.status.as_str(), "KESK" | "HYL" | "SULJ" | "PER" | "EILÄ")
            }).map(|c| format!("<tr><td>—</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">AJAA</td></tr>", c.number, escape(&c.driver), escape(&c.navigator))).collect::<String>();
            let mut pending_added = false;
            for entry in entries {
                if entry.status != crate::ranking::Status::Classified && !pending_added {
                    rows.push_str(&pending);
                    pending_added = true;
                }
                let Some(driver) = archive.competitors.iter().find(|c| {
                    i64::from(c.number) == entry.number && archive.can_publish_competitor(c)
                }) else {
                    continue;
                };
                let approved = archive.has_approved_result(driver.number);
                let classified = entry.status == crate::ranking::Status::Classified;
                let place = if classified {
                    entry.primary_position.to_string()
                } else {
                    escape(entry.status.as_legacy())
                };
                let total = if approved && classified {
                    driver.total_penalties.to_string()
                } else {
                    "—".into()
                };
                rows.push_str(&format!("<tr><td>{place}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">{total}</td></tr>", driver.number, escape(&driver.driver), escape(&driver.navigator)));
            }
            if !pending_added {
                rows.push_str(&pending);
            }
            selected.push((group.code, rows));
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
    Ok(selected.iter().enumerate().map(|(i, (class, rows))| {
        let next = url(&names[(i + 1) % names.len()]);
        let html = format!(r#"<!doctype html><html lang="fi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="refresh" content="{refresh_seconds};url={next}"><meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"><title>{competition} · {class}</title><style>
:root{{color-scheme:light;font:16px/1.2 system-ui,sans-serif;color:#152433;background:#f2f5f8}}body{{margin:0}}header,main{{max-width:1400px;margin:auto;padding:.5rem 1rem}}h1{{font-size:1.6rem;margin:.15rem 0}}h2{{font-size:1.25rem;margin:0 0 .4rem}}p{{margin:.2rem 0}}.brand{{font-weight:750;letter-spacing:.06em;color:#245779;font-size:.8rem}}.subtitle{{color:#4d6375;font-size:.85rem}}section{{background:#fff;border:1px solid #d7e0e8;border-radius:10px;padding:.6rem}}.table-wrap{{overflow-x:auto}}table{{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}}th,td{{text-align:left;padding:.18rem .5rem;border-bottom:1px solid #dce4eb}}th{{background:#edf2f6;font-size:.9rem}}.numeric{{text-align:right;white-space:nowrap}}tbody tr:nth-child(even){{background:#f8fafc}}@media(max-width:600px){{:root{{font-size:13px}}header,main{{padding:.4rem}}th,td{{padding:.18rem .25rem}}}}
</style></head><body><header><p class="brand">AS2026</p><h1>{competition}</h1><p class="subtitle">Epäviralliset tulokset · Päivitetty {updated} · Sivu {page}/{count}</p></header><main><section><h2>Luokka: {class}</h2><div class="table-wrap"><table><thead><tr><th>Sij.</th><th>Nro</th><th>Kuljettaja</th><th>Kartanlukija</th><th class="numeric">Tulos</th></tr></thead><tbody>{rows}</tbody></table></div></section></main></body></html>"#,
            competition=escape(&project.kilpailu.nimi), class=escape(class), page=i+1, count=selected.len());
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
