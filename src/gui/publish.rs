//! Live-julkaisun paikallinen ohjaus. Verkkoyhteyttä ei avata automaattisesti.
use super::*;
use as2026::live_publish::{Config, Publisher};
fn configuration(app: &AppWindow) -> GuiResult<Config> {
    let field = |key| {
        app.get_publish_fields()
            .iter()
            .find(|f| f.key == key)
            .map(|f| f.value.to_string())
            .unwrap_or_default()
    };
    let config = Config {
        ftp: app.get_publish_protocol() == 1,
        host: field(0).trim().into(),
        port: field(1)
            .parse()
            .map_err(|_| invalid("Anna portti 1–65535."))?,
        user: field(2),
        password: field(3),
        key_file: field(4),
        fingerprint: field(5),
        directory: field(6),
        filename: field(7),
        public_url: field(8),
    };
    config.validate().map_err(|e| invalid(&e))?;
    Ok(config)
}
pub(super) fn bind(state: &SharedState, app: &AppWindow, timer: &Timer) {
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_open_publish(move || {if let Some(app)=weak.upgrade(){
        let s=shared.borrow();
        let Some(ws)=s.workspace.as_ref().filter(|w|w.project().is_some()) else {status(&app,"Avaa ensin AS2026-kilpailu.",true);return};
        let config=s.preferences.as_ref().and_then(|p|p.get().live_julkaisut.get(ws.path())).cloned().unwrap_or(Config {port:22,filename:"tulokset.html".into(),..Default::default()});
        let values=[config.host,config.port.to_string(),config.user,String::new(),config.key_file,config.fingerprint,config.directory,config.filename,config.public_url];
        let labels=["Palvelin","Portti (SFTP 22, FTP 21)","Käyttäjätunnus","Salasana tai yksityisen avaimen salasana (ei tallenneta)","Yksityisen avaimen paikallinen tiedostopolku (tyhjä = salasana)","SFTP-palvelimen SHA256-heksasormenjälki","Palvelimen kohdehakemisto","Tulostiedoston nimi (olemassa oleva tiedosto korvataan)","Julkinen tulososoite (valinnainen)"];
        app.set_publish_fields(model(values.into_iter().zip(labels).enumerate().map(|(key,(value,label))|ResultEntryInput{key:key as i32,label:label.into(),value:value.into()}).collect()));
        app.set_publish_protocol(i32::from(config.ftp));
        app.set_publish_running(s.publisher.is_some());
        if s.publisher.is_none(){app.set_publish_status("Ei käynnissä. Testaa yhteys ennen aloittamista. Tulossivun päivitysväli on 60 s.".into());}
        app.set_publish_open(true);
    }});
    let weak = app.as_weak();
    app.on_publish_edited(move |key, value| {
        if let Some(app) = weak.upgrade() {
            let fields = app.get_publish_fields();
            if let Some(i) = fields.iter().position(|f| f.key == key) {
                let mut field = fields.row_data(i).unwrap();
                field.value = value;
                fields.set_row_data(i, field);
            }
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_publish_test(move || {
        if let Some(app) = weak.upgrade() {
            let result: GuiResult<()> = (|| {
                let config = configuration(&app)?;
                let (tx, rx) = std::sync::mpsc::channel();
                std::thread::Builder::new()
                    .name("as2026-live-yhteyskoe".into())
                    .spawn(move || {
                        let _ = tx.send(as2026::live_publish::transfer(&config, None));
                    })
                    .map_err(|_| invalid("Yhteyskokeen käynnistys epäonnistui."))?;
                shared.borrow_mut().publish_test = Some(rx);
                app.set_publish_busy(true);
                app.set_publish_status(
                    "Testataan kirjautuminen ja kohdehakemisto. Tiedostoja ei julkaista.".into(),
                );
                Ok(())
            })();
            if let Err(error) = result {
                app.set_publish_status(error.to_string().into());
            }
        }
    });
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_publish_toggle(move||{if let Some(app)=weak.upgrade(){
        let result: GuiResult<()>=(||{let mut s=shared.borrow_mut();if s.publisher.take().is_some(){app.set_publish_running(false);app.set_publish_status("Julkaisu pysäytetty. Palvelimella oleva tulossivu säilyy. Jo käynnissä oleva siirto voi vielä valmistua.".into());return Ok(())}
            let config=configuration(&app)?;
            let ws=s.workspace.as_ref().ok_or_else(||invalid("Avaa kilpailu."))?;let path=ws.path().to_owned();
            if !app.get_demo() && let Some(preferences)=&mut s.preferences {preferences.save_publish(path.clone(),config.clone()).map_err(|e|invalid(&e))?;}
            let publisher=Publisher::start(config,path).map_err(|e|invalid(&e))?;
            if let Some(project)=s.workspace.as_ref().and_then(Workspace::project){publisher.offer(project);}
            s.publisher=Some(publisher);app.set_publish_running(true);app.set_publish_status("Julkaisu aloitettiin. Uusin tulostilanne lähetetään taustalla.".into());Ok(())})();
        if let Err(error)=result {app.set_publish_status(error.to_string().into());}
    }});
    let shared = state.clone();
    let weak = app.as_weak();
    app.on_publish_finish(move || { if let Some(app) = weak.upgrade() {
        let s = shared.borrow();
        if let (Some(publisher), Some(project)) = (&s.publisher, s.workspace.as_ref().and_then(Workspace::project)) {
            match publisher.finish(project) {
                Ok(()) => app.set_publish_status("Lopullisia tuloksia lähetetään. Julkaisu päättyy onnistuneen lähetyksen jälkeen.".into()),
                Err(error) => app.set_publish_status(error.into()),
            }
        }
    }});
    let shared = state.clone();
    let weak = app.as_weak();
    timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
        if let Some(app) = weak.upgrade() {
            let mut s = shared.borrow_mut();
            if let Some(rx) = &s.publish_test
                && let Ok(result) = rx.try_recv()
            {
                app.set_publish_busy(false);
                app.set_publish_status(
                    result
                        .map_or_else(
                            |e| e,
                            |()| {
                                "Yhteys ja kohdehakemisto toimivat. Tiedostoja ei julkaistu.".into()
                            },
                        )
                        .into(),
                );
                s.publish_test = None;
            }
            if s.publisher
                .as_ref()
                .is_some_and(|p| s.workspace.as_ref().is_none_or(|ws| ws.path() != p.path))
            {
                s.publisher = None;
                app.set_publish_running(false);
                app.set_publish_status("Julkaisu pysäytettiin kilpailun vaihtuessa.".into());
            }
            if s.publisher.as_ref().is_some_and(Publisher::is_finished) {
                app.set_publish_status(s.publisher.as_ref().unwrap().status().into());
                app.set_publish_running(false);
                s.publisher = None;
            }
            if let Some(publisher) = &s.publisher {
                if let Some(project) = s.workspace.as_ref().and_then(Workspace::project) {
                    publisher.offer(project);
                }
                app.set_publish_status(publisher.status().into());
            }
        }
    });
}
