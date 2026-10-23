//! Lähiverkkoyhteyden käyttöliittymä. Verkkotyö ja levyn pysyvä jono ovat ytimessä.
use super::*;
use as2026::lan::{Client, Host, PORT};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub(super) struct Joined {
    host: Option<Host>,
    client: Client,
    path: PathBuf,
}
fn endpoint(text: &str) -> GuiResult<SocketAddr> {
    let text = text.trim();
    if let Ok(address) = text.parse::<SocketAddr>() {
        return Ok(address);
    }
    text.parse::<IpAddr>()
        .map(|ip| SocketAddr::new(ip, PORT))
        .map_err(|_| invalid("Anna isäntäkoneen IP-osoite, esimerkiksi 192.168.1.20"))
}
fn start_host(s: &mut State, app: &AppWindow) -> GuiResult<()> {
    if s.worker.is_some() {
        return Err(invalid(
            "Pysäytä Emit-lukija jakamisen käynnistyksen ajaksi. Voit käynnistää sen heti uudelleen.",
        ));
    }
    if !s.unsaved.is_empty() {
        return Err(invalid(
            "Tallenna odottavat Emit-luvut ennen jakamisen aloittamista.",
        ));
    }
    if !can_leave(s, app) || s.lan_pending.is_some() {
        return Ok(());
    }
    let path = s
        .workspace
        .as_ref()
        .filter(|ws| ws.is_editable() && ws.lan_client().is_none())
        .ok_or_else(|| invalid("Avaa ensin paikallinen AS2026-kilpailu"))?
        .path()
        .to_owned();
    let (tx, rx) = std::sync::mpsc::channel();
    s.lan_pending = Some(rx);
    app.set_lan_busy(true);
    app.set_lan_error("".into());
    std::thread::spawn(move || {
        let result = (|| {
            let host = Host::start(&path, SocketAddr::new(Ipv4Addr::UNSPECIFIED.into(), PORT))?;
            let address = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), host.address.port());
            let client = Client::join(
                address,
                &host.code,
                &path.with_extension("lan-jono.json"),
                true,
            )?;
            Ok(Joined {
                host: Some(host),
                client,
                path,
            })
        })();
        let _ = tx.send(result);
    });
    Ok(())
}
fn join(s: &mut State, app: &AppWindow) -> GuiResult<()> {
    if !s.unsaved.is_empty() {
        return Err(invalid(
            "Tallenna odottavat Emit-luvut ennen toiseen kilpailuun liittymistä.",
        ));
    }
    if !can_leave(s, app) || s.lan_pending.is_some() {
        return Ok(());
    }
    if s.worker.is_some() {
        return Err(invalid(
            "Pysäytä paikallinen lukija ennen toiseen kilpailuun liittymistä",
        ));
    }
    if s.workspace
        .as_ref()
        .and_then(Workspace::lan_client)
        .is_some()
    {
        return Err(invalid("Katkaise nykyinen LAN-yhteys ensin"));
    }
    let address = endpoint(app.get_lan_address().as_str())?;
    let code = app.get_lan_code().to_string();
    if code.trim().len() < 12 {
        return Err(invalid("Anna isäntäkoneessa näkyvä 12 merkin parituskoodi"));
    }
    let root = s
        .preferences
        .as_ref()
        .ok_or_else(|| invalid("Sovellusasetuksia ei voitu avata"))?
        .competition_root()
        .map_err(|e| invalid(&e))?;
    let (tx, rx) = std::sync::mpsc::channel();
    s.lan_pending = Some(rx);
    app.set_lan_busy(true);
    app.set_lan_error("".into());
    std::thread::spawn(move || {
        let result = (|| {
            let id = Client::identify(address, &code)?;
            let cache = root.join("LAN-kopiot").join(id);
            std::fs::create_dir_all(&cache).map_err(|e| invalid(&io_message(&e)))?;
            let path = cache.join(native::FILE_NAME);
            let client = Client::join(address, &code, &cache.join("lahetysjono.json"), false)?;
            Ok(Joined {
                host: None,
                client,
                path,
            })
        })();
        let _ = tx.send(result);
    });
    Ok(())
}
fn leave(s: &mut State, app: &AppWindow) -> GuiResult<()> {
    if !s.unsaved.is_empty() {
        return Err(invalid(
            "Tallenna odottavat Emit-luvut ennen yhteyden sulkemista.",
        ));
    }
    let Some(client) = s.workspace.as_ref().and_then(Workspace::lan_client) else {
        return Ok(());
    };
    if client.pending() > 0 {
        return Err(invalid(
            "Odota tapahtumien kuittausta ennen yhteyden sulkemista. Yhteyskatkon aikana Emit-luvut säilyvät paikallisessa jonossa.",
        ));
    }
    if !can_leave(s, app) {
        return Ok(());
    }
    let ws = s.workspace.as_ref().unwrap();
    let path = ws.path().to_owned();
    ws.lan_client().unwrap().stop();
    s.lan_host.take();
    s.workspace = Some(Workspace::open(&path)?);
    s.lan_last_notice.clear();
    app.set_lan_active(false);
    app.set_lan_hosting(false);
    app.set_lan_status("Lähiverkko".into());
    app.set_lan_error("".into());
    app.set_lan_info("Yhteys suljettu. Paikallinen kilpailukopio säilyy.".into());
    update_header(s, app)
}
fn accept_join(s: &mut State, app: &AppWindow, joined: Joined) -> GuiResult<()> {
    if !joined.path.exists() {
        ProjectStore::create(&joined.path, joined.client.snapshot()?)?;
    }
    let mut ws = if joined.host.is_some() {
        // Isännän tiedostoa voidaan jo päivittää asiakkaalta. Nykyinen GUI-varasto
        // liitetään palvelimeen ilman kilpailevaa levylukua.
        s.workspace
            .take()
            .ok_or_else(|| invalid("Isäntäkoneen kilpailu puuttuu"))?
    } else {
        Workspace::open(&joined.path)?
    };
    ws.attach_lan(joined.client)?;
    ws.set_bc_view(false)?;
    s.workspace = Some(ws);
    s.lan_host = joined.host;
    s.selection = Selection::None;
    s.selected = -1;
    s.detail = -1;
    s.draft.clear();
    s.edit_draft.clear();
    s.route_draft.clear();
    s.invalid_fields.clear();
    s.invalid_route_fields.clear();
    s.invalid_field = false;
    s.lan_last_notice.clear();
    s.page = 0;
    app.set_page(0);
    app.set_lan_active(true);
    app.set_lan_hosting(s.lan_host.is_some());
    app.set_lan_error("".into());
    if let Some(host) = &s.lan_host {
        app.set_lan_code(host.code.clone().into());
        let address = host.connection_address().map_or_else(
            || format!("portti {}", host.address.port()),
            |a| a.to_string(),
        );
        app.set_lan_host_address(address.into());
        app.set_lan_info(
            "Liity toiselta koneelta tällä koodilla. Kaikilla koneilla on samat muokkausoikeudet."
                .into(),
        );
    } else {
        app.set_lan_info("Kilpailu on yhdistetty. Emit-luvut säilyvät paikallisessa jonossa myös yhteyskatkon aikana.".into());
    }
    update_header(s, app)?;
    update_tables(s, app)?;
    update_form(s, app);
    Ok(())
}
fn poll(s: &mut State, app: &AppWindow) {
    let outcome = s.lan_pending.as_ref().and_then(|rx| rx.try_recv().ok());
    if let Some(result) = outcome {
        s.lan_pending = None;
        app.set_lan_busy(false);
        if let Err(error) = result.and_then(|joined| accept_join(s, app, joined)) {
            app.set_lan_error(error_text(&error).into());
        }
    }
    if let Some(ws) = s.workspace.as_mut() {
        if let Some(client) = ws.lan_client() {
            app.set_lan_status(client.status().into());
        }
        match ws.poll_lan() {
            Ok(true) => {
                if let Err(error) = update_header(s, app).and_then(|()| update_tables(s, app)) {
                    app.set_lan_error(error_text(&error).into());
                }
                // Aktiivista lomaketta, sen syötettä tai muokkauspohjaa ei vaihdeta.
            }
            Ok(false) => {}
            Err(error) => app.set_lan_error(error_text(&error).into()),
        }
    }
    if s.pending_result.is_some()
        && can_follow_readout(s, app)
        && let Err(error) = follow_pending_result(s, app)
    {
        status(app, error_text(&error), true);
    }
    let notice = s
        .workspace
        .as_ref()
        .and_then(Workspace::lan_client)
        .map_or_else(String::new, Client::notice);
    if notice != s.lan_last_notice {
        app.set_lan_error(notice.as_str().into());
        s.lan_last_notice = notice;
    }
}
pub(super) fn bind(app: &AppWindow, state: &SharedState, timer: &Timer) {
    let weak = app.as_weak();
    let shared = state.clone();
    app.on_lan_host(move || {
        if let Some(app) = weak.upgrade()
            && let Err(error) = start_host(&mut shared.borrow_mut(), &app)
        {
            app.set_lan_error(error_text(&error).into());
        }
    });
    let weak = app.as_weak();
    let shared = state.clone();
    app.on_lan_join(move || {
        if let Some(app) = weak.upgrade()
            && let Err(error) = join(&mut shared.borrow_mut(), &app)
        {
            app.set_lan_error(error_text(&error).into());
        }
    });
    let weak = app.as_weak();
    let shared = state.clone();
    app.on_lan_leave(move || {
        if let Some(app) = weak.upgrade()
            && let Err(error) = leave(&mut shared.borrow_mut(), &app)
        {
            app.set_lan_error(error_text(&error).into());
        }
    });
    let weak = app.as_weak();
    app.on_lan_discover(move || {
        let Some(app) = weak.upgrade() else { return; };
        app.set_lan_error("".into());
        app.set_lan_info("Etsitään kilpailua lähiverkosta".into());
        let weak = app.as_weak();
        std::thread::spawn(move || {
            let result = as2026::lan::discover();
            let _ = weak.upgrade_in_event_loop(move |app| match result {
                Ok(found) => {
                    if app.get_lan_active() || app.get_lan_busy() { return; }
                    if let Some(first) = found.first() {
                        app.set_lan_address(first.address.to_string().into());
                        app.set_lan_info(found.iter().map(|d| format!("{} · {}", d.name, d.address))
                            .collect::<Vec<_>>().join("\n").into());
                    } else {
                        app.set_lan_info("Isäntää ei löytynyt. Anna IP-osoite käsin. Molempien koneiden pitää olla samassa lähiverkossa.".into());
                    }
                }
                Err(error) => app.set_lan_error(error_text(&error).into()),
            });
        });
    });
    let weak = app.as_weak();
    let shared = state.clone();
    timer.start(TimerMode::Repeated, Duration::from_millis(250), move || {
        if let Some(app) = weak.upgrade() {
            poll(&mut shared.borrow_mut(), &app);
        }
    });
}
