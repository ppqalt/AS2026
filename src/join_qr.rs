//! QR muodostetaan paikallisesti; sisältönä on vain puhelimen liittymislinkki.
use qrcodegen::{QrCode, QrCodeEcc};
pub struct Bitmap {
    pub side: u32,
    pub rgba: Vec<u8>,
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
