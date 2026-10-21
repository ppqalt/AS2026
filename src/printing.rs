//! Tulosteiden yhteinen A4-sivumalli. Tulostus ei suorita kilpailuaineiston koodia.
use crate::{CoreError, application, legacy::Archive, native::Project, ranking};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Text {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub size: f64,
    pub bold: bool,
    pub value: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Line {
    pub x: f64,
    pub y: f64,
    pub x2: f64,
    pub y2: f64,
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub texts: Vec<Text>,
    pub lines: Vec<Line>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub title: String,
    pub landscape: bool,
    pub pages: Vec<Page>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Classes,
    Stations,
}

fn invalid(text: &str) -> CoreError {
    CoreError::InvalidInput(text.into())
}
fn field(driver: &crate::legacy::Competitor, index: usize) -> String {
    driver.header_fields.get(index).cloned().unwrap_or_default()
}
fn duration(seconds: i64) -> String {
    let n = seconds.unsigned_abs();
    format!(
        "{}{:02}:{:02}:{:02}",
        if seconds < 0 { "-" } else { "" },
        n / 3600,
        n / 60 % 60,
        n % 60
    )
}
fn signed(seconds: i64) -> String {
    let n = seconds.unsigned_abs();
    format!(
        "{}{:02}:{:02}",
        if seconds >= 0 { "+" } else { "-" },
        n / 60,
        n % 60
    )
}

pub fn class_label(code: &str) -> &str {
    code.split_once(": ")
        .filter(|(prefix, _)| {
            prefix
                .strip_prefix('Y')
                .is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()))
        })
        .map_or(code, |(_, name)| name)
}
impl Page {
    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        size: f64,
        bold: bool,
        value: impl Into<String>,
    ) {
        self.texts.push(Text {
            x,
            y,
            width,
            height,
            size,
            bold,
            value: value.into(),
        });
    }
    fn line(&mut self, x: f64, y: f64, x2: f64, y2: f64) {
        self.lines.push(Line { x, y, x2, y2 });
    }
}
fn heading(page: &mut Page, name: &str, title: &str, landscape: bool) {
    let width = if landscape { 277. } else { 190. };
    page.text(10., 10., width, 8., 13., true, name);
    page.text(10., 20., width, 7., 11., true, title);
    page.text(
        10.,
        28.,
        width,
        5.,
        8.,
        false,
        format!(
            "Tulostusaika: {}",
            chrono::Local::now().format("%d.%m.%Y %H:%M:%S")
        ),
    );
}
/// Rivit ja sarakkeet sovitetaan saman sivun sisään. Ilman sovitusta pitkät
/// luokat jatkuvat seuraavalla sivulla toistuvine otsikoineen.
#[allow(clippy::too_many_arguments)]
fn table(
    page: &mut Page,
    top: f64,
    width: f64,
    bottom: f64,
    headers: &[String],
    weights: &[f64],
    rows: &[Vec<String>],
    fit: bool,
) {
    let height = if fit {
        5_f64.min((bottom - top) / (rows.len() + 1) as f64)
    } else {
        5.
    };
    let size = 8.5_f64.min(height * 1.7);
    let total: f64 = weights.iter().sum();
    let mut y = top;
    for (row_index, row) in std::iter::once(headers)
        .chain(rows.iter().map(Vec::as_slice))
        .enumerate()
    {
        let mut x = 10.;
        for (index, value) in row.iter().enumerate() {
            let w = width * weights[index] / total;
            // Pitkä nimi sovitetaan soluun, ei leikata seuraavan sarakkeen päälle.
            let cell_size =
                size.min((w - 1.).max(0.2) / value.chars().count().max(1) as f64 / 0.20);
            page.text(
                x + 0.5,
                y + 0.3,
                (w - 1.).max(0.2),
                height - 0.5,
                cell_size,
                row_index == 0,
                value,
            );
            page.line(x, y, x, y + height);
            x += w;
        }
        page.line(x, y, x, y + height);
        page.line(10., y, 10. + width, y);
        y += height;
    }
    page.line(10., y, 10. + width, y);
}

pub fn competitor(
    project: &Project,
    number: u32,
    class_label: Option<&str>,
) -> Result<Document, CoreError> {
    let archive = application::calculated_project(project)?;
    let driver = archive
        .competitors
        .iter()
        .find(|d| d.number == number)
        .ok_or_else(|| invalid("Kilpailijaa ei löydy"))?;
    if !archive.can_publish_competitor(driver) {
        return Err(invalid(
            "Täydennä kilpailijan ajat ja virhepisteet ennen tulostusta",
        ));
    }
    let source = project
        .kilpailijat
        .iter()
        .find(|d| d.numero == number)
        .unwrap();
    let result = source.tallennettu_tulos.as_ref();
    let mut page = Page::default();
    page.text(10., 8., 130., 7., 13., true, &project.kilpailu.nimi);
    let info = [
        ("Kilpailunumero", number.to_string()),
        (
            "Luokka",
            class_label.unwrap_or(&driver.class_code).to_owned(),
        ),
        ("Kuljettaja", driver.driver.clone()),
        ("Kartanlukija", driver.navigator.clone()),
        (
            "Seurat",
            format!(
                "{} / {}",
                source.kuljettajan_seura, source.kartanlukijan_seura
            ),
        ),
        ("Auto", source.auto.clone()),
        ("Emit", driver.emit_card.clone()),
    ];
    page.text(
        150.,
        8.,
        50.,
        15.,
        33.,
        true,
        if archive.has_approved_result(number) && driver.status == "TRK" {
            driver.total_penalties.to_string()
        } else {
            driver.status.clone()
        },
    );
    for (i, (label, value)) in info.iter().enumerate() {
        let x = 10. + (i % 2) as f64 * 67.;
        let y = 18. + (i / 2) as f64 * 9.;
        page.text(x, y, 65., 3., 8., false, *label);
        page.text(x, y + 3., 65., 6., 11., true, value);
    }
    let summary = [
        ("Lähtöaika", source.lahtoaika.clone()),
        (
            "Maaliaika",
            result.map_or("-".into(), |r| duration(r.maalikello_sekuntia)),
        ),
        (
            "Käytetty",
            result.map_or("-".into(), |r| duration(r.kesto_sekuntia)),
        ),
        (
            "Kokonaismyöhä",
            result.map_or("-".into(), |r| {
                signed(
                    r.rivit
                        .iter()
                        .filter(|s| matches!(s.station_type.as_str(), "JAT" | "M"))
                        .filter_map(|s| {
                            s.section_text
                                .trim()
                                .parse::<i64>()
                                .ok()
                                .map(|m| m * 60 - s.actual_seconds)
                        })
                        .sum(),
                )
            }),
        ),
        (
            "AT",
            result.map_or("-".into(), |r| r.aikavirheet.to_string()),
        ),
        (
            "ATp/ATy",
            result.map_or("-".into(), |r| r.asemien_muut_virheet.to_string()),
        ),
        (
            "RT",
            (source.lisavirheet.puuttuvat_rt_pisteet + source.lisavirheet.ylimaaraiset_rt_pisteet)
                .to_string(),
        ),
        ("Muut", source.lisavirheet.muut_virhepisteet.to_string()),
    ];
    for (i, (label, value)) in summary.iter().enumerate() {
        let x = if i < 4 { 10. } else { 145. };
        let y = if i < 4 {
            56. + i as f64 * 5.
        } else {
            25. + (i - 4) as f64 * 9.
        };
        let size = if i < 4 { 9. } else { 18. };
        page.text(x, y, 30., 8., size, false, *label);
        page.text(
            x + 31.,
            y,
            if i < 4 { 70. } else { 24. },
            8.,
            size,
            true,
            value,
        );
    }
    let headers = [
        "Nimi",
        "Emit",
        "L-aika",
        "Ihanne",
        "Käytetty",
        "AT",
        "ATp/ATy",
        "Myöhä",
    ]
    .map(String::from);
    let mut rows = Vec::new();
    if let Some(result) = result {
        for r in &result.rivit {
            let extra = r.sequence == 0;
            let known = !extra
                && (r.station_type == "L"
                    || if r.expected_code == 0 {
                        result.kasin_annetut_kellot.contains(&r.output_index)
                    } else {
                        r.received_code == r.expected_code
                    });
            if r.station_type != "ULA" {
                rows.push(vec![
                    if extra {
                        "AT liikaa".into()
                    } else {
                        r.name.clone()
                    },
                    if extra {
                        r.received_code.to_string()
                    } else {
                        r.expected_code.to_string()
                    },
                    if known {
                        duration(
                            (r.clock[0] * 3600 + r.clock[1] * 60 + r.clock[2] - r.actual_seconds)
                                .rem_euclid(86400),
                        )
                    } else {
                        String::new()
                    },
                    if extra {
                        String::new()
                    } else {
                        duration(r.target_seconds)
                    },
                    if known {
                        duration(r.actual_seconds)
                    } else {
                        String::new()
                    },
                    if known {
                        signed(r.target_seconds - r.actual_seconds)
                    } else {
                        String::new()
                    },
                    format!("{:+}", r.other_penalty),
                    if known && matches!(r.station_type.as_str(), "JAT" | "M") {
                        r.section_text
                            .trim()
                            .parse::<i64>()
                            .map(|m| signed(m * 60 - r.actual_seconds))
                            .unwrap_or_default()
                    } else {
                        String::new()
                    },
                ]);
            }
        }
    }
    table(
        &mut page,
        80.,
        190.,
        280.,
        &headers,
        &[1.4, 0.7, 1.2, 1.1, 1.1, 1.0, 0.9, 1.2],
        &rows,
        true,
    );
    let reason = source
        .tila
        .as_ref()
        .map(|s| s.syy.as_str())
        .or_else(|| result.map(|r| r.syy.as_str()))
        .unwrap_or("");
    if !reason.is_empty() {
        page.text(10., 282., 190., 5., 8., false, format!("Syy: {reason}"));
    }
    Ok(Document {
        title: format!("AS2026 kilpailija {number}"),
        landscape: false,
        pages: vec![page],
    })
}

pub fn class_choices(archive: &Archive) -> Result<Vec<String>, CoreError> {
    Ok(
        application::rank_stored(archive, ranking::TiePolicy::default())?
            .groups
            .into_iter()
            .map(|g| g.code)
            .collect(),
    )
}
pub fn classes(
    archive: &Archive,
    selected: &[String],
    kind: Kind,
    landscape: bool,
    fit: bool,
) -> Result<Document, CoreError> {
    if selected.is_empty() {
        return Err(invalid("Valitse vähintään yksi tulostettava luokka"));
    }
    let publication = archive;
    let ranked = application::rank_stored(archive, ranking::TiePolicy::default())?;
    if selected
        .iter()
        .any(|code| !ranked.groups.iter().any(|g| &g.code == code))
    {
        return Err(invalid("Valittua luokkaa ei löydy"));
    }
    let name = archive.basic.identity.get(1).cloned().unwrap_or_default();
    let mut document = Document {
        title: "AS2026 luokkatulokset".into(),
        landscape,
        pages: Vec::new(),
    };
    for group in ranked.groups.iter().filter(|g| selected.contains(&g.code)) {
        let mut drivers = group
            .entries
            .iter()
            .filter_map(|entry| {
                publication
                    .competitors
                    .iter()
                    .find(|d| i64::from(d.number) == entry.number)
                    .map(|d| (Some(entry), d))
            })
            .collect::<Vec<_>>();
        let member_codes = if group.kind == ranking::GroupKind::Combined {
            archive
                .basic
                .combined_classes
                .iter()
                .find(|g| {
                    g.first()
                        .zip(g.get(1))
                        .is_some_and(|(i, name)| format!("Y{i}: {}", name.trim()) == group.code)
                })
                .map(|g| {
                    archive
                        .basic
                        .classes
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| g.get(4 + i).is_some_and(|v| v.trim() == "1"))
                        .map(|(_, c)| c.code.as_str())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        } else {
            vec![group.code.as_str()]
        };
        for driver in &publication.competitors {
            if member_codes.contains(&driver.class_code.as_str())
                && !drivers.iter().any(|(_, d)| d.number == driver.number)
            {
                drivers.push((None, driver));
            }
        }
        drivers.sort_by_key(|(entry, d)| {
            let classified = entry.is_some_and(|e| e.status == ranking::Status::Classified);
            let category = if classified {
                0
            } else {
                match d.status.as_str() {
                    "KESK" => 2,
                    "HYL" | "SULJ" => 3,
                    "PER" | "EILÄ" => 4,
                    _ => 1,
                }
            };
            (
                category,
                if classified {
                    entry.unwrap().primary_position
                } else {
                    d.number
                },
            )
        });
        let mut stations = Vec::new();
        if kind == Kind::Stations {
            for (_, d) in &drivers {
                for r in &d.results {
                    let key = (r.sequence, r.station_type.clone(), r.name.clone());
                    if r.sequence != 0
                        && matches!(r.station_type.as_str(), "AT" | "JAT" | "M")
                        && !stations.contains(&key)
                    {
                        stations.push(key);
                    }
                }
            }
            stations.sort_by_key(|s| s.0);
        }
        let mut headers = ["Sij", "Nro", "Kuljettaja", "Kartanlukija"]
            .map(String::from)
            .to_vec();
        let mut weights = vec![0.5, 0.6, 2.4, 2.4];
        if kind == Kind::Classes {
            headers.extend(["Seurat", "Auto", "Tulos", "Np", "Sp", "Syy"].map(String::from));
            weights.extend([2.1, 1.8, 0.9, 0.5, 0.5, 1.6]);
        } else {
            headers.extend(stations.iter().map(|s| s.2.clone()));
            weights.extend(vec![0.85; stations.len()]);
            headers.extend(["AT-yht", "ATp/ATy", "RTp", "Muut", "Tulos", "Syy"].map(String::from));
            weights.extend([1., 1., 1., 1., 1., 1.5]);
        }
        let rows = drivers
            .iter()
            .map(|(entry, d)| {
                let classified = entry.is_some_and(|e| e.status == ranking::Status::Classified);
                let driving = !classified
                    && !matches!(d.status.as_str(), "KESK" | "HYL" | "SULJ" | "PER" | "EILÄ");
                let mut row = vec![
                    if classified {
                        entry.unwrap().primary_position.to_string()
                    } else {
                        String::new()
                    },
                    d.number.to_string(),
                    d.driver.clone(),
                    d.navigator.clone(),
                ];
                let total = if classified {
                    d.total_penalties.to_string()
                } else if driving {
                    String::new()
                } else {
                    d.status.clone()
                };
                if kind == Kind::Classes {
                    row.extend([
                        format!("{}/{}", field(d, 7), field(d, 8)),
                        field(d, 9),
                        total,
                        if group.np_enabled && classified {
                            entry.unwrap().table_points.to_string()
                        } else {
                            String::new()
                        },
                        if group.series_method != 0 && classified {
                            entry.unwrap().series_points.to_string()
                        } else {
                            String::new()
                        },
                        if driving { "AJAA".into() } else { field(d, 29) },
                    ]);
                } else {
                    row.extend(stations.iter().map(|key| {
                        d.results
                            .iter()
                            .find(|r| {
                                (r.sequence, &r.station_type, &r.name) == (key.0, &key.1, &key.2)
                            })
                            .filter(|_| classified)
                            .map_or(String::new(), |r| format!("{:+}", r.signed_time_penalty))
                    }));
                    row.extend([
                        if classified {
                            d.results
                                .iter()
                                .map(|r| r.signed_time_penalty.abs())
                                .sum::<i64>()
                                .to_string()
                        } else {
                            String::new()
                        },
                        if classified {
                            d.results
                                .iter()
                                .map(|r| r.other_penalty)
                                .sum::<i64>()
                                .to_string()
                        } else {
                            String::new()
                        },
                        if classified {
                            (field(d, 22).parse::<i64>().unwrap_or(0)
                                + field(d, 24).parse::<i64>().unwrap_or(0))
                            .to_string()
                        } else {
                            String::new()
                        },
                        if classified {
                            field(d, 25)
                        } else {
                            String::new()
                        },
                        total,
                        if driving { "AJAA".into() } else { field(d, 29) },
                    ]);
                }
                row
            })
            .collect::<Vec<_>>();
        let bottom = if landscape { 194. } else { 281. };
        let width = if landscape { 277. } else { 190. };
        let per_page = if fit {
            rows.len().max(1)
        } else {
            ((bottom - 38.) / 5.) as usize - 1
        };
        let chunks = if rows.is_empty() {
            vec![&rows[..]]
        } else {
            rows.chunks(per_page).collect()
        };
        for chunk in chunks {
            let mut page = Page::default();
            heading(
                &mut page,
                &name,
                &format!(
                    "{} - luokka {}",
                    if kind == Kind::Stations {
                        "Asemakohtaiset tulokset"
                    } else {
                        "Tulokset"
                    },
                    class_label(&group.code),
                ),
                landscape,
            );
            table(
                &mut page, 38., width, bottom, &headers, &weights, chunk, fit,
            );
            let stats = &group.statistics;
            page.text(
                10.,
                bottom + 1.,
                width,
                5.,
                8.,
                false,
                format!(
                    "Kpl {} | tulos {} | hylätty {} | kesk/suljettu {} | perunut {} | avoin {}",
                    stats.all_entries,
                    stats.classified,
                    stats.disqualified,
                    stats.interrupted_or_excluded,
                    stats.cancelled,
                    stats.unresolved
                ),
            );
            document.pages.push(page);
        }
    }
    Ok(document)
}

/// PostScript käyttää samaa sivumallia kuin Windowsin GDI-tulostus.
pub fn postscript(document: &Document) -> String {
    fn literal(value: &str) -> String {
        value
            .chars()
            .map(|c| match c {
                '(' | ')' | '\\' => format!("\\{c}"),
                c if (32..=126).contains(&(c as u32)) => c.to_string(),
                c if (160..=255).contains(&(c as u32)) => format!("\\{:03o}", c as u32),
                _ => " ".into(),
            })
            .collect()
    }
    let (w, h) = if document.landscape {
        (297., 210.)
    } else {
        (210., 297.)
    };
    let mut out = format!(
        "%!PS-Adobe-3.0\n%%Pages: {}\n<< /PageSize [{} {}] >> setpagedevice\n/F /Helvetica findfont dup length dict begin {{1 index /FID ne {{def}} {{pop pop}} ifelse}} forall /Encoding ISOLatin1Encoding def currentdict end definefont pop\n/FB /Helvetica-Bold findfont dup length dict begin {{1 index /FID ne {{def}} {{pop pop}} ifelse}} forall /Encoding ISOLatin1Encoding def currentdict end definefont pop\n",
        document.pages.len(),
        w * 72. / 25.4,
        h * 72. / 25.4
    );
    for (index, page) in document.pages.iter().enumerate() {
        out.push_str(&format!("%%Page: {} {}\ngsave 2.83464567 2.83464567 scale 0 {h} translate 1 -1 scale 0.15 setlinewidth\n",index+1,index+1));
        for l in &page.lines {
            out.push_str(&format!(
                "newpath {} {} moveto {} {} lineto stroke\n",
                l.x, l.y, l.x2, l.y2
            ));
        }
        for t in &page.texts {
            out.push_str(&format!("gsave newpath {} {} {} {} rectclip {} {} translate 1 -1 scale /{} findfont {} scalefont setfont 0 {} moveto ({}) show grestore\n",t.x,t.y,t.width,t.height,t.x,t.y,if t.bold {"FB"} else {"F"},t.size*25.4/72.,-t.size*25.4/72.,literal(&t.value)));
        }
        out.push_str("grestore showpage\n");
    }
    out.push_str("%%EOF\n");
    out
}

#[cfg(all(feature = "desktop", windows))]
#[path = "printing_windows.rs"]
mod windows;

#[cfg(all(feature = "desktop", windows))]
pub fn printers() -> Result<Vec<String>, String> {
    windows::printers()
}

#[cfg(all(feature = "gui", windows))]
pub fn preview_png(
    document: &Document,
    directory: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, String> {
    windows::preview_png(document, directory)
}

#[cfg(all(feature = "desktop", not(windows)))]
pub fn printers() -> Result<Vec<String>, String> {
    use std::process::Command;
    let output = Command::new("lpstat").args(["-a"]).output();
    let output = output.map_err(|e| {
        format!(
            "Tulostinluetteloa ei voitu hakea: {}",
            crate::io_message(&e)
        )
    })?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().into());
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let mut names = text
        .lines()
        .filter_map(|s| s.split_whitespace().next())
        .map(String::from)
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    Ok(names)
}

#[cfg(all(feature = "desktop", windows))]
pub fn submit(document: &Document, printer: &str) -> Result<(), String> {
    windows::submit(document, printer)
}

#[cfg(all(feature = "desktop", not(windows)))]
pub fn submit(document: &Document, printer: &str) -> Result<(), String> {
    use std::{fs, process::Command};
    struct JobDirectory(std::path::PathBuf);
    impl Drop for JobDirectory {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let directory = JobDirectory(
        std::env::temp_dir().join(format!("as2026-print-{}-{nonce}", std::process::id())),
    );
    fs::create_dir(&directory.0).map_err(|e| crate::io_message(&e))?;
    let output = {
        let path = directory.0.join("tulokset.ps");
        fs::write(&path, postscript(document)).map_err(|e| crate::io_message(&e))?;
        let mut command = Command::new("lp");
        if !printer.is_empty() {
            command.arg("-d").arg(printer);
        }
        command
            .args(["-o", "media=A4", "-o", "fit-to-page", "-t", &document.title])
            .arg(&path)
            .output()
    };
    let output =
        output.map_err(|e| format!("Tulostusta ei voitu käynnistää: {}", crate::io_message(&e)))?;
    if !output.status.success() {
        return Err(format!(
            "Tulostus epäonnistui: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}
