//! Vain luku -yhteyskoe: ei muuta paikallisia asetuksia tai palvelimen tulkintoja.
#[cfg(feature = "desktop")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("Anna asetustiedoston polku")?;
    let settings: as2026::preferences::Preferences =
        toml::from_str(&std::fs::read_to_string(path)?)
            .map_err(|_| "Asetustiedoston rakennetta ei tunnistettu")?;
    for mut config in settings
        .tulkki_yhteydet
        .into_values()
        .filter(|c| c.competition_id > 0 && c.revision > 0)
    {
        let events = as2026::tulkki_yhteys::Api::new(&config.site)?.receive(&mut config)?;
        let pairs = config
            .received
            .iter()
            .map(|r| r.pair_number)
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        println!(
            "Vastaanotto onnistui: {events} tapahtumaa, {pairs} paria, {} jaksoa, kohdistin {}.",
            config.received.len(),
            config.cursor
        );
    }
    Ok(())
}
#[cfg(not(feature = "desktop"))]
fn main() {
    eprintln!("Koe vaatii desktop-ominaisuuden.");
}
