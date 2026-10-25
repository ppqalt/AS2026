# AS2026 0.7.2

Versio kokoaa 25.10.2026 pyydetyt kilpailupäivän korjaukset ja viennit.

- Tasapisteissä käytetään RT-määrän ja aika-asemien parivertailua. Täysin tasainen tai ristiriitainen vertailu jää jaetuksi.
- Puuttuva JAT tai maali aiheuttaa keskeytyksen syyllä **JAT puuttuu** tai **maali puuttuu**. Korjattu Emit-leima poistaa puuttumisesta johtuvan keskeytyksen uudelleenlaskennassa.
- Saman Emit-leimasimen ja 99/999-häiriökoodin vuorottelu tulkitaan yhdeksi leimaksi. Alkuperäinen korttiluku säilyy.
- Emit-editorissa on taulukko, rivikohtainen välitön poisto ja lisäys väliin. Korttinumeron voi korjata, myös kohdistamattomasta lukemasta, ja uuden käsin täytettävän rivin voi luoda ratapohjasta.
- Manuaalisen aikatarkastuksen kellonajan voi syöttää kuutena numerona **hhmmss**. Naapuriasemien aikavälin ylittävä syöte antaa selkeän virheen.
- Luokan lähtöväli annetaan yhdessä kentässä kokonaissekunteina. Vanhojen kilpailutiedostojen minuuttimäärät muunnetaan näkymään oikein.
- Viennissä voi valita luokat ja tavalliset tai asemakohtaiset tulokset. Otsikko on kilpailun nimi, alaotsikko **luokka: luokan nimi**.
- Suora PDF-vienti käyttää upotettua Unicode-fonttia. Asetuksiin tallennettava PNG-logo näkyy vasemmassa ylänurkassa 2 cm korkeana.
- KITIstä tuotu XLSX-pohja voidaan täyttää alkuperäiseen rakenteeseen. Luokkakohdistukset valitaan nimillä ja tilakoodeissa käytetään pohjan selitteen mukaisia oletuksia. Koodit voi tarvittaessa muuttaa. Alkuperäistä pohjaa ei korvata.
- Windowsin tulostinluettelo haetaan kattavasta tulostinrajapinnasta. Mukana ovat asennetut virtuaalitulostimet.
- Livetuloksissa tarkastamattomat kilpailijat näkyvät **AJAA**-tilassa hyväksyttyjen ja keskeyttäneiden välissä. Selaimen päivityksen ja sivukierron väli asetetaan julkaisuikkunassa.
- LAN-paritus käyttää nelinumeroista PIN-koodia, joka vaihtuu jakamisistunnon alussa. Epäonnistuneita yhteysyrityksiä rajoitetaan.
- Kilpailijatietojen vieritys rajataan uudelleen, kun Seurat ja lisenssit -osio suljetaan tai ikkunan koko muuttuu.

Käsin luodut lukemat ja korjaukset ovat jäljitettäviä. Tulosten muokkaus poistaa niiden hyväksynnät; julkaisu ei hyväksy tuloksia automaattisesti.

Testit ja tarkistuskuvat käyttävät synteettisiä kilpailijoita. Oikeita KITI-esimerkkitiedostoja tai kilpailijoiden henkilötietoja ei toimiteta lähdekoodissa tai julkaisuissa.

Lataa ja pura koko käyttöjärjestelmäsi paketti. Käynnistä **as2026-gui** tai **as2026-gui.exe** ja valitse asennustoiminto. Molempiin paketteihin kuuluvat allekirjoitettu manifesti, käynnistin ja lisenssit.

[Kilpailupäivän ohje](docs/KILPAILUPAIVA.md) · [Tulosvienti](docs/VIENTI_JA_HUOMAUTUKSET.md) · [Emit-editori](docs/EMIT_RIVIN_KORJAUS.md)
