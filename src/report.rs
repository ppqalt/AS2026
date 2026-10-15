//! Itsenäiset suomenkieliset HTML-raportit. Ei verkkoa, komentosarjoja tai
//! ulkoisia tyylitiedostoja. Kaikki aineistosta tuleva teksti suojataan HTML:ää
//! varten. Tulosten tallennus luo aina uuden tiedoston.

use std::{fs::OpenOptions, io::Write, path::Path};

use crate::{
    CoreError,
    application::CalculationReport,
    legacy::{Archive, Competitor},
    ranking::{GroupKind, RankingReport, Status},
};

fn escape(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => result.push_str("&amp;"),
            '<' => result.push_str("&lt;"),
            '>' => result.push_str("&gt;"),
            '"' => result.push_str("&quot;"),
            '\'' => result.push_str("&#39;"),
            _ => result.push(character),
        }
    }
    result
}

fn document(title: &str, competition: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="fi"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — {competition}</title><style>
:root{{color-scheme:light;font:16px/1.5 system-ui,sans-serif;color:#152433;background:#f2f5f8}}
body{{margin:0}}header,main{{max-width:1180px;margin:auto;padding:1.5rem}}header{{padding-bottom:.5rem}}
h1{{font-size:2rem;line-height:1.2;margin:.25rem 0}}h2{{font-size:1.45rem;margin:0 0 .5rem}}h3{{font-size:1.1rem}}
.brand{{font-weight:750;letter-spacing:.06em;color:#245779;margin:0}}.subtitle{{color:#4d6375}}
section{{background:white;border:1px solid #d7e0e8;border-radius:10px;padding:1.25rem;margin-bottom:1.25rem}}
.notice{{background:#edf4fa;border-left:4px solid #27638c;padding:.8rem 1rem}}.warning{{background:#fff5dc;border-left-color:#9b6b00}}
.table-wrap{{overflow-x:auto}}table{{width:100%;border-collapse:collapse;font-variant-numeric:tabular-nums}}
th,td{{text-align:left;padding:.5rem .6rem;border-bottom:1px solid #dce4eb;vertical-align:top}}
th{{background:#edf2f6;font-size:.9rem}}td.numeric,th.numeric{{text-align:right;white-space:nowrap}}
tbody tr:nth-child(even){{background:#f8fafc}}.muted{{color:#536879}}ul{{padding-left:1.4rem}}
.totals{{font-weight:650}}.status{{font-weight:650}}.nowrap{{white-space:nowrap}}
@media(max-width:600px){{header,main{{padding:1rem}}section{{padding:.8rem}}h1{{font-size:1.6rem}}th,td{{padding:.4rem}}}}
@media print{{:root{{font:10pt/1.35 sans-serif;background:white;color:black}}header,main{{padding:0;max-width:none}}
section{{border:0;border-radius:0;padding:.5rem 0;margin:0 0 1rem}}.table-wrap{{overflow:visible}}table{{font-size:9pt}}
thead{{display:table-header-group}}tr{{break-inside:avoid}}h2,h3{{break-after:avoid}}.notice{{background:white;border:1px solid #999}}
th{{background:#eee}}}}
</style></head><body><header><p class="brand">AS2015</p><h1>{title}</h1><p class="subtitle">{competition}</p></header><main>
"#,
        title = escape(title),
        competition = escape(competition)
    )
}

fn finish(mut html: String) -> String {
    html.push_str("</main></body></html>\n");
    html
}

fn warnings(html: &mut String, warnings: &[String]) {
    if warnings.is_empty() {
        return;
    }
    html.push_str("<section class=\"notice warning\"><h2>Huomautukset</h2><ul>");
    for warning in warnings {
        html.push_str(&format!("<li>{}</li>\n", escape(warning)));
    }
    html.push_str("</ul></section>\n");
}

/// Raakaleimauksista johdettu erillinen esikatselu, joka säilyttää tallennetut
/// tulokset vertailuarvoina ja näyttää myös osittaiset tai epäonnistuneet laskut.
pub fn calculation_html(report: &CalculationReport) -> String {
    let mut html = document("Tuloslaskennan esikatselu", &report.competition_name);
    html.push_str("<section class=\"notice\"><p>Tallennettu tulos säilyy ennallaan. Esikatselu lasketaan raakaleimauksista, eikä sitä käytetä luokka- tai ratatulosten sijoituksiin. Alkuperäisillä kilpailutuloksilla tehtävä vertailu on vielä tarpeen.</p></section>\n");
    html.push_str("<section class=\"notice warning\"><p><strong>Esikatselu sisältää vain asemarivien virhepisteet.</strong> Erillisiä puuttuvien ja ylimääräisten RT-asemien lisäyksiä sekä muita kilpailijakohtaisia virhepisteitä ei vielä yhdistetä esikatseluun.</p></section>\n");
    warnings(&mut html, &report.warnings);
    if report.competitors.is_empty() {
        html.push_str("<section><p>Ei kilpailijoita.</p></section>\n");
    }
    for competitor in &report.competitors {
        html.push_str(&format!("<section><h2>No. {} — {}</h2><p>Luokka: {}. Tallennettu tulos: <strong>{} p</strong>.</p>\n",
            competitor.number, escape(&competitor.driver), escape(&competitor.class_code), competitor.imported_total));
        if !competitor.notes.is_empty() {
            html.push_str("<ul>");
            for note in &competitor.notes {
                html.push_str(&format!("<li>{}</li>", escape(note)));
            }
            html.push_str("</ul>");
        }
        match &competitor.calculation {
            Err(error) => html.push_str(&format!(
                "<p class=\"notice warning\">Esikatselua ei voitu laskea: {}</p>",
                escape(&error.to_string())
            )),
            Ok(calculation) => {
                html.push_str(&format!("<p class=\"status\">Tarkastustulos: {}{}</p><p class=\"totals\">Aikavirhepisteet: {} p · Muut virhepisteet: {} p · Virhepisteet yhteensä (asemarivit): {} p</p>\n",
                    calculation.status.as_legacy(), if calculation.reason.is_empty() { String::new() } else { format!(" — {}", escape(&calculation.reason)) },
                    calculation.totals.time_points, calculation.totals.other_points, calculation.totals.total_points));
                if !calculation.diagnostics.is_empty() {
                    html.push_str("<h3>Laskennan huomautukset</h3><ul>");
                    for diagnostic in &calculation.diagnostics {
                        let row = diagnostic
                            .row
                            .map_or(String::new(), |row| format!("Rivi {row}: "));
                        html.push_str(&format!("<li>{row}{}</li>", escape(&diagnostic.message)));
                    }
                    html.push_str("</ul>");
                }
                html.push_str("<div class=\"table-wrap\"><table><thead><tr><th scope=\"col\">Nro</th><th scope=\"col\">Tyyppi</th><th scope=\"col\">Nimi</th><th scope=\"col\">Emit</th><th scope=\"col\" class=\"numeric\">Tavoite (s)</th><th scope=\"col\" class=\"numeric\">Toteutunut (s)</th><th scope=\"col\" class=\"numeric\">Aikavirhepisteet</th><th scope=\"col\" class=\"numeric\">Muut virhepisteet</th><th scope=\"col\" class=\"numeric\">Virhepisteet yhteensä</th><th scope=\"col\">Huomautus</th></tr></thead><tbody>\n");
                for row in &calculation.rows {
                    let station = row.source.station.as_ref();
                    let kind =
                        station.map_or("ylimääräinen", |station| station.kind.as_legacy());
                    let name = station.map_or("", |station| station.label.as_str());
                    html.push_str(&format!("<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td>{}</td></tr>\n",
                        row.source.output_index, escape(kind), escape(name), row.received_code, row.target_seconds, row.actual_seconds,
                        row.signed_time_points, row.other_points, row.total_points, escape(&row.note)));
                }
                html.push_str("</tbody></table></div>\n");
            }
        }
        html.push_str("</section>\n");
    }
    finish(html)
}

fn competition_name(archive: &Archive) -> &str {
    archive
        .basic
        .identity
        .get(1)
        .map(String::as_str)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Nimetön kilpailu")
}

fn competitor(archive: &Archive, number: i64) -> Option<&Competitor> {
    archive
        .competitors
        .iter()
        .find(|competitor| i64::from(competitor.number) == number)
}

fn place(position: u32, status: &Status) -> String {
    if *status == Status::Classified && (1..25_000).contains(&position) {
        position.to_string()
    } else {
        escape(status.as_legacy())
    }
}

fn ranking_heading(html: &mut String, points: bool) {
    html.push_str("<div class=\"table-wrap\"><table><thead><tr><th scope=\"col\">Sij.</th><th scope=\"col\">No.</th><th scope=\"col\">Kuljettaja</th><th scope=\"col\">Kartanlukija</th><th scope=\"col\">Luokka</th><th scope=\"col\" class=\"numeric\">Tulos</th>");
    if points {
        html.push_str("<th scope=\"col\" class=\"numeric\" title=\"nousupisteet\">Np</th><th scope=\"col\" class=\"numeric\" title=\"sarjapisteet\">Sp</th>");
    }
    html.push_str("</tr></thead><tbody>\n");
}

fn ranking_row(
    html: &mut String,
    archive: &Archive,
    number: i64,
    position: u32,
    status: &Status,
    points: Option<(Option<u8>, Option<i64>)>,
) {
    let source = competitor(archive, number);
    let driver = source.map_or("—", |competitor| competitor.driver.as_str());
    let navigator = source.map_or("—", |competitor| competitor.navigator.as_str());
    let class = source.map_or("—", |competitor| competitor.class_code.as_str());
    let total = source.map_or("—".to_owned(), |competitor| {
        format!("{} p", competitor.total_penalties)
    });
    html.push_str(&format!("<tr><td>{}</td><td>{number}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">{total}</td>",
        place(position, status), escape(driver), escape(navigator), escape(class)));
    if let Some((np, sp)) = points {
        html.push_str(&format!(
            "<td class=\"numeric\">{}</td><td class=\"numeric\">{}</td>",
            np.map_or("—".to_owned(), |points| points.to_string()),
            sp.map_or("—".to_owned(), |points| points.to_string())
        ));
    }
    html.push_str("</tr>\n");
}

/// Luokka- ja ratatulokset perustuvat tallennettuihin tuloksiin. Raportti ei
/// korvaa niitä korttilaskennan esikatselulla eikä muuta lähdetiedostoja.
pub fn ranking_html(archive: &Archive, report: &RankingReport) -> String {
    let mut html = document("Luokkakohtaiset tulokset", competition_name(archive));
    html.push_str("<section class=\"notice\"><p>Sijoitukset ja pisteet perustuvat tuodun Kilp.txt-tiedoston tallennettuihin tuloksiin. Korttilaskennan esikatselu ei muuta näitä tuloksia. Tasapisteiden esitysjärjestys ei ratkaise jaettua sijaa.</p></section>\n");
    warnings(&mut html, &archive.warnings);
    if !report.diagnostics.is_empty() {
        html.push_str("<section class=\"notice warning\"><h2>Tulosten huomautukset</h2><ul>");
        for diagnostic in &report.diagnostics {
            html.push_str(&format!("<li>{}</li>", escape(&diagnostic.to_string())));
        }
        html.push_str("</ul></section>\n");
    }
    if !report.omitted_source_indices.is_empty() {
        html.push_str(&format!("<section class=\"notice\"><p>Tulosluettelon ulkopuolelle jäi {} tietuetta tarkastustuloksen perusteella. Tietueet säilyvät alkuperäisessä aineistossa.</p></section>", report.omitted_source_indices.len()));
    }
    if report.groups.is_empty() {
        html.push_str("<section><p>Ei luokkakohtaisia tuloksia.</p></section>");
    }
    for group in &report.groups {
        let label = if group.kind == GroupKind::Combined {
            "Yhdistetty luokka"
        } else {
            "Luokka"
        };
        html.push_str(&format!("<section><h2>{label}: {}</h2><p>Osallistumistilastoa: {} kpl. Tuloslaskennan lähtijämäärä: {}.</p>", escape(&group.code), group.statistics.all_entries, group.statistics.starter_count()));
        if group.provisional {
            html.push_str("<p class=\"notice warning\"><strong>Alustavat tulokset.</strong> Nousu- ja sarjapisteitä ei julkaista ennen avoimien tarkastustulosten selvittämistä.</p>");
        }
        ranking_heading(&mut html, true);
        for entry in &group.entries {
            ranking_row(
                &mut html,
                archive,
                entry.number,
                entry.primary_position,
                &entry.status,
                Some((
                    (group.np_enabled && !group.provisional).then_some(entry.table_points),
                    (group.series_method != 0 && !group.provisional).then_some(entry.series_points),
                )),
            );
        }
        html.push_str("</tbody></table></div></section>\n");
    }
    html.push_str("<h2>Ratakohtaiset tulokset</h2>\n");
    if report.routes.is_empty() {
        html.push_str("<section><p>Ei ratakohtaisia tuloksia.</p></section>");
    }
    for route in &report.routes {
        html.push_str(&format!("<section><h3>Rata: {}</h3>", escape(&route.code)));
        ranking_heading(&mut html, false);
        for entry in &route.entries {
            ranking_row(
                &mut html,
                archive,
                entry.number,
                entry.primary_position,
                &entry.status,
                None,
            );
        }
        html.push_str("</tbody></table></div></section>\n");
    }
    finish(html)
}

/// Luo uuden UTF-8-tiedoston. Myös olemassa oleva symbolinen linkki estää
/// tallennuksen; olemassa olevaa tiedostoa ei avata kirjoitettavaksi.
pub fn save_new(path: &Path, html: &str) -> Result<(), CoreError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            CoreError::InvalidInput(if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "Tiedosto on jo olemassa, sitä ei korvata: {}",
                    path.display()
                )
            } else {
                format!(
                    "Raporttia ei voitu luoda: {}: {}",
                    path.display(),
                    crate::io_message(&error)
                )
            })
        })?;
    file.write_all(html.as_bytes()).map_err(|error| {
        CoreError::InvalidInput(format!(
            "Raportin kirjoitus epäonnistui; keskeneräinen tiedosto voi jäädä polkuun {}: {}",
            path.display(),
            crate::io_message(&error)
        ))
    })
}
