//! Eristetty natiivi GUI-koe: vain --demo ja synteettiset tulkinnat.
use super::*;
use as2026::tulkki_yhteys::{self, ChangePage, Config, InterpretationEvent};
use serde_json::json;

pub(super) fn start(app: &AppWindow, state: &SharedState, directory: PathBuf) -> GuiResult<Timer> {
    if !app.get_demo() {
        return Err(invalid("Tulkintakoe vaatii --demo-valinnan"));
    }
    std::fs::create_dir_all(&directory).map_err(|e| invalid(&io_message(&e)))?;
    let mut s = state.borrow_mut();
    let ws = s
        .workspace
        .as_mut()
        .ok_or_else(|| invalid("Esimerkkikilpailu puuttuu"))?;
    let rows = ["15", "15", "AT", "JAT", "M"]
        .into_iter()
        .enumerate()
        .map(|(i, v)| (i as u32 + 1, v.to_string()))
        .collect::<Vec<_>>();
    let preview = as2026::interpretation::preview_rows("Syotto_koe", &rows);
    ws.import_right_row(1, &preview)?;
    ws.set_right_row_time_input(1, "r1-e3", Some(true))?;
    ws.set_right_row_time_input(1, "r1-e4", Some(false))?;
    ws.set_right_row_time_input(1, "r1-e5", Some(false))?;
    let project = ws.project().unwrap().clone();
    let pair = project
        .kilpailijat
        .first()
        .ok_or_else(|| invalid("Esimerkkipari puuttuu"))?
        .numero;
    let path = ws.path().to_owned();
    let mut config =
        Config::prepare("https://example.invalid", "koe", "GUI-koe").map_err(|e| invalid(&e))?;
    config.competition_id = 7;
    config.revision = 1;
    config.published_manifest = tulkki_yhteys::manifest(&project)
        .map_err(|e| invalid(&e))?
        .to_string();
    let first:InterpretationEvent=serde_json::from_value(json!({"eventId":10,"pairNumber":pair,"revision":1,"reviewer":"Tarkistaja A","savedAt":"2026-11-10T18:00:00Z","sections":[
        {"id":"r1-j1","missingEntryIds":["r1-e2","r1-e4"],"times":{"r1-e3":"12:34:03"},"note":"Savukokeen huomautus\n".repeat(30),"otherErrors":2,"missingSigns":1,"missingTimes":1,"interpreted":true},
        {"id":"r1-j2","missingEntryIds":[],"times":{},"note":"Maali tarkistettu","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}
    ]})).unwrap();
    tulkki_yhteys::apply_page(
        &mut config,
        ChangePage {
            events: vec![first],
            next_cursor: 10,
            has_more: false,
        },
    )
    .map_err(|e| invalid(&e))?;
    s.preferences
        .as_mut()
        .unwrap()
        .save_tulkki(path.clone(), config)
        .map_err(|e| invalid(&e))?;
    s.page = 7;
    s.filter.clear();
    app.set_page(7);
    app.set_filter("".into());
    app.set_tulkki_view(true);
    s.tulkki_sync_notice = "Synteettinen vastaanotto ajan tasalla".into();
    update_tables(&s, app)?;
    drop(s);
    let shared = state.clone();
    let weak = app.as_weak();
    let phase = Rc::new(std::cell::Cell::new(0));
    let restore_config = Rc::new(RefCell::new(None::<Config>));
    let timer = Timer::default();
    timer.start(TimerMode::Repeated,Duration::from_millis(350),move ||{
        let Some(app)=weak.upgrade()else{return;};
        match phase.get(){
            0=>{
                let row=app.get_tulkki_rows().iter().find(|r|r.key==pair as i32).unwrap();
                assert_eq!(row.cells.row_data(2).unwrap(),"Tulkattu");
                assert_eq!(row.cells.row_data(3).unwrap(),"1");
                assert_eq!(row.cells.row_data(4).unwrap(),"1");
                save_view(&app,&directory.join("0-tulkinnat.png")).unwrap();
                app.invoke_select_tulkki_pair(pair as i32);
            }
            1=>{
                assert!(app.get_tulkki_details_open());
                let text=app.get_tulkki_detail_text();
                assert!(text.contains("15 (jakson rivi 2)"));
                assert!(text.contains("JAT (jakson rivi 4)"));
                assert!(text.contains("12:34:03"));
                assert!(text.contains("Savukokeen huomautus"));
                save_view(&app,&directory.join("1-yksityiskohdat.png")).unwrap();
                app.window().set_size(slint::LogicalSize::new(900.,600.));
            }
            2=>{
                save_view(&app,&directory.join("2-kapea.png")).unwrap();
                app.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled{position:slint::LogicalPosition::new(450.,300.),delta_x:0.,delta_y:-1200.});
            }
            3=>{
                save_view(&app,&directory.join("3-vieritetty.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Tab);
                smoke::key(&app,slint::platform::Key::Escape);
            }
            4=>{
                assert!(!app.get_tulkki_details_open());
                let mut s=shared.borrow_mut();
                let mut config=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let correction:InterpretationEvent=serde_json::from_value(json!({"eventId":11,"pairNumber":pair,"revision":1,"reviewer":"Tarkistaja B","savedAt":"2026-11-10T18:01:00Z","sections":[
                    {"id":"r1-j1","missingEntryIds":["r1-e1"],"times":{"r1-e3":"12:34:03"},"note":"Korjattu tulkinta","otherErrors":1,"missingSigns":1,"missingTimes":0,"interpreted":true}
                ]})).unwrap();
                tulkki_yhteys::apply_page(&mut config,ChangePage{events:vec![correction],next_cursor:11,has_more:false}).unwrap();
                s.preferences.as_mut().unwrap().save_tulkki(path.clone(),config.clone()).unwrap();
                let reopened=PreferencesStore::open_at(s.preferences.as_ref().unwrap().path(),&directory.join("Kilpailut")).unwrap();
                assert_eq!(reopened.get().tulkki_yhteydet[&path],config);
                assert_eq!(s.workspace.as_ref().unwrap().project().unwrap(),&project,"Vastaanotto ei saa muuttaa laskentatietoja");
                tulkki::refresh(&s,&app);
                let row=app.get_tulkki_rows().iter().find(|r|r.key==pair as i32).unwrap();
                assert_eq!(row.cells.row_data(3).unwrap(),"1");assert_eq!(row.cells.row_data(4).unwrap(),"0");
                drop(s);app.invoke_select_tulkki_pair(pair as i32);
            }
            5=>{
                let text=app.get_tulkki_detail_text();
                assert!(text.contains("15 (jakson rivi 1)"));assert!(!text.contains("Savukokeen huomautus"));
                assert!(text.contains("Korjattu tulkinta"));assert!(text.contains("Maali tarkistettu"));
                save_view(&app,&directory.join("4-korjattu.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Escape);
                let mut s=shared.borrow_mut();s.tulkki_sync_notice="Vastaanotto ei ole ajan tasalla: synteettinen verkkokatko".into();
                app.set_tulkki_sync_error(true);tulkki::refresh(&s,&app);
            }
            6=>{
                save_view(&app,&directory.join("5-yhteyskatko.png")).unwrap();
                assert!(app.get_tulkki_sync_error());assert!(app.get_tulkki_sync_status().contains("ei ole ajan tasalla"));
                let mut s=shared.borrow_mut();s.tulkki_sync_notice="Vastaanotto ajan tasalla".into();app.set_tulkki_sync_error(false);tulkki::refresh(&s,&app);
                drop(s);app.invoke_filter_changed(pair.to_string().into());
            }
            7=>{
                assert_eq!(app.get_tulkki_rows().row_count(),1);
                save_view(&app,&directory.join("6-palautunut.png")).unwrap();
                app.window().set_size(slint::LogicalSize::new(1480.,960.));
                app.invoke_navigate(3);app.invoke_filter_changed("".into());app.invoke_select_row(0);
                app.set_details_open(false);
            }
            8=>{
                assert!(app.get_headers().iter().any(|h|h=="Tulkkaus"));
                let column=app.get_headers().iter().position(|h|h=="Tulkkaus").unwrap();
                let row=app.get_rows().iter().find(|r|r.cells.row_data(1).as_deref()==Some(&pair.to_string())).unwrap();
                assert_eq!(row.cells.row_data(column).unwrap(),"Tulkattu");
                assert_eq!(app.get_result_tulkki_status(),"Tulkattu");assert!(app.get_result_tulkki_available());
                assert!(app.get_result_tulkki_has_data());
                assert_eq!(app.get_result_tulkki_missing_signs(),"1");
                assert_eq!(app.get_result_tulkki_missing_times(),"0");
                assert_eq!(app.get_result_tulkki_other_errors(),"1");
                save_view(&app,&directory.join("7-tulosluettelo.png")).unwrap();
                app.set_details_open(true);
            }
            9=>{
                save_view(&app,&directory.join("8-tuloksen-yhteenveto.png")).unwrap();
                app.invoke_open_result_tulkki();
                assert!(app.get_tulkki_details_open());assert_eq!(app.get_tulkki_pair(),pair as i32);
                assert!(app.get_tulkki_detail_text().contains("Korjattu tulkinta"));
            }
            10=>{
                save_view(&app,&directory.join("9-tulkinta-tuloksesta.png")).unwrap();
                smoke::key(&app,slint::platform::Key::Escape);
                let mut s=shared.borrow_mut();
                let full=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                *restore_config.borrow_mut()=Some(full.clone());
                let mut partial=full;partial.received.retain(|r|r.section.id!="r1-j2");
                s.preferences.as_mut().unwrap().save_tulkki(path.clone(),partial).unwrap();
                s.edit_draft.insert(0,"Keskeneräinen muokkaus".into());s.invalid_field=true;
                let selected=s.selected;let detail=s.detail;let draft=s.edit_draft.clone();
                let column=app.get_headers().iter().position(|h|h=="Tulkkaus").unwrap();
                let before=app.get_rows().iter().map(|r|(r.key,r.cells.iter().take(column).collect::<Vec<_>>())).collect::<Vec<_>>();
                tulkki::refresh(&s,&app);
                assert_eq!(app.get_result_tulkki_status(),"1/2 jaksoa");
                for (row,(key,cells)) in app.get_rows().iter().zip(before) {assert_eq!(row.key,key);assert_eq!(row.cells.iter().take(column).collect::<Vec<_>>(),cells);assert_eq!(row.cells.row_data(column).unwrap(),"1/2 jaksoa");}
                assert_eq!(s.selected,selected);assert_eq!(s.detail,detail);assert_eq!(s.edit_draft,draft);assert!(s.invalid_field);
                s.edit_draft.clear();s.invalid_field=false;
            }
            11=>{
                save_view(&app,&directory.join("10-osittainen-tulkinta.png")).unwrap();
                let mut s=shared.borrow_mut();s.preferences.as_mut().unwrap().save_tulkki(path.clone(),restore_config.borrow_mut().take().unwrap()).unwrap();
                tulkki::refresh(&s,&app);assert_eq!(app.get_result_tulkki_status(),"Tulkattu");
                assert_eq!(s.workspace.as_ref().unwrap().project().unwrap(),&project);
                app.set_details_open(false);
            }
            12=>{
                let mut s=shared.borrow_mut();
                let config=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let ws=s.workspace.as_mut().unwrap();
                assert!(ws.apply_tulkki_penalties(&config).unwrap());
                let penalties=&ws.project().unwrap().kilpailijat.iter().find(|d|d.numero==pair).unwrap().lisavirheet;
                assert_eq!(penalties.puuttuvat_rt_kpl,1);
                assert_eq!(penalties.puuttuvat_rt_pisteet,600);
                assert_eq!(penalties.muut_virhepisteet,project.kilpailijat.iter().find(|d|d.numero==pair).unwrap().lisavirheet.muut_virhepisteet+600);
                assert!(!ws.apply_tulkki_penalties(&config).unwrap());
                update_tables(&s,&app).unwrap();update_form(&s,&app);
                app.set_details_open(true);
            }
            13=>{
                save_view(&app,&directory.join("11-tulosnakyma-palautunut.png")).unwrap();
                let mut s=shared.borrow_mut();
                for number in [2,3] {s.workspace.as_mut().unwrap().add_track(as2026::workspace::TrackEdit{number:number.to_string(),name:format!("Tuontirata {number}")}).unwrap();}
                s.right_row_previews=[("Syotto_B",["AT1","JAT3","M"]),("Syotto_C",["AT2","JAT4","M"])].into_iter().map(|(sheet,labels)|as2026::interpretation::preview_rows(sheet,&labels.into_iter().enumerate().map(|(i,v)|(i as u32+1,v.to_owned())).collect::<Vec<_>>())).collect();
                s.right_row_routes=vec![1,2,3];
                s.pending=Some(PendingAction::Notice);
                app.set_right_row_sheets(model(vec!["Syotto_B".into(),"Syotto_C".into()]));
                app.set_right_row_routes(model(vec!["1".into(),"2".into(),"3".into()]));
                app.set_right_row_sheet(0);app.set_right_row_route(1);app.set_right_row_view_only(false);
                app.set_confirm_title("Tuo oikea rivi".into());app.set_confirm_action_label("Tallenna oikea rivi".into());
                app.set_right_row_open(true);app.set_confirm_open(true);
                right_row::refresh(&mut s,&app).unwrap();drop(s);
                app.invoke_confirm_action(true);
            }
            14=>{
                assert!(app.get_right_row_open());assert!(app.get_confirm_open());
                assert_eq!(shared.borrow().right_row_previews.len(),2);
                assert!(shared.borrow().workspace.as_ref().unwrap().project().unwrap().oikeat_rivit.iter().any(|r|r.rata==2));
                save_view(&app,&directory.join("12-ensimmainen-rata-tallennettu.png")).unwrap();
                app.set_right_row_sheet(1);app.set_right_row_route(2);app.invoke_right_row_selection();app.invoke_confirm_action(true);
            }
            15=>{
                assert!(app.get_right_row_open());assert!(app.get_confirm_open());
                let s=shared.borrow();let p=s.workspace.as_ref().unwrap().project().unwrap();
                assert_eq!(p.oikeat_rivit.iter().find(|r|r.rata==2).unwrap().merkinnat[0].merkinta.label,"AT1");
                assert_eq!(p.oikeat_rivit.iter().find(|r|r.rata==3).unwrap().merkinnat[0].merkinta.label,"AT2");drop(s);
                save_view(&app,&directory.join("13-toinen-rata-tallennettu.png")).unwrap();
                app.invoke_confirm_action(false);assert!(!app.get_right_row_open());assert!(!app.get_confirm_open());
                app.invoke_navigate(7);app.set_tulkki_view(false);app.set_signs_route(0);
                right_row::update_page(&shared.borrow(),&app);
            }
            16=>{
                save_view(&app,&directory.join("14-kilvet-kohdistus.png")).unwrap();
                let mut p=as2026::native::Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml")).unwrap();
                p.kilpailu.ajanottotapa=0;p.leimaluvut.clear();p.kilpailijat[0].emit=0;
                let rows=["08","AT1","AT2","M"].into_iter().enumerate().map(|(i,v)|(i as u32+1,v.to_owned())).collect::<Vec<_>>();
                as2026::interpretation::import_route(&mut p,1,&as2026::interpretation::preview_rows("Syotto_A",&rows)).unwrap();
                let path=directory.join(format!("paperikortti-{}.toml",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
                as2026::storage::ProjectStore::create(&path,p).unwrap();
                let mut s=shared.borrow_mut();s.workspace=Some(Workspace::open(&path).unwrap());s.filter.clear();s.result_class.clear();drop(s);
                app.invoke_navigate(3);app.invoke_select_row(0);
            }
            17=>{
                printing::open_entry(&mut shared.borrow_mut(),&app).unwrap();assert!(app.get_result_entry_open());
                app.invoke_result_entry_edited(2,"180452".into());
            }
            18=>{smoke::key(&app,slint::platform::Key::Tab);}
            19=>{
                assert_eq!(app.get_result_entry_fields().iter().find(|f|f.key==2).unwrap().value,"18:04:52");
                for (key,value) in [(2,"-"),(3,"121000"),(4,"121100")] {app.invoke_result_entry_edited(key,value.into());}
                app.invoke_result_entry_save();assert!(!app.get_result_entry_open());
                assert!(!shared.borrow().entry_dirty);
                let s=shared.borrow();let p=s.workspace.as_ref().unwrap().project().unwrap();assert!(p.leimaluvut.is_empty());assert_eq!(p.kilpailijat[0].emit,0);
                let result=p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();assert!(result.ilman_emit);assert!(result.hyvaksytty);assert!(result.can_approve());assert_eq!(p.kilpailijat[0].lisavirheet.muut_virhepisteet,1800);
            }
            20=>{
                save_view(&app,&directory.join("15-manuaalinen-tulos.png")).unwrap();
                let mut s=shared.borrow_mut();let ws=s.workspace.as_mut().unwrap();let p=ws.project().unwrap();
                let mut config=Config{competition_id:7,revision:1,..Config::default()};config.published_manifest=tulkki_yhteys::manifest(p).unwrap().to_string();
                let event=serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Puhelimen koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":["r1-e1","r1-e2"],"times":{"r1-e3":"12:10:30","r1-e4":"12:11:00"},"note":"AT1 puuttuu","otherErrors":1,"missingSigns":1,"missingTimes":1,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut config,ChangePage{events:vec![event],next_cursor:1,has_more:false}).unwrap();
                assert!(ws.apply_tulkki_penalties(&config).unwrap());assert!(!ws.apply_tulkki_penalties(&config).unwrap());
                ws.reload().unwrap();let p=ws.project().unwrap();let d=&p.kilpailijat[0];let r=d.tallennettu_tulos.as_ref().unwrap();
                assert!(p.leimaluvut.is_empty());assert!(!r.hyvaksytty);assert_eq!(r.rivit[2].clock,[12,10,30]);assert_eq!(d.lisavirheet.muut_virhepisteet,2400);assert_eq!(d.lisavirheet.puuttuvat_rt_kpl,1);
                let mut preview=p.clone();preview.kilpailijat[0].tallennettu_tulos.as_mut().unwrap().hyvaksytty=true;
                let document=as2026::printing::competitor(&preview,7,None).unwrap();assert!(document.pages.iter().flat_map(|p|&p.texts).any(|t|t.value=="Puuttuvat"));
                as2026::printing::preview_png(&document,&directory.join("tuloslappu-puutteet")).unwrap();
                let path=ws.path().to_owned();s.preferences.as_mut().unwrap().save_tulkki(path,config).unwrap();update_tables(&s,&app).unwrap();update_form(&s,&app);app.set_details_open(true);
            }
            21=>{
                save_view(&app,&directory.join("16-puhelimen-ajat-ja-at-puute.png")).unwrap();
                app.invoke_navigate(7);app.invoke_open_rt_summary();assert!(app.get_rt_summary_open());
            }
            22=>{
                assert_eq!(app.get_rt_summary_rows().row_count(),1);let row=app.get_rt_summary_rows().row_data(0).unwrap();assert_eq!(row.cells.row_data(2).unwrap(),"08");assert_eq!(row.cells.row_data(3).unwrap(),"1");assert_eq!(row.cells.row_data(5).unwrap(),"7");
                save_view(&app,&directory.join("18-rt-puutteiden-koonti.png")).unwrap();app.window().set_size(slint::LogicalSize::new(900.,600.));
            }
            23=>{
                save_view(&app,&directory.join("19-rt-koonti-pieni.png")).unwrap();
                let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();let mut config=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let event=serde_json::from_value(json!({"eventId":2,"pairNumber":7,"revision":1,"reviewer":"Puhelimen koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e2":"12:05:00","r1-e3":"12:10:30","r1-e4":"12:11:00"},"note":"AT1 korjattu","otherErrors":1,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut config,ChangePage{events:vec![event],next_cursor:2,has_more:false}).unwrap();
                let ws=s.workspace.as_mut().unwrap();assert!(ws.apply_tulkki_penalties(&config).unwrap());ws.reload().unwrap();assert_eq!(ws.project().unwrap().kilpailijat[0].lisavirheet.muut_virhepisteet,600);
                s.preferences.as_mut().unwrap().save_tulkki(path,config).unwrap();update_tables(&s,&app).unwrap();update_form(&s,&app);
                assert!(app.get_rt_summary_open());assert_eq!(app.get_rt_summary_rows().row_count(),0);
            }
            24=>{
                save_view(&app,&directory.join("20-rt-koonti-paivittynyt.png")).unwrap();smoke::key(&app,slint::platform::Key::Escape);assert!(!app.get_rt_summary_open());app.window().set_size(slint::LogicalSize::new(1480.,960.));app.invoke_navigate(3);app.invoke_select_row(0);app.set_details_open(true);
            }
            25=>{
                save_view(&app,&directory.join("17-puhelimen-korjattu-aika.png")).unwrap();
                let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();let mut config=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let ws=s.workspace.as_mut().unwrap();
                ws.complete_result_entry_with_missing(7,&[(3,[12,10,30]),(4,[12,11,0])],&[2],0,600).unwrap();
                assert_eq!(ws.project().unwrap().kilpailijat[0].lisavirheet.muut_virhepisteet,2400);assert!(!ws.apply_tulkki_penalties(&config).unwrap());
                let event=serde_json::from_value(json!({"eventId":3,"pairNumber":7,"revision":1,"reviewer":"Puhelimen koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e2":"12:05:00","r1-e3":"12:10:30","r1-e4":"12:11:00"},"note":"Paikallinen AT-puute korjattu puhelimessa","otherErrors":1,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut config,ChangePage{events:vec![event],next_cursor:3,has_more:false}).unwrap();
                assert!(ws.apply_tulkki_penalties(&config).unwrap());assert_eq!(ws.project().unwrap().kilpailijat[0].lisavirheet.muut_virhepisteet,600);
                ws.complete_result_entry_with_missing(7,&[(3,[12,10,30]),(4,[12,11,0])],&[2],0,600).unwrap();
                ws.edit_station_result(7,2,as2026::workspace::ResultField::Clock,"12:05:00").unwrap();ws.recalculate_stored(7).unwrap();ws.reload().unwrap();
                let d=&ws.project().unwrap().kilpailijat[0];assert_eq!(d.lisavirheet.muut_virhepisteet,600);assert_eq!(d.lisavirheet.tulkki_siirto.as_ref().unwrap().muut_virhepisteet,600);assert_eq!(d.tallennettu_tulos.as_ref().unwrap().puuttuvien_at_pisteet,0);
                config.site="https://example.invalid/".into();config.join_code="ABCDEF123456".into();
                let path=ws.path().to_owned();s.preferences.as_mut().unwrap().save_tulkki(path,config).unwrap();drop(s);
                app.set_details_open(false);app.invoke_open_tulkki();assert!(app.get_tulkki_qr_ready());app.invoke_open_tulkki_qr();assert!(app.get_tulkki_qr_open());
                assert!(app.get_tulkki_qr_link().contains("#join=ABCDEF123456"));
            }
            26=>{save_view(&app,&directory.join("21-liittymisen-qr.png")).unwrap();app.window().set_size(slint::LogicalSize::new(900.,600.));}
            27=>{save_view(&app,&directory.join("22-liittymisen-qr-pieni.png")).unwrap();smoke::key(&app,slint::platform::Key::Escape);assert!(!app.get_tulkki_qr_open());assert!(app.get_tulkki_open());app.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled{position:slint::LogicalPosition::new(400.,250.),delta_x:0.,delta_y:-1600.});
            }
            28=>{
                save_view(&app,&directory.join("23-yhteyden-hallinta.png")).unwrap();app.invoke_close_tulkki();
                let mut s=shared.borrow_mut();s.print_test=true;let path=s.workspace.as_ref().unwrap().path().to_owned();let mut c=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let event=serde_json::from_value(json!({"eventId":4,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-11T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":["r1-e1"],"times":{"r1-e2":"12:05:00","r1-e3":"12:10:30","r1-e4":"12:11:00"},"note":"RT puuttuu","otherErrors":1,"missingSigns":1,"missingTimes":0,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut c,ChangePage{events:vec![event],next_cursor:4,has_more:false}).unwrap();let ws=s.workspace.as_mut().unwrap();ws.apply_tulkki_penalties(&c).unwrap();ws.approve_result(7).unwrap();
                s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();drop(s);app.invoke_navigate(7);app.invoke_sign_void_toggled(0,true);assert!(app.get_confirm_open());
            }
            29=>{
                save_view(&app,&directory.join("24-mitatoinnin-esikatselu.png")).unwrap();app.invoke_confirm_action(false);
                assert!(!shared.borrow().workspace.as_ref().unwrap().project().unwrap().oikeat_rivit[0].merkinnat[0].mitatoity);
                app.invoke_sign_void_toggled(0,true);
                {let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();let mut c=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();c.last_received_at="12.11.2026 23:59:59".into();s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();}
                app.invoke_confirm_action(true);
                let s=shared.borrow();let p=s.workspace.as_ref().unwrap().project().unwrap();assert!(p.oikeat_rivit[0].merkinnat[0].mitatoity);assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl,0);assert_eq!(p.kilpailijat[0].lisavirheet.muut_virhepisteet,600);assert!(p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);assert!(app.get_confirm_open());
            }
            30=>{
                save_view(&app,&directory.join("25-muuttuneiden-tulostus.png")).unwrap();app.invoke_confirm_action(true);assert!(shared.borrow().last_print.is_some());
                app.invoke_sign_void_toggled(0,false);let mut s=shared.borrow_mut();s.workspace.as_mut().unwrap().reload().unwrap();assert!(s.workspace.as_ref().unwrap().project().unwrap().oikeat_rivit[0].merkinnat[0].mitatoity);right_row::update_page(&s,&app);assert_eq!(app.get_signs_rows().row_data(0).unwrap().cells.row_data(4).unwrap(),"1");
            }
            31=>{
                save_view(&app,&directory.join("26-kilpi-mitatoity.png")).unwrap();
                let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();
                let mut c=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();
                let latest=tulkki_yhteys::manifest(s.workspace.as_ref().unwrap().project().unwrap()).unwrap();
                tulkki_yhteys::reconcile_manifest(&mut c,&latest).unwrap();c.automatic_results=true;
                s.preferences.as_mut().unwrap().save_tulkki(path.clone(),c).unwrap();s.last_print=None;
                tulkki::process_automatic(&mut s,&app).unwrap();assert!(s.last_print.is_some());
                assert!(s.workspace.as_ref().unwrap().project().unwrap().kilpailijat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
                assert!(!s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].automatic_prints.is_empty());
                s.last_print=None;tulkki::process_automatic(&mut s,&app).unwrap();assert!(s.last_print.is_none(),"samaa tulosta ei tulosteta kahdesti");
                drop(s);app.invoke_open_tulkki();assert!(app.get_tulkki_automatic());
                app.window().dispatch_event(slint::platform::WindowEvent::PointerScrolled{position:slint::LogicalPosition::new(400.,250.),delta_x:0.,delta_y:-1600.});
            }
            32=>{
                save_view(&app,&directory.join("27-automaattitulokset.png")).unwrap();
                app.invoke_tulkki_automatic_changed(false);assert!(!app.get_tulkki_automatic());
                app.invoke_close_tulkki();
                let mut s=shared.borrow_mut();right_row::open(&mut s,&app,true).unwrap();
                s.right_row_previews[0]=as2026::interpretation::preview_rows("Korjattu",&[(1,"08".into()),(2,"09".into()),(3,"AT1".into()),(4,"AT2".into()),(5,"M".into())]);
                app.set_right_row_view_only(false);right_row::refresh(&mut s,&app).unwrap();drop(s);
                app.invoke_confirm_action(true);assert_eq!(app.get_confirm_action_label(),"Korvaa oikea rivi");
            }
            33=>{
                assert!(app.get_confirm_text().contains("Uusia kilpiä ei merkitä automaattisesti puuttuviksi"));
                save_view(&app,&directory.join("28-korvaamisen-esikatselu.png")).unwrap();
                app.invoke_confirm_action(true);
                let mut s=shared.borrow_mut();s.workspace.as_mut().unwrap().reload().unwrap();
                let p=s.workspace.as_ref().unwrap().project().unwrap();assert_eq!(p.oikeat_rivit[0].merkinnat.len(),5);assert!(p.oikeat_rivit[0].merkinnat[0].mitatoity);assert_eq!(p.oikeat_rivit[0].merkinnat[1].merkinta.label,"09");assert_eq!(p.oikeat_rivit[0].merkinnat[2].tunniste,"r1-e2");
                assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl,0);assert_eq!(p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().rivit[2].clock,[12,10,30]);
            }
            34=>{
                save_view(&app,&directory.join("29-oikea-rivi-korvattu.png")).unwrap();
                app.invoke_confirm_action(false);
                let mut p=as2026::native::Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml")).unwrap();p.kilpailu.ajanottotapa=0;p.leimaluvut.clear();p.kilpailijat[0].emit=0;
                let old=p.radat[0].asemat.clone();let mut jat=old[2].clone();jat.tyyppi="JAT".into();jat.nimi="JAT3".into();let mut ula=jat.clone();ula.tyyppi="ULA".into();ula.nimi="ULA3".into();ula.tavoiteaika_sekuntia=0;
                p.radat[0].asemat=vec![old[0].clone(),old[1].clone(),jat,ula,old[2].clone(),old[3].clone()];
                as2026::interpretation::import_route(&mut p,1,&as2026::interpretation::preview_rows("ULA-koe",&[(1,"AT1".into()),(2,"JAT3".into()),(3,"AT2".into()),(4,"M".into())])).unwrap();
                let mut c=Config{competition_id:7,revision:1,..Config::default()};c.published_manifest=tulkki_yhteys::manifest(&p).unwrap().to_string();
                let event=serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-12T12:00:00Z","sections":[{"id":"r1-j1","missingEntryIds":[],"times":{"r1-e1":"12:05:00","r1-e2":"12:10:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true},{"id":"r1-j2","missingEntryIds":[],"times":{"r1-e3":"12:22:00","r1-e4":"12:22:00"},"note":"","otherErrors":0,"missingSigns":0,"missingTimes":0,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut c,ChangePage{events:vec![event],next_cursor:1,has_more:false}).unwrap();
                let path=directory.join(format!("ula-{}.toml",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));as2026::storage::ProjectStore::create(&path,p).unwrap();
                let mut s=shared.borrow_mut();s.workspace=Some(Workspace::open(&path).unwrap());s.workspace.as_mut().unwrap().apply_tulkki_penalties(&c).unwrap();let path=s.workspace.as_ref().unwrap().path().to_owned();s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();s.selection=Selection::None;
                right_row::open(&mut s,&app,true).unwrap();s.right_row_previews[0]=as2026::interpretation::preview_rows("ULA-koe",&[(1,"AT1".into()),(2,"JAT3".into()),(3,"ULA3".into()),(4,"AT2".into()),(5,"M".into())]);app.set_right_row_view_only(false);right_row::refresh(&mut s,&app).unwrap();drop(s);
                app.invoke_confirm_action(true);assert_eq!(app.get_confirm_action_label(),"Korvaa oikea rivi");assert_eq!(app.get_right_row_error(),"");
            }
            35=>{
                save_view(&app,&directory.join("30-ULA-lisaamisen-esikatselu.png")).unwrap();app.invoke_confirm_action(true);assert_eq!(app.get_right_row_error(),"");
                let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();let p=s.workspace.as_ref().unwrap().project().unwrap();let r=p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap();assert_eq!(r.pending_manual_clocks(),vec![4]);assert!(r.puuttuvat_aikamerkinnat.is_empty());assert_eq!(r.rivit[4].clock,[12,22,0]);
                let mut c=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();tulkki_yhteys::reconcile_manifest(&mut c,&tulkki_yhteys::manifest(p).unwrap()).unwrap();c.revision=2;assert_eq!(c.received[0].section.pending_time_ids,vec!["r1-e5"]);s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();drop(s);app.invoke_confirm_action(false);app.invoke_select_tulkki_pair(7);assert!(app.get_tulkki_detail_text().contains("Täydennettävät ULA-ajat: ULA3"));
            }
            36=>{
                save_view(&app,&directory.join("31-ULA-ajan-taydennys.png")).unwrap();
                app.set_tulkki_details_open(false);
                let mut p=as2026::native::Project::from_toml(include_str!("../../fixtures/native-demo/kilpailu.toml")).unwrap();p.kilpailu.ajanottotapa=0;p.leimaluvut.clear();p.kilpailijat[0].emit=0;p.radat[0].numero=2;p.radat[0].nimi="C".into();p.luokat[0].tunnus="C".into();p.luokat[0].rata=2;p.kilpailijat[0].luokka="C".into();
                as2026::interpretation::import_route(&mut p,2,&as2026::interpretation::preview_rows("Syotto_C",&[(1,"08".into()),(2,"AT1".into()),(3,"AT2".into()),(4,"M".into())])).unwrap();
                let mut c=Config{competition_id:7,revision:1,last_received_at:"12.11.2026 12:00:00".into(),..Config::default()};c.published_manifest=tulkki_yhteys::manifest(&p).unwrap().to_string();
                let event=serde_json::from_value(json!({"eventId":1,"pairNumber":7,"revision":1,"reviewer":"Koe","savedAt":"2026-11-12T12:00:00Z","sections":[{"id":"r2-j1","missingEntryIds":["r2-e1"],"times":{"r2-e2":"12:05:00","r2-e3":"12:10:00","r2-e4":"12:11:00"},"note":"C-rata","otherErrors":0,"missingSigns":1,"missingTimes":0,"interpreted":true}]})).unwrap();
                tulkki_yhteys::apply_page(&mut c,ChangePage{events:vec![event],next_cursor:1,has_more:false}).unwrap();
                let path=directory.join(format!("c-mitatointi-{}.toml",std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));as2026::storage::ProjectStore::create(&path,p).unwrap();
                let mut s=shared.borrow_mut();s.workspace=Some(Workspace::open(&path).unwrap());let ws=s.workspace.as_mut().unwrap();ws.apply_tulkki_penalties(&c).unwrap();ws.approve_result(7).unwrap();let path=ws.path().to_owned();s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();s.selection=Selection::None;s.filter.clear();drop(s);app.invoke_navigate(7);app.set_signs_route(0);right_row::update_page(&shared.borrow(),&app);app.invoke_sign_void_toggled(0,true);assert!(app.get_confirm_text().contains("Rata 2"));
            }
            37=>{
                save_view(&app,&directory.join("32-C-radan-mitatointi.png")).unwrap();
                {let mut s=shared.borrow_mut();let path=s.workspace.as_ref().unwrap().path().to_owned();let mut c=s.preferences.as_ref().unwrap().get().tulkki_yhteydet[&path].clone();c.last_received_at="12.11.2026 12:00:05".into();s.preferences.as_mut().unwrap().save_tulkki(path,c).unwrap();}
                app.invoke_confirm_action(true);
                let s=shared.borrow();let p=s.workspace.as_ref().unwrap().project().unwrap();assert!(p.oikeat_rivit[0].merkinnat[0].mitatoity);assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_kpl,0);assert_eq!(p.kilpailijat[0].lisavirheet.puuttuvat_rt_pisteet,0);assert!(p.kilpailijat[0].tallennettu_tulos.as_ref().unwrap().hyvaksytty);
            }
            38=>{
                app.invoke_confirm_action(true);assert!(shared.borrow().last_print.is_some());let mut s=shared.borrow_mut();s.workspace.as_mut().unwrap().reload().unwrap();assert!(s.workspace.as_ref().unwrap().project().unwrap().oikeat_rivit[0].merkinnat[0].mitatoity);drop(s);
                save_view(&app,&directory.join("33-C-kilpi-mitatoity.png")).unwrap();
                std::fs::write(directory.join("result.txt"),"PASS: aiemmat GUI-kokeet sekä C-radan mitätöinti taustahaun kellonajan muuttuessa, pistekorjaus, hyväksynnän säilyminen, tulostus ja uudelleenavaus").unwrap();slint::quit_event_loop().unwrap();
            }            _=>{}
        }
        phase.set(phase.get()+1);
    });
    Ok(timer)
}
