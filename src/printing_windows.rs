//! Windowsin GDI-tulostus ilman PowerShelliä tai .NET-ajoympäristöä.
//! Tulostus ja jonoon lähettämätön koekuvarenderöinti käyttävät samaa piirtäjää.
use super::{Document, Page};
#[cfg(feature = "gui")]
use std::ffi::c_void;
use std::{
    mem::size_of,
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::{ERROR_INSUFFICIENT_BUFFER, GetLastError, RECT},
    Graphics::{Gdi::*, Printing::*},
    Storage::Xps::*,
};

fn error(context: &str) -> String {
    format!("{context}: {}", std::io::Error::last_os_error())
}
fn wide(value: &str) -> Result<Vec<u16>, String> {
    if value.contains('\0') {
        return Err("Tulostustieto sisältää kielletyn nollamerkin".into());
    }
    Ok(value.encode_utf16().chain(Some(0)).collect())
}
// Win32:n palauttamat rakenteet tarvitsevat osoitintasauksen.
fn buffer(bytes: usize) -> Result<Vec<usize>, String> {
    if bytes > 16 * 1024 * 1024 {
        return Err("Tulostin palautti liian suuren asetusrakenteen".into());
    }
    Ok(vec![0; bytes.div_ceil(size_of::<usize>())])
}

struct Dc(HDC);
impl Drop for Dc {
    fn drop(&mut self) {
        // SAFETY: Dc omistaa vain onnistuneesti luodun kontekstin.
        unsafe {
            DeleteDC(self.0);
        }
    }
}
struct Printer(PRINTER_HANDLE);
impl Drop for Printer {
    fn drop(&mut self) {
        // SAFETY: Printer omistaa OpenPrinterW:n palauttaman kahvan.
        unsafe {
            ClosePrinter(self.0);
        }
    }
}
struct Selection {
    dc: HDC,
    object: HGDIOBJ,
    previous: HGDIOBJ,
}
impl Selection {
    fn new(dc: HDC, object: HGDIOBJ) -> Result<Self, String> {
        if object.is_null() {
            return Err(error("Tulosteen piirto-objektia ei voitu luoda"));
        }
        // SAFETY: Luotu GDI-objekti valitaan elävään kontekstiin.
        let previous = unsafe { SelectObject(dc, object) };
        if previous.is_null() || previous as isize == -1 {
            unsafe {
                DeleteObject(object);
            }
            return Err(error("Tulosteen piirto-objektia ei voitu valita"));
        }
        Ok(Self {
            dc,
            object,
            previous,
        })
    }
}
impl Drop for Selection {
    fn drop(&mut self) {
        // SAFETY: Palautus ennen poistoa; konteksti elää valintaa pidempään.
        unsafe {
            SelectObject(self.dc, self.previous);
            DeleteObject(self.object);
        }
    }
}

/// Nimet luetaan vain Win32:n palauttaman puskurin sisältä.
fn name_in_buffer(data: &[usize], pointer: *const u16) -> Option<String> {
    let start = data.as_ptr() as usize;
    let end = start.checked_add(size_of_val(data))?;
    let address = pointer as usize;
    if address < start || address >= end || !address.is_multiple_of(2) {
        return None;
    }
    // SAFETY: Osoite ja kaikkien merkkien rajat tarkistettiin puskurista.
    let units = unsafe { std::slice::from_raw_parts(pointer, (end - address) / 2) };
    let length = units.iter().position(|&c| c == 0)?;
    Some(String::from_utf16_lossy(&units[..length]))
}

pub(super) fn printers() -> Result<Vec<String>, String> {
    let mut needed = 0;
    let mut returned = 0;
    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;
    // SAFETY: Ensimmäinen kutsu kysyy puskurin koon, eikä kirjoita dataa.
    let first =
        unsafe { EnumPrintersW(flags, null(), 4, null_mut(), 0, &mut needed, &mut returned) };
    if needed == 0 {
        return if first != 0 {
            Ok(Vec::new())
        } else {
            Err(error("Tulostinluetteloa ei voitu hakea"))
        };
    }
    for _ in 0..3 {
        let mut data = buffer(needed as usize)?;
        // SAFETY: Puskuri on vähintään pyydetyn kokoinen ja oikein tasattu.
        if unsafe {
            EnumPrintersW(
                flags,
                null(),
                4,
                data.as_mut_ptr().cast(),
                needed,
                &mut needed,
                &mut returned,
            )
        } == 0
        {
            if unsafe { GetLastError() } == ERROR_INSUFFICIENT_BUFFER {
                continue;
            }
            return Err(error("Tulostinluetteloa ei voitu hakea"));
        }
        if returned as usize > size_of_val(data.as_slice()) / size_of::<PRINTER_INFO_4W>() {
            return Err("Tulostinluettelon rakenne on virheellinen".into());
        }
        // SAFETY: Win32 palautti returned kpl PRINTER_INFO_4W-rakenteita.
        let printers = unsafe {
            std::slice::from_raw_parts(data.as_ptr().cast::<PRINTER_INFO_4W>(), returned as usize)
        };
        let mut names = printers
            .iter()
            .filter_map(|p| name_in_buffer(&data, p.pPrinterName))
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        names.sort();
        names.dedup();
        return Ok(names);
    }
    Err("Tulostinluettelo muuttui haun aikana. Päivitä luettelo uudelleen.".into())
}

fn default_printer() -> Result<Vec<u16>, String> {
    let mut length = 0;
    // SAFETY: Koko kysytään ennen puskurin varausta.
    unsafe {
        GetDefaultPrinterW(null_mut(), &mut length);
    }
    if length == 0 || length > 32768 {
        return Err("Oletustulostinta ei ole valittu. Valitse tulostin asetuksista.".into());
    }
    let mut name = vec![0; length as usize];
    if unsafe { GetDefaultPrinterW(name.as_mut_ptr(), &mut length) } == 0 {
        return Err(error("Oletustulostinta ei voitu hakea"));
    }
    Ok(name)
}

fn printer_context(printer: &str, landscape: bool) -> Result<Dc, String> {
    let name = if printer.is_empty() {
        default_printer()?
    } else {
        wide(printer)?
    };
    let mut handle = PRINTER_HANDLE::default();
    // SAFETY: Nollapäätteinen nimi, kahvaan varattu kelvollinen osoite.
    if unsafe { OpenPrinterW(name.as_ptr(), &mut handle, null()) } == 0 {
        return Err(error("Valittua tulostinta ei voi avata"));
    }
    let printer = Printer(handle);
    // Tulostinajurin yksityinen DEVMODE-lisäosa on säilytettävä kokonaisena.
    let bytes =
        unsafe { DocumentPropertiesW(null_mut(), printer.0, name.as_ptr(), null_mut(), null(), 0) };
    if bytes < size_of::<DEVMODEW>() as i32 {
        return Err(error("Tulostimen asetuksia ei voitu hakea"));
    }
    let mut mode_data = buffer(bytes as usize)?;
    let mode = mode_data.as_mut_ptr().cast::<DEVMODEW>();
    // SAFETY: Ajurin ilmoittaman kokoinen ja oikein tasattu DEVMODE-puskuri.
    if unsafe {
        DocumentPropertiesW(
            null_mut(),
            printer.0,
            name.as_ptr(),
            mode,
            null(),
            DM_OUT_BUFFER,
        )
    } != 1
    {
        return Err(error("Tulostimen asetuksia ei voitu lukea"));
    }
    // Varmista A4-tuki ennen tulostustyön avaamista.
    let paper_count =
        unsafe { DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, null_mut(), mode) };
    if paper_count <= 0 || paper_count > 65536 {
        return Err("Tulostimen paperikokoja ei voitu varmistaa".into());
    }
    let mut papers = vec![0u16; paper_count as usize];
    if unsafe { DeviceCapabilitiesW(name.as_ptr(), null(), DC_PAPERS, papers.as_mut_ptr(), mode) }
        != paper_count
        || !papers.contains(&(DMPAPER_A4 as u16))
    {
        return Err("Tulostin ei tue A4-paperia".into());
    }
    let orientation = if landscape {
        DMORIENT_LANDSCAPE
    } else {
        DMORIENT_PORTRAIT
    } as i16;
    // SAFETY: DEVMODE on tulostimesta, joten unionin tulostinosa on voimassa.
    unsafe {
        (*mode).dmFields =
            ((*mode).dmFields & !(DM_PAPERLENGTH | DM_PAPERWIDTH)) | DM_PAPERSIZE | DM_ORIENTATION;
        (*mode).Anonymous1.Anonymous1.dmPaperSize = DMPAPER_A4 as i16;
        (*mode).Anonymous1.Anonymous1.dmOrientation = orientation;
        if DocumentPropertiesW(
            null_mut(),
            printer.0,
            name.as_ptr(),
            mode,
            mode,
            DM_IN_BUFFER | DM_OUT_BUFFER,
        ) != 1
        {
            return Err(error("A4-tulostusasetuksia ei voitu vahvistaa"));
        }
        if (*mode).Anonymous1.Anonymous1.dmPaperSize != DMPAPER_A4 as i16
            || (*mode).Anonymous1.Anonymous1.dmOrientation != orientation
        {
            return Err("Tulostin ei hyväksynyt A4-paperia tai valittua suuntaa".into());
        }
    }
    let driver = wide("WINSPOOL")?;
    let dc = unsafe { CreateDCW(driver.as_ptr(), name.as_ptr(), null(), mode) };
    if dc.is_null() {
        return Err(error("Tulostuskontekstia ei voitu luoda"));
    }
    Ok(Dc(dc))
}

struct Canvas {
    dc: HDC,
    dpi_x: i32,
    dpi_y: i32,
    offset_x: i32,
    offset_y: i32,
}
impl Canvas {
    fn x(&self, mm: f64) -> i32 {
        (mm * self.dpi_x as f64 / 25.4).round() as i32 - self.offset_x
    }
    fn y(&self, mm: f64) -> i32 {
        (mm * self.dpi_y as f64 / 25.4).round() as i32 - self.offset_y
    }
    fn draw(&self, page: &Page) -> Result<(), String> {
        let pen_width = (0.15 * self.dpi_x as f64 / 25.4).round().max(1.) as i32;
        // SAFETY: Kaikki piirrot kohdistetaan elävään DC:hen. Valintavartijat
        // palauttavat edelliset objektit ennen luotujen objektien vapauttamista.
        let _pen = Selection::new(self.dc, unsafe { CreatePen(PS_SOLID, pen_width, 0) })?;
        unsafe {
            SetBkMode(self.dc, TRANSPARENT as i32);
            SetTextColor(self.dc, 0);
        }
        for line in &page.lines {
            if unsafe { MoveToEx(self.dc, self.x(line.x), self.y(line.y), null_mut()) } == 0
                || unsafe { LineTo(self.dc, self.x(line.x2), self.y(line.y2)) } == 0
            {
                return Err(error("Tulosteen viivaa ei voitu piirtää"));
            }
        }
        let family = wide("Arial")?;
        for text in &page.texts {
            if text.value.is_empty() {
                continue;
            }
            let font_height = -(text.size * self.dpi_y as f64 / 72.).round().max(1.) as i32;
            let _font = Selection::new(self.dc, unsafe {
                CreateFontW(
                    font_height,
                    0,
                    0,
                    0,
                    if text.bold {
                        FW_BOLD as i32
                    } else {
                        FW_NORMAL as i32
                    },
                    0,
                    0,
                    0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    DEFAULT_QUALITY as u32,
                    DEFAULT_PITCH as u32,
                    family.as_ptr(),
                )
            })?;
            let value = wide(&text.value)?;
            let mut rect = RECT {
                left: self.x(text.x),
                top: self.y(text.y),
                right: self.x(text.x + text.width),
                bottom: self.y(text.y + text.height),
            };
            // Nimet eivät tulkitse &-merkkiä pikanäppäimeksi. Ei rivinvaihtoa
            // eikä tekstiä seuraavan sarakkeen päälle.
            if unsafe {
                DrawTextW(
                    self.dc,
                    value.as_ptr(),
                    (value.len() - 1) as i32,
                    &mut rect,
                    DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_END_ELLIPSIS,
                )
            } == 0
            {
                return Err(error("Tulosteen tekstiä ei voitu piirtää"));
            }
        }
        Ok(())
    }
}

pub(super) fn submit(document: &Document, printer: &str) -> Result<(), String> {
    if document.pages.is_empty() {
        return Err("Tulosteessa ei ole sivuja".into());
    }
    let title = wide(&document.title)?;
    let dc = printer_context(printer, document.landscape)?;
    let canvas = Canvas {
        dc: dc.0,
        dpi_x: unsafe { GetDeviceCaps(dc.0, LOGPIXELSX as i32) },
        dpi_y: unsafe { GetDeviceCaps(dc.0, LOGPIXELSY as i32) },
        offset_x: unsafe { GetDeviceCaps(dc.0, PHYSICALOFFSETX as i32) },
        offset_y: unsafe { GetDeviceCaps(dc.0, PHYSICALOFFSETY as i32) },
    };
    if canvas.dpi_x <= 0 || canvas.dpi_y <= 0 {
        return Err("Tulostimen tarkkuutta ei voitu hakea".into());
    }
    let info = DOCINFOW {
        cbSize: size_of::<DOCINFOW>() as i32,
        lpszDocName: title.as_ptr(),
        ..Default::default()
    };
    if unsafe { StartDocW(dc.0, &info) } <= 0 {
        return Err(error("Tulostustyötä ei voitu avata"));
    }
    // Keskeytä epäonnistunut työ myös sivun piirtäjän palauttaessa virheen.
    struct Job(HDC, bool);
    impl Drop for Job {
        fn drop(&mut self) {
            if !self.1 {
                unsafe {
                    AbortDoc(self.0);
                }
            }
        }
    }
    let mut job = Job(dc.0, false);
    for page in &document.pages {
        if unsafe { StartPage(dc.0) } <= 0 {
            return Err(error("Tulosteen sivua ei voitu aloittaa"));
        }
        canvas.draw(page)?;
        if unsafe { EndPage(dc.0) } <= 0 {
            return Err(error("Tulosteen sivua ei voitu lähettää"));
        }
    }
    if unsafe { EndDoc(dc.0) } <= 0 {
        return Err(error("Tulostustyötä ei voitu lähettää"));
    }
    job.1 = true;
    Ok(())
}

/// Sama GDI-piirtäjä muistibittikarttaan; ei avaa tulostinta tai tulostusjonoa.
#[cfg(feature = "gui")]
pub(super) fn preview_png(
    document: &Document,
    directory: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, String> {
    std::fs::create_dir_all(directory).map_err(|e| e.to_string())?;
    let (width, height) = if document.landscape {
        (1754, 1240)
    } else {
        (1240, 1754)
    };
    let dc = unsafe { CreateCompatibleDC(null_mut()) };
    if dc.is_null() {
        return Err(error("Koekuvan kontekstia ei voitu luoda"));
    }
    let dc = Dc(dc);
    let info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels: *mut c_void = null_mut();
    let bitmap =
        unsafe { CreateDIBSection(dc.0, &info, DIB_RGB_COLORS, &mut pixels, null_mut(), 0) };
    let _bitmap = Selection::new(dc.0, bitmap)?;
    if pixels.is_null() {
        return Err("Koekuvan pikselipuskuria ei voitu luoda".into());
    }
    let canvas = Canvas {
        dc: dc.0,
        dpi_x: 150,
        dpi_y: 150,
        offset_x: 0,
        offset_y: 0,
    };
    let mut paths = Vec::new();
    for (index, page) in document.pages.iter().enumerate() {
        // GDI voi puskuroida piirtoja. Synkronoi ennen suoraa pikselien käsittelyä.
        if unsafe { GdiFlush() } == 0 {
            return Err(error("Koekuvan piirtoja ei voitu viimeistellä"));
        }
        let length = width as usize * height as usize * 4;
        unsafe {
            std::ptr::write_bytes(pixels, 255, length);
        }
        canvas.draw(page)?;
        if unsafe { GdiFlush() } == 0 {
            return Err(error("Koekuvan piirtoja ei voitu viimeistellä"));
        }
        // SAFETY: DIB varasi width*height*4 tavua; yläreuna ensin (neg. korkeus).
        let source = unsafe { std::slice::from_raw_parts(pixels.cast::<u8>(), length) };
        let mut rgb = Vec::with_capacity(width as usize * height as usize * 3);
        for pixel in source.as_chunks::<4>().0 {
            rgb.extend([pixel[2], pixel[1], pixel[0]]);
        }
        let path = directory.join(format!("sivu-{}.png", index + 1));
        image::save_buffer(
            &path,
            &rgb,
            width as u32,
            height as u32,
            image::ColorType::Rgb8,
        )
        .map_err(|e| e.to_string())?;
        paths.push(path);
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utf16_names_are_bounded_and_preserve_finnish_text() {
        assert!(wide("Väärä\0nimi").is_err());
        let name = wide("Äänekoski & Yö").unwrap();
        let mut data = buffer(name.len() * 2).unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(
                name.as_ptr(),
                data.as_mut_ptr().cast::<u16>(),
                name.len(),
            );
        }
        assert_eq!(
            name_in_buffer(&data, data.as_ptr().cast()).as_deref(),
            Some("Äänekoski & Yö")
        );
        assert!(name_in_buffer(&data, null()).is_none());
        assert!(
            name_in_buffer(&data, unsafe {
                data.as_ptr().byte_add(size_of_val(data.as_slice())).cast()
            })
            .is_none()
        );
        assert!(name_in_buffer(&data, unsafe { data.as_ptr().byte_add(1).cast() }).is_none());
        let unterminated = [usize::MAX; 2];
        assert!(name_in_buffer(&unterminated, unterminated.as_ptr().cast()).is_none());
    }
    #[test]
    fn page_coordinates_use_physical_origin_and_printer_dpi() {
        let canvas = Canvas {
            dc: null_mut(),
            dpi_x: 600,
            dpi_y: 300,
            offset_x: 12,
            offset_y: 7,
        };
        assert_eq!(canvas.x(25.4), 588);
        assert_eq!(canvas.y(25.4), 293);
        assert_eq!(canvas.x(0.), -12);
    }
}
