use as2026::{
    ClockTime,
    legacy::Encoding,
    terminal::{Action, MenuChoice, clean_path, menu, parse_choice, parse_clock, parse_yes_no},
};
use std::{
    io::Cursor,
    path::PathBuf,
    process::{Command, Stdio},
};

fn choose(input: &str) -> (Action, String) {
    let mut output = Vec::new();
    let action = menu(&mut Cursor::new(input.as_bytes()), &mut output).unwrap();
    (action, String::from_utf8(output).unwrap())
}

#[test]
fn menu_eof_exits_at_every_prompt_without_waiting() {
    for input in ["", "1\n", "2\nkilpailu\n", "4\n", "4\nframe.raw\n"] {
        assert_eq!(choose(input).0, Action::Quit, "input: {input:?}");
    }
}

#[test]
fn application_exits_successfully_when_stdin_is_closed() {
    let output = Command::new(env!("CARGO_BIN_EXE_as2026"))
        .stdin(Stdio::null())
        .output()
        .expect("Rust application starts");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn menu_io_errors_are_shown_in_finnish() {
    let mut invalid_utf8 = Cursor::new([0xff, b'\n']);
    let error = menu(&mut invalid_utf8, &mut Vec::new()).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    assert_eq!(
        error.to_string(),
        "Käyttövalikon syötteen lukeminen tai tulostaminen epäonnistui"
    );

    struct BrokenWriter;
    impl std::io::Write for BrokenWriter {
        fn write(&mut self, _bytes: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("English implementation detail"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let error = menu(&mut Cursor::new(b"0\n"), &mut BrokenWriter).unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert!(!error.to_string().contains("English"));
}

#[test]
fn menu_recovers_from_invalid_choice_and_empty_path() {
    let (action, output) = choose("väärä\n1\n\n5\n");
    assert_eq!(action, Action::Demo);
    assert!(output.contains("Valintaa ei tunnistettu"));
}

#[test]
fn directory_choices_preserve_unicode_and_support_windows_line_endings() {
    for (number, expected) in [
        (
            1,
            Action::Inspect {
                directory: PathBuf::from(r"C:\Kilpailut\Äänekoski 2026"),
                encoding: Encoding::Windows1252,
            },
        ),
        (
            2,
            Action::Calculate {
                directory: PathBuf::from(r"C:\Kilpailut\Äänekoski 2026"),
                encoding: Encoding::Windows1252,
            },
        ),
        (
            3,
            Action::RankStored {
                directory: PathBuf::from(r"C:\Kilpailut\Äänekoski 2026"),
                encoding: Encoding::Windows1252,
            },
        ),
    ] {
        let input = format!("{number}\r\n \"C:\\Kilpailut\\Äänekoski 2026\" \r\n\r\n");
        assert_eq!(choose(&input).0, expected);
    }
}

#[test]
fn encoding_prompt_retries_and_accepts_finnish_yes() {
    let (action, output) = choose("1\n/tmp/Äänekoski\nehkä\nKYLLÄ\n");
    assert_eq!(
        action,
        Action::Inspect {
            directory: PathBuf::from("/tmp/Äänekoski"),
            encoding: Encoding::Utf8,
        }
    );
    assert!(output.contains("Vastaa k"));
}

#[test]
fn decode_clock_is_optional_and_invalid_clocks_can_be_corrected() {
    assert_eq!(
        choose("4\n'Emit kortti.raw'\n\n").0,
        Action::Decode {
            path: PathBuf::from("Emit kortti.raw"),
            readout_clock: None,
        }
    );
    let (action, output) = choose("4\nframe.raw\n24:00:00\n12:11:00\n");
    assert_eq!(
        action,
        Action::Decode {
            path: PathBuf::from("frame.raw"),
            readout_clock: Some(ClockTime::new(12, 11, 0).unwrap()),
        }
    );
    assert!(output.contains("Virheellinen syöte"));
}

#[test]
fn pasted_paths_preserve_windows_separators_and_literal_characters() {
    let cases = [
        ("  '/home/user/Kilpailu ä'  ", "/home/user/Kilpailu ä"),
        ("/home/user/Kilpailu\\ ä", "/home/user/Kilpailu ä"),
        (r#""C:\Kilpailut\ä""#, r"C:\Kilpailut\ä"),
        (r"C:\Kilpailut\ ää", r"C:\Kilpailut\ ää"),
        (r"\\server\Kilpailut\ ää", r"\\server\Kilpailut\ ää"),
        ("'$(do-not-execute)'", "$(do-not-execute)"),
    ];
    for (input, expected) in cases {
        assert_eq!(clean_path(input), PathBuf::from(expected));
    }
}

#[test]
fn choice_yes_no_and_clock_validation_are_explicit() {
    assert_eq!(parse_choice("  DEMO "), Some(MenuChoice::Demo));
    assert_eq!(parse_choice("6"), Some(MenuChoice::ReadLive));
    assert_eq!(parse_yes_no(" kyllä "), Some(true));
    assert_eq!(parse_yes_no("EI"), Some(false));
    assert_eq!(parse_yes_no(""), None);
    assert_eq!(parse_clock("00:00:00").unwrap().hour, 0);
    assert_eq!(parse_clock("23:59:59").unwrap().second, 59);
    for value in ["24:00:00", "12:60:00", "12:00:60", "1:02:03", "a:b:c"] {
        assert!(parse_clock(value).is_err(), "{value}");
    }
}

#[test]
fn live_reader_choice_starts_automatic_detection_without_prompt() {
    let (action, output) = choose("6\n");
    assert_eq!(action, Action::ReadLive { port: None });
    assert!(!output.contains("Sarjaportti ("));
}

#[test]
fn create_competition_menu_preserves_name() {
    assert_eq!(
        choose("7\n/tmp/Harjoitus\nSyysajo\n").0,
        Action::Create {
            directory: PathBuf::from("/tmp/Harjoitus"),
            name: "Syysajo".into()
        }
    );
    assert_eq!(
        choose("7\n/tmp/Harjoitus\n\n").0,
        Action::Create {
            directory: PathBuf::from("/tmp/Harjoitus"),
            name: "Uusi kilpailu".into()
        }
    );
}
