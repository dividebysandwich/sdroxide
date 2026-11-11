# sdroxide — de Nederlandse vertaling van de interface. De referentie is
# en-US/sdroxide.ftl; wat hier ontbreekt, wordt in het Engels getoond.

## The language's own name, as the language picker lists it.
language-name = Nederlands

## Values from sdroxide-types, as the screen names them (src/i18n/labels.rs).

layout-mode-auto = Automatisch
layout-mode-desktop = Desktop
layout-mode-tablet = Tablet
layout-mode-small = Klein scherm
layout-mode-phone = Telefoon

ui-theme-default = Standaard
ui-theme-light = Licht
ui-theme-high-contrast = Hoog contrast
ui-theme-green-phosphor = Groen fosfor
ui-theme-amber-phosphor = Amberkleurig fosfor
ui-theme-teal-orange = Blauwgroen / oranje
ui-theme-rainbow = Regenboog
ui-theme-nord = Nord
ui-theme-nord-dark = Nord donker
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized donker
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin mocha
ui-theme-catppuccin-latte = Catppuccin latte
ui-theme-modern-minimalist = Modern minimalistisch

chrome-style-angled = Hoekig
chrome-style-rectangular = Rechthoekig
chrome-style-rounded = Afgerond
chrome-style-gradient = Verloop
chrome-style-bevel = 3D-facet
chrome-style-terminal = Terminal

font-size-small = Klein
font-size-medium = Middel
font-size-large = Groot

bandplan-kind-ham = Ham
bandplan-kind-cw = CW
bandplan-kind-digi = Digitaal
bandplan-kind-phone = Spraak
bandplan-kind-beacon = Baken
bandplan-kind-broadcast = Omroep
bandplan-kind-am = AM / LW / MW
bandplan-kind-cb = CB

speech-verbosity-terse = Beknopt
speech-verbosity-normal = Normaal
speech-verbosity-full = Volledig

# The examples in brackets are what the (English) voice says.
speech-freq-style-dial = Dial (14 point 074)
speech-freq-style-digits = Elk cijfer
speech-freq-style-full = Dial met eenheden

speech-callsign-style-phonetic = Fonetisch (kilo one alpha)
speech-callsign-style-letters = Letters (kay one ay)
speech-callsign-style-as-is = Zoals geschreven

## Settings window, UI tab (src/app/settings/ui_tab.rs).

settings-ui-display = Weergave
settings-ui-moved-note = Panadapterdetail, de reactie van het spectrum en de scrollsnelheid van de waterval worden ingesteld met de knop SPEC op de weergavebalk — het menu DISP op een smal venster.
settings-ui-language = Taal
    .hint = De taal van de interface. De handleiding achter F1 en de spraakmeldingen zijn in het Engels, wat hier ook gekozen is.
settings-ui-language-system = Systeem ({ $language })
settings-ui-layout = Indeling
settings-ui-theme = Thema
settings-ui-button-style = Knopstijl
settings-ui-window-style = Vensterstijl
settings-ui-frame-rate = Schermvernieuwingssnelheid
settings-ui-palette = Kleurenpalet van de waterval
settings-ui-tuning-buttons = Afstemknoppen
settings-ui-tuning-buttons-show = Tonen op telefoon en tablet
    .hint = Een rij knoppen om omlaag, stap en omhoog te gaan, onder de bedieningsbalk op een aanraakscherm. Er is geen wiel om een cijfer mee te scrollen en geen draaiknop, dus zonder deze knoppen kun je een bekende stap alleen bereiken door de hele frequentie in te typen. Worden nooit op een desktop getoond.
settings-ui-tuning-step = Stap: { $step }
settings-ui-tuning-snap = Eerste druk springt naar de stap
    .hint = Een afstemming die tussen twee veelvouden van de stap ligt, springt bij de eerste druk naar het volgende veelvoud in de ingedrukte richting — 7 074 300 gaat bij een stap van 1 kHz omhoog naar 7 075 000 en omlaag naar 7 074 000. Daarna bewegen de knoppen gewoon met de stap. Uit: elke druk verplaatst precies één stap.
settings-ui-smoothing = Uitvloeiing van de waterval
settings-ui-smoothing-interpolate = Interpoleren
    .hint = Mengt elke schermpixel met de bins en rijen eromheen, zodat een signaal er doorlopend uitziet waar het beeld breder is dan de transformatie. Zet het uit voor een rechthoekige waterval — één blok per bin, één per rij — wat nodig is om de vorm van een signaal van het beeld af te lezen: een geïnterpoleerd signaal is niet te onderscheiden van een werkelijk breder signaal. Een grotere FFT (de FFT-chip) is de andere helft daarvan.
settings-ui-clock = Klok
settings-ui-clock-show = UTC + lokaal op de panadapter
    .hint = Een klok rechtsboven op de panadapter, in UTC en in je eigen tijd.
settings-ui-background = Achtergrond van het spectrum
settings-ui-background-gradient = Verloop
settings-ui-background-top = boven
settings-ui-background-bottom = onder
settings-ui-spot-colours = Kleuren van spotlabels
    .hint = De tint van elke spotbron: de vakjes langs de onderkant van de waterval, de badges in de lijst SPOTS en de stippen op de wereldkaart.
settings-ui-spot-colour = Kleur voor { $kind }-spots
settings-ui-spot-colours-reset = Zet elke spotkleur terug op de standaard
settings-ui-reset = Herstellen
settings-ui-bandplan-colours = Kleuren van het bandplan
    .hint = De tint waarin elke soort toewijzing wordt getekend op de strook van het bandplan langs de onderkant van de waterval. De blokken worden halfdoorzichtig over de waterval getekend, dus daar zijn ze donkerder dan in het voorbeeld hier.
settings-ui-bandplan-colour = Kleur voor { $kind }-toewijzingen
settings-ui-bandplan-colours-reset = Zet elke bandplankleur terug op de standaard
settings-ui-skimmer-font = Lettergrootte van de skimmer
settings-ui-waterfall-font = Lettergrootte van waterval / spectrum
settings-ui-interface-font = Lettergrootte van de interface
    .hint = Schaalt de hele interface — menu's, dialogen, vensters, de radiotabbladen, de bovenbalk en zijn knoppen. De twee maten hierboven zijn er relatief aan.
settings-ui-cities = Steden op kaarten
    .hint =
        Teken de steden van de wereld — een stip per plaats, met de naam ernaast waar ruimte is — op de vlakke kaarten: FT8/WSPR, APRS, ADS-B en AIS.

        Ze zijn grotendeels wat aangeeft *waar* een stip op een wereldkaart staat, en ze zijn ook het drukste op de kaart. Zet ze uit als de namen in de weg zitten bij de zenders die je leest. De 3D-bol blijft ongewijzigd — de steden daar zijn nachtlichten en geen markeringen, en er staat niets over een contact geschreven.
settings-ui-cities-show = steden en hun namen tonen
settings-ui-update-check = Controleren op updates bij het opstarten
    .hint = Vraagt bij elke start één keer aan sdroxide.com of er een nieuwere versie is verschenen, en meldt dat in een banner boven de waterval die je kunt wegklikken. Er wordt niets verzonden behalve het verzoek zelf.
settings-ui-band-conditions-download = Bandcondities-verwachting downloaden
    .hint = Haalt hoogstens eens per uur de HF-bandcondities-verwachting van HAMQSL.com op om het bandmenu te kleuren. Uit toont het bandmenu geen verwachting, tenzij de 3D-weergave open is. Er wordt niets verzonden behalve het verzoek zelf.
settings-ui-broadcast-schedule-download = Omroepschema downloaden
    .hint = Haalt het EiBi-kortegolfomroepschema (ongeveer 1 MB) op van eibispace.de wanneer een nieuw omroepseizoen begint. Uit blijft het in sdroxide ingebouwde (of het laatst gedownloade) schema in gebruik; Instellingen → Spots → Schema nu downloaden haalt het nog steeds op verzoek op.
settings-ui-3d-view = 3D-weergave
settings-ui-clouds = Weergave van wolken
settings-ui-clouds-layered = Gelaagd
settings-ui-clouds-volumetric = Volumetrisch
settings-ui-clouds-note = Hoe de laag WOLKEN in de 3D-weergave het weer tekent. Gelaagd stapelt doorsneden door de troposfeer en is de goedkope optie. Volumetrisch laat een straal erdoorheen lopen, zodat de zon de wolkentoppen op het dek eronder werpt en bliksem door de storm gloeit die hem maakt, in plaats van alleen de buitenkant op te lichten — tegen een paar keer de kosten per pixel.

## Settings window, UI tab: voice announcements.

speech-title = Spraakmeldingen
speech-enabled = Wijzigingen aan de radio uitspreken
    .hint = Leest voor wat er veranderd is, zodat de radio bediend kan worden zonder ernaar te kijken
speech-voice = Stem
speech-voice-shipped = Meegeleverde stem
speech-speed = Snelheid
    .hint = De stem rekt of perst haar eigen frasering, zodat de toonhoogte niet verandert. Boven ongeveer 2× wordt het niet meer korter.
speech-volume = Volume
speech-output = Uitvoer
speech-detail = Detail
speech-duck = Ontvanger dempen
speech-duck-while = Tijdens het spreken
speech-test = Test
speech-what = Wat aangekondigd wordt
speech-cat-frequency = Frequentie
speech-cat-mode-band = Modus en band
speech-cat-vfo-split = VFO en split
speech-cat-agc-gain = AGC en versterking
speech-cat-levels = Aansturing, afstemmen en microfoon
speech-cat-ptt = Zenden en ontvangen
speech-cat-rit-xit = RIT en XIT
speech-cat-memory-scan = Geheugens en scannen
speech-cat-band-edge = Een amateurband verlaten
speech-cat-notices = Waarschuwingen en berichten
speech-cat-filters = Filters, squelch en ruisonderdrukking
    .hint = Standaard uit: deze veranderen voortdurend bij het najagen van een signaal
speech-decoded = Gedecodeerde berichten
speech-ft8-to-me = FT8-oproepen aan mij
speech-ft8-cq = FT8 CQ's waarop ik kan antwoorden
    .hint = Een drukke avond op twintig meter levert er honderd per minuut op
speech-js8 = JS8-berichten aan mij
speech-js8-allcall = JS8 @ALLCALL ook
speech-fsq = FSQ-berichten aan mij
speech-include-snr = Het rapport erbij zeggen
    .hint = Alleen waar het bericht zelf geen rapport bevat — een decodering die al een getal meldt, krijgt er niet ook ons getal bij
speech-ft8-qso = Mijn eigen FT8-uitwisseling
    .hint = Wat de sequencer op het punt staat te sturen, en het eindigen van het contact
speech-reading-text = Gedecodeerde tekst voorlezen
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW alleen terwijl de decoder vergrendeld is
    .hint = Het uitvoer van een niet-vergrendelde decoder voorlezen is erger dan stilte
speech-text-note = Beide staan standaard uit. Een decoder levert tekst sneller dan spraak die voorleest, dus alles dat te ver achter het live geluid aan loopt, wordt weggegooid in plaats van in de wachtrij gezet.
speech-tuning-up = Afstemmen
speech-swr-while-tuning = De SWR voorlezen terwijl TUNE ingedrukt is
speech-swr-every = Elke
speech-swr-summary = Bij loslaten de beste aanpassing melden
speech-swr-alarm = Waarschuwen bij hoge SWR tijdens elke zending
speech-how-read = Hoe dingen worden voorgelezen
speech-frequencies = Frequenties
speech-callsigns = Roepletters
speech-quiet-on-tx = Stil blijven tijdens het zenden
    .hint = Spraak gaat naar je luidsprekers en dus in je microfoon. Waarschuwingen bij hoge SWR komen er wel doorheen.
speech-device-note = Meldingen spelen af op hun eigen geluidsapparaat, dus ze worden nooit opgenomen en nooit naar iemand verstuurd die op afstand luistert. Toetsen om de status uit te spreken, de laatste melding te herhalen en midden in een zin te stoppen staan op het tabblad Bediening.

## Settings window (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = Wachten op de netwerkconfiguratie van het station…
settings-device-reports = zoals het apparaat het meldt
settings-transverters = Transverters
settings-one-row-per-box-while = Eén rij per vak. Zolang de afstemming binnen de band van een rij ligt, is de radio afgestemd op dial + offset, en gelden die rij's zendregel en aansturingsgrens; buiten alle rijen werkt de radio op zijn eigen banden. De rijen worden van boven naar beneden geprobeerd.
settings-name = Naam
settings-band-low = Onderkant band
settings-band-high = Bovenkant band
settings-offset = Offset
settings-transmit = Zenden
settings-max-drive = Maximale aansturing
settings-off-takes-transverter-out-line = Uit haalt deze transverter uit de lijn zonder de rij te verliezen — de band gaat terug naar wat de radio zelf bereikt.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Hoe je het noemt. Wordt in het logboek getoond wanneer de dial het selecteert.
settings-bottom-band-transverter-works-dial = De onderkant van de band waarmee deze transverter werkt, op de dial. 144 voor 2 m.
settings-top-band-dial-148-2 = De bovenkant van de band, op de dial. 148 voor 2 m in Regio 2 en 3, 146 in Regio 1.
settings-radio-ends-up-dial-offset = De radio komt op dial + offset uit, dus een transverter die een band naar een I.F. omlaag brengt is negatief: 2 m naar een 28 MHz I.F. is -116, want 144 - 116 = 28. Sleep om het per hertz bij te stellen voor een oscillator die net niet klopt.
settings-off-while-converting-receive-only =
    Uit tijdens converteren: alleen ontvangen — de rij is een converter, geen transverter, en er wordt op deze band niets gezonden.

    Via dezelfde converter: het werkt in beide richtingen, en zenden neemt dezelfde offset.
settings-ceiling-transmit-drive-while-transverter =
    Een plafond op de zendaansturing terwijl deze transverter geselecteerd is, als percentage van vol. De I.F.-ingang van een transverter neemt milliwatts, en de aansturing die voor de eigen banden van de radio goed is, zal hem vernielen, dus dit is het belangrijkste veld van de rij. 100% is geen limiet.

    Je instelling Aansturing wordt hieronder gehouden in plaats van verschoven, zodat het getal dat je op HF gebruikt terugkomt zodra de dial de band verlaat.
settings-remove = VERWIJDEREN
settings-add-transverter = TRANSVERTER TOEVOEGEN
settings-takes-effect-apply-reconnect-like = Werkt bij Toepassen / opnieuw verbinden, net als de converteroffset hierboven.
settings-maximum-transmit-drive = Maximale zendaansturing
settings-hard-limit-drive-tune-controls = Een harde grens waar de regelaars Aansturing en TUNE niet overheen kunnen, toegepast na de bandkalibratie hieronder, zodat niets de aansturing er weer overheen kan trekken. Stel hem in op het punt waar de radio zijn nominale vermogen haalt, dan is het hele regelbereik van Aansturing bruikbaar — op een zender die zijn eigen samples moduleert (HPSDR, LimeSDR, PlutoSDR, HackRF) is volledige aansturing de zender helemaal open, wat bij een versterker met veel gain ver boven zijn nominale eindtrap ligt. Uit betekent dat de regelaars tot volledige aansturing komen.
settings-limit-drive = Aansturing beperken tot
settings-per-cent-full-drive-key = Procent van volledige aansturing. Zend de radio in een dummyload en draai dit omlaag tot de meter het vermogen aangeeft waarvoor de versterker is gespecificeerd.
settings-applies-immediately-every-band-tune = Werkt direct, op elke band en ook voor TUNE.
settings-transmit-drive-band = Zendaansturing per band
settings-every-amplifier-has-different-gain = Elke versterker heeft op elke band een andere gain, dus één instelling voor Aansturing geeft op elke band een ander vermogen. Meet het uitgangsvermogen op elke band en stel vervolgens de banden bij die te hoog uitkomen tot ze allemaal gelijk zijn: stel Aansturing in op de band die het meest nodig heeft (meestal de hoogste) en breng de andere naar dat niveau omlaag. Decibels uitgangsvermogen, toegepast op spraak, digitaal en TUNE gelijkelijk. Overal nul — de standaard — verandert niets.
settings-other = Overig
settings-clear-all = ALLES WISSEN
settings-back-no-calibration-drive-setting = Terug naar geen kalibratie: de instelling Aansturing bereikt elke band volledig.
settings-applies-immediately-band-you-would = Werkt direct, en op de band waarop je zou zenden — achter een transverter is dat de band op de dial, en de eigen aansturingsgrens van de transverter wint altijd van alles wat hier is ingesteld.
settings-settings = Instellingen
settings-enter-server-s-address-first = vul eerst het adres van de server in
settings-enter-receiver-s-address-first = vul eerst het adres van de ontvanger in, of kies er een uit Publieke SDR's
settings-no-radio-selected-press-discover = geen radio geselecteerd — druk op Zoeken of vul een adres in
settings-enter-radio-s-address-first = vul eerst het adres van de radio in
settings-general = Algemeen
settings-radio = Radio
settings-ui = UI
settings-alerts = Waarschuwingen
settings-controls = Bediening
settings-spots = Spots
settings-freedv = FreeDV
settings-uploads = Uploads
settings-winlink = Winlink
settings-servers = Servers
settings-t-r-switch = T/R-schakelaar
settings-amplifier = Versterker
settings-remote = Op afstand
settings-tle = TLE
settings-profiles = Profielen
settings-station = Station
settings-enter-digital-mode-ft8-sstv = Kies één keer een digitale modus (FT8 / SSTV / …) om de opgeslagen waarden te laden.
settings-callsign = Roepletters
settings-grid-square = Locatievak
settings-your-callsign-grid-shared-across = Je roepletters en locatievak, gedeeld over FT8/FT4/FT2, SSTV-beeldkoppen en het logboek. Ook te bewerken in het instelvenster FT8 / SSTV.
settings-iaru-region = IARU-regio
settings-which-radio-interface-server-uses = Welke radio-interface de server gebruikt, wordt ingesteld op de machine waarop hij draait.
settings-radio-interface = Radio-interface
settings-waiting-machine-radio-attached-interface = Wachten op de machine waar de radio op is aangesloten. De interface-instellingen staan hieronder en kunnen ondertussen hier gewijzigd worden.
settings-converter = Converter
settings-manual-type-offset-below = Handmatig — typ hieronder een offset
settings-what-box-reads-when-offset = Wat dit vak toont wanneer de offset geen van de vooraf ingestelde waarden is. Stel hem in door te typen in het veld Offset hieronder; hier valt niets te selecteren.
settings-frequency-converter-front-receiver-pick = Een frequentieconverter voor de ontvanger. Kies er een, of typ ernaast een offset.
settings-how-far-converter-moves-signal =
    Hoe ver een converter het signaal verplaatst op weg naar de ontvanger, in Hz — hetzelfde getal en teken dat elke converterdocumentatie en elk ander SDR-programma vermeldt. De radio komt op dial + offset uit: positief voor een upconverter (een Ham It Up is 125000000), negatief voor alles wat een band naar een I.F. omlaag brengt — een satellietLNB, of een 2 m-transverter op een 28 MHz I.F., dat is -116000000 (28 − 144). 0 = geen converter.

    Sleep om het per hertz bij te stellen, wat een converter wil wiens oscillator net niet klopt.

    Dit is het ontvangstpad. Wat in de zendlijn zit, staat in de rij hieronder.

    Werkt bij Toepassen.
settings-what-transmit-line-while-converter =
    Wat in de zendlijn zit terwijl een converter is ingesteld.

    Uit tijdens converteren: er wordt niets gezonden — de veilige standaard, en de juiste keuze voor een toebehoren dat alleen ontvangt.

    Via dezelfde converter: één vak converteert in beide richtingen, een transverter. Zenden neemt de offset hierboven en volgt die als hij bijgesteld wordt.

    Eigen offset: de zendlijn is anders dan de ontvangstlijn. Nul is het gangbare geval — een converter (een LNB, een Ham It Up) voor ontvangst met de zender op zijn eigen antenne, wat het QO-100-station is.

    De controle op amateurbanden geldt nog steeds, op de frequentie waarop je zendt.

    Werkt bij Toepassen.
settings-how-far-transmit-line-moves =
    Hoe ver de zendlijn het signaal verplaatst, in Hz, volgens dezelfde tekenregel als de ontvangstoffset: de radio zendt op dial + offset.

    0 = de zender is aangesloten op zijn eigen antenne en werkt op de frequentie die de dial aangeeft — de 2,4 GHz-uplink van een QO-100-station.

    Een zendconverter die een I.F. *omhoog* brengt, is een negatief getal: de radio werkt onder de dial.
settings-rx-range = Ontvangstbereik
settings-which-frequencies-radio-receives-mhz =
    Welke frequenties deze radio ontvangt, in MHz: 144-146, 430-440. Laat leeg om te gebruiken wat het apparaat over zichzelf meldt.

    Bandknoppen buiten het bereik zijn grijs en de dial gaat daar niet heen.

    Dit zijn dialfrequenties. Met een converter ingesteld geef je de band op waarop je afstemt — 144-148 voor een 2 m-transverter — niet het I.F. waarop de radio werkelijk staat.

    Werkt bij Toepassen.
settings-tx-range = Zendbereik
settings-which-frequencies-radio-transmits-mhz =
    Op welke frequenties deze radio zendt, in MHz: 144-146, 430-440. Laat leeg om te gebruiken wat het apparaat meldt — en als het niets meldt, wordt het stuurprogramma op zijn woord geloofd en is elke frequentie toegestaan.

    Dialfrequenties, zoals het ontvangstbereik hierboven.

    Dit is een grens die jij instelt, geen vergunning: zenden buiten de amateurbanden wordt altijd geweigerd, tenzij je dat in config.toml hebt uitgeschakeld. Ook geeft het een alleen-ontvangstapparaat geen zender.

    Werkt bij Toepassen.
settings-antenna = Antenne staat
settings-where-radio-listens-station-antenna =
    Waar deze radio vandaan luistert.

    Op het station: de antenne is van jou, dus je eigen locator zegt waar hij staat. De standaard, en de juiste keuze voor alles in de shack.

    Ergens anders: een online ontvanger, of je eigen opstelling op een heuvel. Wat die hoort, wordt gemeld aan PSK Reporter, WSPRnet en FreeDV Reporter met de locator hiernaast, nooit met die van jou — en ADS-B plaatst vliegtuigen er ook tegen.

    Een ontvanger kiezen onder "Publieke SDR's" vult dit voor je in.

    Werkt bij Toepassen.
settings-receiver-s-maidenhead-locator-jn88ec =
    De Maidenhead-locator van de ontvanger — JN88ec, DO30db. Vier of zes tekens.

    Laat dit leeg als je niet weet waar de ontvanger staat: dan wordt er helemaal niets gemeld, wat het enige eerlijke antwoord is. Melden vanuit je eigen vak zou andermans ontvangst op het verkeerde continent zetten.
settings-audio = Audio naar
settings-gain-per-band = Gain per band
settings-remember-receive-gain-per-band = Ontvangstgain per band onthouden
settings-each-band-keeps-receive-gain = Elke band houdt de ontvangstgain die je er het laatst op hebt gezet, en krijgt die terug wanneer de dial er weer op komt — een bandknop, een sleep over een bandrand of de eigen draaiknop van de radio. Algemene dekking is verdeeld in HF, VHF, UHF en hoger. Een band waarop je geen gain hebt ingesteld, houdt wat er al geldt. Uit: één gain overal.
settings-s-meter-calibration = S-meterkalibratie
settings-added-radio-s-s-meter =
    Wordt opgeteld bij de S-meteraflezing van deze radio. De meter van een SDR begint met dBFS aflezen — het niveau ten opzichte van het volle bereik van de converter — dus een ontvanger met veel gain kan S9 tonen terwijl er geen antenne aangesloten is. Stel dit in op het verschil tussen wat de meter toont en een niveau dat je kent: een signaalgenerator, of een rustige band waarvan je weet dat die rond S2–S3 ligt. Werkt direct.

    Bovenop cal_offset_db in config.toml, dat één getal is voor het hele station.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = De antenne staat ergens anders en er is geen locator ingesteld, dus wat deze radio hoort, wordt niet gemeld aan PSK Reporter, WSPRnet of FreeDV Reporter. Typ hierboven de locator van de ontvanger om vanaf daar te melden.
settings-ranges-are-mhz-low-high = Bereiken staan in MHz, laag-hoog, gescheiden door komma's: 144-146, 430-440 — dat is 144000000-146000000 Hz en 430000000-440000000 Hz. Het zijn dialfrequenties: een converteroffset verschuift ze niet. De converteroffset hierboven is het veld in hertz. Laat een bereik leeg om te gebruiken wat het apparaat over zichzelf meldt; een apparaat dat niets meldt, wordt op zijn woord geloofd.
settings-careful-rtl-sdr-blog-v4 = Pas op met een RTL-SDR: de Blog V4 zet zelf onder 28,8 MHz al op, dus een negatieve offset die de hardware daarheen brengt, verschuift twee keer.
settings-transmit-audio = Zendaudio
settings-use = in gebruik
settings-every-stage-off = elke trap uit
settings-open-tx-audio = TX-audio openen…
settings-microphone-processing-chain-gate-eq = De bewerkingsketen van de microfoon — gate, EQ, de-esser, compressor en begrenzer — met een testopname om hem op gehoor af te stellen, zonder te zenden. Alleen spraakmodi. Wordt ook geopend door te klikken op Mic in het TX-vak.
settings-pick-radio-interface-above-configuration = Kies hierboven een radio-interface (deze configuratie gebruikte de verwijderde automatische detectie).
settings-configuration-used-removed-auto-detect = Deze configuratie gebruikte de verwijderde automatische detectie. Kies hierboven een interface zodra de machine waarop de radio is aangesloten heeft gereageerd.
settings-radio-has-no-interface-yet = Deze radio heeft nog geen interface — kies hierboven een en druk op Toepassen / opnieuw verbinden.
settings-radio-has-no-interface-yet-2 = Deze radio heeft nog geen interface. Er kan hierboven een gekozen worden zodra de machine waarop de radio is aangesloten heeft gereageerd.
settings-apply-reconnect = Toepassen / opnieuw verbinden
settings-switch-interface-now-no-restart = Nu naar deze interface schakelen — geen herstart nodig
settings-switch-server-s-radio-interface = Schakel de radio van de server nu naar deze interface — geen herstart nodig. De oude blijft draaien als de nieuwe niet geopend kan worden.
settings-switches-live-radio-without-restarting = Schakelt de werkende radio om zonder te herstarten.
settings-everything-above-server-s-own = Alles hierboven is de eigen configuratie van de server, en wijzigingen worden daar opgeslagen. De meeste instellingen werken zodra je ze wijzigt; de instellingen die vastliggen wanneer het apparaat geopend wordt — de samplefrequentie, een adres, de interface zelf — hebben deze knop nodig.
settings-dx-cluster-telnet = DX-cluster (telnet)
settings-enabled = Ingeschakeld
settings-host = Host
settings-port = Poort
settings-login-call = Inlogroepletters
settings-commands = Opdrachten
settings-one-per-line-e-g = één per regel, bijv. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Leest de CW/RTTY-skimmers van de wereld en voedt de propagatiekaart ermee. Dit is wat de kaart banden laat tonen waar deze radio niet op luistert. Standaard aan: het zet niets op de lucht, heeft geen account nodig en gebruikt de roepletters van het tabblad Algemeen. RBN-spots verschijnen niet in de spotlijst — het zijn metingen, geen uitnodigingen.
settings-t-7000-cw-rtty-feed-7001 = 7000 is de CW/RTTY-feed, 7001 de FT8/FT4-feed
settings-one-per-line-e-g-2 = één per regel, bijv. set/filter cont=eu
settings-sent-after-login-place-narrow = Wordt na het inloggen verstuurd. Hier beperk je de feed — zonder filter is dit elke skimmer op aarde.
settings-rbn-paths-are-placed-country = RBN-paden worden geplaatst vanuit landcentra, niet vanuit locatoren — nauwkeurig voor een klein land, ver naast de waarheid voor een groot. Het zijn een aparte laag op de propagatiekaart en die kan daar uitgeschakeld worden.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = POTA-activatorspots
settings-sota-spots = SOTA-spots
settings-psk-reporter-current-band = PSK Reporter (huidige band)
settings-upload-my-ft8-ft4-ft2 = Mijn FT8/FT4/FT2/JS8-decodeerresultaten uploaden
settings-report-what-station-hears-pskreporter = Meld wat dit station hoort aan pskreporter.info, zodat het daar als ontvanger verschijnt. Gebruikt de roepletters en het locatievak van het tabblad Algemeen.
settings-antenna-2 = Antenne
settings-collector = Verzamelaar
settings-t-4739-live-collector-14739-test = 4739 is de live verzamelaar, 14739 de testverzamelaar
settings-max-age-s = Maximale leeftijd (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Mijn WSPR-decodeerresultaten uploaden
settings-send-every-wspr-reception-wsprnet = Stuur elke WSPR-ontvangst naar wsprnet.org. Standaard aan: het zet niets op de lucht, en melden wat je hoort is wat een WSPR-ontvanger deel van het netwerk maakt in plaats van een privé-hobby. Een tijdslot waarin niets gedecodeerd werd, wordt ook gemeld — zo ziet het netwerk het verschil tussen een dichte band en een ontvanger die uit staat.
settings-download-who-heard-me = Downloaden wie mij hoorde
settings-ask-wsprnet-org-which-stations = Vraag wsprnet.org welke stations dit station gedecodeerd hebben. WSPR kent geen bevestiging van welke aard ook, dus dit is de enige manier waarop een zendende baken iets over zijn bereik te weten komt.
settings-ask-every = Vragen elke
settings-looking-back = terugkijkend
settings-apply = {" "}TOEPASSEN{" "}
settings-persist-re-connect-feeds = De feeds opslaan en (opnieuw) verbinden
settings-broadcast-stations = Omroepstations
settings-winlink-account = Winlink-account
settings-password = Wachtwoord
settings-winlink-account-password-not-gateway = Het wachtwoord van het Winlink-account, niet het wachtwoord van de gateway. Het is hoofdlettergevoelig — vul het precies in zoals het is uitgegeven.
settings-locator = Locator
settings-how-connect = Hoe verbinden
settings-route = Route
settings-internet = Internet
settings-forward-cms-over-internet = Doorsturen met de CMS via internet
settings-radio-packet = Radio (pakket)
settings-call-rms-gateway-air-radio = Roep een RMS-gateway aan via de lucht. De radio moet op PACKET of PACKET-HF staan.
settings-gateway = Gateway
settings-via = Via
settings-digipeaters-order-separated-spaces-usually = Digipeaters, in volgorde, gescheiden door spaties. Meestal leeg — een gateway die je rechtstreeks hoort, is een gateway die je rechtstreeks moet oproepen.
settings-speed = Snelheid
settings-most-rms-gateways-answer-1200 = De meeste RMS-gateways antwoorden op 1200. Een 1200-gateway aanroepen op 9600 klinkt precies als een gateway die uit staat, dus stel dit in naar wat de eigenaar van de gateway publiceert in plaats van te raden. 9600 vereist ook de datapoort van de radio aan beide kanten. 300 is HF.
settings-frequency = Frequentie
settings-clear = WISSEN
settings-zero-leaves-dial-alone-which = Nul laat de dial met rust, wat je wilt als je op één kanaal blijft staan. Alles anders stemt de radio af zodra de sessie begint.
settings-my-gateways = Mijn gateways
settings-winlink-s-published-gateway-list = De gepubliceerde gatewaylijst van Winlink vereist een API-sleutel die sdroxide niet heeft, dus houd je eigen lijst bij. De twee of drie gateways die vanaf één locatie bereikbaar zijn, leer je door te proberen, en ze veranderen zelden.
settings-use-2 = GEBRUIKEN
settings-call-one-next-connect = Roep deze bij de volgende verbinding aan
settings-current-dial = huidige dial
settings-forget = VERGETEN
settings-forget-gateway = Deze gateway vergeten
settings-add-gateway = + GATEWAY TOEVOEGEN
settings-save-gateway-above-list = Bewaar de gateway hierboven in de lijst
settings-internet-gateway = Internetgateway
settings-cms-address = CMS-adres
settings-client-name = Clientnaam
settings-winlink-s-production-servers-only = De productieservers van Winlink accepteren alleen clientnamen die ze kennen, en antwoorden op een onbekende met "Unknown client types are not allowed on production servers". Totdat sdroxide is geregistreerd bij het Winlink Development Team, heeft verbinden een naam nodig die zij herkennen.
settings-automatic-connection = Automatische verbinding
settings-connect-timer = Verbinden met een timer
settings-every = Elke
settings-persist-winlink-account = Het Winlink-account opslaan
settings-callsign-lookup = Roepletters opzoeken
settings-provider = Aanbieder
settings-auto-fill-name-qth-grid = Naam/QTH/locatie automatisch invullen bij klikken op een spot en bij een QSO
settings-choose-provider-enter-login = Kies een aanbieder om de login in te vullen.
settings-qrz-user = QRZ-gebruiker
settings-qrz-pass = QRZ-wachtwoord
settings-hamqth-user = HamQTH-gebruiker
settings-hamqth-pass = HamQTH-wachtwoord
settings-one-hamqth-account-does-both = Eén HamQTH-account doet beide — deze login is die van het tabblad HamQTH-upload.
settings-upload = Upload
settings-auto-upload-each-new-qso = Elk nieuw QSO automatisch uploaden
settings-master-switch-which-services-qso = De hoofdschakelaar. Naar welke diensten een QSO gaat, wordt ingesteld op elk tabblad hieronder.
settings-turn-auto-upload-each-new = Zet eerst "Elk nieuw QSO automatisch uploaden" hierboven aan — een dienst die hier is aangevinkt, wordt pas verstuurd als dat aan staat.
settings-auto-upload-off-above-so = Automatisch uploaden staat hierboven uit, dus de dienstvinkjes zijn uitgeschakeld en er wordt nog niets automatisch verstuurd — de knop UP per QSO in het logboek werkt nog wel.
settings-qrz-log-key = QRZ-logboeksleutel
settings-qrz-logbook-api-key-your = De API-sleutel van het QRZ-logboek uit je QRZ-logboekinstellingen — niet de login voor het XML-opzoeken.
settings-eqsl-user = eQSL-gebruiker
settings-eqsl-pass = eQSL-wachtwoord
settings-eqsl-qth = eQSL-QTH
settings-qth-nickname-eqsl-profile-upload = De QTH-bijnaam van het eQSL-profiel waarnaar geüpload en vanwaaruit gedownload wordt — alleen nodig als het account er meer dan één heeft; laat het anders leeg.
settings-same-login-hamqth-callsign-lookup = Dezelfde login als het opzoeken van roepletters bij HamQTH — het invullen van één van beide vult er ook de andere mee in.
settings-club-log-email = Club Log-e-mail
settings-club-log-pass = Club Log-wachtwoord
settings-club-log-key = Club Log-sleutel
settings-wrl-api-key = WRL API-sleutel
settings-developer-api-key-world-radio = De ontwikkelaars-API-sleutel uit World Radio League → Integrations → Developer API. Hij wordt maar één keer getoond als je hem aanmaakt, dus kopieer hem dan. Contacten gaan naar je standaardlogboek — stel er één in bij WRL als je er meer dan één bewaart.
settings-confirmations-download = Bevestigingen (downloaden)
settings-lotw-user = LoTW-gebruiker
settings-lotw-pass = LoTW-wachtwoord
settings-lotw-upload-uses-tqsl-export = De upload naar LoTW gebruikt TQSL — exporteer ADIF uit het logboek en onderteken het. LoTW- en eQSL-bevestigingen worden hier gedownload om gewerkt en bevestigd van elkaar te onderscheiden.
settings-sync-confirmations = BEVESTIGINGEN SYNCHRONISEREN
settings-fetching-subscriptions = Abonnementen ophalen…
settings-spectrum-waterfall-everything-read-off =
    Het spectrum, de waterval en alles wat daarvan afgelezen wordt, staan op het tabblad van die radio, en daarom heeft hij er geen eigen. De interfaceinstellingen hierboven zijn nog steeds van deze radio en blijven gelden — druk na het wijzigen op Toepassen / opnieuw verbinden en beide radio's komen samen weer op.

    Om hem weer alleen te gebruiken, leeg je het vak Panadapter-ontvanger op de pagina van die radio.
settings-panadapter = Panadapter
settings-give-radio-wideband-panadapter-borrowing = Geef deze radio een breedband-panadapter door de ontvanger van een andere radio te lenen — een SDR op dezelfde antenne, of een die gevoed wordt uit het I.F.-uitgang van deze radio. Het spectrum, de waterval, de subontvanger, de digitale modi en de skimmer komen allemaal van die ontvanger; de dial, de modus, het filter en de zender blijven hier.
settings-receiver = Ontvanger
settings-none = Geen
settings-which-radio-receives-one-leaves =
    Welke radio voor deze ontvangt. Hij verdwijnt uit de tabbalk zolang hij uitgeleend is — zijn front-end hoort bij de engine van deze radio — en komt meteen terug zodra dit op Geen wordt gezet.

    Werkt bij Toepassen.
settings-connected = Verbonden met
settings-how-far-above-radio-s =
    Hoe ver de ontvanger boven de dial van deze radio zit, in hertz — 0 voor een ontvanger op dezelfde antenne. Voor een I.F.-aftakking de tussenfrequentie waarop de ontvanger staat: 8215000 voor de KXV3B van een K3, 9000000 voor een 9 MHz I.F.

    Dit is niet het veld Converter hierboven, dat de radio afstemt, en ook niet de I/Q-middenoffset op het tabblad CAT, die over de eigen geluidskaart van deze radio gaat. Wat hier getypt wordt, wordt nooit naar een van beide radio's gestuurd.

    Werkt bij Toepassen.
settings-audio-2 = Audio van
settings-follow-dial = De dial volgen
settings-keep-receiver-radio-s-dial = Houd de ontvanger op de dial van deze radio, en verplaats de dial wanneer je op het panadapter afstemt. Uit parkeert de ontvanger waar hij staat — een vaste wacht op één segment terwijl de radio elders is.
settings-invert-spectrum = Spectrum omkeren
settings-mirror-receiver-s-span-about = Spiegelt de bereik van de ontvanger om zijn midden, voor een aftakking waarvan de oscillator boven het signaal zit en de band de verkeerde kant op aanlevert. Laat het uit, tenzij de waterval vol staat met signalen die allemaal aan de verkeerde kant van de dial liggen.
settings-mute-transmit = Dempen bij zenden
settings-silence-receive-audio-while-radio = Ontvangstaudio stil zetten terwijl deze radio zendt. Standaard aan: met de ontvanger op dezelfde antenne, of op het I.F. van deze radio, hoort die tijdens een zending je eigen zender. Zendingen die op de radio zelf gestart worden — de PTT van de microfoon, een voetschakelaar, MOX — tellen ook mee, zodra de CAT-verbinding meldt dat de zender naar zenden is gegaan.
settings-blank-transmit = Leegmaken bij zenden
settings-stop-panadapter-waterfall-while-radio = Het panadapter en de waterval stilzetten terwijl deze radio zendt. Standaard aan: een zender die over het hele bereik getekend wordt, wist de band erachter uit. Zet het uit om de band — of je eigen signaal — door een zending heen te volgen, wat de moeite waard is met de ontvanger op een aparte antenne. Geldt voor zendingen die sdroxide start; een zending die op de radio zelf gestart wordt, wordt volledig ontvangen, dus het beeld blijft lopen wat hier ook staat.
settings-per-mode-offsets-radio-whose = Offsets per modus, voor een radio wiens I.F. met de modus meeverschuift. Een modus die leeg is, gebruikt de offset hierboven.
settings-computer = Op deze computer
settings-add-radio-computer = Een radio op deze computer toevoegen
settings-radio-s-settings-are-below = De instellingen van deze radio staan hieronder. Sleep om hem langs de balk te verplaatsen.
settings-switch-radio-dialog-follows-drag = Naar deze radio schakelen (het dialoogvenster volgt). Sleep om hem langs de balk te verplaatsen.
settings-tx = ● TX
settings-station-radio = Station:
settings-station-radio-tip = De stationsradio. Hij draait de netwerkdiensten van het station (spotfeeds, rotor, versterker, zend-/ontvangschakelaar) en krijgt de instellingen van de opdrachtregel, daarom blijft hij in de lijst. Om hem te laten rusten, schakel hem uit met VERBINDING; voor andere hardware kiest u hieronder een andere interface.
settings-mute-radio-s-audio = De audio van deze radio dempen
settings-link = VERBINDING
settings-close-radio-configuration-kept = Deze radio sluiten (de configuratie blijft bewaard)
settings-close-tab-close-radio-station = Dit tabblad sluiten, of de radio sluiten op zijn station
settings-close-tab = Dit tabblad sluiten
settings-transverters-full = { $max } is het maximum dat deze tabel bevat
settings-dialling = Kiezen { $url }…
settings-region-note = Waar het station staat. Stelt elk bandplan in: de bandgrenzen, de CW-/data-/spraaksubsegmenten op de strook van de waterval, waar de skimmers luisteren, de aangeboden roepfrequenties en wat als buiten de band geldt voor zenden. In Regio { $region } betekent dat voor 70 cm { $region ->
        [1] 430–440 MHz — dus 446 is buiten de band, en 40 m stopt bij 7.200
        [2] 420–450 MHz, en 40 m loopt tot 7.300
       *[3] 430–450 MHz, en 80 m stopt bij 3.900
    }. Werkt meteen, en geldt voor elke radio op dit station.
settings-rx-grid-reported = Ontvangsten via deze radio worden gemeld vanuit { $grid }, niet vanuit je eigen locator.
settings-rx-grid-invalid = { $grid } is geen Maidenhead-locator, dus wat deze radio hoort, wordt niet gemeld. Vier of zes tekens: twee letters, twee cijfers en eventueel nog twee letters.
settings-bypassed = omzeild
# $state is one of settings-bypassed, settings-use or settings-every-stage-off.
settings-tx-audio-chain = Keten { $state }.
settings-gateway-via = { " " }via { $via }
settings-auto-upload-to = Elk nieuw QSO automatisch uploaden naar { $service }
settings-tle-updated = Bijgewerkt: { $n } { $n ->
        [one] abonnement
       *[other] abonnementen
    }: { $total } satellieten.
settings-tle-updated-failed = Bijgewerkt: { $ok } van { $n }; { $failed } mislukt — zie de rijen hierboven.
settings-panadapter-of = Deze radio is het panadapter van { $radio }
settings-radio-n = radio { $n }
settings-on-station = Op { $station }
settings-add-radio-on-station = Een radio toevoegen op { $station } — het station bedient hem meteen
settings-panadapter-receiver-for = Panadapter-ontvanger voor { $radio }
settings-hang-up-stays-on = Ophangen. De radio blijft op { $station }.
settings-close-on-station = { $radio } op { $station } sluiten? De configuratie blijft op die machine staan; het tabblad hier sluit ermee, en dat van iedereen ook.
settings-close-it-on = Sluit hem af op { $station }
settings-locator-hint = locator

## Settings window, Alerts (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Hoorbare meldingen
settings-alerts-sound-alarm-when-decode-matters = Een alarm laten klinken wanneer een decodering ertoe doet
settings-alerts-plays-over-own-audio-output = Speelt af via zijn eigen audio-uitgang, dus het is hoorbaar ook als de band op een andere luidspreker staat dan dit scherm.
settings-alerts-volume = Volume
settings-alerts-output = Uitvoer
settings-alerts-test = Test
settings-alerts-what-sound = Wat er klinkt
settings-alerts-each-station-quiet-while-after = Elk station blijft even stil na een melding, zodat een drukke band niet bij elk tijdslot gaat rinkelen. Stem en Toon + stem worden voorgelezen door de gesproken stem — schakel die in via Instellingen → UI.

## Settings window, Amplifier (src/app/settings/amp.rs)

settings-amp-amplifier = Versterker
settings-amp-waiting-station-s-amplifier-configuration = Wachten op de versterkerconfiguratie van het station…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = Een HobbyPCB Hardrock-500 of Hardrock-50 op zijn USB-seriële poort. SDRoxide leest zijn meters, schakelt zijn band, modus en antenne, en bedient zijn tuner vanuit het venster AMP. De versterker wordt nog steeds gestuurd door de PTT-lijn van de radio, zoals voorheen.
settings-amp-enable = Inschakelen
settings-amp-model = Model
settings-amp-untested-panel-has-only-been = Niet getest: dit paneel is alleen getest met een Hardrock-500. De meterlimieten voor dit model zijn afgeleid van die van de Hardrock-500 en kunnen onjuist zijn. Let op het eigen display van de versterker.
settings-amp-port = Poort
settings-amp-choose = — kies —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Moet overeenkomen met het menu van de versterker. Het eigen bedieningspaneel van HobbyPCB vereist 115200.
settings-amp-follow-radio-s-band = De band van de radio volgen
settings-amp-sends-transmit-frequency-amplifier-every = Stuurt de zendfrequentie bij elke bandwissel naar de versterker, zodat zijn filters de radio volgen zonder een eigen CAT-kabel. Wordt nooit tijdens een zending verstuurd.
settings-amp-show-power-swr-tx-meter = Zijn vermogen en SWR tonen op de TX-meter
settings-amp-only-where-radio-does-not = Alleen waar de radio ze zelf niet meet, en alleen terwijl de versterker in de lijn zit.
settings-amp-off = Uit
settings-amp-connecting = Verbinden…
settings-amp-status = Status
settings-amp-apply = {" "}TOEPASSEN{" "}
settings-amp-save-re-open-amplifier-s = De poort van de versterker opslaan en (opnieuw) openen

## Settings window, Controls (src/app/settings/controls.rs)

settings-controls-keyboard = Toetsenbord
settings-controls-click-shortcut-rebind-then-press = Klik op een sneltoets om hem opnieuw toe te wijzen, en druk daarna de toetscombinatie in (Esc annuleert). Sneltoetsen worden genegeerd terwijl je in een tekstveld typt.
settings-controls-shortcut = Sneltoets
settings-controls-does = Doet
settings-controls-step-mode = Stap / modus
settings-controls-accel = Versnelling
settings-controls-on = Aan
settings-controls-press-key = druk op een toets…
settings-controls-remove-binding = Deze toewijzing verwijderen
settings-controls-add-shortcut = + Sneltoets toevoegen
settings-controls-restore-defaults = Standaardwaarden herstellen
settings-controls-bind-hold-talk-space = Spreken-bij-ingedrukt-houden aan Spatie toewijzen
settings-controls-hold-space-transmit-releasing-losing = Houd Spatie ingedrukt om te zenden; loslaten — of het vensterfocus verliezen — zet de zender uit
settings-controls-unkey-held-ptt-after = Een ingedrukte PTT uitschakelen na
settings-controls-backstop-against-stuck-key-controller = Vangnet tegen een vastzittende toets of een bediening die stopt met melden. 0 schakelt het uit.
settings-controls-panadapter-mouse = Muis op het panadapter
settings-controls-wheel = Wiel
settings-controls-wheel-shift = Wiel + Shift
settings-controls-tune-step = Afstemstap
settings-controls-zoom-rate = Zoomsnelheid
settings-controls-click-tune-rounding = Afronding bij afstemmen met klikken
settings-controls-invert-wheel-direction = Richting van het wiel omkeren
settings-controls-left-drag-tunes-well-pans = Links slepen stemt af én verschuift
settings-controls-off-makes-left-drag-pan = Uit maakt links slepen een verschuiving van het beeld, net als rechts slepen.
settings-controls-scroll-digit-frequency-readout-tune = Scroll een cijfer op de frequentieaflezing om het af te stemmen
settings-controls-both-wheel-actions-are-tune = Beide wielacties zijn Afstemmen — er is dan geen manier meer om te zoomen.
settings-controls-restore-mouse-defaults = Standaardwaarden van de muis herstellen
settings-controls-mouse-buttons = Muisknoppen
settings-controls-left-right-buttons-are-reserved = De linker- en rechterknop zijn gereserveerd voor afstemmen en verschuiven; de middelste en extra knoppen zijn vrij. Een zijknop die ingedrukt wordt voor PTT, gedraagt zich als een voetschakelaar.
settings-controls-add-mouse-button = + Muisknop toevoegen
settings-controls-f1-always-opens-manual-even = F1 opent altijd deze handleiding, ook tijdens het typen, dus die kan niet opnieuw toegewezen worden.
settings-controls-midi-controller = MIDI-controller
settings-controls-midi-controllers-need-native-app = MIDI-controllers vereisen de native app — de browserclient heeft geen MIDI-toegang.
settings-controls-any-class-compliant-midi-surface = Elk klasse-compatibel MIDI-oppervlak werkt: het jogwiel van een DJ-controller is een prima VFO-knop, de pads zorgen voor PTT en bandknoppen, de schuifregelaars voor gainregeling.
settings-controls-enable = Inschakelen
settings-controls-controller = Controller
settings-controls-feedback = Terugkoppeling naar
settings-controls-rescan-ports = Poorten opnieuw zoeken
settings-controls-not-connected = Niet verbonden.
settings-controls-move-control-see-here = Beweeg een regelaar om hem hier te zien.
settings-controls-control = Regelaar
settings-controls-reads = Leest als
settings-controls-led = LED
settings-controls-move = beweeg hem…
settings-controls-click-then-move-control-you = Klik en beweeg daarna de regelaar die je wilt toewijzen
settings-controls-speed-sensitivity-spin-faster-tune = Gevoeligheid voor snelheid: draai sneller om sneller af te stemmen
settings-controls-send-current-value-back-light = Stuur de huidige waarde terug, om een LED te laten branden of een schuifregelaar te verplaatsen
settings-controls-add-midi-control = + MIDI-regelaar toevoegen
settings-controls-clear-all = Alles wissen
settings-controls-endless-jog-encoders-send-relative = Eindloze (jog-)encoders sturen een relatieve stap in plaats van een positie, in een van drie coderingen die bij kleine bewegingen op elkaar lijken. LEER raadt uit een draai met de klok mee; als de knop dan de verkeerde kant op afstemt, vink “rev” aan.
settings-controls-none = — geen —

## Settings window, General (src/app/settings/general.rs)

settings-general-system-default = Systeemstandaard
settings-general-remote-access = Toegang op afstand
settings-general-signed-in-station = Aangemeld station
settings-general-sign-out = {" "}AFMELDEN{" "}
settings-general-sign-out-forgets-device = Vergeet de aanmelding van dit apparaat. Het station vraagt het bij de volgende verbinding opnieuw; deze verbinding blijft open tot je haar sluit.
settings-general-password-kept-no-cookie = Voor { $stations } wordt een wachtwoord in deze browser onthouden, omdat er geen aanmeldcookie werd gegeven. Oudere stations doen dat; het is niet hetzelfde.
settings-general-what-remote-client-browser-page = Wat een externe client — de browserpagina, of een andere sdroxide gestart met --connect — moet opgeven voordat dit station hem laat bedienen. Geldt in servermodus (--server); de volgende aanmelding neemt een wijziging over, zonder herstart.
settings-general-username = Gebruikersnaam
settings-general-password = Wachtwoord
settings-general-clients-must-give-password-leaving = Clients moeten het wachtwoord opgeven. Een lege gebruikersnaam is prima.
settings-general-clients-must-sign = Clients moeten zich aanmelden.
settings-general-empty-anyone-who-can-reach = ⚠ Leeg: iedereen die de poort van de server kan bereiken, kan deze radio bedienen, op jouw roepletters. Stel een wachtwoord in voordat je de poort doorstuurt.
settings-general-stored-clear-config-toml-like = Wordt in leesbare tekst opgeslagen in config.toml, net als de andere wachtwoorden die sdroxide bewaart.
settings-general-receive-audio-gain = Gain van ontvangstaudio
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Extra gain op alles wat deze radio naar de luidsprekers stuurt, bovenop de volumeregelaar. Laat dit op 0 dB staan, tenzij de radio op vol volume zacht is: de bovenkant van de volumeschaal is de audio zoals die binnenkomt, dus het kan een radio zachter zetten maar nooit harder, en de USB-geluidsuitgang van sommige transceivers zit ver onder het volle bereik.

    Ga per 6 dB omhoog. Te veel knipt — de audio wordt begrensd op het volle bereik in plaats van dat het overloopt, dus overdrijven klinkt schel in plaats van luid. Opnames worden vóór dit genomen en worden er niet door beïnvloed.
settings-general-reload-band-plan = BANDPLAN OPNIEUW LADEN
settings-general-re-read-bandplan-json-machine = Lees bandplan.json opnieuw op de machine waarop de radio is aangesloten en pas het toe — bandgrenzen, subsegmenten en skimmervensters — zonder te herstarten.
settings-general-running-station-s-own-band = Draait op het eigen bandplan van het station.
settings-general-running-built-iaru-tables = Draait op de ingebouwde IARU-tabellen.
settings-general-settings-file = Instellingenbestand
settings-general-settings-are-machine-radio-attached = De instellingen staan op de machine waarop de radio is aangesloten. Exporteer ze daar.
settings-general-export = EXPORTEREN…
settings-general-write-every-setting-station-radios = Schrijf elke instelling van dit station — de radio's, de modi, de servers, de geheugens, het bandplan — naar één bestand dat je naar een andere installatie kunt meenemen. Je logboek en elk bewaard serverwachtwoord blijven hier.
settings-general-import = IMPORTEREN…
settings-general-replace-station-s-settings-ones = Vervang de instellingen van dit station door die in een bestand dat vanuit een andere installatie is geëxporteerd. Start sdroxide daarna opnieuw.
settings-general-import-overwrites-what-here-file = Een import overschrijft wat hier staat, bestand voor bestand, en werkt de volgende keer dat sdroxide start — de instellingen die al in het geheugen staan, zouden er anders meteen weer overheen geschreven worden. Wat het bestand niet vermeldt, blijft staan, dus een bundel van een station met één radio verwijdert hier geen tweede radio. Je logboek zit nooit in het bestand: exporteer het als ADIF vanuit het venster LOG als je dat ook wilt verplaatsen.
settings-general-sdroxide-settings = sdroxide-instellingen
settings-general-swr-guard = SWR-bewaking
settings-general-stop-transmitting-high-swr = Stoppen met zenden bij hoge SWR
settings-general-trip = Schakelen bij
settings-general-stops-transmission-when-radio-reports =
    Stopt de zending wanneer de radio een SWR meldt op of boven deze waarde, en houdt zenden vergrendeld tot je het bevestigt. Vangt een losgeraakte antenne, een defecte voedingslijn of een schakelaar die op de verkeerde poort staat.

    Afstemmen wordt anders behandeld, omdat het voeden van een mismatch het doel ervan is: een antennetuner krijgt de dubbele grens en ongeveer vijf seconden voordat de bewaking hem Ã¼berhaupt toepast. Een handmatige tuner die langer dan dat duurt, vraagt om de bewaking voor de sessie uit te schakelen.

    Vereist een radio die de SWR via CAT meldt. Negeert de eerste vijfde seconde van elke zending en wacht niet op hoog vermogen.
settings-general-per-mode-settings = Instellingen per modus
settings-general-reset-every-mode = ELKE MODUS RESETTEN
settings-general-forget-every-mode-s-remembered = Vergeet de onthouden AGC-, squelch-, ruisonderdrukkings-, notch- en stereo-schakelaars van elke modus, en zet de eigen standaardwaarden van elke modus terug.
settings-general-selecting-mode-lays-mode-s = Het kiezen van een modus zet de eigen startwaarden van die modus voor AGC, squelch, ruisonderdrukking, de notch en de stereoschakelaars op de ontvanger, en elke wijziging wordt alleen voor die modus onthouden. Het cirkelpijltje aan het eind van de rij filter/ruis van de ontvanger geeft weer wat er gewijzigd is en zet de huidige modus terug.
settings-general-your-audio-speakers-microphone = Je audio (luidsprekers / microfoon)
settings-general-output = Uitvoer
settings-general-channels = Kanalen
settings-general-which-speaker-channel-s-received = Naar welk(e) luidsprekerkanaal/kanalen de ontvangen audio gaat. Links of rechts zet alles, gemengd tot mono, op die kant alleen.
settings-general-input = Invoer

## Settings window, network services (src/app/settings/net.rs)

settings-net-operator = Operator
settings-net-callsign-grid-are-set-general = Roepletters en locatievak worden ingesteld op het tabblad Algemeen.
settings-net-set-your-callsign-grid-general = ⚠ Stel je roepletters en locatievak in op het tabblad Algemeen.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Inschakelen
settings-net-connects-whenever-enabled-your-station = Verbindt zodra het is ingeschakeld. Je station wordt alleen aan anderen getoond zolang de radio in RADE staat — in elke andere modus blijf je verbonden maar verborgen.
settings-net-station = Station
settings-net-message = Bericht
settings-net-receive-only-i-cannot-transmit = Alleen ontvangen (ik kan niet zenden)
settings-net-server = Server
settings-net-host = Host
settings-net-port = Poort
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Nog niet geïmplementeerd — de FreeDV-GUI gebruikt ook gewoon ws://.
settings-net-reporting = Melden
settings-net-report-stations-i-decode = Stations die ik decodeer melden
settings-net-sends-rx-report-each-callsign = Stuurt een rx_report voor elke roepletter die uit een RADE End-of-Over-frame is gehaald.
settings-net-show-other-reporter-stations-spots = Andere meldende stations als spots tonen
settings-net-adds-them-panadapter-overlay-world = Voegt ze toe aan de panadapter-overlay, de wereldkaart en het venster SPOTS onder het filter FREEDV.
settings-net-set-your-callsign-grid-general-2 = ⚠ Stel je roepletters en locatievak in op het tabblad Algemeen. Zonder beide is de verbinding alleen-kijken: je ziet andere stations maar verschijnt zelf niet.
settings-net-apply = {" "}TOEPASSEN{" "}
settings-net-persist-re-connect = Opslaan en (opnieuw) verbinden
settings-net-longwave-shortwave-stations-labelled-waterfall = De lange- en kortegolfstations die op de waterval worden aangeduid. Het kortegolfschema wordt voor het huidige uitzendseizoen van eibispace.de gedownload en opnieuw gedownload wanneer het seizoen verandert.
settings-net-season = Seizoen
settings-net-downloading = downloaden…
settings-net-using-built-copy = de ingebouwde kopie wordt gebruikt
settings-net-your-own-stations-corrections-go = Je eigen stations en correcties gaan in dit bestand. sdroxide schrijft er nooit naar, en een vermelding hier vervangt een geplande vermelding met dezelfde naam en frequentie.
settings-net-reload = Opnieuw laden
settings-net-re-read-your-station-file = Lees je stationsbestand opnieuw in na het bewerken
settings-net-download-schedule-now = Schema nu downloaden
settings-net-fetch-season-s-schedule-again = Haal het schema van dit seizoen opnieuw op en vervang de gecachte kopie
settings-net-broadcast-stations-labelled-waterfall-come = De omroepstations die op de waterval worden aangeduid, komen uit de lijst die in deze build is ingebouwd. Bewerken ervan vereist de desktopapp.
settings-net-ask-service-whether-these-credentials = Vraag de dienst of deze inloggegevens werken. Past eerst de instellingen hierboven toe, en logt niets: het leest alleen.
settings-net-checking = controleren…

## Settings window: messages with values filled in.

settings-amp-connected-to = Verbonden met de { $model }
settings-controls-last-midi = Laatste bericht: { $message }  waarde { $value }
settings-general-band-plan-file = Elke bandgrens en elk subsegment komt uit { $path }, de eerste keer geschreven vanuit de ingebouwde IARU-tabellen en daarna van jou om te bewerken — versmal een band tot je vergunning en sdroxide weigert buiten die band te zenden. Frequenties staan in MHz; verwijder het bestand voor een verse kopie van de standaardwaarden. Dit is de regionale toewijzing, niet je vergunning: je eigen voorwaarden kunnen smaller zijn, en nationale plannen verschillen binnen een regio.
settings-general-exported = { $note } geëxporteerd — kies waar je het opslaat.
settings-general-export-failed = Exporteren mislukt: { $error }
settings-general-import-failed = Importeren mislukt: { $error }
settings-general-imported = { $summary } — start sdroxide opnieuw om ze te gebruiken.
settings-general-import-skipped = { $path } overgeslagen: { $why }
settings-general-swr-tripped = ⚠ Nu geschakeld bij { $swr }:1 — zenden is vergrendeld.
settings-net-call-grid-set-on-general = { $call } / { $grid } — ingesteld op het tabblad Algemeen
settings-net-reporting-as = Melden als { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Downloaden mislukt: { $error }. Het ingebouwde schema wordt gebruikt.
settings-net-test-service = Test { $service }

## Settings window (src/app/settings/mod.rs), continued

settings-transmit-off-while-converter-set = Zenden is uit terwijl een converter is ingesteld — zeg wat er in de zendlijn hierboven zit om het weer aan te zetten.
settings-transmit-goes-through-same-converter = Zenden gaat via dezelfde converter, dus de radio werkt in beide richtingen met dezelfde offset van de dial.
settings-transmit-not-converted-radio-transmits = Zenden is niet geconverteerd: de radio zendt op de frequentie die de dial aangeeft, terwijl ontvangst via de converter loopt.
settings-transmit-takes-own-offset-separate = Zenden heeft een eigen offset, los van de ontvangstoffset.
settings-no-enabled-subscriptions-update = Geen ingeschakelde abonnementen om bij te werken.
settings-add-radio-here-station-you = Een radio toevoegen — hier, of op een station waarmee je verbonden bent

## Settings window, General, continued

settings-general-every-band-edge-sub-segment = Elke bandgrens en elk subsegment komt uit bandplan.json op de machine waarop de radio is aangesloten. Dit is de regionale toewijzing, niet je vergunning: je eigen voorwaarden kunnen smaller zijn, en nationale plannen verschillen binnen een regio.

## Settings window, picture upload (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Ontvangen beelden → FTP
settings-picup-uploads-every-received-sstv-picture = Uploadt elk ontvangen SSTV-beeld (en, als het is aangevinkt, elke weerkaart) zodra het is opgeslagen, ingedeeld zoals QSSTV het doet, zodat webpagina's die voor QSSTV zijn geschreven ongewijzigd werken.
settings-picup-upload-received-pictures = Ontvangen beelden uploaden
settings-picup-server = Server
settings-picup-port = Poort
settings-picup-security = Beveiliging
settings-picup-user = Gebruiker
settings-picup-password = Wachtwoord
settings-picup-certificate = Certificaat
settings-picup-trusted-when-computer-s-operating = Vertrouwd wanneer het besturingssysteem van deze computer het vertrouwt, zoals een browser hier dat zou doen. Een privé-CA kan hieronder toegevoegd worden (PEM).
settings-picup-extra-ca = Extra CA
settings-picup-only-certificate-fingerprint-accepted-way = Alleen het certificaat met deze vingerafdruk wordt geaccepteerd — de manier om een zelfondertekende server te vertrouwen. TEST toont die van de server.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Elk certificaat wordt geaccepteerd. De verbinding is versleuteld, maar iedereen op het pad kan zich als de server voordoen en het wachtwoord lezen.
settings-picup-plain-ftp-sends-password-pictures = ⚠ Gewone FTP verstuurt het wachtwoord en de beelden onversleuteld.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = SSTV-map
settings-picup-weather-fax = Weerfax
settings-picup-wefax-dir = WEFAX-map
settings-picup-empty-login-directory = Leeg is de loginmap.
settings-picup-file-names = Bestandsnamen
settings-picup-format = Indeling
settings-picup-add-extension = extensie toevoegen
settings-picup-image1-png-rather-than-qsstv = image1.png in plaats van de kale image1 van QSSTV
settings-picup-caption-bar = Onderschriftbalk
settings-picup-time-frequency-mode-sender-s = Tijd, frequentie, modus en de FSK-ID van de zender bovenaan, zoals QSSTV ze tekent. Alleen de geüploade kopie wordt gemarkeerd.
settings-picup-fit-960-768 = Passend maken op 960 × 768
settings-picup-qsstv-s-heartbeat-tiny-picture = De hartslag van QSSTV: een klein beeld waarvan de hoogte de minuut is, naast elk beeld geüpload, en twee pixels breed gemaakt wanneer het programma sluit.
settings-picup-nat-workaround = NAT-oplossing
settings-picup-ignore-private-address-server-s = Negeer een privéadres in het antwoord van de server in passieve modus en gebruik het eigen adres van de server — voor een server achter NAT die zijn publieke adres niet kent.
settings-picup-apply = {" "}TOEPASSEN{" "}
settings-picup-test = TEST
settings-picup-connect-settings-above-applied-not = Verbind met de instellingen hierboven — toegepast of niet — meld je aan, en schrijf en verwijder een klein bestand in elke uploadmap.
settings-picup-checking = controleren…
settings-picup-trust-certificate = Dit certificaat vertrouwen
settings-picup-pin-switch-pinned-fingerprint-one = Vastzetten: schakel naar "Vastgezette vingerafdruk" met deze. Controleer hem eerst bij de server — dit is wat de server zei, geen bewijs van wie hij is. Druk daarna op TOEPASSEN.
settings-picup-last-upload = Laatste upload

## Settings window, Profiles (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Profielen: de opgeslagen instellingen van het station
settings-profiles-profile-captures-whole-working-setup = Een profiel legt de hele werkinstelling vast — de dials en VFO's, de modus en filters, de gains, aansturing en antennes, je roepletters en berichtsjablonen, en de registers per band — en past die in één klik weer toe. De hardware (backend, geluidsapparaten, converters) wordt bewust met rust gelaten: een profiel is de manier waarop je het station bedient, niet de radio waarop het draait.
settings-profiles-save-current-setup = De huidige instelling opslaan
settings-profiles-profile-name = profielnaam
settings-profiles-save = Opslaan
settings-profiles-applies-whatever-sdroxide-attached-dials = Wordt toegepast op wat sdroxide ook maar aangesloten heeft: dials en VFO's, modus en filters, niveaus, gains, aansturing, antennes, de digitale identiteit en de bandstapels.
settings-profiles-no-profiles-saved-yet-name = Nog geen profielen opgeslagen — geef er hierboven een naam en druk op Opslaan.
settings-profiles-saved-profiles = Opgeslagen profielen
settings-profiles-apply = Toepassen

## Settings window, T/R switch (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Zend-/ontvangstschakelaar
settings-relay-waiting-station-s-t-r = Wachten op de configuratie van de T/R-schakelaar van het station…
settings-relay-closes-contact-while-station-transmits = Sluit een contact terwijl dit station zendt — om de antenne-ingang van de SDR los te koppelen en te aarden, en om in volgorde met de SDR een versterker of een externe T/R-relais aan te sturen. Werkt met de goedkope USB-relaiskaarten (LCUS, KMtronic, Numato), met een seriële RTS/DTR-lijn naar elke interface die een PTT-contact wil, en met al het andere via een opdrachthaak.
settings-relay-hardware = Hardware
settings-relay-board = Kaart
settings-relay-lcus-four-byte-a0-command = LCUS is de vierbytekaart met A0-opdrachten die onder tien namen verkocht wordt; KMtronic en Numato kunnen ook gevraagd worden naar hun instelling
settings-relay-port = Poort
settings-relay-choose = — kies —
settings-relay-device = Apparaat
settings-relay-gpio-pin = GPIO-pin
settings-relay-pin-3-every-homebrew-plan = Pin 3 op elk zelfgebouwd plan en elke commerciële kaart — het is die aan het eind van de behuizing waar een draad aan vastgemaakt kan worden. Als de relais niet klikt, is de kaart een kloon die deze pinnen helemaal niet implementeert.
settings-relay-gpio-chip = GPIO-chip
settings-relay-linux-only-gpiodetect-lists-chips = Alleen Linux. `gpiodetect` toont de chips van de machine.
settings-relay-transmit = Bij zenden
settings-relay-receive = Bij ontvangen
settings-relay-nothing-switched = Er wordt niets geschakeld.
settings-relay-starting-program-costs-unpredictable-amount = Het starten van een programma kost een onvoorspelbare hoeveelheid tijd, en niets daarvan telt mee in de voorsprong — laat hier dus een ruimere marge dan je voor een relaiskaart zou nemen. De getallen van de contacttabel betekenen niets voor deze verbinding: je opdrachten worden één keer bij het indrukken en één keer bij het loslaten uitgevoerd.
settings-relay-contacts = Contacten
settings-relay-contact-1-rts-contact-2 = Contact 1 is RTS en contact 2 is DTR.
settings-relay-no = Nr.
settings-relay-gpio = GPIO
settings-relay-name = Naam
settings-relay-job = Taak
settings-relay-tx-closes = Zend sluit
settings-relay-lead = Voorsprong
settings-relay-hold = Vasthouden
settings-relay-line-s-offset-chip-bcm = De offset van de lijn op deze chip — het BCM-nummer op een Raspberry Pi-header, wat niet het fysieke pinnummer is
settings-relay-sdr-antenna = SDR-antenne
settings-relay-decides-default-timings-what-log = Bepaalt de standaardtiming en wat het logboek het noemt — en, voor een bandkoppelaar, dat het contact de bandtabel hieronder volgt
settings-relay-transmitting-energises-coil-off-transmitting =
    Aan: zenden bekrachtigt de spoel. Uit: zenden laat hem los.

    Dit is eerst een bedradingskeuze en pas daarna een instelling — zie de notitie hieronder.
settings-relay-how-long-before-rf-contact = Hoe lang vóór RF dit contact van het RX-woord van zijn band naar zijn TX-woord gaat. Een hertuning verplaatst het meteen; dit is voor het indrukken, wanneer een filter dat alleen voor zenden is in de schakeling moet staan voordat het vermogen er is.
settings-relay-closed-long-before-rf-small = Gesloten zo lang vóór RF. Een kleine coaxrelais schakelt in 5 tot 15 ms. Zenden wacht op de langste van deze, dus een grote waarde is een hoorbaar gat in de ontvangstaudio; alles boven 250 ms wordt genegeerd.
settings-relay-how-long-after-rf-stops = Hoe lang nadat RF stopt dit contact van zijn TX-woord terug naar zijn RX-woord gaat.
settings-relay-opened-long-after-rf-stops = Geopend zo lang nadat RF stopt. Bewust langer dan de voorsprong: de antenne een moment te laat weer terugschakelen kost niets, en te vroeg kost een front-end. Nul is een echt antwoord, en het juiste voor de sleutelleiding van een versterker.
settings-relay-test = {" "}TEST{" "}
settings-relay-not-while-station-air = Niet terwijl het station op de lucht is
settings-relay-close-contact-half-second-so = Sluit dit contact een halve seconde, zodat je de relais hoort en de bedrading controleert met de zender koud
settings-relay-remove = Verwijderen
settings-relay-add-contact = + een contact toevoegen
settings-relay-if-will-not-answer = Als hij niet reageert
settings-relay-switch-exists-protect-receiver-not = Een schakelaar die bestaat om een ontvanger te beschermen, is het niet waard als hij stilletjes ophoudt die te beschermen — dus weigeren is de standaard. Kies de andere waar een losse USB-kabel die je wedstrijd beëindigt de slechtere uitkomst is.
settings-relay-order-timings-longest-lead-closes = De volgorde is de timing: de langste voorsprong sluit eerst, en de langste vasthouding opent als laatste. Geef de antennerelais dus de langere voorsprong en de langere vasthouding, en de versterker de kortere van elk — dan wordt de antenne altijd geschakeld voordat de versterker gestuurd wordt, en gaat de versterker altijd uit voordat de antenne terugkomt.
settings-relay-transmit-sense-input = Ingang voor het zendsignaal
settings-relay-when-transceiver-keys-itself-own = Wanneer een transceiver zichzelf naar zenden schakelt — de eigen microfoonknop, voetschakelaar, VOX of keyer — ontdekt sdroxide dat door het via CAT te vragen, en dat antwoord komt enkele honderden milliseconden in de zending binnen. Sluit de SEND-lijn van de radio via een opto-isolator aan op een handshake-ingang op dezelfde poort, dan wordt het in plaats daarvan in enkele milliseconden gezien. De meter en de zendvergrendeling worden er sneller mee.
settings-relay-line = Lijn
settings-relay-transmitting = Zenden is
settings-relay-high-line = een hoge lijn
settings-relay-most-opto-isolated-interfaces-pull = De meeste opto-geïsoleerde interfaces trekken de lijn omlaag wanneer de radio zendt, dus dit staat meestal uit
settings-relay-belongs-radio = Hoort bij radio
settings-relay-which-radio-tab-sensed-transceiver = Op welk radiotabblad de waargenomen transceiver zit. Op een station met één radio is dit 1 en hangt er niets van af.
settings-relay-needs-serial-port-handshake-lines = Vereist een seriële poort — de handshake-lijnen zitten daarop.
settings-relay-wiring-only-fail-safe-survives = Alleen de bedrading is een veiligheid die dit programma overleeft.
settings-relay-choose-tx-closes-so-de = Kies “Zenden sluit” zodat het *niet-bekrachtigde* contact de toestand is die je wilt wanneer de kaart losgekoppeld is, de computer uit staat of sdroxide niet draait. Zo bedraad dat niet-bekrachtigd betekent: geaard, laat een dode relais de SDR doof — vervelend, maar veilig. Andersom bedraad, geeft een dode relais plus één zending een dode front-end. Met een echte versterker erachter kies je de eerste.
settings-relay-cannot-protect-first-instant-over = Dit kan het eerste moment van een zending die je bij de radio start niet beschermen. Zend de radio vanuit sdroxide en de contacten lopen altijd voor op de RF; bedraad je de zendingang hierboven, dan volgen ze binnen milliseconden. Voor een front-end die echt de moeite van beschermen waard is, gebruik je daarnaast een hardware-T/R-schakelaar die op de RF reageert — geen programma op een computer kan het hele antwoord zijn.
settings-relay-contacts-transmit = contacten op ZENDEN
settings-relay-not-open = Niet open.
settings-relay-status-unknown-press-apply = Status onbekend — druk op TOEPASSEN.
settings-relay-station-air-apply-will-take = Het station is op de lucht — TOEPASSEN wordt van kracht na deze zending.
settings-relay-band-decoder-outputs = Uitgangen van de banddecoder
settings-relay-one-control-word-per-band = Eén stuurwoord per band: bit 0 is contact 1, bit 1 is contact 2, enzovoort, in dezelfde nummering als hierboven. RX volgt de band van de ontvangstdial; TX de band van de radio die gezonden heeft, geschakeld op de voorsprong van elk contact vóór de RF en terug na zijn vasthouding — geef ze dezelfde waarde voor een filter dat niet uitmaakt welke kant de RF op gaat, en verschillende waarden voor een alleen-ontvangende voorversterkerbypass of een alleen-zendende LPF-bank. Banden op 0 zetten niets. Werkt bij Toepassen / opnieuw verbinden.
settings-relay-band = Band
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Uitgangen actief
settings-relay-other = Overig
settings-relay-everywhere-outside-amateur-bands-short = Overal buiten de amateurbanden — kortegolfluisteren, en alles waar de dial van een transverter op uitkomt dat geen band dekt.
settings-relay-apply = {" "}TOEPASSEN{" "}
settings-relay-persist-re-open-switch = De schakelaar opslaan en (opnieuw) openen

## Settings window, Remote (src/app/settings/remote.rs)

settings-remote-connect-server = Verbinden met een server
settings-remote-drive-sdroxide-running-somewhere-else = Bedien een sdroxide die ergens anders draait — de machine in de shack, een externe locatie — over het netwerk. Start hem daar met --server, en vul hier zijn adres in. De radio, de decoders en het logboek blijven op die machine; dit scherm krijgt de waterval en de audio, en de luidsprekers en microfoon die je hier gebruikt, zijn de jouwe.
settings-remote-address = Adres
settings-remote-host-name-ip = hostnaam of IP
settings-remote-machine-running-sdroxide-server-host = De machine waarop sdroxide --server draait: een hostnaam, een IPv4- of IPv6-adres, of een volledige ws:// (of wss://) URL als de server achter een reverse proxy staat — een URL wordt precies gebruikt zoals getypt en negeert de poort en de schakelaar hieronder.
settings-remote-port = Poort
settings-remote-port-server-listens-server-port = De poort waarop die server luistert — server_port in zijn config.toml. 4950, tenzij hij --port heeft gekregen.
settings-remote-secure = Beveiligd
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Vink dit aan wanneer de server achter iets staat dat HTTPS voor hem afhandelt — een reverse proxy op poort 443, de gebruikelijke manier waarop een station over het open internet bereikt wordt. De eigen server van sdroxide spreekt gewoon ws:// en wil dit niet.
settings-remote-connect = {" "}VERBINDEN{" "}
settings-remote-open-station-radio-tab-own = Dit station openen als een eigen radiotabblad
settings-remote-client-cannot-hold-second-connection = Deze client kan geen tweede verbinding aanhouden.
settings-remote-station-appears-tab-beside-radios =
    Het station verschijnt als tabblad naast de radio's die hier al open staan, en de tabbalk bovenaan het venster schakelt ertussen (⊞ zet er twee naast elkaar). Sluit het via de lijst bovenaan het tabblad Radio — dat hangt op; er wordt niets op de server veranderd.

    Vraagt de server om een gebruikersnaam en wachtwoord, dan verschijnt het aanmeldscherm in het nieuwe tabblad. Een gewone ws://-verbinding draagt dit alles niet versleuteld, dus over het open internet moet je het door een VPN of een SSH-tunnel halen, of HTTPS voor de server zetten en Beveiligd hierboven aanvinken.

## Settings window, Servers (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = WSJT-X UDP-uitzending
settings-servers-waiting-station-s-wsjt-x = Wachten op de configuratie van de WSJT-X-uitzending van het station…
settings-servers-sends-decodes-station-status-logged = Stuurt decodeerresultaten, de status van het station en gelogde QSO's op de manier van WSJT-X, zodat GridTracker, JTAlert, N1MM+ en Log4OM ongewijzigd met sdroxide werken. Alleen uitvoer — niets op deze socket kan de radio bedienen.
settings-servers-enable = Inschakelen
settings-servers-send = Versturen naar
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 bereikt clients op deze machine; een LAN-adres of een multicastgroep (224.0.0.1) bereikt andere
settings-servers-port = Poort
settings-servers-t-2237-port-every-client-defaults = 2237 is de poort waarop elke client standaard staat
settings-servers-identify = Zich voordoen als
settings-servers-name-clients-see-some-loggers = De naam die clients zien. Sommige logboekprogramma's accepteren alleen verkeer dat zichzelf als WSJT-X identificeert.
settings-servers-n1mm-contactinfo-broadcast = N1MM+ contactinfo-uitzending
settings-servers-same-news-n1mm-s-words = Hetzelfde nieuws in de woorden van N1MM: één XML-datagram per gelogd contact, voor de logboekprogramma's die naar N1MM luisteren in plaats van naar WSJT-X — de brug van de World Radio League neemt beide. Alleen contacten; er is geen decodeerstroom in dit protocol. Eigen schakelaar en eigen poort, omdat een station misschien beide wil.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 bereikt clients op deze machine. Het eigen advies van N1MM voor een wedstrijdnetwerk is het broadcastadres van dit subnet — 192.168.1.255 voor een netwerk 192.168.1.n — dat elke positie tegelijk bereikt.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 is de poort die de documentatie van N1MM aanraadt
settings-servers-station-name = Stationsnaam
settings-servers-what-n1mm-calls-stationname-name = Wat N1MM de StationName noemt: de naam van de computer die het pakket verstuurde. Logboekprogramma's tonen die om de ene operatorpositie van de andere te onderscheiden.
settings-servers-decoded-cw-over-udp = Gedecodeerde CW via UDP
settings-servers-cw-panel-s-copy-plain = De tekst van het CW-paneel als gewone tekst, één UDP-datagram per stuk zodra het zich vastgezet heeft — voor een stationscontroller, het CW-venster van een logboekprogramma of een script. Geen protocol: een luisteraar voegt gewoon toe wat binnenkomt (probeer nc -ul 9999). Alleen vastgelegde tekst wordt verstuurd, nooit het staartje dat de decoder nog aan het herzien is. Elke radio heeft zijn eigen schakelaar.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 bereikt programma's op deze machine; een LAN-adres of het broadcastadres van dit subnet bereikt andere.
settings-servers-t-9999-default = Standaard 9999
settings-servers-apply = {" "}TOEPASSEN{" "}
settings-servers-persist-re-open-broadcast-sockets = De broadcast-sockets opslaan en (opnieuw) openen
settings-servers-hamlib-rigctld-server = Hamlib rigctld-server
settings-servers-waiting-station-s-rigctld-configuration = Wachten op de rigctld-configuratie van het station…
settings-servers-listen = Luisteren op
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (deze machine)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (heel het netwerk)
settings-servers-t-4532-port-every-rigctld-client = 4532 is de poort waar elke rigctld-client van uitgaat
settings-servers-rig-name = Naam van de radio
settings-servers-reported-clients = gemeld aan clients
settings-servers-max-clients = Maximaal aantal clients
settings-servers-allow-clients-transmit = Clients laten zenden
settings-servers-off-refuses-every-key-request = Uit weigert elk verzoek om te zenden en stopt met adverteren dat er een zendbereik is, zodat Hamlib zelf weigert te zenden.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld heeft geen authenticatie. Op 0.0.0.0 kan iedereen die deze poort bereikt je zender afstemmen en naar zenden zetten.
settings-servers-t-1-client = 1 client
settings-servers-not-running = Draait niet.
settings-servers-status-unknown-press-apply = Status onbekend — druk op TOEPASSEN.
settings-servers-persist-re-bind-server = De server opslaan en (opnieuw) koppelen
settings-servers-lets-any-hamlib-capable-program = Laat elk Hamlib-programma deze radio bedienen: frequentie, modus, PTT, split, RIT en vermogen. Kies in WSJT-X, fldigi of CQRLOG de radio “Hamlib NET rigctl” (model 2) en wijs die naar dit adres; in GPredict en N1MM vul je de host en poort direct in. Anders dan de TCI-server draagt dit alleen besturing — geen audio, geen IQ.
settings-servers-antenna-rotator = Antennerotor
settings-servers-waiting-station-s-rotator-configuration = Wachten op de rotorconfiguratie van het station…
settings-servers-points-motorized-antenna-satellite-you = Richt een gemotoriseerde antenne op de satelliet waarop je vergrendeld bent, via een Hamlib rotctld-daemon (start bijvoorbeeld “rotctld -m 603 -r /dev/ttyUSB0” voor een GS-232B-rotor, of “rotctld -m 1” om het zonder hardware te proberen) — één daemon dekt elke rotor die Hamlib aanstuurt — of via PSTRotator, met UDP Control ingeschakeld in zijn Setup.
settings-servers-program = Programma
settings-servers-host = Host
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 is de standaard van rotctld. De UDP-besturingspoort van PSTRotator is standaard 12000, en die beantwoordt positievragen op de volgende poort, 12001 — die op deze machine vrij moet zijn.
settings-servers-min-elevation = Min. elevatie
settings-servers-below-rotator-parks-instead-tracking = Onder deze waarde gaat de rotor parkeren in plaats van volgen — stel hem in op je lokale horizon of dakrand
settings-servers-azimuth-offset = Azimutoffset
settings-servers-added-every-command-rotator-whose = Wordt bij elk commando opgeteld, voor een rotor waarvan het noorden niet klopt
settings-servers-min-movement = Min. beweging
settings-servers-steps-smaller-than-are-not = Stappen kleiner dan dit worden niet verstuurd — het najagen van tienden van een graad slijt motoren voor niets
settings-servers-park-position = Parkeerstand
settings-servers-drive-position-when-idle = in rust naar een stand draaien
settings-servers-off-leaves-antenna-wherever-last = Uit laat de antenne staan waar de laatste passage eindigde
settings-servers-not-connected = Niet verbonden.
settings-servers-persist-re-connect-daemon = De daemon opslaan en (opnieuw) verbinden
settings-servers-waiting-station-s-tci-server = Wachten op de TCI-serverconfiguratie van het station…
settings-servers-built-tci-server = Ingebouwde TCI-server
settings-servers-device-name = Apparaatnaam
settings-servers-off-leaves-control-receive-streams = Uit laat besturing en de ontvangststromen werken, maar elk verzoek om te zenden wordt geweigerd.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI heeft geen authenticatie. Op 0.0.0.0 kan iedereen die deze poort bereikt je zender afstemmen en naar zenden zetten.
settings-servers-tci-client = TCI-client
settings-servers-when-you-tune-send-radio = Wanneer je afstemt, stuurt dit de VFO van deze radio naar een andere TCI-server, zodat die radio meeloopt. Alleen besturing — geen audio, geen IQ, en de zender wordt er niet door naar zenden gezet.
settings-servers-server = Server
settings-servers-same-address-server-above-client = Dit is hetzelfde adres als de server hierboven. De client verbindt alleen wanneer die server niet degene is die luistert — laat hem uit, of zet hem op een andere poort, wanneer 590Ctrl deze poort gebruikt.
settings-servers-tci-client-connected = ● TCI-client verbonden
settings-servers-tci-client-connecting = TCI-client verbinden…
settings-servers-tci-client-press-apply = TCI-client — druk op TOEPASSEN.
settings-servers-persist-re-bind-server-connect = De server opslaan en (opnieuw) koppelen, en de TCI-client verbinden
settings-servers-server-lets-tci-capable-programs = De server laat TCI-programma's deze radio bedienen: frequentie- en modusbesturing, breedband-IQ, ontvangstaudio en zendaudio. Kies in WSJT-X het radiotype SunSDR (TCI), wijs het naar dit adres en zet PTT op TCI. De TCI-client is de andere richting: de dial van sdroxide wordt als VFO-commando naar 590Ctrl (of een andere TCI-server) op het adres hierboven gestuurd. Er gaat geen audio doorheen.

## Settings window, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satellieten: baansets en frequenties
settings-tle-waiting-station-s-satellite-configuration = Wachten op de satellietconfiguratie van het station…
settings-tle-tracker-already-fetches-celestrak-s = De tracker haalt de amateurgroep van CelesTrak al zelf op. Dit is voor al het andere: de NOAA-weersatellieten, een cubesat die te nieuw is om in de groep te staan, of een frissere baanset dan die binnenkwam. De lijsten worden opgehaald — en bewaard — waar de radio-engine draait, dus het is dezelfde set satellieten waar de app ook open staat.
settings-tle-subscriptions = Abonnementen
settings-tle-listings-fetched-kept-current-same = Lijsten worden opgehaald en actueel gehouden, op hetzelfde zesuurlijkse ritme als de amateurset. Ververst wanneer het zonnevenster open is, en met BIJWERKEN NU hier.
settings-tle-fetch-track-listing = Deze lijst ophalen en volgen
settings-tle-remove-subscription = Dit abonnement verwijderen
settings-tle-orbits = Banen
settings-tle-which-satellites-listing-get-orbit = Welke satellieten in deze lijst een baanring en een label krijgen. Een hele groep wil “gecureerd”: negentig ringen tegelijk is onleesbaar, en geen enkele laat negentig anonieme stippen over.
settings-tle-nothing-listing-sdroxide-s-curated = Niets in deze lijst staat in de gecureerde lijst van sdroxide — die lijst bevat tien amateursatellieten, dus dit zou exact als “geen” werken.
settings-tle-all-satellites = alle satellieten
settings-tle-catalogue-numbers-keep-comma-separated = Catalogusnummers om te behouden, gescheiden door komma's. Leeg volgt alles wat de lijst bevat.
settings-tle-not-fetched-yet = nog niet opgehaald
settings-tle-subscription = + Abonnement
settings-tle-update-now = {" "}BIJWERKEN NU{" "}
settings-tle-ask-radio-engine-fetch-every = Vraag de radio-engine om nu elk ingeschakeld abonnement op te halen
settings-tle-celestrak-groups = CelesTrak-groepen
settings-tle-pasted-element-sets = Geplakte baansets
settings-tle-one-off-these-do-not = Voor een eenmalig gebruik. Deze werken zichzelf niet bij, en SGP4 stopt met het propageren van een baanset zodra die twee weken voorbij zijn epoch is — abonneer je liever voor alles wat je wilt bewaren.
settings-tle-track-one = Deze volgen
settings-tle-show-two-element-lines = De twee baanregels tonen
settings-tle-remove = Verwijderen
settings-tle-paste-two-three-line-element = Plak hier baansets van twee of drie regels
settings-tle-add-pasted = + Geplakte toevoegen
settings-tle-nothing-paste-box-looked-like = Niets in het plakvak leek op een baanset.
settings-tle-clear-box = Vak leegmaken
settings-tle-frequencies = Frequenties
settings-tle-shown-under-pass-table-solar = Wordt getoond onder de passagetabel in het zonnevenster. Een vermelding hier vervangt de ingebouwde voor dat catalogusnummer volledig, dus begin met een kopie ervan, tenzij je de rest wilt laten vallen.
settings-tle-remove-satellite-s-entry = De vermelding van deze satelliet verwijderen
settings-tle-link = VERBINDING
settings-tle-downlink = DOWNLINK
settings-tle-uplink = UPLINK
settings-tle-mode = MODUS
settings-tle-note = OPMERKING
settings-tle-fm-repeater = FM-repeater
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67.0 Hz
settings-tle-link-2 = + Verbinding
settings-tle-copy-built = Ingebouwde kopiëren
settings-tle-frequency-either-one-number-145 = Een frequentie is ofwel één getal (145.800) of een transponderbandbreedte geschreven als 145.950-145.970. Laat een richting leeg voor een baken.
settings-tle-norad = NORAD
settings-tle-satellite = + Satelliet
settings-tle-frequency-entry-needs-satellite-s = Een frequentievermelding heeft het NORAD-catalogusnummer van de satelliet nodig.

## Settings window, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Adres
settings-wavelog-api-key = API-sleutel
settings-wavelog-your-wavelog-s-front-page = Het adres van de voorpagina van je Wavelog — https://log.example.org of http://192.168.1.10/wavelog — en een sleutel van de API-pagina. Uploaden vereist een sleutel met lees- en schrijfrechten.
settings-wavelog-plain-http-address-internet-api = Gewone http:// naar een adres op het internet: de API-sleutel gaat onversleuteld mee. Prima binnen je eigen netwerk; gebruik https:// over het internet.
settings-wavelog-trusted-cert = Vertrouwd certificaat
settings-wavelog-forget-certificate = Dit certificaat vergeten
settings-wavelog-go-back-ordinary-checking-against = Terug naar gewone controle tegen de publieke certificaatautoriteiten — voor een server die nu een goed certificaat heeft.
settings-wavelog-server-presented-certificate-sha-256 = De server toonde dit certificaat (SHA-256):
settings-wavelog-compare-your-server-s-e = Vergelijk het met dat van je server — bijv. `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — en vertrouw het alleen als ze overeenkomen. Vanaf dan wordt precies dit certificaat geaccepteerd en elk ander geweigerd.
settings-wavelog-trust-certificate = Dit certificaat vertrouwen
settings-wavelog-station-location = Stationslocatie
settings-wavelog-none = — geen —
settings-wavelog-press-test-above-list-station = Druk hierboven op Test om de stationslocaties uit je Wavelog op te sommen.
settings-wavelog-default = Standaard
settings-wavelog-where-qsos-go-when-operating = Waar QSO's naartoe gaan wanneer het bedieningsprofiel geen locatie noemt.
settings-wavelog-profile = Dit profiel
settings-wavelog-use-default = De standaard gebruiken
settings-wavelog-not-set = niet ingesteld
settings-wavelog-no-station-location-chosen-uploads = Geen stationslocatie gekozen — uploads mislukken tot er een is.
settings-wavelog-qsos-not-wavelog = QSO's die niet in Wavelog staan
settings-wavelog-every-qso-logbook-wavelog = Elke QSO in het logboek staat in Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Alles in het logboek dat Wavelog niet heeft bevestigd — ook contacten van voordat het automatisch uploaden aanstond. Die waarvan Wavelog al een kopie heeft, komen terug als dubbel en worden gewoon als verstuurd gemarkeerd.
settings-wavelog-set-address-key-station-location = Stel eerst het adres, de sleutel en een stationslocatie in.
settings-wavelog-yes-send-them = Ja, verstuur ze
settings-wavelog-cancel = Annuleren
settings-wavelog-download-into-logbook = Downloaden naar dit logboek
settings-wavelog-downloading-runs-where-radio-attached = Downloaden draait op de machine waar de radio is aangesloten, naar het logboek van die machine — stel het daar in.
settings-wavelog-download-qsos-start-every = QSO's downloaden bij het starten en elke
settings-wavelog-downloaded-qsos-join-logbook-so = Gedownloade QSO's komen in het logboek, zodat FT8 en de spotlijsten weten wat je met andere programma's en radio's al gewerkt hebt. Elke download haalt alleen wat nieuw is.
settings-wavelog-press-test-above-choose-which = Druk hierboven op Test om te kiezen welke stationslocaties je downloadt.
settings-wavelog-sync-now = Nu synchroniseren
settings-wavelog-fetch-what-wavelog-has-added = Haalt op wat Wavelog sinds de vorige download heeft toegevoegd. Past eerst de instellingen hierboven toe.
settings-wavelog-full-resync = Volledige resynchronisatie…
settings-wavelog-download-ticked-locations-whole-compare = Download de aangevinkte locaties volledig en vergelijk ze met het logboek: QSO's die in Wavelog zijn bewerkt of verwijderd, of van een locatie die niet meer is aangevinkt, worden gelijkgetrokken. Je ziet de wijzigingen voordat er iets gebeurt.
settings-wavelog-full-resync-changes-make = Volledige resynchronisatie — te maken wijzigingen
settings-wavelog-qsos-logged-here-imported-file = QSO's die hier gelogd of uit een bestand geïmporteerd zijn, worden nooit gewijzigd of verwijderd — alleen die uit Wavelog gedownload zijn.
settings-wavelog-apply = Toepassen
settings-wavelog-discard = Weggooien
settings-wavelog-live-radio-status = Live radiostatus
settings-wavelog-send-frequency-mode-wavelog-they = Stuur frequentie en modus naar Wavelog terwijl ze veranderen
settings-wavelog-radio-name = Naam van de radio
settings-wavelog-pick-radio-wavelog-s-live = Kies deze radio in de live logging van Wavelog en het formulier volgt de dial. Vereist een sleutel met lees- en schrijfrechten. Alleen vanaf de hoofdradio van het station.

## Settings window, picture upload: messages with values

settings-picup-server-fingerprint = Certificaat-SHA-256 van de server: { $fingerprint }

## Settings window, Profiles: messages with values

settings-profiles-apply-hint = Zet het station terug op “{ $name }”: dials, VFO's, modus, filters, gains, aansturing, antennes, identiteit en bandstapels.
settings-profiles-delete = “{ $name }” verwijderen

## Settings window, T/R switch: messages with values

settings-relay-not-usable = Nog niet bruikbaar: { $why }
settings-relay-receiving = ontvangen
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Settings window, Servers: messages with values

settings-servers-n-clients = { $n ->
        [one] { $n } client
       *[other] { $n } clients
    }
settings-servers-listening = ● Luistert op { $addr } — { $clients }
settings-servers-failed = Mislukt: { $error }
settings-servers-rotator-at = ● Verbonden — antenne op { $az }° az, { $el }° el
settings-servers-tci-client-error = TCI-client: { $error }

## Settings window, TLE: messages with values

settings-tle-subscription-age = { $count } satellieten · { $age } oud
settings-tle-subscribed = Geabonneerd op { $name }. Druk op BIJWERKEN NU om hem op te halen.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } oud — te oud om te propageren
settings-tle-norad-age = NORAD { $norad } · { $age } oud
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = { $n } baanset { $n ->
        [one] toegevoegd
       *[other] toegevoegd
    }.
settings-tle-added-refreshed = { $added } toegevoegd en { $r } baanset { $r ->
        [one] ververst
       *[other] ververst
    }.
settings-tle-links = { $n } { $n ->
        [one] verbinding
       *[other] verbindingen
    }
settings-tle-replace-links = Deze verbindingen vervangen door de ingebouwde voor { $name }

## Settings window, Wavelog: messages with values

settings-wavelog-call-mismatch = Locatie "{ $location }" is { $station_call }, maar je logt als { $call } — Wavelog weigert deze QSO's.
settings-wavelog-grid-mismatch = Locatie "{ $location }" staat op { $station_grid }, maar je locatievak is { $grid } — Wavelog weigert deze QSO's.
settings-wavelog-location-n = locatie #{ $id }
settings-wavelog-saved-with-callsign = Opgeslagen met de roepletters ({ $call }) in Instellingen → Profielen, zodat een draagbaar profiel naar een draagbare locatie kan loggen.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO
       *[other] QSO's
    } gelogd sinds automatisch uploaden aanstond en nog te versturen — elke 5 minuten opnieuw geprobeerd.
settings-wavelog-send-unsent = { $n } { $n ->
        [one] QSO
       *[other] QSO's
    } nog niet in Wavelog versturen…
settings-wavelog-send-confirm = { $n } QSO's naar { $location } versturen? Instellingen worden eerst toegepast.
settings-wavelog-plan-add = • { $n } QSO's in Wavelog om aan het logboek toe te voegen
settings-wavelog-plan-update = • { $n } gedownloade QSO's die in Wavelog gewijzigd zijn om bij te werken
settings-wavelog-plan-remove = • { $n } gedownloade QSO's die niet meer in Wavelog staan (of van een niet-aangevinkte locatie) om te verwijderen
settings-wavelog-plan-mark = • { $n } van je eigen QSO's om als in Wavelog gemarkeerd te zetten

## Settings window, Radio tab (src/app/settings/radio.rs)

settings-radio-antenna = Antenne
settings-radio-which-socket-back-radio-using =
    Welke aansluiting op de achterkant de radio gebruikt — zijn eigen ANT-commando, dezelfde instelling als de knop ANT op het voorpaneel.

    Werkt meteen, en het is de instelling van de radio zelf en geen kopie die hier bewaard wordt: de aansluiting waarop de radio staat, wordt bij het openen van de sessie teruggelezen.

    De keuze wordt per band onthouden. Schakel op 2 m naar de bundel en op 40 naar de draad, dan komt elk terug de volgende keer dat de dial in die band komt — en de volgende keer dat sdroxide start. Een geheugenkanaal dat hier is opgeslagen, neemt zijn aansluiting ook mee.
settings-radio-receive-antenna = Ontvangstantenne
settings-radio-radio-s-separate-receiving-antenna =
    De aparte ontvangstantenne van de radio — RX ANT op de achterkant — in of uit het ontvangstpad geschakeld. Eigen instelling, dezelfde als de knop RX ANT van de radio.

    De antenne op de hoofdaansluiting blijft in beide gevallen voor zenden: dit is een extra ingang voor ontvangst, een lus of een Beverage, geen keuze van aansluiting.

    Anders dan de antenne hierboven wordt dit *niet* hier onthouden. De radio houdt het zelf per band bij, dus sdroxide leest het na elke bandwissel terug en toont wat de radio zegt; alleen dit vakje aanvinken verplaatst het.
settings-radio-receive-path = In het ontvangstpad
settings-radio-radio-power = Radiovoeding
settings-radio-switch-radio-itself-off-back =
    Schakel de radio zelf uit en weer aan, via de besturingsverbinding. Dit is de enige echte aan/uit-schakelaar in het programma — niet de eigen schakelaar LINK van sdroxide, die alleen het eigen einde sluit en de radio aan laat staan.

    Om de inschakeling iets te laten bereiken, moet het besturingseinde van de radio wakker blijven terwijl hij uit staat. Over het netwerk doet Network Control dat. Over een seriële kabel is het de CI-V-poort van de radio, die onder spanning blijft op het lichtnet bij een toestel dat aan de voorkant is uitgezet in plaats van losgekoppeld; sdroxide stuurt de wakkerroep die de eigen documentatie van Icom vraagt vóór het inschakelen.

    Uitschakelen beëindigt de audio en de meters — er is geen radio meer achter — en sdroxide houdt de besturingsverbinding open zodat het inschakelen ergens naartoe kan.
settings-radio-on = Aan
settings-radio-off = Uit
settings-radio-waiting-configuration-machine-radio-attached = Wachten op de configuratie van de machine waarop de radio is aangesloten.
settings-radio-sound-format = Geluidsindeling
settings-radio-invert-spectrum = Spectrum omkeren
settings-radio-swap-i-q = I/Q wisselen
settings-radio-mirror-panadapter-about-tuned-frequency =
    Spiegel het panadapter om de afgestemde frequentie, voor een radio die I en Q op zijn geluidskaart andersom meestuurt. Het verraad is een waterval vol overtuigende signalen die allemaal aan de verkeerde kant van de dial liggen, met SSB op de tegenoverliggende zijband — het omwisselen van de twee kabels op de geluidskaart lost het net zo goed op.

    Alleen ontvangen: zenden geeft de radio één echt audiosignaal, dat geen zijband heeft om om te keren.
settings-radio-i-q-sample-rate = I/Q-samplefrequentie
settings-radio-how-fast-radio-s-i =
    Hoe snel de I/Q-geluidskaart van de radio draait — en daarmee hoeveel band het panadapter laat zien, want een quadratuurstroom beslaat zijn hele samplefrequentie: 48 kHz geeft ±24 kHz aan weerszijden van de dial, 192 kHz geeft ±96.

    De kaart heeft het laatste woord. Een kaart die de hier gekozen frequentie niet aankan, wordt op de dichtstbijzijnde geopend, en het panadapter is dan zoveel smaller — het logboek meldt dat bij het opstarten als het gebeurt.

    Een snellere kaart is ook meer werk: de machine heeft evenredig minder tijd om hem leeg te maken, en een kaart die dat niet bijhoudt laat samples vallen. Dat toont zich als audio die breekt terwijl de waterval er nog perfect uitziet, en ook dat wordt gelogd. Als je het ziet, zet het dan een stap lager.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = I/Q-middenoffset
settings-radio-how-far-radio-s-i =
    Hoe ver de I/Q-uitgang van de radio boven zijn eigen dial gecentreerd is, voor een toestel waarvan de ontvangst-I.F. van nul is verschoven — op een Elecraft, MENU:RX SHFT ingesteld op 8.0 in plaats van NOR, wat de dial van de DC-piek van de mixer afhaalt (en voorkomt dat een sterk nabij SSB/AM-station als AM gedetecteerd wordt).

    Geen converter: de radio toont en zendt nog steeds op de echte frequentie, en er wordt hier nooit iets naartoe gestuurd. Dit zegt alleen waar de samples op de geluidskaart zitten, en de stroom wordt bij binnenkomst terug naar de dial verschoven.

    Laat op 0 tenzij je zo'n menu-item hebt ingeschakeld. Als signalen op het dubbele van de offset landen, staat het teken andersom.
settings-radio-iq-correction = IQ-correctie
settings-radio-cancel-mirror-image-every-signal =
    Verwijdert het spiegelbeeld van elk signaal, en de DC-piek in het midden van de waterval.

    De I/Q-uitgang van een radio is twee analoge paden die nooit helemaal gelijk in gain zijn en nooit precies 90° uit elkaar liggen, en wat dat achterlaat is een kopie van elk signaal die om de afgestemde frequentie gespiegeld wordt, meestal 30-40 dB lager — sterk genoeg om op een station te lijken dat er niet is, en om als zodanig gedecodeerd te worden. Radio's met balansregelaars op het voorpaneel stellen precies dit bij; hier wordt het gemeten aan de ontvangen ruis en is er geen instelling nodig.

    Laat het aan. Zet het uit als je AM beluistert precies op de draaggolf — de draaggolf is DC, dus die gaat mee met de piek — of om te controleren of een signaal echt is door te kijken of het standhoudt.
settings-radio-cancel-mirror-images = Spiegelbeelden annuleren
settings-radio-dc-notch = DC-notch
settings-radio-widen-hole-taken-out-middle =
    Verbreed het gat dat uit het midden van het bereik wordt gehaald, voor een radio waarvan de middenpiek breder is dan de offset eronder.

    0 laat de gewone blokkering, die een paar tientallen hertz breed is en de offset verwijdert zonder iets anders aan te raken. Draai hem op, dan gaat de onderkant van het bereik mee: een hoogdoorlaatfilter van de eerste orde, 3 dB lager op de hier ingestelde waarde en verder naar beneden.

    Hij is gecentreerd waar de I/Q van de radio gecentreerd is, dat is de dial tenzij de offset hierboven die verschoven heeft — dus alles wat daar afgestemd is, gaat ook mee. Een CW-toon van 600 Hz valt binnen een instelling van 600 Hz.
settings-radio-panadapter-bw = Panadapter-bandbreedte
settings-radio-cat-family = CAT-familie
settings-radio-rigctld-address = rigctld-adres
settings-radio-host-port-running-hamlib-rigctld =
    host:poort van een draaiende Hamlib rigctld — 127.0.0.1:4532 is de eigen standaard, op deze machine.

    Start er een met bijvoorbeeld `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, waarin -m het Hamlib-modelnummer van je radio is (`rigctl -l` somt ze op).

    Dit is de vangnet-optie: hij bereikt frequentie, modus, PTT, vermogen, S-meter en SWR, en verder niets. Waar een van de eigen families hierboven op je radio past, doet hij meer — zenden vanuit de eigen tekstbuffer van de radio, het ontvangstfilter, en meterschalen per model.
settings-radio-flrig-address = flrig-adres
settings-radio-host-port-running-flrig-127 =
    host:poort van een draaiende flrig — 127.0.0.1:12345 is de eigen standaard, op deze machine. flrig biedt XML-RPC aan zolang het draait; de poort staat onder zijn Config → Setup → Server.

    Net als de Hamlib-optie stuurt dit een daemon aan en niet de radio, maar via de eigen driver per model van flrig — bij een aantal radio's is de afhandeling van vermogen en filters daar de getrouwere van de twee. Het bereikt frequentie, modus, PTT, zendvermogen (in hele watts), de ontvangstbandbreedte, de S-meter, SWR en uitgangsvermogen. CW gaat via de eigen cwio-poort van flrig, die in flrig zelf ingesteld moet worden.
settings-radio-serial-port = Seriële poort
settings-radio-select = — kies —
settings-radio-baud = Baud
settings-radio-data-bits = Databits
settings-radio-parity = Pariteit
settings-radio-stop-bits = Stopbits
settings-radio-force-rts = RTS forceren
settings-radio-force-dtr = DTR forceren
settings-radio-ptt-method = PTT-methode
settings-radio-how-transmit-keyed = Hoe zenden wordt aangestuurd.
settings-radio-how-transmit-keyed-network-link = Hoe zenden wordt aangestuurd. Een netwerkverbinding heeft geen besturingslijnen, dus DTR en RTS sturen hier niets aan — gebruik CAT, dat de daemon vraagt de radio aan te sturen, of VOX.
settings-radio-mode-control = Modusbesturing
settings-radio-digimode-mode = Digimodus-modus
settings-radio-what-put-radio-mode-sdroxide =
    In welke stand de radio gezet moet worden voor een modus die sdroxide via zijn geluidskaart moduleert. "DIGI" kiest de DATA/PKT-stand van de radio, die de zendaudio van de USB- of ACC-ingang neemt met de spraakbewerking van het microfoonpad eruit; "USB" laat hem op de gewone zijband staan, voor een radio die al SSB vanuit zijn dataingang moduleert of een zonder DATA-stand.

    Het geldt voor alle digitale modi — FT8, FT4, PSK, RTTY, SSTV en de rest — en voor CW verzonden als "Geluidskaart", en het gaat voor Modusbesturing voor die modi. SSTV is de enige waarvan de zijband met de band meeloopt, dus daar betekent het de DATA-stand op die zijband: USB-D boven 40 m, LSB-D op en onder 40 m. SSTV-FM valt er niet onder — een FM-draaggolf heeft geen zijband om uit te kiezen.
settings-radio-cw-keying = CW-sturing
settings-radio-how-cw-panel-s-keyer =
    Hoe de keyer van het CW-paneel zendt. "Radiokeyer" zet de radio op CW en geeft hem de tekst om met zijn eigen keyer te versturen. Het gebruikt de keyersnelheid van de radio (ingesteld vanaf de WPM van het paneel), en op Yaesu verzendt het via keyergeheugen 1, waarbij alles wat daarin stond overschreven wordt.

    Het vereist break-in aan de kant van de radio, dat sdroxide zelf bij elk bericht aanzet op Icom, Yaesu en Kenwood — een radio met break-in uit neemt het bericht aan en zendt niets. Het blijft daarna aan, en volledige break-in wordt nooit teruggezet naar semi.

    "Geluidskaart" verstuurt de gekeyde sidetone in plaats daarvan als audio (MCW), een toon op dial + toonhoogte — en omdat een radio op CW zijn geluidskaart helemaal zou negeren, volgt CW-selectie dan de instelling Digimodus-modus (USB, DATA of Radiogestuurd) in plaats van de radio naar CW te schakelen. Kies daar "Radiogestuurd" voor radio's waarvan de datastand niet via CAT te bevelen is (de U-D van een Xiegu) en zet de radio er zelf op, zoals bij FT8.
settings-radio-poll-rate = Pollsnelheid
settings-radio-how-often-radio-asked-what =
    Hoe vaak de radio gevraagd wordt wat hij doet — zijn dial, zijn modus en zijn meters. Dit is het deel van de besturingsverbinding dat van de radio terug naar sdroxide loopt: draai aan de VFO-knop van de radio of wissel de modus op het voorpaneel, en de uitlezing, de band en het panadapter volgen binnen één poll.

    Het is ook het hele besturingsverkeer van dit einde, en bij een moderne Icom is dat verkeer niet gratis: de CI-V-poort en de geluidskaart zitten achter één USB-hub in de radio, dus de frames die hier gevraagd worden, zijn bustijd die de audio niet krijgt — hoorbaar als uitvallen die op een DSP-fout lijken en dat niet zijn. De standaard van 2 Hz loopt een halve seconde achter de knop en is stil genoeg om de audio niet in de weg te zitten. Verhoog hem waar de besturingspoort een eigen apparaat is (een aparte USB-seriële adapter, een netwerk-rigctld); verlaag hem als de audio van de radio breekt.

    Een Icom met CI-V Transceive ingeschakeld in zijn eigen menu heeft hiervoor niets nodig voor de dial: hij meldt de knop op het moment dat die beweegt, en sdroxide zet de dialpoll stil zodra het dat ziet — en pakt hem weer op als de radio ooit beweegt zonder het te melden.

    De snelheid is die van de dial. De modus meeloopt alleen bij elke vierde poll, omdat het een instelling is die een paar keer per avond verandert en niet een die een knop volgt.
settings-radio-send-command = Opdracht verzenden
settings-radio-which-transceiver-generation-keys-rig =
    Welke transceivergeneratie de radio bedient, voor PTT-methode "CAT". De twee zijn het niet eens over wat de TX-parameter betekent, en niets op de draad onderscheidt ze, dus kies degene die bij je radio past.

    "TS-2000-stijl (TX;)" — TS-480, TS-570, TS-870, TS-2000: het gewone zenden, op de hoofdband.

    "TS-590-stijl (TX1;)" — TS-590S/SG, TS-890, TS-990: DATA SEND, dat zendt met de ACC2/USB-audio-ingang live. Op deze radio's kiest het gewone zenden de microfoon in plaats daarvan en dempt het de audio die sdroxide zendt.

    Verkeerd ingesteld zendt een TS-590 stilte — maar een TS-2000 zendt op de subband, wat een heel andere band is.
settings-radio-radio = Radio
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Eén profiel dekt de familie: de K3-opdrachtenset, die de KX3, KX2 en K4 allemaal beantwoorden. Hier valt niets te kiezen — hoeveel watt de schuif Aansturing bestrijkt (12 op een kale KX2 of KX3, 110 met een KPA3 of een KXPA100) wordt gelezen uit de eigen optiemodulevraag van de radio zodra de poort opent.

    Let op de baudrates hierboven: de K3, K3S, KX3 en KX2 gaan niet sneller dan 38400, en een radio die lager staat dan de hier gekozen snelheid, antwoordt helemaal niet.
settings-radio-transmit-input = Zendingang
settings-radio-where-radio-takes-transmit-audio =
    Waar de radio zijn zendaudio vandaan haalt — het TI-commando van de radio, dat menu 32 "TX IN" op het voorpaneel is. Wordt ingesteld zodra de poort opent.

    "USB-audio" is wat deze interface laat werken: de FDM-DUO zendt wat sdroxide in zijn USB-geluidskaart zet. Een radio die op "Microfoon" staat, zendt in plaats daarvan de kamer, en niets op het scherm zegt dat.

    "Auto" laat de radio kiezen — de microfoon bij een PTT-druk op de microfoon, de USB-poort bij een CAT- of RTS-indrukking.

    "Laten zoals ingesteld op de radio" stuurt helemaal geen TI, voor een operator die dit op het voorpaneel instelt.
settings-radio-which-two-sockets-back-receiver =
    Op welke van de twee aansluitingen op de achterkant de ontvanger luistert — het AN-commando van de radio, dat menu 31 "ANTENNAS" op het voorpaneel is en de aanduiding "ANT 1 2" op het display.

    "RTX" is één antenne die beide taken doet, op de M-aansluiting die ook zenden draagt. "Alleen RX" verplaatst ontvangst naar de tweede aansluiting en laat zenden op RTX — een ontvangstantenne, een lus of een Beverage, met de bundel nog op de zender.

    Werkt meteen, en wordt bij het openen van de poort uit de radio teruggelezen: dit is de eigen instelling van de radio en geen kopie die hier bewaard wordt.
settings-radio-one-profile-covers-both-there =
    Eén profiel dekt beide. Hier valt niets te kiezen — de radio geeft zichzelf een naam zodra de poort opent.

    Dit is alleen het CAT-deel: het stuurt de dial, de modus, PTT, de S-meter, de SWR en het zendvermogen via de CAT-USB-poort van de radio, en neemt audio van zijn USB Audio-poort. De derde USB-poort van de FDM-DUO draagt breedband-I/Q, wat deze interface niet leest — kies daarvoor de ELAD-interface hierboven.

    Let op de baudrate: menu 70 "CAT BAUD" op de radio moet overeenkomen, en hij staat standaard op 38400.

    CW wordt gestuurd met de eigen sleutel of paddle van de radio. De FDM-DUO heeft geen commando dat tekst aanneemt, dus het CW-paneel kan hem niet via CAT sturen.
settings-radio-one-profile-covers-range-qrp =
    Eén profiel dekt het bereik: de eigen opdrachtenset van QRP Labs, die een deelverzameling van de Kenwood TS-480 is met een flink aantal toevoegingen. Hier valt niets te kiezen — de radio geeft zijn naam en firmwareversie zodra de poort opent, en die versie bepaalt of de SWR-beveiligingsaflezing überhaupt gevraagd wordt.

    Niet het Kenwood-profiel: op een QMX is het PC-commando de vermogensMETER in plaats van de vermogensregeling, dus een radio die als Kenwood werd aangestuurd, zou zijn meteraflezing schrijven alsof het een instelling was, en MD8 — een Kenwood-modus — is hier SWR Tune.

    Het zendvermogen wordt op de radio zelf ingesteld. Er is geen CAT-commando voor, dus de schuif Aansturing bereikt alleen het niveau van de audio die in de geluidskaart gaat.

    Het ontvangstfilter is ook van de radio: hij meldt de breedte die zijn modus impliceert (3,2 kHz in Digi, 300 Hz in CW) en biedt niets aan om die te veranderen.
settings-radio-baud-rate-ignored-port-usb = Baudrate wordt genegeerd — de poort is USB
settings-radio-qmx-serves-own-virtual-com =
    Een QMX levert zijn eigen virtuele COM-poorten via USB, dus de hierboven ingestelde snelheid heeft aan beide kanten geen effect. Hij biedt er maximaal drie aan; als de radio niets antwoordt, is de meest waarschijnlijke reden de verkeerde.

    Eén ding telt wel voor de poort: een carriage return die erop gestuurd wordt, zet de radio voor de rest van de sessie in terminalmodus. sdroxide stuurt er nooit een — maar een terminalprogramma dat op dezelfde poort openstaat, wel, en CAT houdt op te werken zodra dat gebeurt.
settings-radio-i-q-mode-switched-radio = I/Q-modus wordt op de radio ingeschakeld
settings-radio-radio-s-sound-card-carries =
    De geluidskaart van de radio draagt ofwel gedemoduleerde audio ofwel de ruwe I/Q die zijn ADC ziet, en sdroxide zet wat er bij Geluidsindeling hierboven gevraagd wordt zodra de poort opent (het Q9-commando van de radio, het menu-item "IQ mode").

    De middenoffset en de samplefrequentie hierboven zijn ingesteld op wat een QMX nodig heeft: zijn ontvanger is een superhet met een 12 kHz I.F., dus de synthesizer zit 12 kHz onder de dial en alles op de kaart zit 12 kHz boven het midden van het bereik. De kaart draait op 48 kHz, wat het panadapter 48 kHz breed maakt.

    In CW verschuift de radio dat I.F. nog ongeveer 700 Hz, wat één getal niet kan volgen — tel dat zelf op als je het panadapter draait met de radio in CW. In Digi, waar een radio die als I/Q-front-end gebruikt wordt normaal staat, valt er niets op te tellen.

    QRP Labs merkt op dat de I/Q-modus niet geschikt is voor WSJT-X en andere programma's die gedemoduleerde audio verwachten; hier gebeurt de demodulatie aan deze kant, dus dat klopt wel.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (5 W HF-transceiver)
settings-radio-hobbypcb-s-rs-hfiq-there =
    De RS-HFIQ van HobbyPCB. Hier valt niets te kiezen: het profiel is de volledige opdrachtenset van de radio, en de firmwareversie wordt gelogd zodra de poort opent.

    De besturingsverbinding draagt twee dingen en niet meer — waar de oscillator op moet staan, en of er gezonden wordt. Er is geen commando voor modus, vermogen, filter, squelch of meter, omdat de radio die niet heeft. Al het andere is van sdroxide: wat van de geluidskaart komt, is complex basisband gecentreerd op de dial, dus de modus, het filter en de modulatie gebeuren allemaal aan deze kant.

    Het frequentiecommando dekt 3–30 MHz. Vraag je iets erbuiten, dan weigert de radio het en blijft hij staan; het logboek zegt dat.

    CW wordt als audio verstuurd via de zendketen — deze radio heeft geen keyer die tekst aanneemt. (Hij heeft een interne CW-generator; sdroxide gebruikt die nooit, op het eigen advies van de firmware.)
settings-radio-sound-format-baud-ptt-are = Geluidsindeling, baudrate en PTT zijn van de radio, geen instellingen
settings-radio-all-three-have-been-filled =
    Alle drie zijn ingevuld, en geen ervan is een voorkeur:

    • Geluidsindeling is I/Q (stereo) — de kaart draagt complex basisband en niets anders.
    • 57600 baud, 8N1 — de enige snelheid van de firmware, zonder menu om hem te wijzigen. Elke andere is een stille verbinding, dus hij wordt vastgezet zodra de poort opent.
    • PTT over CAT — het *X-commando is de enige zendschakelaar die de interface heeft.

    De middenoffset is op nul gezet: de synthesizer draait op vier keer de dial in een quadratuurdetector, dus het midden van het bereik is de dial. Zet de samplefrequentie hierboven op wat je geluidskaart werkelijk draait; dat bepaalt hoe breed het panadapter is.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    De pocket-QRP-transceiver van DL2MAN/PE1NNZ en de open uSDX-firmware waaruit hij is gegroeid. Hij emuleert een Kenwood TS-480, maar de deelverzameling is dun: de dial, de modus, PTT en het wissen van RIT/XIT zijn alles wat hij beantwoordt. Geen S-meter, geen SWR, geen vermogensregeling, geen VFO B, geen split en geen CAT-keyer, dus de schuif Aansturing en de meters doen niets via CAT — het audioniveau is de enige zendregeling die er is.

    CW wordt als audio (MCW) via de zijband verstuurd, of met de sleutel of paddle op de radio; er is geen commando dat tekst aanneemt.
settings-radio-audio = Audio
settings-radio-tr-usdx-has-no-sound =
    Een (tr)uSDX heeft geen eigen geluidskaart en twee manieren om gehoord te worden, en welke de juiste is, is een feit over je shack en niet over de radio.

    "Eén kabel" neemt de audio binnen de CAT-seriële verbinding zelf — de eigen 8-bitstroom van de radio, over dezelfde USB-kabel als de besturing, zonder iets anders om in te steken. De keerzijde is de firmware: hij kan geen CAT-commando aannemen terwijl zijn stroom loopt (een commando dat in de stroom gestuurd wordt, pauzeert hem niet maar breekt hem af), dus deze modus pollt nooit. Afstemmen en moduswijzigingen worden verstuurd terwijl je ze maakt, elk pauzeert de stroom even en start hem weer, dus reken op een korte onderbreking bij elke wijziging; er wordt niets verstuurd terwijl je gewoon luistert. De eigen dial en modus van de radio worden ook niet gevolgd — er is geen lezing om ze mee te volgen — dus bedien hem vanaf hier.

    "USB-geluidskaart" neemt de audio van een externe geluidskaart die op de luidspreker-/microfoonjack van 3,5 mm van de radio is aangesloten, en gedraagt zich als elke andere CAT-radio: de dialpoll loopt, dus de eigen knop en modus van de radio worden gevolgd. Kies de kaart hieronder bij Radio-audio.

    Beide modi openen de poort met DTR hoog: de DTR van de seriële adapter is aan de reset van de radio gekoppeld, dus het openen van de poort start hem opnieuw op, en de eerste seconde of twee is het stil terwijl hij opkomt. DTR wordt nooit gebruikt om te sturen.
settings-radio-radio-model = Radiomodel
settings-radio-which-icom-two-things-ci =
    Welke Icom, voor de twee dingen waarin CI-V niet op alle modellen hetzelfde werkt.

    Het transceiveradres hieronder wordt daaruit ingevuld: elk model heeft een ander adres, en een frame naar het verkeerde adres wordt gewoon genegeerd — een radio die niets antwoordt, zonder dat er ergens een fout staat om te zeggen waarom.

    Het bepaalt ook of sdroxide de DATA-modus kan kiezen. Op CI-V zijn USB en USB-DATA dezelfde modusbyte, en een apart commando onderscheidt ze; zonder dat gaat een digitale zending via de microfooningang de lucht in, met de spraakverwerking en het SSB-filter van de radio in het pad.

    "Overig" laat het adres aan jou over en stuurt helemaal geen DATA-modus-commando.
settings-radio-radio-id-hex = Radio-ID (hex)
settings-radio-show-radio-s-spectrum-scope = Het spectrumscherm van de radio tonen
settings-radio-scope-span = Bereik van het scherm
settings-radio-how-wide-sweep-radio-keeps = Hoe breed het wordt afgeveegd. De radio houdt het bereik vast dat het laatst op zijn eigen scherm gekozen is — vaak een paar kHz. Een bereik hier instellen zet het scherm ook in middenmodus, zodat het de dial volgt. Het verandert ook het eigen display van de radio; "Zoals ingesteld op de radio" laat het met rust.
settings-radio-press-apply-reconnect-switch-without = Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-radio-audio-sound-card = Radio-audio (geluidskaart)
settings-radio-waiting-sound-cards-machine-radio = Wachten op de geluidskaarten op de machine waarop de radio is aangesloten.
settings-radio-radio-rx = Van de radio (RX)
settings-radio-radio-tx = Naar de radio (TX)
settings-radio-apply-reconnect = Toepassen / opnieuw verbinden
settings-radio-reopen-cat-rig-these-sound = De CAT-radio opnieuw openen met deze geluidskaarten — zonder herstart
settings-radio-reconnects-radio-without-restarting = Verbindt de radio opnieuw zonder te herstarten.
settings-radio-receive-radio-pc = Ontvangen (radio → pc)
settings-radio-sound-card-radio-s-own = De geluidskaart waarop de eigen audio van de radio binnenkomt — zijn koptelefoonaansluiting naar de line- of microfooningang van de computer. Alles op het panadapter en in de decoders komt hier binnen.
settings-radio-transmit-pc-radio-mic = Zenden (pc → microfoon van de radio)
settings-radio-sound-card-carries-audio-radio = De geluidskaart die de audio draagt die de radio moet uitzenden — de uitgang van de computer naar de microfoonaansluiting van de radio. De radio schakelt zichzelf via VOX naar zenden zodra er audio hier binnenkomt, dus dit apparaat is stil tenzij er via de radio gezonden wordt.
settings-radio-radio-has-no-control-cable = Deze radio heeft geen besturingskabel — hij schakelt zichzelf via VOX naar zenden, dus er is geen PTT om aan te sturen. Zenden is audio naar de microfoon van de radio.
settings-radio-reopen-radio-these-sound-cards = De radio opnieuw openen met deze geluidskaarten — zonder herstart
settings-radio-devices = Apparaten
settings-radio-discover = Zoeken
settings-radio-no-devices-press-discover = geen apparaten — druk op Zoeken
settings-radio-manual-ip = Handmatig IP
settings-radio-optional-e-g-192-168 = optioneel, bijv. 192.168.1.50
settings-radio-sample-rate = Samplefrequentie
settings-radio-receiver-ddc = Ontvanger (DDC)
settings-radio-protocol-2-board-carries-several = Een Protocol 2-bord draagt meerdere onafhankelijk afstembare ontvangers (DDC's) over één verbinding — laat deze radio op DDC1 draaien en een andere radio, met hetzelfde adres, op DDC2. De zender hoort bij de radio op DDC1. Protocol 1-borden hebben alleen DDC1.
settings-radio-lna-gain = LNA-gain
settings-radio-front-end-gain-hermes-lite = Front-endgain van een Hermes-Lite 2. Werkt meteen — zonder opnieuw verbinden — en wordt onthouden als het niveau waarmee de radio start. Te hoog en de ADC klipt, en het hele band klinkt vervormd; te laag en de ontvanger wordt doof.
settings-radio-overload-protection = Overbelastingsbeveiliging
settings-radio-back-lna-gain-off-itself =
    Draai de LNA-gain zelfstandig terug zolang het bord meldt dat zijn ADC overstuurt, en laat hem weer oplopen zodra het ophoudt.

    Een Hermes-Lite 2 bemonstert heel 0–38 MHz op één 12-bits converter, zonder mixer en zonder preselector ervoor, dus een omroepstation een band verderop kan hem in overbelasting drijven terwijl het band dat je bekijkt niets verkeerds laat zien — de ruisvloer stijgt, alles intermoduleert en de decoders stoppen. Het bord weet het, en dit is wat erop reageert.

    Standaard uit: het verplaatst een regelaar die jij ingesteld hebt. Tijdens het zenden gebeurt er niets.
settings-radio-wind-gain-back-when-adc = Draai de gain terug wanneer de ADC overstuurt
settings-radio-applies-apply-reconnect-main-window = Werkt bij Toepassen / opnieuw verbinden. De schaal Gain van het hoofdvenster leest de gain die het bord werkelijk draait, dus die volgt de lus en je kunt zien wat hij doet; de schuif hierboven is het niveau waarmee de radio start, en de lus herschrijft die niet.
settings-radio-step = {"  "}Stap
settings-radio-how-far-gain-moves-each = Hoe ver de gain elke keer beweegt, in dB. Eén is de stap die het eigen gainregister van het bord heeft.
settings-radio-attack-decay = {"  "}Aanval / verval
settings-radio-how-often-gain-may-come =
    Hoe vaak de gain omlaag mag terwijl de converter overstuurt, en hoe vaak hij weer omhoog mag zodra dat ophoudt.

    De twee liggen bewust honderd keer uit elkaar. Trek je meteen terug: elke milliseconde overbelasting is een ontvanger vol intermodulatie. Herstel langzaam: wat het veroorzaakte — een buurman die zendt, een omroeper die bij zonsondergang opkomt — is meestal niet weg, en een lus die net zo snel herstelde als hij terugtrok, zou de avond heen en weer slingeren over de drempel. 100 ms en 10 s per decibel zijn wat zowel Auto S-Att van PowerSDR als HermesIntf van N1GP gebruikten.
settings-radio-range = {"  "}Bereik
settings-radio-lowest-highest-gain-loop-may = De laagste en hoogste gain die de lus mag gebruiken, in dB. Het plafond voorkomt dat hij bepaalt hoe gevoelig je ontvanger moet zijn; de ondergrens is het punt waarop je zegt dat de overbelasting onder een bepaald niveau andermans probleem is en het antwoord een filter is, geen nog twintig decibel.
settings-radio-filter-board = Filterbord
settings-radio-accessory-board-hermes-lite-2 = Toebehorenbord op de J16-header van de Hermes-Lite 2 (of de open-collectoruitgangen van elk ander openHPSDR-bord — ook Protocol 2). "N2ADR" kiest één relais per band; "Alex / Hermes-bandcode" zet het bandnummer op uitgangen 1–4, wat het Alex-bord van een ANAN, een Zeus SDR, een HiQSDR en Quisk verwachten. "Aangepast" opent een tabel hieronder waarin jij de zeven uitgangen zelf opgeeft, band voor band, op ontvangst en op zenden — voor een antenneschakelaar, de banddecoder van een versterker of iets anders waar geen van beide voorinstellingen bij past; beide voorinstellingen kunnen erin gekopieerd worden als startpunt. Laat dit op "Geen" staan tenzij er echt iets op zit: die zeven pinnen zijn algemene open-collectoruitgangen, en operators sluiten ze ook aan op PTT van een versterker, antenne-relais en transverterschakeling. Ze aansturen op basis van bandgegevens zou beginnen met bedienen wat er aangesloten is. Werkt bij Toepassen / opnieuw verbinden.
settings-radio-io-board-rx-input = Ontvangstingang van het IO-bord
settings-radio-where-n2adr-hl2ioboard-takes-receive = Waar een N2ADR HL2IOBoard zijn ontvangstsignaal vandaan haalt. Het bord zelf wordt automatisch gevonden en heeft geen instelling nodig; deze is alleen voor operators die de eigen SMA-aansluitingen ervan bedraad hebben. Laat hem anders op "Eigen ingang van de radio" staan — de J9 van het IO-bord kiezen terwijl er niets op aangesloten is, maakt de ontvanger doof. Werkt bij Toepassen / opnieuw verbinden. Ingesteld op een van de J9-keuzes, verschijnen de ingangen ook op de regelaar ANT en worden ze per band onthouden.
settings-radio-power-amplifier = Eindversterker
settings-radio-use-hermes-lite-2-s = De ingebouwde eindversterker van de Hermes-Lite 2 gebruiken
settings-radio-default-what-you-want-unless = Standaard aan, en wat je wilt tenzij een externe versterker vanaf de laagvermogen-RF1-uitgang van het bord wordt aangestuurd. Staat hij uit, dan stuurt de radio nog steeds — het T/R-relais schakelt en elk toebehorenbord volgt — maar de antennebus levert helemaal geen vermogen, en het relais wordt bewust op ontvangst vastgehouden. Wordt genegeerd op andere borden dan een Hermes-Lite.
settings-radio-mirror-board-s-spectrum-about = Spiegel het spectrum van het bord om de afgestemde frequentie, bij zenden zowel als bij ontvangen. Standaard aan: een Hermes-Lite 2 heeft het nodig. Zet het alleen uit als signalen aan de verkeerde kant van de dial verschijnen en er niets decodeert — het verraad is een waterval vol overtuigende sporen terwijl SSB op de verkeerde zijband landt en FT8 helemaal geen decodes oplevert.
settings-radio-frequency-correction = Frequentiecorrectie
settings-radio-crystal-tcxo-error-ppm-applied = Kristal-/TCXO-fout in ppm, toegepast op RX en TX. Werkt meteen.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Zendbuffer
settings-radio-how-far-ahead-real-time = Hoe ver vooruit op de werkelijke tijd de zendaudio over het netwerk naar het bord gestuurd wordt, voordat sdroxide vertraagt om gelijk te lopen. Het bord zelf heeft zo'n buffer niet — dit verbreedt alleen de eigen marge van sdroxide. Verhoog hem op een WiFi-verbinding of een VPN, waar de lage standaard (goed voor een directe bedrade verbinding) niet genoeg ruimte biedt tegen jitter en de uitgezonden audio of PTT gaat haperen; hoger kost zendlatentie. Werkt bij APPLY, dat opnieuw verbindt met het bord.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Adaptieve predistortie: lineariseert de zender aan de hand van een monster van wat hij daadwerkelijk uitzond, voor twintig-en-iets decibel minder intermodulatie bij hetzelfde vermogen. De ontvanger is het terugkoppelpad — het bord blijft tijdens een zending ontvangen — dus een richtkoppeler en een verzwakker moeten een monster van de uitgang van de versterker in een ingang leggen die de T/R-schakelaar bij zenden niet wegneemt. Op een Hermes-Lite 2 is dat de PureSignal-jack van het IO-bord, met de ontvangstingang hierboven daarop ingesteld; op een bord zonder die jack de RX-poort van een ANAN, of een extern relais dat de koppeler voor de duur van de zending inschakelt. Het is deze ontvanger die gelezen wordt — een bord met een tweede ADC kan RX2 niet als terugkoppelpunt gebruiken, hoe de eigen firmware het ook routeert. Zonder iets gekoppeld in, vergrendelt de lus nooit en blijft de zender precies zo als hij anders zou zijn geweest. Werkt bij Toepassen / opnieuw verbinden, en alleen op de radio die de zender bezit (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Corrigeer de zender met de eigen terugkoppeling van de ontvanger
settings-radio-table-steps = {"  "}Tabelstappen
settings-radio-how-finely-amplifier-s-curve = Hoe fijn de curve van de versterker gemodelleerd wordt. Meer stappen volgen een scherpere knik en duren langer om in te vullen; 32 is een verstandig begin.
settings-radio-adaptation = {"  "}Aanpassing
settings-radio-how-fast-correction-follows-what = Hoe snel de correctie volgt wat de koppeler meldt. Langzaam is goed — dit is het middelen van een curve van een versterker, die niet beweegt, uit een terugkoppelpad waar ruis in zit.
settings-radio-hold = Vasthouden
settings-radio-stop-adapting-keep-correction-where = Stop met aanpassen en houd de correctie waar die is — voor meten, en voor een operator die tevreden is met wat hij geleerd heeft.
settings-radio-manual-ip-overrides-discovery-press = Een handmatig IP heeft voorrang op zoeken. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-open-collector-outputs-band = Open-collectoruitgangen per band
settings-radio-one-control-word-per-band = Eén stuurwoord per band, zoals de documentatie van je hardware het beschrijft: bit 0 is uitgang 1, bit 6 is uitgang 7. RX is actief tijdens ontvangst op die band en TX terwijl de zender gestuurd wordt — geef ze dezelfde waarde voor een filter, en verschillende waarden voor alles dat maar aan één kant van de omschakeling hoort (de sleutelleiding van een versterker, de bypass van een ontvangstvoorversterker). Banden op 00 laten niets actief worden. Werkt bij Toepassen / opnieuw verbinden.
settings-radio-alex-band-code = ALEX BANDCODE
settings-radio-replace-table-what-preset-would = Vervang de tabel door wat deze voorinstelling op elke band zou sturen, als startpunt om te bewerken. Niets anders in de configuratie verandert.
settings-radio-clear = WISSEN
settings-radio-every-output-off-every-band = Elke uitgang uit op elke band, wat "Geen" doet.
settings-radio-band = Band
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Uitgangen actief
settings-radio-other = Overig
settings-radio-everywhere-outside-amateur-bands-short = Overal buiten de amateurbanden — kortegolfluisteren, en alles waar de dial van een transverter op uitkomt dat geen band dekt.
settings-radio-dongle = Dongle
settings-radio-rescan = Opnieuw zoeken
settings-radio-re-list-usb-bus-no = De USB-bus opnieuw weergeven. Er wordt geen apparaat geopend, dus dit kan veilig ingedrukt worden tijdens het ontvangen.
settings-radio-no-dongles-press-rescan = geen dongles — druk op Opnieuw zoeken
settings-radio-first-one-found = — eerste gevonden —
settings-radio-rtl2832u-s-resampler-reaches-225 = De resampler van de RTL2832U bereikt 225–300 kHz en 900 kHz–3,2 MHz, niets daartussen. Werkt bij Toepassen.
settings-radio-often-drops-samples = {"  "}(laat vaak samples vallen)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Handmatig is de instelling voor meten en digitale modi op zwakke signalen. De tuner en de demodulator hebben onafhankelijke automatische lussen.
settings-radio-tuner-gain = Tunergain
settings-radio-applies-immediately-no-reconnect-tuner = Werkt meteen — zonder opnieuw verbinden. De tuner heeft 29 afzonderlijke stappen, dus de waarde springt naar de dichtstbijzijnde die hij kan maken. Wordt genegeerd terwijl de tuner-AGC draait.
settings-radio-crystal-error-parts-per-million = Kristalfout in delen per miljoen. Draai met RUST_LOG=sdroxide_rtlsdr=debug en het logboek drukt na ongeveer 20 seconden de gemeten klookfout af — dat is het getal om in te voeren. Werkt meteen.
settings-radio-hf-reception = HF-ontvangst
settings-radio-tuner-itself-starts-24-mhz =
    De tuner zelf begint bij 24 MHz. Een RTL-SDR Blog V4 zet daaronder in de hardware zelf op; andere dongles bereiken HF alleen door de ADC rechtstreeks te bemonsteren, via de HF-poort van de V3. Automatisch geeft alles onder 24 MHz aan de ADC en alles erboven aan de tuner.

    Directe bemonstering dekt elke HF-band, 17 m en 15 m inbegrepen — ze komen in de tweede Nyquist-zone van de ADC binnen, rechtop. Niets filtert de ingang van de ADC, dus wat er op 28,8 MHz min de dial zit, komt er ook mee: 10,7 MHz onder 17 m, 7,726 MHz onder 15 m.

    Het wisselen van modus onderbreekt de stroom even.
settings-radio-removes-dongle-s-own-dc =
    Verwijdert de eigen DC-piek van de dongle uit het midden van het bereik, en het spiegelbeeld dat elk signaal daaromheen achterlaat, door de onbalans in de samples zelf te meten — geen kalibratie, en het werkt meteen. De tuner heeft geen offset-afstemmodus, dus dit is de enige manier om het midden vrij te maken.

    Een AM-draaggolf die precies op de dial is afgestemd, zit ook op DC, dus die gaat mee met de piek: stem een kilohertz ernaast, of zet dit uit.
settings-radio-remove-centre-spike-mirror-image = De middenpiek en het spiegelbeeld verwijderen
settings-radio-bias-tee = Bias-tee
settings-radio-feed-4-5-v-dc = Ongeveer 4,5 V DC over de coax voeden
settings-radio-bias-tee-never-connect-transceiver = De bias-tee staat AAN. Sluit nooit een transceiver, een geaarde antenne of een voorversterker die elders gevoed wordt aan terwijl dit aan staat — de gelijkspanning gaat rechtstreeks door de voedingslijn.
settings-radio-receive-only-dongle-sample-rate = Alleen ontvangen. De dongle en de samplefrequentie werken bij Toepassen; al het andere werkt zodra je het wijzigt.
settings-radio-server-address = Serveradres
settings-radio-where-rtl-tcp-listening-address =
    Waar rtl_tcp luistert: een adres, of een adres en poort. De poort staat standaard op 1234, de eigen standaard van rtl_tcp.

    Start hem aan het andere eind als `rtl_tcp -a 0.0.0.0` — gebonden aan 127.0.0.1, wat hij doet zonder -a, accepteert hij alleen verbindingen van diezelfde machine.

    Werkt bij Toepassen.
settings-radio-host-host-port-e-g = host of host:poort, bijv. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Gevraagd aan de server, met dezelfde resamplergrenzen als een lokale dongle — het is hetzelfde silicium aan het andere eind.

    Het getal naast elke snelheid is wat het op de verbinding kost: de samples worden ongecomprimeerd verstuurd, en een snelheid die het netwerk niet aankan, laat rtl_tcp de verbinding verbreken in plaats van degraderen. Werkt bij Toepassen.
settings-radio-wired-link = {"  "}(bedrade verbinding)
settings-radio-runs-server-s-dongle-manual = Draait op de dongle van de server. Handmatig is de instelling voor meten en digitale modi op zwakke signalen.
settings-radio-applies-immediately-no-reconnect-sent = Werkt meteen — zonder opnieuw verbinden. Verstuurd in tiende delen van een dB en door de server vastgezet op een stap die zijn tuner heeft; het protocol kent geen antwoorden, dus waar hij op uitkwam kan niet teruggelezen worden, en deze schuif blijft tonen wat er gevraagd is. Wordt genegeerd terwijl de tuner-AGC draait.
settings-radio-crystal-error-server-s-dongle =
    Kristalfout van de dongle van de *server*, in delen per miljoen — een eigenschap van die hardware, dus hij wordt hier ingesteld en niet op de dongles van deze machine. Werkt meteen.

    De gemeten klookfout die de USB-interface afdrukt, is hier niet beschikbaar: over een netwerk meet dat de buffering, niet het kristal, en het is duizenden ppm fout. Kalibreer de dongle één keer via USB en neem het getal over, of stem een omroepstation met een bekende frequentie af en stel bij tot het op de dial zit.
settings-radio-tuner-starts-24-mhz-below =
    De tuner begint bij 24 MHz; daaronder heeft het verre eind hulp nodig. Een Blog V4 zet zelf op en Automatisch laat hem met rust — wat het enige is wat het kan doen, want het protocol meldt de tunerchip en niets anders, en een V4 is over de draad niet te onderscheiden van een gewone R828D.

    Op een V3 of elke andere dongle schakelt Automatisch de server onder de eigen 24 MHz-ondergrens van de tuner over op directe bemonstering — wat elke HF-band dekt, met 17 m en 15 m die in de tweede Nyquist-zone van de ADC binnenkomen, met 28,8 MHz min de dial erbovenop gevouwen. Kies Directe bemonstering expliciet voor een gewone R828D die op HF niets hoort. Het omschakelen onderbreekt de stroom even.
settings-radio-removes-dongle-s-dc-spike =
    Verwijdert hier de DC-piek van de dongle en het spiegelbeeld, uit de samples zoals ze binnenkomen — het zijn artefacten van de hardware, dus ze reizen met al het andere over het netwerk. Werkt meteen.

    Een AM-draaggolf die precies op de dial is afgestemd, zit ook op DC, dus die gaat mee met de piek: stem een kilohertz ernaast, of zet dit uit.
settings-radio-powers-preamp-far-end-s = Voedt een voorversterker vanaf de dongle van het verre eind. Oudere servers implementeren het commando niet en negeren het stilzwijgend — het protocol heeft geen manier om nee te zeggen — dus een bias-tee die niet aangaat, is niet per se de schuld van dit eind.
settings-radio-feed-4-5-v-dc-2 = Ongeveer 4,5 V DC over de externe coax voeden
settings-radio-bias-tee-hardware-somewhere-else = De bias-tee staat AAN, op hardware die ergens anders staat. Wat er aan het andere eind van die voedingslijn zit — een transceiver, een geaarde antenne, een voorversterker die al gevoed wordt — staat niet voor je om te controleren.
settings-radio-sdrplay-server-rsp-tcp = SDRplay-server (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Een SDRplay-server begroet precies als een dongle, dus deze worden altijd getoond in plaats van alleen wanneer er een gedetecteerd wordt. Tegen een gewone rtl_tcp-server worden ze simpelweg genegeerd — het protocol heeft geen antwoorden en negeert commando's die het niet kent. Het tabblad Apparaat noemt de server rsp_tcp zodra die zichzelf heeft geïdentificeerd.
settings-radio-input = Ingang A
settings-radio-input-b = Ingang B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = LNA-stand
settings-radio-step-index-not-db-figure = Een stapindex, geen dB-waarde: hoeveel elke stap waard is hangt af van het RSP-model en de band. 0 is de meeste gain.
settings-radio-if-gain-reduction = IF-gainreductie
settings-radio-reduction-so-more-less-signal = Een reductie, dus meer is minder signaal. Wordt alleen gevolgd met de AGC van de RSP uit.
settings-radio-enable = Inschakelen
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = Het niveau dat de AGC van de RSP probeert vast te houden.
settings-radio-notches = Notches
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = RF
settings-radio-reference-out = Referentie-uit
settings-radio-t-24-mhz-clock-out = 24 MHz-klookuitgang
settings-radio-rsp2-rspduo-only = Alleen RSP2 en RSPduo.
settings-radio-receive-only-address-sample-rate = Alleen ontvangen. Het adres en de samplefrequentie werken bij Toepassen; al het andere werkt zodra je het wijzigt. Een verbroken verbinding wordt zelf opnieuw geprobeerd, dus een server die herstart wordt, komt terug zonder hier iets aan te raken.
settings-radio-where-spyserver-listening-address-address =
    Waar spyserver luistert: een adres, of een adres en poort. De poort staat standaard op 5555, de eigen standaard van spyserver.

    Controleer aan het andere eind dat het configuratiebestand een adres bindt dat andere machines kunnen bereiken — gebonden aan 127.0.0.1 accepteert hij alleen verbindingen van diezelfde machine.

    Werkt bij Toepassen.
settings-radio-host-host-port-e-g-2 = host of host:poort, bijv. raspberrypi.local:5555
settings-radio-i-q-bandwidth = I/Q-bandbreedte
settings-radio-how-wide-slice-band-arrives =
    Hoe breed een stuk van de band als I/Q binnenkomt — en dus hoeveel van de verbinding dit gebruikt. Dit is het *gedemoduleerde* venster: het volgt de dial, en alles wat breder is, is de FFT van de server in de strook boven het panadapter.

    Automatisch mikt op ongeveer 96 kHz, wat elke modus hier draagt, inclusief brede FM, en nog steeds past op een mobiele uplink. Werkt bij Toepassen.
settings-radio-which-stage-server-s-own =
    Welke trap van de eigen snelheidsladder van de server gevraagd wordt. Elke ontvanger heeft een andere ladder — het is zijn maximumsnelheid, trap voor trap gehalveerd — dus dit wordt opgeslagen als de trap en niet als een getal in hertz, en dezelfde instelling heeft nog steeds zin als hij naar een andere server wijst.

    Automatisch mikt op ongeveer 1 Msps. Druk op Verbinding testen om te zien wat de trappen op deze server opleveren. Werkt bij Toepassen.
settings-radio-automatic = Automatisch
settings-radio-sample-format = Sampleformaat
settings-radio-bits-per-component-wire-so =
    Bits per component op de draad, en dus wat dit op de verbinding kost: 16-bit is twee keer 8-bit, en 32-bit float is er vier keer zoveel voor niet meer informatie dan de ADC van de ontvanger had.

    8-bit is wat een externe ontvanger over een thuisuplink laat werken, en het is goed voor bijna alles. Een server kan dit overschrijven — sommige zijn geconfigureerd om één formaat af te dwingen — en meldt dat in het logboek als het gebeurt. Werkt bij Toepassen.
settings-radio-gain = Gain
settings-radio-server-s-gain-stage-index =
    De gaintrap van de server, als index — geen aantal decibel. Wat elke index betekent, hoort bij de ontvanger aan het andere eind en verandert met de band, en het protocol zegt het nooit, dus niets hier kan het in dB omrekenen zonder een getal te verzinnen.

    Het echte bereik is van de server en is pas bekend zodra er verbonden is; een index daarbuiten wordt begrensd. Werkt meteen — zonder opnieuw verbinden.
settings-radio-index = index{" "}
settings-radio-digital-gain = Digitale gain
settings-radio-how-far-server-scales-samples =
    Hoe ver de server zijn samples omhoog schaalt voordat hij ze kwantiseert voor de draad. Automatisch kijkt wat er werkelijk binnenkomt en houdt de piek op de helft van het volle bereik, wat het enige is dat op zowel een dode band als een drukke band juist kan zijn — die liggen dertig decibel uit elkaar en geen vaste waarde dient beide.

    Het telt alleen bij 8 bits, waar een signaal ver onder het volle bereik zijn lage bits verliest aan de kwantiseerder en een signaal erboven plat geknipt wordt. Bredere formaten hebben ruimte over en krijgen het getal dat elke andere client berekent, uit de gainindex en de decimatietrap. Werkt meteen.
settings-radio-full-band-strip = Strook over de volle band
settings-radio-server-s-own-fft-whole =
    De eigen FFT van de server over de hele band, getekend in de strook boven het panadapter. In deze interface is het de enige bandweergave die er is — het panadapter zelf is alleen zo breed als de I/Q die ontvangen wordt.

    Het uitzetten laat een ontvanger zonder manier om iets te zien waar hij niet al op afgestemd is. Werkt meteen, met een korte onderbreking van de audio terwijl de server verandert wat hij stuurt.
settings-radio-ask-server-low-rate-fft = Vraag de server om een FFT met lage snelheid van de hele band naast de I/Q, en teken die in de strook boven het panadapter.

    Het kost bijna niets — een paar kilobyte per frame, een dozijn keer per seconde of zo — en het toont de hele ontvanger in plaats van het deel dat gedemoduleerd wordt. Het is het waard om dit alleen op een verbinding waar elke byte telt uit te zetten. Werkt meteen, met een korte onderbreking van de audio.
settings-radio-show = Tonen
settings-radio-whole-band = hele band
settings-radio-how-much-receiver-strip-covers = Hoeveel van de ontvanger de strook bestrijkt. De hele band is de breedste weergave die er is; het smaller maken zet hetzelfde aantal bins over minder spectrum, wat fijner detail geeft over een kleiner stuk.
settings-radio-strip-db-window = dB-venster van de strook
settings-radio-server-quantises-fft-into-window =
    De server kwantiseert zijn FFT in dit venster voordat hij die verstuurt, één byte per bin — dus deze bepalen hoe fijn de strook gemeten wordt, niet alleen hoe hij getekend wordt. De ondergrens en bovengrens waarmee de strook *getoond* wordt, worden ingesteld door de automatische niveauregeling van de engine en zijn een apart ding.

    De standaard van 150 dB is het volledige bereik van het protocol en vraagt geen aandacht. Het versmallen rond de ruisvloer levert resolutie op bij een ontvanger waarvan alle signalen in een klein deel van de schaal zitten. Werkt meteen.
settings-radio-top = boven{" "}
settings-radio-range-2 = bereik{" "}
settings-radio-i-q-correction = I/Q-correctie
settings-radio-remove-dc-spike-mirror-image = De DC-piek en het spiegelbeeld in de DSP verwijderen, aan deze kant. Of de ontvanger aan het andere eind het nodig heeft, hangt af van wat hij is — een Airspy HF+ niet, een RTL-SDR wel — en het protocol zegt niet welke hij is. Werkt meteen.
settings-radio-enabled = Ingeschakeld
settings-radio-test-connection = Verbinding testen
settings-radio-connect-read-what-server-says = Verbinden, lezen wat de server over zijn ontvanger zegt, en weer verbreken — zonder een stroom te starten, dus het is veilig om in te drukken op een server die iemand anders gebruikt.
settings-radio-receive-only-panadapter-narrow-i = Alleen ontvangen. Het panadapter is het smalle I/Q-venster, dat de dial volgt; de bandweergave is de FFT van de server in de strook erboven. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-receive-only-wideband-i-q = Alleen ontvangen. Breedband-I/Q, hetzelfde als elke lokale SDR. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-receiver-address = Adres van de ontvanger
settings-radio-where-receiver-serves-web-page =
    Waar de ontvanger zijn webpagina aanbiedt: een adres, of een adres en poort. De poort staat standaard op 8073, de eigen poort van een KiwiSDR.

    Let op: een ontvanger die via de proxy van het project bereikt wordt (iets.proxy.kiwisdr.com — bijna de helft van de publieke) antwoordt op poort 80, dus geef die expliciet op. Een ontvanger kiezen uit "Publieke SDR's" vult dit correct in.

    Werkt bij Toepassen.
settings-radio-host-host-port-e-g-3 = host of host:poort, bijv. kiwi.example.org:8073
settings-radio-password = Wachtwoord
settings-radio-receiver-s-user-password-where =
    Het *gebruikers*-wachtwoord van de ontvanger, waar zijn operator er een heeft ingesteld. Bij bijna elke publieke ontvanger leeg. Niet het beheerderswachtwoord.

    Werkt bij Toepassen.
settings-radio-announce = Zich aanmelden als
settings-radio-name-connection-shows-up-under =
    De naam waaronder deze verbinding verschijnt, die de eigenaar van de ontvanger en iedereen die meeluistert kan zien.

    Leeg is het je eigen roepletters van het station, wat de conventie van het netwerk is — sommige operators blokkeren clients die zich niet identificeren. Werkt bij Toepassen.
settings-radio-blank-station-callsign = leeg = roepletters van het station
settings-radio-band-view = Bandweergave
settings-radio-ask-receiver-own-waterfall-well =
    Vraag de ontvanger om zijn eigen waterval naast zijn I/Q, en teken die in de strook boven het panadapter.

    Het is het waard: de I/Q is maar ongeveer 12 kHz breed, dus zonder dit is er niets om *op* af te stemmen. Het is een tweede verbinding die op volle snelheid ongeveer 20 kB/s draagt, tegenover de 44 kB/s van de I/Q.

    Werkt bij Toepassen.
settings-radio-show-receiver-s-0-30 = de waterval van de ontvanger van 0–30 MHz tonen
settings-radio-band-view-speed = Snelheid van de bandweergave
settings-radio-how-often-receiver-sends-waterfall =
    Hoe vaak de ontvanger een rij van de waterval stuurt, van 1 (traagst) tot 4. Dit is de enige instelling hier die tijdens het draaien verandert wat de verbinding kost, dus zet hem lager op een verbinding waarvoor je betaalt per volume.

    Werkt meteen.
settings-radio-band-view-span = Bereik van de bandweergave
settings-radio-how-much-band-receiver-s =
    Hoeveel band de waterval van de ontvanger bestrijkt. Zijn volledige 0–30 MHz staat links van de schuif, en elke stap naar rechts halveert het.

    De waterval is altijd 1024 bins breed, hoe breed het venster ook is, dus dit is eigenlijk een resolutieregelaar: de hele band is 29 kHz per bin, wat een *bandkaart* is en geen beeld van iets, terwijl een paar honderd kilohertz afzonderlijke zenders in een omroepband laat zien. Ingezoomd volgt het venster je dial.

    De hele band is de standaard en maakt de strook iets om op af te stemmen; het panadapter eronder is de ~12 kHz I/Q. Werkt meteen.
settings-radio-receiver-agc = AGC van de ontvanger
settings-radio-receiver-s-agc-which-far =
    De AGC van de *ontvanger*, die aan de andere kant van de verbinding zit en vóór de I/Q — dus die grijpt in voordat sdroxide iets doet.

    Standaard aan, anders dan elke lokale SDR hier, omdat het gemeten alternatief slechter was: op een live ontvanger was de handmatige gain niet monotoon over zijn bereik en klipte het hoge eind de I/Q op volle schaal, terwijl de AGC rond −24 dBFS bleef met 7 dB reserve.

    Het betekent wel dat de samplenamplitude geen signaalniveau is, en daarom wordt de S-meter afgelezen uit het eigen getal van de ontvanger. Werkt meteen.
settings-radio-let-receiver-ride-own-gain = de ontvanger zijn eigen gain laten volgen
settings-radio-manual-gain = Handmatige gain
settings-radio-fixed-gain-when-receiver-s = Vaste gain wanneer de AGC van de ontvanger uit staat, op zijn eigen schaal van 0–90. Niet in decibel van iets wat vermeld wordt — het protocol noemt het manGain en zegt verder niets, en het is gemeten als niet-monotoon, dus behandel het als een draaiknop om een goed punt op te vinden, niet als een kalibratie. Werkt meteen.
settings-radio-read-receiver-s-status-page = Lees de statuspagina van de ontvanger: wat hij is, wat hij bestrijkt, hoeveel kanalen vrij zijn, en of zijn operator verbindingen van andere apps dan een browser toestaat. Gebruikt geen van zijn kanalen.
settings-radio-host-port-e-g-127 = host:poort, bijv. 127.0.0.1:50001
settings-radio-iq-sample-rate = IQ-samplefrequentie
settings-radio-receiver = Ontvanger
settings-radio-rig-two-receivers-sunsdr2dx-can = Een radio met twee ontvangers (SunSDR2DX) kan twee radiotabbladen bedienen vanaf één verbinding — laat deze radio op RX1 draaien en een andere op RX2. De zender hoort bij de radio op RX1.
settings-radio-stream-delay = Streamvertraging
settings-radio-how-long-rig-s-iq = Hoe lang de I/Q van de radio nodig heeft om op een nieuw midden binnen te komen nadat sdroxide het verplaatst. Het is de eigen DSP-keten van de radio, niet het netwerk — een server op deze machine heeft er ook een — en terwijl het panadapter helemaal uitgezoomd gesleept wordt, verschuift een fout van deze grootte de nieuwste rijen van de waterval zijwaarts ten opzichte van de geschiedenis, met de fout maal de snelheid van de sleep. Te hoog verplaatst ze precies zo ver als te laag, de andere kant op. Meet het met `cargo run --release -p sdroxide-tci --example retune_latency`; de standaard is een SunSDR2DX op ExpertSDR3 bij 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = Breedband-IQ ontvangen, audio zenden. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-radio-address = Adres van de radio
settings-radio-address-shown-radio-under-set = Het adres dat op de radio onder SET > Network staat. Network Control moet daar aan staan, en de radio moet een netwerkgebruikersnaam en wachtwoord hebben.
settings-radio-host-ip-e-g-192 = host of IP, bijv. 192.168.1.50
settings-radio-control-port = Besturingspoort
settings-radio-t-50001-unless-has-been-changed = 50001, tenzij die op de radio is gewijzigd.
settings-radio-network-user = Netwerkgebruiker
settings-radio-stored-clear-radio-json-protocol = Wordt in leesbare tekst opgeslagen in radio.json. Het protocol verhult het omkeerbaar op de draad, dus niets hier zou het een geheim maken.
settings-radio-receive = Ontvangen van
settings-radio-af-radio-demodulates-sdroxide-shows = AF: de radio demoduleert en sdroxide toont de audioband. 12 kHz IF: de radio stuurt in plaats daarvan zijn DRM-tussenfrequentie en sdroxide demoduleert, waarmee de eigen filters, ruisonderdrukking en decoders over ongeveer ±12 kHz werken. Hoe dan ook is de brede waterval de eigen scope van de radio — geen Icom stuurt I/Q. Een IF-voorkeur gebruikt tijdelijk AF voor bredere modi zoals WFM, en zet daarna IF terug voor compatibele modi. Een AF-voorkeur blijft AF. HD Radio is niet beschikbaar op deze verbinding.
settings-radio-audio-sample-rate = Audio-samplefrequentie
settings-radio-t-12-khz-if-needs-48000 = Een 12 kHz IF heeft de 48000 Hz-stroom nodig — bij deze snelheid wordt in plaats daarvan de gedemoduleerde audio van de radio gebruikt.
settings-radio-if-spectrum = IF-spectrum
settings-radio-which-way-round-radio-s = Welke kant de 12 kHz IF van de radio op draait. "Automatisch" gebruikt wat het model bekendstaat om te doen — gespiegeld op een IC-7760, normaal op elke andere Icom. Stel het met de hand in als SSB op de verkeerde zijband uitkomt: het verraad is dat je USB moet kiezen waar de band LSB draait, terwijl de eigen modusaanduiding van de radio het de hele tijd met sdroxide eens is.
settings-radio-normal = Normaal
settings-radio-mirrored = Gespiegeld
settings-radio-displayed-bandwidth = Weergegeven bandbreedte
settings-radio-width-audio-band-panadapter-cat = Breedte van het panadapter van de audioband, zoals voor een CAT-radio. Gebruikt bij de digitale modi en wanneer het scherm van de radio uit staat; anders is het panadapter het scherm, en Bereik van het scherm bepaalt de breedte.
settings-radio-how-cw-panel-s-keyer-2 = Hoe de keyer van het CW-paneel zendt. "Radiokeyer" zet de radio op CW en geeft hem de tekst om met zijn eigen keyer te versturen (CI-V 0x17). "Geluidskaart" stuurt de gekeyde sidetone in plaats daarvan als audio over het LAN (MCW), een toon op dial + toonhoogte — en omdat een radio op CW zijn modulatie-ingang zou negeren, houdt CW-selectie de radio dan op gewone USB, dezelfde modus waarop de digitale modi meerijden.
settings-radio-how-much-audio-radio-holds = Hoeveel audio de radio vasthoudt voordat hij moduleert. Meer overleeft een slechter netwerk, ten koste van zendlatentie.
settings-radio-streams-radio-s-own-475 = Streamt de eigen sweep van 475 bins van de radio. Op AF is het het panadapter — de audio die de radio stuurt is wat door zijn filter kwam, geen beeld van de band — en op de 12 kHz IF is het de waterval over de hele band erboven. In beide gevallen is het het beeld van de radio, niet de DSP van sdroxide: er is geen I/Q om er een uit te berekenen.
settings-radio-how-wide-sweep-only-wide = Hoe breed het wordt afgeveegd. Dit is het enige brede beeld dat een Icom heeft, en de radio houdt het bereik vast dat het laatst op zijn eigen scherm gekozen is — vaak een paar kHz, en daarom kan de strook niet breder komen dan het panadapter eronder. Een bereik hier instellen zet het scherm ook in middenmodus, zodat het de dial volgt. Het verandert ook het eigen display van de radio; "Zoals ingesteld op de radio" laat het met rust.
settings-radio-switch-modulation-input-lan = Modulatie-ingang naar LAN schakelen
settings-radio-transmit-audio-only-heard-when = Zendaudio wordt alleen gehoord wanneer de MOD-ingang van de radio op LAN staat. sdroxide kan dat schrijven op een model waarvan het menunummering bekend is; op elk ander zegt het dat en laat het menu met rust. Het is een uitlening: wat de radio daar had staan, wordt eerst gelezen en teruggezet wanneer de sessie eindigt, zodat een radio die daarna alleen gebruikt wordt nog steeds zijn eigen microfoon hoort.
settings-radio-copy-diagnostic-report = Diagnoserapport kopiëren
settings-radio-copies-radio-s-last-session = Kopieert de laatste sessie van deze radio — de handshake en het CI-V-spoor — naar het klembord, voor een foutmelding. Van deze radio, niet van het station: met twee Icoms op het LAN antwoordt elk tabblad over zijn eigen adres.
settings-radio-control-audio-radio-s-own = Besturing, audio en het eigen scherm van de radio over één netwerkverbinding — geen seriële kabel en geen geluidskaart. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-testing = Testen…
settings-radio-was-only-check-press-apply = Dat was alleen een controle — druk hieronder op Toepassen / opnieuw verbinden om deze radio te gaan gebruiken.
settings-radio-radios = Radio's
settings-radio-asks-network-iio-devices-also = Vraagt het netwerk om IIO-apparaten, en probeert ook 192.168.2.1 rechtstreeks — een Pluto aan het eind van een USB-kabel is via multicast vaak niet bereikbaar, ook al werkt het adres wel.
settings-radio-no-radios-press-discover = geen radio's — druk op Zoeken
settings-radio-address = Adres
settings-radio-overrides-selection-above-usb-cable = Heeft voorrang op de selectie hierboven. De USB-kabel presenteert de Pluto als netwerkadapter, niet als seriële poort, dus dit is een IP-adres, ook als de radio op je bureau staat.
settings-radio-pluto-revision-c-pluto-unlocked = Een Pluto+ of een Pluto revisie C die naar 2R2T is ontgrendeld, kan twee radiotabbladen vanuit één doosje bedienen — deze radio op RX1 en een andere op RX2, elk op een eigen antenne. De twee ketens delen één oscillator, dus afstemmen op een van beide verplaatst beide; wat RX2 oplevert is een tweede antenne op hetzelfde spectrum (diversiteit), geen tweede band. De zender hoort bij de radio op RX1. Een standaard 1R1T-Pluto weigert RX2 bij het verbinden.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Breedte van het spectrum dat sdroxide ontvangt. De AD9361 haalt 61,44 Msps; de USB-netwerkverbinding niet, en daarop is deze lijst afgestemd. De laagste snelheden hebben een filterconfiguratie nodig die in de AD9361 geladen wordt, wat sdroxide niet doet — een standaard Pluto draait ze in plaats daarvan op ongeveer 2,084 Msps en zegt dat bij het verbinden. Werkt bij Toepassen.
settings-radio-more-than-usb-2-will = {"  "}(meer dan USB 2 aankan)
settings-radio-stock-pluto-runs-2-084 = {"  "}(een standaard Pluto draait op 2,084)
settings-radio-analog-filter = Analoog filter
settings-radio-ad9361-s-baseband-filter-leave = Het basisbandfilter van de AD9361. Laat op 0 voor automatisch, dat het opent tot negen tiende van de samplefrequentie — bewust breed, omdat de ontvanger zijn oscillator een kwart bereik van de dial wegzet om signalen vrij van de DC-piek te houden, en een smal filter zou precies het deel afsnijden waarheen ze verplaatst werden. Werkt bij Toepassen.
settings-radio-ad9361-has-four-modes-not = De AD9361 heeft vier modi, geen aan/uit-schakelaar. Langzame aanval past bij SSB en CW; snelle aanval bij signalen in pulsen; handmatig is de instelling voor meten en digitale modi op zwakke signalen. Werkt meteen.
settings-radio-rx-gain = RX-gain
settings-radio-applies-immediately-no-reconnect-ignored = Werkt meteen — zonder opnieuw verbinden. Wordt genegeerd tenzij de AGC op handmatig staat, wat het eigen gedrag van de AD9361 is, niet dat van sdroxide.
settings-radio-tx-gain = TX-gain
settings-radio-negative-because-ad9361-expresses-transmit = Negatief omdat de AD9361 het zendniveau als demping uitdrukt: 0 dB is volledig uitgangsvermogen. Werkt meteen. De zender wordt op zijn stilste gezet voordat deze waarde bij het verbinden wordt toegepast, zodat niets dat het vorige programma heeft achtergelaten actief kan zijn.
settings-radio-reference-error-parts-per-million = Referentiefout in delen per miljoen. Draai met RUST_LOG=sdroxide_pluto=debug en het logboek drukt na ongeveer 20 seconden de gemeten klokfout af — dat is het getal om in te voeren. Wordt door sdroxide toegepast, niet naar de eigen permanente trim van de radio geschreven. Werkt meteen.
settings-radio-full-duplex = Full duplex
settings-radio-keep-receiving-while-transmitting = Blijven ontvangen tijdens het zenden
settings-radio-leave-off-pluto-reached-over =
    Laat dit uit op een Pluto die via zijn USB-kabel bereikt wordt. Die verbinding kan niet tegelijk een megasample per seconde in beide richtingen aan, en een zending die de zendbuffer laat uitdrogen gaat er gehakt uit — dus standaard stopt ontvangst voor de duur van een zending.

    Zet het aan voor een bord op echt Ethernet (een LibreSDR, een Pluto op een gigabitadapter), waar ruimte is voor beide: je hoort dan de ontvanger door je eigen zending heen, wat is hoe een QO-100-station zijn eigen downlink afluistert.

    Een zending heeft twee keer de samplefrequentie aan verbinding nodig — 2,5 Msps is 10 MB/s per richting — dus verlaag de snelheid als het logboek begint te zeggen dat de verbinding het niet aankan. Het panadapter toont het zonden signaal ook tijdens een zending; het is de audio die blijft komen.

    Werkt bij Toepassen.
settings-radio-duplex = Duplex
settings-radio-whether-ad9361-runs-both-directions =
    Of de AD9361 beide richtingen tegelijk draait (FDD, zoals een Pluto opstart en waarin sdroxide hem altijd heeft achtergelaten) of één tegelijk (TDD).

    Laat dit op FDD tenzij je de PTT-pinnen hieronder wilt — TDD is waarmee die sturen, en het sluit Full duplex hierboven uit. Werkt bij Toepassen.
settings-radio-ptt-pins = PTT-pinnen
settings-radio-pluto-s-four-gpo-test =
    De vier GPO-testpunten van de Pluto kunnen op zichzelf een externe eindversterker, LNA of zend-ontvangstschakelaar sturen. Kies een paar en de ene pin staat de hele tijd hoog terwijl de radio ontvangt, de andere de hele tijd terwijl hij zendt — geen hostsoftware in de lus en geen seriële PTT-lijn om te bedraden.

    Dit zet de radio op TDD, wat de rij Duplex ook zegt, omdat de pinnen de enable-lijnen van de AD9361 volgen en in FDD staan die beide de hele sessie aan. Full duplex gaat er dan mee uit, dus laat beide op hun standaard als je satellieten werkt en je eigen downlink wilt horen.

    De eigen notitie van Analog Devices zet een externe LNA op GPO0/GPO1, dus gebruik GPO2/GPO3 als je bord zo bedraad is. De pinnen zijn ongeveer 1,3 V bij een paar milliampère: stuur er een transistor of een opto-isolator mee aan, nooit een relaisspoel.

    Werkt bij Toepassen.
settings-radio-buffer-size = Buffergrootte
settings-radio-how-much-device-holds-before =
    Hoeveel het apparaat vasthoudt voordat elke overdracht, in complexe samples. De standaard van 32768 is ongeveer 16 ms bij 2 Msps: lang genoeg dat de rondreis per overdracht geen knelpunt is, kort genoeg dat een hertuning niet zichtbaar laat wachten.

    Halveer hem als het logboek meldt dat de ontvangstsocket vervangen wordt. Een marginale verbinding — een Pluto die via zijn USB-kabel op een hoge samplefrequentie bereikt wordt, is de gebruikelijke — blijft halverwege een overdracht hangen, en een kleinere overdracht is minder snel midden in de vlucht geraakt en daarna sneller hersteld. Verhoog hem om hertunlatentie in te ruilen voor minder rondreizen.

    Werkt bij Toepassen.
settings-radio-samples = {" "}samples
settings-radio-rx-tx-port = RX-/TX-poort
settings-radio-ad9361-s-rf-port-select = De rf_port_select van de AD9361. Een standaard Pluto bedraadt er één van elk, dus laat deze leeg tenzij je een bord hebt dat dat niet doet. Werkt bij Toepassen.
settings-radio-opens-radio-reads-what-says = Opent de radio, leest wat hij over zichzelf zegt, en meldt het afstembereik van dit specifieke bord. Start geen stroom.
settings-radio-copies-last-session-s-protocol = Kopieert het protocolspoor van de laatste sessie naar het klembord, voor een foutmelding.
settings-radio-wideband-iq-receive-transmit-half = Breedband-IQ ontvangen en zenden. Halve duplex — ontvangst stopt voor de duur van een zending, omdat de USB-netwerkverbinding niet beide tegelijk aankan. Geen SoapySDR en geen libiio nodig.
settings-radio-not-yet-verified-against-real = Nog niet geverifieerd op echte hardware. Als het niet goed werkt, stuur dan het diagnoserapport — het bevat elk commando dat met de radio is uitgewisseld, en de eerste bytes van de samplestroom.
settings-radio-flexradio-announces-itself-local-network = Een FlexRadio kondigt zichzelf ongeveer eens per seconde op het lokale netwerk aan. Een radio die via een router of een VPN bereikt wordt, zendt nooit naar je uit — vul hieronder zijn adres in.
settings-radio-another-gui-client-has-radio = Een andere GUI-client heeft deze radio en multiFLEX staat uit. Verbreek die client, of zet multiFLEX aan op de radio.
settings-radio-overrides-selection-above-use-radio = Heeft voorrang op de selectie hierboven. Gebruik dit voor een radio op een ander subnet, achter een VPN, of op een niet-standaardpoort.
settings-radio-width-spectrum-sdroxide-receives-192 = Breedte van het spectrum dat sdroxide ontvangt. 192 kHz is het maximum van de radio voor een DAX-IQ-stroom, dus het breedste bereik dat deze interface kan tonen.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz is de breedste DAX-IQ-stroom die een FLEX verstuurt, en de radio koppelt het bereik van het panadapter eraan — dus dat is het hele bereik, en geen instelling hier maakt het breder.
settings-radio-dax-iq-channel = DAX-IQ-kanaal
settings-radio-radio-has-four-change-only = De radio heeft er vier. Verander dit alleen als iets anders op het netwerk al kanaal 1 gebruikt — de radio weigert een kanaal dubbel.
settings-radio-station-name = Stationsnaam
settings-radio-shown-against-session-radio-s = Wordt bij deze sessie getoond in de clientlijst van de radio. De radio onthoudt een client ook op deze naam, dus hernoemen laat de radio sdroxide als een nieuwe behandelen.
settings-radio-gui-client-id = GUI-client-ID
settings-radio-identity-radio-restores-session-s = De identiteit waarmee de radio de slices en panadapters van deze sessie herstelt. Blijft het leeg, dan wordt die afgeleid van de stationsnaam — stabiel, maar hetzelfde op elke sdroxide die de standaardnaam hield, en een radio lost een dubbele op door de eerdere client eruit te gooien. sdroxide herkent dat op de draad en valt terug op een identiteit voor één sessie, wat het herstel kost; zet hier een eigen UUID om dat te behouden.
settings-radio-optional-e-g-uuid-your = optioneel, bijv. een eigen UUID
settings-radio-mirror-radio-s-i-q =
    Spiegel de I/Q van de radio om het midden van het panadapter. Standaard uit, zo is een FLEX-6600 geverifieerd.

    Probeer het als de ontvangstaudio op USB *en* op LSB onverstaanbaar is en er niets decodeert, terwijl de waterval er volledig overtuigend uitziet — zo ziet een gespiegelde stroom eruit, en het is de enige storing zonder ander symptoom (issue #368). Als het dat niet is, maakt dit het aanzetten duidelijk in plaats van subtiel. Werkt bij Toepassen / opnieuw verbinden.
settings-radio-network-mtu = Netwerk-MTU
settings-radio-largest-datagram-radio-may-send = Het grootste datagram dat de radio mag verzenden. 1450 is wat SmartSDR zelf vraagt. Verlaag het op een pad met een kleinere MTU — een VPN of een tunnel — waar de fragmenten worden weggegooid en er helemaal geen spectrum aankomt.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Controleert of de radio antwoordt, zonder zich als GUI-client aan te melden — dus het verstoort geen SmartSDR-sessie die al loopt.
settings-radio-wideband-iq-receive-over-dax = Breedband-IQ ontvangen via DAX, audio zenden die de radio moduleert. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-not-yet-verified-against-real-2 = Nog niet geverifieerd op echte hardware. Als het niet goed werkt, stuur dan het diagnoserapport — het bevat elke protocolregel die met de radio is uitgewisseld.
settings-radio-devices-soapysdr-can-see = Apparaten die SoapySDR ziet
settings-radio-waiting-machine-radio-attached-where = Wachten op de machine waarop de radio is aangesloten, waar de modules geïnstalleerd zijn. Welk apparaat geopend wordt, is `device_args` in het config.toml van die machine, of zijn --device.
settings-radio-ask-every-installed-soapysdr-module = Vraag elke geïnstalleerde SoapySDR-module om te zoeken. Er wordt niets geopend, dus dit is veilig tijdens het ontvangen — maar het kan even duren.
settings-radio-not-enumerated-yet-press-rescan = Nog niet opgesomd — druk op Opnieuw zoeken.
settings-radio-no-soapysdr-devices-found-check = Geen SoapySDR-apparaten gevonden. Controleer of de module voor je radio geïnstalleerd is en of je het apparaat mag benaderen.
settings-radio-not-radio = geen radio
settings-radio-sound-card-listed-above-if = Een geluidskaart staat hierboven alsof het een SDR is. Die accepteert elke frequentie en negeert hem, dus openen toont de ingang van de geluidskaart in plaats van de band. sdroxide kiest die niet automatisch — maar een device_args-regel die er een noemt, wordt wel gevolgd.
settings-radio-which-one-opens-set-device = Welke geopend wordt, staat in --device of device_args in config.toml; zonder beide wint de eerste radio in de lijst.
settings-radio-no-device = geen apparaat
settings-radio-rig-has-no-software-adjustable = Deze radio heeft geen softwarematig regelbare gains.
settings-radio-rx-gains = RX-gains
settings-radio-tx-gains = TX-gains
settings-radio-antennas = Antennes
settings-radio-remembered-next-start = Onthouden voor de volgende start.
settings-radio-these-controls-appear-once-soapysdr = Deze regelaars verschijnen zodra een SoapySDR-apparaat open is — het zijn de eigen regelaars van het apparaat, en daar moet naar gevraagd worden.
settings-radio-stream = Stroom
settings-radio-follow-app-setting = De instelling van de app volgen
settings-radio-msps = of, in Msps
settings-radio-baseband-filter = Basisbandfilter
settings-radio-let-driver-choose = De driver laten kiezen
settings-radio-changing-either-reopens-radio = Het wijzigen van een van beide opent de radio opnieuw.
settings-radio-driver-publishes-no-settings-own = Deze driver publiceert geen eigen instellingen.
settings-radio-applied-once-remembered-next-start = Meteen toegepast, en onthouden voor de volgende start.
settings-radio-no-rx-888-found-press = Geen RX-888 gevonden — druk op Opnieuw zoeken
settings-radio-adc-clock = ADC-klok
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps vereist een SuperSpeed-verbinding en een snelle host; 64,8 is de veilige standaard. Wijzigen opent de ontvanger automatisch opnieuw, wat even duurt maar geen herstart nodig heeft.
settings-radio-panadapter-width = Breedte van het panadapter
settings-radio-hf-only = {"  "}· alleen HF
settings-radio-usable-both-front-ends = Bruikbaar op beide front-ends.
settings-radio-below-vhf-crossover-ordinary-width = Onder de VHF-overgang is dit een gewone breedte. Erboven kan de 8 MHz-IF van de tuner niet in een venster van deze breedte gecentreerd worden, dus het live spectrum staat aan één kant van het panadapter met niets ernaast.
settings-radio-dial-above-crossover-now-so = De dial staat nu boven de overgang, dus deze breedte toont de 8 MHz-IF van de tuner uit het midden met dood spectrum ernaast. Maak de breedte smaller om het panadapter te vullen.
settings-radio-vga-gain = VGA-gain
settings-radio-ad8370-variable-gain-amplifier-ahead = AD8370-versterker met variabele gain vóór de ADC.
settings-radio-attenuator = Verzwakker
settings-radio-pe4304-step-attenuator-0-5 = PE4304-stapverzwakker, in stappen van 0,5 dB.
settings-radio-adc-range = ADC-bereik
settings-radio-wide-2-25-vp-p = Breed (2,25 Vp-p)
settings-radio-selects-adc-s-wider-input = Kiest het bredere ingangsbereik van de ADC: meer ruimte voor sterke omroepsignalen, minder counts voor zwakke. Uit kiest het gevoeligere bereik van 1,5 Vp-p.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Voegt een klein ditherssignaal toe vóór de ADC: kost een beetje ruisvloer, levert vrij van stoorlijnen dynamisch bereik op.
settings-radio-randomiser = Randomizer
settings-radio-adc-scrambles-output-so-digital = De ADC schrambleert zijn uitvoer zodat de digitale bus niet meer in het front-end uitstraalt; de driver ontschrambelt hem. Laat dit aan tenzij je aan het debuggen bent. Werkt bij opnieuw verbinden.
settings-radio-dc-hf-antenna-port = DC op de HF-antennepoort
settings-radio-powers-active-antenna-preamp-down = Voedt een actieve antenne of voorversterker via de coax.
settings-radio-vhf-tuner-gain = VHF-tunergain
settings-radio-r828d-rf-gain-used-above = RF-gain van de R828D, gebruikt boven de automatische HF/VHF-overgang. 29 afzonderlijke stappen; de dichtstbijzijnde wordt gebruikt.
settings-radio-auto = Automatisch
settings-radio-let-tuner-run-own-lna = De tuner de eigen LNA- en mixerlussen laten draaien.
settings-radio-bias-tee-vhf = Bias-tee (VHF)
settings-radio-dc-vhf-antenna-port = DC op de VHF-antennepoort
settings-radio-clock-trim = Kloktrim
settings-radio-corrects-reference-oscillator-applied-when = Corrigeert de referentie-oscillator. Wordt toegepast zodra je de waarde loslaat — opnieuw openen bij elke pixel van een sleep zou de ontvanger honderden keren herstarten.
settings-radio-receive-only-below-adc-s = Alleen ontvangen. Onder de Nyquist-grens van de ADC wordt de antenne rechtstreeks bemonsterd en is hertunen direct, omdat er geen hardware-downconverter is — de volledige ADC-stroom wordt op de host naar basisband omgezet. Erboven schakelt de ontvanger automatisch over naar zijn R828D-tuner en de VHF-SMA, dus beide antennes moeten aangesloten zijn. VHF heeft een ADC-klok van 32,4 Msps of meer nodig, zodat de IF van de tuner past, en bij klokken onder 48 Msps is er een gat tussen de twee bereiken dat niets kan bereiken. Elke instelling hier werkt meteen — er is geen knop Toepassen om op te drukken.
settings-radio-device = Apparaat
settings-radio-no-devices-press-rescan = geen apparaten — druk op Opnieuw zoeken
settings-radio-six-rates-are-six-different =
    De zes snelheden zijn zes verschillende FPGA-images, geen register — daarom kiest niets in het protocol van ELAD tussen hen, en betekent wat deze instelling inhoudt iets anders per model.

    FDM-S1 / FDM-S2: een commando. Hun FPGA wordt bij elke inschakeling door de computer geladen, en sdroxide draait de eigen elad-firmwarelader van ELAD om het image van deze snelheid erin te zetten. Zonder die lader geïnstalleerd stuurt een sampler helemaal geen samples, en sdroxide zegt dat op het scherm.

    FDM-DUO: geen commando. De radio start zijn eigen FPGA en komt op welke snelheid hij bij het inschakelen had of FDM-SW2 laatst achterliet (192 kHz op een nieuwe), en dit zegt alleen hoe de stroom GELEZEN wordt. Gaat dat mis, dan is het panadapter gewoon de verkeerde breedte, met elke frequentie erin evenredig geschaald.

    De werkelijke snelheid van de stroom wordt een paar seconden na de start gemeten, en een afwijking wordt op het scherm gemeld. Werkt bij Toepassen.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: het FPGA-image dat geladen wordt — FDM-DUO: hoe de stroom gelezen wordt
settings-radio-receiver-s-input-pad-one = De ingangsverzwakker van de ontvanger — één stap, omhoog of omlaag. Dezelfde regelaar als de schuif Gain van het hoofdvenster.
settings-radio-pre-selection-filters = Voorselectiefilters
settings-radio-low-pass-bank-front-adc = De laagdoorlaatbank vóór de ADC. Die te omzeilen geeft de breedste weergave en het slechtste gedrag bij sterke signalen buiten de band — een omroepzender een paar mijl verderop zet beelden over de band. Laat hem aan, tenzij je bewust buiten het gefilterde bereik luistert.
settings-radio-filters-circuit = Filters in het circuit
settings-radio-rig-control-fdm-duo-only = Radiobesturing — alleen FDM-DUO
settings-radio-transceiver-s-cat-usb-port = De CAT-USB-poort van de transceiver, een aparte kabel van die voor ontvangst. Laat de poort leeg op een FDM-S1 of FDM-S2, die er geen hebben — en op een FDM-DUO wil je liever alleen via zijn ontvangstkabel sturen, waar hij nog steeds afgesteld en gestuurd kan worden maar geen meter gelezen kan worden.
settings-radio-port-set-radio-s-display = Met de poort ingesteld zijn het display van de radio en deze dial in beide richtingen hetzelfde getal: het ontvangstvenster van de FDM-DUO is om zijn VFO gebouwd, dus hier afstemmen verplaatst het voorpaneel, en aan de knop op het voorpaneel draaien verplaatst de uitlezing hier. Het panadapter centreert zich op de dial terwijl het beweegt. Zonder poort — of met een poort waarop de radio niet antwoordt — is er geen VFO om te bevelen, en blijft afstemmen binnen het venster dat de radio al stuurt.
settings-radio-none-usb-control-only = — geen (alleen USB-besturing) —
settings-radio-must-match-menu-70-cat = Moet overeenkomen met menu 70 "CAT BAUD" op de radio, die op 38400 staat. De radio heeft deze vier snelheden en geen andere, en een poort die op een andere geopend wordt, is in beide richtingen stil — de radio negeert de dial en weigert te zenden, zonder dat er iets zegt waarom.
settings-radio-how-transmit-keyed-cat-sends = Hoe zenden wordt aangestuurd. "CAT" stuurt het eigen TX-commando van de radio en vereist niets in te stellen op de radio; RTS vereist dat menu 54 "PTT" op PTT+RTS staat.
settings-radio-where-radio-takes-transmit-audio-2 =
    Waar de radio zijn zendaudio vandaan haalt — het TI-commando van de radio, dat menu 32 "TX IN" op het voorpaneel is. Wordt ingesteld zodra de poort opent.

    "USB-audio" is wat zenden hier laat werken: de FDM-DUO zendt wat sdroxide in zijn USB-geluidskaart zet. Een radio die op "Microfoon" staat, zendt in plaats daarvan de kamer, en niets op het scherm zegt dat.
settings-radio-which-two-sockets-back-receiver-2 =
    Op welke van de twee aansluitingen op de achterkant de ontvanger luistert — het AN-commando van de radio, dat menu 31 "ANTENNAS" op het voorpaneel is en de aanduiding "ANT 1 2" op het display.

    "RTX" is één antenne die beide taken doet, op de M-aansluiting die ook zenden draagt. "Alleen RX" verplaatst ontvangst naar de tweede aansluiting en laat zenden op RTX — een ontvangstantenne, een lus of een Beverage, met de bundel nog op de zender.

    Het verplaatst deze hele ontvanger: het panadapter, de demodulatoren en de eigen audio van de radio komen allemaal van de hier gekozen aansluiting.

    Werkt meteen, en wordt bij het openen van de besturingspoort uit de radio teruggelezen — dit is de eigen instelling van de radio, geen kopie die hier bewaard wordt.
settings-radio-how-often-radio-asked-what-2 = Hoe vaak de radio gevraagd wordt wat hij doet — zijn dial, zijn modus en zijn meters. Draai aan de eigen VFO-knop van de radio en de uitlezing volgt binnen één poll. Het is ook het hele besturingsverkeer van dit einde, dus verlaag hem als de audio van de radio breekt.
settings-radio-transmit-audio = Zendaudio
settings-radio-system-default = — systeemstandaard —
settings-radio-fdm-duo-s-own-usb = De eigen USB Audio-poort van de FDM-DUO. Blijft die op de systeemstandaard, dan is het bijna nooit de radio — het zijn dan de luidsprekers van de machine.
settings-radio-press-rescan-list-sound-cards = Druk op Opnieuw zoeken om de geluidskaarten te tonen
settings-radio-copies-last-session-s-trace = Kopieert het spoor van de laatste sessie naar het klembord, voor een foutmelding: elk commando dat met het apparaat is uitgewisseld, en de eerste bytes van de samplestroom.
settings-radio-only-receiving-fdm-duo-has = Alleen ontvangen op een FDM-DUO is tot nu toe op echte hardware uitgeprobeerd. Als er iets niet goed werkt, voeg dan het diagnoserapport toe aan een foutmelding.
settings-radio-cw-keyed-radio-s-own = CW wordt gestuurd met de eigen sleutel of paddle van de radio: de FDM-DUO heeft geen commando dat tekst aanneemt, dus het CW-paneel kan hem niet sturen.
settings-radio-press-apply-reopen-radio = Druk op Toepassen om de radio opnieuw te openen.
settings-radio-no-receivers-press-rescan = geen ontvangers — druk op Opnieuw zoeken
settings-radio-which-rates-receiver-has-depends = Welke snelheden een ontvanger heeft, hangt af van het model en de firmware. Werkt bij Toepassen.
settings-radio-every-rate-any-hf-model = elke snelheid die een HF+-model biedt — verbind er een om die van hem te zien
settings-radio-receiver-s-own-gain-control = De eigen gainregeling van de ontvanger. Laat die aan voor algemeen luisteren; zet hem uit om de verzwakker met de hand in te stellen voor metingen.
settings-radio-agc-threshold = AGC-drempel
settings-radio-high = Hoog
settings-radio-high-trades-little-sensitivity-headroom = Hoog ruilt wat gevoeligheid in voor reserve tegen sterke buren — de juiste instelling op een drukke band 's nachts.
settings-radio-front-end-attenuation-gain-0 = Front-endverzwakking, als gain — 0 dB is geen. Wordt alleen gevolgd met de AGC uit.
settings-radio-preamp = Voorversterker
settings-radio-hf-low-noise-amplifier-buys = De HF-ruisarme versterker. Koopt gevoeligheid in ruil voor intermodulatie, dus hij staat standaard uit — wat op een echte antenne meestal juist is.
settings-radio-lna = LNA aan
settings-radio-frequency-calibration = Frequentiekalibratie
settings-radio-parts-per-billion-receiver-s = Delen per miljard — de eigen eenheid van deze ontvanger, duizend keer fijner dan de ppm die een RTL-SDR gebruikt. Hier wordt nooit iets naar het flash-geheugen van de ontvanger geschreven: dit overschrijft de opgeslagen waarde alleen voor de sessie.
settings-radio-use-receiver-s-stored-value = De opgeslagen waarde van de ontvanger gebruiken
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = DC over de coax voeden
settings-radio-not-every-hf-has-one = Niet elke HF+ heeft er een; op een ontvanger zonder doet dit niets.
settings-radio-host-dsp = DSP op de host
settings-radio-image-balancer-zero-if-offset = De beeldbalancer, de zero-IF-offset en de oscillator voor fijnafstemming. Zet die alleen uit om de ruwe uitvoer van de hardware te zien — uitgeschakeld verschijnt het spiegelbeeld op de zero-IF-snelheden en is de dial alleen nauwkeurig tot op de dichtstbijzijnde kilohertz.
settings-radio-correct-image-fine-tune = Het beeld corrigeren en fijn afstemmen
settings-radio-copies-last-session-s-trace-2 = Kopieert het spoor van de laatste sessie naar het klembord, voor een foutmelding: elk commando dat met de ontvanger is uitgewisseld, en de eerste bytes van de samplestroom.
settings-radio-receive-only-no-soapysdr-no = Alleen ontvangen. Geen SoapySDR en geen libairspyhf nodig. Onder de ondergrens van de synthesizer doet de oscillator op de host het afstemmen, waarmee deze ontvanger het VLF-gebied bereikt. De ontvanger en de samplefrequentie werken bij Toepassen; al het andere werkt zodra je het wijzigt.
settings-radio-not-yet-verified-against-real-3 = Nog niet geverifieerd op echte hardware. Als het niet goed werkt, stuur dan het diagnoserapport — het bevat elk commando dat met de ontvanger is uitgewisseld, en de eerste bytes van de samplestroom gedecodeerd als I/Q-paren.
settings-radio-no-airspy-r2-mini-found = Geen Airspy R2 of Mini gevonden — druk op Opnieuw zoeken
settings-radio-read-receiver = gelezen van deze ontvanger
settings-radio-r2-mini-offer-different-rates = een R2 en een Mini bieden verschillende snelheden en zijn niet te onderscheiden tot er een open is
settings-radio-rate-you-get-receiver-s = Dit is de snelheid die je krijgt. De ADC van de ontvanger draait op het dubbele — hij digitaliseert een echt signaal en sdroxide maakt er op de host complex basisband van.
settings-radio-gain-curve = Gaincurve
settings-radio-step-along-curve-above-not = Een stap langs de curve hierboven, geen dB-waarde — de LNA, mixer en VGA van de tuner bewegen samen, en wat elke stap waard is, hangt af van de curve en de band. 0 is het stille eind.
settings-radio-tuner-agc = Tuner-AGC
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Geeft de LNA aan de eigen lus van de tuner, en negeert de curve.
settings-radio-mixer = Mixer
settings-radio-same-mixer-stage = Hetzelfde voor de mixertrap.
settings-radio-loop-running-gain-slider-no = Terwijl een lus draait, stelt de gainschuif de trap die de lus bezit niet meer in — de lus overschrijft hem.
settings-radio-dc-antenna-port = DC op de antennepoort
settings-radio-t-12-bit-packing = 12-bits pakken
settings-radio-third-less-usb-traffic-leave = Een derde minder USB-verkeer. Laat het aan: dit is een USB 2.0-apparaat en de hoogste snelheid is 30 MB/s gepakt tegenover 40 ongepakt. Werkt bij opnieuw verbinden.
settings-radio-dc-removal = DC-verwijdering
settings-radio-remove-adc-s-offset = De offset van de ADC verwijderen
settings-radio-turn-off-see-raw-hardware = Zet hem uit om de ruwe uitvoer van de hardware te zien. Goed om te weten waar de spur terechtkomt: de offset landt aan de rand van het bereik, niet in het midden, omdat het signaal onderweg met een kwart van de samplefrequentie verschoven wordt.
settings-radio-every-command-exchanged-receiver-sample = Elk commando dat met de ontvanger is uitgewisseld, de rekenkunde van de samplefrequentie, en de eerste samples gedecodeerd als I/Q.
settings-radio-receive-only-24-1800-mhz = Alleen ontvangen, 24–1800 MHz. Geen SoapySDR en geen libairspy nodig. De ontvanger, samplefrequentie en pakking werken bij Toepassen; al het andere werkt zodra je het wijzigt.
settings-radio-not-yet-verified-against-real-4 = Nog niet geverifieerd op echte hardware. Als het niet goed werkt, stuur dan het diagnoserapport — het bevat elk commando dat met de ontvanger is uitgewisseld, de rekenkunde van de snelheden, en de eerste samples gedecodeerd als I/Q-paren.
settings-radio-no-hydrasdr-rfone-found-press = Geen HydraSDR RFOne gevonden — druk op Opnieuw zoeken
settings-radio-these-are-rates-receiver-turned = dit zijn de snelheden die deze ontvanger bleek te hebben
settings-radio-receiver-only-reports-three-these = de ontvanger meldt er maar drie van deze; de rest staat in de alternatieve tabel van de firmware, en een oudere build heeft ze mogelijk niet
settings-radio-rf-input = RF-ingang
settings-radio-antenna-sma-only-socket-bias = De antenne-SMA — de enige aansluiting met de bias-tee erachter.
settings-radio-cable-socket-no-bias-tee = Een kabelaansluiting. Geen bias-tee hier; de hardware zet hem op ANT alleen.
settings-radio-powers-active-antenna-preamp-down-2 = Voedt een actieve antenne of voorversterker via de coax. Alleen de ANT-aansluiting heeft er een — de twee kabelpoorten zijn gewone ingangen.
settings-radio-third-less-usb-traffic-leave-2 = Een derde minder USB-verkeer. Laat het aan: dit is een USB 2.0-apparaat en de hoogste snelheid is 36 MB/s gepakt tegenover 48 ongepakt. Werkt bij opnieuw verbinden.
settings-radio-one-these-boards-1d50-60a1 = Een van deze borden zit op 1d50:60a1, de USB-id die de prototypes van HydraSDR delen met de Airspy R2 en Mini. sdroxide controleert de firmware na het openen en zegt het als de verkeerde interface gekozen is, in beide richtingen.
settings-radio-receive-only-24-1800-mhz-2 = Alleen ontvangen, 24–1800 MHz. Geen SoapySDR en geen libhydrasdr nodig. De ontvanger, samplefrequentie en pakking werken bij Toepassen; al het andere werkt zodra je het wijzigt.
settings-radio-re-list-fobos-sdrs-no = Fobos-SDR's opnieuw weergeven. Er wordt geen apparaat geopend, dus dit is veilig tijdens het ontvangen.
settings-radio-no-fobos-sdr-found-press = Geen Fobos SDR gevonden — druk op Opnieuw zoeken
settings-radio-input-2 = Ingang
settings-radio-through-tuner-lna-vga-mixer = Via de tuner: LNA, VGA en het eigen frequentiebereik van de mixer.
settings-radio-direct-sampling-hf1-antenna-input = Directe bemonstering op de HF1-antenne-ingang — geen tuner, geen LNA/VGA-gain, volledig in software afgestemd.
settings-radio-direct-sampling-hf2-antenna-input = Directe bemonstering op de HF2-antenne-ingang, net als HF1.
settings-radio-both-real-adc-channels-once = Beide echte ADC-kanalen tegelijk, gecombineerd door het diversiteitsfilter hieronder — een ruisbron die weggenuld wordt, of twee vervagende paden die gecombineerd worden.
settings-radio-direct-sampling-tuner-so-lna = Directe bemonstering — de tuner (en dus LNA/VGA-gain) zit niet in het pad. De snelheid hieronder is een doel voor de downconverter in software, geen hardware-instelling, dus de bereikte snelheid kan ervan afwijken.
settings-radio-receiver-s-own-reported-rates = de eigen gemelde snelheden van deze ontvanger
settings-radio-measured-one-real-unit-connect = gemeten op één echt exemplaar — verbind deze ontvanger en zoek opnieuw om de eigen lijst te zien
settings-radio-clock-source = Kloonbron
settings-radio-external-reference = Externe referentie
settings-radio-diversity-taps = Diversiteitstaps
settings-radio-taps = {" "}taps
settings-radio-one-tap-gain-phase-null = Eén tap is een gain en een fase — een nul op één frequentie die er aan weerszijden slechter van wordt, wat alles is wat een analoge phaser kan. Elke extra tap koopt één samplepériode van het padverschil tussen HF1 en HF2 dat het filter kan egaliseren, en dat maakt die inkeping tot een band die er overal stil is.
settings-radio-receive-only-rf-port-tuner = Alleen ontvangen. RF-poort (tuner), HF1/HF2 (directe bemonstering, in software gedecodeerd) en HF1+HF2 (beide tegelijk, gecombineerd door het diversiteitsfilter) werken allemaal. De ontvanger, ingang en samplefrequentie werken bij Toepassen; gain, kloonbron en taps werken zodra je ze wijzigt — de modus, de aanpassingssnelheid en het vasthouden van het diversiteitsfilter staan in het eigen vak DIV van het hoofdvenster zodra deze radio geselecteerd is.
settings-radio-no-hackrf-found-press-rescan = Geen HackRF gevonden — druk op Opnieuw zoeken
settings-radio-front-end-amplifier-8-db = Front-endversterker, in stappen van 8 dB. Dit is de trap die de gevoeligheid verandert — en de trap die als eerste overbelast raakt op een echte antenne.
settings-radio-baseband-amplifier-after-mixer-2 = Basisbandversterker na de mixer, in stappen van 2 dB. Draai dit omhoog voor een zwak signaal voordat je naar de LNA grijpt.
settings-radio-rf-amp = RF-versterker
settings-radio-one-switch-shared-transmit-setting = Eén schakelaar, gedeeld met de zendinstelling hieronder — de radio past degene toe die bij de richting hoort waarin hij gaat. Uit is op een echte antenne meestal juist.
settings-radio-hackrf-pro-chooses-itself-three = Een HackRF Pro kiest dit zelf — drie kwart van de samplefrequentie, gefilterd in de FPGA — en negeert alles wat de host vraagt.
settings-radio-leave-automatic-choosing-one-too = Laat op Automatisch. Er een te smal kiezen verzacht niet alleen de bandranden — het trekt de LO-offset in die de DC-piek van je signaal weghoudt.
settings-radio-about-3-v-50-ma = Ongeveer 3 V bij 50 mA over de coax, voor een actieve antenne of voorversterker. Alleen een HackRF One of Pro; de Jawbreaker en rad1o hebben zo'n schakeling niet.
settings-radio-remove-dc-mirror-image = DC en het spiegelbeeld verwijderen
settings-radio-zero-if-radio-so-own = Dit is een zero-IF-radio, dus zijn eigen LO-lek zit midden in het bereik en de kwadratuurfout van de mixer legt er een spiegelbeeld overheen. Dit uitzetten toont de ruwe uitvoer van de hardware, de snelle manier om een driverprobleem van een DSP-probleem te onderscheiden.
settings-radio-corrects-reference-oscillator = Corrigeert de referentie-oscillator.
settings-radio-transmit = Zenden
settings-radio-transmitter = Zender
settings-radio-off-default-while-off-backend = Standaard uit. Zolang dit uit staat publiceert de backend helemaal geen zendkanaal, dus niets kan de radio sturen. Werkt bij opnieuw verbinden.
settings-radio-tx-vga-gain = TX-VGA-gain
settings-radio-transmit-driver-amplifier-drive-applied = De zenddriverversterker. Aansturing wordt digitaal toegepast vóór deze trap, dus laat de aansturing hoog staan en stel het uitgangsniveau hier in — de aansturing omlaag zetten laat de DAC op een fractie van het volle bereik draaien en verhoogt de intermodulatie.
settings-radio-same-switch-receive-setting-above = Dezelfde schakelaar als de ontvangstinstelling hierboven, toegepast wanneer de radio van richting wisselt.
settings-radio-transmit-armed-into-dummy-load = Zenden staat gereed. Op een dummyload totdat je hem gemeten hebt: de harmonischen van een HackRF zijn sterk genoeg dat hij een externe laagdoorlaatfilter voor de band waarop je zit nodig heeft, en hij is halve duplex, dus ontvangst stopt voor de duur van elke zending.
settings-radio-every-command-exchanged-radio-session = Elk commando dat deze sessie met de radio is uitgewisseld, in volgorde, inclusief de reeks rond elke indrukking.
settings-radio-dc-7-25-ghz-half = DC – 7,25 GHz, halve duplex. Geen SoapySDR, geen libusb en geen libhackrf nodig. De radio en de samplefrequentie werken bij Toepassen; al het andere werkt zodra je het wijzigt. De bandranden komen van de firmware: een HackRF is gespecificeerd voor 1 MHz – 6 GHz (100 kHz op een Pro) en wordt daarbuiten sterk verzwakt, maar hij stemt er wel af.
settings-radio-ask-sdrplay-api-service-device = Vraag de SDRplay API-service om zijn apparatenlijst. Er wordt niets geopend, dus dit is veilig tijdens het ontvangen.
settings-radio-no-rsps-press-rescan-needs = geen RSP's — druk op Opnieuw zoeken (vereist de SDRplay API-service)
settings-radio-both-tuners-running-api-fixes = Met beide tuners in bedrijf zet de API de ADC vast op 6 MHz en levert 2 Msps uit een lage IF — dus 2 Msps is het breedste bereik, en de smallere zijn daarvan gedecimeerd. Werkt bij Toepassen.
settings-radio-rates-below-2-msps-run = Snelheden onder 2 Msps draaien de ADC op 2 Msps en decimeren in de service. Werkt bij Toepassen.
settings-radio-reduced-adc-resolution = {"  "}(verlaagde ADC-resolutie)
settings-radio-if-bandwidth = IF-bandbreedte
settings-radio-tuner-s-analog-filter-auto = Het analoge filter van de tuner. Automatisch kiest het breedste dat past bij de samplefrequentie. Werkt bij Toepassen.
settings-radio-rsp-s-own-if-gain = De eigen IF-gainlus van de RSP, gedraaid door de API-service. Uit geeft de IF-gainschuif weer aan jou — de instelling voor metingen en digitale modi op zwakke signalen.
settings-radio-agc-set-point = AGC-setpoint
settings-radio-signal-level-loop-holds-adc = Het signaalniveau dat de lus de ADC op houdt. Lager laat meer reserve voor signalen buiten het kanaal. Hoe laag het mag, hangt af van de samplefrequentie — boven 8,064 Msps heeft de converter minder reserve te geven, en boven 9,216 Msps nog minder.
settings-radio-rsp-s-native-gain-unit = De eigen gain-eenheid van de RSP: 20 dB is maximale gain (0 met het uitgebreide bereik hieronder), 59 dB minimaal. Werkt meteen. Wordt genegeerd terwijl de AGC draait — de lus bezit deze waarde dan, en de S-meter toont waar hij op uitkwam.
settings-radio-extended-if-range = Uitgebreid IF-bereik
settings-radio-lets-if-gain-reduction-go = Laat de IF-gainreductie onder 20 dB gaan, tot 0 — de laatste 20 dB gain die de ontvanger heeft. Uit is de eigen standaard van de API en de juiste keuze voor gewoon luisteren, omdat de onderkant van het bereik is waar een RSP het makkelijkst overbelast raakt. De moeite waard om aan te zetten voor zwakke signalen met de LNA al op 0. Het laat het AGC-setpoint ook tot 0 dBFS gaan. Werkt meteen.
settings-radio-allow-if-gain-reduction-below = IF-gainreductie onder 20 dB toestaan
settings-radio-front-end-attenuation-steps-0 = Front-endverzwakking in stappen: 0 is maximale gain, elke stap omhoog schakelt meer verzwakking in. Sommige banden hebben minder stappen — de driver begrenst en houdt je keuze vast voor wanneer je weer terugstemt, en de schaal hier eindigt bij wat de band waarop de radio staat aankan. Een hogere keuze op een andere band blijft bewaard en wordt er naast getoond. Werkt meteen.
settings-radio-t-0-max-gain = 0 = maximale gain
settings-radio-reference-error-parts-per-million-2 = Referentiefout in delen per miljoen, toegepast door het apparaat zelf. Werkt meteen.
settings-radio-radio-s-tuner = De tuner van deze radio
settings-radio-tuner = Tuner
settings-radio-which-rspduo-s-two-tuners = Welke van de twee tuners van de RSPduo deze radio afluistert. De andere hoort bij de radio die daarvoor is ingesteld — geef die dezelfde ontvanger en de andere tuner. Werkt bij Toepassen.
settings-radio-which-rspduo-s-two-tuners-2 = Welke van de twee tuners van de RSPduo de antenne draagt waarnaar je luistert. De andere draagt de tweede antenne. Werkt bij Toepassen.
settings-radio-which-rspduo-s-two-tuners-3 = Welke van de twee tuners van de RSPduo draaien (één tegelijk). Werkt bij Toepassen.
settings-radio-applies-immediately = Werkt meteen.
settings-radio-fm-broadcast-notch = FM-omroepinkeping
settings-radio-hardware-notch-over-88-108 = Hardware-inkeping over de FM-omroepband van 88–108 MHz, voor als een lokale zender alles andere overbelast. Werkt meteen.
settings-radio-dab-notch = DAB-inkeping
settings-radio-hardware-notch-over-165-230 = Hardware-inkeping over de DAB-band van 165–230 MHz. Werkt meteen.
settings-radio-hdr-mode = HDR-modus
settings-radio-enable-below-2-mhz = Inschakelen onder 2 MHz
settings-radio-rspdx-s-high-dynamic-range =
    Het pad met hoog dynamisch bereik van de RSPdx voor LF/MF. Werkt meteen.

    Het werkt nog niet: op de ene RSPdx waarop het is geprobeerd, dooft het inschakelen de ontvanger — bij elk centrum, filter, poort, gain en elke snelheid die geprobeerd is. Wat de modus nodig heeft, staat niet op het gedocumenteerde API-oppervlak, en uitzoeken wat dat is, is iemands volgende klus. Laat dit uit tenzij jij degene bent die ernaar kijkt.

    Wat bekend is: het is geen modus die de dial volgt. Het pad heeft een vast analoog filter, dat alleen gebouwd is op de centra die onder het filter hieronder staan, dus afstemmen ergens anders kan niet werken, zelfs als de rest wel werkt.
settings-radio-hdr-filter = HDR-filter
settings-radio-feed-4-7-v-dc = Ongeveer 4,7 V DC over de coax voeden
settings-radio-second-tuner = Tweede tuner
settings-radio-rspduo-two-whole-tuners-one = Een RSPduo is twee volwaardige tuners op één bord, geklokt vanaf één referentie. Laat ze allebei draaien en ze horen hun bereiken op hetzelfde moment van dezelfde klok — wat twee antennes erop coherent maakt, met een relatieve fase die door de voedingslijnen wordt bepaald en niet door toeval, en het paar laat combineren. Of laat ze apart: de tuners stemmen los af, dus de andere kan een tweede radio op een eigen band zijn.
settings-radio-run-both-tuners = Beide tuners laten draaien
settings-radio-puts-rspduo-api-s-dual = Zet de RSPduo in de dual-tuner-modus van de API, waarin de ADC vastligt op 6 MHz en het breedste bereik 2 Msps is. Werkt bij Toepassen, omdat de modus gekozen wordt wanneer het apparaat geopend wordt.
settings-radio-used = Gebruikt voor
settings-radio-what-other-tuner-takes-effect = Waar de andere tuner voor is. Werkt bij Toepassen — net als het gebruik van beide tegelijk wordt dit gekozen wanneer het bord geopend wordt.
settings-radio-add-second-radio-settings-radio = Voeg een tweede radio toe (Instellingen → Radio → +), geef hem dezelfde ontvanger en de andere tuner van de RSPduo, en zet hem ook op beide tuners — welke radio het bord ook opent, zet het in dual-tunermodus, dus beide moeten dat verwachten. De twee stemmen dan onafhankelijk af: HF in het ene tabblad en VHF in het andere, van één bord. Ze delen één ADC-klok, dus beide draaien op de samplefrequentie die welke radio het bord ook opende gevraagd heeft, en geen van beide kan zenden.
settings-radio-what-do = Wat ermee te doen
settings-radio-lna-state-2 = Zijn LNA-stand
settings-radio-second-tuner-s-own-front = De eigen front-endverzwakking van de tweede tuner. Zet die zo dat beide antennes ongeveer dezelfde ruisvloer tonen: dit is de afstelling waar alles op rust, want combineren weegt de twee takken naar hun ruis, en een tweede front-end die overbelast raakt, geeft het filter een vervormde kopie van de storing — die je niet van een niet-vervormde kan aftrekken. Werkt meteen.
settings-radio-if-gain-reduction-2 = Zijn IF-gainreductie
settings-radio-second-tuner-s-if-gain = De IF-gain van de tweede tuner, in de eigen eenheid van de RSP: 20 dB is maximale gain. Wordt genegeerd terwijl de AGC draait — en een stabiele gain is wat het filter wil, dus de AGC uitzetten loont voor een inkeping die je wilt vasthouden.
settings-radio-filter-length = Filterlengte
settings-radio-one-tap-gain-phase-null-2 = Eén tap is een gain en een fase — een nul op één frequentie die er aan weerszijden slechter van wordt, wat alles is wat een analoge phaser kan. Elke extra tap koopt één samplepériode van het padverschil tussen de twee antennes dat het filter kan egaliseren, en dat maakt die inkeping tot een band die er overal stil is.
settings-radio-adaptation-2 = Aanpassing
settings-radio-slow-steady-left-converging-inside = Links langzaam en gelijkmatig, convergerend binnen een fractie van een seconde en rechts zichtbaar zoekend. Begin snel om de nul te vinden, en houd hem dan vast.
settings-radio-stop-filter-moving-reach-moment = Laat het filter stoppen met bewegen. Grijp hiernaar het moment dat een nul verschijnt: een filter dat blijft aanpassen, richt zich opnieuw op wat het luidst wordt, en op een rustige band is dat het station waarnaar je luistert.
settings-radio-restart = Opnieuw starten
settings-radio-zero-filter-find-null-again = Het filter op nul zetten en de nul opnieuw zoeken.
settings-radio-nothing-here-can-tell-wanted = Niets hier kan een gewenst signaal van een ongewenst onderscheiden — het filter weet alleen wat de twee antennes gemeen hebben. Bij Annuleren wil de tweede antenne de ruisbron horen en zo weinig mogelijk van de band, anders annuleert hij braaf ook het station. Bij Combineren willen beide hetzelfde station horen. Hoe diep de nul wordt, staat om de paar seconden in het logboek.
settings-radio-not-yet-verified-against-rspduo = Nog niet geverifieerd op een RSPduo: dual-tunerwerking is hier geschreven vanuit de SDRplay-API en niet op de hardware gemeten.
settings-radio-no-receiver-listed-check-against = Er staat geen ontvanger om dit tegen te controleren — druk op Opnieuw zoeken, of lees in het logboek wat er werkelijk geopend is.
settings-radio-receive-only-1-khz-2 = Alleen ontvangen, 1 kHz–2 GHz. Vereist de API-service van SDRplay (sdrplay.com/api) — de RSP's na de oorspronkelijke RSP1 hebben geen open protocol. Apparaat, samplefrequentie, bandbreedte en RSPduo-tuner werken bij Toepassen; al het andere werkt zodra je het wijzigt.
settings-radio-board = Bord
settings-radio-first-one-found-2 = Eerste gevonden
settings-radio-receive-chain = Ontvangstketen
settings-radio-both-chains-tune-together-they = Beide ketens stemmen samen af — ze delen één synthesizer — maar het zijn afzonderlijke front-ends op aparte aansluitingen. Kies degene waarop je antenne zit; dat is de instelling om naar te grijpen wanneer de ene keten de HF-aanpassingsmodificatie heeft gehad en de andere standaard is.
settings-radio-receive-gain = Ontvangstgain
settings-radio-one-combined-figure-which-limesuite = Eén gecombineerd getal, dat LimeSuite zelf verdeelt over de LNA, de TIA en de PGA. Het neemt hele decibels, dus alles fijner wordt afgekapt.
settings-radio-receive-port = Ontvangstpoort
settings-radio-automatic-follows-frequency-lnal-low = Automatisch volgt de frequentie: LNAL laag, LNAH hoog — tenzij er hieronder een LimeRFE is aangesloten, wat één kabel in één aansluiting is, en dan is het LNAW op elke frequentie. Noem de aansluiting waarop de jouwe zit als dat niet die is. Tot welke keten de aansluiting hoort, is de kiezer hierboven.
settings-radio-t-0-follows-sample-rate-worth = 0 volgt de samplefrequentie. Het loont om het daar te laten staan: een filter dat smaller is dan een kwart van het bereik trekt stilletjes de zero-IF-LO-offset weg, waardoor het LO-lek weer bovenop het signaal komt dat je afluistert.
settings-radio-following-rate = volgt de snelheid
settings-radio-corrections = Correcties
settings-radio-host-iq-dc-correction = IQ-/DC-correctie op de host
settings-radio-adaptive-image-dc-removal-side = Adaptieve verwijdering van beeld en DC aan deze kant, bovenop de eigen kalibratie van de chip. Het uitzetten is de ene klik om een driverprobleem van een DSP-probleem te onderscheiden.
settings-radio-calibrate-automatically = Automatisch kalibreren
settings-radio-runs-chip-s-own-dc = Draait de eigen DC-offset- en beeldkalibratie van de chip wanneer de radio geopend wordt, en nogmaals zodra de dial zich op een nieuwe band of een andere aansluiting heeft gesetteld. Die getallen worden op één frequentie gemeten en zijn elders fout, wat meestal een draaggolf in het midden van het bereik is. Kost elke keer ongeveer een seconde.
settings-radio-calibrate-now = Nu kalibreren
settings-radio-stalls-receiver-better-part-second = Houdt de ontvanger voor het grootste deel van een seconde stil.
settings-radio-second-aerial = Tweede antenne
settings-radio-socket = Zijn aansluiting
settings-radio-same-first = Hetzelfde als de eerste
settings-radio-gain-2 = Zijn gain
settings-radio-set-low-coupled-sample-your = Zet dit LAAG. Het gekoppelde monster van je eigen zender is een sterk signaal, en een terugkoppelketen die in compressie gedreven wordt, meet de curve van de versterker verkeerd — het leert de correctie zijn eigen vervorming. Begin onderaan en gebruik de verzwakker van de koppeler.
settings-radio-set-so-both-aerials-show = Zet zo dat beide antennes ongeveer dezelfde ruisvloer tonen. Dit is de afstelling waar het hele ding op rust: combineren weegt de twee takken naar hun ruis, en een tweede keten die in compressie gedreven wordt, geeft het filter een vervormde kopie van de storing — die je niet van een niet-vervormde kan aftrekken.
settings-radio-table-steps-2 = Tabelstappen
settings-radio-how-finely-correction-follows-amplifier = Hoe fijn de correctie de curve van de versterker volgt. Meer stappen volgen een scherpere knik, maar elk moet geleerd worden uit de samples die erin vielen — en de top van een histogram van spraakamplitude is dun. Tweeëndertig is genoeg voor de gladde curve die een HF-versterker werkelijk heeft. De waarde wijzigen start de correctie opnieuw.
settings-radio-how-hard-each-block-feedback = Hoe hard elk blok terugkoppeling de correctie verplaatst. De curve van een versterker verandert niet, dus er is geen reden om haast te maken — het midden middelt er de ruis van meerdere zendingen uit.
settings-radio-keep-correction-curve-learned-clean = Houd de correctie zoals hij is. Een curve die op een schone zending geleerd is, is het waard om vast te houden, en de versterker zal bij de volgende niet veranderd zijn.
settings-radio-forget-correction-learn-again = Vergeet de correctie en leer hem opnieuw.
settings-radio-directional-coupler-amplifier-s-output = Een richtkoppeler op de uitgang van de versterker gaat in deze keten, en de zender vergelijkt wat terugkwam met wat hij bedoelde te zenden — en stuurt dan het omgekeerde van het verschil, zodat wat de versterker verlaat recht is. Twintig-en-iets decibel minder intermodulatie op andermans QSO's, terwijl de versterker zijn vermogen houdt. De correctie blijft op eenheid tot de terugkoppeling op de zending aansluit, dus een koppeler die niet aangesloten is, kost niets; en hij kan de converter nooit meer dan volle schaal vragen, dus een terugkoppelpad dat onzin meet, kan niets overdrijven. Hoe het gaat, staat tijdens het zenden in het logboek.
settings-radio-transmit-not-armed-so-there = Zenden staat niet gereed, dus er valt hier niets te corrigeren.
settings-radio-off-interface-publishes-no-transmit = Met dit uit publiceert de interface helemaal geen zendkanaal, dus niets kan de radio sturen.
settings-radio-transmit-gain = Zendgain
settings-radio-transmit-port = Zendpoort
settings-radio-board-s-two-transmit-sockets = De twee zendaansluitingen van het bord zijn twee verschillende aanpassingsnetwerken, geen twee aansluitingen op hetzelfde: BAND1 (TX_1) draagt 30 MHz tot 1,9 GHz en BAND2 (TX_2) is de 13 cm-poort. Een amateurband onder 23 cm via BAND2 zenden, stopt de zending in een aanpassingsnetwerk dat er bijna niets van doorlaat — elke instelling leest goed, de aansturing is wat je zet, en de vermogensmeter blijft op nul. Laat dit op Automatisch staan, dan volgt het de dial.
settings-radio-limesdr-transmits-about-100-khz = Een LimeSDR zendt van ongeveer 100 kHz tot 3,8 GHz zonder eigen filtering. Gebruik een laagdoorlaatfilter, een passend LimeRFE-kanaal of een dummyload.
settings-radio-limerfe-front-end = LimeRFE-front-end
settings-radio-connected = Verbonden door
settings-radio-limerfe-s-own-micro-usb = De eigen micro-USB-poort van de LimeRFE, niet die van de radio. 9600 baud, vastgelegd door zijn firmware.
settings-radio-receive-connector = Ontvangstconnector
settings-radio-transmit-connector = Zendconnector
settings-radio-follow-dial = De dial volgen
settings-radio-switch-filters-match-operating-frequency = Schakel de filters om zodat ze bij de werkfrequentie passen, voordat er RF verschijnt. Afstemmen binnen één band zet niets op de besturingsverbinding.
settings-radio-relays = Relais
settings-radio-receive-attenuator = Ontvangstverzwakker
settings-radio-notch-filter = Inkepingsfilter
settings-radio-fan = Ventilator
settings-radio-worth-having-any-sustained-transmitting = De moeite waard om aan te hebben bij elk langdurig zenden.
settings-radio-automatic-board-receives-switched-transmit = Op Automatisch ontvangt het bord, en wordt bij het indrukken naar zenden geschakeld en bij het loslaten terug — op beide bedradingen. De amateurkanalen hebben één filter met aan weerszijden een zend-/ontvangstschakelaar, dus een bord dat beide tegelijk vraagt zet die schakelaar op de zender en hoort niets meer. Het 30 MHz-kanaal is alleen via J5 bereikbaar, één connector voor beide richtingen; daarboven houdt zenden vanaf J4 het ontvangstpad weg van de connector die de versterker aanstuurt.
settings-radio-limesdr-support-has-not-been = LimeSDR-ondersteuning is niet geverifieerd op hardware. Dit is het spoor van de laatste sessie, voor een issuemelding.
settings-radio-gains-filters-corrections-antenna-sockets = Gains, filters, correcties, de antenneaansluitingen en elke LimeRFE-regelaar — zijn connectoren, band, relais, verzwakker, inkeping en ventilator — werken meteen. Het bord, de ontvangstketen, de samplefrequentie, het gereedzetten van zenden en de verbinding van de LimeRFE werken bij Toepassen.

## Settings window, Radio tab: messages with values

settings-radio-fill-from = VULLEN UIT { $name }
settings-radio-decimation-auto = Automatisch (dichtstbij { $khz } kHz)
settings-radio-decimation-stage-server = Trap { $stage } — de snelheid van de server ÷ { $divisor }
settings-radio-decimation-stage = Trap { $stage } — snelheid ÷ { $divisor }
settings-radio-kiwi-receive-only = Alleen ontvangen — dit is andermans antenne. Het panadapter is de ~12 kHz die de ontvanger als I/Q stuurt, en die volgt de dial; het bandvenster erboven is zijn eigen waterval, en over het bereik heen afstemmen stemt de ontvanger af. Ongeveer { $rate } kB/s terwijl verbonden. Druk op "Toepassen / opnieuw verbinden" om zonder herstart te schakelen.
settings-radio-connected-detail = Verbonden: { $detail }
settings-radio-failed = Mislukt: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB per overdracht
settings-radio-driver-settings = Instellingen van { $driver }
settings-radio-rx888-coverage = 0–{ $mhz } MHz dekking, { $mbytes } MB/s over USB
settings-radio-rx888-digitised = van de { $mhz } MHz gedigitaliseerd
settings-radio-elad-16-bit = { $khz } kHz (16-bits samples)
settings-radio-elad-pad = { $db } dB pad in
settings-radio-hackrf-rx-preamp = { $db } dB voorversterker bij ontvangst
settings-radio-hackrf-tx-preamp = { $db } dB voorversterker bij zenden
settings-radio-device-not-found = { $device } (niet gevonden)
settings-radio-lime-chain = Keten { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Keten { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = De andere ontvangstketen van het bord, op de { $sockets }-aansluitingen. Die deelt de synthesizer, dus hij hoort hetzelfde bereik op hetzelfde moment als de eerste — wat hem een tweede antenne laat dragen, of een monster van je eigen zender.
settings-radio-tx-blocked = Zenden is geblokkeerd: { $reason }
settings-radio-rfe-rx-fallback = Ontvangen op { $port }; deze vallen terug op het ongefilterde breedbandpad: { $bands }.
settings-radio-rfe-tx-fallback = Zenden op { $port }; deze vallen terug op het breedbandpad — geen bandversterker en geen filtering: { $bands }.

## Top bar and control strip (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Hoeveel één druk de dial verplaatst. Tik om de volgende stap te nemen: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 en 25 kHz. Zet de rij uit in Instellingen › UI.
topbar-volume-gain-agc-squelch-noise = Volume, gain, AGC, squelch, de ruisregelaars, en de RDS-uitlezing bij FM-omroep
topbar-receiver = ONTVANGER
topbar-vfo-b-split-rit-xit = VFO A/B, split, en de RIT/XIT-offsets
topbar-tuning = AFSTEMMEN
topbar-diversity-filter-which-way-combines = Het diversiteitsfilter: op welke manier het de twee antennes combineert, hoe snel het zich aanpast, en het vasthouden ervan waar het staat
topbar-diversity = DIVERSITEIT
topbar-second-receiver-s-frequency-mode = De frequentie, modus, het filter en het niveau van de tweede ontvanger
topbar-sub-receiver = SUBONTVANGER
topbar-transceiver-s-own-aerial-socket = De eigen antenneaansluiting van de transceiver, en zijn aan/uit-schakelaar
topbar-radio = RADIO
topbar-tune-voice-keyer-drive-mic = Afstemmen, de spraakkeyer, en de niveaus van aansturing en microfoon
topbar-transmit = ZENDEN
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = Het panadapter — de twee lagen, hun snelheden en detail, peak hold — plus contrast van de waterval, FFT-grootte en de skimmers
topbar-display = WEERGAVE
topbar-logbook-spots-awards-memories-scanner = Logboek, spots, prijzen, geheugens, de scanner, instellingen en de handleiding
topbar-system = SYSTEEM
topbar-linked = Gekoppeld
topbar-not-linked = Niet gekoppeld
topbar-bar-trace = staaf / spoor
topbar-needle-bar-trace = naald / staaf / spoor
topbar-swap-vfos = VFO's wisselen
topbar-copy-b = A naar B kopiëren
topbar-second-receiver-right-ear-tunes = Tweede ontvanger, in het rechteroor. Hij stemt onafhankelijk van A/B af — zijn regelaars staan in de module SUB, en zijn doorlaatband op de waterval.
topbar-band-plan = , uit het bandplan
topbar-repeater-shift-following-band-plan = Repeaterverschuiving: volgt het bandplan, dat voor deze frequentie geen verschuiving heeft — simplex
topbar-repeater-shift-transmit-above-below = Repeaterverschuiving: zend boven of onder de dial om een repeater te werken. Nu simplex
topbar-repeater-shift = REPEATERVERSCHUIVING
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Neem de verschuiving uit het bandplan terwijl de dial beweegt. Het spreekt alleen binnen een uitgangssubband van een repeater — overal anders laat het de radio simplex, zodat de roepkanalen simplex blijven.
topbar-offset = Offset
topbar-repeater-tone-ctcss-dcs-goes = Repeatertoon: de CTCSS/DCS die onder de stem uitgaat, de burst van 1750 Hz, en de toonsquelch bij ontvangst
topbar-transmit-tone = ZENDTOON
topbar-set-here-sent-nfm-sub = hier ingesteld, verstuurd op NFM — subhoorbare signalering hoort bij een FM-kanaal
topbar-normal = NORMAAL
topbar-invert = OMKEREN
topbar-bit-order-dcs-encoded-here = De bitvolgorde waarmee DCS hier gecodeerd wordt, is uit de standaard overgenomen en is nooit tegen een repeater gecontroleerd — als hij niet opent, probeer CTCSS
topbar-t-1750-hz-burst = 1750 HZ-BURST
topbar-every-over = ELKE ZENDING
topbar-open-burst-every-time-transmitter = Openen met de burst elke keer dat de zender gestuurd wordt
topbar-send = VERSTUREN
topbar-send-burst-now-keying-transmitter = De burst nu versturen — de zender voor die duur sturen als hij nog niet gestuurd wordt
topbar-nfm-only-burst-fm-repeater = Alleen NFM: de burst is de deuropener van een FM-repeater
topbar-receive-tone-squelch = TOONSQUELCH BIJ ONTVANGST
topbar-require-receive-what-station-transmits = Bij ontvangst eisen wat dit station zendt
topbar-hide-docked-band-selector-click = De vastgezette bandkiezer verbergen — klik opnieuw om hem terug te halen
topbar-show-docked-band-selector = De vastgezette bandkiezer tonen
topbar-dock = VASTZETTEN
topbar-keep-band-mode-selector-open = De band- en modus-kiezer naast de waterval open houden, in plaats van dit venster elke keer te sluiten
topbar-band-mode = BAND & MODUS
topbar-hide-band-chip-brings-back = Verbergen — het bandkaartje haalt hem terug
topbar-undock = LOSMAKEN
topbar-return-selector-top-bar-popup = De kiezer terugzetten in het pop-upvenster van de bovenbalk
topbar-dec-off = DEC uit
topbar-vol = Vol
topbar-counted-steps-not-decibels-what =
    {""}

    Geteld in stappen, niet in decibel: wat één stap waard is, is de eigen zaak van de ontvanger, en op een SDRplay hangt het ook van de band af. Rechts is nog steeds meer gain.
topbar-gain = Gain
topbar-plutosdr-s-own-agc-setting =
    {""}

    De eigen AGC van de PlutoSDR stelt deze gain in, dus het verschuiven van de schuif doet niets. Zet Instellingen → Radio → AGC op Handmatig om hem hier te regelen.
topbar-agc-hang-time-click-cycle = Hangtijd AGC — klik om te wisselen: Uit / Langzaam / Gemiddeld / Snel
topbar-radio-s-own-agc-click = De eigen AGC van de radio — klik om te wisselen: Langzaam / Gemiddeld / Snel. Hij kan vanaf hier niet uitgeschakeld worden.
topbar-man = Hand
topbar-manual-audio-gain-used-while = Handmatige audiogain, gebruikt terwijl de AGC uit staat. Begint op het niveau dat de AGC vasthield toen hij uitgeschakeld werd.
topbar-radio-s-own-squelch-sent = De eigen squelch van de radio, verstuurd over de besturingsverbinding. Dit is de poort waar de audio daadwerkelijk doorheen gaat — de software-squelch zou alleen nog verder sluiten op wat de radio al doorliet.
topbar-max-gain = maximale gain
topbar-manual-gain = handmatige gain
topbar-noise-reduction = ruisonderdrukking
topbar-impulse-noise-blanker = Impulsruisblanker
topbar-auto-notch-cancel-constant-tone = Automatische notch: constante toonelementen wegnemen (heterodynes)
topbar-noise-reduction-off-click-switch = Ruisonderdrukking, uit — klik om hem in te schakelen, of kies een engine in de rijen hieronder
topbar-binaural-audio-not-while-sub = Binaurale audio — niet terwijl de subontvanger het rechteroor heeft
topbar-binaural-audio-passband-spread-across = Binaurale audio: de doorlaatband wordt over de twee oren verdeeld, zodat signalen op verschillende toonhoogten uit verschillende richtingen komen en het signaal dat je afstemt, over de band heen zweeft. Klik om terug te gaan naar mono
topbar-binaural-cw-spread-passband-across = Binaurale CW: verdeel de doorlaatband over de twee oren, zodat signalen op verschillende toonhoogten uit verschillende richtingen komen — een pile-up wordt meerdere plekken in plaats van één volle toon, en een station afstemmen laat het over de band heen zweven. Het best met een koptelefoon
topbar-binaural-audio-spread-passband-across = Binaurale audio: verdeel de doorlaatband over de twee oren. Bij spraak verspreidt de ruis zich over het hele beeld terwijl het station in het midden blijft, wat makkelijker is om een uur naar te luisteren — ten koste van dat de spraak zelf uitgespreid klinkt. Het best met een koptelefoon
topbar-mute = DEMPEN
topbar-record-audio-raw-i-q = De audio, de ruwe I/Q, of beide opnemen
topbar-wfm-stereo-forced-off-click = WFM-stereo geforceerd uit — klik voor automatische stereo
topbar-wfm-stereo-pilot-locked-click = WFM-stereo: pilot vergrendeld. Klik om mono te forceren
topbar-wfm-stereo-automatic-no-pilot = WFM-stereo: automatisch, geen pilot op dit station
topbar-rds-data-arriving-click-see = RDS-gegevens komen binnen — klik om ze te zien
topbar-rds-no-data-station-click = RDS — geen gegevens op dit station. Klik toch voor de diagnose van de decoder
topbar-drm-decoding-click-details = DRM wordt gedecodeerd — klik voor de details
topbar-drm-click-decoder-s-state = DRM — klik voor de status van de decoder
topbar-hd-radio-decoding-click-details = HD Radio wordt gedecodeerd — klik voor de details
topbar-hd-radio-click-decoder-s = HD Radio — klik voor de status van de decoder
topbar-presets = VOORINSTELLINGEN
topbar-preset-right-click = Rechtsklik om deze voorinstelling te wijzigen
topbar-preset-edit = VOORINSTELLING BEWERKEN
topbar-preset-label = Label
topbar-preset-label-hint = Wat er op de chip staat. Maximaal 8 tekens.
topbar-preset-lo = Laag
topbar-preset-hi = Hoog
topbar-preset-use-current = Huidige gebruiken
topbar-preset-use-current-hint = De randen van het huidige filter van de ontvanger overnemen
topbar-preset-save = Opslaan
topbar-preset-reset = Herstellen
topbar-preset-reset-hint = De ingebouwde voorinstelling van deze modus terugzetten
topbar-preset-cancel = Annuleren
topbar-width-per-sideband = BREEDTE PER ZIJBAND
topbar-width = BREEDTE
topbar-passband-width-hertz-type-drag = Breedte van de doorlaatband in hertz — typ hem, of sleep. Klik in het veld om een exacte waarde in te voeren.
topbar-edges = RANDEN
topbar-low-edge-hz-carrier = Lage rand, in Hz vanaf de draaggolf
topbar-high-edge-hz-carrier = Hoge rand, in Hz vanaf de draaggolf
topbar-noise-reduction-voice-off-click = Ruisonderdrukking (spraak), uit — klik om een engine te kiezen
topbar-record-audio = AUDIO OPNEMEN
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = Wat je hoort, als MP3: de ontvanger in het ene kanaal en je eigen zending in het andere. Dit is de opname van een QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Gemengd naar één kanaal — klik voor twee
topbar-two-channels-rx-left-tx = Twee kanalen: RX links, TX rechts — klik voor één gemengd kanaal
topbar-quick-clip = SNELLE CLIP
topbar-stop-after = STOPPEN NA
topbar-no-stop = geen stop
topbar-auto-record = AUTO-OPNAME
topbar-set-squelch-auto-record-needs = Stel een squelch in — auto-opname heeft er een nodig om stilte te herkennen
topbar-auto-record-needs-sdr-front = Auto-opname heeft een SDR-front-end met een software-squelch nodig
topbar-stop-following-squelch = Niet meer de squelch volgen
topbar-waiting-signal = wachten op een signaal
topbar-record-spectrum = SPECTRUM OPNEMEN
topbar-i-q-wav = ● I/Q-WAV
topbar-i-q-wav-2 = I/Q-WAV
topbar-raw-spectrum-receiver-delivering-stereo = Het ruwe spectrum dat de ontvanger levert, als stereo 32-bits float-WAV (RF64 boven 4 GB) die SDR#, SDRuno, HDSDR en SDRangel openen — en die sdroxide zelf afspeelt met --file. Groot: 8 bytes per sample.
topbar-radio-hands-over-demodulated-audio = Deze radio levert gedemoduleerde audio, dus er is geen I/Q om op te nemen.
topbar-what-capture-costs-sample-rate = Wat een opname kost bij deze samplefrequentie
topbar-engine = ENGINE
topbar-no-noise-reduction-decoders-never = Geen ruisonderdrukking — de decoders hebben het toch nooit gezien
topbar-strength = STERKTE
topbar-tone = TOON
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — geen subhoorbare toon op dit signaal
topbar-tone-squelch = TOONSQUELCH
topbar-carrier-squelch-open-any-signal = Draaggolfsquelch: open bij elk signaal
topbar-require-tone-currently-being-received = Eisen dat de toon die nu ontvangen wordt
topbar-ctcss = CTCSS
topbar-any-dcs = ELKE DCS
topbar-open-any-dcs-coded-signal = Open bij elk DCS-gecodeerd signaal. Welke van de 104 codes het draagt, kan hier niet betrouwbaar gelezen worden, dus er valt niets fijners te kiezen
topbar-which-socket-back-radio-receiving =
    Op welke aansluiting op de achterkant de radio ontvangt — het eigen ANT-commando, dezelfde instelling als de knop ANT op het voorpaneel. Klik om naar de volgende te gaan.

    De keuze wordt per band onthouden, en teruggezet de volgende keer dat de dial die band in komt.
topbar-radio-s-separate-receiving-antenna =
    De aparte ontvangstantenne van de radio, in of uit het ontvangstpad geschakeld — de eigen instelling RX ANT. De antenne op de hoofdaansluiting blijft in beide gevallen op zenden.

    De radio onthoudt dit zelf per band, dus sdroxide leest het na elke bandwissel terug in plaats van terug te zetten wat het laatst zag: klikken hier is het enige dat het verplaatst.
topbar-radio-s-own-power-switch =
    De eigen aan/uit-schakelaar van de radio, via de besturingsverbinding, en de enige echte aan/uit in het programma — niet de schakelaar LINK van sdroxide, die het eigen einde van sdroxide sluit en de radio aan laat staan.

    Om AAN iets te laten bereiken, moet het besturingseinde van de radio wakker blijven terwijl hij uit staat: Network Control over het LAN, of een CI-V-poort die nog op het lichtnet zit bij een toestel dat aan de voorkant uitgezet is.
topbar-switch-radio = De radio inschakelen
topbar-switch-radio-off-audio-meters = De radio uitschakelen. De audio en de meters stoppen ermee.
topbar-what-second-aerial-click-swap =
    Waar de tweede antenne voor is — klik om te wisselen.

    ANNULEREN trekt die van de eerste af, met de gain, fase en vertraging die een lokale ruisbron op beide gelijk laten lopen: de DSP-vorm van een ruisonderdrukkende phaser. COMBINEREN telt de twee op in de fase die versterkt, gewogen naar hoe goed elk hoort — diversiteitsontvangst, die vervagingen opvult.
topbar-hold = VASTHOUDEN
topbar-stop-filter-moving-reach-moment = Laat het filter stoppen met bewegen. Grijp hiernaar het moment dat een nul verschijnt: een filter dat blijft aanpassen, richt zich opnieuw op wat het luidst wordt, en op een rustige band is dat het station waarnaar je luistert.
topbar-restart = OPNIEUW
topbar-zero-filter-find-null-again = Het filter op nul zetten en de nul opnieuw zoeken.
topbar-adapt = Aanpassen
topbar-how-fast-filter-chases-slow = Hoe snel het filter achter aan zit: links langzaam en gelijkmatig, convergerend binnen een fractie van een seconde, en rechts zichtbaar zoekend. Begin snel om de nul te vinden, en VASTHOUDEN dan.
topbar-where-sub-receiver-listens-shift = Waar de subontvanger luistert. Shift-klik op de waterval, of sleep binnen de doorlaatband van de sub, om hem te verplaatsen.
topbar-dial = ←DIAL
topbar-move-sub-receiver-main-dial = De subontvanger naar de hoofddial verplaatsen
topbar-dial-2 = DIAL←
topbar-move-main-dial-sub-receiver = De hoofddial naar de subontvanger verplaatsen
topbar-filter = Filter
topbar-sub-receiver-passband-edges-hz = Randen van de doorlaatband van de subontvanger, in Hz
topbar-sub-receiver-level-plays-right = Niveau van de subontvanger (het speelt in het rechteroor)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Spraakkeyer: opgeslagen berichten opnemen en zenden
topbar-drive = Aansturing
topbar-tune-2 = Afstemmen
topbar-tx-audio-processing-microphone-click = TX-audio bewerkt de microfoon — klik om hem te openen
topbar-click-open-tx-audio-gate = Klik om TX-audio te openen: gate, EQ, compressor en begrenzer voor de microfoon
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Microfoongain
topbar-deviation-how-far-mode-s =
    Deviatie: hoe ver de burst van deze modus een radio uitslaat die hem zelf moduleert. Een FM-zender zet het audioniveau om in frequentie-uitslag en heeft geen ALC om het op te vangen, dus volle schaal in een dataingang die voor spraak is ingesteld, overdrijft de deviatie — wat voor een luisteraar volkomen normaal klinkt en voor niemand decodeert.

    Per modus onthouden, zodat een deviatie die voor 1200 baud pakketten is ingesteld nooit op FT8 terechtkomt.
topbar-transmit-audio-how-hard-mode =
    Zendaudio: hoe hard deze modus de modulator aanstuurt van een radio die zelf moduleert wat wij hem sturen — een CAT-radio via zijn geluidskaart, een FLEX, een Icom via zijn netwerkpoort. Draai hem omlaag tot de ALC van de radio nauwelijks beweegt, en stel het vermogen op de radio in; ALC die meeloopt bij een digitale modus met constante envelop is wat het uitsmeert.

    Aansturing is niet deze regelaar: op deze radio's bereikt Aansturing het vermogensregister van de radio en raakt het zijn audio nooit.

    Per modus onthouden — FT8, RTTY, PSK en MCW houden elk hun eigen waarde.
topbar-tx-audio = TX-audio
topbar-cw-psk-rtty-skimmers-decode = CW / PSK / RTTY-skimmers — decoderen signalen over de hele band en markeren ze op de waterval
topbar-skimmers = SKIMMERS
topbar-run-skimmer = Deze skimmer draaien
topbar-minimum-snr-decoded-signal-needs = Minimale SNR die een gedecodeerd signaal nodig heeft om gespot te worden
topbar-needs-wideband-iq-source = heeft een breedband-IQ-bron nodig
topbar-cw-decoder = CW-DECODER
topbar-how-many-signals-model-reads = Hoeveel signalen het model tegelijk leest. De rest houdt zijn markering, maar draagt geen tekst.
topbar-spots = SPOTS
topbar-save-skimmer-s-spot-list = De spotlijst van de skimmer naar een bestand opslaan
topbar-show-hide-full-band-waterfall = De waterfall-strook over de hele band boven het panadapter tonen/verbergen — alles wat deze ontvanger tegelijk kan zien
topbar-spectrum-waterfall-either-layer-both = Spectrum en waterval — één laag, beide, of geen. Brandt terwijl beide getoond worden.
topbar-spectrum = SPECTRUM
topbar-show-spectrum = SPECTRUM TONEN
topbar-draw-spectrum-line-across-top = De spectrumlijn over de bovenkant van het panadapter tekenen. Uitgeschakeld neemt de waterval de volle hoogte in.
topbar-peak-hold = PEAK HOLD
topbar-trace-highest-level-each-column = Het hoogste niveau dat elke kolom heeft bereikt over de live lijn tekenen, terugzakkend — wat de band deed terwijl je ergens anders keek
topbar-draw-spectrum-receding-surface-instead = Het spectrum als een wegtrekkend oppervlak tekenen in plaats van een platte lijn: het nieuwste spectrum vooraan, de eerdere stromend weg van je. De laatste paar seconden van de band als landschap — een draaggolf die komt en gaat is een rug in plaats van een lijn die trekt. Het raster en de peak hold horen bij de platte lijn en worden er niet op getekend.
topbar-how-quickly-spectrum-line-follows = Hoe snel de spectrumlijn de band volgt. Langzamer middelt meer frames in elkaar: een rustigere lijn, en een zwakke draaggolf die stil genoeg blijft staan om af te lezen. De waterval wordt er niet door geraakt — die rijen krijgen in beide gevallen elk frame.
topbar-waterfall = WATERVAL
topbar-show-waterfall = WATERVAL TONEN
topbar-draw-scrolling-waterfall-below-spectrum = De scrollende waterval onder het spectrum tekenen. Uitgeschakeld neemt de spectrumlijn de volle hoogte in.
topbar-how-fast-waterfall-scrolls-lines = Hoe snel de waterval scrolt, in lijnen per seconde: Langzaam 5, Gemiddeld 28, Snel 56, Sneller 112, Snelst 224. De engine klokt deze zelf, dus de twee snelste zijn echt detail en niet dezelfde lijn twee keer getekend — voor zover de ontvanger ze kan voeden: een lijn kan nooit meer dan één transformatie tonen, en een smalle front-end levert er maar een paar tientallen per seconde. Ze kosten geschiedenis, want de waterval houdt een vast aantal lijnen bij — 73 seconden bij Gemiddeld, 9 bij Snelst.
topbar-decode-labels = DECODE-LABELS
topbar-mark-every-decoded-station-waterfall = Elk gedecodeerd station op de waterval markeren met zijn roepletters, op de frequentie waarop het gehoord werd — FT8, FT4 en de andere slot-modi. Een goede opening zet er dertig over het bereik, twee keer per minuut, over de sporen die je leest heen; zet ze uit en de waterval is gewoon de band. De decodeerlijst ernaast heeft nog steeds elke roepletter.
topbar-switch-3d-set = Schakel 3D eerst in om dit in te stellen
topbar-how-3d-spectrum-drawn-only = Hoe het 3D-spectrum getekend wordt. Alleen de vorm verschilt — beide weergaven tonen dezelfde spectra, en beide verbergen wat erachter ligt, dus een sterk signaal vooraan staat boven de band die het bedekt.
topbar-lines = LIJNEN
topbar-one-trace-per-remembered-spectrum = Één spoor per onthouden spectrum, in de eigen kleur van de platte lijn, elk het spoor erachter verbergend. De vorm zonder de niveaus, en de keuze wanneer het de vorm is die je zoekt — het kost iets meer dan de volle variant in plaats van minder, omdat het hetzelfde oppervlak tekent en daarna elke kam erbovenop streept.
topbar-solid = VOL
topbar-filled-surface-coloured-waterfall-s = Een gevuld oppervlak gekleurd met het kleurenpalet van de waterval, zodat het niveau zowel in de kleur als in de hoogte zit en de twee helften van het panadapter het eens zijn over hoe een sterk signaal eruitziet. Verander het palet in Instellingen › Weergave.
topbar-how-fast-3d-spectrum-flows = Hoe snel het 3D-spectrum van je wegstroomt, in rijen per seconde: Langzaam 6, Gemiddeld 12, Snel 24, Sneller 48. Het oppervlak is een vast aantal rijen diep, dus dit is ook hoeveel tijd het vasthoudt. Langzamer is een langer geheugen en een oppervlak dat kruipt; sneller is een korter geheugen dat beweegt. De waterval houdt zijn eigen scrollsnelheid aan, hieronder.
topbar-how-many-columns-panadapter-waterfall = Hoeveel kolommen het panadapter en zijn waterval getekend worden. Auto leest de renderer van deze machine en de grootte van het panadapter en kiest het meeste wat ze aankunnen — een 4K-scherm wil 4096. Elke kolom is een byte in elk frame, dus een client die met een station op afstand verbonden is, betaalt voor het detail op zijn verbinding: 4096 kolommen bij 60 fps is ongeveer een kwart megabyte per seconde, twee keer de standaardbreedte.
topbar-read-machine-screen-take-most = Lees deze machine en dit scherm, neem het meeste wat ze aankunnen, en volg ze als ze veranderen
topbar-width-force-right-now-whatever = De breedte die nu van kracht is, wat de rij hierboven ook vraagt
topbar-keep-floor-ceiling-set-best = De vloer/het plafond voor het beste contrast van de waterval houden — terug op zijn plaats gezet bij een bandwissel, na een verschuiving of zoom, en wanneer de niveaus wegzakken. Zet het aan om meteen te passen; zet het uit om de niveaus te houden waar je ze zette.
topbar-keep-tuned-frequency-middle-panadapter = De afgestemde frequentie in het midden van het panadapter houden: het venster schuift onder de dial in plaats van dat het beeld een volle bereik springt wanneer je van de rand afstemt. Zet het aan om meteen te centreren; zet het uit om te verschuiven en zoomen waar je wilt.
topbar-spectrum-floor-ceiling-fft-size = Vloer / plafond van het spectrum en FFT-grootte
topbar-level-drawn-bottom-spectrum-darkest = Het niveau dat onderaan het spectrum getekend wordt en als de donkerste kleur van de waterval. Draai het omhoog tot de ruisvloer net donkerder wordt. Het instellen zet FIT uit.
topbar-ceil = plafond{" "}
topbar-level-drawn-top-bring-down = Het niveau dat bovenaan getekend wordt. Draai het omlaag tot het sterkste signaal dat je belangrijk vindt de volle kleur bereikt. Het instellen zet FIT uit.
topbar-fft-size = FFT-GROOTTE
topbar-flip = OMDRAAIEN
topbar-scroll-waterfall-upwards-newest-row = De waterval omhoog scrollen — nieuwste rij onderaan (V)
topbar-logbook-all-qsos-digital-manual = Logboek — alle QSO's (digitaal + handmatig)
topbar-live-spots-dx-cluster-pota = Live spots — DX-cluster, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Prijzenregistratie — DXCC / WAS / WAZ / locatievakken
topbar-band-conditions-published-forecast-beside = Bandcondities — de gepubliceerde verwachting naast wat er op elke band werkelijk gehoord is
topbar-satellite-qo-100-beacon-calibration = Satelliet — de kalibratie van het QO-100-baken draait
topbar-satellite-doppler-tracking-qo-100 = Satelliet — Doppler-tracking, en de kalibratie van het QO-100-baken
topbar-ism-band-devices-decoding-now = ISM-bandapparaten — decoderen nu, ook als dit venster niet open is. Zet het uit met DECODEREN in het venster.
topbar-ism-band-devices-weather-sensors = ISM-bandapparaten — weersensoren, meters en huisautomatisering die om je heen gehoord worden
topbar-public-sdrs-internet-browse-kiwisdr = Publieke SDR's op het internet — blader door de KiwiSDR- en SpyServer-mappen en open er één als radio
topbar-winlink-radio-email = Winlink-radio-e-mail
topbar-memory-channels = Geheugenkanalen
topbar-scanner-stopped-signal = Scanner — gestopt op een signaal
topbar-scanner-running = Scanner — draait
topbar-scan-memory-channels-frequency-range = Geheugenkanalen of een frequentiebereik scannen
topbar-hfdl-ground-network-decode-log = HFDL-grondnetwerk — het decodeerlogboek en de vliegtuigkaart, onder de waterval. Zet de decoder aan met LUISTEREN in het paneel.
topbar-hfdl-ground-network-decoding-now = HFDL-grondnetwerk — decodeert nu. Open het paneel.
topbar-hfdl-ground-network-aircraft-shortwave = HFDL-grondnetwerk — de kortegolf-datalink voor vliegtuigen, één luisterkanaal tegelijk
topbar-grid-tracker-worked-maidenhead-squares = Locatievak-tracker — gewerkte Maidenhead-vakken op een kaart, met wat er gehoord wordt
topbar-amplifier-set-up-hardrock-under = Versterker — stel een Hardrock in onder Instellingen → Versterker
topbar-settings-device-gains-antennas-audio = Instellingen — gains van apparaten, antennes, audioapparaten
topbar-user-manual-f1 = Gebruikershandleiding (F1)
topbar-range-was-typed-settings-radio = Dat bereik is ingetypt in Instellingen ▸ Radio ▸ RX-bereik, niet door de radio gemeld. Verbreed het, of maak het vak leeg om te gebruiken wat het apparaat over zichzelf zegt.
topbar-what-device-reports-about-itself = Dat is wat het apparaat over zichzelf meldt. Als de radio deze band wel dekt, geef dan zijn echte bereik op in Instellingen ▸ Radio ▸ RX-bereik.
topbar-band = BAND
topbar-recent = RECENT
topbar-recent-empty =
    Vult na 3 s op een plek
topbar-mode = MODUS
topbar-digital = DIGITAAL

## Top bar: the System box's window chips.

topbar-chip-log = LOG
topbar-chip-spots = SPOTS
topbar-chip-awards = PRIJZEN
topbar-chip-bands = BANDEN
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = PUBLIEKE SDR
topbar-chip-mail = MAIL
topbar-chip-mem = GEH
topbar-chip-scan = SCAN
topbar-chip-hfdl = HFDL
topbar-chip-grid = GRID
topbar-chip-amp = VERSTERKER
topbar-chip-settings = ⚙ INSTELLINGEN
topbar-chip-help = ? HELP

## Top bar: the mode-defaults chip.

topbar-changed-agc = AGC
topbar-changed-squelch = squelch
topbar-changed-auto-notch = automatische notch
topbar-changed-stereo = stereo
topbar-changed-binaural = binauraal
# $mode is a mode name (USB, CW…); $changed lists what differs from its defaults.
topbar-mode-defaults =
    Terug naar de standaarden van { $mode }. Gewijzigd ten opzichte daarvan: { $changed }.

    Klik om ze terug te zetten. De eigen waarden van { $mode } komen terug, en wat je hier instelde, wordt vergeten.

## Settings window, Radio tab: longer messages with values

settings-radio-no-such-baud = de radio heeft geen instelling voor { $baud } baud — in plaats daarvan wordt { $default } gebruikt
settings-radio-icom-scope-hint =
    Streamt de eigen scopesweep van de radio over de CI-V-verbinding en tekent die als het panadapter — het enige beeld van de band dat een radio met gedemoduleerde audio kan geven, omdat de audio die hij stuurt al door zijn filter gekomen is.

    Vereist dat de radio ervoor ingesteld is: CI-V USB Baud Rate op 115200 en CI-V USB Port op "Unlink from {"["}REMOTE]" (beide onder SET > Connectors > CI-V), en de baudrate hierboven daarmee gelijk — onder { $min_baud } passen de sweeps niet door de verbinding en blijft de scope uit.

    De sweeps delen de interne USB-bus van de radio met zijn eigen geluidskaart. Als ontvangen audio begint weg te vallen met de scope aan, is dit vakje het eerste om uit te zetten.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = Hardware hierboven wordt rechtstreeks door sdroxide ondersteund: { $names }. Die interface hierboven kiezen geeft je de eigen instellingen — gaintrappen, filters en inkepingen die SoapySDR niet kan uitdrukken — en vereist geen SoapySDR-module.
settings-radio-rx888-width-hint = Hoeveel van het gedigitaliseerde spectrum het panadapter tegelijk toont. De hele DSP-keten draait op deze breedte, dus breder kost evenredig meer CPU — 1/2 is de hele band in de waterval, en een flink stuk rekenwerk. Boven de VHF-overgang ({ $crossover } MHz) kijkt het panadapter niet naar de antenne maar naar de IF van de tuner, die 8 MHz breed is en op { $if_centre } MHz staat — dus alles breder dan { $widest } kan er niet op gecentreerd worden, en de extra breedte is dood spectrum naast het signaal in plaats van meer ervan.
settings-radio-sdrplay-serial-missing = Serienummer { $serial } staat niet tussen de ontvangers die de SDRplay-service meldt. Kies een van de vermelde ontvangers, of steek deze opnieuw in en druk op Opnieuw zoeken.
settings-radio-hdr-filter-hint =
    Het analoge filter vóór het HDR-pad — een andere regelaar dan de eigen bandbreedte van de ontvanger. Elke instelling is op een vaste reeks centra gebouwd en kan elders niets doen; deze is gebouwd op { $centres }. Werkt meteen.

    Het pad zelf werkt nog niet — zie de schakelaar hierboven — dus dit kiest een filter voor een modus die nu stil is.
settings-radio-rspduo-only = Alleen een RSPduo heeft een tweede tuner, en de hier gekozen ontvanger is een { $model }, dus dit doet niets.
settings-radio-lime-tx-gain-low = Zendgain is { $db } dB, onderaan het bereik — een paar microwatt uit het bord, wat op een vermogensmeter als niets gelezen wordt, wat er ook achter zit. Dat is de standaard zodat een gereed staande zender je niet kan verrassen; verhoog hem voordat je stuurt.

## Settings window, General: SWR

settings-general-swr-tune-limit = (afstemmen: { $limit }:1)

## Top bar and control strip: messages with values

# $reading is the meter's own hover text; $faces lists the faces a click cycles through.
topbar-smeter-hover =
    { $reading }

    Klik om te wisselen tussen meterweergaven: { $faces }
topbar-step-down = Omlaag { $step }
topbar-step-up = Omhoog { $step }
# $auto is topbar-band-plan or empty.
topbar-repeater-shift-hover = Repeaterverschuiving { $shift } — ontvangen op { $rx }, zenden op { $tx } MHz{ $auto }
topbar-tone-and-burst = Zenden { $tone } onder de stem, en een burst van { $ms } ms op 1750 Hz aan het begin van elke zending
topbar-tone-under-voice = Zenden { $tone } onder de stem
topbar-burst-every-over = Een burst van { $ms } ms op 1750 Hz aan het begin van elke zending
topbar-match-tx = GELIJK AAN ZENDEN ({ $tone })
topbar-decimation-on = Decimatie in de front-end: de ontvanger krijgt het middelste 1/{ $n } van de { $msps } Msps die deze radio streamt — { $span } — en de rest wordt weggegooid voordat er iets van de ontvanger bereikt.
topbar-decimation-off = Decimatie in de front-end, uit: de ontvanger ziet de hele { $span } die deze radio streamt.
topbar-decimation-hint =
    { $what }

    Een smaller bereik betekent fijnere resolutie van de waterval, een rustigere ruisvloer (3 dB per halvering) en minder CPU — ten koste van de band aan weerszijden ervan. De dial stemt nog steeds overal af; de radio verplaatst zijn LO om te volgen.

    Klik om te wisselen: uit / 2 / 4 … { $max }.
topbar-rx-gain-hint = Ontvangstgain van de front-end ({ $stage }). Te veel klipt de ADC van de ontvanger en smeert schijnsignalen over de band; te weinig en hij wordt doof.
topbar-rx-gain-stages =
    {""}

    Deze radio heeft { $n } ontvangstgaintrappen — de rest staat in Instellingen → Apparaat.
topbar-passband-now =
    {""}

    De doorlaatband staat nu op { $dbfs } dBFS.
# $level is topbar-passband-now or empty.
topbar-squelch-hint =
    Dempt de audio onder dit vermogen in de ontvangstdoorlaatband, in dBFS. Links is open.{ $level } Zet hem boven de ruis en onder het signaal.

    Een stroom die al met de eigen AGC van de radio binnenkomt — bijvoorbeeld de 12 kHz-IF van een Icom — zit op deze schaal veel hoger dan het ruwe basisband van een SDR, en daarom bereikt de schaal vol.
topbar-nr-cycle-hint =
    Ruisonderdrukking: { $strength } — { $engine }

    Klik om de sterkte te wisselen: Laag / Gem / Hoog / Uit. De engine staat in de rijen hieronder.
topbar-recording-both = { $audio } en { $iq } opnemen
topbar-recording-audio = Audio opnemen naar { $audio }
topbar-recording-iq = I/Q opnemen naar { $iq }
topbar-auto-record-armed = Auto-opname gereed: een bestand per zending, gesloten na { $secs } s stilte
topbar-rds-hover = RDS: { $ps }. Klik voor de gegevens van het station
topbar-drm-hover = DRM: { $service }. Klik voor de details van de uitzending
topbar-drm-no-codec = DRM: { $summary } — vergrendeld, maar de { $codec }-audio kan hier niet gedecodeerd worden. Klik voor wat ontbreekt
topbar-drm-state = DRM: { $summary } — klik voor de status van de decoder
topbar-hd-hover = HD Radio: { $station }. Klik voor de details van de uitzending
topbar-hd-state = HD Radio: { $summary } — klik voor de status van de decoder
topbar-binaural-preset = { $preset } in elk oor — { $lo } … { $hi } Hz, beide zijbanden
topbar-nr-hover =
    Ruisonderdrukking: { $strength } — { $engine }

    Klik om de engine of de sterkte te wijzigen
topbar-clipping = Clip { $span } — druk opnieuw om hem opnieuw te beginnen
topbar-record-clip = { $span } opnemen en stoppen — een clip om bij een ontvangstrapport te voegen
topbar-starting-clip = een clip van { $span } starten…
topbar-stop-recording-after = De MP3-opname stoppen na { $minutes } minuten
topbar-stops-in = stopt over { $time }
topbar-auto-record-hint = Elke zending naar een eigen bestand opnemen, gesloten na { $secs } seconden stilte
topbar-recording-silence = opnemen · { $secs } s stilte (sluit bij { $hold })
topbar-tone-open = Ontvangen van { $tone }, dat is de toonsquelch — audio open
topbar-tone-closed = Ontvangen van { $heard }, maar de toonsquelch wil { $wanted } — audio blijft dicht
topbar-tone-heard = CTCSS/DCS { $tone } ontvangen
topbar-tone-nothing = Toonsquelch { $tone }: er wordt niets ontvangen dat erbij past
topbar-tone-use = { $tone } GEBRUIKEN
topbar-sockets = Aansluitingen: { $sockets }
topbar-voice-keyer-tx = { $slot } wordt verzonden — klik om de spraakkeyer te openen
topbar-drive-uncal-gen = Deze frequentie, die op geen enkele amateurband ligt, is niet gekalibreerd, dus de instelling bereikt de zender volledig.
topbar-drive-uncal-band = { $band } is niet gekalibreerd, dus de instelling bereikt de zender volledig.
# $trim is signed (+1.5); $diff is its size; $more is "less" or "more".
topbar-drive-cal-gen = Deze frequentie, die op geen enkele amateurband ligt, is { $trim } dB gekalibreerd, dus dezelfde instelling zet hier { $diff } dB { $more ->
        [less] minder
       *[more] meer
    } op de lucht dan op een niet-gekalibreerde band.
topbar-drive-cal-band = { $band } is { $trim } dB gekalibreerd, dus dezelfde instelling zet hier { $diff } dB { $more ->
        [less] minder
       *[more] meer
    } op de lucht dan op een niet-gekalibreerde band.
topbar-drive-hover =
    Hoe hard de zender wordt aangestuurd. Op een radio die sdroxide zelf moduleert, schaalt het de gemoduleerde samples; op een radio met een eigen vermogensregeling is het de fractie van het nominale vermogen die aan de radio gevraagd wordt.

    { $trim }

    Instellingen → Radio → Zendaansturing per band.
topbar-cessb-hover =
    SSB met geregelde envelop: meer gemiddeld vermogen bij dezelfde piek, zonder uitsmeren — drie of vier decibel schijnbare luidheid aan de andere kant. Hoeveel decibel de stem in de processor gedreven wordt; 0 is uit, 6 is een verstandige eerste poging. Het raakt Aansturing niet aan.

    Nu: { $now }
topbar-waterfall-rate = { $rows } rijen per seconde — { $seconds } seconden band
topbar-detail-auto-columns = { $n } kolommen, wat Auto hier zou kiezen
topbar-n-columns = { $n } kolommen
topbar-fft-bins-whole = { $n } bins over de hele stroom — één per kolom, of fijner
topbar-fft-bins-pooled = { $n } bins samengevoegd in de { $cols } kolommen van het panadapter: scherpere signalen, niet meer ervan
topbar-sat-locked = Satelliet — vergrendeld op { $name }
topbar-amp-in-line = { $model } — in de lijn (PTT)
topbar-amp-bypassed = { $model } — omzeild (UIT)
topbar-amp-not-answering = { $model } — antwoordt niet
topbar-filter-edges = { $lo } … { $hi } Hz vanaf de draaggolf
topbar-filter-isb = Ontvangstfilter: { $width } Hz in elk oor ({ $edges }). De twee zijbanden van ISB zijn aparte zendingen, dus de breedte is die van één ervan.
topbar-filter-width = Ontvangstfilter: { $width } Hz breed ({ $edges }).
topbar-filter-hint =
    { $what }

    Klik voor de breedte, de twee randen en de voorinstellingen van deze modus, als getallen — de handvatten van het panadapter plaatsen een doorlaatband op het oog, en hier typ je een exacte waarde.
topbar-band-outside-range =
    { $band } valt buiten wat deze radio ontvangt ({ $ranges } MHz).

    { $why }
# $time is "day" or "night".
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] overdag
       *[night] 's nachts
    }) — verwachting van HAMQSL.com op basis van de zonne-indices, geen meting van je eigen pad.

## Top bar: short words and captions

topbar-squelch-open = open
topbar-off = uit
topbar-recording = opnemen
topbar-sql = sql
topbar-stations = stations
topbar-caption-reaction = reactie
topbar-caption-scroll = scroll
topbar-caption-surface = oppervlak
topbar-caption-flow = stroom
topbar-caption-detail = detail
topbar-floor = vloer

## Digital-mode panel tabs (src/app/panels/mod.rs, pane_label).

pane-aircraft = VLIEGTUIGEN
pane-chart = KAART
pane-chat = CHAT
pane-decodes = DECODES
pane-files = BESTANDEN
pane-heard = GEHOORD
pane-image = BEELD
pane-map = KAART
pane-messages = BERICHTEN
pane-monitor = MONITOR
pane-panel = PANEEL
pane-qso = QSO
pane-reading = LEZEN
pane-receive = ONTVANGEN
pane-saved = OPGESLAGEN
pane-send = VERZENDEN
pane-spots = SPOTS
pane-stations = STATIONS
pane-status = STATUS
pane-terminal = TERMINAL
pane-text = TEKST
pane-traffic = VERKEER
pane-vessels = SCHEPEN
pane-wfall = WFALL

## Digital-mode panels, shared (src/app/panels/mod.rs)

panels-save = {" "}OPSLAAN{" "}
panels-nothing-decoded-save = Niets gedecodeerd om op te slaan
panels-clear-rx = {" "}RX WISSEN{" "}
panels-empty-receive-window-nothing-air = Het ontvangstvenster leegmaken. Niets wat op de lucht is, stopt.
panels-nothing-received-clear = Niets ontvangen om te wissen
panels-save-what-panel-has-decoded = Opslaan wat dit paneel gedecodeerd heeft naar een bestand
panels-prop = PROP
panels-shade-map-where-signals-are = De kaart schaduwen naar waar signalen daadwerkelijk doorkomen, op elke band. Elke ontvangst wordt op het midden van zijn pad geplaatst — het stuk ionosfeer dat het heeft gebogen — niet bij het verre station, dus dit is een kaart van de hemel en niet van waar radioamateurs wonen. Opgebouwd uit paden waaraan dit station heeft deelgenomen, plus — wanneer het Reverse Beacon Network aan staat onder Instellingen → Spots — wat de skimmers van de wereld horen, wat de banden dekt waarop deze radio niet staat.
panels-night = NACHT
panels-shade-where-sun-down-twilight = De schemering en het donker waar de zon onder is, schaduwen, zodat de grijze lijn op de kaart staat. Lage banden worden langer en hoge banden sluiten aan de nachtkant ervan, en de terminator zelf is waar de DX zit.
panels-all-bands = ALLE BANDEN
panels-every-band-once-one-hue = Elke band tegelijk, elk met een eigen tint — algemene condities.
panels-one-band = ÉÉN BAND
panels-one-band-cold-hot-blue = Één band, koud naar heet: blauw, groen, geel, rood.
panels-frequency-you-are-actually-working = De frequentie waarop je werkelijk werkt: de dial plus de toonoffset ernaast. Dit is wat in het logboek komt — de dial alleen zit zoveel te laag.
panels-send-return = VERSTUREN BIJ TERUGKOMST
panels-freq = ⇵ FREQ
panels-nothing-agreed-mode-save-your = Niets afgesproken voor deze modus — sla hieronder je eigen op.
panels-forget-saved-frequency = Deze opgeslagen frequentie vergeten
panels-dial-already-one-your-saved = Deze dial staat al bij je opgeslagen frequenties.
panels-dial-already-agreed-list-mode = Deze dial staat al in de afgesproken lijst voor deze modus.
panels-decode-squelch-raise-stop-decoding = Decode-squelch — omhoog zetten om het decoderen van ruis te stoppen
panels-message-repeats-long-transmit-held = Het bericht herhaalt zolang zenden ingedrukt wordt — een meteoor vangt het deel dat op dat moment voorbijkomt.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}ZENDEN AAN{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}CQ ROEPEN{" "}
panels-nocall = GEEN ROEP

## TX Audio window (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Te weinig spraak gehoord — praat normaal gedurende de hele telling en probeer het opnieuw.
txaudio-tx-audio = TX-audio
txaudio-mode-not-voice-mode-so = Deze modus is geen spraakmodus, dus wat hij verstuurt gaat niet door de keten. Instellingen die hier gemaakt worden, werken in SSB, AM en FM.
txaudio-bypass = OMZEILEN
txaudio-switch-every-stage-out-once = Schakel elke trap in één keer uit, de ruisonderdrukker inbegrepen, zodat de microfoon onbewerkt de lucht in gaat. Klik nogmaals om precies de trappen die aan stonden terug te zetten — de snelste A/B-vergelijking van wat de keten doet.
txaudio-denoise = RUISONDERDRUKKING
txaudio-rnnoise-microphone-ahead-every-other = RNNoise op de microfoon, vóór alle andere trappen, zodat kamerruis weg is voordat de gate of de compressor hem omhoog kan brengen. Voegt ongeveer 10 ms vertraging toe.
txaudio-digital-voice = DIGITALE STEM
txaudio-also-run-freedv-rade-digital = Ook FreeDV / RADE digitale stem door de keten laten lopen. Standaard uit: een vocoder is getraind op natuurlijke spraak en kan een zwaar bewerkte stem slechter decoderen, dus probeer hem op de lucht voordat je erop vertrouwt. TCI-audio en de digitale modi gaan nooit door de keten.
txaudio-test = Test
txaudio-rec = {" "}⏺ OPNEMEN{" "}
txaudio-play = {" "}▶ AFSPELEN{" "}
txaudio-play-test-phrase-through-chain = De testzin door de keten afspelen zoals die nu staat, op je luidsprekers. Pas aan terwijl hij speelt en hoor de verandering. Ontvangstaudio wordt vervangen terwijl hij speelt.
txaudio-recording = opnemen…
txaudio-no-phrase-yet = nog geen zin
txaudio-live = Live
txaudio-live-2 = {" "}🎧 LIVE{" "}
txaudio-hear-your-microphone-through-chain = Hoor je microfoon door de keten terwijl je praat, zonder de zender te sturen. Gebruik een koptelefoon: via luidsprekers geeft het terugkoppeling. Je hoort jezelf een paar tientallen milliseconden te laat, wat praten lastig maakt — stel niveaus in op de meters, en beoordeel het geluid met OPNEMEN / AFSPELEN. Ontvangstaudio wordt vervangen terwijl dit draait; PTT indrukken stopt het.
txaudio-meters = {" "}METERS{" "}
txaudio-run-your-microphone-through-chain = Laat je microfoon door de keten lopen alleen voor de meters: niets op de luidsprekers, dus geen vertraging om tegenin te praten en niets om terug te koppelen. Zo stel je microfoon- en ingangsgain in, de drempel van de gate en hoe hard de compressor werkt. Ontvangstaudio loopt door; PTT indrukken stopt het.
txaudio-monitoring-live-use-headphones-speakers = Live meeluisteren — gebruik een koptelefoon, anders geeft de luidspreker terugkoppeling in de microfoon.
txaudio-voice-modes-only-tci-clients = Alleen spraakmodi. TCI-clients en de digitale modi omzeilen de keten; zolang hij in gebruik is, houd de eigen zend-EQ en processor van de radio vlak.
txaudio-auto-set-stopped-microphone-was = Automatisch instellen gestopt: de microfoon is ingenomen.
txaudio-microphone-may-clipping-before-gets = {" "}De microfoon klipt misschien voordat het signaal hier aankomt — draai de eigen gain van de microfoon omlaag als het schel klinkt.
txaudio-level = Niveau
txaudio-mic-control-same-one-transmit = De regelaar Mic — dezelfde als op de zendbalk, gedeeld door elk profiel. 0 dB in het midden, +6 dB bovenaan.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Gain bovenop Mic voor deze microfoon, opgeslagen bij het profiel. De drempels van de voorinstellingen gaan uit van een stem die rond −10 dBFS piekt op de meter IN; Automatisch instellen zet hem daar. Blijft binnen onder OMZEILEN, en geldt ook voor digitale stem: het is niveau, geen bewerking.
txaudio-input = Ingang
txaudio-auto-set = Automatisch instellen
txaudio-auto-set-cancelled = Automatisch instellen geannuleerd.
txaudio-talk-now-you-would-air = Praat nu, zoals je op de lucht zou doen…
txaudio-db = -∞ dB
txaudio-chain = Keten
txaudio-order-audio-meets-them = In volgorde komt de audio ze tegen
txaudio-switch-stage = Deze trap inschakelen
txaudio-earlier-chain = Eerder in de keten
txaudio-later-chain = Later in de keten
txaudio-output-limiter = De uitgangsbegrenzer
txaudio-output = Uitgang
txaudio-trim-limiter-always-last = Bijstelling en begrenzer — altijd als laatste
txaudio-profiles = Profielen…
txaudio-gate-expander = Gate / expander
txaudio-pushes-room-noise-fan-rumble = Duwt kamerruis, ventilatorgeruis en gesis omlaag in de pauzes. Zet de drempel net boven je ruisvloer en onder je stilste lettergrepen.
txaudio-gate = GATE
txaudio-hard-gate-10-1-down = Harde gate: 10:1, tot −40 dB omlaag
txaudio-expander = EXPANDER
txaudio-gentle-expander-2-1-never = Zachte expander: 2:1, nooit meer dan −15 dB — de kamer wordt nooit dood
txaudio-reset = RESET
txaudio-back-defaults = Terug naar de standaardwaarden
txaudio-threshold = Drempel
txaudio-below-gate-starts-close = Onder deze waarde begint de gate te sluiten.
txaudio-return = Terug
txaudio-hysteresis-once-open-closes-only = Hysterese: eenmaal open sluit hij alleen zo ver onder de drempel, zodat een stem die er net boven of onder zweeft de gate niet laat fladderen.
txaudio-hold = Vasthouden
txaudio-how-long-stays-open-after = Hoe lang hij open blijft nadat de stem wegvalt, zodat woordeinden niet afgeknipt worden.
txaudio-release = Loslaten
txaudio-how-fast-closes-once-hold = Hoe snel hij sluit zodra het vasthouden voorbij is. Verleng het als de gate pompt.
txaudio-floor = Vloer
txaudio-most-ever-turns-pauses-down = Het meeste wat hij de pauzes ooit omlaag draait.
txaudio-ratio = Verhouding
txaudio-how-steeply-closes-below-threshold = Hoe steil hij onder de drempel sluit.
txaudio-look-ahead = Vooruitkijken
txaudio-opens-much-before-word-starts = Opent zoveel vóór een woord begint, zodat de eerste medeklinker niet afgeknipt wordt. Voegt dezelfde vertraging toe aan alles wat verzonden wordt.
txaudio-high-pass-low-shelf-two = Hoogdoorlaat, lage shelf, twee pieken, hoge shelf en laagdoorlaat. Sleep een punt op de curve, of stel het hieronder in. Zet de laagdoorlaat op de breedte van je zendfilter.
txaudio-flat = VLAK
txaudio-every-band-back-0-db = Elke band terug naar 0 dB, beide randen uit
txaudio-freq = Freq
txaudio-gain = Gain
txaudio-q-slope = Q / helling
txaudio-high-pass = Hoogdoorlaat
txaudio-removes-handling-noise-breath-pops = Verwijdert hanteergeruis, adempops en brom onder de hoek.
txaudio-low-shelf = Lage shelf
txaudio-body-mud-cut-little-here = Body, of modder: snijd hier een beetje weg voor een stem die kartonachtig klinkt.
txaudio-peak-1 = Piek 1
txaudio-bump-dip-voice-band-often = Een bult of dip in de stemband — vaak een afsnijding rond 300–500 Hz.
txaudio-peak-2 = Piek 2
txaudio-presence-lift-around-2-3 = Aanwezigheid: een verhoging rond 2–3 kHz is wat op SSB doordringt.
txaudio-high-shelf = Hoge shelf
txaudio-brightness = Helderheid.
txaudio-low-pass = Laagdoorlaat
txaudio-stops-energy-above-transmit-filter = Voorkomt dat energie boven het zendfilter in de modulator verbruikt wordt.
txaudio-t-24-db-oct = 24 dB/okt
txaudio-t-12-db-oct = 12 dB/okt
txaudio-click-steeper-gentler-slope = Klik voor een steilere of zachtere helling
txaudio-de-esser = De-esser
txaudio-turns-down-only-sibilant-band = Draait alleen de sisband omlaag wanneer "s" en "sj" de drempel passeren, zodat de rest van de stem zijn niveau houdt. Vind de frequentie door hem af te tasten terwijl je "sister silver cellophane" zegt en te parkeren waar het gesis het luidst is.
txaudio-frequency = Frequentie
txaudio-centre-sibilant-band = Midden van de sisband.
txaudio-width-band-higher-narrower = Breedte van de band — hoger is smaller.
txaudio-band-level-where-reduction-starts = Bandniveau waar de reductie begint. Te laag en hij werkt de hele tijd, wat de stem dof maakt.
txaudio-amount = Mate
txaudio-most-band-ever-turned-down = Het meeste wat de band ooit omlaag gedraaid wordt. Begin rond −6 dB.
txaudio-attack = Aanval
txaudio-how-fast-reacts = Hoe snel hij reageert.
txaudio-how-fast-lets-go = Hoe snel hij loslaat.
txaudio-compressor = Compressor
txaudio-brings-quiet-syllables-up-holds = Trekt zachte lettergrepen omhoog en houdt harde tegen. Mik op 3–6 dB reductie op de pieken; meer dan 10 dB is duidelijke compressie, beter geschikt voor wedstrijden.
txaudio-level-above-which-starts-compress = Niveau waarboven hij begint te comprimeren.
txaudio-how-hard-compresses-above-threshold = Hoe hard hij comprimeert boven de drempel.
txaudio-how-fast-reacts-peak-10 = Hoe snel hij op een piek reageert. 10–25 ms houdt medeklinkers scherp.
txaudio-how-fast-recovers = Hoe snel hij herstelt.
txaudio-knee = Knik
txaudio-soft-knee-width-0-hard = Breedte van de zachte knik; 0 is een harde knik.
txaudio-makeup = Compensatie
txaudio-gain-after-compression-bring-level = Gain na de compressie, om het niveau weer omhoog te brengen.
txaudio-drive = Aansturing
txaudio-gain-ahead-compressor-so-more = Gain vóór de compressor, zodat meer van de stem de drempel passeert — verhoogt het gemiddelde vermogen. Gebruik het samen met de uitgangsbegrenzer.
txaudio-phase = Fase
txaudio-all-pass-phase-rotator-ahead = Allpass-faseroteerder vóór de detector. Egaliseert de scheve pieken die een stem heeft, zodat hetzelfde piekniveau meer gemiddeld vermogen draagt. 4 is een gebruikelijke omroepinstelling; 0 is uit.
txaudio-last-stage-trim-look-ahead = De laatste trap: een bijstelling, en een begrenzer met vooruitkijken die elke piek onder het plafond houdt zonder de uitsmering die harde clipping geeft. Standaard uit, zodat het inschakelen van de keten een aansturingsniveau dat je al hebt ingesteld niet verandert.
txaudio-limiter = Begrenzer
txaudio-on = Aan
txaudio-ceiling = Plafond
txaudio-no-peak-leaves-chain-above = Geen piek verlaat de keten boven dit niveau.
txaudio-trim = Bijstelling
txaudio-output-gain-ahead-limiter = Uitgangsgain, vóór de begrenzer.
txaudio-off = {"  "}uit
txaudio-meters-move-while-you-transmit = De meters bewegen terwijl je een stem over de lucht zendt, de testzin afspeelt, of LIVE of METERS draait.
txaudio-profiles-2 = Profielen
txaudio-profile-whole-chain-every-stage = Een profiel is de hele keten: de instellingen van elke trap, welke aan staan, hun volgorde, en de ingangsgain voor je microfoon. Het laden van een profiel vervangt de keten; de regelaar Mic en de aansturing blijven zoals ze zijn.
txaudio-name = Naam
txaudio-save = Opslaan
txaudio-store-chain-now-under-name = De keten zoals hij nu is onder deze naam opslaan, en een profiel met dezelfde naam vervangen
txaudio-import = Importeren…
txaudio-read-profile-json-file = Een profiel lezen uit een .json-bestand
txaudio-tx-audio-profile = Profiel TX-audio
txaudio-no-profiles-yet-save-stores = Nog geen profielen. Opslaan bewaart de keten die je nu hebt ingesteld.
txaudio-load = Laden
txaudio-export = Exporteren…
txaudio-delete = Verwijderen
txaudio-starting-points = Startpunten
txaudio-set-your-level-auto-set = Stel je niveau eerst in met Automatisch instellen — de drempels van de voorinstellingen gaan daarvan uit — en stem daarna vanaf hier af in plaats van te zenden zoals geladen: neem een zin op en luister na elke wijziging. Een voorinstelling behoudt je ingangsgain.
txaudio-profile-loaded = Profiel geladen.

## Solar-system 3D view overlay (src/solar3d/overlay.rs)

solar-prop = PROP
solar-where-signals-are-actually-getting =
    Waar signalen werkelijk doorkomen, band voor band. Elke ontvangst waaraan dit station deelneemt — WSPR, FT8/FT4, JS8 en het logboek — wordt op het midden van zijn pad geplaatst, het stuk ionosfeer dat het heeft gebogen, en niet bij het verre station. Dus dit is een kaart van de hemel, niet van waar radioamateurs wonen.

    Het kan alleen paden tonen waar dit station een van de uiteinden van was. Oceanen lichten op omdat de middens van lange paden daar vallen; Antarctica blijft donker omdat niemand er vandaan zendt. Standaard uit — het schildert de hele planeet.
solar-orbits = BANEN
solar-orbital-paths = Baanpaden
solar-clouds = WOLKEN
solar-cloud-cover-noaa-s-global = Wolkenbedekking uit de wereldwijde geostationaire mozaïek van NOAA, getekend als een diepte van lucht in plaats van een beeld op het oppervlak geplakt: infrarood zegt hoe koud elke wolkentop is, en daaruit volgt hoe hoog hij staat. Onweersbuien flitsen binnen de torens die ze opbouwen.
solar-planets = PLANETEN
solar-other-seven-planets-eighteen-their = De andere zeven planeten, achttien van hun manen, en de ringen van Saturnus en Uranus. De posities van de planeten komen uit de Keplerse elementenset van JPL; de manen zijn cirkelbanen die op JPL Horizons zijn gepast.
solar-coronal-mass-ejection-trajectory-cones = Traject-kegels van coronale massa-ejecties
solar-sun-obs = ZON-WN
solar-solar-observations-sun-s-disk = Zonwaarnemingen op de schijf van de zon: zonnevlekgebieden, en waar de vlammen vandaan kwamen
solar-labels = LABELS
solar-body-region-labels = Labels van hemellichamen en gebieden
solar-small-bodies = KLEINE LICHAMEN
solar-names-asteroids-comets-they-are = Namen op de asteroïden en kometen. Ze worden hoe dan ook getekend — dit bepaalt alleen of vijfendertig aanduidingen erbij getekend worden. Wat het ook staat, het lichaam waarop de camera staat en alles wat het zoekvak heeft gevonden, blijven benoemd.
solar-decoded-ft8-ft4-stations-path = Gedecodeerde FT8/FT4-stations en het pad naar het station waarmee gewerkt wordt
solar-sats = SATS
solar-amateur-radio-satellites-their-orbits = Amateurradiosatellieten, hun banen en hoogte vanaf je QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = De aurorale ovaal uit het OVATION-model van NOAA, getekend als emissieschillen op hun echte hoogten, met de rand naar de evenaar van het 10%-contour aangegeven op het oppervlak
solar-awards = PRIJZEN
solar-award-coverage-every-dxcc-entity = Prijsdekking: elke DXCC-entiteit ingekleurd naar wat je logboek nog mist. Oranje brandt waar je de entiteit nooit gewerkt hebt, amber waar je hem wel gewerkt hebt maar niet bevestigd is, dimgroen waar een QSL is teruggekomen. Volgt het bandfilter in het venster PRIJZEN.
solar-view = WEERGAVE
solar-which-body-camera-orbits-animated = Om welk hemellichaam de camera draait, en de geanimeerde rondleiding
solar-layers = LAGEN
solar-what-drawn-orbits-clouds-cmes = Wat getekend wordt: banen, wolken, CME's, labels, QSO's, aurora
solar-which-sdo-channel-wraps-sun = Welk SDO-kanaal de zon omhult, en hoe oud het is
solar-weather = WEER
solar-aurora-propagation-numbers-which-bands = De aurora, de propagatiegetallen en welke banden open zijn
solar-space-weather = RUIMTEWEER
solar-nothing-measured-yet = nog niets gemeten
solar-scale = SCHAAL
solar-body-moon-orbit-exaggeration = Overdrijving van hemellichaam en maanbaan
solar-time = TIJD
solar-scrub-whole-scene-forward-back = De hele scène vooruit en achteruit doorspoelen
solar-activity = ACTIVITEIT
solar-replay-last-hour-decodes-globe = De laatste uur aan decodeerresultaten op de bol opnieuw afspelen
solar-what-propagation-heat-map-showing = Wat de propagatie-warmtekaart toont
solar-propagation = PROPAGATIE
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Vlieg een spline door een reeks ingekaderde gezichtspunten. Elke muisactie breekt hem af.
solar-moons-ride-circular-orbits-fitted = De manen volgen cirkelbanen die op JPL Horizons zijn gepast: binnen een graad of twee van waar ze werkelijk zijn, en tot zes voor Miranda, wiens baanvlak te snel zwaait voor een cirkel om te volgen.
solar-fetch-everything-again-now = Alles nu opnieuw ophalen
solar-all-sats = ALLE SATS
solar-show-every-satellite-element-set = Elke satelliet in de elementenset tonen, niet alleen de populaire. De baanringen blijven bij de gecureerde paar — negentig tegelijk is onleesbaar.
solar-starting = starten…
solar-body = hemellichaam{" "}
solar-moon-orbit = maanbaan{" "}
solar-stretch-earth-moon-distance-so = De afstand Aarde→Maan uitrekken zodat het paar apart te zien is
solar-sun = zon{" "}
solar-sun-radius-exaggeration-leave-1 = Overdrijving van de straal van de zon. Laat op 1× om de geometrie van de CME's leesbaar te houden — een opgezwollen zon verzwelgt de basis van elke kegel.
solar-show = TONEN
solar-paint-globe-what-getting-through = De bol inkleuren naar wat er doorkomt
solar-paints-globe-what-getting-through = Kleurt de bol naar wat er doorkomt, op elke band. Elke ontvangst wordt op het midden van zijn pad geplaatst — waar de ionosfeer het werk deed — en niet bij het verre station. Opgebouwd uit wat dit station hoort, plus de skimmers van de wereld als het Reverse Beacon Network aan staat.
solar-display = WEERGAVE
solar-all-bands = ALLE BANDEN
solar-every-band-once-one-hue = Elke band tegelijk, elk met een eigen tint, en waar twee overlappen mengen ze. De weergave voor 'hoe zijn de condities' in plaats van 'is twintig open'.
solar-one-band = ÉÉN BAND
solar-one-band-cold-hot-blue = Één band, koud naar heet: blauw, groen, geel, rood.
solar-band = BAND
solar-nothing-heard-yet = nog niets gehoord
solar-bands = BANDEN
solar-sources = BRONNEN
solar-contacts-logbook-path-was-open = Contacten in het logboek. Een pad dat open was, zonder signaalrapport dat de naam waard is — een RST is geen SNR, dus deze tellen mee voor hoe druk een cel is en nooit voor de marge ervan.
solar-stations-reported-hearing-one-downloaded = Stations die melden dit station gehoord te hebben, gedownload van WSPRnet. De enige terugkoppeling die een zendend baken ooit krijgt.
solar-reverse-beacon-network-skimmers-what = Reverse Beacon Network-skimmers: wat iedereen anders hoort, op elke band, ook de banden waarop deze radio niet staat. Grover dan de rest — een RBN-regel draagt geen locatoren, dus beide uiteinden worden op het centrum van hun land geplaatst, wat dichtbij is voor een klein land en ver naast voor een groot. Aanzetten onder Instellingen → Spots.
solar-decodes-station-made-each-measured = Decoderingen die dit station gemaakt heeft. Elk wordt gemeten tegen de drempel van zijn eigen modus, zodat een WSPR-melding en een FT4-melding vergelijkbaar zijn.
solar-memory = GEHEUGEN
solar-half-life = halveringstijd{" "}
solar-how-long-reception-takes-count = Hoe lang een ontvangst duurt om half zo veel mee te tellen. Het eigen geheugen van de ionosfeer is kort — een opening van twee uur oud hoeft niet te twisten met een van twee minuten geleden.
solar-live = LIVE
solar-follow-band-happens = De band volgen terwijl het gebeurt
solar-replay = ⏸ HERHALEN
solar-replay-2 = ▶ HERHALEN
solar-replay-last-hour-decodes-over = De laatste uur aan decodeerresultaten herhalen, steeds opnieuw
solar-min-ago = {" "}min geleden
solar-where-last-hour-globe-showing = Waar in het laatste uur de bol op toont
solar-trail = spoor{" "}
solar-how-long-decode-s-arc = Hoe lang de boog van een decodering op de bol achter de kop blijft staan
solar-speed = snelheid{" "}
solar-how-much-faster-than-real = Hoeveel sneller dan de werkelijke tijd de herhaling loopt
solar-qso-layer-off = QSO-laag uit
solar-no-decodes-yet = nog geen decoderingen
solar-qth-not-set-enter-your = QTH niet ingesteld — vul je locatievak in bij Instellingen
solar-mode = MODUS
solar-path = PAD
solar-sent = VERZONDEN
solar-rcvd = ONTV
solar-grid = GRID
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geostationair — altijd boven je horizon.
solar-no-passes-next-48-hours = Geen passages in de komende 48 uur vanaf je QTH.
solar-start = START
solar-max-el = MAX EL
solar-aos-los-are-azimuths-horizon = AOS/LOS zijn azimuts aan de horizon. Tijden zijn UTC.
solar-locked-unlock = {" "}● VERGRENDELD — ONTGRENDELEN{" "}
solar-release-satellite-lock = De satellietvergrendeling loslaten
solar-lock = {" "}VERGRENDELEN{" "}
solar-track-satellite-main-window-doppler = Deze satelliet volgen in het hoofdvenster: Doppler-gecorrigeerde RX en TX, de uplink via de transponder, en de sturing van de rotor
solar-no-frequencies-file-one-add = Geen frequenties in het bestand voor deze — voeg ze toe in Instellingen ▸ TLE.
solar-frequencies-yours = FREQUENTIES · JOUW
solar-frequencies = FREQUENTIES
solar-link = VERBINDING
solar-downlink-mhz = DOWNLINK (MHz)
solar-uplink-mhz = UPLINK (MHz)
solar-doppler-shifts-these-few-khz = Doppler verschuift deze met een paar kHz over een LEO-passage.
solar-sim = -- SIM
solar-satellite-planet-comet = satelliet, planeet of komeet
solar-planet-asteroid-comet = planeet, asteroïde of komeet
solar-clear-search = Het zoeken wissen
solar-no-match = geen overeenkomst
solar-no-sounder = geen sonde
solar-heard = GEHOORD ≥
solar-kp = Kp / A
solar-x-ray = Röntgen
solar-observed-above-sounder-better-than = waargenomen boven de sonde — beter dan gemodelleerd
solar-power-n-s = vermogen N/S
solar-edge-n-s = rand N/S
solar-kp-peak-24-h = Kp-piek 24 u
solar-observed-forecast = waargenomen | verwachting
solar-bands-open = BANDEN OPEN
solar-dxcc-coverage = DXCC-DEKKING
solar-ir-vis = IR+VIS
solar-ir-only = alleen IR
solar-no-deep-convection = geen diepe convectie
solar-t-1-storm = 1 storm
solar-glancing = {" "}· schuin
solar-direction-estimated = {" "}· richting geschat

## Digital-mode setup dialogs (src/app/panels/setup.rs)

setup-aprs-setup = APRS-instelling
setup-packet-setup = Pakketinstelling
setup-ft8-ft4-ft2-setup = FT8 / FT4 / FT2-instelling
setup-my-callsign = Mijn roepletters
setup-my-grid = Mijn locatievak
setup-station-call = Stationsroep
setup-ssid-oe3jjs-10-packet-station = Met een SSID — OE3JJS-10. Een pakketstation heeft conventioneel een andere SSID dan de operator, en er wordt niets verzonden tot dit is ingesteld.
setup-speed = Snelheid
setup-t-9600-needs-radio-s-data = 9600 vereist de datapoort van de radio. Een microfoon- en luidsprekerpad vernietigt die aan beide kanten — dat is de radio, niet sdroxide.
setup-tx-delay = Zendvertraging
setup-flags-sent-before-frame-so = Vlaggen die vóór een frame verstuurd worden, zodat het verre eind ons kan horen en zijn klok kan vergrendelen. Op een CAT-radio besteedt sdroxide alleen al 165–240 ms om op de lucht te komen, en de eigen zendgereedtijd van de radio komt daar nog bovenop.
setup-packet-length = Pakketlengte
setup-bytes = {" "}bytes
setup-most-single-frame-carries-shorter = Het meeste wat een enkel frame draagt. Kortere frames overleven een marginaal pad — alleen het geraakte frame wordt opnieuw verstuurd — en kosten meer overhead. 128 op HF, 256 waar het pad goed is.
setup-frames-sent-before-waiting-acknowledgement = Frames die verstuurd worden voordat op een bevestiging gewacht wordt. Meer vult een goed pad; op een slecht pad is het meer om opnieuw te versturen.
setup-extended = Uitgebreid
setup-ask-mod-128 = Vragen om mod-128
setup-extended-sequence-numbers-window-bigger = Uitgebreide volgnummers, voor een venster groter dan zeven. Veel knooppunten weigeren het verzoek met een DM, wat precies lijkt op een station dat niet met je wil praten — laat het dus uit tenzij je weet dat het verre eind het wil.
setup-answer-calls = Oproepen beantwoorden
setup-accept-connections = Verbindingen accepteren
setup-off-winlink-client-which-dials = Uit voor een Winlink-client, die uitbelt en geen reden heeft om te antwoorden. Aan om als peer bereikbaar te zijn: een oproep komt aan in het terminalpaneel en je praat met wie hem maakte. Een station waarvan de verbinding al bezig is met een Winlink-sessie weigert oproepen tot die klaar is.
setup-connect-text = Verbindingstekst
setup-what-caller-greeted = waarmee een beller begroet wordt
setup-sent-station-connects-us-once = Verstuurd naar een station dat met ons verbindt, zodra de verbinding staat. Leeg verstuurt niets — wat er kapot uitziet voor wie belde.
setup-default-via = Standaard via
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = Het digipeaterpad waarmee de terminal begint. Het pad naar je lokale knooppunt is elke keer hetzelfde, en het opnieuw intypen is hoe een sprong wegvalt.
setup-beacon = Baken
setup-t-0-disables-timer = 0 schakelt de timer uit
setup-beacon-text = baken-tekst
setup-kiss-server = KISS-server
setup-serve = Aanbieden
setup-offer-modem-kiss-tnc-socket = Deze modem als KISS-TNC aanbieden op een socket, zodat Pat, een APRS-client of de Linux AX.25-stack de radio kunnen gebruiken.
setup-aprs-call = APRS-roep
setup-callsign-station-beacons-under-ssid = De roepletters waaronder dit station bakent, met zijn SSID — OE3JJS-9 voor een auto, -10 voor een I-gate, -5 voor een telefoon. Laat het leeg om de roepletters hierboven te gebruiken.
setup-no-callsign-station-will-not = Geen roepletters: dit station zendt helemaal niets — geen baken, geen bericht, geen bevestiging.
setup-symbol = Symbool
setup-two-characters-protocol-carries-them = De twee tekens zoals het protocol ze draagt: een tabel (`/` of `\`) en een symboolcode. Een cijfer of letter in de eerste positie is een overlay, die bovenop het icoon van de alternatieve tabel getekend wordt.
setup-path = Pad
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Hoe ver je vraagt herhaald te worden. Dit is de belangrijkste instelling op het kanaal: elke sprong vermenigvuldigt de zendingen die het hele netwerk op één gedeelde frequentie maakt.
setup-position = Positie
setup-my-grid-2 = Uit mijn locatievak
setup-beacon-centre-locator-above-reported = Bakent het midden van de locator hierboven, gemeld met de onnauwkeurigheid die een locator werkelijk heeft — een locator van zes tekens is een paar kilometer breed, en dat zeggen is eerlijk. Schakel het uit om in plaats daarvan exacte coördinaten te geven.
setup-latitude = Breedtegraad
setup-longitude = Lengtegraad
setup-locator = uit de locator
setup-not-coordinate = geen coördinaat
setup-comment = Opmerking
setup-sent-every-beacon = bij elk baken verstuurd
setup-t-0-default-never-beacons-timer = 0 — de standaard — bakent nooit op een timer. Dertig minuten is de conventie voor een vast station; een bewegend station bakent vaker, maar elk baken is andermans kanaaltijd.
setup-compressed = Gecomprimeerd
setup-compressed-position-format-third-air = Het gecomprimeerde positieformaat: een derde van de zendtijd en nauwkeuriger. Elke ontvanger sinds de jaren negentig leest het.
setup-first-goes-out-one-interval = De eerste gaat één interval vanaf nu uit, niet meteen.
setup-messages = Berichten
setup-acknowledge = Bevestigen
setup-answer-messages-addressed-you-acknowledgement = Beantwoord berichten die aan jou gericht zijn. Een bevestiging is een zending die dit station doet zonder dat erom gevraagd is, dus een opstelling die alleen ontvangt moet hem uitzetten — en dan houdt het baken ook op te beweren dat het bereikbaar is.
setup-flags-sent-ahead-every-frame = Vlaggen die voor elk frame verstuurd worden, zodat het modem aan het andere eind de draaggolf hoort en zijn klok vergrendelt voordat de gegevens beginnen. Het moet langer duren dan alles tussen het indrukken van zenden en het uitstralen: sdroxide alleen al besteedt er 165–240 ms aan op een CAT-radio, en een radio die audio over een netwerk aanneemt, buffert er nog meer bovenop. Te weinig en het verre eind vergrendelt nooit — de zending is er en niets decodeert hem.
setup-shared-packet-mode = gedeeld met de pakketmodus
setup-keep-stations = Stations bewaren
setup-how-long-station-stays-map = Hoe lang een station op de kaart blijft nadat het voor het laatst gehoord is. Het is ook waartegen het vervagen van de kaart gemeten wordt, dus een kort venster toont alleen wat live is.
setup-auto-reply = Automatisch antwoord
setup-answer-snr-grid-status = Antwoord op SNR? / GRID? / STATUS?
setup-answer-direct-question-addressed-you = Beantwoord een directe vraag die aan jou of aan @ALLCALL gericht is, met het antwoord in plaats van een bevestiging. Beantwoordt nooit het verkeer van een ander station, en beantwoordt nooit zichzelf.
setup-heartbeat = Hartslag
setup-off = Uit
setup-turbo-does-not-beacon-local = Turbo bakent niet — het is de lokale en VHF-snelheid.
setup-sends-your-callsign-grid-so = Verstuurt je roepletters en locatievak zodat anderen weten dat je ontvangbaar bent. De eerste gaat één interval vanaf nu uit, niet meteen.
setup-beacon-frequency = Bakenfrequentie
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Verplaatst elk baken naar een vrije plek in het hartslag-subbereik, zoals JS8Call doet: daar kijken stations die op bakens letten, en het houdt een onbewaakte zender uit andermans QSO. De plek wordt gekozen op het moment dat het baken werkelijk uitgaat, vrij van alles wat gedecodeerd wordt.
setup-working-freq = Werkfrequentie
setup-beacon-where-you-are-working = Bakent waar je werkt. Tegen de bandconventie in, maar het houdt alles wat je verstuurt op één plek.
setup-heartbeat-reply = Antwoord op hartslag
setup-answer-heartbeats-signal-report = Beantwoord hartslagen met een signaalrapport
setup-tell-station-beaconed-how-well = Vertel een station dat bakende hoe goed je het ontving. Standaard uit: een drukke band draagt elk tijdslot een hartslag, en op al die antwoorden te reageren zou precies de band overspoelen die hartslagen stil moeten houden. Beperkt tot één antwoord per station per 15 minuten, en nooit terwijl er nog een bericht binnenkomt of terwijl er iets in de wachtrij staat om te versturen.
setup-status-message = Statusbericht
setup-tx-period = Zendperiode
setup-even = Even
setup-odd = Oneven
setup-auto-sequence = Automatische volgorde
setup-decode-depth = Decodeerdiepte
setup-fast = Snel
setup-normal = Normaal
setup-deep = Diep
setup-how-hard-ft8-decoder-works =
    Hoe hard de FT8-decoder werkt voor zwakke signalen.

    Snel — één doorgang, geen signaalaftrekking: het snelst, de minste decodeerresultaten.
    Normaal — vlakke meervoudige aftrekking: iets sneller dan Diep, iets minder grondig.
    Diep — de meervoudige doorgang met controlepunten: de meeste decodeerresultaten, en ~1,2 s op een drukke slot.
setup-auto-tx-frequency = Automatische zendfrequentie
setup-choose-transmit-frequency-automatically-quietest = Kies de zendfrequentie automatisch: de rustigste plek in de periode waarin je zendt, in plaats van de frequentie van het station dat je beantwoordt. Uit houdt hem NIET vast — hij antwoordt op de frequentie van het station dat gebeld wordt. Gebruik Zendfrequentie vasthouden om hem vast te zetten.
setup-hold-tx-frequency = Zendfrequentie vasthouden
setup-pin-transmit-tone-where-nothing = Zet de zendtoon vast waar hij staat. Niets verplaatst hem: niet het beantwoorden van een station, niet de oproepwachtrij, niet CQ roepen, niet een klik op een decodering of de waterval. Heeft voorrang op Automatische zendfrequentie. Van band wisselen is de enige uitzondering: de offset die je laatst op de nieuwe band instelde, komt er weer mee terug. Voor als je vergunning smaller is dan het bandplan — op een Brits 60 m-dial van 5357 kHz eindigt de toewijzing bij 5358.0, dus de toon moet onder 1000 Hz blijven.
setup-tx-watchdog = Zend-watchdog
setup-stop-transmitting-after-long-no = Stop met zenden na zo lang zonder antwoord en zonder actie van jou. 0 schakelt het uit.
setup-give-up-after = Opgeven na
setup-calls = {" "}oproepen
setup-unanswered-calls-one-station-before = Onbeantwoorde oproepen naar één station voordat er verder gegaan wordt. CQ roepen is hiervan uitgezonderd. 0 schakelt het uit.
setup-contest = Wedstrijd
setup-everyday-exchange-grid-report-rr73 = De dagelijkse uitwisseling: locatievak, rapport, RR73.
setup-european-vhf-contests-send-signal = Europese VHF-wedstrijden: stuur een signaalrapport, een serienummer en je locator van zes tekens, in de berichtopmaak die WSJT-X EU VHF Contest noemt. CQ gaat uit als CQ TEST.
setup-serial = Serienummer
setup-serial-number-next-exchange-will = Het serienummer dat de volgende uitwisseling draagt. Het loopt zelf op zodra elk contact gelogd wordt en wordt tussen sessies onthouden — stel het alleen met de hand in om een wedstrijd te beginnen, of om verder te gaan waar een papieren logboek was gebleven.
setup-needs-6-character-grid = ⚠ heeft een locatievak van 6 tekens nodig
setup-eu-vhf-contest-messages-carry = Berichten van de EU VHF Contest dragen een locator van zes tekens (JN88DD, niet JN88). Zolang Mijn locatievak er geen heeft, wordt de gewone uitwisseling verstuurd.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Gewone FT8-werking.
setup-calling-dxpedition-running-fox-mode = Een DXpedition aanroepen die in Fox-modus draait: roep vanaf boven 1000 Hz, ga omlaag naar de Fox zodra hij antwoordt, en log op zijn RR73 zonder 73 te sturen.
setup-run-pile-up-several-signals = Draai de pile-up: meerdere signalen tegelijk, een wachtrij van roepers, het sterkste en zeldzaamste eerst afgewerkt. CQ ROEPEN start hem, QSO STOPPEN zet hem stil.
setup-fox-signals = Fox-signalen
setup-once = {" "}tegelijk
setup-simultaneous-transmissions-spaced-60-hz = Gelijktijdige zendingen, 60 Hz uit elkaar. Ze delen het vermogen van de zender, dus meer signalen betekent dat elk zwakker is.
setup-tx-audio = TX-audio
setup-how-loud-mode-s-burst =
    Hoe hard de burst van deze modus aan een radio wordt gegeven die hem zelf moduleert — en bij FM is dat de deviatie, omdat een FM-zender audioniveau omzet in frequentie-uitslag en geen ALC heeft om het op te vangen. 1200 baud pakket wil ongeveer 3 kHz waar spraak 5 wil, dus volle schaal in een dataingang die voor spraak is ingesteld, overdrijft de deviatie — wat voor een luisteraar volkomen normaal klinkt en voor niemand decodeert. Draai hem omlaag tot andere stations je melden, of stel het niveau op de radio in.

    Dit niveau hoort bij deze modus alleen. Een modus die je nooit hebt ingesteld, begint bij de FM-waarde, en de zijbandmodi beginnen bij hun eigen — dus een deviatie die voor 1200 baud is ingesteld, komt nooit op FT8 terecht.
setup-how-loud-mode-s-over =
    Hoe hard de zending van deze modus aan een radio wordt gegeven die hem zelf moduleert — een CAT-radio via zijn geluidskaart, een FLEX, een Icom via zijn netwerkpoort. Op zijbanden is dit aansturing van de modulator: draai hem omlaag tot de ALC van de radio nauwelijks beweegt en stel het vermogen op de radio in, omdat ALC die meeloopt op een modus met constante envelop is wat uitsmeert. Aansturing bereikt hier het vermogensregister van de radio, niet zijn audio, dus dit is het niveau.

    Dit niveau hoort bij deze modus alleen: FT8, RTTY, PSK en MCW houden elk hun eigen, omdat ze een modulator niet op dezelfde manier belasten. Dezelfde regelaar staat op de zendstrook, waar je hem kunt bereiken terwijl je zendt.
setup-deviation-fm = deviatie, op FM
setup-drive-ssb = aansturing, op SSB
setup-grid = Locatievak
setup-report = Rapport
setup-r-report = R+Rapport

## AMP window (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , schaal 10 W, herschaald
amp-t-10-w-scale = , schaal 10 W
amp-t-2-w-scale-rescaled = , schaal 2 W, herschaald
amp-t-2-w-scale = , schaal 2 W
amp-t-100-w-scale-rescaled = , schaal 100 W, herschaald
amp-t-100-w-scale = , schaal 100 W
amp-t-20-w-scale-rescaled = , schaal 20 W, herschaald
amp-t-20-w-scale = , schaal 20 W
amp-rescaled = , herschaald
amp-and = {" "}EN{" "}
amp-not-while-transmitting-switching-relay = Niet terwijl er gezonden wordt: een relais schakelen met RF erop kan het beschadigen. Zet eerst de zender uit.
amp-no-amplifier-set-up-choose = Er is geen versterker ingesteld. Kies de poort ervan onder Instellingen → Versterker.
amp-settings = INSTELLINGEN
amp-dock = VASTZETTEN
amp-put-amp-window-back-inside = Het venster AMP terugzetten in het hoofdvenster
amp-pop-out = LOSMAKEN
amp-give-amp-window-window-own = Het venster AMP een eigen venster geven, om naar een ander scherm te verplaatsen
amp-rig = , van de radio
amp-rig-flagged-reading = , de radio markeerde zijn meting
amp-drv = , van DRV
amp-rig-s-swr = , van de SWR van de radio
amp-ptt-keys-amplifier-radio-off = PTT stuurt de versterker mee met de radio; UIT laat hem in bypass. Klik om te schakelen.
amp-switch-antenna-jack = Antenne-aansluiting schakelen
amp-band-down = Band omlaag
amp-band-up = Band omhoog
amp-tuner = TUNER
amp-tuned = Afgestemd
amp-tune = AFSTEMMEN
amp-start-tune-then-key-steady = Start een afstemming — stuur daarna een stabiele draaggolf (CW, FM of RTTY). Druk opnieuw om te stoppen.
amp-put-tuner-line-bypass = De tuner in de lijn zetten, of hem omzeilen
amp-tuning = Afstemmen…
amp-bottom = ONDER
amp-pick-any-reading-either-needle = Kies elke meting voor een van beide naalden. De bovenste naald is wit, de onderste blauw.
amp-forward-power-measured-radio-amplifier = Voorwaarts vermogen, gemeten door de radio: de versterker meldt er geen in UIT
amp-forward-power-taken-drv-amplifier = Voorwaarts vermogen, afgenomen van DRV: de versterker meldt er geen in UIT, en in bypass gaat het vermogen van de radio rechtstreeks door
amp-forward-power = Voorwaarts vermogen
amp-reflected-power-measured-radio-amplifier = Gereflecteerd vermogen, gemeten door de radio: de versterker meldt er geen in UIT
amp-reflected-power-worked-out-fwd = Gereflecteerd vermogen, berekend uit FWD en de SWR: de versterker meldt er geen in UIT
amp-reflected-power = Gereflecteerd vermogen
amp-standing-wave-ratio-red-past = Staandegolfverhouding: rood bij of boven de afstemgrens (Instellingen → Algemeen). Controleer de antenne
amp-standing-wave-ratio-measured-radio = Staandegolfverhouding, gemeten door de radio: de versterker meldt er geen in UIT
amp-standing-wave-ratio = Staandegolfverhouding
amp-drive-radio = Aansturing van de radio
amp-drain-supply = Drainvoeding
amp-drain-current = Drainstroom
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Temperatuur van de warmtespreider: geel vanaf 49 °C (120 °F), rood boven 65 °C (149 °F), uit de lijn boven 70 °C (158 °F). Klik voor °C/°F.
amp-c = °C
amp-f = °F
amp-performance = PRESTATIES
amp-numbers-hold-peaks-bars-are = getallen houden pieken vast · balken zijn live
amp-input-power = INGANGSVERMOGEN
amp-supply-volts-drain-amps = Voedingsspanning × draanstroom in ampère
amp-efficiency = RENDEMENT
amp-output-power-input-power = Uitgangsvermogen ÷ ingangsvermogen
amp-gain = GAIN
amp-output-power-over-drive-decibels = Uitgangsvermogen ten opzichte van de aansturing, in decibel
amp-off-line-over-temperature = UIT DE LIJN · te heet
amp-connected-transmitting = VERBONDEN · ZENDT
amp-connected = VERBONDEN
amp-connecting = Verbinden…
amp-atu-bypassed = ATU OMZEILD
amp-no-inductance-needed = geen spoel nodig
amp-no-capacitance-needed = geen condensator nodig
amp-c-out-antenna-side = C UIT · antennezijde
amp-c-transmitter-side = C IN · zenderzijde

## SSTV panel (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Ontvangen brokstukken (verlicht) en nog ontbrekende (donker)
sstv-sstv = SSTV
sstv-auto = Auto
sstv-signal = Signaal
sstv-station-sending-mode-sdroxide-does = Een station zendt in een modus die sdroxide niet heeft. Het signaal en de afstemming zijn in orde — er is alleen geen decoder voor deze. Er valt aan jouw kant niets te repareren.
sstv-listening = luisteren…
sstv-restart-rx = Ontvangst opnieuw starten
sstv-abandon-picture-being-received-listen = Het beeld dat ontvangen wordt laten vallen en luisteren naar de volgende kop. Voor een zending die in de verkeerde modus begon te decoderen — de ontvanger zit er anders aan vast tot die modus afloopt.
sstv-callsign-last-station-sent-fsk = De roepletters die het laatste station na zijn beeld als FSK-ID verstuurde.
sstv-tx-slant = Zendscheefstand
sstv-transmit-clock-trim-ppm-remove = Kloktrim van de zender (ppm) om de scheefstand bij de decoder aan het andere eind weg te halen
sstv-reset-0-ppm = Terugzetten naar 0 ppm
sstv-fsk-id = FSK-ID
sstv-send-your-callsign-tones-after = Verstuur je roepletters in tonen na elk beeld — de identificatie die SSTV-repeaters en andere programma's lezen. Voegt ongeveer 2,5 seconden toe, en verstuurt helemaal niets tot je roepletters hebt ingesteld.
sstv-tx-lead = Zendvoorsprong
sstv-silence-sent-after-keying-before = Stilte die verstuurd wordt na het indrukken en vóór de voorloop en VIS-code van het beeld. Een decoder die een deel van die kop mist, toont helemaal geen beeld, dus dit dekt de tijd tussen het vragen van PTT aan een radio en het werkelijk op de lucht zijn. 0 voor een SDR die meteen stuurt.
sstv-live = LIVE
sstv-waiting-picture = wachten op een beeld…
sstv-waiting-signal = wachten op een signaal…
sstv-no-low-audio = geen / lage audio
sstv-received = ONTVANGEN
sstv-reading-radio-s-pictures = De beelden van de radio worden gelezen…
sstv-decoded-pictures-collect-here = Gedecodeerde beelden verzamelen zich hier.
sstv-click-enlarge-right-click-delete = Klik om te vergroten · rechtsklik om te verwijderen
sstv-delete-picture = Dit beeld verwijderen
sstv-loading = laden…
sstv-transmit = ZENDEN
sstv-click-edit-slot-s-message = Klik om het bericht van deze plek te bewerken · dubbelklik om een beeld te laden
sstv-change-image = Beeld wijzigen…
sstv-load-image = Beeld laden…
sstv-clear = Wissen
sstv-empty-slot-s-picture-message = Het beeld van deze plek leegmaken (het bericht blijft)
sstv-banner = Banner…
sstv-what-printed-across-top-every = Wat bovenaan elk beeld gedrukt staat dat dit station verstuurt
sstv-fetching-slot-s-picture = Het beeld van deze plek ophalen…
sstv-load-image-into-slot = Laad een beeld in deze plek →
sstv-drawn-slot-s-image = Getekend op het beeld van deze plek
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}ZENDEN AFBREKEN{" "}
sstv-delete-picture-store = Dit beeld uit de opslag verwijderen
sstv-received-image = Ontvangen beeld
sstv-no-longer-store = niet meer in de opslag
sstv-loading-full-size = op ware grootte laden…
sstv-save-picture = Beeld opslaan…
sstv-save-copy-computer = Een kopie op deze computer opslaan
sstv-delete-sure = Verwijderen — zeker?
sstv-click-again-delete-good = Klik nogmaals om het definitief te verwijderen
sstv-delete = Verwijderen…
sstv-image-banner = Beeldbanner
sstv-drawn-into-every-picture-station = Getekend in elk beeld dat dit station zendt, over het eigen bericht van de plek heen.
sstv-draw-banner = De banner tekenen
sstv-substituted-when-picture-composed = Vervangen wanneer het beeld wordt samengesteld:
sstv-top-left = Linksboven
sstv-top-right = Rechtsboven
sstv-colours = Kleuren
sstv-strip-top-edge-fades-black = De strook, aan zijn bovenrand — die vervaagt naar zwart, of naar de verloopkleur eronder
sstv-both-texts = Beide teksten
sstv-gradient = Verloop
sstv-fade = vervagen naar
sstv-fade-strip-top-colour-second = De strook van zijn bovenkleur naar een tweede kleur laten vervagen, in plaats van naar zwart.
sstv-text-outline = Tekstomlijning
sstv-draw-outline-around-banner-text = Een omlijning om de tekst van de banner tekenen, zodat hij leesbaar blijft over een druk of licht strook.
sstv-text-gradient = Tekstverloop
sstv-fade-banner-text-colour-second = De tekst van de banner van zijn kleur naar een tweede kleur laten vervagen, over de breedte van het beeld.
sstv-rainbow-text = Regenboogtekst
sstv-all-text-overriding-colours-above = alle tekst, de kleuren hierboven overschrijvend
sstv-draw-every-text-picture-banner = Elke tekst in het beeld — banner en bericht — tekenen als een horizontale regenboog, de kleuren hierboven negerend.
sstv-message = Bericht
sstv-slot-message-s-text-colour = De tekstkleur van het bericht van de plek
sstv-outline = omlijning in
sstv-height = Hoogte
sstv-how-tall-strip-pixels-transmitted = Hoe hoog de strook is, in pixels van het verzonden beeld. De tekst wordt daarop afgestemd — een SSTV-frame is maar 320 pixels breed, en een hogere banner is wat hem aan de andere kant leesbaar maakt.
sstv-reset = Herstellen
sstv-back-callsign-left-version-right = Terug naar de roepletters links en de versie rechts
sstv-waiting-station-s-digital-mode = Wachten tot de instellingen van de digitale modus van het station geladen zijn…
sstv-rifp = RIFP
sstv-size = Grootte
sstv-calling-frequency-draft-names = De roepfrequentie die het concept noemt
sstv-encode = Coderen
sstv-how-picture-encoded-into-object = Hoe het beeld gecodeerd wordt in het object dat RIFP draagt. Auto probeert elke methode en verstuurt de kleinste.
sstv-try-every-encoding-send-smallest = Elke codering proberen, de kleinste versturen (nooit verliesgevend)
sstv-gray = Grijs
sstv-grayscale-depth-rifp-s-raster = Grijstintdiepte. Het raster van RIFP is per definitie grijstinten — kleur heeft geen plaats in zijn manifest.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Kwantiseringsfout verspreiden — de moeite waard onder 8 bits
sstv-caption = Onderschrift
sstv-sent-manifest-content-hint-shown = Verstuurd in het manifest als inhoudshint, en door ontvangers onder het beeld getoond. Reist als tekst mee — het wordt niet in het beeld getekend.
sstv-what-picture = Wat dit beeld is
sstv-repeat-data = Gegevens herhalen
sstv-send-every-data-frame-many = Elk gegevensframe zoveel keer versturen. RIFP is eenrichting zonder herstelverzoeken, dus dit is het enige herstel dat een ontvanger krijgt.
sstv-chunk = Blok
sstv-payload-octets-per-data-frame = Nuttige octetten per gegevensframe (het profiel adviseert 192)
sstv-valid-frames-frames-failed-crc = Geldige frames, frames die de CRC niet haalden, en volledig geverifieerde beelden
sstv-no-transfer-progress = geen overdracht bezig
sstv-forget-incomplete-transfer = Deze onvolledige overdracht vergeten

## Memories and voice keyer windows (src/app/windows.rs)

windows-recall = Oproepen
windows-delete = Verwijderen
windows-edit-name-frequency-mode-antenna = De naam, frequentie, modus, antenne en repeaterinstelling bewerken
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = De repeaterverschuiving, toon en burst van 1750 Hz die bij dit kanaal opgeslagen zijn
windows-abandon-edit-escape = De bewerking laten vallen (Escape)
windows-save = OPSLAAN
windows-keep-changes-enter = De wijzigingen bewaren (Enter)
windows-recall-channel-without-moving-antenna = Dit kanaal oproepen zonder de antenne te verplaatsen
windows-antenna-switch-when-channel-recalled = De antenne waarnaar geschakeld wordt wanneer dit kanaal wordt opgeroepen. — laat staan waar de radio al op staat.
windows-shift = Verschuiving
windows-tone = Toon
windows-invert = OMKEREN
windows-normal = NORMAAL
windows-dcs-polarity = DCS-polariteit
windows-open-every-over-tone-burst = Elke zending openen met de toonburst terwijl dit kanaal opgeroepen is
windows-memories = Geheugens
windows-memory-name = naam van het geheugen
windows-store = Opslaan
windows-folder-name = mapnaam
windows-new-folder = Nieuwe map
windows-import = IMPORTEREN
windows-read-channel-list-chirp-csv =
    Een kanaallijst lezen uit een CHIRP-CSV-bestand (.csv) — een export van een repeaterlijst, een kanaaltabel voor de marine of PMR, of wat CHIRP verder kan schrijven.

    Elk kanaal brengt zijn frequentie, modus, repeaterverschuiving en CTCSS- of DCS-toon mee. Kanalen die al op deze lijst staan — dezelfde frequentie, dezelfde modus — worden overgeslagen, dus opnieuw importeren van een bijgewerkte lijst voegt toe wat nieuw is, in plaats van te verdubbelen wat dat niet is.
windows-chirp-csv = CHIRP-CSV
windows-export = EXPORTEREN
windows-write-list-out-chirp-csv = Deze lijst wegschrijven als CHIRP-CSV-bestand, om in een handradio te laden of als back-up te bewaren.
windows-no-memories-yet = nog geen geheugens
windows-drag-memory-onto-folder-file = sleep een geheugen op een map om het daarin te zetten, eronder om het eruit te halen
windows-drop-here-unfile = hier neerzetten om het eruit te halen
windows-sort = Sorteren
windows-order-they-were-stored = In de volgorde waarin ze opgeslagen zijn
windows-name-ignoring-case = Op naam, hoofdletters genegeerd
windows-frequency = Op frequentie
windows-band-then-frequency-inside-each = Op band, daarna op frequentie binnen elke band
windows-descending-click-ascending = Aflopend — klik voor oplopend
windows-ascending-click-descending = Oplopend — klik voor aflopend
windows-delete-folder-memories-move-top = Map verwijderen — de geheugens daarin verhuizen naar het hoogste niveau
windows-rename-folder = Map hernoemen
windows-empty-drop-memories-here = leeg — zet hier geheugens neer
windows-voice-keyer = Spraakkeyer
windows-rec-records-your-microphone-play = REC neemt op van je microfoon, PLAY laat je luisteren naar wat je opnam, TX zendt het uit — net als een toets op het numeriek toetsenblok, een MIDI-pad, of rigctld's send_voice_mem.
windows-stop-store = Stoppen en opslaan
windows-stop = STOP
windows-play = AFSPELEN
windows-stop-listening = Stoppen met luisteren
windows-nothing-recorded-slot = Niets opgenomen in deze plek
windows-not-while-transmitting = Niet terwijl er gezonden wordt
windows-listen-message-nothing-transmitted = Luister naar dit bericht — er wordt niets uitgezonden
windows-stop-transmitting = Stoppen met zenden
windows-tune-active-switch-off-first = TUNE is actief — zet hem eerst uit
windows-transmit-message = Dit bericht uitzenden
windows-erase-recording = Deze opname wissen
windows-rade-message-encoded-digital-voice = RADE: het bericht wordt gecodeerd door de digitale-spraakcodec, precies zoals een live zending dat zou zijn.

## TX Audio window: messages with values

txaudio-rec-hint = Neem een testzin op van de microfoon — tot { $max } s, zonder de zender te sturen. Klik nogmaals om te stoppen. Hij wordt onbewerkt bewaard, zodat AFSPELEN dezelfde zin door elke wijziging die je maakt kan laten gaan.
txaudio-phrase-length = Zin van { $secs } s
# $clip is txaudio-microphone-may-clipping-before-gets or empty.
txaudio-auto-set-done = Ingangsgain ingesteld op { $gain } dB (stempieken waren { $loud } dBFS).{ $clip }
txaudio-listening = Luisteren… { $secs } s
txaudio-auto-set-hint = Praat normaal gedurende { $secs } s — zoals je op de lucht zou doen — en de ingangsgain wordt zo ingesteld dat je stem rond { $target } dBFS piekt. Klik nogmaals om te annuleren.
txaudio-eq-point = { $hz } Hz, { $db } dB — sleep om te verplaatsen
txaudio-not-a-profile = Geen profiel TX-audio: { $error }
txaudio-imported = "{ $name }" geïmporteerd.
txaudio-saved = "{ $name }" opgeslagen.
txaudio-deleted = "{ $name }" verwijderd.
txaudio-loaded-preset = Het startpunt "{ $name }" geladen.

## Solar-system 3D view: messages with values

solar-age-offline = { $age } · offline
solar-radius-exaggeration = Overdrijving van de straal van Aarde/Maan (max. { $max }× bij deze schaal van de maanbaan)
solar-brightest-cell = helderste cel ≈ { $peak } paden · { $bands } { $bands ->
        [one] band
       *[other] banden
    }
solar-hits-in-hour = { $hits } in het uur
solar-passes-from = { $name } · PASSAGES VANAF { $grid }
solar-elements-age = elementen { $age } oud · SGP4
solar-point-at = Richt op { $az }° ({ $compass }), hoogte { $el }°.
solar-published-as = gepubliceerd als { $name }
solar-fly-there = { $name } — ↵ om er naartoe te vliegen
solar-sats-tracked = { $n } van { $total } gevolgd
solar-bodies-found = { $n } van { $total } lichamen
solar-sats-and-bodies = { $n } van { $total } gevolgd · { $bodies } van { $bodies_total } lichamen
solar-confidence-on-band = { $confidence } op { $band }
solar-kp-now = { $kp } nu
solar-kp-in = { $kp } over { $hours } u
solar-valid-age = geldig { $valid } · { $age } oud
solar-full-bar = volle balk = { $share } van de wereld
solar-own-paths-memory = waar de eigen paden van dit station heen gingen · geheugen van { $minutes } min
solar-n-storms = { $n } storingen
solar-clouds-legend = WOLKEN { $time } · { $age } oud · { $channels } · { $storms } · bliksem gesimuleerd
solar-cme-eta = ETA { $eta } (+{ $hours } u)
solar-cme-arrived = aankomst was { $eta } ({ $hours } u geleden)
# $glancing and $estimated are solar-glancing / solar-direction-estimated or empty.
solar-cme-earth-directed = CME NAAR DE AARDE { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Digital-mode setup dialogs: messages with values

setup-mode-setup = Instelling { $mode }
setup-transmits-as = Zendt uit als { $call }
setup-decimal-degrees = Decimale graden, negatief voor { $hemisphere }. Plak er een rechtstreeks van een kaart.

## AMP window: messages with values

amp-unreliable-at-swr = { $meters } ONBETROUWBAAR BIJ DEZE SWR
amp-lower-drive-first = Verlaag eerst de aansturing: { $drive } W zou de versterker in de lijn overbelasten (de grens is { $limit } W).
amp-live-value = live { $value } { $unit }
amp-tripped = GESCHAKELD{ $why }

## Digital-mode panels, shared: messages with values

panels-top-paths = top ≈ { $n } paden
panels-no-agreed-frequency = Er is geen frequentie afgesproken voor { $mode } — open dit om die waarop je zit op te slaan
panels-mode-frequencies-hint = De { $n } frequenties voor { $mode } — een kiezen stemt de dial af, en je kunt je eigen opslaan
panels-mode-frequencies = { $mode } · frequenties
panels-global-convention = Een wereldwijde conventie die het bandplan van IARU-regio { $region } geen smalle data toekent — controleer je eigen bandplan voordat je hier zendt.
panels-outside-data-segment = Amber: buiten het datasegment van regio { $region }.
panels-save-dial = ＋ { $mhz } MHz opslaan
panels-save-dial-hint = Onthoud deze dial onder { $mode } en bied hem vanaf nu hier aan. Opgeslagen op het station, dus elk scherm dat aan deze radio hangt, heeft hem.
panels-tr-period = T/R-periode van { $period } seconden
# $mark is panels-burst-mark while transmitting, otherwise empty.
panels-this-turn = Deze beurt: { $into } s van { $slot }. Decoderingen komen aan het eind ervan binnen, en dan mag de volgende zending beginnen.{ $mark }
panels-burst-mark = {" "}De markering is waar de burst stopt, { $secs } s erin.
panels-tr-period-burst = T/R-periode van { $slot } seconden, burst van { $burst } s

## Memories and voice keyer windows: messages with values

windows-n-channels = { $n } { $n ->
        [one] kanaal
       *[other] kanalen
    }
windows-slot-n = Plek { $n }
windows-record-from-mic = Opnemen van de microfoon (tot { $max } s)

## SSTV panel: messages with values

sstv-picture-too-big = Dat beeld is { $size } MB — de grens is { $limit } MB.
sstv-auto-mode = Auto ({ $mode })
sstv-not-decoded = { $what } — niet gedecodeerd
sstv-last = laatste: { $mode }
# $place is sstv-store-on-radio or sstv-store-in.
sstv-saved-where = Gedecodeerde beelden worden { $place } opgeslagen en verzamelen zich hier.
sstv-older-in-store = { $n } ouder in de opslag
sstv-older-load-more = { $n } ouder — meer laden
sstv-delete-picture-from = Dit beeld { $place } verwijderen
sstv-rifp-profile = { $profile } · ~{ $khz } kHz bezet · de dial is het midden van het kanaal
sstv-rifp-too-wide = ⚠ { $profile } beslaat ~{ $khz } kHz — te breed voor een smalbandsegment
sstv-rifp-no-frequency = RIFP kent geen frequentie toe, en sdroxide zendt hem uit waar je ook afstemt. Een kanaal van { $khz } kHz past alleen waar breedband- of FM-werking is toegestaan — { $segments } — en niet in een smalbandsegment, en zeker niet op HF. Zelfs daarbinnen kunnen je eigen vergunningsvoorwaarden smaller zijn. Jij bent de operator; controleer je eigen regels.
sstv-rifp-tx-frame = ● TX-frame { $frame }/{ $frames } · { $secs } s over
sstv-rifp-sent-as = verstuurd als { $encoding } · { $octets } octetten
sstv-rifp-rx-stats = frames { $frames } · fout { $bad } · beelden { $pictures }
sstv-rifp-session-idle = sessie { $session } · inactief { $secs } s
sstv-rifp-session-waiting = sessie { $session } · blokken vastgehouden, wacht nog op het manifest · inactief { $secs } s
sstv-store-on-radio = op de radio, in { $dir }
sstv-store-in = in { $dir }

## decodes (src/app/panels/decodes.rs)

decodes-decodes = DECODERINGEN
decodes-adif = ADIF
decodes-sort = Sorteren
decodes-single-list = Eén lijst
decodes-every-decode-one-list-newest = Elke decodering in één lijst, de nieuwste beurt eerst, in plaats van oneven/even beurtblokken. De sorteerchips ordenen dan de hele lijst in plaats van elke beurt, en elke rij draagt zijn tijdslottijd — cyaan voor een even slot, goud voor een oneven.
decodes-cq-only = Alleen CQ
decodes-only-stations-calling-cq-only = Alleen stations die CQ roepen — en alleen de oproepen die je mag beantwoorden: een gerichte CQ (DX, EU, JA, POTA, TEST …) wordt getoond wanneer hij jou noemt en verborgen wanneer hij iemand anders noemt.
decodes-new-only = Alleen nieuw
decodes-only-stations-would-new-entity = Alleen stations die nieuw zouden zijn: entiteit, bandslot, locatievak of roepletters
decodes-auto-tx-frq = Auto zend-FRQ
decodes-overridden-hold-tx-lift-hold = Overschreven door Zendfrequentie vasthouden. Hef de vasthouding op om te kiezen hoe de zendfrequentie beweegt.
decodes-pick-our-transmit-frequency-automatically = Kies onze zendfrequentie automatisch: de rustigste plek in de periode waarin we zenden, in plaats van de frequentie van degene die we beantwoorden — die zenden in de andere periode, dus wat zij zeggen, zegt niets over wie er is wanneer wij sturen. Uit houdt de frequentie NIET vast: hij antwoordt op de frequentie van het station dat gebeld wordt. Gebruik Zendfrequentie vasthouden om hem vast te zetten.
decodes-hold-tx = Zendfrequentie vasthouden
decodes-pin-transmit-tone-where-nothing =
    Zet de zendtoon vast waar hij staat. Niets verplaatst hem: niet het beantwoorden van een station, niet de oproepwachtrij, niet CQ roepen, niet een klik op een decodering of op de waterval. Zet hem uit om te verplaatsen, en dan weer aan.

    Van band wisselen is de enige uitzondering, en dat is je eigen handeling: de offset die je laatst op de nieuwe band instelde, komt ermee terug, dus een waarde van 60 m wordt niet naar 20 m meegenomen en die van 20 m niet naar 60 m.

    Voor als de vergunning smaller is dan het bandplan — op een Brits 60 m-dial van 5357 kHz eindigt de toewijzing bij 5358.0, dus de toon moet onder 1000 Hz blijven, en beide automatische verplaatsers lopen tussen de ene zending en de volgende uit de band.
decodes-type-transmit-offset-hz-then = Typ de zendoffset in Hz, en druk dan op Enter. Waar je vergunning smaller is dan het bandplan, is het hele bereik onder de rand van jou om uit te kiezen, dus er wordt hier niets voorgesteld.
decodes-clear-rx = {" "}RX WISSEN{" "}
decodes-empty-decode-list-nothing-air = De lijst met decoderingen leegmaken. Niets wat op de lucht is, stopt.
decodes-even = EVEN
decodes-text = TEKST
decodes-dxcc = DXCC
decodes-band = BAND
decodes-grid = GRID
decodes-dupe = DUBBEL
decodes-reply = ANTWOORD
decodes-queued-click-remove = In de wachtrij — klik om te verwijderen
decodes-work-station-after-current-one = Dit station werken na het huidige
decodes-qsos-worked-since-sdroxide-was = QSO's gewerkt sinds sdroxide gestart is
decodes-setup = ⚙ INSTELLING
decodes-qso-complete = ✓ QSO AFGEROND
decodes-contact-complete-log-held-here = Het contact is af en staat in het logboek. Het wordt hier een paar minuten vastgehouden zodat het laatste bericht opnieuw verstuurd kan worden als het andere station het zijne herhaalt — er is verder niets verschuldigd.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = DXpedition-modus. De zendfrequentie wordt buiten de helft van de Fox in de doorlaatband gehouden (onder 1000 Hz) tot de Fox antwoordt.
decodes-watchdog = WATCHDOG
decodes-transmitting-stopped-no-reply-no = Zenden gestopt: geen antwoord en geen actie gedurende de watchdog-periode. Roep CQ of kies een bericht om verder te gaan.
decodes-well-inside-tolerance = Ruim binnen de tolerantie.
decodes-no-active-qso-pick-decode = geen actieve QSO — kies een decodering om te antwoorden, of roep CQ
decodes-ft8-engine-idle = FT8-engine inactief
decodes-being-worked = wordt gewerkt
decodes-clear = WISSEN
decodes-no-messages = — geen berichten —
decodes-call-cq = {"  "}CQ ROEPEN{"  "}
decodes-stop-qso = {" "}QSO STOPPEN{" "}
decodes-stop-tx = {" "}ZENDEN STOPPEN{" "}
decodes-free-text-13-chars = vrije tekst (13 tekens)
decodes-send = VERSTUREN

## sat (src/app/sat.rs)

sat-pass-profile = PASSAGEPROFIEL
sat-satellite = SATELLIET
sat-satellites = SATELLIETEN
sat-passes = PASSAGES
sat-set-your-grid-passes-settings = Stel je locatievak in voor passages (Instellingen ▸ Algemeen).
sat-next = VOLGENDE
sat-peak = PIEK ≥
sat-all-satellites = alle satellieten
sat-off-only-satellites-frequencies-file = Uit: alleen satellieten met frequenties in het bestand — de satellieten waarmee je kunt werken — en alle die je zelf geplakt hebt. Aan: elke satelliet in de lijst, wat er met een abonnement duizenden kunnen zijn, en het zoeken duurt een tijdje.
sat-no-passes-window-lower-peak = Geen passages in dit venster. Verlaag de piek, of verbreed het venster.
sat-length = LENGTE
sat-peak-2 = PIEK
sat-from-to = VAN → TOT
sat-up-now = ● nu boven
sat-unlock = ONTGRENDELEN
sat-release-lock-dial-stays-where = De vergrendeling loslaten; de dial blijft staan waar hij is
sat-azimuth = AZIMUT
sat-elevation = HOOGTE
sat-range = AFSTAND
sat-rate = SNELHEID
sat-downlink = DOWNLINK
sat-doppler-rx = DOPPLER RX
sat-uplink = UPLINK
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = Geen passage binnen de komende 48 uur vanaf je QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elementen verouderd — correcties opgeschort tot een TLE-vernieuwing
sat-doppler-correction-needs-iq-front = Doppler-correctie vereist een IQ-front-end — een CAT-radio volgt alleen
sat-rotator-not-connected = Rotor: niet verbonden
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = De berekende correctie toepassen op RX en TX
sat-rotator = ROTOR
sat-steer-antenna-through-rotctld-client = De antenne sturen via de rotctld-client (Instellingen ▸ Servers)
sat-name-norad = naam of NORAD
sat-set-your-grid-passes-settings-2 = stel je locatievak in voor passages (Instellingen ▸ Algemeen)
sat-norad = NORAD
sat-pass = PASSAGE
sat-always-visible = altijd zichtbaar
sat-not-here = niet vanaf hier
sat-no-elements = geen elementen
sat-pick-satellite-see-links-lock = Kies een satelliet om zijn verbindingen te zien en erop te vergrendelen.
sat-no-frequencies-file-one-add = Geen frequenties in het bestand voor deze — voeg ze toe in Instellingen ▸ TLE.
sat-link = VERBINDING
sat-downlink-mhz = DOWNLINK (MHz)
sat-uplink-mhz = UPLINK (MHz)
sat-mode = MODUS
sat-tune = AFSTEMMEN
sat-set-dial-mode-link-no = De dial en modus instellen op deze verbinding — geen vergrendeling, geen correctie
sat-locked = {" "}VERGRENDELD{" "}
sat-lock = {" "}VERGRENDELEN{" "}
sat-track-satellite-doppler-correction-rx = Deze satelliet volgen: Doppler-correctie op RX en TX, uplink via de transponder, en het 3D-beeld volgt hem

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ STOP{" "}
wefax-end-chart-now-save-what = De kaart nu beëindigen en opslaan wat is binnengekomen
wefax-start = {" "}● START{" "}
wefax-start-chart-now-without-waiting = Nu een kaart starten, zonder te wachten op een starttoon
wefax-phasing = fasering…
wefax-subcarrier-offset-1900-hz-tune = Subdraaggolfoffset vanaf 1900 Hz. Stem af tot ongeveer nul: de faxdraaggolf is 1500 Hz zwart tot 2300 Hz wit, en een ontvanger die een paar honderd hertz ernaast zit, knipt het beeld tot volledig zwart of volledig wit.
wefax-auto-start = AUTOMATISCH STARTEN
wefax-begin-chart-when-300-hz = Een kaart beginnen wanneer de starttoon van 300 Hz (IOC 576) of 675 Hz gehoord wordt — en de lopende beëindigen en de nieuwe faseren zodra de volgende zending binnenkomt, of er nu eerst een stoptoon kwam of niet.
wefax-auto-stop = AUTOMATISCH STOPPEN
wefax-end-450-hz-stop-tone = Beëindigen op de stoptoon van 450 Hz. Uitzetten om door te blijven opnemen bij een station dat meerdere kaarten achter elkaar stuurt.
wefax-phase = FASE
wefax-slant = SCHEEFSTAND
wefax-sample-clock-trim-if-chart = Kloktrim van de samples. Als de kaart naar links leunt, verhoog dit; naar rechts, verlaag het. Een geluidskaart die honderd ppm afwijkt, schuift een kaart van een kwartier grotendeels een lijn opzij.
wefax-view = WEERGAVE
wefax-scale-chart-panel-width = De kaart schalen naar de breedte van het paneel
wefax-whole = HEEL
wefax-shrink-chart-until-all-view = De kaart verkleinen tot alles in één keer in beeld is
wefax-half-size = Halve grootte
wefax-one-screen-pixel-per-fax = Eén schermpixel per faxpixel — scrollen voor detail
wefax-twice-size-scroll-move-around = Dubbele grootte; scrol om over de kaart te bewegen
wefax-height = HOOGTE
wefax-stretch-picture-vertically-chart-came = Het beeld verticaal uitrekken. Een kaart die er ingedrukt of uitgerekt uitkwam, wordt meestal met de verkeerde lijnsnelheid gedecodeerd — dit maakt hem leesbaar, en de LPM-chips lossen het goed op. Dubbelklik om een waarde in te typen.
wefax-reset = HERSTELLEN
wefax-back-picture-s-own-proportions = Terug naar de eigen verhoudingen van het beeld
wefax-follow = VOLGEN
wefax-keep-newest-lines-view-scrolling = De nieuwste regels in beeld houden. Omhoog scrollen zet dit uit zodat je kunt lezen wat al binnen is; weer naar beneden scrollen zet het weer aan.
wefax-tune-fax-schedule-usb-wait = Stem een faxschema af in USB en wacht op een starttoon, of druk op START om midden in een kaart te beginnen.
wefax-saved = OPGESLAGEN
wefax-path = PAD
wefax-reading-radio-s-charts = De kaarten van de radio worden gelezen…
wefax-finished-charts-collect-here = Afgemaakte kaarten verzamelen zich hier.
wefax-delete-chart = Deze kaart verwijderen
wefax-loading = laden…
wefax-stations = STATIONS
wefax-broadcast-radiofax-schedules-picking-one = Radiofaxschema's van omroepen — een kiezen stemt de dial af
wefax-frequencies-are-published-carrier-dial = De frequenties zijn de gepubliceerde draaggolf; de dial gaat 1,9 kHz eronder, wat voor je gedaan wordt. Schema's veranderen en stations sluiten — gebruik dit als plek om te beginnen zoeken, niet als dienstregeling.
wefax-delete-chart-store = Deze kaart uit de opslag verwijderen
wefax-newer = ◀ NIEUWER
wefax-chart-received-after-one = De kaart die na deze is ontvangen
wefax-older = OUDER ▶
wefax-chart-received-before-one = De kaart die voor deze is ontvangen
wefax-no-longer-store = niet meer in de opslag
wefax-loading-full-size = op ware grootte laden…
wefax-save = OPSLAAN
wefax-save-copy-chart-computer = Een kopie van deze kaart op deze computer opslaan
wefax-sure = ZEKER?
wefax-click-again-delete-chart-good = Klik nogmaals om deze kaart definitief te verwijderen
wefax-delete = VERWIJDEREN
wefax-file-chart-was-saved = Het bestand waarnaar deze kaart is opgeslagen

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = de APRS-modem starten…
aprs-aprs = APRS
aprs-no-callsign = geen roepletters
aprs-nothing-will-transmitted-no-beacon = Er wordt niets verzonden — geen baken, geen bericht, zelfs geen bevestiging — tot dit station roepletters heeft. Stel die in onder Instellingen → Algemeen, of een APRS-specifieke met zijn SSID onder INSTELLING.
aprs-no-position = geen positie
aprs-beacon-needs-somewhere-report-fill = Een baken heeft een plek nodig om naar te melden. Vul je locator in onder Instellingen → Algemeen, of geef coördinaten onder INSTELLING.
aprs-setup = INSTELLING
aprs-callsign-symbol-digipeater-path-position = Roepletters, symbool, digipeaterpad, positie en baken
aprs-how-often-beacon-your-position = Hoe vaak je positie gebakend wordt, zonder toezicht. `off` — de standaard — bakent nooit: het kiezen van een modus mag een station niet op de lucht zetten. Dertig minuten is de conventie voor een vast station, vaker voor een bewegend station, en elk baken is andermans kanaaltijd.
aprs-beacon = {" "}BAKEN{" "}
aprs-station-has-no-callsign-see = Dit station heeft geen roepletters — zie de waarschuwing links.
aprs-station-has-no-position-report = Dit station heeft geen positie om te melden — zie de waarschuwing links.
aprs-send-one-position-report-now = Nu één positierapport versturen, zonder op het interval te wachten. Het wacht tot het kanaal vrij is, zoals alles.
aprs-stations = STATIONS
aprs-nothing-heard-yet = nog niets gehoord
aprs-heard-direct = rechtstreeks gehoord
aprs-two-characters-station-sent-say = De twee tekens die het station stuurde om te zeggen wat het is
aprs-killed = VERWIJDERD
aprs-station-put-object-here-has = Het station dat dit object hier neerzette, heeft het ingetrokken
aprs-sender-deliberately-reported-square-rather = De zender meldde bewust een vierkant in plaats van een punt. De kaart tekent het vierkant.
aprs-digipeaters-last-frame-came-through = De digipeaters waar het laatste frame doorheen kwam. Een `*` markeert er een die het daadwerkelijk herhaalde.
aprs-message = {" "}BERICHT{" "}
aprs-address-message-box-station = Het berichtvak aan dit station richten
aprs-center = {" "}MIDDEN{" "}
aprs-put-station-middle-map-double = Dit station in het midden van de kaart zetten. Dubbelklik op de kaart om het beeld terug te geven aan de automatische passing.
aprs-channel = KANAAL
aprs-messages = BERICHTEN
aprs-frames-arrived-failed-their-check = Frames die binnenkwamen en hun controlereeks niet haalden — een botsing, een vervaging, of een signaal dat te zwak is om te lezen. Een oplopend aantal terwijl er niets gedecodeerd wordt, betekent dat de modem het kanaal *wel* hoort: controleer de niveaumeter, en open de squelch van de radio.
aprs-frames-read-cleanly-off-channel = Frames die netjes van het kanaal gelezen werden maar geen APRS waren — de pakketsessie van iemand anders, of een formaat dat deze build niet decodeert.
aprs-every-frame-channel-arrived-other = Elk frame op het kanaal, zoals het binnenkwam — ook het verkeer van anderen, en alles wat de decoder niet kon lezen.
aprs-nothing-channel-yet = nog niets op het kanaal
aprs-no-messages = Geen berichten.
aprs-waiting-channel = wachten op het kanaal
aprs-sent-retried-still-waiting-acknowledged = verstuurd en opnieuw geprobeerd, wacht nog op bevestiging
aprs-air-waiting-acknowledged = op de lucht, wacht op bevestiging
aprs-acknowledged-far-end = bevestigd door het verre eind
aprs-refused-far-end-not-retried = geweigerd door het verre eind — niet opnieuw geprobeerd
aprs-no-ack = geen bevestiging
aprs-no-answer-after-every-retry = geen antwoord na elke herhaling
aprs-send = {" "}VERSTUREN{" "}
aprs-station-has-no-callsign-so = Dit station heeft geen roepletters, dus er kan niets verzonden worden. Stel die in onder Instellingen → Algemeen, of een APRS-specifieke met zijn SSID onder INSTELLING.
aprs-needs-station-address-something-say = Heeft een station nodig om aan te richten, en iets om te zeggen.
aprs-send-keep-retrying-until-far = Versturen, en blijven herhalen tot het verre eind bevestigt. Berichten zijn het enige op dit kanaal dat beantwoord wordt.
aprs-busy = BEZET
aprs-stations-heard-still-inside-window = Stations die gehoord zijn en nog binnen het venster vallen dat in APRS-instelling is ingesteld, en of er nu een ander station op het kanaal is. Er wordt niets gestuurd zolang het bezet is.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = De lopende zending, de frames die wachten tot het kanaal vrij is, of de tijd tot het volgende geplande baken.
aprs-audio-reaching-modem-flat-means = Audio die de modem bereikt. Vlak betekent dat er helemaal niets binnenkomt — controleer of de data-uitgang van de radio dezelfde is als waar sdroxide naar luistert. Hij licht groen op zodra de modem een draaggolf hoort.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = de VDL2-decoder starten…
vdl2-aircraft-datalink = datalink voor vliegtuigen
vdl2-tune-middle-vdl2-group-decoder = Stem af op het midden van de VDL2-groep. Het eigen venster van de decoder schuift vanaf daar om zoveel van de veertien kanalen op te nemen als de ontvanger kan bereiken.
vdl2-setup = INSTELLING
vdl2-which-channels-listen-how-hard = Op welke kanalen er geluisterd wordt, hoe sterk een burst moet zijn, en hoeveel logboek bewaard wordt
vdl2-save-vdl2-message-log-file = Het VDL2-berichtenlogboek naar een bestand opslaan
vdl2-vdl2-channel = VDL2-kanaal
vdl2-messages = BERICHTEN
vdl2-address-flight-registration-label = adres, vlucht, registratie of label
vdl2-held = VASTGEHOUDEN
vdl2-hold = VASTHOUDEN
vdl2-hold-message-log-where-so = Het berichtenlogboek op zijn plek houden, zodat er één gelezen kan worden zonder dat de volgende zending hem het scherm uit duwt. De decoder blijft draaien en de tellers blijven lopen; loslaten toont alles wat in de tussentijd binnenkwam.
vdl2-nothing-decoded-yet = nog niets gedecodeerd
vdl2-nothing-matches-filter = niets past bij het filter
vdl2-stations = STATIONS
vdl2-nobody-heard-yet = nog niemand gehoord
vdl2-vdl2-setup = VDL2-instelling
vdl2-channels = Kanalen
vdl2-every-25-khz-slot-136 = Elk slot van 25 kHz van 136,650 tot 136,975 MHz. Eén downconverter per stuk, allemaal binnen hetzelfde ontvangstvenster. Eén uitzetten bespaart een beetje processortijd; het maakt de andere niet gevoeliger.
vdl2-burst-threshold = Drempel voor burst
vdl2-above-each-channel-s-own = boven de eigen ruisvloer van elk kanaal
vdl2-lower-catches-weaker-transmissions-costs = Lager vangt zwakkere zendingen op en kost processortijd aan ruis; hoger mist ze. De ruisvloer wordt per kanaal geleerd, en een wijziging hier gooit hem niet weg.
vdl2-keep-log = Bewaren in het logboek
vdl2-messages-2 = {" "}berichten
vdl2-track-most = Maximaal volgen
vdl2-stations-2 = {" "}stations
vdl2-longest-silent-go-first = de langst stille gaan eerst
vdl2-forget-station-after = Een station vergeten na
vdl2-nothing-heard = waarvan niets gehoord is
vdl2-show-unread-payloads = Ongelezen payloads tonen
vdl2-hex = als hex
vdl2-frames-carrying-x-25-clnp = Frames die X.25, CLNP of de datalinktoepassingen daarboven dragen. SDRoxide noemt ze en toont de bytes in plaats ze te lezen, en ze verbergen zou verbergen hoeveel van het verkeer dat is.
vdl2-registration = Registratie
vdl2-flight = Vlucht
vdl2-label = Label
vdl2-block = Blok
vdl2-sequence = Volgorde
vdl2-mode = Modus
vdl2-more-blocks-follow = er volgen meer blokken
vdl2-check = Controle
vdl2-message-crc-good = CRC van het bericht goed
vdl2-message-crc-not-checked = CRC van het bericht niet gecontroleerd
vdl2-parity = Pariteit
vdl2-link-control-no-payload = linkbesturing, geen payload
vdl2-name = NAAM
vdl2-addr = ADRES
vdl2-type = TYPE
vdl2-msgs = BERICHTEN

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = De toon die gekopieerd wordt, en de toon die verzonden wordt — bij CW is het dezelfde frequentie. Klik op de waterval om hem op een signaal te zetten.
cw-down-10-hz = 10 Hz omlaag
cw-up-10-hz = 10 Hz omhoog
cw-qrg-main-readout-tuning-line = QRG: de hoofduitlezing en de afstemlijn staan op de frequentie waarop gewerkt wordt. Klik voor de dial in plaats daarvan, een sidetone-toonhoogte eronder — wat de meeste radio's tonen.
cw-put-main-readout-tuning-line =
    Zet de hoofduitlezing en de afstemlijn op het signaal in plaats van op de dial, zodat de getoonde frequentie die is die beide operators zouden noemen, en de afstemlijn in het midden van de doorlaatband staat. Het afstemmen verandert niet; alleen de getallen verschuiven.

    Een klik op een signaal zet het alleen zo dicht op de cursor als de klikstap toelaat — Bediening → afronding bij afstemmen met klikken, standaard 10 Hz. Een grove stap laat het signaal tot de helft ervan naast de toonhoogte staan, en de uitlezing zegt dat.
cw-sending-speed-read-off-signal = Zendsnelheid gelezen uit het signaal
cw-signal-noise-500-hz-same = Signaal ten opzichte van ruis in 500 Hz — hetzelfde getal dat een rapport noemt
cw-how-far-off-cursor-signal = Hoe ver het signaal werkelijk naast de cursor zit. De decoder volgt het; de doorlaatband niet, dus duw de cursor een beetje als het groter wordt.
cw-listening = — luisteren —
cw-tx = ● TX
cw-trainer = TRAINER
cw-learn-morse-read-characters-words = Leer morse: lees tekens en woorden, speel ze af in je eigen tempo, en een Koch-oefening die er telkens één tekens bij voegt. Het speelt hier af, via de luidsprekers van deze computer — niets stuurt de radio.
cw-nothing-copied-yet = — nog niets gekopieerd —
cw-key-send-characters-you-send = Met de sleutel zenden — de tekens die je verstuurt verschijnen hier.
cw-type-line-return-sends = Typ een regel, Enter verstuurt hem…
cw-type-here-send = Typ hier om te versturen…
cw-key = {" "}SLEUTEL ●{" "}
cw-key-2 = {" "}SLEUTEL{" "}
cw-hold-key-bound-cw-straight = De toets die aan CW-sleutel is gekoppeld — standaard Spatie, en elke toets die je wilt in Instellingen → Bediening — als recht sleutel gebruiken: omlaag zolang hij ingedrukt is, omhoog bij loslaten, in plaats van tekst typen. Het zendvak is vergrendeld zolang hij aan staat, en de hele keyer wordt aan de sleutel overgedragen: alle tekst die in de wachtrij stond, vervalt.
cw-radio-sends-own-keyer-text =
    Deze radio zendt vanuit zijn eigen keyer: de tekst gaat over de besturingspoort en de radio regelt de timing van de elementen, dus er zit niets tussen de rechte sleutel en de lucht dat een hand kan aansturen.

    Om hem met de hand te sturen, zet je CW-sturing op "Geluidskaart (MCW)" in Instellingen → Radio. De radio wordt dan op een zijband gehouden en de eigen sidetone van de keyer wordt als audio verzonden — dat is de route die de rechte sleutel aanstuurt.
cw-watchdog = WATCHDOG
cw-straight-key-was-held-down = De rechte sleutel is te lang ingedrukt gehouden — een verloren loslaten in plaats van een hand — dus de draaggolf is gestopt en zenden is uitgezet. Druk opnieuw op de sleutel om door te gaan.
cw-tx-2 = {"  "}TX AAN{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Stuur wat er nu in het vak staat, zonder op Enter te wachten
cw-hold-key-down-between-characters = De toets tussen de tekens ingedrukt houden, zodat niets wat getypt wordt, wacht
cw-call-cq = {" "}CQ ROEPEN{" "}
cw-nocall = GEEN ROEP
cw-clear = {" "}WISSEN{" "}
cw-stop-sending-drop-whatever-has = Stoppen met zenden en alles laten vallen wat nog niet is verstuurd. Met de rechte sleutel aan, gaat de terugkoppeling over wat je hebt gestuurd ook mee.
cw-sidetone = SIDETONE
cw-play-keyed-tone-through-computer = De gestuurde toon ook via de luidsprekers van deze computer afspelen terwijl hij verzonden wordt, zodat je hoort wat je stuurt. Bij `Geluidskaart (MCW)` gaat de toon alleen naar de radio en nergens anders, dus zonder dit zend je in stilte. Uit waar de eigen monitor van de radio of een kopie van de lucht het werk al doet.
cw-hold-what-typed-until-return = De getypte tekst vasthouden tot Enter, en de regel dan in één keer verzenden, in plaats van elk teken te sturen zodra het getypt wordt. Het is de moeite waard op een transceiver die zichzelf vanuit tekst stuurt, waar elke overdracht aan zijn keyer weer een zend-ontvangstcyclus is.
cw-your-own-message-buttons-contest = Je eigen berichtknoppen — een wedstrijduitwisseling, een naam-en-QTH-antwoord, TNX 73 GL. Elk verstuurt zijn hele tekst in één keer, en F2–F10 drukken de eerste negen in. Ze reizen met de configuratie van het station mee, dus een client op afstand heeft ze ook.
cw-cw-messages = CW-BERICHTEN
cw-clear-rx = {" "}RX WISSEN{" "}
cw-empty-receive-window-straight-key = Het ontvangstvenster en de terugkoppeling van de rechte sleutel leegmaken. Niets wat op de lucht is, stopt.
cw-lock = VERGRENDELEN
cw-decode-speed-set-here-instead = Decodeer op de snelheid die hier ingesteld is, in plaats van die uit het signaal te lezen. Helpt bij een signaal dat te zwak is voor de snelheidszoektocht om zich te zetten.
cw-farnsworth-send-characters-full-speed = Farnsworth: de tekens op volle snelheid versturen en alleen de gaten ertussen rekken, zodat ze in het juiste ritme gehoord worden maar langzaam genoeg aankomen om op te schrijven.
cw-off-normal-spacing = Uit — normale spatiëring
cw-keying-speed = Sneltempo
cw-idle-0 = INACTIEF 0
cw-how-long-transmit-held-after = Hoe lang zenden vastgehouden wordt na het laatste teken, of nadat de rechte sleutel losgelaten is, voordat de draaggolf wegvalt. Langer overbrugt de pauzes van een langzame typist; korter verlaat de frequentie sneller; Uit laat hem vallen zodra alles in de wachtrij verstuurd is.
cw-off-drop-once = Uit — meteen laten vallen

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = 10 Hz omlaag afstemmen
textmodem-tune-up-10-hz = 10 Hz omhoog afstemmen
textmodem-tx = ● TX
textmodem-listening = — luisteren —
textmodem-type-line-return-sends = Typ een regel, Enter verstuurt hem…
textmodem-type-here-transmit = Typ hier om te zenden…
textmodem-tx-2 = {"  "}TX AAN{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}CQ ROEPEN{" "}
textmodem-nocall = GEEN ROEP
textmodem-clear = {" "}WISSEN{" "}
textmodem-hold-what-typed-until-return = De getypte tekst vasthouden tot Enter, en de regel dan in één stuk versturen, in plaats van elk teken op de lucht te zetten zodra het getypt wordt. Laat een regel teruglezen en corrigeren voordat er iets van uitgaat.
textmodem-your-own-message-buttons-working = Je eigen berichtknoppen — werkomstandigheden, het weer, een standaardantwoord. Elk verstuurt zijn hele tekst in één keer, en F2–F10 drukken de eerste negen in. Ze reizen met de configuratie van het station mee, dus een client op afstand heeft ze ook.
textmodem-messages = BERICHTEN
textmodem-hell = HELL
textmodem-contrast = Contrast
textmodem-harder-softer-dots-redraws-whole = Hardere of zachtere stippen — tekent de hele strook opnieuw
textmodem-width = Breedte
textmodem-t-2row = {" "}2RIJ{" "}
textmodem-draw-every-column-twice-stacked = Elke kolom twee keer getekend, onder elkaar. Hell heeft geen verticale synchronisatie, dus dit houdt één volledige kopie van de tekst leesbaar, wat de fase ook is.
textmodem-rev = {" "}OMK{" "}
textmodem-reverse-video-light-dots-dark = Omgekeerd beeld — lichte stippen op donker papier
textmodem-clear-rx = {" "}RX WISSEN{" "}
textmodem-hold-channel-idle-sends-blank = Het kanaal vasthouden: inactief verstuurt leeg papier, zodat de strook blijft scrollen
textmodem-hold-what-typed-until-return-2 = De getypte tekst vasthouden tot Enter, en de regel dan in één stuk versturen, in plaats van elk teken op de strook te schilderen zodra het getypt wordt. Een correctie die live getypt wordt, staat al op het papier; een correctie die vóór Enter gemaakt wordt, nooit.
textmodem-shift = Verschuiving
textmodem-shift-5-hz = Verschuiving −5 Hz
textmodem-shift-5-hz-2 = Verschuiving +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Omkeren: markering en spatie verwisselen. Nodig wanneer het signaal op de tegenovergestelde zijband wordt ontvangen van waarop het verzonden werd, wat de tonen omkeert.
textmodem-track-tuning-error-automatically-instead = De afstemfout automatisch volgen, in plaats van vast op de cursor te blijven staan.
textmodem-tones = Tonen
textmodem-mode = Modus
textmodem-off-keyed = aan/uit gestuurd

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = de AIS-decoder starten…
ais-tune-between-two-ais-channels = Stem af tussen de twee AIS-kanalen — 161,975 (AIS 1, maritiem 87B) en 162,025 (AIS 2, 88B). De dial staat in het midden omdat er daar niets gezonden wordt, dus de DC-piek van een zero-IF-ontvanger valt op geen van beide.
ais-setup = INSTELLING
ais-channels-timeouts-trail-length-how = Kanalen, time-outs, lengte van het spoor en hoe ver de vectoren reiken
ais-vessels = SCHEPEN
ais-nothing-heard-yet = nog niets gehoord
ais-virtual-nothing-there = virtueel — er is niets
ais-position-stale = positie verouderd
ais-search-rescue-transmitter-man-overboard = een zoek- en reddingszender, man-overboard-baken of AIS-EPIRB
ais-call-sign = roepnaam
ais-station-time = stationstijd
ais-dgnss = {" "}(DGNSS)
ais-first-heard = eerst gehoord
ais-center = MIDDEN
ais-put-vessel-middle-chart = Dit schip in het midden van de kaart zetten
ais-last-message-station-nmea-sentences = Het laatste bericht van dit station, als de NMEA-zinnen die een ontvanger op de draad zou zetten. Plak het in een andere AIS-decoder om te controleren wat sdroxide ervan gemaakt heeft.
ais-ais-setup = AIS-instelling
ais-channels = Kanalen
ais-drop-chart-after = Van de kaart halen na
ais-without-position-report = zonder positiemelding
ais-five-minutes-default-not-ads = Standaard vijf minuten, niet de tien seconden van ADS-B: een schip dat voor anker ligt, meldt eens per drie minuten, en een korter venster zou het grootste deel van een haven leegmaken tussen twee volkomen goede meldingen. Daarna gaat het schip van de kaart en wordt de rij grijs — niet vervaagd, want een dof symbool op een verouderde positie is nog steeds een bewering over waar een schip is, in dezelfde inkt als de ware.
ais-drop-list-after = Van de lijst halen na
ais-nothing-heard-all = waarvan helemaal niets gehoord is
ais-message-carrying-ship-s-name = Het bericht met de naam van een schip komt om de zes minuten voorbij, dus een kort lijstvenster gooit schepen steeds weg vlak voordat ze zeggen hoe ze heten.
ais-trail-length = Lengte van het spoor
ais-history-behind-each-target = geschiedenis achter elk doel
ais-minutes-rather-than-points-because = In minuten in plaats van in punten, omdat de meldingsfrequenties van AIS twee ordes van grootte uiteenlopen: een vast aantal zou tachtig seconden van een veerboot zijn en twee uur van een voor anker liggende tanker, identiek getekend. Nul zet de sporen uit.
ais-speed-vector = Snelheidsvector
ais-how-far-ahead-vector-reaches = hoe ver vooruit de vector reikt
ais-slot-threshold = Drempel voor het slot
ais-above-channel-s-learned-noise = boven de geleerde ruisvloer van het kanaal
ais-track-most = Maximaal volgen
ais-vessels-2 = {" "}schepen
ais-ais-receive-only-here-always = AIS is hier alleen-ontvangst en blijft dat: het is een dienst voor de veiligheid van mensenlevens, en valse scheepsverkeer erop zetten valt niet onder een vergunning.
ais-fill-my-grid-digimode-setup = Vul Mijn locatievak in de digimode-instelling in voor afstanden en peilingen, zodat de kaart zichzelf rond jouw positie kadreert in plaats van rond wat het verst weg is.
ais-name = NAAM
ais-mmsi = MMSI
ais-type = TYPE

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Deze ontvanger bereikt 10489,750 MHz niet op eigen kracht — stel eerst een LNB/converteroffset in (Instellingen ▸ Radio)
qo100-run-fast-spectral-tracker-every = De snelle spectrale tracker draaien: elke seconde in het parkeervenster zoeken naar de twee symmetrische lobben van het baken, en melden waar de draaggolf zit
qo100-telemetry = TELEMETRIE
qo100-also-run-ao-40-uncoded = Ook de ongecodeerde frame-decoder van AO-40 draaien — synchronisatiewoord, CRC en de telemetrietekst — met een stap-voor-stap-uitlezing (draaggolf → synchronisatie → CRC) van hoe ver elke passage komt
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Automatisch corrigeren: de tracker de converter/LNB-offset zelf laten aanpassen — een langzame gesloten lus die, bij een schone en stabiele schatting, het baken terug op 10489,750 MHz duwt en daar houdt terwijl de LNB wegdrijft. Opent het front-end elke keer opnieuw als het handelt.
qo100-narrower-search-smaller-slice = Smaller — een kleiner stuk doorzoeken
qo100-wider-when-beacon-isn-t = Breder — voor als het baken niet gevonden wordt bij de huidige breedte
qo100-clear-mark = markering wissen
qo100-drop-hand-placed-beacon-mark = De met de hand geplaatste bakenmarkering laten vallen en teruggaan naar de eigen aflezing van de decoder
qo100-lo = lo −
qo100-lo-2 = lo +
qo100-hi = hi −
qo100-hi-2 = hi +
qo100-radio-s-own-tuning-range = Het eigen afstembereik van deze radio reikt niet tot 10489,750 MHz — er is een LNB/converteroffset nodig (Instellingen ▸ Radio ▸ Converter) voordat dit venster iets kan doen.
qo100-open-settings-radio = Instellingen ▸ Radio openen
qo100-tune-10489-750-mhz = Afstemmen op 10489,750 MHz
qo100-tip-if-you-can-see = tip: als je het baken in de strook ziet maar het niet vastloopt, dubbelklik erop — dat markeert het als 10489,750 MHz en laat TOEPASSEN ernaar corrigeren
qo100-tracker = TRACKER
qo100-no-twin-lobe-shape-parking = geen tweelobbenvorm in het parkeervenster
qo100-shape = VORM
qo100-drift = DRIFT
qo100-need-longer-run-estimates = — (er is een langere reeks schattingen nodig)
qo100-cycles = CYCLI
qo100-receiver = ONTVANGER
qo100-target = DOEL
qo100-measured = GEMETEN
qo100-clicked = {"  "}(aangeklikt)
qo100-tracker-2 = {"  "}(tracker)
qo100-not-locked-yet = nog niet vergrendeld
qo100-converter-offset = CONVERTEROFFSET
qo100-decode = DECODEREN
qo100-decode-filling-first-frame-buffer = DECODEREN — de eerste framebuffer wordt gevuld…
qo100-decode-off-turn-telemetry-chip = DECODEREN — uit. Zet de chip TELEMETRIE (bovenste rij) aan voor de uitlezing draaggolf → synchronisatie → CRC.
qo100-apply-correction = {" "}CORRECTIE TOEPASSEN{" "}
qo100-write-converter-lnb-offset-puts = De converter/LNB-offset schrijven die het baken waar je klikte op 10489,750 MHz zet, en de ontvanger opnieuw openen — dezelfde korte onderbreking die Instellingen ▸ Radio ▸ Toepassen geeft
qo100-write-converter-lnb-offset-puts-2 = De converter/LNB-offset schrijven die de schatting van de tracker op 10489,750 MHz zet, en de ontvanger opnieuw openen — dezelfde korte onderbreking die Instellingen ▸ Radio ▸ Toepassen geeft
qo100-waiting-second-crc-valid-frame = Wachten op een tweede CRC-geldig frame voordat wordt aangeboden dit weg te schrijven — één vergrendeling alleen kan een toevallige match zijn
qo100-write-corrected-converter-lnb-offset = De gecorrigeerde converter/LNB-offset schrijven en de ontvanger opnieuw openen — een korte onderbreking, dezelfde die Instellingen ▸ Radio ▸ Toepassen geeft
qo100-decoder-runs-receiving-station-readout = De decoder draait op het ontvangststation — de uitlezing ervan wordt niet naar clients op afstand gestuurd
qo100-starting = starten…
qo100-searching-first-window-fills-after = zoeken — het eerste venster vult zich na ongeveer 24 s, en herhaalt dan

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Decodeer elke JS8-snelheid, niet alleen de snelheid waarop je zendt. De vier snelheden delen het subbereik en zijn vier verschillende golfvormen, dus zonder dit is een station op een andere snelheid onzichtbaar — en een uitwisseling tussen twee snelheden kan helemaal niet plaatsvinden. Kost ongeveer vier keer de ontvangst-CPU.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = Turbo bakent niet — het is de lokale en VHF-snelheid
js8-beacon-your-callsign-grid-every = Je roepletters en locatievak elke 15 minuten bakenen
js8-until-next-heartbeat = Tot de volgende hartslag
js8-setup = ⚙ INSTELLING
js8-tx = ● TX
js8-heard = GEHOORD
js8-nothing-heard-yet = — nog niets gehoord —
js8-dxcc = DXCC
js8-band = BAND
js8-grid = GRID
js8-dupe = DUBBEL
js8-reply = ANTWOORD
js8-no-messages = — geen berichten —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}NAAR WISSEN{" "}
js8-forget-selected-station-composer-goes = Het geselecteerde station vergeten — de opsteller gaat terug naar @ALLCALL
js8-already-addressing-allcall = Richt zich al op @ALLCALL
js8-stop = {" "}STOP{" "}
js8-send = {" "}VERSTUREN{" "}
js8-message = Bericht…

## logbook (src/app/logbook.rs)

palette-hint = Ga naar… een modus, een band, een venster, een instelling, een geheugen of een frequentie (14.074)
palette-nothing = niets met die naam
palette-group-window = Venster
palette-group-settings = Instellingen
palette-group-memory = Geheugen
palette-group-tune = Afstemmen
palette-tune-to = Afstemmen op { $mhz } MHz
palette-win-bands = Bandcondities
palette-win-awards = Prijzen
palette-win-windows = Vensterlijst
logbook-logbook = LOGBOEK
logbook-new-entry = + NIEUWE INVOER
logbook-adif = ADIF
logbook-import = IMPORTEREN
logbook-import-qsos-adif-adi-adif = QSO's importeren uit een ADIF-bestand (.adi of .adif)
logbook-new-qso = NIEUWE QSO
logbook-edit-qso = QSO BEWERKEN
logbook-call = Roepletters
logbook-lookup = OPZOEKEN
logbook-look-up-name-qth-grid = Naam / QTH / locatievak opzoeken
logbook-grid = Locatievak
logbook-freq-mhz = Freq MHz
logbook-mode = Modus
logbook-rst-sent = RST verzonden
logbook-rst-rcvd = RST ontvangen
logbook-name = Naam
logbook-state = Staat
logbook-country = Land
logbook-date-utc = Datum UTC
logbook-time = Tijd
logbook-pwr-w = Verm W
logbook-contest = Wedstrijd
logbook-s-sent = S# verzonden
logbook-s-rcvd = S# ontvangen
logbook-comment = Opmerking
logbook-save = {" "}OPSLAAN{" "}
logbook-cancel = ANNULEREN
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = kaart ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Deze invoer verwijderen
logbook-edit = BEWERKEN
logbook-upload-qso-configured-logs = Deze QSO uploaden naar de ingestelde logboeken
logbook-no-qsos-yet-run-ft8 = nog geen QSO's — draai FT8/FT4 of voeg een handmatige invoer toe

## scanner (src/app/scanner.rs)

scanner-scanner = Scanner
scanner-work-through-stored-memory-channels = De opgeslagen geheugenkanalen doorlopen
scanner-work-through-slice-band-channel = Een stuk van een band doorlopen op een kanaalraster
scanner-stop = STOP
scanner-start = START
scanner-from = Van
scanner-step = Stap
scanner-mode = Modus
scanner-range-empty-high-edge-has = Dat bereik is leeg — de bovenrand moet ten minste één kanaal boven de onderrand liggen.
scanner-skip-while-holding-pass-over = OVERSLAAN terwijl het vasthoudt om dat kanaal vanaf nu over te slaan
scanner-skipping = Overslaan
scanner-clear = WISSEN
scanner-stop-passing-over-any-them = Stoppen met een van hen over te slaan
scanner-stop-passing-over-channel = Stoppen met dit kanaal over te slaan
scanner-no-memory-channels-stored-yet = Nog geen geheugenkanalen opgeslagen — sla er een paar op in het venster MEM
scanner-skip-channel-pass-over = OVERSLAAN een kanaal om het over te slaan
scanner-skip = OVERSLAAN
scanner-look-all-channels-fall-inside = Zoek alle kanalen die binnen één ontvangstvenster vallen op dezelfde transformatie waarvan het panadapter gemaakt is, en stem alleen af op de kanalen waarop iets zit. Een lijst op één band kost dan één afstemming per ronde, hoe lang hij ook is, in plaats van een insteltijd per kanaal. De scan luistert nog steeds naar elke kandidaat voordat hij stopt, dus wat hem stopt is ongewijzigd — maar de sweep meet via de FFT in plaats van via het filter van de ontvanger, dus controleer de drempel als hij begint te stoppen op niets.
scanner-radio-hands-over-demodulated-audio = Deze radio levert gedemoduleerde audio en heeft geen eigen spectrum om in te zoeken, dus zijn geheugenscan bezoekt in elk geval elk kanaal.
scanner-fast = SNEL
scanner-read-list-off-spectrum-instead = de lijst van het spectrum aflezen in plaats van elk kanaal te bezoeken
scanner-folders = Mappen
scanner-scan-every-folder-every-folder = Elke map scannen, en elke map die vanaf nu gemaakt wordt. Kies mappen om alleen die te scannen.
scanner-unfiled = Niet ingedeeld
scanner-nothing-filed-under-folders-you = Er staat niets in de mappen die je koos.
scanner-stops = Stopt bij
scanner-use-receiver-s-own-squelch = De eigen squelch van de ontvanger gebruiken, zodat de scan precies stopt waar de audio zou openen
scanner-squelch-off-scan-will-stop = squelch staat uit — de scan stopt op het eerste waar hij naar kijkt
scanner-channel-power-signal-has-reach = Kanaalvermogen dat een signaal moet bereiken om de scan te stoppen
scanner-listens = Luistert naar
scanner-how-long-stay-candidate-before = Hoe lang op een kandidaat te blijven voordat hij beoordeeld wordt. Onder ongeveer een tiende seconde heeft de niveaumeter zich nog niet gezet en worden zwakke signalen gemist
scanner-resumes = Hervat
scanner-carry-once-signal-drops = Doorgaan zodra het signaal wegvalt
scanner-carry-after-fixed-time-regardless = Doorgaan na een vaste tijd, ongeacht
scanner-stay-until-you-press-next = Blijven tot je VOLGENDE indrukt
scanner-how-long-stay = Hoe lang blijven
scanner-how-long-wait-after-signal = Hoe lang wachten nadat het signaal wegvalt
scanner-move-don-t-stop-channel = Doorgaan, en op dit kanaal niet opnieuw stoppen
scanner-next = VOLGENDE
scanner-move-now = Nu doorgaan

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = de ADS-B-decoder starten…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Mode S
adsb-tune-receiver-1090-000-mhz = Stem de ontvanger af op 1090,000 MHz, de wereldwijde ADS-B-downlink. Er is maar één kanaal.
adsb-setup = INSTELLING
adsb-timeouts-trail-length-how-far = Time-outs, lengte van het spoor en hoe ver de snelheidsvectoren reiken
adsb-aircraft = VLIEGTUIGEN
adsb-nothing-heard-yet = nog niets gehoord
adsb-ground = op de grond
adsb-position-stale = positie verouderd
adsb-gnss-altitude = GNSS-hoogte
adsb-ground-speed = grondsnelheid
adsb-vertical-rate = verticale snelheid
adsb-first-heard = eerst gehoord
adsb-center = MIDDEN
adsb-put-aircraft-middle-map = Dit vliegtuig in het midden van de kaart zetten
adsb-last-frame-accepted-aircraft = Het laatste frame dat van dit vliegtuig is aangenomen
adsb-ads-b-setup = ADS-B-instelling
adsb-drop-map-after = Van de kaart halen na
adsb-without-position-report = zonder positiemelding
adsb-past-aircraft-comes-off-map = Daarna gaat het vliegtuig van de kaart en wordt de rij grijs. Hij wordt niet vervaagd: een dof vierkantje op een verouderde positie is nog steeds een bewering over waar een vliegtuig is, in dezelfde inkt als de ware.
adsb-drop-list-after = Van de lijst halen na
adsb-nothing-heard-all = waarvan helemaal niets gehoord is
adsb-trail-length = Lengte van het spoor
adsb-points = {" "}punten
adsb-history-dots-behind-each-target = geschiedenisstippen achter elk doel
adsb-speed-vector = Snelheidsvector
adsb-how-far-ahead-leader-line = hoe ver vooruit de leidingslijn reikt
adsb-one-minute-usual-radar-convention = Eén minuut is de gebruikelijke radarconventie: de lijn is precies zo lang als de afstand die het vliegtuig in die tijd aflegt, dus twee gelijke leidingen zijn twee gelijke snelheden bij elke zoom. Nul zet ze uit.
adsb-track-most = Maximaal volgen
adsb-aircraft-2 = {" "}vliegtuigen
adsb-aircraft-ground-are-placed-against = Vliegtuigen op de grond worden geplaatst ten opzichte van de eigen positie van het station — een grondsquitter heeft geen eenduidige decodering van zichzelf — dus vul Mijn locatievak in de digimode-instelling in als de nabije luchthaven niets toont.
adsb-call = ROEP
adsb-icao = ICAO
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = wachten op de ontvanger…
drm-not-drm-set-mode-drm = Niet in DRM. Zet de modus op DRM op een digitale kortegolfuitzending — de dial gaat op het midden van het kanaal, niet op een zijband.
drm-no-drm-signal-locked-decoder = Geen DRM-signaal vergrendeld. De decoder heeft een paar seconden nodig op een schone draaggolf: DRM interleavet over 400 ms of 2 s voordat er iets van gelezen kan worden.
drm-station-s-audio-codec-not = De audiocodec van dit station decodeert hier niet. xHE-AAC heeft libfdk-aac op het systeem nodig — het kan niet ingebouwd worden, omdat de licentie daarvan en die van dit programma niet samengaan. Installeer het (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll naast sdroxide.exe) en start opnieuw. Al het andere op dit scherm decodeert gewoon.
drm-quality = KWALITEIT
drm-last-minute = laatste minuut
drm-waiting-locked-signal = wachten op een vergrendeld signaal
drm-audio-frames-decoding-but-station = Audioframes worden gedecodeerd — maar de codec van dit station kan hier niet gedecodeerd worden
drm-audio-frames-decoding = Audioframes worden gedecodeerd
drm-samples-reaching-decoder = Samples die de decoder bereiken
drm-time = TIJD
drm-symbol-timing-recovered = Symboolzeitgeving hersteld
drm-frame = FRAME
drm-transmission-frames-found = Gevonden zendingsframes
drm-fast-access-channel-what-transmission = Fast Access Channel — wat de zending is
drm-service-description-channel-what-services = Service Description Channel — wat de diensten zijn
drm-audio = AUDIO
drm-constellation = CONSTELLATIE
drm-waiting-decoded-symbols = wachten op gedecodeerde symbolen
drm-sampled-across-frame = over het frame bemonsterd
drm-whole-frame = heel frame
drm-mode = MODUS
drm-robustness-mode-channel-width-ground = Robuustheidsmodus en kanaalbreedte. A is voor een grondgolfpad en draagt het meest; D is voor een sterk verstrooid skywavepad en draagt het minst.
drm-interleave = INTERLEAVE
drm-how-far-transmission-spreads-each = Hoe ver de zending elk frame in de tijd spreidt. Lang overleeft diepere vervagingen en duurt dienovereenkomstig langer om te vergrendelen.
drm-protection = BESCHERMING
drm-offset = OFFSET
drm-residual-sample-clock-error-against = Resterende kloonfout van de samples ten opzichte van de zender. Groot en stabiel betekent dat de referentie van de ontvanger afwijkt, niet de uitzending.
drm-doppler = DOPPLER
drm-delay = VERTRAGING
drm-doppler-delay-spread-path-how = Doppler en vertragingsspreiding van het pad — hoe snel het beweegt en hoe ver zijn echo's uit elkaar aankomen.
drm-service = DIENST
drm-decode-service-multiplex = Deze dienst van de multiplex decoderen

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = het AtCHAT NET-station starten…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — toetsenbord en bestand voor meerdere stations
atchat-net = {" "}OP HET NET{" "}
atchat-net-click-leave-state-kept = Op het net — klik om te vertrekken (de toestand blijft bewaard; klik opnieuw om weer aan te sluiten)
atchat-rejoin = {" "}OPNIEUW AANSLUITEN{" "}
atchat-off-net-click-rejoin = Niet op het net — klik om weer aan te sluiten
atchat-off-net = niet op het net
atchat-carrier = ● DRAAGGOLF
atchat-another-station-transmitting-channel-busy = Een ander station zendt — het kanaal is bezet.
atchat-tx = ● TX
atchat-setup = ⚙ INSTELLING
atchat-station-callsign-virtual-channel-address = Roepletters van het station en het adres van het virtuele kanaal
atchat-virtual-channel = VIRTUEEL KANAAL
atchat-work-net-over-tcp-endpoint = Het net werken via een TCP-eindpunt in plaats van de radio — een channel_server-compatibele loopback voor ontwikkeling zonder radio. Uit zet het station op de lucht.
atchat-roster = LEDENLIJST
atchat-nobody-heard-yet = nog niemand gehoord
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = Het gemeenschappelijke kanaal — regels hier gaan naar iedereen (ALLEN)
atchat-log = {" "}LOG{" "}
atchat-station-s-own-air-activity = De eigen activiteit van het station op de lucht — verkiezing van de master, veroudering van de ledenlijst, ARQ-herhalingen
atchat-no-activity-logged-yet = Nog geen activiteit gelogd.
atchat-no-messages-yet = Nog geen berichten.
atchat-message-everyone-all = bericht aan iedereen (ALLEN)
atchat-send = {" "}VERSTUREN{" "}
atchat-transfers = OVERDRACHTEN
atchat-all = → ALLEN
atchat-sent-file-follows-active-chat = Een verzonden bestand volgt het actieve chattabblad
atchat-send-file = {" "}BESTAND VERSTUREN{" "}
atchat-send-file-image-over-air = Een bestand of beeld via de lucht versturen, met blok-CRC-ARQ
atchat-nothing-flight = niets onderweg
atchat-images = BEELDEN
atchat-no-images-received = Geen beelden ontvangen.


## rds (src/app/rds.rs)

rds-station = ZENDER
rds-diagnostics = DIAGNOSE
rds-which-programme-type-table-read = Aan welke tabel van programmatypes dit station gelezen wordt. RDS wordt overal buiten Noord-Amerika gebruikt; RBDS voegt de roepletters toe die de identiteitscode van een Amerikaans station spelt. Auto volgt de uitgebreide landcode die het station stuurt, en tot die binnenkomt spelt het geen roepletters — de identiteitsbereiken die het zou lezen, worden gedeeld met de RDS-codes van negen landen, dus kies RBDS om een station te benoemen dat geen landcode stuurt.
rds-waiting-receiver = wachten op de ontvanger…
rds-no-rds-station-not-every = Geen RDS op dit station. Niet elke zender draagt het, en een zwakke kan het dragen zonder dat het aankomt.
rds-rds-only-carried-wfm-broadcast = RDS wordt alleen gedragen door WFM-omroepstations.
rds-holding = VASTHOUDEN
rds-block-sync-has-dropped-last = De blokssynchronisatie is weggevallen — dit is het laatst gelezen station.
rds-identity = IDENTITEIT
rds-programme = PROGRAMMA
rds-described = OMSCHREVEN ALS
rds-content = INHOUD
rds-music = Muziek
rds-speech = Spraak
rds-traffic = VERKEER
rds-announcement-air-now = aankondiging nu op de lucht
rds-carried-station = gedragen door dit station
rds-not-carried = niet gedragen
rds-radio-text = RADIOTEKST
rds-station-clock = STATIONSKLOK
rds-country-code = LANDCODE
rds-also = OOK OP
rds-also-tagged = OOK GETAGD
rds-sync = SYNC
rds-no-sync = GEEN SYNC
rds-blocks-failed-their-check-plus = Blokken die hun controle niet haalden, plus die de foutcorrector moest herstellen. Correcties tellen bewust als fouten: een oplopend getal is het eerste teken dat een station op het punt staat niet meer te decoderen.
rds-front-end-overloading = FRONT-END OVERBELAST
rds-converter-clipping-which-puts-distortion = De converter klipt, wat vervorming over de hele multiplex legt — ook op de 57 kHz-datasubdraaggolf. Draai de RF-gain omlaag, of schakel een verzwakker in: RDS gaat ongeveer een decibel voordat er iets hoorbaar is, dus een station dat perfect klinkt, kan nog steeds veel te sterk zijn om zijn data te laten overleven.
rds-group-types = GROEPSTYPES
rds-recent-groups-newest-last = RECENTE GROEPEN — nieuwste onderaan
rds-nothing-decoded-yet = nog niets gedecodeerd

## winlink (src/app/winlink.rs)

winlink-mail = MAIL
winlink-abort = AFBREKEN
winlink-stop-session-progress = De lopende sessie stoppen
winlink-connect = VERBINDEN
winlink-compose = OPSTELLEN
winlink-connecting = verbinden…
winlink-not-connected-yet = nog niet verbonden
winlink-loading = laden…
winlink-date = DATUM
winlink-from = VAN
winlink-subject = ONDERWERP
winlink-no-subject = (geen onderwerp)
winlink-delete-message = Dit bericht verwijderen
winlink-arch = ARCH
winlink-move-archive = Naar het archief verplaatsen
winlink-select-message = kies een bericht
winlink-reply = ANTWOORDEN
winlink-delete = VERWIJDEREN
winlink-to = Aan
winlink-cc = Cc
winlink-subject-2 = Onderwerp
winlink-callsign-smtp-someone-example-org = Een roepletter, of SMTP:iemand@example.org voor internetmail. Scheid er meerdere met komma's.
winlink-file-outbox = IN UITBAK ZETTEN
winlink-discard = WEGGOOIEN
winlink-filed-messages-go-out-next = berichten in de uitbak gaan bij de volgende sessie uit
winlink-mail-log = MAILLOGBOEK
winlink-session-running = sessie loopt…
winlink-no-session-yet = nog geen sessie

## wspr (src/app/panels/wspr.rs)

wspr-receptions = ONTVANGSTEN
wspr-save-wspr-reception-list-csv = De WSPR-ontvangstlijst als CSV opslaan
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = propagatiebaken voor zwakke signalen
wspr-time-left-two-minute-slot = Tijd over in dit tijdslot van twee minuten. Aan het eind ervan gaat de opname naar de decoder en valt de volgende zendbeslissing.
wspr-tx = ● TX
wspr-decoding = decoderen…
wspr-band = Band
wspr-last-slot = Laatste slot
wspr-nothing-heard = niets gehoord
wspr-receive-only = alleen ontvangen
wspr-next-slot = Volgend slot
wspr-waiting-radio-take-change = wachten tot de radio de wijziging overneemt…
wspr-hop-next = Volgende springen
wspr-transmit = ZENDEN
wspr-receive-only-2 = Alleen ontvangen.
wspr-power = VERMOGEN
wspr-what-you-actually-radiate = wat je werkelijk uitstraalt
wspr-goes-out-message-everyone-who = Dit gaat uit in het bericht, en iedereen die je hoort, beoordeelt het pad daarop — dus een optimistisch getal hier maakt hun metingen fout, net als die van jou.
wspr-roam = ROND ZWERVEN
wspr-pick-different-offset-inside-200 = Kies voor elke zending een andere offset binnen het venster van 200 Hz. Dit is de conventie: tweehonderd hertz die door iedereen gedeeld wordt, werkt alleen als niemand midden erin gaat staan. Uit houdt vast waar de cursor staat.
wspr-band-hop = BANDSPRING
wspr-move-dial-band-band-between = De dial tussen de slots van band naar band verplaatsen, zodat één ontvanger het hele spectrum bemonstert in plaats van één stuk ervan. Zelf aan de VFO draaien pauzeert het — druk dit twee keer om te hervatten.
wspr-upload = UPLOADEN
wspr-send-what-station-decodes-wsprnet = Stuur wat dit station decodeert naar wsprnet.org. Melden wat je hoort is wat een WSPR-ontvanger deel van het netwerk maakt; het zet niets op de lucht.
wspr-who-heard-me = WIE HOORDE MIJ
wspr-ask-wsprnet-org-every-few = Vraag elke paar minuten aan wsprnet.org welke stations dit station gedecodeerd hebben. WSPR kent geen bevestiging van welke aard ook, dus dit is de enige manier waarop een baken iets over zijn bereik te weten komt.
wspr-bands = BANDEN
wspr-none-selected-hopping-has-nowhere = niets geselecteerd — het springen heeft nergens heen
wspr-set-your-callsign-grid-general = Stel je roepletters en locatievak in op het tabblad Algemeen van Instellingen voordat je zendt.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = BEZET
packet-frames-arrived-but-failed-their = Frames die binnenkwamen maar hun controlereeks niet haalden — een botsing, een vervaging, of een signaal dat te zwak is om te lezen.
packet-setup = ⚙ INSTELLING
packet-station-callsign-speed-tx-delay = Roepletters van het station, snelheid, zendvertraging, het digipeaterpad, het baken en de KISS-server
packet-starting-packet-modem = de pakketmodem starten…
packet-nothing-heard-yet = nog niets gehoord
packet-terminal = TERMINAL
packet-connected = {" "}VERBONDEN{" "}
packet-calling = {" "}ROEPEN{" "}
packet-idle = {" "}INACTIEF{" "}
packet-extended-sequence-numbers-window-up = Uitgebreide volgnummers — een venster van maximaal 127 frames.
packet-frames-sent-not-yet-acknowledged = Frames die verstuurd zijn en nog niet door het verre eind bevestigd.
packet-retries-against-link-s-limit = Herhalingen tegen de grens van deze verbinding. Een teller die stijgt terwijl de onbevestigde frames blijven staan, ziet er van deze kant zo uit als een vervagend pad.
packet-mail-window-using-link = Het venster MAIL gebruikt de verbinding.
packet-one-radio-one-channel-one = Eén radio, één kanaal, één verbinding. Een Winlink-sessie en deze terminal zijn twee manieren om het te gebruiken, en wie het eerst vraagt, krijgt het.
packet-digipeaters-nearest-first-separated-commas = Digipeaters, de dichtstbijzijnde eerst, gescheiden door komma's — OE3XLR-1,OE3XMS-1. Laat het leeg voor een station dat je rechtstreeks hoort, of om het standaardpad uit de pakketinstellingen te gebruiken.
packet-disconnect = {" "}VERBREKEN{" "}
packet-hang-up-properly-wait-far = Netjes ophangen, en wachten tot het verre eind het ermee eens is. Van modus wisselen beëindigt de sessie in plaats daarvan zonder iemand te vertellen.
packet-connect = {" "}VERBINDEN{" "}
packet-mail-window-has-link-finish = Het venster MAIL heeft de verbinding. Rond die sessie eerst af of stop hem.
packet-station-has-no-callsign-yet = Dit station heeft nog geen roepletters — stel die in onder INSTELLING, boven het monitorpaneel. Er wordt niets verzonden tot die is ingesteld.
packet-call-station-connected-mode-node = Roep dit station op in de verbonden modus — een knooppunt, een BBS, of een andere operator.
packet-needs-callsign-call = Heeft een roepletter nodig om op te roepen.
packet-clear = {" "}WISSEN{" "}
packet-empty-transcript-link-untouched-connected = Het transcript leegmaken. De verbinding blijft ongemoeid — een verbonden station blijft verbonden.
packet-not-connected-type-callsign-above = Niet verbonden. Typ hierboven een roepletter en druk op VERBINDEN om een knooppunt of een BBS op te roepen.
packet-type-here = typ hier
packet-not-connected = niet verbonden
packet-send = {" "}VERSTUREN{" "}
packet-send-line-node-bbs-reads = De regel versturen. Een knooppunt of een BBS leest één regel tegelijk.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Snelheid
fsq-call = Roep
fsq-callsign-directed-fsqcall-messages-defaults = Roepletters voor gerichte (FSQCALL) berichten; staat standaard op je eigen roepletters
fsq-tx = ● TX
fsq-contacts = CONTACTEN
fsq-heard = GEHOORD
fsq-none = — geen —
fsq-allcall = ALLCALL
fsq-image = BEELD
fsq-clear = WISSEN
fsq-forget-pictures-received-so-far = De tot nu toe ontvangen beelden vergeten
fsq-send-image = Beeld versturen…
fsq-right-click-remove = Rechtsklik om te verwijderen
fsq-remove-picture = Dit beeld verwijderen
fsq-listening = — luisteren —
fsq-message = Bericht…
fsq-send = {" "}VERSTUREN{" "}
fsq-call-cq = {" "}CQ ROEPEN{" "}
fsq-heard-2 = {" "}? gehoord{" "}
fsq-clear-2 = {" "}WISSEN{" "}
fsq-fsq-contacts = FSQ-contacten
fsq-add = Toevoegen

## publicsdr (src/app/publicsdr.rs)

publicsdr-not-stated = niet opgegeven
publicsdr-public-sdrs = PUBLIEKE SDR'S
publicsdr-available = BESCHIKBAAR
publicsdr-hide-receivers-are-full-ones = Ontvangers verbergen die vol zijn, en die waarvan de operator geen kanalen naar andere apps dan een browser heeft opengezet
publicsdr-band = IN BAND
publicsdr-only-receivers-cover-current-dial = Alleen ontvangers die de huidige dialfrequentie bestrijken
publicsdr-low-bw = LAGE BW
publicsdr-map-cities = STEDEN
publicsdr-map-cities-tip = Steden op deze kaart, aan of uit.
publicsdr-view-list = LIJST
publicsdr-view-map = KAART
publicsdr-view-map-tip = Dezelfde ontvangers op een wereldkaart. Klik op een punt om alle ontvangers eronder te kiezen.
publicsdr-map-pick = Klik op een punt om de ontvangers eronder te kiezen.
publicsdr-map-ghost = { $name }: nu niet in de lijst — hier voor het laatst gezien op { $seen }
publicsdr-map-names = NAMEN
publicsdr-map-names-tip = Toon de namen van de ontvangers op de kaart. Uit: alleen de gekozen ontvangers krijgen een naam.
publicsdr-check-online = ONLINE
publicsdr-check-silent = STIL
publicsdr-check-waiting = CONTROLE…
publicsdr-check-no-answer = Geen antwoord binnen 20 seconden.
publicsdr-take-spyserver-low-bandwidth-shape = Een SpyServer in zijn vorm met lage bandbreedte nemen: een smal I/Q-venster dat de dial volgt plus de eigen bandweergave van de server, in plaats van megabits aan breedband-I/Q. Geen effect op een KiwiSDR, die maar de ene vorm heeft.
publicsdr-refresh = ⟳ VERNIEUWEN
publicsdr-fetch-both-directories-again = Beide mappen opnieuw ophalen
publicsdr-name-place-antenna-band = naam, plaats, antenne, band
publicsdr-clear-search = Het zoeken wissen
publicsdr-fetching-receiver-lists = De ontvangerlijsten ophalen…
publicsdr-machine-radio-attached-does-not = De machine waarop deze radio is aangesloten, beantwoordt geen vragen over apparaten, dus kan hij de lijsten ook niet ophalen.
publicsdr-no-match = geen overeenkomst
publicsdr-nothing-matches-try-turning-filter = niets past — probeer een filter weer aan te zetten

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = wachten op de ontvanger…
hd-not-hd-radio-set-mode = Niet in HD Radio. Zet de modus op HD Radio op een FM-omroep die de digitale zijbanden draagt — de dial gaat op het midden van de analoge draaggolf, niet ernaast.
hd-no-hd-radio-lock-digital = Geen HD Radio-vergrendeling. De digitale zijbanden worden ongeveer 20 dB onder de analoge draaggolf uitgezonden, dus een station dat hoorbaar is, kan nog te ruisig zijn om te decoderen: de decoder heeft een paar seconden nodig op een schoon exemplaar.
hd-sync = SYNC
hd-ofdm-frame-timing-recovered = OFDM-frametiming hersteld
hd-audio = AUDIO
hd-audio-frames-decoding = Audioframes worden gedecodeerd
hd-quality = KWALITEIT
hd-lower = ONDER
hd-upper = BOVEN
hd-last-minute = laatste minuut
hd-waiting-locked-signal = wachten op een vergrendeld signaal
hd-mer-l = MER L
hd-modulation-error-ratio-lower-digital = Modulatiefoutverhouding van de onderste digitale zijband.
hd-mer-u = MER U
hd-modulation-error-ratio-upper-digital = Modulatiefoutverhouding van de bovenste digitale zijband.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Kanaalbitfoutverhouding na de binnencode. Onder ongeveer 1e-3 kan de buitencode het corrigeren; erboven begint de audio te breken.
hd-offset = OFFSET
hd-residual-carrier-frequency-offset-large = Resterende draaggolffrequentieoffset. Groot en stabiel betekent dat de referentie van de ontvanger afwijkt, niet de uitzending.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Primary Service Mode Indicator: welke van de FM-hybride dienstmodi het station uitzendt.
hd-programme = PROGRAMMA
hd-restricted-access = {" "}— beperkte toegang

## bands (src/app/bands.rs)

bands-bands = BANDEN
bands-daylight = ☀ DAG
bands-night = ☾ NACHT
bands-no-forecast-yet-fetching-hamqsl = · nog geen verwachting — ophalen bij HAMQSL.com
bands-forecast-download-off = · downloaden van de verwachting staat uit (Instellingen → UI)
bands-band = BAND
bands-condx = CONDX
bands-paths = PADEN
bands-reach = BEREIK
bands-best = BESTE
bands-condx-global-forecast-rest-what = CONDX is een wereldwijde verwachting; de rest is wat werkelijk gehoord is. Een lege rij betekent dat er niets gedecodeerd is — dat kan betekenen dat de band dicht zat, of alleen dat niemand erop zat.
bands-ibp-beacons = IBP-BAKENS
bands-meteor-showers = METEORENREGENS
bands-peak = PIEK

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTS
spots-longwave-shortwave-broadcast-stations-air = Lange- en kortegolf-omroepstations die nu op de lucht zijn
spots-view = IN BEELD
spots-only-spots-inside-panadapter-span = Alleen spots binnen het bereik van het panadapter
spots-setup = ⚙ INSTELLING
spots-feeds-lookup-upload-settings = Feeds, opzoeken en uploadinstellingen
spots-call-station-site-frequency = roepletters, station, locatie, frequentie
spots-clear-search = Het zoeken wissen
spots-no-match = geen overeenkomst
spots-no-spots-enable-feed-setup = geen spots — schakel een feed in via ⚙ INSTELLING
spots-nothing-matches-search = niets past bij het zoeken

## ism (src/app/ism.rs)

ism-ism-devices = ISM-APPARATEN
ism-decoding = DECODEREN
ism-needs-wideband-iq-source-cat = Vereist een breedband-IQ-bron; een CAT-radio stuurt alleen audio
ism-how-far-burst-must-stand = Hoe ver een burst boven de eigen ruisvloer van het kanaal moet uitsteken voordat hij gedecodeerd wordt
ism-give-each-band-width-normally = Geef elke band de breedte die hij normaal nodig heeft — een kwart megahertz voor de OOK-banden op 315, 345 en 433 MHz, een volle voor 868 en 915.
ism-several-hundred-more-device-decoders = Enkele honderden extra apparaatdecoders, uit het project rtl_433, ingebouwd. Leest de 433 MHz-OOK-apparaten die de eigen decoders van sdroxide helemaal niet bereiken. Waar beide een apparaat kunnen lezen, kent rtl_433 veel meer varianten en neemt het het over.
ism-t-1-your-own = 1 van jezelf
ism-device-decoders-registered-how-many = Geregistreerde apparaatdecoders, hoeveel er uit je rtl433_flex.conf komen, en hoeveel zendingen er sinds de start gedecodeerd zijn.
ism-reload-decoders = DECODERS OPNIEUW LADEN
ism-re-read-rtl433-flex-conf = rtl433_flex.conf opnieuw inlezen na het bewerken. De apparaten die al gehoord zijn, blijven in de lijst.
ism-rtl-433-knows-more-variants = rtl_433 kent meer varianten hiervan dan de eigen decoders van sdroxide, dus het leest ze terwijl het hier luistert. Kies een andere band en de eigen decoders van sdroxide nemen het weer over.
ism-decoder-your-rtl433-flex-conf = Een decoder in je rtl433_flex.conf is geweigerd. De andere zijn wel geladen. Repareer de regel, en druk daarna op DECODERS OPNIEUW LADEN.
ism-waiting-decoder = wachten op de decoder…
ism-where-decoder-actually-listening-follows = Waar de decoder werkelijk luistert. Dit volgt het hardwarecentrum van de ontvanger, niet de VFO — op een breedband-front-end kunnen de twee ver uit elkaar liggen, en het is het venster dat bepaalt welke kanalen bereikbaar zijn.
ism-transmissions-gate-opened-how-many = Zendingen waarop de poort openging, en hoeveel er een geldig frame opleverden. Veel bursts en geen decoderingen betekent dat de band bezet is met apparaten die sdroxide nog niet kan lezen. Helemaal niets betekent dat niets de drempel bereikt — verlaag hem, of controleer of de dial op de band staat.
ism-nothing-heard-yet = nog niets gehoord
ism-click-tune-receiver =
    {""}

    Klik om de ontvanger erop af te stemmen

## app (src/app/mod.rs)

app-hpsdr = HPSDR
app-rtl-sdr = RTL-SDR
app-rtl-sdr-rtl-tcp = RTL-SDR (rtl_tcp)
app-spyserver-vfo = SpyServer (VFO)
app-airspy-hf = Airspy HF+
app-sdrplay = SDRplay
app-elad = ELAD
app-airspy = Airspy
app-sxceiver = SXceiver
app-usrp = USRP

## navtex (src/app/panels/navtex.rs)

navtex-messages = BERICHTEN
navtex-sync = SYNC
navtex-character-phase-locked-signal-being = De tekenfase is vergrendeld — er wordt een signaal gelezen.
navtex-no-character-phase-yet-no = Nog geen tekenfase: geen signaal, of niet deze.
navtex-characters-whose-first-copy-was = Tekens waarvan de eerste kopie beschadigd was en die uit de herhaling vijf slots later genomen zijn, en die waarbij beide slecht waren. Een NAVTEX-teken wordt twee keer verzonden; dat is het hele foutherstel.
navtex-swap-mark-space-tones-signal = De markering- en spatietonen omwisselen, voor een signaal dat op de andere zijband ontvangen wordt. Uit is de bovenste zijband op de kanaalfrequentie, wat elke gepubliceerde afstemmingsinstructie voor de dienst zegt.
navtex-starting-navtex-receiver = de NAVTEX-ontvanger starten…
navtex-nothing-yet-station-transmits-ten = nog niets — een station zendt tien minuten uit elke vier uur
navtex-cut-short = afgebroken
navtex-message = BERICHT
navtex-time-day-stated-message-utc = Tijdstip van de dag dat in het bericht staat (UTC): alleen weergave
navtex-receiving = ONTVANGEN
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = alles gedecodeerd, bericht of niet

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = RF-TEKENEN
rfpaint-scan-speed = Scansnelheid
rfpaint-how-fast-text-image-scanned = Hoe snel de tekst/het beeld op de waterval gescand wordt. Lager is trager en beter leesbaar; 100% = basissnelheid, 25% (midden) is de standaard.
rfpaint-abort = Afbreken
rfpaint-text-paint = TEKST TEKENEN
rfpaint-type-text-paint = Typ tekst om te tekenen…
rfpaint-preview-waterfall = VOORBEELD WATERVAL
rfpaint-type-text-preview = typ tekst voor het voorbeeld
rfpaint-transmit = {"  "}ZENDEN{"  "}
rfpaint-image-paint = BEELD TEKENEN
rfpaint-no-image-loaded = geen beeld geladen
rfpaint-load-image = Beeld laden…
rfpaint-load-image-preview = laad een beeld om het voorbeeld te zien

## hfdl (src/app/hfdl.rs)

hfdl-listen = LUISTEREN
hfdl-switches-engine-s-hfdl-downconverter = Zet de HFDL-downconverter en -decoder van de engine aan of uit. Decoderen kost een baan van 24 kHz en een werkthread, of het venster open is of niet.
hfdl-channel-assigned-frequency-listen-anything = Het kanaal (de toegewezen frequentie) waarop geluisterd wordt. Alles in de band werkt — HFDL-stations zenden op 2,8 tot 22 MHz — tegen een vaste baan van 24 kHz gecentreerd hier.
hfdl-save-hfdl-decode-log-file = Het HFDL-decodeerlogboek naar een bestand opslaan
hfdl-riverhead-northern-atlantic-north-america = Riverhead (noordelijke Atlantische Oceaan / Noord-Amerika) — de standaard, en het kanaal waarop de decoder gevalideerd is.
hfdl-hfdl-assigned-frequency = Een toegewezen HFDL-frequentie.
hfdl-one-your-saved-channels-click = Een van je opgeslagen kanalen — klik om af te stemmen, rechtsklik om te verwijderen.
hfdl-hfdl-selected-but-decoding-off = HFDL is gekozen maar decoderen staat uit — druk hierboven op LUISTEREN
hfdl-decodes = DECODERINGEN
hfdl-nothing-decoded-yet-ground-station = Nog niets gedecodeerd — de squitter van een grondstation herhaalt elke ~32 s zodra er één in het kanaal is.
hfdl-no-decode-matches-filter = Geen decodering past bij het filter.
hfdl-decoding = DECODEREN
hfdl-decoding-off = DECODEREN UIT
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = FreeDV V1 digitale stem
rade-tx = ● TX
rade-mute-analog = ANALOOG DEMPEN
rade-mute-demodulated-audio-so-only = Het gedemoduleerde geluid dempen, zodat alleen gedecodeerde spraak te horen is. Het ruwe signaal wordt anders doorgelaten zodra de modem niets heeft om af te spelen — dat gesis is hoe je een zending vindt voordat die synchroniseert, dus laat dit uit tijdens het afstemmen.
rade-sync = SYNC
rade-how-far-received-signal-sits = Hoe ver het ontvangen signaal ligt van waar de modem het verwacht. Grote waarden decoderen nog steeds — de acquisitielus volgt ze — maar de dial een beetje bijstellen tot dit dichtbij nul komt, geeft de beste marge.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    De roepletters die het station in zijn End-of-Over-frame zette — het enige moment in een RADE-zending waarop er een wordt verstuurd. Het is ook wat aan FreeDV Reporter gemeld wordt, dus een station dat je hoort, verschijnt op qso.freedv.org als gehoord door jou.

    Een minuut vastgehouden nadat het signaal weg is, en laten vallen wanneer je zelf een zending begint: het noemt het station op de frequentie, niet elk station dat de ontvanger ooit heeft gehoord.
rade-stop-talking = STOP MET PRATEN
rade-talk-ptt = PRATEN (PTT)
rade-open-close-rade-over-modem = Een RADE-zending openen of sluiten. De modem heeft ~120 ms spraak nodig voordat het eerste frame uitgaat, en stuurt een end-of-over-frame wanneer je stopt, dus de zending loopt een klein stukje door na de knop.
rade-rade-v1-occupies-roughly-1060 = RADE V1 beslaat ongeveer 1060–1880 Hz van de doorlaatband — bovenste zijband, of onderste op 160, 80 en 40 m, waar RADE de spraakpraktijk houdt en sdroxide voor je schakelt. Zet het signaal binnen de gearceerde band op de waterval; de modem vindt het vanaf daar.

## frame (src/app/frame.rs)

frame-acknowledge = Bevestigen
frame-dismiss = Sluiten
frame-get-sdroxide-com = Haal het op sdroxide.com
frame-reconnecting = Opnieuw verbinden…
frame-not-connected = Niet verbonden.
frame-reconnect-now = Nu opnieuw verbinden
frame-transmit-lockout-disabled = ⚠ ZENDVERGRENDELING UITGESCHAKELD
frame-engine-was-started-oob-tx = Deze engine is gestart met --oob-tx. De vergrendeling op amateurbanden staat uit: hij stuurt de zender op elke frequentie die de hardware ondersteunt.
frame-transmitting-outside-your-licence-offence = Zenden buiten je vergunning is in elk land dat er een uitgeeft een strafbaar feit. Ga alleen door als je gemachtigd bent om de frequenties te gebruiken waarop je op het punt staat te sturen — een MARS/CAP- of commerciële vergunning, een experimentele vergunning, of een dummyload.
frame-i-understand = {"  "}IK BEGRIJP HET{"  "}
frame-restart-without-oob-tx-put = Herstart zonder --oob-tx om de vergrendeling terug te zetten.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — hoe hard de zender wordt aangestuurd, als percentage van het meeste wat hij kan afgeven. 100 % is volle schaal, niet volledig vermogen in watt: wat dat aan de antenne waard is, hangt af van de radio en van Aansturing.
smeter-radio-reports-no-forward-power =
    {""}

    Deze radio meldt geen voorwaarts vermogen, dus het getal naast ALC is opnieuw de ALC-aflezing in plaats van watt.
smeter-no-swr-radio-has-no =
    {""}

    Geen SWR: deze radio heeft geen brug om het mee te meten, en sdroxide verzint er geen — het kan alleen tonen wat de radio zelf meldt. Een radio met een SWR-meter (via CAT, TCI of een LAN-verbinding) en een HPSDR-bord vullen dit allebei in; een gewone SDR-zender kan dat niet.
smeter-no-signal-report-yet-receiver = Nog geen signaalrapport — de ontvanger heeft nog geen meteraflezing geleverd.
smeter-s9-73-dbm-each-s =
    {""}

    S9 is −73 dBm en elke S-eenheid eronder is 6 dB. Het dBm-getal is maar zo eerlijk als de kalibratie van het front-end — een ongekalibreerde ontvanger meldt dBFS met een dBm-label.
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD: de converter zit tegen zijn grenzen, dus het getoonde niveau onderschat het signaal. Draai de front-endgain terug.
smeter-table-held = , tabel vastgehouden
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-mute-radio-s-audio = De audio van deze radio dempen
multi-show-radio-here = Deze radio hier tonen
multi-show-on-map = Op de kaart tonen
multi-add-radio-computer-settings-radio = Een radio op deze computer toevoegen (Instellingen → Radio om er één bij een station toe te voegen)
multi-add-radio = Een radio toevoegen
multi-take-radio-off-row = Deze radio uit deze rij halen. Hij blijft draaien; de ringen schakelen hem uit.
multi-open-pane = Een volgend paneel openen met een radio die niet in beeld is
multi-pick-radio = Radio ▼
multi-pick-radio-tip = Een radio in deze rij zetten. De ringen markeren radio's waarvan de verbinding open is.
multi-new-radio = Nieuwe radio…
multi-asked-station-another-radio = Het station om nog een radio gevraagd…
multi-client-cannot-open-connection = Deze client kan geen verbinding openen.

## net (src/app/net.rs)

net-fail = MISLUKT
net-will-retry = {" "}(probeert opnieuw)
net-full-resync-downloading = Volledige resynchronisatie: downloaden…
net-downloading = Downloaden…
net-full-resync-log-already-matches = Volledige resynchronisatie: het logboek komt al overeen met Wavelog
net-full-resync-review-changes-below = Volledige resynchronisatie: bekijk de wijzigingen hieronder
net-full-resync-discarded-nothing-changed = Volledige resynchronisatie weggegooid; er is niets veranderd
net-adif-import-failed-file-could = ADIF-import mislukt: het bestand kon niet worden gelezen
net-channel-import-failed-file-could = Kanaalimport mislukt: het bestand kon niet worden gelezen
net-channel-import-nothing-read-chirp = Kanaalimport: niets om te lezen — een CHIRP-CSV-bestand begint met een kopregel die de kolommen noemt, waarvan één Frequency moet zijn

## widgets (src/app/panels/widgets.rs)

widgets-free-text = vrije tekst
widgets-hashed-callsign-not-yet-heard = gehashte roepletters, nog niet volledig gehoord
widgets-entity-unknown = entiteit onbekend
widgets-band = deze band
widgets-new-entity-never-worked-any = Nieuwe entiteit — nooit gewerkt, op geen enkele band
widgets-new-grid-square = Nieuw locatievak
widgets-not-worked-before = Eerder niet gewerkt
widgets-worked-before-but-not-band = Eerder gewerkt, maar niet op deze band
widgets-call-queue = In de oproepwachtrij
widgets-image = Beeld

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = ONTVANGSTEN
pi4-save-pi4-reception-list-csv = De PI4-ontvangstlijst als CSV opslaan
pi4-next-generation-beacon = Next Generation Beacon
pi4-time-left-one-minute-beacon = Tijd over in deze cyclus van het baken van één minuut. Het PI4-bericht wordt gezocht zodra het venster tijd heeft gehad om voorbij het punt waar het zou eindigen te vullen.
pi4-decoding = decoderen…
pi4-band = Band
pi4-last-cycle = Laatste cyclus
pi4-nothing-heard = niets gehoord
pi4-receive-only-tune-so-beacon = Alleen ontvangen. Stem zo af dat de CW-identificatie van het baken op 800 Hz audio zit — de eigen conventie van het netwerk — en de vier PI4-tonen vallen waar deze decoder ze standaard zoekt.

## awards (src/app/awards.rs)

awards-awards = PRIJZEN
awards-band = Band
awards-all = Alle
awards-dxcc = DXCC
awards-grids = Locatievakken
awards-worked-all-states = Worked All States
awards-cq-zones-waz = CQ-zones (WAZ)
awards-dxcc-entities = DXCC-entiteiten

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Audio in de doorlaatband van de decoder.
acars-blocks-framed-blocks-whose-check = Blokken die in een frame zijn gezet, en blokken waarvan de controlereeks mislukte.
acars-starting-acars-receiver = de ACARS-ontvanger starten…
acars-channel = KANAAL
acars-tune-dial-here = Stem de dial hier af.
acars-nothing-yet-acars-quiet-between = Nog niets. ACARS is stil tussen vliegtuigen — laat het op een kanaal staan en wacht op een burst.
acars-check-failed = controle mislukt
acars-block-check-sequence-did-not = De blokcontrolereeks klopte niet — de tekst kan fout zijn.
acars-no-text = (geen tekst)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Gebruikersnaam
login-password = Wachtwoord
login-remember-device = Onthouden op dit apparaat
login-signs-without-asking-next-time = Meldt je de volgende keer aan zonder te vragen, en laat het tabblad van het 3D-beeld zonder vragen binnen. Het wachtwoord wordt in leesbare tekst bewaard, dus zet dit uit op een machine die anderen gebruiken.
login-keeps-password-device-clear = Bewaart het wachtwoord op dit apparaat, in leesbare tekst.
login-remember-station = Dit station onthouden
login-remember-me = ONTHOUD MIJ
login-station-keeps-sign-in = Het station bewaart je aanmelding, niet deze pagina — niets in te vullen op de andere radio's, en niets te vergeten als je weggaat.
login-not-this-time = Deze keer niet: deze browser vergeet het bij het sluiten, dus je wordt de volgende sessie opnieuw gevraagd en er blijft niets van deze aanmelding op het apparaat achter.
login-checking = {" "}CONTROLEREN…{" "}
login-sign = {" "}AANMELDEN{" "}
login-wrong-password-shuts-server-s = Een fout wachtwoord sluit de deur van de server voor een paar seconden, dus dit kan even duren.

## help (src/help.rs)

help-find = ZOEKEN
help-find-manual = zoeken in de handleiding
help-previous-match-shift-enter-shift = Vorige treffer (Shift+Enter, Shift+F3)
help-next-match-enter-f3 = Volgende treffer (Enter, F3)
help-no-matches = geen treffers
help-clear-search-esc = Het zoeken wissen (Esc)
help-sdroxide-manual = SDROXIDE-HANDLEIDING
help-contents = INHOUD

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 station gehoord — wachten tot het een positie meldt
aismap-double-click-reframe = dubbelklik om opnieuw te kaderen
aismap-zoom-out-wheel-pinch-do = Uitzoomen — het wiel of een knijpbeweging doet dit ook
aismap-zoom-about-middle-chart = Inzoomen rond het midden van de kaart
aismap-frame-everything-being-tracked-again = Alles wat gevolgd wordt opnieuw kaderen — een dubbelklik doet dit ook

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = wachten op spectrum…
specview-hounds = HONDEN

## spectrum (src/app/spectrum.rs)
spectrum-window-has-no-gpu-renderer = Dit venster heeft geen GPU-renderer, dus de waterval blijft op zijn standaardbreedte.
spectrum-machine-drawing-without-gpu-every = Deze machine tekent zonder GPU — elke kolom van de waterval is het werk van de processor, en die deelt dat al met de radio.
spectrum-window-renders-through-opengl-which = Dit venster rendert via OpenGL, het compatibiliteitspad van sdroxide — een Raspberry Pi, een oudere grafische chip, of een browser zonder WebGPU. Een bredere waterval is daar de framesnelheid niet waard.
spectrum-wider-than-renderer-can-draw = Breder dan deze renderer kan tekenen.

## macros (src/app/panels/macros.rs)

macros-nocall = GEEN ROEP
macros-each-button-sends-whole-text = Elke knop verstuurt zijn hele tekst in één keer. F2–F10 drukken de eerste negen in, zolang niets op het scherm het toetsenbord heeft.
macros-remove-button = Deze knop verwijderen

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = LOCATIEVAK-TRACKER
gridtracker-shade-squares-heard-live-decode = De vakken kleuren die in de live lijst met decoderingen gehoord zijn, niet alleen die in het logboek — wat nu op de lucht is, in cyaan onder de gewerkte vakken.
gridtracker-drag-pan-wheel-zoom-amber = Slepen om te verschuiven, wiel om te zoomen. Amber = gewerkt, groen = bevestigd, cyaan = gehoord.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = server bezet — een andere client is verbonden
remote-transmitting-but-client-s-microphone = Zendt, maar de microfoon van deze client levert helemaal geen audio — er wordt niets gemoduleerd. Controleer in een browser of de pagina de microfoon mocht gebruiken en of geen andere toepassing hem vasthoudt.
remote-connection-closed = verbinding verbroken

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — zonnestelsel

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = 3D-weergave van het zonnestelsel — Zon, Aarde, Maan, zonnevlekken en CME's (apart venster)
solarbtn-solar-system-3d-view-sun-2 = 3D-weergave van het zonnestelsel — Zon, Aarde, Maan, zonnevlekken en CME's (nieuw browsertabblad)

## widespec (src/widgets/wide_spectrum.rs)


## speech (src/app/speech.rs)

speech-loading-voice = De stem laden…
speech-browser-client-cannot-speak-yet = de browserclient kan nog niet spreken

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Dubbelklik om een frequentie te typen
freqdisplay-tap-type-frequency = Tik om een frequentie te typen

## solarstate (src/solar3d/state.rs)

solarstate-home = THUIS
solarstate-dwarf-planets = DWERGPLANETEN

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = dubbelklik om opnieuw te kaderen

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = dubbelklik om opnieuw te kaderen

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = verbinding verbroken

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = de meldingsdraad kon niet gestart worden

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = nog geen vliegtuigen gelokaliseerd

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = DUBBELKLIK OM OPNIEUW TE KADEREN

## chrome (src/chrome.rs)



## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Stelt de eigen vermogensinstelling van de FDM-DUO in (menu 34 TX POWER). Aansturing en Afstemmen stellen dan het vermogen binnen die waarde in, en SDRoxide stuurt de instelling opnieuw elke keer dat het verbindt.

## Band conditions window: a peak date.

# $month is 1–12.
bands-peak-date = { $day } { $month ->
        [1] jan
        [2] feb
        [3] mrt
        [4] apr
        [5] mei
        [6] jun
        [7] jul
        [8] aug
        [9] sep
        [10] okt
        [11] nov
       *[12] dec
    }

## download (src/download.rs)

download-browser-could-not-read-file = de browser kon dat bestand niet lezen

## morse (src/app/morse.rs)

morse-translate = VERTALEN
morse-practice = OEFENEN
morse-learn = LEREN
morse-morse = MORSE
morse-text-morse = Tekst → Morse
morse-morse-text = Morse → Tekst
morse-per-character-space-between-characters = `.` en `-` per teken, een spatie tussen tekens, ` / ` tussen woorden. Het ITU-alfabet, cijfers, leestekens en de gebruikelijke prosigns.
morse-reference-translator-available-desktop-build = De referentievertaler is beschikbaar in de desktopbouw.
morse-play-what-you-type-your = Speel af wat je typt, op je eigen luidsprekers
morse-play = AFSPELEN
morse-stop = STOP
morse-every-character-learned-well-done = Elk teken geleerd — goed gedaan.
morse-drill-needs-sound-which-desktop = De oefening heeft geluid nodig, wat de desktopbouw heeft.
morse-reveal = TONEN
morse-reset = HERSTELLEN
morse-replay = OPNIEUW AFSPELEN
morse-what-did-you-hear = Wat hoorde je?
morse-check = CONTROLEREN
morse-press-new-hear-character = Druk op NIEUW om een teken te horen.
morse-tone = Toon
morse-speed = Snelheid
morse-spacing = Spatiëring
morse-characters-go-out-speed-above = Tekens gaan uit op de snelheid hierboven; dit is de snelheid waarmee de *gaten* verstuurd worden, zodat een beginner de letter als geheel hoort voordat hij ze zo snel kan sturen.
morse-playback-available-desktop-build = Afspelen is beschikbaar in de desktopbouw.

## ADS-B panel: aircraft detail rows.

adsb-row-altitude = hoogte
adsb-row-track = koers
adsb-row-turning = draaiend
adsb-row-squawk = squawk
adsb-row-range = afstand
adsb-row-position = positie
adsb-row-signal = signaal
adsb-row-frames = frames
rade-transmitting = zendt
decodes-odd = ONEVEN
decodes-new = NIEUW
js8-new = NIEUW

## AIS panel: vessel detail rows.

ais-row-status = status
ais-row-speed = snelheid
ais-row-course = koers
ais-row-heading = koers
ais-row-turning = draaiend
ais-row-altitude = hoogte
ais-row-size = grootte
ais-row-draught = diepgang
ais-row-destination = bestemming
ais-row-range = afstand
ais-row-signal = signaal
ais-row-type = type
ais-row-position = positie
ais-row-messages = berichten
ism-off = UIT
textmodem-frequency-shifted = frequentieverschoven
winlink-to-heading = AAN

## adsbmap: messages with values (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/min

## aismap: messages with values (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } gehoord — geen positie nieuwer dan { $drop_map_s } s
aismap-stations-heard-waiting-position-report = { $heard } stations gehoord — wachten op een positiemelding
aismap-km-across = { $lon_span }° · ~{ $km } km breed

## alerts: messages with values (src/app/alerts.rs)

alerts-alarming = alarm op { $device }
alerts-alarm-device-unavailable = alarmapparaat niet beschikbaar: { $error }

## awards: messages with values (src/app/awards.rs)

awards-worked = { $w } gewerkt
awards-confirmed = { $value } bevestigd

## bands: messages with values (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · verwachting van HAMQSL.com, { $value } oud
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · volgend baken over { $now } s
bands-hearing-means-path-open-each = { $callsign } op { $location } — het horen ervan betekent dat het pad op { $band } open is. Elk baken stapt elke 10 s een band omhoog, dus deze rij verandert elk tijdslot.
bands-radiants-placed = radianten geplaatst voor { $lat }°, { $lon }°
bands-zhr = ZHR { $zhr }
bands-radiant = radiant { $alt_deg }° { $az_deg }
bands-radiant-down = radiant onder de horizon ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, moederlichaam { $parent }. Piekfrequentie ZHR { $zhr } rond { $peak }.

    Een radiant boven de horizon betekent dat de sporen je kunnen bereiken; een snelle regenstroom laat langer levende geïoniseerde sporen achter voor meteorverstrooiing op 6 m en 2 m, en de korte openingen op 10 m.

## drm: messages with values (src/app/drm.rs)

drm-not-decoding = { $channel } decodeert niet
drm-symbols = { $value } symbolen · { $frame }
drm-not-decodable = { $value } — niet te decoderen
drm-broadcaster-s-clock-utc = klok van de omroep { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: messages with values (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: het volle vermogen van de FDM-DUO, ongeveer 8 W. De instelling om te gebruiken met de HR500 in de lijn.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Met de HR500 in de lijn bereikt hij op de meeste banden bij deze instelling niet het volle vermogen: gebruik MAX, en Aansturing om het omlaag te brengen.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, voor zenden zonder versterker of QRP.

    { $base }

## frame: messages with values (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Schakelt zenden weer in na { $swr }:1. Controleer eerst de antenne: is de storing er nog, dan stopt de volgende zending ook.
frame-sdroxide-has-been-released = SDRoxide { $version } is uitgebracht — dit is { $sdroxide_version }.
frame-reconnecting-s = Opnieuw verbinden over { $value } s…

## gridtracker: messages with values (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } gewerkt
gridtracker-confirmed = { $confirmed } bevestigd
gridtracker-heard = { $heard } gehoord

## hd: messages with values (src/app/hd.rs)

hd-decode = Decoderen { $value } ({ $codec_label })

## hfdl: messages with values (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz heeft al een chip
hfdl-save-current-channel-khz-one = Het huidige kanaal ({ $cur_khz } kHz) opslaan als een van je eigen chips
hfdl-aircraft = { $aircraft } vliegtuigen
hfdl-bursts = { $bursts } bursts
hfdl-decodes-2 = { $decodes } decoderingen
hfdl-fixed = { $fec } hersteld

## ism: messages with values (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — nog niet gedecodeerd
ism-listen-tunes-radio-there-one = Luister op { $value } — stemt de radio daar af. Eén band tegelijk: ze liggen te ver uit elkaar om met één ontvanger er twee te bestrijken.
ism-watch-hz-around-band-centre = Let op { $value }Hz rond het midden van de band, wat de band ook zou vragen. Smaller past bij een ontvanger die de eigen breedte van de band niet kan leveren; breder bereikt apparaten die verder van het midden zitten. Je ontvanger moet ongeveer een derde meer aanleveren dan dit.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } van jezelf
ism-decoders-decoded = { $decoders } decoders ({ $flex }), { $decodes } gedecodeerd
ism-handled-rtl-433 = afgehandeld door rtl_433: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = venster { $mhz } MHz, { $e62 } MHz breed
ism-bursts-decoded = { $bursts } bursts, { $decodes } gedecodeerd
ism-devices = { $ism_reports } apparaten
ism-id-mhz-db-heard-first =
    { $protocol } id { $device }
    { $e6 } MHz { $snr_db } dB gehoord { $value }×
    eerst gehoord { $first_at } geleden

## logbook: messages with values (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Bevestigingen: { $recs } gedownload, { $matched } nieuw bevestigd
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ EERDER GEWERKT ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: messages with values (src/app/mod.rs)

app-radio = radio { $id }

## morse: messages with values (src/app/morse.rs)

morse-could-not-start-morse-audio = de morse-audiodraad kon niet gestart worden: { $error }
morse-no-audio-output = geen audio-uitvoer: { $error }
morse-about-s = ongeveer { $farnsworth_wpm } s
morse-characters-play = Tekens in gebruik: { $set }
morse-score-run = Score: { $correct } / { $total }. Reeks: { $streak } van { $value }.
morse-correct = goed — { $value }
morse-was = dat was { $value }

## net: messages with values (src/app/net.rs)

net-synced-z-new-already-log = Gesynchroniseerd om { $when }Z: { $added } nieuw, { $matched } stond al in het logboek
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Volledige resynchronisatie gestopt, er is niets veranderd: { $error }
net-full-resync-applied-added-updated = Volledige resynchronisatie toegepast: { $add } toegevoegd, { $update } bijgewerkt, { $remove } verwijderd, { $mark } van jou gemarkeerd als in Wavelog
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = ADIF-import mislukt: { $error }
net-not-unicode-read = {" "}(geen Unicode; gelezen als { $enc })
net-received-reports-swl-left-out = , { $n } ontvangstrapporten (SWL) weggelaten
net-adif-import-added-duplicates-skipped = ADIF-import: { $added } toegevoegd, { $skipped } dubbelen overgeslagen{ $reports }{ $assumed }
net-channel-import-failed = Kanaalimport mislukt: { $error }
net-line-s-skipped = , { $n } regel(s) overgeslagen
net-not-unicode-read-2 = {" "}(geen Unicode; gelezen als { $enc })
net-channel-import-channel-s-read = Kanaalimport: { $n } kanaal/kanalen gelezen{ $skipped }{ $assumed }

## acars: messages with values (src/app/panels/acars.rs)

acars-frames-bad = { $frames } frames · { $bad } fout
acars-mode = modus { $mode }

## adsb: messages with values (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } vliegtuigen
adsb-frames = { $frames } frames
adsb-preambles = { $preambles } preambules
adsb-bad-crc = { $bad_crc } foute CRC
adsb-tune = AFSTEMMEN { $mhz }
adsb-ft-barometric = { $ft } ft barometrisch

## ais: messages with values (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } schepen
ais-messages = { $messages } berichten
ais-slots = { $bursts } slots
ais-bad-fcs = { $bad_fcs } foute FCS
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } slots, { $messages } berichten, vloer { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = AFSTEMMEN { $mhz }
ais-ships-heard-are-khz-off = De gehoorde schepen staan { $khz } kHz naast de frequentie ({ $mhz } ppm bij 162 MHz) — stel de frequentiecorrectie van het front-end in, anders stoppen zendingen die ruim 5 kHz afwijken met decoderen
ais-over-ground = { $value }° over de grond
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (laatste type { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — maritiem kanaal { $ch }. Een schip wisselt tussen de twee, dus één uitzetten halveert hoe vaak elk schip gehoord wordt.

## aprs: messages with values (src/app/panels/aprs.rs)

aprs-object = object van { $reported_by }
aprs-via = via { $via }
aprs-object-reported = object gemeld door { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } cijfer(s) leeggemaakt
aprs-wind-m-s = wind { $value }° { $value2 } m/s
aprs-gust = windstoot { $value }
aprs-rain-mm-h = regen { $value } mm/u
aprs-frame-s-last-ago = { $packets } frame(s), laatste { $last_heard } geleden
aprs-via-2 = via { $via }
aprs-bad = { $bad_frames } fout
aprs-other = { $non_aprs } overig
aprs-sent = verstuurd ×{ $tries }
aprs-stn = { $stations } stn
aprs-queued = { $tx_queue } in de wachtrij

## atchat: messages with values (src/app/panels/atchat.rs)

atchat-master = master { $value }
atchat-last-heard-s-ago-click = { $status } — laatst gehoord { $age_s } s geleden · klik om een tabblad voor een direct bericht te openen
atchat-close-tab = het tabblad { $peer } sluiten
atchat-direct-message = direct bericht aan { $value }
atchat-cannot-display = kan { $path } niet tonen

## cw: messages with values (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Klik voor de { $neural }-decoder. De neurale kopieert verder naar beneden en leest met de hand verzonden CW die een timingfit niet accepteert; de timingdecoder is de enige die de geaccentueerde letters kopieert — Ä, Ö, Å, Ü, É — omdat het model daar geen uitvoerklasse voor heeft.
cw-idle-s = INACTIEF { $idle } s

## decodes: messages with values (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = De { $n } decoderingen in de lijst opslaan als ADIF — ontvangen rapporten, geen contacten. Decoderingen die geen zender noemen, worden weggelaten.
decodes-save-decodes-list-csv-one = De { $n } decoderingen in de lijst opslaan als CSV, één rij per stuk
decodes-session-qso = Sessie: { $session } QSO
decodes-save-whole-logbook-qso-adif = Het hele logboek ({ $logged } QSO) opslaan als ADIF
decodes-save-whole-logbook-qso-text = Het hele logboek ({ $logged } QSO) opslaan als tekst
decodes-hz-slots = { $audio_hz } Hz · { $odd } slots
decodes-your-slot-timing-against-stations =
    Je tijdslotafstemming ten opzichte van de stations die je hoort.
    { $after }

    Het dekt het hele ontvangstpad, dus een trage audio- of netwerkketen telt even zwaar als een verkeerde klok. Onder 0,5 s is comfortabel.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = WACHTRIJ { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Klik om te verwijderen

## js8: messages with values (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Elke snelheid wordt gedecodeerd; ▸ markeert { $speed }, degene waarop je zendt
js8-every-speed-being-decoded-click = Elke snelheid wordt gedecodeerd. Klik om in plaats daarvan op { $speed } te zenden
js8-transmit-decode = Zenden en decoderen op { $speed }
js8-beaconing-every-min-click-stop = Baken elke { $hb_min } min — klik om te stoppen
js8-last-beacon-went-out-hz = Het laatste baken ging uit op { $hz } Hz — een vrij slot in het hartslag-subbereik van { $value }–{ $value2 } Hz, gekozen zodat het naast de gedecodeerde signalen valt.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } frames · { $left } s
js8-decoded = Gedecodeerd op { $speed }
js8-frames = … ({ $frames } frames)
js8-reply-2 = Antwoord aan { $value }: “{ $value2 }”
js8-address-composer = Richt de opsteller op { $value }

## macros: messages with values (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: verstuurt “{ $text }”
macros-sends = Verstuurt “{ $text }”
macros-most = { $value } is het maximum

## navtex: messages with values (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } hersteld · { $lost } verloren
navtex-characters-lost = { $lost } tekens verloren

## packet: messages with values (src/app/panels/packet.rs)

packet-bad = { $bad_frames } fout
packet-link-layer = De linklaag staat op { $link }
packet-via = {" "}via { $via }
packet-unacked = { $unacked } onbevestigd
packet-retry = herhaling { $retries }

## pi4: messages with values (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baken
       *[other] bakens
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } geleden

## rade: messages with values (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = offset { $freq_offset_hz } Hz
rade-heard = gehoord { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } samples weggevallen tussen de ontvanger en de decoder — deze machine houdt het niet bij met de neurale decodering.

## textmodem: messages with values (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } tekens/s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } tekens/s, { $bandwidth_hz } Hz breed, { $keyed }

## vdl2: messages with values (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } frames
vdl2-bursts = { $bursts } bursts
vdl2-sync = { $syncs } sync
vdl2-hdlc-bad = { $hdlc_bad } HDLC fout
vdl2-bad-fcs = { $fcs_bad } foute FCS
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } bursts, { $frames } frames, ruisvloer { $floor_dbfs } dBFS
vdl2-tune = AFSTEMMEN { $mhz }
vdl2-held-2 = VASTGEHOUDEN +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — wereldwijd in gebruik, en daar begint elke verbinding. Degene om te houden als je er maar één houdt.
vdl2-assigned-which-channels-carry-anything = Toegewezen aan een { $what }. Welke kanalen iets dragen hangt af van waar je bent: laat ze allemaal aan, tenzij je anders weet.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } RS-symbolen hersteld
vdl2-characters = { $parity_errors } tekens
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] parameter
       *[other] parameters
    } die SDRoxide niet leest — hierboven getoond als identificatie en bytes

## wefax: messages with values (src/app/panels/wefax.rs)

wefax-lines = { $lines } regels
wefax-lines-held = { $live_size } regels vastgehouden
wefax-pixels-per-line = { $width } pixels per regel
wefax-shift-picture-pixels = Het beeld { $px } pixels verschuiven
wefax-charts-are-saved-click-copy =
    Kaarten worden { $place } opgeslagen

    Klik om het pad te kopiëren
wefax-finished-charts-are-saved-collect = Afgemaakte kaarten worden { $place } opgeslagen en verzamelen zich hier.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } pixels
    { $value }

    Rechtsklik om te verwijderen
wefax-older-store = { $older } ouder in de opslag
wefax-older-load-more = { $older } ouder — meer laden
wefax-published-carrier-khz-dial-khz = Gepubliceerde draaggolf { $f } kHz → dial { $value } kHz USB
wefax-delete-chart-2 = Deze kaart verwijderen { $dir }

## widgets: messages with values (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · CQ-zone { $cq_zone } · ITU-zone { $itu_zone }
widgets-new-entity = Nieuwe entiteit op { $band_label }
widgets-worked-before = Eerder gewerkt op { $band_label }
widgets-calling-cq-includes-you = CQ roepend { $target } — dat omvat jou
widgets-calling-cq-not-aimed-you = CQ roepend { $target } — niet op jou gericht

## wspr: messages with values (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } hoorde ons
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] baken
       *[other] bakens
    }
wspr-beaconing-duty = bakenen — { $wspr_power_dbm }, { $wspr_tx_percent }% duty
wspr-listening-duty = luisteren ({ $wspr_tx_percent }% duty)
wspr-beacon-one-two-minute-slot = Baken in één tijdslot van twee minuten van elke { $pct } — { $pct2 }% ervan, gelijkmatig verdeeld — op { $wspr_power_dbm }. Op welk slot de cyclus begint, wordt uit je roepletters getrokken, dus twee stations die dit programma draaien, beginnen niet tegelijk.
wspr-transmitting-set-general-tab-settings = Zendt uit als { $call } in { $sent } — ingesteld op het tabblad Algemeen van Instellingen.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } geleden

## publicsdr: messages with values (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    antenne: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } ontvangers · { $per_net } · opgehaald { $age } geleden
publicsdr-showing-search-narrow = { $shown } van { $n } getoond — zoek om te verfijnen
publicsdr-match = { $n } treffer
publicsdr-map-status = { $shown } op de kaart · { $unplaced } zonder positie
publicsdr-opening-another-radio = { $entry } openen als een andere radio…

## qo100: messages with values (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Voordat je AAN zet: stem zo af dat de twee lobben van het baken tussen +{ $park_lo_hz } en +{ $park_hi_hz } kHz in de strook liggen (de gearceerde baan).
qo100-receiver-currently-listening-around-mhz = De ontvanger luistert nu rond { $mhz } MHz — ver weg van het baken op 10489,750 MHz, dus er is hier niets te tekenen. De decoder blijft hoe dan ook zoeken.
qo100-receiver-currently-covers-only-khz = de ontvanger bestrijkt hier alleen ±{ $khz } kHz — de rest van de strook blijft leeg
qo100-null-db-sym-snr-db = nul { $est_null_depth_db } dB sym { $est_symmetry } snr { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (teruggedraaid vóór decoderen)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (teruggedraaid vóór decoderen)
qo100-found-empty = { $est_updates } gevonden, { $est_misses } leeg
qo100-last-s-ago = {"   "}laatste { $auto_last_hz } ({ $auto_last_unix } s geleden)
qo100-over-correction = { $auto_total_hz } over { $auto_applies } { $auto_applies ->
        [one] correctie
       *[other] correcties
    }{ $last }
qo100-sync-32-err = sync { $sync_bit_errors } / 32 fout ×{ $sync_matches }
qo100-frame-buffer = framebuffer { $frame_fill }%
qo100-blocks-tried-decoded = { $blocks_tried } blokken geprobeerd, { $blocks_locked } gedecodeerd
qo100-last-applied-s-ago-hz = laatst toegepast { $ago } s geleden: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = vergrendeld — { $blocks_tried } { $blocks_tried ->
        [one] blok
       *[other] blokken
    } geprobeerd, { $blocks_locked } vergrendeld
qo100-searching-block-tried-none-locked = zoeken — { $blocks_tried } { $blocks_tried ->
        [one] blok
       *[other] blokken
    } geprobeerd, nog niets vergrendeld

## rds: messages with values (src/app/rds.rs)

rds-auto = Auto ({ $standard })
rds-type = type { $class }
rds-groups = · { $groups } groepen
rds-blocks-error = · { $ber } % van de blokken met fouten
rds-clean-corrected-lost = { $blocks_ok } schoon · { $blocks_corrected } hersteld · { $blocks_bad } verloren

## sat: messages with values (src/app/sat.rs)
sat-utc-local = { $unix } UTC ({ $offset } lokaal)
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } boven de horizon
sat-min = over { $value } min
sat-h-min = over { $value } u { $value2 } min
sat-searching = zoeken… { $next } van { $entries }
sat-locked-2 = ● VERGRENDELD — { $t }
sat-pass-until-max = Passage tot { $set_unix } · max { $max_el }°
sat-next-pass-rises-max = Volgende passage { $rise_unix } { $rise_unix2 } ({ $now }) · stijgt { $rise_az }° { $rise_az2 } · max { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } rigctld-{ $clients ->
        [one] client
       *[other] clients
    } verbonden — satellietsoftware die daar de dial stuurt, zou de Doppler twee keer corrigeren
sat-rotator-antenna-az-el = Rotor: antenne op { $az }° az { $el }° el
sat-rotator-2 = Rotor: { $error }
sat-up-now-2 = ● nu boven, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: messages with values (src/app/scanner.rs)

scanner-holding-mhz = vasthouden op { $here } MHz
scanner-scanning-mhz = scannen · { $here } MHz

## spectrum: messages with values (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Deze renderer houdt geen textuur vast die breder is dan { $max_texture_dim } pixels.
spectrum-mem = Geh: { $f } / { $value }
spectrum-mem-2 = Geh: { $value }

## speech: messages with values (src/app/speech.rs)

speech-voice-2 = Stem: { $voice }
speech-speech-unavailable = Spraak is niet beschikbaar: { $error }

## spots: messages with values (src/app/spots.rs)

spots-transmissions = { $stations } zendingen

## winlink: messages with values (src/app/winlink.rs)

winlink-last-session-failed = laatste sessie mislukt — { $err }
winlink-last-session-received-sent = laatste sessie: { $last_received } ontvangen, { $last_sent } verzonden
winlink-empty = { $to_lowercase } is leeg
winlink-showing = { $entries } van { $total } getoond
winlink-fetching = { $mid } ophalen…
winlink-from-2 = van { $msg } { $date }
winlink-attachment-bytes = bijlage: { $att } ({ $data } bytes)

## aprsmap: messages with values (src/aprs_map.rs)

aprsmap-object =
    {""}
    object van { $reported_by }
aprsmap-heard-ago =
    {""}
    gehoord { $age } geleden

## download: messages with values (src/download.rs)

download-reading = lezen van { $path }: { $error }

## help: messages with values (src/help.rs)

help-missing-image = {"["}ontbrekend beeld: { $path }]

## hfdlmap: messages with values (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } positie{ $es }
hfdlmap-heard-ago =
    {""}
    gehoord { $last_at } geleden

## multi: messages with values (src/multi.rs)

multi-radio = Radio { $n }
multi-could-not-add-radio = Een radio kon niet worden toegevoegd: { $error }
multi-was-closed-station = { $value } is gesloten op het station.
multi-could-not-open = { $peer } kon niet worden geopend: { $error }
multi-has-tab-own-now = { $value } heeft nu een eigen tabblad.

## remote: messages with values (src/remote.rs)

remote-protocol-error = protocolfout: { $error }

## solarnet: messages with values (src/solar3d/net.rs)

solarnet-bad-message = ongeldig bericht: { $error }
solarnet-solar-protocol-mismatch-server-client = zonneprotocol komt niet overeen: server { $proto }, client { $value }

## smeter: messages with values (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Voorwaarts vermogen — { $w } W, gemeten door de eigen sensor van de radio.
smeter-po-radio-s-own-power =
    {""}

    PO — de eigen vermogensmeter van de radio, op { $clamp } % van zijn volle schaal. Een naaldpositie zoals de radio die meldt, geen gekalibreerd wattage; waar je op let, is of het daalt terwijl Aansturing blijft staan.
smeter-swr-1-match-antenna-presenting =
    {""}

    SWR — { $swr }:1, de aanpassing die de antenne biedt, zoals de radio die meet. Onder 2:1 is niets bijzonders; boven 3:1 wordt de schaal rood.
smeter-received-signal-strength-s9-db = Ontvangen signaalsterkte: S9 + { $over } dB ({ $s_dbm } dBm in de doorlaatband).
smeter-received-signal-strength-s-dbm = Ontvangen signaalsterkte: S{ $value } ({ $s_dbm } dBm in de doorlaatband).
smeter-radio-reports-c =
    {""}

    De radio meldt { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal corrigeert { $correction_db } dB compressie{ $held } (de terugkoppeling kwam met de zending overeen op { $score }).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal staat aan maar heeft de zending niet gevonden in het terugkoppelpad (beste overeenkomst { $score }), dus de zender gaat ongecorrigeerd de lucht in. Controleer of een monster van de uitgang van de versterker echt een ontvangstingang bereikt die de T/R-schakelaar bij zenden niet wegneemt, en of de verzwakker het niet opslokt. De lus luistert alleen naar deze ontvanger — een koppeler die op een tweede ontvanger (RX2, ADC2) is aangesloten, wordt nooit als terugkoppeling gelezen.

## specview: messages with values (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: nested messages

decodes-you-transmit-s-everyone-else = Je zendt { $off } s { $after } alle anderen — de gewone reden dat oproepen onbeantwoord blijven. Synchroniseer de klok van je computer.

## publicsdr: nested messages

publicsdr-noise-floor-score = score ruisvloer { $snr }
publicsdr-up-khz-i-q = tot { $khz } kHz I/Q

## txaudio: more messages

txaudio-gr-gate-de-ess-comp = GR gate { $gate_gr_db } de-ess { $deess_gr_db } comp { $comp_gr_db } lim { $limiter_gr_db }

## amp: more messages

amp-h-step = ~{ $l_step } µH · stap { $l_step2 }
amp-pf-step = ~{ $c_step } pF · stap { $c_step2 }

## sstv: more messages

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel }-bit · { $media_type } / { $content_encoding } · { $encoded_size } octetten in { $chunk_count } blokken ({ $chunks_first_pass } bij de eerste doorgang) · sessie { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } baud CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz bezette bandbreedte

## Clock, and a few short labels with values.

time-utc-and-local = { $utc } UTC · { $local } lokaal
sat-in-seconds = over { $s } s
winlink-to-list = aan { $to }
winlink-cc-list = cc { $cc }
spots-n-of-total = { $n } van { $total }
wefax-n-of-total = { $n } van { $total }
adsb-range-bearing = { $km } km op { $bearing }°
ais-range-bearing = { $km } km op { $bearing }°

## More values from sdroxide-types, as pickers name them (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Uit (alleen tuner, vanaf 24 MHz)
rtl-sdr-hf-mode-auto = Automatisch
rtl-sdr-hf-mode-direct-q = Directe bemonstering (Q-tak)
rtl-sdr-agc-manual = Handmatig (geen AGC)
rtl-sdr-agc-tuner = Tuner-AGC
rtl-sdr-agc-rtl = RTL-digitale AGC
rtl-sdr-agc-both = Tuner- + RTL-AGC
mode-control-cat = CAT
mode-control-radio = Radiogestuurd
line-state-none = Geen
line-state-high = Hoog
line-state-low = Laag
elad-tx-input-usb-audio = USB-audio
elad-tx-input-mic = Microfoon
elad-tx-input-auto = Automatisch
elad-tx-input-leave = Laten zoals ingesteld op de radio
tr-usdx-audio-one-cable = Eén kabel (audio in de CAT-stroom)
tr-usdx-audio-sound-card = USB-geluidskaart (3,5 mm-jack)
spy-server-format-uint8 = 8-bit (laagste bandbreedte)
spy-server-format-int16 = 16-bit
spy-server-format-float32 = 32-bit float
sound-format-iq = IQ (stereo)
sound-format-demod-audio = Gedemoduleerde audio
sdr-play-duo-tuner-tuner1 = Tuner 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Tuner 2 (50 Ohm)
sdr-play-duo-role-diversity = Een tweede antenne (diversiteit / onderdrukking van QRM)
sdr-play-duo-role-second-radio = Een tweede radio, op de eigen frequentie
sdr-play-agc-off = Uit (handmatige IF-gain)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Uit
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (beide tegelijk)
pluto-duplex-tdd = TDD (één tegelijk)
pluto-agc-manual = Handmatig
pluto-agc-slow-attack = Langzame aanval
pluto-agc-fast-attack = Snelle aanval
pluto-agc-hybrid = Hybride
parity-none = Geen
parity-even = Even
parity-odd = Oneven
panadapter-tap-antenna = Antenne (gedeeld met de radio)
panadapter-tap-if-output = De I.F.-uitgang van de radio
panadapter-audio-attached = De aangesloten ontvanger
panadapter-audio-transceiver = De transceiver
kenwood-send-ts2000 = TS-2000-stijl (TX;)
kenwood-send-ts590 = TS-590-stijl (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Radiogestuurd
cw-keying-cat = Radiokeyer (CAT)
cw-keying-audio = Geluidskaart (MCW)
audio-route-both = Beide kanalen
audio-route-left = Alleen links
audio-route-right = Alleen rechts
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (niet getest)
alert-sound-ding = Ding
alert-sound-two-tone = Tweetoon
alert-sound-triplet = Drieklank
alert-sound-warble = Trillen
alert-sound-digital = Digitaal
alert-reply-tone = Toon
alert-reply-voice = Stem
alert-reply-both = Toon + stem
icom-model-ic7300 = IC-7300
icom-model-ic7300-mk2 = IC-7300MK2
icom-model-ic705 = IC-705
icom-model-ic7760 = IC-7760
icom-model-ic905 = IC-905
icom-model-ic9700 = IC-9700
icom-model-ic7610 = IC-7610
icom-model-ic7100 = IC-7100
icom-model-ic7410 = IC-7410
icom-model-ic7600 = IC-7600
icom-model-ic7700 = IC-7700
icom-model-ic7800 = IC-7800
icom-model-ic9100 = IC-9100
icom-model-ic7200 = IC-7200
icom-model-ic7000 = IC-7000
icom-model-other = Overig (adres met de hand instellen)
backend-auto = Automatisch detecteren (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Audio
backend-hpsdr = HPSDR (netwerk)
backend-tci = TCI (netwerk)
backend-icom-net = Icom LAN (netwerk)
backend-smart-sdr = SmartSDR / FlexRadio (netwerk)
backend-pluto = PlutoSDR (netwerk)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR via rtl_tcp (netwerk)
backend-spy-server = SpyServer (netwerk)
backend-spy-server-vfo = SpyServer VFO+FFT, lage bandbreedte (netwerk)
backend-kiwi-sdr = KiwiSDR / Web-888 (netwerk)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = USB-audioradio (geluidskaart)
backend-none = Niet ingesteld
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (netwerk)
cat-family-flrig = flrig (netwerk)

## Values from sdroxide-types drawn on chips, menus and lists (src/i18n/labels.rs).

agc-mode-off = Uit
agc-mode-slow = Langzaam
agc-mode-med = Gemiddeld
agc-mode-fast = Snel
airspy-gain-linearity = Lineariteit (beste omgang met sterke signalen)
airspy-gain-sensitivity = Gevoeligheid (beste voor zwakke signalen)
ais-kind-class-a = Klasse A
ais-kind-class-b = Klasse B
ais-kind-base-station = basisstation
ais-kind-sar-aircraft = SAR-vliegtuig
ais-kind-aid-to-navigation = vaarwegmarkering
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = bijbehorend vaartuig
alert-event-called = Een station roept mij op
alert-event-cq = Een CQ waarop ik kan antwoorden
alert-event-new-dxcc = Nieuwe DXCC-entiteit
alert-event-new-dxcc-band = Nieuwe entiteit op deze band
alert-event-new-grid = Nieuw locatievak
audio-routing-both = Links + rechts
audio-routing-left = Alleen links
audio-routing-right = Alleen rechts
button-mode-momentary = Vasthouden
button-mode-toggle = Wisselen
contest-mode-none = Geen
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Uit tijdens converteren
converter-tx-transverter = Via dezelfde converter
converter-tx-own = Eigen offset
cw-engine-neural = NEURAAL
cw-engine-timing = TIMING
decode-sort-none = Geen
decode-sort-signal = SNR
decode-sort-distance = Afst
decode-sort-country = Land
diversity-mode-cancel = Annuleren — een ruisbron wegnullen
diversity-mode-combine = Combineren — diversiteitsontvangst
dxped-mode-normal = Normaal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Weigeren te zenden
fail-safe-warn-only = Toch zenden, waarschuwen
ftp-security-plain = FTP (geen versleuteling)
ftp-security-explicit-tls = FTPS expliciet (AUTH TLS)
ftp-security-implicit-tls = FTPS impliciet
hpsdr-filter-board-none = Geen — uitgangen blijven uit
hpsdr-filter-board-n2adr = N2ADR-filterbord
hpsdr-filter-board-alex = Alex / Hermes-bandcode (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Aangepast — mijn eigen tabel hieronder
hpsdr-io-rx-input-radio = Eigen ingang van de radio
hpsdr-io-rx-input-io-board = IO-bord J9
hpsdr-io-rx-input-io-board-pure-signal = IO-bord J9, PureSignal bij zenden
hydra-sdr-gain-linearity = Lineariteit (beste omgang met sterke signalen)
hydra-sdr-gain-sensitivity = Gevoeligheid (beste voor zwakke signalen)
icom-rx-source-af = AF — de radio demoduleert
icom-rx-source-if12k = 12 kHz IF — sdroxide demoduleert
icom-scope-span-radio = Zoals ingesteld op de radio
icom-scope-span-khz2-5 = 5 kHz (±2,5k)
icom-scope-span-khz5 = 10 kHz (±5k)
icom-scope-span-khz10 = 20 kHz (±10k)
icom-scope-span-khz25 = 50 kHz (±25k)
icom-scope-span-khz50 = 100 kHz (±50k)
icom-scope-span-khz100 = 200 kHz (±100k)
icom-scope-span-khz250 = 500 kHz (±250k)
icom-scope-span-khz500 = 1 MHz (±500k)
ism-family-weather = WEER
ism-family-meter = METERS
ism-family-home-auto = THUIS
ism-family-lo-ra = LORA
ism-family-unidentified = ONBEKEND
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = niet geïdentificeerd
js8-speed-normal = NORMAAL
js8-speed-fast = SNEL
js8-speed-turbo = TURBO
js8-speed-slow = LANGZAAM
lime-aux-role-off = Niet gebruikt
lime-aux-role-diversity = Een tweede antenne (diversiteit / onderdrukking van QRM)
lime-aux-role-pure-signal = Zendterugkoppeling (PureSignal-predistortie)
lookup-provider-none = Uit
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = INBAK
mail-folder-outbox = UITBAK
mail-folder-sent = VERZONDEN
mail-folder-archive = ARCHIEF
memory-sort-name = Naam
memory-sort-freq = Freq
memory-sort-band = Band
memory-sort-stored = Opgeslagen
mouse-button-middle = Midden
mouse-button-extra1 = Terug (extra 1)
mouse-button-extra2 = Vooruit (extra 2)
nr-level-off = Uit
nr-level-low = Laag
nr-level-medium = Midden
nr-level-high = Hoog
nr-level-rnn-low = RNN laag
nr-level-rnn-med = RNN midden
nr-level-rnn-high = RNN hoog
nr-level-spec-low = SPEC laag
nr-level-spec-med = SPEC midden
nr-level-spec-high = SPEC hoog
nr-level-nr2-low = NR2 laag
nr-level-nr2-med = NR2 midden
nr-level-nr2-high = NR2 hoog
nr-level-df-low = DFNR laag
nr-level-df-med = DFNR midden
nr-level-df-high = DFNR hoog
nr-strength-low = Laag
nr-strength-med = Gem
nr-strength-high = Hoog
orbit-rings-none = geen
orbit-rings-curated = gecureerd
orbit-rings-all = alle
qso-step-idle = Inactief
qso-step-wait-cq = CQ afwachten
qso-step-calling-cq = CQ roepen
qso-step-tx-grid = Zenden locatievak
qso-step-tx-report = Zenden rapport
qso-step-tx-r-report = Zenden R+rapport
qso-step-tx-rr73 = Zenden RR73
qso-step-tx73 = Zenden 73
qso-step-confirming = Bevestigen
rds-standard-auto = Auto
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Regio 1 — Europa, Afrika, Midden-Oosten, Noord-Azië
region-r2 = Regio 2 — Amerika's
region-r3 = Regio 3 — Zuid- en Oost-Azië, Australasië, Pacific
relative-mode-absolute = Absoluut (schuifregelaar)
relative-mode-sign-magnitude = Relatief: teken/grootte
relative-mode-twos-complement = Relatief: twee-complement
relative-mode-binary-offset = Relatief: rond 64
relay-link-off = Geen
relay-link-serial = USB-relaiskaart (serieel)
relay-link-serial-lines = Seriële RTS/DTR-lijn
relay-link-hid = USB-relaiskaart (HID)
relay-link-cm108 = CM108/CM119 GPIO van de geluidskaart
relay-link-gpio = Linux GPIO-lijn
relay-link-command = Externe opdracht
relay-role-unused = Niet gebruikt
relay-role-sdr-antenna = De SDR-antenne aarden
relay-role-amplifier = Een versterker / T/R-relais sturen
relay-role-aux = Hulpuitgang
relay-role-band-decoder = Banddecoder (filter / transverter)
rfe-channel-wb1000 = Breedband 1 – 1000 MHz
rfe-channel-wb4000 = Breedband 1 – 4 GHz
rfe-channel-ham0030 = HF, tot 30 MHz
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1,25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2,3 – 2,5 GHz)
rfe-channel-ham3500 = 9 cm (3,3 – 3,7 GHz)
rfe-channel-cell-band01 = Mobiel band 1
rfe-channel-cell-band02 = Mobiel band 2
rfe-channel-cell-band03 = Mobiel band 3
rfe-channel-cell-band07 = Mobiel band 7
rfe-channel-cell-band38 = Mobiel band 38
rfe-channel-not-selected = Niets gekozen
rfe-link-off = Niet verbonden
rfe-link-serial = Eigen USB-kabel (serieel)
rfe-link-board = Via de LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automatisch (volgt de bedrading)
rfe-mode-control-rx = Altijd ontvangen
rfe-mode-control-tx = Altijd zenden
rfe-mode-control-tx-rx = Altijd beide (alleen gesplitste connectors)
rifp-encoding-auto = Auto
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Ruw
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = GEH
scan-kind-range = BEREIK
scan-resume-carrier = DRAAGGOLF
scan-resume-timed = MET TIMER
scan-resume-manual = HANDMATIG
sense-line-off = Niet aangesloten
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (draaggolfdetectie)
shift-simplex = SIMPLEX
shift-minus = −
shift-plus = +
speed-slow = Langzaam
speed-medium = Gemiddeld
speed-fast = Snel
speed-faster = Sneller
speed-fastest = Snelst
tls-verify-system = Vertrouwensopslag van het systeem
tls-verify-pinned = Vastgezette vingerafdruk
tls-verify-off = Niet controleren
tone-mode-off = UIT
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Gate
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-esser
tx-voice-stage-comp = Compressor
upload-naming-sequence = image1 … imageN (roteren)
upload-naming-original = Oorspronkelijke bestandsnaam
wheel-action-zoom = Zoomen
wheel-action-tune = Afstemmen
wheel-action-none = Niets

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = DOCK — zet de band- en modusselectie terug in het hoofdvenster, hetzelfde als dit venster sluiten zonder het sluitvakje te zoeken.
frame-dock-panel-hint = DOCK — zet dit paneel terug onder de waterval
frame-dock-spectrum-hint = DOCK — zet dit spectrum terug in het hoofdvenster, hetzelfde als dit venster sluiten zonder het sluitvakje te zoeken.
frame-every-module-own-window = elke module staat in een eigen venster
frame-module-aux-panadapter = aux-panadapter
frame-module-band-mode = band & modus
frame-module-controls = bediening
frame-module-panadapter = panadapter
frame-module-panel = paneel
multi-dock-all-windows = ALLE VENSTERS DOCKEN
multi-module-panel = Bedieningspaneel
multi-undock-all-windows = ALLE VENSTERS LOSKOPPELEN
settings-ui-dock-all-hint = Breng elk losgekoppeld venster terug in dit venster.
settings-ui-docked = Gedockt in dit venster
settings-ui-module-aux = AUX SP (tweede spectrum)
settings-ui-module-band-menu = Band- en modusselectie
settings-ui-module-controls = Bediening
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Zet elke module in een eigen venster — het hoofdvenster blijft over als het spectrum en de decoders. Een module zonder iets om te tonen, zoals het bedieningspaneel in een spraakmodus, opent geen venster.
settings-ui-undock-hint =
    Losgekoppeld verhuist deze module naar een eigen venster en geeft het hoofdvenster de ruimte terug aan wat overblijft — zet de waterfall op een tweede monitor, of de bediening op een derde. Het venster sluiten koppelt de module weer vast.

    Onder Wayland komt de grootte terug, maar de compositor bepaalt waar een venster komt: laat het zweven en zet het vast op een monitor met een vensterregel die overeenkomt met app-id "{ $app_id }".
settings-ui-undocked = Losgekoppeld — eigen venster
settings-ui-window-label = Venster { $module }
frame-window-chip-hint = VENSTER — verplaats dit venster naar een eigen OS-venster, om het op een andere monitor te zetten
frame-dock-panel-pane-hint = DOCK — zet dit paneel terug in het vlak onder de waterval
frame-panel-window-title = { $mode } — paneel
frame-roll-up-hint = Oprollen — verlaat de titelbalk en geef het venster terug aan de waterval
frame-unroll-hint = Uitrollen — toon het paneel weer
frame-window-panel-hint = VENSTER — geef dit paneel een eigen venster, om op een andere monitor te zetten
topbar-dock-all = ALLES VASTZETTEN
topbar-dock-all-hint = Breng elk modulevenster terug in dit venster
topbar-module-windows-title = Modulevensters
topbar-undock-all = ALLES LOSKOPPELEN
topbar-undock-all-hint = Zet elke module in een eigen venster — dit venster houdt alleen het spectrum en de decoders
topbar-windows-chip-hint =
    ▣  Links: breng elk modulevenster naar huis
    Rechts: de lijst met modulevensters
topbar-wname-aux = SP2 (AUX) — een tweede spectrum
topbar-wname-band-menu = Band en modus
topbar-wname-controls = RX-bediening
topbar-wname-panadapter = SP1 — het spectrum
topbar-wname-panel = Bedieningspaneel
topbar-wstate-float = Til { $name } uit het paneel en laat het zweven boven dit venster
topbar-wstate-floating-panel-only = Zweven is de eigen toestand van het bedieningspaneel — een spectrum of een console heeft één plek in dit venster
topbar-wstate-here-now = { $name } is nu hier
topbar-wstate-no-panel = Geen bedieningspaneel in { $mode } — dit geldt in een modus die er een heeft
topbar-wstate-own-window = Geef { $name } een eigen venster, op elke monitor
topbar-wstate-put-back = Zet { $name } terug in dit venster
frame-workspace-missing = Geen werkruimte { $n } opgeslagen — sla er een op in Instellingen › UI › Werkruimtes en deze toets roept hem terug.
settings-ui-workspaces = Werkruimtes
settings-ui-workspaces-note = Een benoemde vensterindeling — welke modules in eigen vensters staan en waar die vensters zijn. Sla de huidige op, of zet een opgeslagen indeling terug. Maximaal tien.
settings-ui-workspaces-none = Nog geen werkruimtes opgeslagen.
settings-ui-workspace-restore-home = Zet deze indeling terug — elke module in het hoofdvenster
settings-ui-workspace-restore-out = Zet deze indeling terug — { $n } module(s) in eigen venster(s)
settings-ui-workspace-forget = Deze werkruimte vergeten
settings-ui-workspace-name = geef deze indeling een naam
settings-ui-workspace-full = Tien werkruimtes is de limiet die SDRuno bewaart; verwijder er eerst een
settings-ui-workspace-save = HUIDIGE OPSLAAN
settings-ui-workspace-save-hint = Sla de huidige indeling onder deze naam op
settings-ui-workspace-name-first = Typ eerst een naam
topbar-band-window2-hint = VENSTER — open de selectie in een eigen venster, zo breed als je wilt, op elke monitor
tnc-link = VERBINDING
tnc-connected = VERBONDEN
tnc-calling = ROEPT
tnc-idle = LUISTERT
tnc-tx = TX
tnc-busy = BEZET
tnc-busy-hint = Er is iemand anders op het kanaal. Een oproep nu zou er bovenop vallen.
tnc-setup = ⚙ INSTELLING
tnc-peer = Tegenstation
tnc-width = Breedte
tnc-speed = Snelheid
tnc-snr = S/R
tnc-bytes = Bytes
tnc-queued = In wachtrij
tnc-host = Host
tnc-ports = Poorten
tnc-ports-off = uit
tnc-nobody = niemand
tnc-holder-panel = dit paneel
tnc-holder-winlink = het MAIL-venster
tnc-holder-tcp = een programma op { $addr }
tnc-tune = AFSTEMMEN
tnc-abort = AFBREKEN
tnc-abort-hint = De verbinding nu verbreken, wie hem ook bedient. De andere kant loopt in een time-out in plaats van het te horen.
tnc-terminal = TERMINAL
tnc-callsign = roepnaam
tnc-connect = VERBINDEN
tnc-disconnect = VERBREKEN
tnc-disconnect-hint = Netjes ophangen: wat in de wachtrij staat gaat eerst, daarna stemt de andere kant in.
tnc-held-by = Het modem is in gebruik door { $holder }.
tnc-no-callsign = Stel eerst een roepnaam in (⚙ INSTELLING)
tnc-clear = WISSEN
tnc-clear-hint = Het transcript leegmaken. De verbinding blijft.
tnc-nothing-yet = Nog niets. Typ hierboven een roepnaam en VERBINDEN, of zet Luisteren aan in ⚙ INSTELLING om oproepen aan te nemen.
tnc-type-here = typ hier, Enter verstuurt
tnc-not-connected = niet verbonden
tnc-send = VERSTUREN
setup-tnc-bandwidth = Bandbreedte
setup-tnc-listen = Oproepen aannemen
setup-tnc-listen-check = Luisteren vanuit het paneel
setup-tnc-listen-hint = Inkomende oproepen aannemen in de terminal van het paneel. Geen effect zolang de TCP-hostpoorten aan staan, want een programma daar wil de oproepen.
setup-tnc-host = TCP-host
playback-radio-n = { " " }· radio { $n }
playback-no-stamp = geen tijdstempel in de naam — speelt af in de modus waarin je zit
playback-title = AFSPELEN — opgenomen audio
playback-find-hint = modus, MHz, datum, naam…
playback-refresh-hint = Lees de opnamemap opnieuw
playback-asking = Het station om zijn opnames vragen…
playback-folder = Map: { $dir }
playback-none = Nog geen opnames. Druk op REC → MP3 terwijl er iets in de lucht is; het bestand verschijnt hier, genoemd naar tijd, frequentie en modus.
playback-on = ▶ AAN
playback-play = ▶ AFSPELEN
playback-iq-hint = Een I/Q-opname, geen audio. Afspelen neemt de audio die de recorder schrijft; een I/Q-opname wordt afgespeeld met --file.
playback-not-while-tx = Niet tijdens zenden
playback-play-hint = Speel deze opname vanaf het begin af in de decoders
playback-busy-playing = Wordt afgespeeld — stop het voordat je het verwijdert
playback-busy-recording = Wordt nog opgenomen — stop REC voordat je het verwijdert
playback-delete-ask = WISSEN?
playback-delete-hint-armed = Druk nogmaals om dit bestand definitief te wissen. Het kan niet ongedaan worden gemaakt.
playback-delete-hint = Wis deze opname uit de map (vraagt eerst nog één keer)
playback-strip-label = ▶ AFSPELEN
playback-not-live = niet live — de radio staat stil
playback-carry-on = Verder afspelen
playback-pause = ⏸ PAUZE
playback-pause-hint = Pauze — de decoders horen stilte totdat je verdergaat
playback-again = ↺ OPNIEUW
playback-again-hint = Terug naar het begin — nog een poging voor een beeld of een decode
playback-loop = LUS
playback-loop-hint = Begin opnieuw bovenaan wanneer het bestand eindigt
playback-stop = ■ STOP
playback-stop-hint = Beëindig het afspelen en zet de radio terug waar hij was
playback-files = BESTANDEN
playback-end-of-file = einde bestand
playback-paused = gepauzeerd
playback-recorded-on-mode = Opgenomen op { $mhz } MHz in { $mode } — de dial en modus zijn ingesteld op basis van de bestandsnaam.
playback-recorded-on = Opgenomen op { $mhz } MHz — de dial en modus zijn ingesteld op basis van de bestandsnaam.
playback-no-dial-mode = De naam bevat geen dial of modus, dus het speelt af in de modus waarin je zit — kies de decoder in het modusmenu.
playback-format-stereo = { $rate } Hz, stereo — het linker (ontvangst)kanaal wordt afgespeeld.
playback-format-mono = { $rate } Hz, mono — het enige kanaal wordt afgespeeld.
playback-stopped = Gestopt: { $fault }
playback-intro = Speel een opname terug in de decoders — SSTV, CW, RTTY, PSK, FT8 en elke andere modus die uit audio decodeert. De radio staat stil tijdens het afspelen, de dial en modus gaan naar waarop de opname is gemaakt, en STOP zet de radio terug. Wijzig de modus tijdens het afspelen om een andere decoder te proberen.
topbar-stereo-yields-hint = WFM-stereo is uit zolang ruisonderdrukking of de auto-notch actief is — ze zouden de twee kanalen uitsmeren. Zet ze uit om stereo te horen
topbar-stereo-playback-hint = Stereo: de opname wordt in beide oren afgespeeld, zoals opgenomen. Klik om mono te forceren
topbar-stereo-mono-recording-hint = Deze opname is mono — er is geen tweede kanaal om af te spelen
topbar-caption-play-back = Afspelen
topbar-recordings-chip = ▶ OPNAMES…
topbar-recordings-hint = Speel een opname terug in de decoders — een gemist SSTV-beeld, een zwak RTTY- of CW-signaal, een FT8-slot — met de waterfall en het decodeerlogboek, alsof het in de ether was. De radio houdt zich stil terwijl het speelt.
playback-find = Zoeken
playback-refresh = VERVERSEN
wxsat-title = WEERSATELLIETEN
sat-wxsat-hint = WEERSATELLIETEN — het beeld dat een NOAA- (APT) of Meteor-satelliet (LRPT) op 137 MHz uitzendt, getekend terwijl het binnenkomt, in een losmaakbaar venster
sat-wxsat = WEERSATELLIETEN
wxsat-decode = Decoderen
wxsat-channel = Satelliet
wxsat-tune = Hier afstemmen
wxsat-tune-hint = Zet de decoder aan en brengt de frequentie naar dit kanaal, zodat het signaal binnen het samplevenster valt.
wxsat-clear = Wissen
wxsat-save = Opslaan als PNG
wxsat-flip = Omdraaien
wxsat-flip-hint = Draait het beeld om: een satelliet die naar het zuiden vliegt wordt ondersteboven getekend.
wxsat-state-off = Decoder uit
wxsat-level = Niveau { $db } dBFS
wxsat-locked = Lijnsync vergrendeld
wxsat-searching = Zoeken naar de lijnsync
wxsat-lines = { $n } lijnen
wxsat-outside-window = Het kanaal ligt buiten het samplevenster, of te dicht bij de rand voor het hele signaal. Gebruik Hier afstemmen, of een breder venster.
wxsat-waiting = Nog geen beeld. Zet de decoder aan zodra een satelliet boven de horizon staat; een overkomst duurt ongeveer tien minuten.
wxsat-signal = Signaal
wxsat-kind-apt = APT
wxsat-kind-lrpt = LRPT
wxsat-image-channel = Kanaal { $n }
wxsat-rows = { $n } rijen
wxsat-frame-locked = Framesync vergrendeld
wxsat-frames = { $n } goede frames
wxsat-waiting-lrpt = Nog geen beeld. Meteor LRPT is digitaal: het beeld begint een paar seconden nadat de satelliet is gehoord, rij voor rij, en elk kanaal is een apart beeld.
wxsat-searching-frame = Zoeken naar de framesync
sondemap-no-sonde-yet = Nog geen radiosonde ontvangen
sonde-decoding = DECODEREN
sonde-decoding-off = DECODER UIT
sonde-count = Sondes: { $n }
sonde-frames = Frames: { $n }
sonde-channel = Kanaal
sonde-channel-hover = De sondefrequentie waarop wordt geluisterd (400–406 MHz). De radio wordt er ook op afgestemd, zodat het kanaal in het venster ligt.
sonde-col-serial = Sonde
sonde-col-alt = Hoogte
sonde-col-climb = Stijging
sonde-col-speed = Snelheid
sonde-col-battery = Batterij
sonde-col-heard = Ontvangen
sonde-scan = Scan
sonde-scan-hover = Zoekt in het hele samplevenster naar sondes en decodeert er tot vier tegelijk, in plaats van op één kanaal te luisteren. Het venster moet ze bestrijken: het kanaalveld centreert de radio.
sonde-online = Online (SondeHub)
sonde-online-hover = Toont ook sondes die andere stations hebben ontvangen, uit de openbare lijst van SondeHub. Alleen lezen: uw locator op 0,1° afgerond en een straal van 400 km worden verstuurd; er wordt niets geüpload. Uit tot u het inschakelt.
sonde-online-count = { $n } online
sonde-online-no-grid = Vul uw locator in bij de instellingen van de digitale modes om online sondes te zien.
sonde-online-failed = SondeHub antwoordde niet.
sonde-export = Vlucht exporteren
sonde-export-kml-hover = De vlucht van deze sonde opslaan als KML-track (Google Earth): tijd, lengtegraad, breedtegraad, hoogte in meters
sonde-export-csv-hover = De vlucht van deze sonde opslaan als CSV-tabel voor een spreadsheet: tijd, breedtegraad, lengtegraad, hoogte in meters
sonde-export-none = Nog geen positie bewaard voor deze sonde.
sonde-col-temp = Temp.
sonde-col-humidity = Vochtigheid
sonde-col-pressure = Druk
sonde-online-why-signin = Het station vraagt eerst om aanmelden.
sonde-online-why-busy = Het station vraagt SondeHub al zo vaak als is toegestaan. Probeer het over een minuut opnieuw.
sonde-online-why-upstream = Het station kon SondeHub niet bereiken.
sonde-online-why-unreadable = Het antwoord was geen lijst met sondes.
sonde-online-why-no-station = Geen station om te vragen, en deze pagina kan SondeHub zelf niet bereiken.
sonde-online-why-http = Antwoord: HTTP { $n }.
sonde-card-source = Bron
sonde-card-source-radio = Deze radio
sonde-card-source-hub = SondeHub (door een ander station gehoord)
sonde-card-model = Model
sonde-card-maker = Fabrikant
sonde-card-variant = Variant
sonde-card-position = Positie
sonde-card-from-you = Vanaf u
sonde-card-heading = Koers
sonde-card-satellites = Satellieten
sonde-card-snr = Signaal-ruisverhouding
sonde-card-station = Gehoord door
sonde-card-station-position = Positie van het station
sonde-card-antenna = Antenne
sonde-card-software = Software
sonde-card-frames = Ontvangen frames
sonde-card-open = Openen op { $site }
sonde-card-close = Sluiten
sonde-card-source-both = Radio + SondeHub (beide horen haar)
sonde-card-stale = Status
sonde-card-stale-value = Niet meer in de lijst: laatst bekende positie
sonde-card-detach = Losmaken
sonde-card-attach = Vastmaken
sonde-card-listen-hover = Klik om de radio op dit kanaal af te stemmen: de afstemknop gaat erheen en de decoder luistert alleen daar.
sonde-card-decoding = Gedecodeerd door deze radio
sonde-card-waiting = Luistert op dit kanaal, nog niets gedecodeerd
sonde-card-click-to-listen = Klik op de frequentie om te luisteren
sonde-type-all = Alle typen
sonde-type-hover = Zoek één type sonde: een keuze start de scan, die het venster over de band verschuift tot dat type te horen is. Alleen dat type wordt getoond en getekend.
sonde-export-saved = { $name } opgeslagen.

## Keyboard modes: MT63 (src/app/panels/text_modem.rs)
textmodem-mt63-long-hint = MT63-{ $label }, { $bw } Hz breed — de lange interleave spreidt een burst over { $chars } tekens
textmodem-mt63-short-hint = MT63-{ $label }, { $bw } Hz breed, korte interleave — de korte interleave spreidt een burst over { $chars } tekens
textmodem-mt63-not-offered = 1K en 500L nog niet beschikbaar
textmodem-mt63-not-offered-hint = MT63-1K en de lange interleave van 500 Hz worden gelezen met de batchdecoder, maar nog niet via de live ontvanger: de lock zoekt de symboolfase en de polariteit maar niet het bandcentrum, en een zoekactie die dat wel doet bleek de ene variant voor de andere in te ruilen.

## Keyboard modes: MFSK (src/app/panels/text_modem.rs)
textmodem-mfsk-hint = { $label } — { $baud } baud, { $tones } tonen, { $bw } Hz breed{ $caveat }
textmodem-mfsk-reverse-hint = Draai de toonvolgorde om. Een station dat is ontvangen op de andere zijband dan waarop het is verzonden, komt gespiegeld om zijn midden binnen en klinkt als ruis totdat dit aan staat.
