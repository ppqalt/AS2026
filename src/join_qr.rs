//! QR muodostetaan paikallisesti; sisältönä on vain puhelimen liittymislinkki.
use qrcodegen::{QrCode, QrCodeEcc};
pub struct Bitmap {
    pub side: u32,
    pub rgba: Vec<u8>,
}

pub fn document(
    name: &str,
    link: &str,
    join_code: &str,
) -> Result<crate::printing::Document, String> {
    use crate::printing::{Document, FilledRectangle, Page, Text};
    let code = QrCode::encode_text(link, QrCodeEcc::Medium)
        .map_err(|_| "Liittymislinkki on liian pitkä QR-koodiksi")?;
    let mut page = Page::default();
    for (y, size, bold, value) in [
        (12., 18., true, "Liity kilpailuun QR-koodilla".to_owned()),
        (25., 13., true, name.to_owned()),
        (220., 15., true, format!("Liittymiskoodi: {join_code}")),
        (234., 11., false, "Skannaa QR-koodi puhelimen kameralla. Anna tarkastajan nimi ja valitse Liity kilpailuun.".to_owned()),
        (253., 8., false, link.to_owned()),
    ] {
        page.texts.push(Text { x: 15., y, width: 180., height: if y == 253. { 28. } else { 16. }, size, bold, value });
    }
    let cell = 160. / f64::from(code.size() + 8);
    for y in 0..code.size() {
        for x in 0..code.size() {
            if code.get_module(x, y) {
                page.rectangles.push(FilledRectangle {
                    x: 25. + f64::from(x + 4) * cell,
                    y: 48. + f64::from(y + 4) * cell,
                    width: cell,
                    height: cell,
                });
            }
        }
    }
    Ok(Document {
        title: format!("AS tulkin liittymiskoodi – {name}"),
        landscape: false,
        pages: vec![page],
    })
}
pub fn encode(link: &str) -> Result<Bitmap, String> {
    let code = QrCode::encode_text(link, QrCodeEcc::Medium)
        .map_err(|_| "Liittymislinkki on liian pitkä QR-koodiksi")?;
    let scale = 6;
    let side = (code.size() + 8) * scale;
    let mut rgba = vec![255; (side * side * 4) as usize];
    for y in 0..side {
        for x in 0..side {
            if code.get_module(x / scale - 4, y / scale - 4) {
                let i = ((y * side + x) * 4) as usize;
                rgba[i..i + 3].fill(0);
            }
        }
    }
    Ok(Bitmap {
        side: side as u32,
        rgba,
    })
}
#[cfg(test)]
mod tests {
    #[cfg(windows)]
    #[test]
    fn printed_join_qr_decodes_from_the_same_gdi_renderer_as_the_printer() {
        let link =
            "https://example.fi/as-tulkki/#join=ABCDEF123456&site=https%3A%2F%2Fexample.fi%2F";
        let document = super::document("Koekilpailu", link, "ABCDEF123456").unwrap();
        assert!(!document.landscape);
        assert_eq!(document.pages.len(), 1);
        assert!(
            document.pages[0]
                .texts
                .iter()
                .any(|t| t.value.contains("ABCDEF123456"))
        );
        assert!(crate::printing::postscript(&document).contains("rectfill"));
        let dir = std::env::temp_dir().join(format!(
            "as2026-qr-tuloste-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let paths = crate::printing::preview_png(&document, &dir).unwrap();
        let image = image::open(&paths[0]).unwrap().to_luma8();
        let mut prepared = rqrr::PreparedImage::prepare_from_greyscale(
            image.width() as usize,
            image.height() as usize,
            |x, y| image.get_pixel(x as u32, y as u32)[0],
        );
        let grids = prepared.detect_grids();
        assert!(
            grids
                .iter()
                .any(|grid| grid.decode().is_ok_and(|(_, text)| text == link))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
    #[test]
    fn qr_is_decodable_as_the_complete_join_link() {
        let link = "https://example.fi/kerho/as-tulkki/#join=ABCDEF123456&site=https%3A%2F%2Fexample.fi%2Fkerho%2F";
        let b = super::encode(link).unwrap();
        let mut image = rqrr::PreparedImage::prepare_from_greyscale(
            b.side as usize,
            b.side as usize,
            |x, y| b.rgba[(y * b.side as usize + x) * 4],
        );
        let grids = image.detect_grids();
        assert_eq!(grids.len(), 1);
        assert_eq!(grids[0].decode().unwrap().1, link);
    }
}
