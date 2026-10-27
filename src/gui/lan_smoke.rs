//! Kahden erillisen GUI-prosessin synteettinen koe. Käytettävissä vain --demo-ajossa.
use super::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Pair {
    address: String,
    code: String,
}

fn frame(card: u32) -> emit::EmitFrame {
    let mut bytes = [0u8; emit::FRAME_LENGTH];

    bytes[..2].copy_from_slice(&[255, 255]);

    bytes[2..5].copy_from_slice(&card.to_le_bytes()[..3]);

    for (i, (code, time)) in [(0u8, 0u16), (31, 300), (32, 600), (250, 660)]
        .into_iter()
        .enumerate()
    {
        let offset = 10 + i * 3;

        bytes[offset] = code;

        bytes[offset + 1..offset + 3].copy_from_slice(&time.to_le_bytes());
    }

    emit::parse_frame(&bytes.map(|b| b ^ emit::XOR_MASK)).unwrap()
}

pub(super) fn start(
    app: &AppWindow,
    state: &SharedState,
    role: String,
    dir: PathBuf,
) -> GuiResult<Timer> {
    if !matches!(role.as_str(), "isanta" | "asiakas") {
        return Err(invalid("LAN-kokeen rooli on virheellinen"));
    }

    std::fs::create_dir_all(&dir).map_err(|e| invalid(&io_message(&e)))?;

    if role == "isanta" {
        let mut s = state.borrow_mut();

        let path = s.workspace.as_ref().unwrap().path().to_owned();

        let mut store = ProjectStore::open(&path)?;

        store.update(|p| {
            let mut d = p.kilpailijat[0].clone();

            d.numero = 8;

            d.emit = 234567;

            p.kilpailijat.push(d);

            p.leimaluvut.clear();

            Ok(())
        })?;

        s.workspace = Some(Workspace::open(&path)?);

        update_header(&s, app)?;
    }

    state.borrow_mut().print_test = true;

    let timer = Timer::default();

    let weak = app.as_weak();

    let shared = state.clone();

    let recovery = std::env::var_os("AS2026_LAN_GUI_RECOVERY").is_some();

    let mut phase = 0;

    let started = std::time::Instant::now();

    timer.start(TimerMode::Repeated,Duration::from_millis(250),move||{
let Some(app)=weak.upgrade()else{
return;
}
;
let result=(||->GuiResult<()>{

        if phase==99{
return Ok(());
}

        if started.elapsed()>Duration::from_secs(45){
return Err(invalid("Kahden GUI:n LAN-koe aikakatkaistiin"));
}

        let pair_path=dir.join("paritus.json");

        match phase{

            0=>{
if role=="isanta"{
app.invoke_lan_host();
phase=1;
}
else if pair_path.exists(){
let pair:Pair=serde_json::from_slice(&std::fs::read(&pair_path).map_err(|e|invalid(&io_message(&e)))?).map_err(|_|invalid("Koekoodin luku epäonnistui"))?;
app.set_lan_address(pair.address.into());
app.set_lan_code(pair.code.into());
app.set_lan_open(true);
phase=10;
}
}
,
            1=>{
if app.get_lan_active(){
if role=="isanta"{
let s=shared.borrow();
let host=s.lan_host.as_ref().unwrap();
std::fs::write(&pair_path,serde_json::to_vec(&Pair{
address:format!("127.0.0.1:{}",host.address.port()),code:host.code.clone()}
).unwrap()).map_err(|e|invalid(&io_message(&e)))?;
}
std::fs::write(dir.join(format!("{role}-valmis")),b"valmis").map_err(|e|invalid(&io_message(&e)))?;
app.set_lan_open(true);
save_view(&app,&dir.join(format!("{role}-lahiverkko.png"))).map_err(|e|invalid(&e))?;
app.set_lan_open(false);
phase=2;
}
}
,
            10=>{
app.window().dispatch_event(slint::platform::WindowEvent::KeyPressed { text: slint::platform::Key::Shift.into() });
smoke::key(&app, slint::platform::Key::Tab);
app.window().dispatch_event(slint::platform::WindowEvent::KeyReleased { text: slint::platform::Key::Shift.into() });
smoke::key(&app, slint::platform::Key::End);
smoke::key(&app, " ");
assert!(app.get_lan_code().ends_with(' '), "Kohdistus siirtyi parituskoodiin");
smoke::key(&app, slint::platform::Key::Return);
assert!(app.get_lan_busy(), "Enter parituskoodikentässä aloittaa liittymisen: koodi {:?}, osoite {:?}, virhe {:?}", app.get_lan_code(), app.get_lan_address(), app.get_lan_error());
phase=1;
}
,
            2=>{
if dir.join("isanta-valmis").exists()&&dir.join("asiakas-valmis").exists(){
let card=if role=="isanta"{
123456}
else{
234567}
;
super::handle_event(&mut shared.borrow_mut(),&app,ReaderEvent::Frame{
frame:Box::new(frame(card)),read_clock:ClockTime::new(12,11,0)?}
);
phase=3;
}
}
,
            3=>{
let count=shared.borrow().workspace.as_ref().unwrap().project().unwrap().leimaluvut.len();
if count==2&&app.get_result_entry_open(){
let fields=app.get_result_entry_fields().iter().collect::<Vec<_>>();
for field in fields{
let value=if field.key>0{
"12:10:00"}
else if field.key == -1{
"0"}
else if role=="isanta"{
"25"}
else{
"35"}
;
app.invoke_result_entry_edited(field.key,value.into());
}
app.invoke_result_entry_save();
if app.get_result_entry_open(){
return Err(invalid(app.get_result_entry_error().as_str()));
}
std::fs::write(dir.join(format!("{role}-tulos")),b"hyvaksytty").map_err(|e|invalid(&io_message(&e)))?;
phase=4;
}
}
,
            4=>{
if dir.join("isanta-tulos").exists()&&dir.join("asiakas-tulos").exists(){
let s=shared.borrow();
let p=s.workspace.as_ref().unwrap().project().unwrap();
if p.kilpailijat.iter().filter(|d|d.tallennettu_tulos.as_ref().is_some_and(|r|r.hyvaksytty)).count()!=2{
return Ok(());
}
drop(s);
app.invoke_approve_results();
app.invoke_result_entry_close(true);
if shared.borrow().last_print.is_none(){
return Err(invalid("LAN-tulosteen muodostus epäonnistui"));
}
app.window().set_size(slint::PhysicalSize::new(1366,768));
phase=5;
}
}
,
            5=>{
save_view(&app,&dir.join(format!("{role}-tulokset.png"))).map_err(|e|invalid(&e))?;
let s=shared.borrow();
let p=s.workspace.as_ref().unwrap().project().unwrap();
std::fs::write(dir.join(format!("{role}-lopputulos.toml")),p.to_toml()?).map_err(|e|invalid(&io_message(&e)))?;
std::fs::write(dir.join(format!("{role}-onnistui")),b"Kaksi Emit-lukua, kasikellot, virhepisteet, hyvaksynta ja tulosteen muodostus GUI:ssa").map_err(|e|invalid(&io_message(&e)))?;
println!("LAN-GUI {role}: Emit, manuaaliset ajat, pisteet, hyväksyntä ja tulosteen muodostus läpäisivät");
phase=if recovery{
6}
else{
99}
;
}
,

            6=>{
if role=="isanta" {
if dir.join("asiakas-onnistui").exists(){
app.invoke_lan_leave();
if app.get_lan_active(){
return Err(invalid(app.get_lan_error().as_str()));
}
std::fs::write(dir.join("isanta-katkaistu"),b"katkaistu").map_err(|e|invalid(&io_message(&e)))?;
phase=7;
}
}
else if dir.join("isanta-katkaistu").exists()&&app.get_lan_status().contains("katkennut"){

                super::handle_event(&mut shared.borrow_mut(),&app,ReaderEvent::Frame{
frame:Box::new(frame(234567)),read_clock:ClockTime::new(12,12,0)?}
);
app.set_result_entry_open(false);
shared.borrow_mut().entry_number=None;

                if shared.borrow().workspace.as_ref().unwrap().lan_client().unwrap().pending()!=1{
return Err(invalid("Katkon Emit-lukema ei tallentunut jonoon"));
}

                std::fs::write(dir.join("asiakas-jonossa"),b"pysyva jono").map_err(|e|invalid(&io_message(&e)))?;
app.set_lan_status(shared.borrow().workspace.as_ref().unwrap().lan_client().unwrap().status().into());save_view(&app,&dir.join("asiakas-yhteyskatko.png")).map_err(|e|invalid(&e))?;
phase=7;

            }
}
,
            7=>{
if role=="isanta"{
if dir.join("asiakas-jonossa").exists(){
app.invoke_lan_host();
phase=8;
}
}
else{
let s=shared.borrow();
let ws=s.workspace.as_ref().unwrap();
if ws.lan_client().unwrap().pending()==0&&ws.project().unwrap().leimaluvut.len()==3{
std::fs::write(dir.join("asiakas-palautui"),b"kuittaus").map_err(|e|invalid(&io_message(&e)))?;
phase=9;
}
}
}
,
            8=>{
let s=shared.borrow();
if app.get_lan_active()&&s.workspace.as_ref().unwrap().project().unwrap().leimaluvut.len()==3{
std::fs::write(dir.join("isanta-palautui"),b"3 alkuperaista lukemaa").map_err(|e|invalid(&io_message(&e)))?;
phase=9;
}
}
,
            9
if dir.join("isanta-palautui").exists()&&dir.join("asiakas-palautui").exists()=> {
let s=shared.borrow();
std::fs::write(dir.join(format!("{role}-palautunut.toml")),s.workspace.as_ref().unwrap().project().unwrap().to_toml()?).map_err(|e|invalid(&io_message(&e)))?;
save_view(&app,&dir.join(format!("{role}-palautuminen.png"))).map_err(|e|invalid(&e))?;
println!("LAN-GUI {role}: yhteyskatko, pysyvä Emit-jono ja automaattinen palautuminen läpäisivät");
phase=99;
}
,_=>{
}


        }
Ok(())}
)();
if let Err(e)=result{
let _=std::fs::write(dir.join(format!("{role}-virhe.txt")),e.to_string());
eprintln!("LAN-GUI {role}: {e}");
phase=99;
}

    }
);

    Ok(timer)
}
