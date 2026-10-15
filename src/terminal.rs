//! Pieni suomenkielinen käyttövalikko ilman pääte- tai käyttöliittymäkirjastoja.
//!
//! Valikko palauttaa vain käyttäjän valinnan. Kutsuja lukee kilpailutiedostot
//! ja näyttää tulokset. Syötteen loppu tarkoittaa aina lopettamista.

use crate::{ClockTime, CoreError, legacy::Encoding};
use std::{
    io::{self, BufRead, Write},
    path::PathBuf,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Inspect {
        directory: PathBuf,
        encoding: Encoding,
    },
    Calculate {
        directory: PathBuf,
        encoding: Encoding,
    },
    RankStored {
        directory: PathBuf,
        encoding: Encoding,
    },
    Decode {
        path: PathBuf,
        readout_clock: Option<ClockTime>,
    },
    Demo,
    Create {
        directory: PathBuf,
        name: String,
    },
    ReadLive {
        port: Option<String>,
    },
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuChoice {
    Inspect,
    Calculate,
    RankStored,
    Decode,
    Demo,
    Create,
    ReadLive,
    Quit,
}

pub fn parse_choice(value: &str) -> Option<MenuChoice> {
    match value.trim().to_ascii_lowercase().as_str() {
        "1" | "inspect" | "tarkista" => Some(MenuChoice::Inspect),
        "2" | "calculate" | "laske" => Some(MenuChoice::Calculate),
        "3" | "rank-stored" | "tulokset" => Some(MenuChoice::RankStored),
        "4" | "decode" | "pura" => Some(MenuChoice::Decode),
        "5" | "demo" => Some(MenuChoice::Demo),
        "6" | "lue" | "read" => Some(MenuChoice::ReadLive),
        "7" | "uusi" => Some(MenuChoice::Create),
        "0" | "q" | "quit" | "lopeta" => Some(MenuChoice::Quit),
        _ => None,
    }
}

/// Tyhjä vastaus jätetään kutsujan oletusvalinnan ratkaistavaksi.
pub fn parse_yes_no(value: &str) -> Option<bool> {
    match value.trim().to_lowercase().as_str() {
        "k" | "kyllä" | "kylla" | "y" | "yes" => Some(true),
        "e" | "ei" | "n" | "no" => Some(false),
        _ => None,
    }
}

/// Poistaa liitetyn polun ympäriltä välilyönnit ja yhden lainausmerkkiparin.
/// Windowsin asematunnukset, kenoviivat ja Unicode-merkit säilyvät ennallaan.
/// Lainaamattoman Linux-polun vedossa syntyvä `\ ` tulkitaan välilyönniksi.
/// Tämä ei aja komentotulkkia eikä tulkitse ympäristömuuttujia tai komentoja.
pub fn clean_path(value: &str) -> PathBuf {
    let value = value.trim();
    if let Some(unquoted) = value
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
    {
        return PathBuf::from(unquoted);
    }
    let bytes = value.as_bytes();
    let windows_path = value.starts_with("\\\\")
        || (bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':');
    if windows_path {
        PathBuf::from(value)
    } else {
        PathBuf::from(value.replace("\\ ", " "))
    }
}

pub fn parse_clock(value: &str) -> Result<ClockTime, CoreError> {
    let parts: Vec<&str> = value.trim().split(':').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.len() != 2 || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        return Err(CoreError::InvalidInput(
            "anna kellonaika muodossa HH:MM:SS, esimerkiksi 12:11:00".into(),
        ));
    }
    let number = |p: &str| {
        p.parse::<u32>()
            .map_err(|_| CoreError::InvalidInput("virheellinen kellonaika".into()))
    };
    ClockTime::new(number(parts[0])?, number(parts[1])?, number(parts[2])?)
}

fn line<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    prompt: &str,
) -> io::Result<Option<String>> {
    write!(output, "{prompt}")?;
    output.flush()?;
    let mut value = String::new();
    if input.read_line(&mut value)? == 0 {
        return Ok(None);
    }
    Ok(Some(value.trim_end_matches(['\r', '\n']).to_owned()))
}

fn encoding<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<Option<Encoding>> {
    loop {
        let Some(value) = line(
            input,
            output,
            "Ovatko tiedostot UTF-8-muodossa? Vanhoille tiedostoille vastaa ei. [k/e, oletus e]: ",
        )?
        else {
            return Ok(None);
        };
        if value.trim().is_empty() {
            return Ok(Some(Encoding::Windows1252));
        }
        match parse_yes_no(&value) {
            Some(true) => return Ok(Some(Encoding::Utf8)),
            Some(false) => return Ok(Some(Encoding::Windows1252)),
            None => writeln!(output, "Vastaa k (kyllä) tai e (ei).")?,
        }
    }
}

/// Pyytää yhden toiminnon. Kutsuja voi näyttää valikon uudelleen toiminnon jälkeen.
/// EOF missä tahansa vaiheessa palauttaa `Action::Quit` ilman virhettä.
pub fn menu<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<Action> {
    menu_inner(input, output).map_err(|error| {
        io::Error::new(
            error.kind(),
            "Käyttövalikon syötteen lukeminen tai tulostaminen epäonnistui",
        )
    })
}

fn menu_inner<R: BufRead, W: Write>(input: &mut R, output: &mut W) -> io::Result<Action> {
    loop {
        writeln!(
            output,
            "\nAS2015 — autosuunnistuksen tulostyökalu\n\n\
             1  Tarkastus — kilpailun tiedot\n\
             2  Tuloslaskenta — esikatselu korttiaineistosta\n\
             3  Tulosluettelot — tallennetut pisteet\n\
             4  Pura Emit-kortin kehystiedosto\n\
             5  Kokeile esimerkkikilpailua\n\
             6  Lue Emit-kortti\n\
             7  Uusi kilpailu\n\
             0  Lopeta"
        )?;
        let Some(value) = line(input, output, "Valitse 0–7: ")? else {
            return Ok(Action::Quit);
        };
        let Some(choice) = parse_choice(&value) else {
            writeln!(output, "Valintaa ei tunnistettu. Anna numero 0–7.")?;
            continue;
        };
        match choice {
            MenuChoice::Quit => return Ok(Action::Quit),
            MenuChoice::Demo => return Ok(Action::Demo),
            MenuChoice::ReadLive => return Ok(Action::ReadLive { port: None }),
            MenuChoice::Create => {
                let Some(value) =
                    line(input, output, "Uuden kilpailun kansio (tyhjä = takaisin): ")?
                else {
                    return Ok(Action::Quit);
                };
                let directory = clean_path(&value);
                if directory.as_os_str().is_empty() {
                    continue;
                }
                let Some(name) = line(input, output, "Kilpailun nimi (tyhjä = Uusi kilpailu): ")?
                else {
                    return Ok(Action::Quit);
                };
                return Ok(Action::Create {
                    directory,
                    name: if name.trim().is_empty() {
                        "Uusi kilpailu".to_owned()
                    } else {
                        name
                    },
                });
            }
            MenuChoice::Inspect | MenuChoice::Calculate | MenuChoice::RankStored => {
                writeln!(
                    output,
                    "Valitse kilpailukansio (kilpailu.toml tai vanhat kilpailutiedostot).\n\
                     Voit liittää tai vetää kansion polun tähän. Tyhjä vastaus palaa valikkoon."
                )?;
                let Some(value) = line(input, output, "Kilpailukansion polku: ")? else {
                    return Ok(Action::Quit);
                };
                let directory = clean_path(&value);
                if directory.as_os_str().is_empty() {
                    continue;
                }
                let encoding = if directory.join("kilpailu.toml").is_file() {
                    Encoding::Utf8
                } else {
                    let Some(encoding) = encoding(input, output)? else {
                        return Ok(Action::Quit);
                    };
                    encoding
                };
                return Ok(match choice {
                    MenuChoice::Inspect => Action::Inspect {
                        directory,
                        encoding,
                    },
                    MenuChoice::Calculate => Action::Calculate {
                        directory,
                        encoding,
                    },
                    _ => Action::RankStored {
                        directory,
                        encoding,
                    },
                });
            }
            MenuChoice::Decode => {
                let Some(value) = line(
                    input,
                    output,
                    "Emit-kehystiedoston polku (217 tavua; tyhjä = takaisin): ",
                )?
                else {
                    return Ok(Action::Quit);
                };
                let path = clean_path(&value);
                if path.as_os_str().is_empty() {
                    continue;
                }
                loop {
                    let Some(value) = line(
                        input,
                        output,
                        "Lukukellonaika HH:MM:SS (tyhjä = vain kehyksen tiedot): ",
                    )?
                    else {
                        return Ok(Action::Quit);
                    };
                    if value.trim().is_empty() {
                        return Ok(Action::Decode {
                            path,
                            readout_clock: None,
                        });
                    }
                    match parse_clock(&value) {
                        Ok(readout_clock) => {
                            return Ok(Action::Decode {
                                path,
                                readout_clock: Some(readout_clock),
                            });
                        }
                        Err(error) => writeln!(output, "{error}")?,
                    }
                }
            }
        }
    }
}
