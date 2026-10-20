//! Itsenäiset suomenkieliset HTML-raportit. Ei verkkoa, komentosarjoja tai
//! ulkoisia tyylitiedostoja. Kaikki aineistosta tuleva teksti suojataan HTML:ää
//! varten. Tulosten tallennus luo aina uuden tiedoston.

use std::{borrow::Cow, fs::OpenOptions, io::Write, path::Path};

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
</style></head><body><header><p class="brand">AS2026</p><h1>{title}</h1><p class="subtitle">{competition}</p></header><main>
"#,
        title = escape(title),
        competition = escape(competition)
    )
}

fn finish(mut html: String) -> String {
    html.push_str("</main></body></html>\n");
    html
}

/// Muotoilee laskentaytimen tekniset huomautukset kilpailun toimitsijalle.
/// Laskentatuloksia tai niiden diagnostiikkatietoja ei muuteta.
pub fn user_notice(message: &str) -> Option<String> {
    if message.ends_with(
        "viimeinen kenttä säilytettiin ilman rivinvaihtoa; vanha lukija voi pudottaa sen",
    ) {
        return None;
    }
    let (prefix, message) = message
        .split_once(": ")
        .filter(|(prefix, _)| prefix.starts_with("Kilpailija "))
        .map_or((String::new(), message), |(prefix, rest)| {
            (format!("{prefix}: "), rest)
        });
    let message = message.strip_prefix("Ei vielä tuettu: ").unwrap_or(message);
    let displayed = match message {
        "Esikatselu sisältää asemarivien virhepisteet. Erillisiä RT-lisäyksiä ja muita kilpailijakohtaisia virhepisteitä ei vielä yhdistetä siihen." =>
        {
            return None;
        }
        "Vanha ohjelma voi vähentää leimattoman lähdön aikakorjauksen kahdesti leimausten kellonajoista. Pisteissä ja maaliajassa korjaus tehdään kerran. Tässä leimausten kellonajat noudattavat vanhan ohjelman näyttökaavaa." => {
            "Käsilähdön aikakorjaus on huomioitu."
        }
        "Palautettu korttilaskenta johtaa leimattoman maalin ajan kortin viimeisestä leimasta. Vanhan ohjelman tämän kohdan käsittely on ristiriitainen. Valitun laskentatavan vastaavuus vanhoihin kilpailutuloksiin on vielä tarkistettava." => {
            "Maaliaika on johdettu kortin viimeisestä leimasta."
        }
        "AS2015A:n leimaamattoman maalin toteutunut aika jää nollaksi, ja maalikello kirjoitetaan vasta ennen toista sakkokierrosta. Tulos on staattinen ennuste." => {
            "Maalikello perustuu viimeiseen leimaan, mutta leimaamattoman maalin toteutunut aika on laskennassa 0 s."
        }
        "Palautettu ensimmäinen kierros tyhjentää puuttuvan JAT:n tavoitesiirron ennen erillistä maalilaskentaa; tavoitetta ei lisätä maaliin." => {
            "Puuttuvan JAT-aseman tavoiteaikaa ei lisätä maalijaksoon."
        }
        "Vanhan ohjelman korttilaskenta keskeytyy puuttuvaan maalileimaan, eikä kaikkien tuloskenttien käsittely ole selvä. Tässä laskennassa asemarivit säilytetään ja puuttumissakko sekä muiden asemien virheet lasketaan. Kilpailijan tila on keskeyttänyt." => {
            "Maalileima puuttuu. Kilpailijan tila on keskeyttänyt."
        }
        "Mitätöity rivi säilyy aikarakenteessa. Toisen kierroksen aikamuutoksia ei levitetä uudelleen myöhempien rivien ensimmäisen kierroksen aikoihin." => {
            "Mitätöity asema säilyy radan aikarakenteessa."
        }
        "Korttilaskenta ei määrää tavallista puuttumissakkoa asemalle, jonka aika oli tarkoitus kirjata käsin. Tallennettujen tulosten uudelleenlaskenta käsittelee nämä asemat erikseen." => {
            "Käsin kirjattavan aseman puuttuvasta leimasta ei määrätä puuttumissakkoa."
        }
        "Vanhan ohjelman JAT-aseman myöhästymisrajan vertailu ei voi laueta, kun raja on positiivinen. Laskenta säilyttää tämän vertailun. Kokonaismyöhästymisraja ja maalin jakson raja tarkistetaan erikseen." =>
        {
            return None;
        }
        "Jakso-kenttä ei läpäise palautettua numerotarkistusta; vanha polku lisää kertymään nollan." => {
            "Jakson tavoiteaika ei ole kelvollinen numero. Tarkista aseman tiedot."
        }
        "leimattomalle lähdölle ei ole hyväksyttyä lähtöaikaa; tämän tapauksen laskentaa ei ole vielä vahvistettu" => {
            "Leimattomalle lähdölle ei ole hyväksyttyä lähtöaikaa."
        }
        "00:00:00-käsilähtö luokitellaan vanhassa ohjelmassa puuttuvaksi" => {
            "Lähtöaika 00:00:00 puuttuu. Anna hyväksytty lähtöaika."
        }
        _ => message,
    };
    Some(format!("{prefix}{displayed}"))
}

fn warnings(html: &mut String, warnings: &[String]) {
    let warnings = warnings
        .iter()
        .filter_map(|warning| user_notice(warning))
        .collect::<Vec<_>>();
    if warnings.is_empty() {
        return;
    }
    html.push_str("<section class=\"notice warning\"><h2>Huomautukset</h2><ul>");
    for warning in warnings {
        html.push_str(&format!("<li>{}</li>\n", escape(&warning)));
    }
    html.push_str("</ul></section>\n");
}

/// Raakaleimauksista johdettu erillinen esikatselu, joka säilyttää tallennetut
/// tulokset vertailuarvoina ja näyttää myös osittaiset tai epäonnistuneet laskut.
pub fn calculation_html(report: &CalculationReport) -> String {
    calculation_html_with_source(report, false)
}

pub fn project_calculation_html(report: &CalculationReport) -> String {
    calculation_html_with_source(report, true)
}

fn calculation_html_with_source(report: &CalculationReport, native: bool) -> String {
    let mut html = document("Tuloslaskennan esikatselu", &report.competition_name);
    if native {
        html.push_str("<section class=\"notice\"><p>Hyväksymätön esikatselu. Tämä raportti ei vahvista tuloksia eikä sijoituksia. Asematulokset on laskettu kilpailutiedoista ja Emit-leimauksista.</p></section>\n");
    } else {
        html.push_str("<section class=\"notice\"><p>Asematulokset on laskettu raakaleimauksista. Tuodun kilpailun tallennetut tulokset säilyvät ennallaan.</p></section>\n");
    }
    warnings(&mut html, &report.warnings);
    if report.competitors.is_empty() {
        html.push_str("<section><p>Ei kilpailijoita.</p></section>\n");
    }
    for competitor in &report.competitors {
        if native {
            html.push_str(&format!(
                "<section><h2>Nro {} — {}</h2><p>Luokka: {}.</p>\n",
                competitor.number,
                escape(&competitor.driver),
                escape(&competitor.class_code)
            ));
        } else {
            html.push_str(&format!("<section><h2>Nro {} — {}</h2><p>Luokka: {}. Tallennettu tulos: <strong>{} p</strong>.</p>\n",
            competitor.number, escape(&competitor.driver), escape(&competitor.class_code), competitor.imported_total));
        }
        let notes = competitor
            .notes
            .iter()
            .filter_map(|note| user_notice(note))
            .collect::<Vec<_>>();
        if !notes.is_empty() {
            html.push_str("<ul>");
            for note in &notes {
                html.push_str(&format!("<li>{}</li>", escape(note)));
            }
            html.push_str("</ul>");
        }
        match &competitor.calculation {
            Err(error) => html.push_str(&format!(
                "<p class=\"notice warning\">Laskentaa ei voitu tehdä: {}</p>",
                escape(
                    &user_notice(&error.to_string())
                        .unwrap_or_else(|| "Tarkista kilpailijan ja radan tiedot.".into())
                )
            )),
            Ok(calculation) => {
                html.push_str(&format!("<p class=\"status\">Tarkastustulos: {}{}</p><p class=\"totals\">Aikavirhepisteet: {} p · Muut virhepisteet: {} p · Virhepisteet yhteensä (asemarivit): {} p</p>\n",
                    calculation.status.as_legacy(), if calculation.reason.is_empty() { String::new() } else { format!(" — {}", escape(&calculation.reason)) },
                    calculation.totals.time_points, calculation.totals.other_points, calculation.totals.total_points));
                let diagnostics = calculation
                    .diagnostics
                    .iter()
                    .filter_map(|diagnostic| {
                        user_notice(&diagnostic.message).map(|message| (diagnostic.row, message))
                    })
                    .collect::<Vec<_>>();
                if !diagnostics.is_empty() {
                    html.push_str("<h3>Laskennan huomautukset</h3><ul>");
                    for (row, message) in &diagnostics {
                        let row = row.map_or(String::new(), |row| format!("Rivi {row}: "));
                        html.push_str(&format!("<li>{row}{}</li>", escape(message)));
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
    html.push_str("<div class=\"table-wrap\"><table><thead><tr><th scope=\"col\">Sij.</th><th scope=\"col\">Nro</th><th scope=\"col\">Kuljettaja</th><th scope=\"col\">Kartanlukija</th><th scope=\"col\">Luokka</th><th scope=\"col\" class=\"numeric\">Tulos</th>");
    if points {
        html.push_str("<th scope=\"col\" class=\"numeric\" title=\"nousupisteet\">Np</th><th scope=\"col\" class=\"numeric\" title=\"sarjapisteet\">Sp</th>");
    }
    html.push_str("<th scope=\"col\">Syy</th>");
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
    if source.is_none_or(|c| !archive.can_publish_competitor(c)) {
        return;
    }
    let driver = source.map_or("—", |competitor| competitor.driver.as_str());
    let navigator = source.map_or("—", |competitor| competitor.navigator.as_str());
    let class = source.map_or("—", |competitor| competitor.class_code.as_str());
    let approved = source.is_some_and(|c| archive.has_approved_result(c.number));
    let total = source
        .filter(|_| approved)
        .map_or("—".to_owned(), |competitor| {
            format!("{} p", competitor.total_penalties)
        });
    html.push_str(&format!("<tr><td>{}</td><td>{number}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"numeric\">{total}</td>",
        place(position, status), escape(driver), escape(navigator), escape(class)));
    if let Some((np, sp)) = points {
        html.push_str(&format!(
            "<td class=\"numeric\">{}</td><td class=\"numeric\">{}</td>",
            np.filter(|_| approved)
                .map_or("—".to_owned(), |points| points.to_string()),
            sp.filter(|_| approved)
                .map_or("—".to_owned(), |points| points.to_string())
        ));
    }
    let reason = source
        .and_then(|competitor| competitor.header_fields.get(29))
        .map_or("", String::as_str);
    html.push_str(&format!("<td>{}</td></tr>\n", escape(reason)));
}

/// Luokka- ja ratatulokset perustuvat tallennettuihin tuloksiin. Raportti ei
/// korvaa niitä korttilaskennan esikatselulla eikä muuta lähdetiedostoja.
pub fn ranking_html(archive: &Archive, report: &RankingReport) -> String {
    ranking_html_with_source(archive, report, false)
}

/// AS2026-kilpailun nykyisestä mallista johdetut asematulokset.
pub fn project_ranking_html(archive: &Archive, report: &RankingReport) -> String {
    ranking_html_with_source(archive, report, true)
}

/// Molemmat laskennat julkaistaan erillisistä arkistoista, jotta saman kilpailijan
/// NAC- ja BC-hyväksynnät eivät sekoitu kilpailijanumeron perusteella.
pub fn competition_ranking_html(
    project: &crate::native::Project,
    policy: crate::ranking::TiePolicy,
) -> Result<String, CoreError> {
    let archive = crate::application::calculated_project(project)?;
    let ranked = crate::application::rank_stored(&archive, policy)?;
    let mut html = document("Luokkakohtaiset tulokset", &project.kilpailu.nimi);
    html.push_str(&ranking_body(&archive, &ranked, true));
    if project.beginners_cup.is_some() {
        let bc = crate::beginners::results(project)?;
        let ranked = crate::application::rank_stored(&bc, policy)?;
        html.push_str(&ranking_body(&bc, &ranked, true));
    }
    Ok(finish(html))
}

pub fn competition_ranking_csv(
    project: &crate::native::Project,
    policy: crate::ranking::TiePolicy,
) -> Result<String, CoreError> {
    let archive = crate::application::calculated_project(project)?;
    let ranked = crate::application::rank_stored(&archive, policy)?;
    let mut csv = ranking_csv(&archive, &ranked);
    if project.beginners_cup.is_some() {
        let bc = crate::beginners::results(project)?;
        let ranked = crate::application::rank_stored(&bc, policy)?;
        let bc_csv = ranking_csv(&bc, &ranked);
        if let Some((_, rows)) = bc_csv.split_once('\n') {
            csv.push_str(rows);
        }
    }
    Ok(csv)
}

fn ranking_html_with_source(archive: &Archive, report: &RankingReport, calculated: bool) -> String {
    let mut html = document("Luokkakohtaiset tulokset", competition_name(archive));
    html.push_str(&ranking_body(archive, report, calculated));
    finish(html)
}

fn ranking_body(archive: &Archive, report: &RankingReport, calculated: bool) -> String {
    let mut html = String::new();
    let report = match publication_ranking(archive, report) {
        Ok(report) => report,
        Err(error) => {
            html.push_str(&format!(
                "<p>Tulosten muodostaminen epäonnistui: {}</p>",
                escape(&error.to_string())
            ));
            return html;
        }
    };
    if calculated {
        html.push_str("<section class=\"notice\"><p>Vain hyväksytyt asematulokset sijoitetaan. Vahvistettu keskeytys julkaistaan ilman sijoitusta, maaliaikaa ja pisteitä. Hyväksymättömät esikatselut jäävät tulosluettelon ulkopuolelle. Tasapisteissä kilpailijoilla on jaettu sija.</p></section>\n");
    } else {
        html.push_str("<section class=\"notice\"><p>Sijoitukset ja pisteet perustuvat tuodun kilpailun tallennettuihin tuloksiin. Tasapisteissä kilpailijoilla on jaettu sija.</p></section>\n");
    }
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
    html
}

/// Vienti ei luota vanhentuneeseen tai käsin muodostettuun sijoitusraporttiin.
/// Säilytä käyttäjän valitsema tasapistekäytäntö, mutta käytä nykyisiä hyväksyntöjä.
fn publication_ranking<'a>(
    archive: &Archive,
    report: &'a RankingReport,
) -> Result<Cow<'a, RankingReport>, CoreError> {
    if archive.result_publication.is_some() {
        crate::application::rank_stored(archive, report.tie_policy).map(Cow::Owned)
    } else {
        Ok(Cow::Borrowed(report))
    }
}

/// Escape CSV text for spreadsheet programs. Treat user-provided formula-like
/// strings as text rather than executable spreadsheet expressions.
fn csv_cell(value: &str) -> String {
    let first = value.trim_start().chars().next();
    let prefix = if first.is_some_and(|c| matches!(c, '=' | '+' | '-' | '@')) {
        "'"
    } else {
        ""
    };
    format!("\"{}\"", format!("{prefix}{value}").replace('"', "\"\""))
}

fn csv_record(csv: &mut String, cells: &[String]) {
    for (index, value) in cells.iter().enumerate() {
        if index > 0 {
            csv.push(';');
        }
        csv.push_str(&csv_cell(value));
    }
    csv.push_str("\r\n");
}

/// Export the same class and route ranking as HTML, preserving stored totals,
/// status sentinels, provisional Np/Sp suppression and shared positions.
/// UTF-8, semicolon-delimited CSV with CRLF for Finnish spreadsheet defaults.
pub fn ranking_csv(archive: &Archive, report: &RankingReport) -> String {
    let mut csv = String::new();
    csv_record(
        &mut csv,
        &[
            "Ryhmän tyyppi".into(),
            "Ryhmä".into(),
            "Sija".into(),
            "Numero".into(),
            "Kuljettaja".into(),
            "Kartanlukija".into(),
            "Luokka".into(),
            "Tila".into(),
            "Virhepisteet".into(),
            "Np".into(),
            "Sp".into(),
            "Syy".into(),
        ],
    );
    let report = match publication_ranking(archive, report) {
        Ok(report) => report,
        Err(error) => {
            csv_record(&mut csv, &["Virhe".into(), error.to_string()]);
            return csv;
        }
    };
    for group in &report.groups {
        for entry in &group.entries {
            let source = archive
                .competitors
                .get(entry.source_index.saturating_sub(1));
            if source.is_none_or(|c| !archive.can_publish_competitor(c)) {
                continue;
            }
            let approved = source.is_some_and(|c| archive.has_approved_result(c.number));
            let pos = if entry.status == Status::Classified {
                entry.primary_position.to_string()
            } else {
                entry.status.as_legacy().to_owned()
            };
            csv_record(
                &mut csv,
                &[
                    (if group.kind == GroupKind::Combined {
                        "Yhdistetty"
                    } else {
                        "Luokka"
                    })
                    .into(),
                    group.code.clone(),
                    pos,
                    entry.number.to_string(),
                    source.map_or("", |c| c.driver.as_str()).into(),
                    source.map_or("", |c| c.navigator.as_str()).into(),
                    source.map_or("", |c| c.class_code.as_str()).into(),
                    entry.status.as_legacy().into(),
                    source
                        .filter(|_| approved)
                        .map_or_else(String::new, |c| c.total_penalties.to_string()),
                    if approved && group.np_enabled && !group.provisional {
                        entry.table_points.to_string()
                    } else {
                        String::new()
                    },
                    if approved && group.series_method != 0 && !group.provisional {
                        entry.series_points.to_string()
                    } else {
                        String::new()
                    },
                    source
                        .and_then(|competitor| competitor.header_fields.get(29))
                        .cloned()
                        .unwrap_or_default(),
                ],
            );
        }
    }
    for route in &report.routes {
        for entry in &route.entries {
            let source = archive
                .competitors
                .get(entry.source_index.saturating_sub(1));
            if source.is_none_or(|c| !archive.can_publish_competitor(c)) {
                continue;
            }
            let pos = if entry.status == Status::Classified {
                entry.primary_position.to_string()
            } else {
                entry.status.as_legacy().to_owned()
            };
            csv_record(
                &mut csv,
                &[
                    "Rata".into(),
                    route.code.clone(),
                    pos,
                    entry.number.to_string(),
                    source.map_or("", |c| c.driver.as_str()).into(),
                    source.map_or("", |c| c.navigator.as_str()).into(),
                    source.map_or("", |c| c.class_code.as_str()).into(),
                    entry.status.as_legacy().into(),
                    source
                        .filter(|c| archive.has_approved_result(c.number))
                        .map_or_else(String::new, |c| c.total_penalties.to_string()),
                    String::new(),
                    String::new(),
                    source
                        .and_then(|competitor| competitor.header_fields.get(29))
                        .cloned()
                        .unwrap_or_default(),
                ],
            );
        }
    }
    csv
}

/// Parse our quoted Finnish CSV so the XLSX export uses identical ranks.
fn csv_rows(csv: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut chars = csv.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                cell.push('"');
            }
            '"' => quoted = !quoted,
            ';' if !quoted => row.push(std::mem::take(&mut cell)),
            '\n' if !quoted => {
                if cell.ends_with('\r') {
                    cell.pop();
                }
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
            }
            _ => cell.push(ch),
        }
    }
    if !cell.is_empty() || !row.is_empty() {
        row.push(cell);
        rows.push(row);
    }
    rows
}

fn xml_text(value: &str) -> String {
    let mut output = String::new();
    for ch in value.chars() {
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            continue;
        }
        match ch {
            '&' => output.push_str("&amp;"),
            '<' => output.push_str("&lt;"),
            '>' => output.push_str("&gt;"),
            '"' => output.push_str("&quot;"),
            '\'' => output.push_str("&apos;"),
            _ => output.push(ch),
        }
    }
    output
}

/// OOXML workbook with inline-string user data: no macros or formulas.
pub fn xlsx_from_csv(csv: &str) -> Result<Vec<u8>, CoreError> {
    use std::io::{Cursor, Write};
    let mut sheet = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><sheetData>",
    );
    for (row_index, values) in csv_rows(csv).iter().enumerate() {
        sheet.push_str(&format!("<row r=\"{}\">", row_index + 1));
        for (col_index, value) in values.iter().enumerate() {
            if col_index >= 26 {
                return Err(CoreError::InvalidInput(
                    "XLSX-viennissä voi olla enintään 26 saraketta".into(),
                ));
            }
            let column = char::from(b'A' + col_index as u8);
            let reference = format!("{column}{}", row_index + 1);
            if row_index > 0
                && matches!(col_index, 2 | 3 | 8 | 9 | 10)
                && !value.is_empty()
                && value.parse::<i64>().is_ok()
            {
                sheet.push_str(&format!("<c r=\"{reference}\"><v>{value}</v></c>"));
            } else {
                sheet.push_str(&format!(
                    "<c r=\"{reference}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
                    xml_text(value)
                ));
            }
        }
        sheet.push_str("</row>");
    }
    sheet.push_str("</sheetData></worksheet>");
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let parts = [
        (
            "[Content_Types].xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/worksheets/sheet1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/></Types>",
        ),
        (
            "_rels/.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>",
        ),
        (
            "xl/workbook.xml",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Tulokset\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>",
        ),
        (
            "xl/_rels/workbook.xml.rels",
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>",
        ),
    ];
    for (name, content) in parts {
        zip.start_file(name, options)
            .map_err(|error| CoreError::InvalidInput(format!("XLSX-rakenne: {error}")))?;
        zip.write_all(content.as_bytes())
            .map_err(|error| CoreError::InvalidInput(format!("XLSX-kirjoitus: {error}")))?;
    }
    zip.start_file("xl/worksheets/sheet1.xml", options)
        .map_err(|error| CoreError::InvalidInput(format!("XLSX-arkki: {error}")))?;
    zip.write_all(sheet.as_bytes())
        .map_err(|error| CoreError::InvalidInput(format!("XLSX-arkki: {error}")))?;
    let cursor = zip
        .finish()
        .map_err(|error| CoreError::InvalidInput(format!("XLSX-paketti: {error}")))?;
    Ok(cursor.into_inner())
}

pub fn ranking_xlsx(archive: &Archive, ranked: &RankingReport) -> Result<Vec<u8>, CoreError> {
    publication_ranking(archive, ranked)?;
    xlsx_from_csv(&ranking_csv(archive, ranked))
}

/// Luo uuden UTF-8-tiedoston. Myös olemassa oleva symbolinen linkki estää
/// tallennuksen; olemassa olevaa tiedostoa ei avata kirjoitettavaksi.
pub fn save_new(path: &Path, html: &str) -> Result<(), CoreError> {
    save_new_bytes(path, html.as_bytes())
}

/// Create only a new output file, including binary XLSX files.
pub fn save_new_bytes(path: &Path, bytes: &[u8]) -> Result<(), CoreError> {
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
    file.write_all(bytes).map_err(|error| {
        CoreError::InvalidInput(format!(
            "Raportin kirjoitus epäonnistui; keskeneräinen tiedosto voi jäädä polkuun {}: {}",
            path.display(),
            crate::io_message(&error)
        ))
    })
}

/// Uuden kilpailun asemakohtainen raportti käyttää samaa toimitsijalaskentaa
/// ja tallennettua tarkastustulosta kuin käyttöliittymä. Ei muuta kilpailua.
pub fn project_results_html(project: &crate::native::Project) -> Result<String, crate::CoreError> {
    let archive = crate::application::calculated_project(project)?;
    let mut html = document("Tulokset ja asemarivit", &project.kilpailu.nimi);
    html.push_str("<section class=\"notice\"><p>Käsikellot ja pistekorjaukset säilyvät tallennettuina. Keskeneräisen tuloksen kokonaistulosta, maaliaikaa tai käytettyä aikaa ei julkaista.</p></section>\n");
    warnings(&mut html, &archive.warnings);
    for driver in &archive.competitors {
        let snapshot = project
            .kilpailijat
            .iter()
            .find(|d| d.numero == driver.number)
            .and_then(|d| d.tallennettu_tulos.clone())
            .map_or_else(
                || crate::application::capture_project_result(project, driver.number),
                Ok,
            );
        let valid = snapshot.as_ref().is_ok_and(|s| {
            crate::application::validate_project_result(project, driver.number, s).is_ok()
        });
        let snapshot = snapshot.as_ref().ok().filter(|_| valid);
        let complete = snapshot.is_some_and(|s| s.can_approve());
        let approved = archive.has_approved_result(driver.number);
        let reason = driver.header_fields.get(29).map_or("", String::as_str);
        let field = |index: usize| driver.header_fields.get(index).map_or("—", String::as_str);
        html.push_str(&format!(
            "<section><h2>Nro {} — {} / {}</h2><p>Luokka: {}. Tila: <strong>{}</strong>{}</p>\n",
            driver.number,
            escape(&driver.driver),
            escape(&driver.navigator),
            escape(&driver.class_code),
            escape(&driver.status),
            if reason.is_empty() {
                String::new()
            } else {
                format!(". Syy: {}", escape(reason))
            }
        ));
        if archive.has_confirmed_interruption(driver.number) {
            html.push_str("<p>Vahvistettu keskeytys. Maaliaika: — · Käytetty aika: — · Kokonaistulos: —</p><p>Vahvistus koskee keskeytyspäätöstä ja sen perustelua. Aikoja tai pisteitä ei vahvistettu.</p></section>\n");
            continue;
        }
        let start = format!(
            "{:02}:{:02}:{:02}",
            field(10).parse::<i64>().unwrap_or(0),
            field(11).parse::<i64>().unwrap_or(0),
            field(12).parse::<i64>().unwrap_or(0)
        );
        let duration = if approved {
            format!(
                "{:02}:{:02}:{:02}",
                field(13).parse::<i64>().unwrap_or(0),
                field(14).parse::<i64>().unwrap_or(0),
                field(15).parse::<i64>().unwrap_or(0)
            )
        } else {
            "—".into()
        };
        let finish = if approved {
            format!(
                "{:02}:{:02}:{:02}",
                field(16).parse::<i64>().unwrap_or(0),
                field(17).parse::<i64>().unwrap_or(0),
                field(18).parse::<i64>().unwrap_or(0)
            )
        } else {
            "—".into()
        };
        html.push_str(&format!(
            "<p>Lähtöaika: {start} · Käytetty aika: {duration} · Maaliaika: {finish}</p>\n"
        ));
        if approved {
            html.push_str(&format!(
                "<p class=\"totals\">Kokonaistulos: {} p</p><p>{}</p>\n",
                driver.total_penalties, "Hyväksytty tulos."
            ));
        } else {
            html.push_str(
                "<p class=\"notice warning\">Hyväksymätön esikatselu. Kokonaistulos: —</p>\n",
            );
            if !complete {
                html.push_str("<p>Tulos odottaa tarkastusta.</p>\n");
            }
        }
        html.push_str(&format!("<h3>Viisi pisteosaa</h3><p>Aikavirheet: {} p · Asemien muut virheet: {} p · Puuttuvat RT: {} kpl / {} p · Muut virheet: {} p · Ylimääräiset RT: {} kpl / {} p</p>\n",escape(field(19)),escape(field(20)),escape(field(21)),escape(field(22)),escape(field(25)),escape(field(23)),escape(field(24))));
        if !approved {
            html.push_str("<p>Näytetyt pisteosat ovat tämänhetkiset tallennetut tai lasketut arvot; tarkastus voi muuttaa niitä.</p>\n");
        }
        if driver.results.is_empty() {
            html.push_str("<p>Asematulosta ei ole vielä muodostettu.</p></section>\n");
            continue;
        }
        let pending = snapshot.map_or_else(Vec::new, |s| s.pending_manual_clocks());
        let clock_changed = snapshot.is_some_and(|s| s.kasikellot_muuttuneet);
        let invalid_times = snapshot.is_some_and(|s| !s.virheelliset_ajat.is_empty());
        if !pending.is_empty() {
            html.push_str(&format!("<p>Käsikelloja puuttuu: {}.</p>\n", pending.len()));
        }
        if clock_changed {
            html.push_str("<p>Käsikelloa on muutettu. Tutki laskee toteutuneet ajat ja pisteet uudelleen.</p>\n");
        }
        html.push_str("<div class=\"table-wrap\"><table><thead><tr><th scope=\"col\">Järjestys</th><th scope=\"col\">Tyyppi</th><th scope=\"col\">Nimi</th><th scope=\"col\">Emit</th><th scope=\"col\">Luettu</th><th scope=\"col\">Kello</th><th scope=\"col\">Tavoite (s)</th><th scope=\"col\">Toteutunut (s)</th><th scope=\"col\">Aikavirhe (p)</th><th scope=\"col\">Muu virhe (p)</th><th scope=\"col\">Yhteensä (p)</th><th scope=\"col\">Huomautus</th></tr></thead><tbody>\n");
        let mut reference_known = true;
        for row in &driver.results {
            let clock_known = row.sequence != 0
                && (row.station_type == "L"
                    || (row.expected_code != 0 && row.received_code == row.expected_code)
                    || (row.expected_code == 0 && !pending.contains(&row.output_index)));
            let actual_known = clock_known && reference_known && !clock_changed && !invalid_times;
            let actual = if actual_known {
                row.actual_seconds.to_string()
            } else {
                "—".into()
            };
            let clock = if clock_known {
                format!(
                    "{:02}:{:02}:{:02}",
                    row.clock[0], row.clock[1], row.clock[2]
                )
            } else {
                "—".into()
            };
            if matches!(row.station_type.as_str(), "JAT" | "ULA") {
                if clock_known {
                    reference_known = true;
                } else if row.expected_code == 0 {
                    reference_known = false;
                }
            }
            let sequence = if row.sequence == 0 {
                "—".into()
            } else {
                row.sequence.to_string()
            };
            let expected = if row.sequence == 0 {
                "—".into()
            } else {
                row.expected_code.to_string()
            };
            let received = if row.received_code == 0 {
                "—".into()
            } else {
                row.received_code.to_string()
            };
            html.push_str(&format!("<tr><td>{sequence}</td><td>{}</td><td>{}</td><td>{expected}</td><td>{received}</td><td>{clock}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{actual}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td class=\"numeric\">{}</td><td>{}</td></tr>\n",escape(&row.station_type),escape(&row.name),row.target_seconds,row.signed_time_penalty,row.other_penalty,row.total_penalty,escape(&row.note)));
        }
        html.push_str("</tbody></table></div></section>\n");
    }
    Ok(finish(html))
}
