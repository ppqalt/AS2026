//! Selected result exports share the print layout and its publication rules.
use crate::{CoreError, printing::Document};
use std::{collections::BTreeMap, io::Write};

fn invalid(s: impl Into<String>) -> CoreError {
    CoreError::InvalidInput(s.into())
}
fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Recover semantic rows from the common print model. Coordinates only group
/// cells; the original strings are exported, never rasterized.
pub fn rows(document: &Document) -> Vec<Vec<String>> {
    let mut result = Vec::new();
    for page in &document.pages {
        let mut groups = BTreeMap::<i64, Vec<&crate::printing::Text>>::new();
        for text in &page.texts {
            groups
                .entry((text.y * 1000.).round() as i64)
                .or_default()
                .push(text);
        }
        for (_, mut row) in groups {
            row.sort_by(|a, b| a.x.total_cmp(&b.x));
            result.push(row.into_iter().map(|t| t.value.clone()).collect());
        }
    }
    result
}
pub fn csv(document: &Document) -> String {
    rows(document)
        .iter()
        .map(|row| {
            row.iter()
                .map(|s| format!("\"{}\"", s.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join(";")
                + "\r\n"
        })
        .collect()
}
pub fn html(document: &Document) -> String {
    let mut html = String::from(
        "<!doctype html><html lang=\"fi\"><meta charset=\"utf-8\"><style>body{font:14px sans-serif}table{border-collapse:collapse;margin-bottom:1em}td{padding:4px 8px;border-bottom:1px solid #ddd}</style>",
    );
    let mut table_open = false;
    for row in rows(document) {
        if row.len() == 1 {
            if table_open {
                html.push_str("</tbody></table>");
                table_open = false;
            }
            html.push_str(&format!("<h2>{}</h2>", xml(&row[0])));
        } else {
            if !table_open {
                html.push_str("<table><tbody>");
                table_open = true;
            }
            html.push_str("<tr>");
            for cell in row {
                html.push_str(&format!("<td>{}</td>", xml(&cell)));
            }
            html.push_str("</tr>");
        }
    }
    if table_open {
        html.push_str("</tbody></table>");
    }
    html.push_str("</html>");
    html
}

struct Pdf {
    objects: Vec<Vec<u8>>,
}
impl Pdf {
    fn add(&mut self, bytes: impl Into<Vec<u8>>) -> usize {
        self.objects.push(bytes.into());
        self.objects.len()
    }
    fn stream(&mut self, dictionary: &str, data: &[u8]) -> usize {
        let mut bytes = format!("<< {dictionary} /Length {} >>\nstream\n", data.len()).into_bytes();
        bytes.extend(data);
        bytes.extend(b"\nendstream");
        self.add(bytes)
    }
    fn finish(self) -> Vec<u8> {
        let mut output = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
        let mut offsets = vec![0];
        for (i, object) in self.objects.iter().enumerate() {
            offsets.push(output.len());
            output.extend(format!("{} 0 obj\n", i + 1).bytes());
            output.extend(object);
            output.extend(b"\nendobj\n");
        }
        let offset = output.len();
        output.extend(format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len()).bytes());
        for entry in offsets.iter().skip(1) {
            output.extend(format!("{entry:010} 00000 n \n").bytes());
        }
        output.extend(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{offset}\n%%EOF\n",
                offsets.len()
            )
            .bytes(),
        );
        output
    }
}
fn deflate(bytes: &[u8]) -> Result<Vec<u8>, CoreError> {
    let mut writer = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    writer
        .write_all(bytes)
        .map_err(|e| invalid(e.to_string()))?;
    writer.finish().map_err(|e| invalid(e.to_string()))
}
pub fn validate_logo(bytes: &[u8]) -> Result<(), CoreError> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(invalid("Logo saa olla enintään 4 Mt"));
    }
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .map_err(|_| invalid("Valitse kelvollinen PNG-logo"))?;
    if image.width() == 0 || image.height() == 0 || image.width() > 4096 || image.height() > 4096 {
        return Err(invalid("Logon enimmäiskoko on 4096 × 4096 pikseliä"));
    }
    Ok(())
}
/// Unicode TrueType text and a 20 mm logo, independent of OS printer drivers.
pub fn pdf(document: &Document, logo: &[u8]) -> Result<Vec<u8>, CoreError> {
    const FONT: &[u8] = include_bytes!("../assets/fonts/DejaVuSans.ttf");
    let face =
        ttf_parser::Face::parse(FONT, 0).map_err(|_| invalid("PDF-fontin luku epäonnistui"))?;
    let mut pdf = Pdf {
        objects: vec![Vec::new(), Vec::new()],
    };
    let file = pdf.stream(
        &format!("/Filter /FlateDecode /Length1 {}", FONT.len()),
        &deflate(FONT)?,
    );
    let descriptor=pdf.add(format!("<< /Type /FontDescriptor /FontName /DejaVuSans /Flags 32 /FontBBox [-1021 -463 1793 1232] /ItalicAngle 0 /Ascent 928 /Descent -236 /CapHeight 729 /StemV 80 /FontFile2 {file} 0 R >>"));
    let mut glyphs = BTreeMap::new();
    for text in document.pages.iter().flat_map(|p| &p.texts) {
        for c in text.value.chars() {
            let gid = face
                .glyph_index(c)
                .ok_or_else(|| invalid(format!("PDF-fontista puuttuu merkki {c:?}")))?;
            glyphs.entry(gid.0).or_insert(c);
        }
    }
    let widths = glyphs
        .keys()
        .map(|&g| {
            format!(
                "{g} [{}]",
                u32::from(face.glyph_hor_advance(ttf_parser::GlyphId(g)).unwrap_or(0)) * 1000
                    / u32::from(face.units_per_em())
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let cid=pdf.add(format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /DejaVuSans /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor} 0 R /CIDToGIDMap /Identity /W [{widths}] >>"));
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin 12 dict begin begincmap /CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def /CMapName /AS2026Unicode def /CMapType 2 def 1 begincodespacerange <0000> <FFFF> endcodespacerange\n",
    );
    for chunk in glyphs.iter().collect::<Vec<_>>().chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for (g, c) in chunk {
            let mut buf = [0u16; 2];
            let unicode = c
                .encode_utf16(&mut buf)
                .iter()
                .map(|v| format!("{v:04X}"))
                .collect::<String>();
            cmap.push_str(&format!("<{g:04X}> <{unicode}>\n"));
        }
        cmap.push_str("endbfchar\n");
    }
    cmap.push_str("endcmap CMapName currentdict /CMap defineresource pop end end");
    let unicode = pdf.stream("", cmap.as_bytes());
    let font=pdf.add(format!("<< /Type /Font /Subtype /Type0 /BaseFont /DejaVuSans /Encoding /Identity-H /DescendantFonts [{cid} 0 R] /ToUnicode {unicode} 0 R >>"));
    let image = if logo.is_empty() {
        None
    } else {
        validate_logo(logo)?;
        let rgba = image::load_from_memory_with_format(logo, image::ImageFormat::Png)
            .map_err(|_| invalid("Logoa ei voitu lukea"))?
            .to_rgba8();
        let mut rgb = Vec::new();
        for p in rgba.pixels() {
            for c in &p.0[..3] {
                rgb.push(
                    ((u32::from(*c) * u32::from(p[3]) + 255 * (255 - u32::from(p[3]))) / 255) as u8,
                );
            }
        }
        let id=pdf.stream(&format!("/Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode",rgba.width(),rgba.height()),&deflate(&rgb)?);
        let ratio = f64::from(rgba.width()) / f64::from(rgba.height());
        let h = 20_f64.min(60. / ratio);
        Some((id, h * ratio, h))
    };
    let (width, height) = if document.landscape {
        (297., 210.)
    } else {
        (210., 297.)
    };
    let mm = 72. / 25.4;
    let mut pages = Vec::new();
    for page in &document.pages {
        let mut commands = String::from("0.4 w\n");
        if let Some((_, w, h)) = image {
            commands.push_str(&format!(
                "q {} 0 0 {} {} {} cm /Logo Do Q\n",
                w * mm,
                h * mm,
                10. * mm,
                (height - 10. - h) * mm
            ));
        }
        for line in &page.lines {
            commands.push_str(&format!(
                "{} {} m {} {} l S\n",
                line.x * mm,
                (height - line.y) * mm,
                line.x2 * mm,
                (height - line.y2) * mm
            ));
        }
        for text in &page.texts {
            let mut x = text.x;
            let mut available = text.width;
            if let Some((_, w, h)) = image
                && text.y < 10. + h
                && text.y + text.height > 10.
                && text.x < 10. + w + 3.
            {
                x = 10. + w + 3.;
                available = (text.x + text.width - x).max(1.);
            }
            let advance = text
                .value
                .chars()
                .map(|c| {
                    face.glyph_index(c)
                        .and_then(|g| face.glyph_hor_advance(g))
                        .unwrap_or(0) as f64
                })
                .sum::<f64>()
                / f64::from(face.units_per_em());
            let size = text.size.min(available * mm / advance.max(1.));
            let value = text
                .value
                .chars()
                .map(|c| format!("{:04X}", face.glyph_index(c).unwrap().0))
                .collect::<String>();
            commands.push_str(&format!(
                "BT /F {size} Tf {} Tr 0.15 w 1 0 0 1 {} {} Tm <{value}> Tj ET\n",
                if text.bold { 2 } else { 0 },
                x * mm,
                (height - text.y) * mm - size
            ));
        }
        let content = pdf.stream("/Filter /FlateDecode", &deflate(commands.as_bytes())?);
        let resources = format!(
            "/Font << /F {font} 0 R >> {}",
            image.map_or(String::new(), |(i, _, _)| format!(
                "/XObject << /Logo {i} 0 R >>"
            ))
        );
        pages.push(pdf.add(format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << {resources} >> /Contents {content} 0 R >>",width*mm,height*mm)));
    }
    pdf.objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>".to_vec();
    pdf.objects[1] = format!(
        "<< /Type /Pages /Count {} /Kids [{}] >>",
        pages.len(),
        pages
            .iter()
            .map(|p| format!("{p} 0 R"))
            .collect::<Vec<_>>()
            .join(" ")
    )
    .into_bytes();
    Ok(pdf.finish())
}
