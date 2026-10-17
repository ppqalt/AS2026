//! Local-only live HTML results. Every refresh reloads the competition.
//! No scripts or external assets, and imported AS2015A files remain read-only.
use crate::CoreError;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    time::Duration,
};

pub fn refreshed_html(html: &str) -> String {
    html.replacen(
        "<head>",
        "<head><meta http-equiv=\"refresh\" content=\"5\">",
        1,
    )
}

fn respond(mut stream: TcpStream, render: &mut impl FnMut() -> Result<String, CoreError>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let mut input = [0_u8; 2048];
    let count = stream.read(&mut input).unwrap_or(0);
    let valid = input[..count].starts_with(b"GET / HTTP/1.");
    let (status, body) = if valid {
        match render() {
            Ok(html) => ("200 OK", refreshed_html(&html)),
            Err(_) => (
                "503 Service Unavailable",
                "<!doctype html><html lang=\"fi\"><title>Tulokset</title><p>Tuloksia ei voitu päivittää. Tarkista kilpailun tiedot.</p></html>".to_owned(),
            ),
        }
    } else {
        ("404 Not Found", "Not found".to_owned())
    };
    let headers = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes());
    let _ = stream.write_all(body.as_bytes());
}

/// Blocks, serving localhost only; Ctrl+C stops the process.
pub fn serve(
    port: u16,
    mut render: impl FnMut() -> Result<String, CoreError>,
) -> Result<(), CoreError> {
    if port == 0 {
        return Err(CoreError::InvalidInput(
            "Anna live-HTML:lle portti 1–65535".into(),
        ));
    }
    let listener = TcpListener::bind(("127.0.0.1", port)).map_err(|error| {
        CoreError::InvalidInput(format!(
            "Paikallista tulospalvelinta ei voitu avata: {error}"
        ))
    })?;
    println!("Live HTML: http://127.0.0.1:{port}/ (päivitys 5 s, lopeta Ctrl+C)");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => respond(stream, &mut render),
            Err(error) => eprintln!("Live HTML: yhteysvirhe: {error}"),
        }
    }
    Ok(())
}
