# Palomuuri: Windows ja UFW

Avaa portit vain **isäntäkoneessa** ja luotetulle kilpailuverkolle. TCP **42165**
on kilpailuyhteys; UDP **42166** on automaattinen haku. Reitittimen porttiohjausta
internetiin ei tarvita. Jos haku ei toimi, kokeile isännän IP-osoitetta käsin.

## Windows

Varmista Windowsin **Asetukset → Verkko ja Internet → yhteyden ominaisuudet**, että
luotetun kilpailuverkon profiili on **Yksityinen**. Avaa **Windowsin suojaus →
Palomuuri ja verkon suojaus → Lisäasetukset → Saapuvan liikenteen säännöt → Uusi sääntö**.
Tee porttisäännöt TCP 42165:lle ja UDP 42166:lle, salli yhteys vain yksityisessä
profiilissa. Rajaa sääntöjen ominaisuuksien **Alue**-kohdasta etäosoitteeksi paikallinen
aliverkko. Älä poista palomuuria käytöstä.

Samat säännöt voi tehdä järjestelmänvalvojana avatussa PowerShellissä:

```powershell
New-NetFirewallRule -Name "AS2026-LAN-TCP" -DisplayName "AS2026 kilpailuyhteys" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 42165 -RemoteAddress LocalSubnet -Profile Private
New-NetFirewallRule -Name "AS2026-LAN-UDP" -DisplayName "AS2026 lahiverkkohaku" -Direction Inbound -Action Allow -Protocol UDP -LocalPort 42166 -RemoteAddress LocalSubnet -Profile Private
```

Säännöt poistetaan tarvittaessa:

```powershell
Remove-NetFirewallRule -Name "AS2026-LAN-TCP", "AS2026-LAN-UDP"
```

## Linux / UFW

Tarkista tila komennolla `sudo ufw status`. Seuraava esimerkki sallii oman
**192.168.1.0/24**-aliverkon; korvaa se kilpailuverkon todellisella aliverkolla.
Älä kopioi esimerkkiverkkoa sellaisenaan, jos omasi on toinen.

```sh
sudo ufw allow from 192.168.1.0/24 to any port 42165 proto tcp comment 'AS2026 kilpailuyhteys'
sudo ufw allow from 192.168.1.0/24 to any port 42166 proto udp comment 'AS2026 lahiverkkohaku'
sudo ufw status numbered
```

Ohje ei ota UFW:ta käyttöön eikä muuta muita sääntöjä. Säännöt voi poistaa käyttäen
samaa aliverkkoa:

```sh
sudo ufw delete allow from 192.168.1.0/24 to any port 42165 proto tcp
sudo ufw delete allow from 192.168.1.0/24 to any port 42166 proto udp
```

Jos yhteys ei silti onnistu, tarkista molempien koneiden verkko, isäntäkoneen osoite
ja mahdollinen WLAN-asiakaseristys. Vierasverkko estää usein koneiden välisen yhteyden.

Lähteet: [Microsoft: New-NetFirewallRule](https://learn.microsoft.com/en-us/powershell/module/netsecurity/new-netfirewallrule),
[Ubuntu: palomuuri ja UFW](https://ubuntu.com/server/docs/firewalls/).
