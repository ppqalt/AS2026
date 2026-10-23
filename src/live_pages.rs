//! Compact class pages shared by local preview and authenticated live publication.
use crate::{CoreError, application, native::Project, ranking::TiePolicy};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
            application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag)?
                .groups
                .into_iter()
                .map(|g| g.code),
        );
    }
    Ok(choices)
}
pub fn pages(project: &Project, settings: &Settings) -> Result<Vec<(PathBuf, String)>, CoreError> {
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
        let ranked = application::rank_stored(&archive, TiePolicy::KeepSharedAndFlag)?;
        for group in ranked.groups {
            if !settings.classes.contains(&group.code) {
                continue;
            }
            let mut rows = String::new();
            for entry in &group.entries {
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
        let html = format!(r#"<!doctype html><html lang="fi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="refresh" content="60;url={next}"><meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate"><title>{competition} · {class}</title><style>
:root{{color-scheme:light;font:16px/1.2 system-ui,sans-serif;color:#152433;background:#f2f5f8}}body{{margin:0}}header,main{{max-width:1400px;margin:auto;padding:.5rem 1rem}}h1{{font-size:1.6rem;margin:.15rem 0}}h2{{font-size:1.25rem;margin:0 0 .4rem}}p{{margin:.2rem 0}}.brand{{font-weight:750;letter-spacing:.06em;color:#245779;font-size:.8rem}}.subtitle{{color:#4d6375;font-size:.85rem}}section{{background:#fff;border:1px solid #d7e0e8;border-radius:10px;padding:.6rem}}.table-wrap{{overflow-x:auto}}table{{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}}th,td{{text-align:left;padding:.18rem .5rem;border-bottom:1px solid #dce4eb}}th{{background:#edf2f6;font-size:.9rem}}.numeric{{text-align:right;white-space:nowrap}}tbody tr:nth-child(even){{background:#f8fafc}}@media(max-width:600px){{:root{{font-size:13px}}header,main{{padding:.4rem}}th,td{{padding:.18rem .25rem}}}}
</style></head><body><header><p class="brand">AS2026</p><h1>{competition}</h1><p class="subtitle">Epäviralliset tulokset · Päivitetty {updated} · Sivu {page}/{count}</p></header><main><section><h2>Luokka: {class}</h2><div class="table-wrap"><table><thead><tr><th>Sij.</th><th>Nro</th><th>Kuljettaja</th><th>Kartanlukija</th><th class="numeric">Tulos</th></tr></thead><tbody>{rows}</tbody></table></div></section></main></body></html>"#,
            competition=escape(&project.kilpailu.nimi), class=escape(class), page=i+1, count=selected.len());
        (settings.base.with_file_name(&names[i]), html)
    }).collect())
}

pub fn write_pages(pages: &[(PathBuf, String)]) -> Result<(), CoreError> {
    for (path, html) in pages.iter().rev() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| invalid("Livetulosten kansiota ei voitu luoda"))?;
        }
        let temporary = path.with_extension("html.as2026-tmp");
        std::fs::write(&temporary, html)
            .and_then(|_| crate::storage::file_operation(|| std::fs::rename(&temporary, path)))
            .map_err(|_| invalid("Livetulossivun tallennus epäonnistui"))?;
    }
    Ok(())
}
