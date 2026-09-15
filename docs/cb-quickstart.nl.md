# CB-quickstart (sdroxide — 11 meter)

Een korte, taakgerichte handleiding om **sdroxide** op de **11 meter (27 MHz)**
aan de praat te krijgen: je SDR of CAT-radio instellen, de kanalen per land
gebruiken, en het digitale WSJT-CB-verkeer volgen. Voor de volledige uitleg per
knop zie [`USER_MANUAL.md`](USER_MANUAL.md); voor het waarom van deze fork zie
de [README](../README.md).

> Dit is de CB/SWL-fork van sdroxide. De 11 m en de omroepbanden zijn eraan
> toegevoegd; de amateurbanden en de rest zijn upstream en ongewijzigd.

*English: [cb-quickstart.en.md](cb-quickstart.en.md). PDF: [cb-quickstart.nl.pdf](cb-quickstart.nl.pdf).*

Vetgedrukte namen zoals **SETTINGS** en **Callsign** zijn knoppen en velden
zoals ze op het scherm staan. `Settings > Radio` is een menupad.

---

## Wat je nodig hebt

- **Een SDR of een radio die sdroxide ondersteunt.** Voor 11 m het meest
  gebruikt:
  - **RTL-SDR** (dongle) — native, geen SoapySDR nodig.
  - **RX-888 / RX-888 Mk2** — native; de firmware wordt automatisch naar de
    ontvanger geüpload.
  - **Airspy HF+** (Dual / Discovery / Ranger) — native, 0,5 kHz–31 MHz.
  - Ook mogelijk: HackRF, Airspy R2/Mini, SDRplay RSP, ELAD, PlutoSDR, of een
    **CAT-radio** (Icom/Yaesu/Xiegu) via serieel + geluidskaart, TCI, OpenHPSDR
    of SoapySDR.
- **Een 27 MHz-antenne** die bij je ontvanger past.
- **sdroxide geïnstalleerd** (zie hieronder).

## Installeren

- **Windows** — de installer (`.msi`) of de portable `.zip` (met
  `sdroxide.exe`): zie de [Releases-pagina](https://github.com/madmedicnl/sdroxide/releases/latest).
- **Linux** — de **AppImage** (één bestand, `chmod +x` en starten), de `.deb`,
  of de portable tarball.
- **macOS** — de `.dmg`.

Je kunt sdroxide ook als **server** draaien en in de browser openen:
`sdroxide --server` en dan `http://localhost:4950`. Handig als de antenne
ergens anders staat.

---

## Deel A — Eenmalig instellen

Alles hier wordt bewaard onder `~/.config/sdroxide/`, dus je doet dit één keer.

### 1. Start sdroxide

Het hoofdvenster heeft de bedieningsbalk bovenin en daaronder de panadapter en
de waterval.

### 2. Kies je radio

Open **SETTINGS** en ga naar het tabblad **Radio**. Kies je interface
(bijvoorbeeld **RTL-SDR**, **RX-888** of **Airspy HF+**), daarna de
**sample rate** en de **gain**. Veranderingen gelden direct na **Apply /
reconnect**.

- **Linux — USB-ontvangers:** installeer de meegeleverde udev-regels, anders
  ziet sdroxide het apparaat wel maar kan het niet openen:
  `sudo cp 60-sdroxide-*.rules /usr/lib/udev/rules.d/ && sudo udevadm control --reload`, daarna opnieuw aansluiten.
- **Windows — RX-888:** bind het apparaat eenmalig aan **WinUSB** met
  [Zadig](https://zadig.akeo.ie/) — voor **beide** USB-id's (`04B4:00F3` en
  `04B4:00F1`).

### 3. Vul je gegevens in

Ga naar het tabblad **General**:

- **Callsign** — voor CB/WSJT-CB vul je hier je CB-roepnaam in.
- **Locator / grid** — je Maidenhead-grid; de kaart en de decodering gebruiken
  die.
- **IARU region** — de regio die de bandindeling bepaalt.
- **CB plan** — kies het kanaalplan van jouw land: **World / freeband**,
  **CEPT/EU** (Nederland), **Duitsland 80 kanalen**, **UK 27/81**, **USA**,
  **Australië**. Dit bepaalt de kanalen en het kanaalnummer op de waterval.

---

## Deel B — Op de 11 meter luisteren

1. Kies de **11 m**-band op de bandbalk. De band loopt van **26,965 tot
   27,860 MHz**.
2. Met het **CEPT/EU**-plan is **kanaal 1 = 26,965 MHz** en **kanaal 40 =
   27,405 MHz** (10 kHz-spacing). Het kanaalnummer verschijnt op de
   panadapter.
3. Kies de **mode**: **AM** of **FM** voor spraak, **USB/LSB** voor SSB op de
   freeband.
4. **Alleen luisteren?** Zet **SWL mode** aan: alle zendfuncties (PTT, TUNE,
   CALL CQ, …) verdwijnen en je houdt een schone ontvanger over. Met
   **Simple UI** zie je alleen de knoppen die je nodig hebt, met **AM · FM ·
   USB · LSB** vooraan.

---

## Deel C — Digitaal op 27 MHz (WSJT-CB / FT8-familie)

Deze fork spreekt dezelfde gehashte WSJT-uitwisseling die
[WSJT-CB](https://github.com/vash909/WSJT-CB) op 27 MHz gebruikt.

1. Kies **FT8** (of FT4/FT2).
2. Kies het digitale kanaal uit het kanaalplan / de kanalenlijst van je
   CB-plan.
3. In de **decodelijst**: klik een regel om je audio op dat signaal te zetten
   en druk **REPLY** of **Call CQ**.
4. Elke gedecodeerde zender laat zijn **landvlag** zien — sdroxide volgt de
   roepnaamconventies en de landnummering van WSJT-CB.
5. In **SWL mode** kun je alles meelezen zonder te zenden.

---

## Deel D — Je opstelling bewaren (profielen)

Onder **Settings > Profiles** sla je een hele opstelling op onder een naam en
zet je hem later met één klik terug: frequenties en VFO's, mode en filters,
gain, drive en antennes, de digitale identiteit en de bandstacks. Handig om
tussen "thuis op de 11 m" en "luisteren op de kortegolf" te wisselen.

---

## Deel E — Extra's

- **CW met je toetsenbord:** houd de **spatiebalk** ingedrukt — je toetsenbord
  werkt als een straight key.
- **Decodelijst exporteren** naar **CSV** of een *received-report* **ADIF**,
  voor wie bijhoudt wat er te horen was.
- **Browserclient** kan **ADIF** en **CHIRP**-bestanden importeren.
- **Hoorbare meldingen** bij oproepen en nieuwe DXCC/grids.
- **Thema's:** tien extra kleurenthema's.

---

## Let op — zenden

- Buiten de amateurbanden staat de **zend-lockout** standaard aan. **11 m/CB is
  geen amateurband**, dus zenden is geblokkeerd tenzij je de lockout met
  `--oob-tx` voor die sessie opheft.
- Of je op de 11 m mag zenden, en met welk vermogen/mode, verschilt per land
  (in Nederland de vergunningvrije CEPT-kanalen met beperkt vermogen).
  **Controleer de actuele regelgeving — jij blijft verantwoordelijk.**
- Deze fork is in de eerste plaats gemaakt om te **ontvangen en decoderen**.

---

## Hulp en links

- [README](../README.md) — het volledige overzicht van de fork.
- [USER_MANUAL.md](USER_MANUAL.md) — de handleiding per functie.
- [WSJT-CB](https://github.com/vash909/WSJT-CB) — het digitale 11 m-project
  waarop dit voortbouwt.
- [Releases](https://github.com/madmedicnl/sdroxide/releases/latest) — de
  nieuwste versie voor elk platform.

Tot horens op de 11 meter! 73
