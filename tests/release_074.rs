use as2026::{
    application,
    native::{CombinedClass, Project},
    printing::{self, Kind},
};

fn project() -> Project {
    let mut p = Project::from_toml(include_str!("../fixtures/native-demo/kilpailu.toml")).unwrap();
    let mut other = p.luokat[0].clone();
    other.tunnus = "B".into();
    other.nimi = "Toinen luokka".into();
    p.luokat.push(other);
    p.yhdistetyt_luokat.push(CombinedClass {
        nimi: "Yhteistulos <SM>".into(),
        luokat: vec!["A".into(), "B".into()],
        nousupisteet: 0,
        sarjapistetapa: 0,
    });
    p
}

#[test]
fn selected_class_order_is_kept_for_both_report_kinds_and_orientations() {
    let archive = application::calculated_project(&project()).unwrap();
    let codes = ["Y1: Yhteistulos <SM>", "B", "A"].map(String::from);
    for kind in [Kind::Classes, Kind::Stations] {
        for landscape in [false, true] {
            let document = printing::classes(&archive, &codes, kind, landscape, false).unwrap();
            assert_eq!(document.pages.len(), 3);
            assert_eq!(document.pages[0].texts[1].value, "luokka: Yhteistulos <SM>");
            assert_eq!(document.pages[1].texts[1].value, "luokka: Toinen luokka");
        }
    }
}

#[test]
fn kiti_destination_uses_template_stem_and_folder() {
    let path = std::path::Path::new("tulospohjat/SM syksy 2026.XLSX");
    assert_eq!(
        as2026::kiti::destination(path).unwrap(),
        std::path::Path::new("tulospohjat/SM syksy 2026-valmis.xlsx")
    );
    assert!(as2026::kiti::destination(std::path::Path::new("/")).is_err());
    let p = project();
    assert_eq!(
        as2026::kiti::suggested_base_group(&p, &p.luokat[0].nimi),
        "A"
    );
}

#[cfg(feature = "desktop")]
#[test]
fn live_combined_heading_hides_internal_identifier_and_escapes_name() {
    let p = project();
    let code = "Y1: Yhteistulos <SM>";
    assert!(
        as2026::live_pages::class_choices(&p)
            .unwrap()
            .contains(&code.into())
    );
    let pages = as2026::live_pages::pages(
        &p,
        &as2026::live_pages::Settings {
            base: "tulokset.html".into(),
            classes: vec![code.into()],
        },
    )
    .unwrap();
    assert_eq!(pages.len(), 1);
    assert!(
        pages[0]
            .1
            .contains("<h2>Luokka: Yhteistulos &lt;SM&gt;</h2>")
    );
    assert!(pages[0].1.contains("· Yhteistulos &lt;SM&gt;</title>"));
    assert!(!pages[0].1.contains("Y1:"));
}

#[cfg(feature = "gui")]
#[test]
fn pdf_logo_stays_inside_ten_mm_margins_with_portrait_landscape_and_wide_logos() {
    use std::io::{Cursor, Read};
    let archive = application::calculated_project(&project()).unwrap();
    for landscape in [false, true] {
        for (w, h) in [(80, 40), (800, 10), (10, 800)] {
            let doc = printing::classes(&archive, &["A".into()], Kind::Classes, landscape, false)
                .unwrap();
            let image = image::RgbaImage::from_pixel(w, h, image::Rgba([20, 80, 160, 255]));
            let mut png = Cursor::new(Vec::new());
            image.write_to(&mut png, image::ImageFormat::Png).unwrap();
            let pdf = as2026::export::pdf(&doc, &png.into_inner()).unwrap();
            let mut content = None;
            for (start, _) in pdf
                .windows(7)
                .enumerate()
                .filter(|(_, b)| *b == b"stream\n")
            {
                let tail = &pdf[start + 7..];
                let Some(end) = tail.windows(9).position(|s| s == b"endstream") else {
                    continue;
                };
                let mut data = String::new();
                if flate2::read::ZlibDecoder::new(&tail[..end])
                    .read_to_string(&mut data)
                    .is_ok()
                    && data.contains("/Logo Do")
                {
                    content = Some(data);
                    break;
                }
            }
            let content = content.expect("Logo drawing command");
            let command = content.lines().find(|l| l.contains("/Logo Do")).unwrap();
            let numbers = command
                .split_whitespace()
                .skip(1)
                .take(6)
                .map(|s| s.parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            let mm = 72. / 25.4;
            let height = if landscape { 210. } else { 297. };
            assert!((numbers[4] / mm - 10.).abs() < 0.001);
            assert!((height - (numbers[5] + numbers[3]) / mm - 10.).abs() < 0.001);
            assert!(numbers[0] / mm <= 60.001);
            assert!(numbers[3] / mm <= 20.001);
            assert!((numbers[0] / numbers[3] - f64::from(w) / f64::from(h)).abs() < 0.001);
            if let Some(folder) = std::env::var_os("AS2026_RELEASE_QA_DIR") {
                let folder = std::path::PathBuf::from(folder);
                std::fs::create_dir_all(&folder).unwrap();
                std::fs::write(
                    folder.join(format!("logo-074-{landscape}-{w}x{h}.pdf")),
                    pdf,
                )
                .unwrap();
            }
        }
    }
}
