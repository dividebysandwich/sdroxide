# sdroxide — deutsche Übersetzung der Oberfläche. Die Referenz ist
# en-US/sdroxide.ftl; was hier fehlt, wird auf Englisch angezeigt.

## The language's own name, as the language picker lists it.
language-name = Deutsch

## Values from sdroxide-types, as the screen names them (src/i18n/labels.rs).

layout-mode-auto = Automatisch
layout-mode-desktop = Desktop
layout-mode-tablet = Tablet
layout-mode-small = Kleiner Bildschirm
layout-mode-phone = Smartphone

ui-theme-default = Standard
ui-theme-light = Hell
ui-theme-high-contrast = Hoher Kontrast
ui-theme-green-phosphor = Grüner Phosphor
ui-theme-amber-phosphor = Bernsteinfarbener Phosphor
ui-theme-teal-orange = Blaugrün / Orange
ui-theme-rainbow = Regenbogen
ui-theme-nord = Nord
ui-theme-nord-dark = Nord dunkel
ui-theme-gruvbox = Gruvbox
ui-theme-everforest = Everforest
ui-theme-solarized-dark = Solarized dunkel
ui-theme-solarized = Solarized
ui-theme-dracula = Dracula
ui-theme-catppuccin-mocha = Catppuccin Mocha
ui-theme-catppuccin-latte = Catppuccin Latte
ui-theme-modern-minimalist = Modern minimalistisch

chrome-style-angled = Eckig
chrome-style-rectangular = Rechteckig
chrome-style-rounded = Abgerundet
chrome-style-gradient = Verlauf
chrome-style-bevel = 3D-Facette
chrome-style-terminal = Terminal

font-size-small = Klein
font-size-medium = Mittel
font-size-large = Groß

bandplan-kind-ham = Amateurfunk
bandplan-kind-cw = CW
bandplan-kind-digi = Digital
bandplan-kind-phone = Sprache
bandplan-kind-beacon = Bake
bandplan-kind-broadcast = Rundfunk
bandplan-kind-am = AM / LW / MW
bandplan-kind-cb = CB

speech-verbosity-terse = Knapp
speech-verbosity-normal = Normal
speech-verbosity-full = Ausführlich

# The examples in brackets are what the (English) voice says.
speech-freq-style-dial = Skala (14 Punkt 074)
speech-freq-style-digits = Jede Ziffer
speech-freq-style-full = Skala mit Einheiten

speech-callsign-style-phonetic = Phonetisch (kilo one alpha)
speech-callsign-style-letters = Buchstaben (kay one ay)
speech-callsign-style-as-is = Wie geschrieben

## Settings window, UI tab (src/app/settings/ui_tab.rs).

settings-ui-display = Anzeige
settings-ui-moved-note = Die Breite des Panoramas, die Reaktion des Spektrums und die Scroll-Geschwindigkeit des Wasserfalls werden über die Taste SPEC in der Anzeigeleiste eingestellt — im Menü DISP bei einem schmalen Fenster.
settings-ui-language = Sprache
    .hint = Die Sprache der Oberfläche. Das Handbuch hinter F1 und die Sprachansagen bleiben unabhängig von dieser Auswahl auf Englisch.
settings-ui-language-system = System ({ $language })
settings-ui-layout = Anordnung
settings-ui-theme = Thema
settings-ui-button-style = Schaltflächenstil
settings-ui-window-style = Fensterstil
settings-ui-tooltip-delay = Tooltip-Verzögerung
settings-ui-tooltip-delay-hint = Ruhezeit des Zeigers bis zum Tooltip
settings-ui-tooltip-grace = Tooltip-Folgezeit
settings-ui-tooltip-grace-hint = Nach einem Tooltip: so lange zeigt der nächste ohne Verzögerung; 0 = jeder wartet
settings-ui-frame-rate = Bildschirm-Aktualisierungsrate
settings-ui-palette = Farbpalette des Wasserfalls
settings-ui-tuning-buttons = Abstimmtasten
settings-ui-tuning-buttons-show = Auf Smartphone und Tablet anzeigen
    .hint = Eine Reihe von Tasten zum Verringern / Schritt / Erhöhen unter der Bedienleiste eines Touchscreens. Es gibt kein Rad, mit dem man eine Ziffer durchscrollen kann, und keinen Drehknopf; ohne diese Tasten lässt sich ein bekannter Schritt nur durch Eingabe der ganzen Frequenz ändern. Auf einem Desktop werden sie nie angezeigt.
settings-ui-tuning-step = Schritt: { $step }
settings-ui-tuning-snap = Erster Druck rastet auf den Schritt ein
    .hint = Liegt die Skala zwischen zwei Vielfachen des Schritts, springt sie beim Drücken zum nächsten Vielfachen in dieser Richtung — 7 074 300 geht bei 1 kHz Schritt nach oben auf 7 075 000, nach unten auf 7 074 000. Danach bewegen die Tasten sich wie gewohnt um den Schritt. Ausgeschaltet bewegt sich jeder Druck genau um einen Schritt.
settings-ui-smoothing = Glättung des Wasserfalls
settings-ui-smoothing-interpolate = Interpolieren
    .hint = Mischt jeden Bildschirmpunkt mit den umliegenden Frequenzpunkten und Zeilen, damit ein Signal zusammenhängend aussieht, wenn die Anzeige breiter ist als die Transformation. Für einen rechteckigen Wasserfall ausschalten — ein Block pro Frequenzpunkt, eine Zeile pro Durchlauf —, was nötig ist, um die Form eines Signals am Bild abzulesen: Ein interpoliertes Signal ist von einem tatsächlich breiteren nicht zu unterscheiden. Eine größere FFT (der FFT-Chip) ist die andere Hälfte dieser Lösung.
settings-ui-clock = Uhr
settings-ui-clock-show = UTC + Ortszeit auf dem Panorama
    .hint = Eine Wanduhr oben rechts im Panorama, in UTC und in Ihrer Zeit.
settings-ui-background = Hintergrund des Spektrums
settings-ui-background-gradient = Verlauf
settings-ui-background-top = oben
settings-ui-background-bottom = unten
settings-ui-spot-colours = Farben der Spot-Beschriftungen
    .hint = Die Tönung jeder Spot-Quelle: die Kästchen entlang der Unterkante des Wasserfalls, die Plaketten in der Liste SPOTS und die Punkte auf der Weltkarte.
settings-ui-spot-colour = Farbe für { $kind }-Spots
settings-ui-spot-colours-reset = Jede Spot-Farbe auf ihren Standard zurücksetzen
settings-ui-reset = Zurücksetzen
settings-ui-bandplan-colours = Farben des Bandplans
    .hint = Der Farbton jeder Zuteilungsklasse auf dem Bandplan-Streifen entlang der Unterkante des Wasserfalls. Die Blöcke werden halbtransparent über den Wasserfall gelegt und wirken dort dunkler als das Muster hier.
settings-ui-bandplan-colour = Farbe für Zuteilungen { $kind }
settings-ui-bandplan-colours-reset = Jede Bandplan-Farbe auf ihren Standard zurücksetzen
settings-ui-skimmer-font = Schriftgröße Skimmer
settings-ui-waterfall-font = Schriftgröße Wasserfall / Spektrum
settings-ui-interface-font = Schriftgröße der Oberfläche
    .hint = Skaliert die gesamte Oberfläche — Menüs, Dialoge, Fenster, die Funkreiter, die obere Leiste und ihre Tasten. Die beiden Größen oben beziehen sich darauf.
settings-ui-cities = Städte auf Karten
    .hint =
        Die Städte der Welt — ein Punkt pro Ort, mit dem Namen daneben, wo Platz ist — auf den flachen Karten anzeigen: FT8/WSPR, APRS, ADS-B und AIS.

        Sie zeigen vor allem, *wo* ein Punkt auf einer Weltkarte liegt, und sie sind zugleich das Unruhigste darauf. Schalten Sie sie aus, wenn die Namen die Stationen stören, die Sie lesen. Der 3D-Globus ist nicht betroffen — seine Städte sind Lichter auf der Nachtseite und keine Markierungen, und dort wird nichts über einen Kontakt geschrieben.
settings-ui-cities-show = Städte und ihre Namen anzeigen
settings-ui-update-check = Beim Start nach Updates suchen
    .hint = Fragt einmal pro Start bei sdroxide.com nach, ob eine neuere Version veröffentlicht wurde, und meldet das in einem schließbaren Banner über dem Wasserfall. Außer der Anfrage selbst wird nichts gesendet.
settings-ui-band-conditions-download = Bandbedingungs-Vorhersage herunterladen
    .hint = Ruft höchstens einmal pro Stunde die KW-Bandbedingungs-Vorhersage von HAMQSL.com ab, um das Bandmenü einzufärben. Ausgeschaltet zeigt das Bandmenü keine Vorhersage, außer die 3D-Ansicht ist geöffnet. Außer der Anfrage selbst wird nichts gesendet.
settings-ui-broadcast-schedule-download = Rundfunk-Sendeplan herunterladen
    .hint = Lädt den EiBi-Kurzwellen-Sendeplan (etwa 1 MB) von eibispace.de, wenn eine neue Rundfunksaison beginnt. Ausgeschaltet bleibt der in sdroxide eingebaute (oder der zuletzt heruntergeladene) Plan in Gebrauch; Einstellungen → Spots → „Plan jetzt herunterladen“ lädt ihn weiterhin auf Wunsch.
settings-ui-3d-view = 3D-Ansicht
settings-ui-clouds = Wolkendarstellung
settings-ui-clouds-layered = Geschichtet
settings-ui-clouds-volumetric = Volumetrisch
settings-ui-clouds-note = Wie die Ebene WOLKEN in der 3D-Ansicht das Wetter zeichnet. Geschichtet staffelt Scheiben durch die Troposphäre und ist die sparsame Option. Volumetrisch lässt einen Strahl hindurchlaufen, sodass die Sonne die Wolkenspitzen auf den Boden darunter wirft und Blitze durch das entstehende Gewitter leuchten, statt nur dessen Außenseite aufzuhellen — zum mehrfachen Rechenaufwand je Bildpunkt.

## Settings window, UI tab: voice announcements.

speech-title = Sprachansagen
speech-enabled = Änderungen am Funkgerät ansagen
    .hint = Liest vor, was sich geändert hat, damit das Funkgerät bedient werden kann, ohne es anzusehen
speech-voice = Stimme
speech-voice-shipped = Mitgelieferte Stimme
speech-speed = Geschwindigkeit
    .hint = Die Stimme dehnt oder staucht ihre eigene Sprechweise, ohne die Tonhöhe zu ändern. Ab etwa 2 × wird sie nicht mehr kürzer.
speech-volume = Lautstärke
speech-output = Ausgang
speech-detail = Ausführlichkeit
speech-duck = Empfänger absenken
speech-duck-while = Während des Sprechens
speech-test = Test
speech-what = Was angesagt wird
speech-cat-frequency = Frequenz
speech-cat-mode-band = Betriebsart und Band
speech-cat-vfo-split = VFO und Split
speech-cat-agc-gain = AGC und Verstärkung
speech-cat-levels = Aussteuerung, Abstimmung und Mikrofon
speech-cat-ptt = Senden und Empfangen
speech-cat-rit-xit = RIT und XIT
speech-cat-memory-scan = Speicher und Suchlauf
speech-cat-band-edge = Verlassen eines Amateurfunkbandes
speech-cat-notices = Warnungen und Meldungen
speech-cat-filters = Filter, Rauschsperre und Rauschunterdrückung
    .hint = Standardmäßig aus: Diese Werte ändern sich ständig, während man einem Signal nachgeht
speech-decoded = Dekodierte Meldungen
speech-ft8-to-me = FT8-Rufe an mich
speech-ft8-cq = FT8-CQ, auf die ich antworten könnte
    .hint = Ein lebhafter Abend auf zwanzig Metern bringt hundert davon pro Minute
speech-js8 = JS8-Meldungen an mich
speech-js8-allcall = JS8 @ALLCALL auch
speech-fsq = FSQ-Meldungen an mich
speech-include-snr = Den Bericht einbeziehen
    .hint = Nur dort, wo die Meldung keinen eigenen trägt — eine Dekodierung, die bereits eine Zahl angibt, bekommt nicht zusätzlich unsere
speech-ft8-qso = Mein eigener FT8-Austausch
    .hint = Was der Sequenzer als Nächstes senden wird, und das Ende des Kontakts
speech-reading-text = Dekodierten Text laut vorlesen
speech-text-cw = CW
speech-text-rtty-psk = RTTY, PSK, Olivia, THOR, FSQ
speech-text-cw-locked = CW nur, solange der Decoder einrastet
    .hint = Die Ausgabe eines nicht eingerasteten Decoders vorzulesen ist schlechter als Stille
speech-text-note = Beides ist standardmäßig aus. Ein Decoder erzeugt Text schneller, als die Sprache ihn vorliest; was zu weit hinter dem Live-Audio zurückfällt, wird deshalb verworfen statt eingereiht.
speech-tuning-up = Abstimmen
speech-swr-while-tuning = Das SWR ansagen, solange TUNE gedrückt ist
speech-swr-every = Alle
speech-swr-summary = Beim Loslassen den besten Wert melden
speech-swr-alarm = Bei hohem SWR während jeder Aussendung warnen
speech-how-read = Wie Dinge vorgelesen werden
speech-frequencies = Frequenzen
speech-callsigns = Rufzeichen
speech-quiet-on-tx = Beim Senden still bleiben
    .hint = Die Sprache geht über Ihre Lautsprecher und damit in Ihr Mikrofon. Warnungen bei hohem SWR kommen trotzdem durch.
speech-device-note = Ansagen laufen über ein eigenes Tonausgabegerät und werden daher nie aufgezeichnet und nie an Mithörer aus der Ferne gesendet. Die Tasten zum Ansagen des Status, zum Wiederholen der letzten Ansage und zum Abbrechen mitten im Satz befinden sich im Reiter Bedienung.

## Settings window (src/app/settings/mod.rs)

settings-waiting-station-s-network-configuration = Warte auf die Netzwerkkonfiguration der Station…
settings-device-reports = wie das Gerät es meldet
settings-transverters = Transverter
settings-one-row-per-box-while = Eine Zeile pro Kasten. Liegt die Skala im Band einer Zeile, wird das Funkgerät auf Skala + Versatz abgestimmt, und die Sendeerlaubnis und Aussteuerungsgrenze dieser Zeile gelten; außerhalb jeder Zeile arbeitet das Funkgerät in seinen eigenen Bändern. Die Zeilen werden von oben nach unten geprüft.
settings-name = Name
settings-band-low = Bandanfang
settings-band-high = Bandende
settings-offset = Versatz
settings-transmit = Senden
settings-max-drive = Max. Aussteuerung
settings-off-takes-transverter-out-line = Aus nimmt diesen Transverter aus der Leitung, ohne die Zeile zu verlieren — das Band geht zurück an das, was das Funkgerät selbst erreicht.
settings-t-2-m = 2 m
settings-what-you-call-shown-log = Wie Sie es nennen. Wird im Logbuch angezeigt, wenn die Skala es auswählt.
settings-bottom-band-transverter-works-dial = Der untere Rand des Bandes, das dieser Transverter bedient, auf der Skala. 144 für 2 m.
settings-top-band-dial-148-2 = Der obere Rand des Bandes, auf der Skala. 148 für 2 m in den Regionen 2 und 3, 146 in Region 1.
settings-radio-ends-up-dial-offset = Das Funkgerät landet auf Skala + Versatz; ein Transverter, der ein Band auf eine ZF herunterholt, hat deshalb einen negativen Wert: 2 m in eine 28-MHz-ZF ist -116, weil 144 - 116 = 28. Zum Feinabgleich bei einem leicht verstimmten Oszillator Hertz für Hertz ziehen.
settings-off-while-converting-receive-only =
    Aus beim Umsetzen: nur Empfang — die Zeile ist ein Konverter, kein Transverter, und auf diesem Band wird nichts getastet.

    Über denselben Konverter: Er arbeitet in beide Richtungen, und Senden nutzt denselben Versatz.
settings-ceiling-transmit-drive-while-transverter =
    Eine Obergrenze für die Sendeaussteuerung, während dieser Transverter gewählt ist, als Prozentsatz der Vollaussteuerung. Der ZF-Eingang eines Transverters verträgt Milliwatt, und die Aussteuerung, die für die eigenen Bänder des Funkgeräts richtig ist, würde ihn zerstören; deshalb ist dies das wichtigste Feld der Zeile. 100 % bedeutet keine Begrenzung.

    Ihre Einstellung Aussteuerung wird darunter gehalten statt verschoben, sodass der Wert, den Sie auf KW nutzen, zurückkommt, sobald die Skala das Band verlässt.
settings-remove = ENTFERNEN
settings-add-transverter = TRANSVERTER HINZUFÜGEN
settings-takes-effect-apply-reconnect-like = Wirkt nach Übernehmen / erneut verbinden, wie der Konvertierungsversatz oben.
settings-maximum-transmit-drive = Maximale Sendeaussteuerung
settings-hard-limit-drive-tune-controls = Eine harte Grenze, über die die Regler Aussteuerung und TUNE nicht hinausgehen können; sie wirkt nach der Bandkalibrierung unten, sodass nichts die Aussteuerung darüber heben kann. Setzen Sie sie dort, wo das Funkgerät seine Nennleistung erreicht, dann ist der ganze Regler Aussteuerung nutzbar — bei einem Sender, der seine eigenen Abtastwerte moduliert (HPSDR, LimeSDR, PlutoSDR, HackRF), ist volle Aussteuerung der Sender voll aufgedreht, was bei einem Verstärker mit hoher Verstärkung weit über seine Endstufen-Nennwerte hinausgeht. Aus bedeutet, dass die Regler die volle Aussteuerung erreichen.
settings-limit-drive = Aussteuerung begrenzen auf
settings-per-cent-full-drive-key = Prozent der vollen Aussteuerung. Das Funkgerät in eine Kunstlast tasten und diesen Wert so weit senken, bis das Messgerät die Leistung anzeigt, für die der Verstärker ausgelegt ist.
settings-applies-immediately-every-band-tune = Wirkt sofort, auf jedem Band und auch auf TUNE.
settings-transmit-drive-band = Sendeaussteuerung nach Band
settings-every-amplifier-has-different-gain = Jeder Verstärker hat auf jedem Band eine andere Verstärkung, deshalb erzeugt eine Einstellung der Aussteuerung auf jedem Band eine andere Leistung. Messen Sie die Ausgangsleistung auf jedem Band und korrigieren Sie dann die Bänder, die zu hoch liegen, bis alle übereinstimmen: Stellen Sie die Aussteuerung für das Band ein, das am meisten braucht (meist das höchste), und senken Sie die anderen darauf ab. Dezibel der Ausgangsleistung, angewendet auf Sprache, Digitalbetrieb und TUNE gleichermaßen. Überall null — der Standard — ändert nichts.
settings-other = Andere
settings-clear-all = ALLE LÖSCHEN
settings-back-no-calibration-drive-setting = Zurück zu keiner Kalibrierung: Die Einstellung Aussteuerung erreicht jedes Band unverändert.
settings-applies-immediately-band-you-would = Wirkt sofort und auf das Band, auf dem Sie senden würden — hinter einem Transverter ist das das Band auf der Skala, und die eigene Aussteuerungsgrenze des Transverters hat immer Vorrang vor allem, was hier eingestellt ist.
settings-settings = Einstellungen
settings-enter-server-s-address-first = Zuerst die Adresse des Servers eingeben
settings-enter-receiver-s-address-first = Zuerst die Adresse des Empfängers eingeben oder einen aus den öffentlichen SDRs wählen
settings-no-radio-selected-press-discover = Kein Funkgerät gewählt — Suchen drücken oder eine Adresse eingeben
settings-enter-radio-s-address-first = Zuerst die Adresse des Funkgeräts eingeben
settings-general = Allgemein
settings-radio = Funkgerät
settings-ui = Oberfläche
settings-alerts = Hinweise
settings-controls = Bedienung
settings-spots = Spots
settings-freedv = FreeDV
settings-uploads = Uploads
settings-winlink = Winlink
settings-servers = Server
settings-t-r-switch = Sende-/Empfangsschalter
settings-amplifier = Verstärker
settings-remote = Fernsteuerung
settings-tle = TLE
settings-profiles = Profile
settings-station = Station
settings-enter-digital-mode-ft8-sstv = Einen digitalen Modus (FT8 / SSTV / …) einmal aufrufen, um die gespeicherten Werte zu laden.
settings-callsign = Rufzeichen
settings-grid-square = Locator
settings-your-callsign-grid-shared-across = Ihr Rufzeichen und Ihr Locator, gemeinsam für FT8/FT4/FT2, SSTV-Bildköpfe und das Logbuch genutzt. Auch im Einrichtungsdialog FT8 / SSTV bearbeitbar.
settings-iaru-region = IARU-Region
settings-which-radio-interface-server-uses = Welche Funkschnittstelle der Server nutzt, wird auf dem Rechner eingestellt, auf dem er läuft.
settings-radio-interface = Funkschnittstelle
settings-waiting-machine-radio-attached-interface = Warte auf den Rechner, an dem das Funkgerät angeschlossen ist. Seine Schnittstelleneinstellungen stehen unten und können inzwischen von hier aus geändert werden.
settings-converter = Konverter
settings-manual-type-offset-below = Manuell — Versatz unten eingeben
settings-what-box-reads-when-offset = Was dieses Feld anzeigt, wenn der Versatz keiner der Voreinstellungen entspricht. Man setzt ihn, indem man ihn unten in das Feld Versatz eingibt; hier gibt es nichts auszuwählen.
settings-frequency-converter-front-receiver-pick = Ein Frequenzkonverter vor dem Empfänger. Einen wählen oder daneben einen Versatz eingeben.
settings-how-far-converter-moves-signal =
    Wie weit ein Konverter das Signal auf dem Weg zum Empfänger verschiebt, in Hz — dieselbe Zahl und dasselbe Vorzeichen, die jede Konverter-Dokumentation und jedes andere SDR-Programm angibt. Das Funkgerät landet auf Skala + Versatz: positiv bei einem Aufwärtsumsetzer (ein Ham It Up ist 125000000), negativ bei allem, was ein Band auf eine ZF herunterholt — ein Satelliten-LNB oder ein 2-m-Transverter auf einer 28-MHz-ZF, das ist -116000000 (28 − 144). 0 = kein Konverter.

    Hertz für Hertz ziehen, zum Feinabgleich, den ein Konverter mit leicht verstimmtem Oszillator braucht.

    Dies ist der Empfangsweg. Was in der Sendeleitung liegt, steht in der Zeile darunter.

    Wirkt nach Übernehmen.
settings-what-transmit-line-while-converter =
    Was in der Sendeleitung liegt, während ein Konverter eingestellt ist.

    Aus beim Umsetzen: Es wird nichts gesendet — der sichere Standard, und richtig für ein reines Empfangszubehör.

    Über denselben Konverter: Ein Kasten setzt in beide Richtungen um, wie ein Transverter. Senden nutzt den Versatz oben und folgt ihm, wenn er nachjustiert wird.

    Eigener Versatz: Die Sendeleitung ist eine andere als die Empfangsleitung. Null ist der übliche Fall — ein Konverter (ein LNB, ein Ham It Up) beim Empfang, mit dem Sender an seiner eigenen Antenne, das ist die QO-100-Station.

    Die Prüfung auf Amateurfunkbänder gilt weiterhin, auf der Frequenz, auf der Sie senden.

    Wirkt nach Übernehmen.
settings-how-far-transmit-line-moves =
    Wie weit die Sendeleitung das Signal verschiebt, in Hz, nach derselben Vorzeichenregel wie der Empfangsversatz: Das Funkgerät sendet auf Skala + Versatz.

    0 = der Sender ist an seine eigene Antenne angeschlossen und arbeitet auf der Frequenz, die die Skala angibt — der 2,4-GHz-Uplink einer QO-100-Station.

    Ein Sendekonverter, der eine ZF *nach oben* umsetzt, hat einen negativen Wert: Das Funkgerät arbeitet unterhalb der Skala.
settings-rx-range = Empfangsbereich
settings-which-frequencies-radio-receives-mhz =
    Welche Frequenzen dieses Funkgerät empfängt, in MHz: 144-146, 430-440. Leer lassen, um zu verwenden, was das Gerät über sich selbst meldet.

    Bandtasten außerhalb des Bereichs sind ausgegraut, und die Skala lässt sich dort nicht hinstellen.

    Dies sind Skalenfrequenzen. Bei eingestelltem Konverter das Band angeben, das Sie abstimmen — 144-148 für einen 2-m-Transverter —, nicht die ZF, auf der das Funkgerät tatsächlich liegt.

    Wirkt nach Übernehmen.
settings-tx-range = Sendebereich
settings-which-frequencies-radio-transmits-mhz =
    Auf welchen Frequenzen dieses Funkgerät sendet, in MHz: 144-146, 430-440. Leer lassen, um zu verwenden, was das Gerät meldet — und meldet es nichts, wird der Treiber beim Wort genommen und jede Frequenz ist erlaubt.

    Skalenfrequenzen, wie beim Empfangsbereich oben.

    Dies ist eine Grenze, die Sie setzen, keine Lizenz: Senden außerhalb der Amateurfunkbänder wird ohnehin verweigert, sofern Sie das nicht in config.toml abgeschaltet haben. Auch einem reinen Empfangsgerät verschafft das keinen Sender.

    Wirkt nach Übernehmen.
settings-antenna = Antenne ist
settings-where-radio-listens-station-antenna =
    Von wo dieses Funkgerät hört.

    An der Station: Die Antenne gehört Ihnen, daher sagt Ihr eigener Locator, wo sie steht. Der Standard, und richtig für alles im Shack.

    Anderswo: ein Online-Empfänger oder Ihr eigener Aufbau auf einem Berg. Was er hört, wird über den Locator daneben an PSK Reporter, WSPRnet und FreeDV Reporter gemeldet, nie über Ihren eigenen — und ADS-B ordnet Flugzeuge ebenfalls danach ein.

    Wählen Sie einen Empfänger unter „Öffentliche SDRs“, wird dies für Sie ausgefüllt.

    Wirkt nach Übernehmen.
settings-receiver-s-maidenhead-locator-jn88ec =
    Der Maidenhead-Locator des Empfängers — JN88ec, DO30db. Vier oder sechs Zeichen.

    Leer lassen, wenn Sie nicht wissen, wo der Empfänger steht: Dann wird überhaupt nichts gemeldet, was die einzig ehrliche Antwort ist. Eine Meldung aus Ihrem eigenen Quadrat würde den Empfang eines anderen auf den falschen Kontinent setzen.
settings-audio = Audio nach
settings-gain-per-band = Verstärkung je Band
settings-remember-receive-gain-per-band = Empfangsverstärkung je Band merken
settings-each-band-keeps-receive-gain = Jedes Band behält die Empfangsverstärkung, die Sie zuletzt darauf eingestellt haben, und bekommt sie zurück, wenn die Skala dorthin zurückkehrt — eine Bandtaste, ein Ziehen über eine Bandgrenze oder der eigene Drehknopf des Funkgeräts. Die allgemeine Abdeckung ist in KW, UKW, UHF und höher unterteilt. Für ein Band, dessen Verstärkung Sie nicht gesetzt haben, bleibt, was gerade gilt. Aus: eine Verstärkung überall.
settings-s-meter-calibration = S-Meter-Kalibrierung
settings-added-radio-s-s-meter =
    Wird zum Messwert dieses S-Meters des Funkgeräts addiert. Das Messgerät eines SDR zeigt zunächst dBFS an — Pegel gegen Vollaussteuerung des Konverters —, daher kann ein Empfänger mit viel Verstärkung S9 zeigen, ohne dass eine Antenne angeschlossen ist. Tragen Sie hier die Differenz zwischen dem ein, was das Messgerät anzeigt, und einem Pegel, den Sie kennen: ein Signalgenerator oder ein ruhiges Band, von dem Sie wissen, dass es bei etwa S2–S3 liegt. Wirkt sofort.

    Zusätzlich zu cal_offset_db in config.toml, das ein einziger Wert für die ganze Station ist.
settings-t-0-db = 0 dB
settings-antenna-somewhere-else-no-locator = Die Antenne steht woanders und es ist kein Locator gesetzt, daher wird nichts, was dieses Funkgerät hört, an PSK Reporter, WSPRnet oder FreeDV Reporter gemeldet. Geben Sie oben den Locator des Empfängers ein, um von dort aus zu melden.
settings-ranges-are-mhz-low-high = Bereiche sind in MHz angegeben, untere-obere Grenze, durch Kommas getrennt: 144-146, 430-440 — das sind 144000000-146000000 Hz und 430000000-440000000 Hz. Es sind Skalenfrequenzen: Ein Konverter-Versatz verschiebt sie nicht. Der Konverter-Versatz oben ist das Feld in Hertz. Einen Bereich leer lassen, um zu verwenden, was das Gerät über sich selbst meldet; ein Gerät, das nichts meldet, wird beim Wort genommen.
settings-dwell-recentre = Fenster auf ruhende Abstimmung zentrieren
settings-dwell-recentre-hover = Ruht die Abstimmung 5 s mehr als ein Zwanzigstel des Fensters neben seiner Mitte, wird das I/Q-Fenster einmal auf sie zentriert. Aus: Das Fenster bleibt, bis die Abstimmung es verlässt.
settings-careful-rtl-sdr-blog-v4 = Vorsicht bei einem RTL-SDR: Der Blog V4 setzt unterhalb von 28,8 MHz selbst aufwärts um, daher wird ein negativer Versatz, der die Hardware dorthin bringt, zweimal verschoben.
settings-transmit-audio = Sendeaudio
settings-use = in Benutzung
settings-every-stage-off = jede Stufe aus
settings-open-tx-audio = TX-Audio öffnen…
settings-microphone-processing-chain-gate-eq = Die Mikrofon-Verarbeitungskette — Gate, Equalizer, De-Esser, Kompressor und Begrenzer — mit einer Testaufnahme, um sie nach Gehör abzustimmen, ohne zu senden. Nur für Sprachbetriebsarten. Auch über einen Klick auf Mic im Sendefeld geöffnet.
settings-pick-radio-interface-above-configuration = Oben eine Funkschnittstelle wählen (diese Konfiguration nutzte den entfernten Automatik-Modus).
settings-configuration-used-removed-auto-detect = Diese Konfiguration nutzte den entfernten Automatik-Modus. Wählen Sie oben eine Schnittstelle, sobald der Rechner, an dem das Funkgerät angeschlossen ist, geantwortet hat.
settings-radio-has-no-interface-yet = Dieses Funkgerät hat noch keine Schnittstelle — oben eine wählen und Übernehmen / erneut verbinden drücken.
settings-radio-has-no-interface-yet-2 = Dieses Funkgerät hat noch keine Schnittstelle. Oben kann eine gewählt werden, sobald der Rechner, an dem das Funkgerät angeschlossen ist, geantwortet hat.
settings-apply-reconnect = Übernehmen / erneut verbinden
settings-switch-interface-now-no-restart = Jetzt auf diese Schnittstelle wechseln — kein Neustart nötig
settings-switch-server-s-radio-interface = Das Funkgerät des Servers jetzt auf diese Schnittstelle umstellen — kein Neustart nötig. Die alte läuft weiter, falls die neue sich nicht öffnen lässt.
settings-switches-live-radio-without-restarting = Schaltet das laufende Funkgerät ohne Neustart um.
settings-everything-above-server-s-own = Alles oben ist die eigene Konfiguration des Servers, und Änderungen werden dort gespeichert. Die meisten Einstellungen wirken, sobald Sie sie ändern; die, die beim Öffnen des Geräts festgelegt werden — die Abtastrate, eine Adresse, die Schnittstelle selbst — brauchen diese Taste.
settings-dx-cluster-telnet = DX-Cluster (Telnet)
settings-enabled = Aktiviert
settings-host = Host
settings-port = Port
settings-login-call = Anmelderufzeichen
settings-commands = Befehle
settings-one-per-line-e-g = einer pro Zeile, z. B. SET/FT8
settings-reverse-beacon-network = Reverse Beacon Network
settings-read-world-s-cw-rtty = Die CW-/RTTY-Skimmer der Welt lesen und die Ausbreitungskarte damit füttern. Dadurch zeigt die Karte Bänder an, auf die dieses Funkgerät nicht hört. Standardmäßig an: Es sendet nichts auf die Frequenz, braucht kein Konto und nutzt das Rufzeichen aus dem Reiter Allgemein. RBN-Spots erscheinen nicht in der Spot-Liste — sie sind Messungen, keine Einladungen.
settings-t-7000-cw-rtty-feed-7001 = 7000 ist der CW-/RTTY-Feed, 7001 der FT8/FT4-Feed
settings-one-per-line-e-g-2 = einer pro Zeile, z. B. set/filter cont=eu
settings-sent-after-login-place-narrow = Nach der Anmeldung gesendet. Hier lässt sich der Feed eingrenzen — ohne Filter ist das jeder Skimmer auf der Erde.
settings-rbn-paths-are-placed-country = RBN-Verbindungen werden von Landesmitten aus eingetragen, nicht von Locatoren — für ein kleines Land genau, für ein großes weit daneben. Sie bilden eine eigene Ebene der Ausbreitungskarte und lassen sich dort ausschalten.
settings-pota-sota-psk-reporter = POTA / SOTA / PSK Reporter
settings-pota-activator-spots = POTA-Aktivator-Spots
settings-sota-spots = SOTA-Spots
settings-psk-reporter-current-band = PSK Reporter (aktuelles Band)
settings-upload-my-ft8-ft4-ft2 = Meine FT8/FT4/FT2/JS8-Dekodierungen hochladen
settings-report-what-station-hears-pskreporter = Meldet, was diese Station hört, an pskreporter.info, sodass sie dort als Empfänger erscheint. Nutzt das Rufzeichen und den Locator aus dem Reiter Allgemein.
settings-antenna-2 = Antenne
settings-collector = Sammler
settings-t-4739-live-collector-14739-test = 4739 ist der Live-Sammler, 14739 der Testsammler
settings-max-age-s = Höchstalter (s)
settings-wsprnet = WSPRnet
settings-upload-my-wspr-decodes = Meine WSPR-Dekodierungen hochladen
settings-send-every-wspr-reception-wsprnet = Jeden WSPR-Empfang an wsprnet.org senden. Standardmäßig an: Es sendet nichts auf die Frequenz, und das Melden dessen, was Sie hören, macht einen WSPR-Empfänger zum Teil des Netzwerks statt zu einer privaten Spielerei. Ein Zeitabschnitt ohne Dekodierung wird ebenfalls gemeldet — so erkennt das Netzwerk ein totes Band von einem abgeschalteten Empfänger.
settings-download-who-heard-me = Herunterladen, wer mich gehört hat
settings-ask-wsprnet-org-which-stations = Bei wsprnet.org nachfragen, welche Stationen diese dekodiert haben. WSPR kennt keinerlei Empfangsbestätigung, daher ist dies der einzige Weg, auf dem eine sendende Bake etwas über ihre Reichweite erfährt.
settings-ask-every = Abfragen alle
settings-looking-back = zurückblickend
settings-apply = {" "}ÜBERNEHMEN{" "}
settings-persist-re-connect-feeds = Die Feeds speichern und (erneut) verbinden
settings-broadcast-stations = Rundfunksender
settings-winlink-account = Winlink-Konto
settings-password = Passwort
settings-winlink-account-password-not-gateway = Das Passwort des Winlink-Kontos, nicht das des Gateways. Es unterscheidet Groß- und Kleinschreibung — geben Sie es genau so ein, wie es ausgegeben wurde.
settings-locator = Locator
settings-how-connect = Wie verbinden
settings-route = Weg
settings-internet = Internet
settings-forward-cms-over-internet = Mit dem CMS über das Internet weiterleiten
settings-radio-packet = Funk (Paket)
settings-call-rms-gateway-air-radio = Ein RMS-Gateway über Funk anrufen. Das Funkgerät muss auf PACKET oder PACKET-HF stehen.
settings-gateway = Gateway
settings-via = Über
settings-digipeaters-order-separated-spaces-usually = Digipeater in der Reihenfolge, durch Leerzeichen getrennt. Meist leer — ein Gateway, das Sie direkt hören, ruft man auch direkt an.
settings-speed = Geschwindigkeit
settings-most-rms-gateways-answer-1200 = Die meisten RMS-Gateways antworten mit 1200. Ein 1200-Gateway mit 9600 anzurufen klingt genau wie ein Gateway, das außer Betrieb ist; stellen Sie das deshalb nach dem ein, was der Betreiber des Gateways veröffentlicht, statt zu raten. 9600 braucht außerdem den Datenanschluss des Funkgeräts an beiden Enden. 300 ist KW.
settings-frequency = Frequenz
settings-clear = LÖSCHEN
settings-zero-leaves-dial-alone-which = Null lässt die Skala in Ruhe, was Sie wollen, wenn Sie auf einem einzigen Kanal bleiben. Alles andere stimmt das Funkgerät beim Start der Sitzung ab.
settings-my-gateways = Meine Gateways
settings-winlink-s-published-gateway-list = Die veröffentlichte Gateway-Liste von Winlink braucht einen API-Schlüssel, den sdroxide nicht hat; führen Sie deshalb Ihre eigene. Die zwei oder drei Gateways, die von einem Standort aus erreichbar sind, lernt man durch Ausprobieren, und sie ändern sich selten.
settings-use-2 = NUTZEN
settings-call-one-next-connect = Dieses beim nächsten Verbinden anrufen
settings-current-dial = aktuelle Skala
settings-forget = VERGESSEN
settings-forget-gateway = Dieses Gateway vergessen
settings-add-gateway = + GATEWAY HINZUFÜGEN
settings-save-gateway-above-list = Das Gateway oben in die Liste speichern
settings-internet-gateway = Internet-Gateway
settings-cms-address = CMS-Adresse
settings-client-name = Clientname
settings-winlink-s-production-servers-only = Die Produktivserver von Winlink nehmen nur Clientnamen an, die sie kennen, und beantworten einen unbekannten mit „Unknown client types are not allowed on production servers“. Bis sdroxide beim Winlink Development Team registriert ist, braucht das Verbinden einen Namen, den sie erkennen.
settings-automatic-connection = Automatische Verbindung
settings-connect-timer = Verbinden per Zeitgeber
settings-every = Alle
settings-persist-winlink-account = Das Winlink-Konto speichern
settings-callsign-lookup = Rufzeichenabfrage
settings-provider = Anbieter
settings-auto-fill-name-qth-grid = Name/QTH/Locator beim Klick auf einen Spot und bei QSO automatisch ausfüllen
settings-choose-provider-enter-login = Einen Anbieter wählen, um seine Anmeldung einzugeben.
settings-qrz-user = QRZ-Benutzer
settings-qrz-pass = QRZ-Passwort
settings-hamqth-user = HamQTH-Benutzer
settings-hamqth-pass = HamQTH-Passwort
settings-one-hamqth-account-does-both = Ein HamQTH-Konto erledigt beides — diese Anmeldung nutzt der Reiter HamQTH-Upload.
settings-upload = Upload
settings-auto-upload-each-new-qso = Jedes neue QSO automatisch hochladen
settings-master-switch-which-services-qso = Der Hauptschalter. Welche Dienste ein QSO erhält, wird unten in jedem Reiter eingestellt.
settings-turn-auto-upload-each-new = Schalten Sie zuerst oben „Jedes neue QSO automatisch hochladen“ ein — ein Dienst, der hier angehakt ist, wird erst gesendet, wenn dies aktiv ist.
settings-auto-upload-off-above-so = Der automatische Upload ist oben aus, daher sind die Dienst-Häkchen deaktiviert und nichts wird bisher automatisch gesendet — die Taste HOCH pro QSO im Logbuch funktioniert weiterhin.
settings-qrz-log-key = QRZ-Logbuchschlüssel
settings-qrz-logbook-api-key-your = Der QRZ-Logbuch-API-Schlüssel aus Ihren QRZ-Logbuch-Einstellungen — nicht die Anmeldung für die XML-Abfrage.
settings-eqsl-user = eQSL-Benutzer
settings-eqsl-pass = eQSL-Passwort
settings-eqsl-qth = eQSL-QTH
settings-qth-nickname-eqsl-profile-upload = Der QTH-Spitzname des eQSL-Profils, zu dem hoch- und von dem heruntergeladen wird — nur nötig, wenn das Konto mehr als eines besitzt; sonst leer lassen.
settings-same-login-hamqth-callsign-lookup = Dieselbe Anmeldung wie bei der HamQTH-Rufzeichenabfrage — eine der beiden auszufüllen füllt beide aus.
settings-club-log-email = Club-Log-E-Mail
settings-club-log-pass = Club-Log-Passwort
settings-club-log-key = Club-Log-Schlüssel
settings-wrl-api-key = WRL-API-Schlüssel
settings-developer-api-key-world-radio = Der Entwickler-API-Schlüssel aus World Radio League → Integrations → Developer API. Er wird nur einmal bei der Erzeugung angezeigt, kopieren Sie ihn also dann. Verbindungen gehen in Ihr Standardlogbuch — legen Sie in WRL eines fest, wenn Sie mehr als eines führen.
settings-confirmations-download = Bestätigungen (Herunterladen)
settings-lotw-user = LoTW-Benutzer
settings-lotw-pass = LoTW-Passwort
settings-lotw-upload-uses-tqsl-export = Der LoTW-Upload nutzt TQSL — ADIF aus dem Logbuch exportieren und signieren. LoTW- und eQSL-Bestätigungen werden hier heruntergeladen, um gearbeitet gegen bestätigt zu markieren.
settings-sync-confirmations = BESTÄTIGUNGEN ABGLEICHEN
settings-fetching-subscriptions = Abonnements werden abgerufen…
settings-spectrum-waterfall-everything-read-off =
    Sein Spektrum, sein Wasserfall und alles, was davon abgelesen wird, stehen auf dem Reiter dieses Funkgeräts, deshalb hat es keinen eigenen. Die Oberflächeneinstellungen oben gehören weiterhin diesem Funkgerät und gelten weiterhin — nach dem Ändern Übernehmen / erneut verbinden drücken, dann kommen beide Funkgeräte gemeinsam wieder hoch.

    Um es wieder allein zu nutzen, das Feld Panorama-Empfänger auf der Seite dieses Funkgeräts leeren.
settings-panadapter = Panorama
settings-give-radio-wideband-panadapter-borrowing = Diesem Funkgerät ein Breitband-Panorama geben, indem der Empfänger eines anderen Funkgeräts ausgeliehen wird — ein SDR an derselben Antenne oder einer, der vom ZF-Ausgang dieses Funkgeräts gespeist wird. Das Spektrum, der Wasserfall, der Subempfänger, die digitalen Betriebsarten und der Skimmer kommen alle von diesem Empfänger; Skala, Betriebsart, Filter und Sender bleiben hier.
settings-receiver = Empfänger
settings-none = Keiner
settings-which-radio-receives-one-leaves =
    Welches Funkgerät für dieses empfängt. Es verlässt die Reiterleiste, solange es verliehen ist — seine Frontend-Stufe gehört zur Engine dieses Funkgeräts — und kehrt sofort zurück, wenn dies auf Keiner gestellt wird.

    Wirkt nach Übernehmen.
settings-connected = Verbunden mit
settings-how-far-above-radio-s =
    Wie weit der Empfänger über der Skala dieses Funkgeräts liegt, in Hertz — 0 für einen Empfänger an derselben Antenne. Bei einem ZF-Abgriff die Zwischenfrequenz, auf der der Empfänger steht: 8215000 für den KXV3B eines K3, 9000000 für eine 9-MHz-ZF.

    Das ist nicht das Feld Konverter weiter oben, das das Funkgerät neu abstimmt, und auch nicht der I/Q-Mittenversatz des CAT-Reiters, der die eigene Soundkarte dieses Funkgeräts betrifft. Hier Eingegebenes geht nie an eines der beiden Funkgeräte.

    Wirkt nach Übernehmen.
settings-audio-2 = Audio von
settings-follow-dial = Der Skala folgen
settings-keep-receiver-radio-s-dial = Den Empfänger auf der Skala dieses Funkgeräts halten und die Skala mitbewegen, wenn Sie auf dem Panorama abstimmen. Aus parkt den Empfänger dort, wo er steht — eine feste Beobachtung eines Abschnitts, während das Funkgerät anderswohin geht.
settings-invert-spectrum = Spektrum umkehren
settings-mirror-receiver-s-span-about = Den Frequenzbereich des Empfängers um seine Mitte spiegeln, für einen Abgriff, dessen Oszillator über dem Signal liegt und das Band verkehrt herum weiterreicht. Lassen Sie es aus, sofern der Wasserfall nicht mit Signalen gefüllt wird, die alle auf der falschen Seite der Skala liegen.
settings-mute-transmit = Beim Senden stummschalten
settings-silence-receive-audio-while-radio = Empfangsaudio stummschalten, solange dieses Funkgerät sendet. Standardmäßig an: Bei einem Empfänger an derselben Antenne oder auf der ZF dieses Funkgeräts ist, was er während einer Aussendung hört, Ihr eigener Sender. Aussendungen, die am Funkgerät selbst getastet werden — die PTT am Mikrofon, ein Fußschalter, MOX — zählen ebenfalls, sobald die CAT-Verbindung meldet, dass das Gerät auf Senden geschaltet hat.
settings-blank-transmit = Beim Senden ausblenden
settings-stop-panadapter-waterfall-while-radio = Panorama und Wasserfall anhalten, solange dieses Funkgerät sendet. Standardmäßig an: Ein Sender, der über den ganzen Bereich malt, löscht das Band dahinter aus. Ausschalten, um das Band — oder Ihr eigenes Signal — während einer Aussendung zu beobachten, was lohnt, wenn der Empfänger an einer eigenen Antenne hängt. Gilt für Aussendungen, die sdroxide tastet; eine am Funkgerät getastete wird durchgehend empfangen, daher läuft das Bild unabhängig von dieser Einstellung weiter.
settings-per-mode-offsets-radio-whose = Versatz je Betriebsart, für ein Funkgerät, dessen ZF mit der Betriebsart wandert. Eine Betriebsart, die leer bleibt, nutzt den Versatz oben.
settings-computer = Auf diesem Computer
settings-add-radio-computer = Ein Funkgerät auf diesem Computer hinzufügen
settings-radio-s-settings-are-below = Die Einstellungen dieses Funkgeräts stehen unten. Zum Verschieben entlang der Leiste ziehen.
settings-switch-radio-dialog-follows-drag = Zu diesem Funkgerät wechseln (der Dialog folgt). Zum Verschieben entlang der Leiste ziehen.
settings-tx = ● SENDEN
settings-station-radio = Station:
settings-station-radio-tip = Das Stationsgerät. Es betreibt die Netzdienste der Station (Spot-Feeds, Rotor, Verstärker, Sende-/Empfangsschalter) und übernimmt die Einstellungen der Befehlszeile, deshalb bleibt es in der Liste. Zum Ruhen mit VERBINDUNG abschalten; für andere Hardware unten eine andere Schnittstelle wählen.
settings-mute-radio-s-audio = Audio dieses Funkgeräts stummschalten
settings-link = VERBINDUNG
settings-close-radio-configuration-kept = Dieses Funkgerät schließen (seine Konfiguration bleibt erhalten)
settings-close-tab-close-radio-station = Diesen Reiter schließen oder das Funkgerät an seiner Station schließen
settings-close-tab = Diesen Reiter schließen
settings-transverters-full = { $max } ist das Maximum, das diese Tabelle fasst
settings-dialling = Wählt { $url }…
settings-region-note = Wo die Station steht. Legt jeden Bandplan fest: die Bandgrenzen, die CW-/Daten-/Sprach-Teilabschnitte auf dem Wasserfall-Streifen, wo die Skimmer hören, die angebotenen Anrufsfrequenzen und was als außerhalb des Bandes für das Senden gilt. In Region { $region } ergibt das für 70 cm { $region ->
        [1] 430–440 MHz — also liegt 446 außerhalb, und 40 m endet bei 7.200
        [2] 420–450 MHz, und 40 m reicht bis 7.300
       *[3] 430–450 MHz, und 80 m endet bei 3.900
    }. Wirkt sofort und gilt für jedes Funkgerät an dieser Station.
settings-country = Land
settings-country-off = — (keine Kanalraster)
settings-country-system = Wie dieser Rechner: { $country }
settings-country-germany = Deutschland
settings-country-note = Welche Jedermannfunk-Kanäle der Wasserfall zeigt; das regelt das Land, nicht die IARU-Region. Deutschland: CB-Kanäle 1–80 (41–80 nur FM, Datenkanäle bernsteinfarben), Freenet 149,025–149,1125 MHz und PMR446 446,00625–446,19375 MHz, je als Block im Bandplan-Streifen und als Kanal-Kacheln darüber. Gilt für diesen Bildschirm; die Vorgabe folgt dem Land, auf das der Rechner eingestellt ist.
settings-rx-grid-reported = Empfänge über dieses Funkgerät werden von { $grid } aus gemeldet, nicht von Ihrem eigenen Locator.
settings-rx-grid-invalid = { $grid } ist kein Maidenhead-Locator, daher wird nichts gemeldet, was dieses Funkgerät hört. Vier oder sechs Zeichen: zwei Buchstaben, zwei Ziffern und optional zwei weitere Buchstaben.
settings-bypassed = überbrückt
# $state is one of settings-bypassed, settings-use or settings-every-stage-off.
settings-tx-audio-chain = Kette { $state }.
settings-gateway-via = { " " }über { $via }
settings-auto-upload-to = Jedes neue QSO automatisch hochladen an { $service }
settings-tle-updated = Aktualisiert: { $n } { $n ->
        [one] Abonnement
       *[other] Abonnements
    }: { $total } Satelliten.
settings-tle-updated-failed = { $ok } von { $n } aktualisiert; { $failed } fehlgeschlagen — siehe die Zeilen oben.
settings-panadapter-of = Dieses Funkgerät ist das Panorama von { $radio }
settings-radio-n = Funkgerät { $n }
settings-on-station = Auf { $station }
settings-add-radio-on-station = Ein Funkgerät auf { $station } hinzufügen — die Station bedient es sofort
settings-panadapter-receiver-for = Panorama-Empfänger für { $radio }
settings-hang-up-stays-on = Auflegen. Das Funkgerät bleibt auf { $station }.
settings-close-on-station = { $radio } auf { $station } schließen? Seine Konfiguration bleibt auf diesem Rechner; der Reiter hier schließt sich mit ihm, und ebenso der aller anderen.
settings-close-it-on = Auf { $station } schließen
settings-locator-hint = Locator

## Settings window, Alerts (src/app/settings/alerts.rs)

settings-alerts-audible-alerts = Hörbare Hinweise
settings-alerts-sound-alarm-when-decode-matters = Einen Alarm ertönen lassen, wenn eine Dekodierung wichtig ist
settings-alerts-plays-over-own-audio-output = Spielt über einen eigenen Audioausgang, sodass es auch dann zu hören ist, wenn das Band über einen anderen Lautsprecher läuft als dieser Bildschirm.
settings-alerts-volume = Lautstärke
settings-alerts-output = Ausgang
settings-alerts-test = Test
settings-alerts-what-sound = Was ertönen soll
settings-alerts-each-station-quiet-while-after = Jede Station bleibt nach einem Hinweis eine Weile still, damit ein belebtes Band nicht bei jedem Zeitabschnitt klingelt. Stimme und Ton + Stimme werden von der gesprochenen Ansage vorgelesen — schalten Sie sie unter Einstellungen → Oberfläche ein.

## Settings window, Amplifier (src/app/settings/amp.rs)

settings-amp-amplifier = Verstärker
settings-amp-waiting-station-s-amplifier-configuration = Warte auf die Verstärkerkonfiguration der Station…
settings-amp-hobbypcb-hardrock-500-hardrock-50 = Ein HobbyPCB Hardrock-500 oder Hardrock-50 an seinem USB-Seriellport. SDRoxide liest seine Messgeräte, schaltet sein Band, seine Betriebsart und Antenne und steuert seinen Tuner aus dem Fenster AMP. Der Verstärker wird wie bisher weiterhin über die PTT-Leitung des Funkgeräts getastet.
settings-amp-enable = Aktivieren
settings-amp-model = Modell
settings-amp-untested-panel-has-only-been = Ungetestet: Dieses Panel wurde nur mit einem Hardrock-500 getestet. Die Messgrenzen für dieses Modell sind von denen des Hardrock-500 abgeleitet und können falsch sein. Achten Sie auf die eigene Anzeige des Verstärkers.
settings-amp-port = Port
settings-amp-choose = — wählen —
settings-amp-baud = Baud
settings-amp-must-match-amplifier-s-menu = Muss zum Menü des Verstärkers passen. Das eigene Bedienfeld von HobbyPCB braucht 115200.
settings-amp-follow-radio-s-band = Dem Band des Funkgeräts folgen
settings-amp-sends-transmit-frequency-amplifier-every = Sendet die Sendefrequenz bei jedem Bandwechsel an den Verstärker, damit seine Filter dem Funkgerät folgen, ohne eigenes CAT-Kabel. Wird nie während einer Aussendung gesendet.
settings-amp-show-power-swr-tx-meter = Seine Leistung und sein SWR auf dem Sendemesser anzeigen
settings-amp-only-where-radio-does-not = Nur dort, wo das Funkgerät sie nicht selbst misst, und nur, solange der Verstärker in der Leitung liegt.
settings-amp-off = Aus
settings-amp-connecting = Verbinde…
settings-amp-status = Status
settings-amp-apply = {" "}ÜBERNEHMEN{" "}
settings-amp-save-re-open-amplifier-s = Den Port des Verstärkers speichern und (erneut) öffnen

## Settings window, Controls (src/app/settings/controls.rs)

settings-controls-keyboard = Tastatur
settings-controls-click-shortcut-rebind-then-press = Klicken Sie auf ein Kürzel, um es neu zuzuweisen, und drücken Sie dann die Tastenkombination (Esc bricht ab). Zuweisungen werden ignoriert, solange Sie in ein Textfeld schreiben.
settings-controls-shortcut = Kürzel
settings-controls-does = Tut
settings-controls-step-mode = Schritt / Betriebsart
settings-controls-accel = Beschleunigung
settings-controls-on = Ein
settings-controls-press-key = Taste drücken…
settings-controls-remove-binding = Diese Zuweisung entfernen
settings-controls-add-shortcut = + Kürzel hinzufügen
settings-controls-restore-defaults = Standardwerte wiederherstellen
settings-controls-bind-hold-talk-space = Sprechen bei gedrückter Taste auf Leertaste legen
settings-controls-hold-space-transmit-releasing-losing = Leertaste halten, um zu senden; Loslassen — oder Verlust des Fensterfokus — hebt die Tastung auf
settings-controls-unkey-held-ptt-after = Eine gehaltene PTT aufheben nach
settings-controls-backstop-against-stuck-key-controller = Sicherung gegen eine klemmende Taste oder einen Controller, der nichts mehr meldet. 0 schaltet ab.
settings-controls-panadapter-mouse = Maus im Panorama
settings-controls-wheel = Rad
settings-controls-wheel-shift = Rad + Umschalt
settings-controls-tune-step = Abstimmschritt
settings-controls-zoom-rate = Zoomgeschwindigkeit
settings-controls-click-tune-rounding = Runden beim Abstimmen per Klick
settings-controls-invert-wheel-direction = Radrichtung umkehren
settings-controls-left-drag-tunes-well-pans = Ziehen mit links stimmt ab wie auch schwenkt
settings-controls-off-makes-left-drag-pan = Aus macht das Ziehen mit links zum reinen Schwenken der Ansicht, wie das Ziehen mit rechts.
settings-controls-scroll-digit-frequency-readout-tune = Eine Ziffer der Frequenzanzeige scrollen, um sie abzustimmen
settings-controls-both-wheel-actions-are-tune = Beide Radaktionen sind Abstimmen — es bleibt kein Weg zum Zoomen.
settings-controls-restore-mouse-defaults = Maus-Standardwerte wiederherstellen
settings-controls-mouse-buttons = Maustasten
settings-controls-left-right-buttons-are-reserved = Die linke und rechte Taste sind für Abstimmen und Schwenken reserviert; die mittlere und die zusätzlichen Tasten sind frei. Eine seitliche Taste, die für PTT gehalten wird, verhält sich wie ein Fußschalter.
settings-controls-add-mouse-button = + Maustaste hinzufügen
settings-controls-f1-always-opens-manual-even = F1 öffnet immer dieses Handbuch, auch während der Eingabe, deshalb ist die Taste nicht neu belegbar.
settings-controls-midi-controller = MIDI-Controller
settings-controls-midi-controllers-need-native-app = MIDI-Controller brauchen die native App — der Browser-Client hat keinen MIDI-Zugriff.
settings-controls-any-class-compliant-midi-surface = Jede klassenkonforme MIDI-Oberfläche funktioniert: Das Jog-Rad eines DJ-Controllers wird zu einem feinen VFO-Knopf, seine Pads werden zu PTT- und Bandtasten, seine Regler werden zu Verstärkungsreglern.
settings-controls-enable = Aktivieren
settings-controls-controller = Controller
settings-controls-feedback = Rückmeldung an
settings-controls-rescan-ports = Anschlüsse neu suchen
settings-controls-not-connected = Nicht verbunden.
settings-controls-move-control-see-here = Einen Regler bewegen, um ihn hier zu sehen.
settings-controls-control = Regler
settings-controls-reads = Liest als
settings-controls-led = LED
settings-controls-move = bewegen…
settings-controls-click-then-move-control-you = Klicken, dann den Regler bewegen, den Sie belegen wollen
settings-controls-speed-sensitivity-spin-faster-tune = Geschwindigkeitsempfindlichkeit: schneller drehen, um schneller abzustimmen
settings-controls-send-current-value-back-light = Den aktuellen Wert zurücksenden, um eine LED zu beleuchten oder einen Regler zu bewegen
settings-controls-add-midi-control = + MIDI-Regler hinzufügen
settings-controls-clear-all = Alle löschen
settings-controls-endless-jog-encoders-send-relative = Endlose (Jog-)Encoder senden einen relativen Schritt statt einer Position, in einer von drei Kodierungen, die bei kleinen Bewegungen gleich aussehen. LERNEN rät aus einer Drehung im Uhrzeigersinn; dreht der Knopf dann in die falsche Richtung, „rev“ ankreuzen.
settings-controls-none = — keiner —

## Settings window, General (src/app/settings/general.rs)

settings-general-system-default = Systemstandard
settings-general-remote-access = Fernzugriff
settings-general-signed-in-station = Angemeldete Station
settings-general-sign-out = {" "}ABMELDEN{" "}
settings-general-sign-out-forgets-device = Vergisst die Anmeldung dieses Geräts. Die Station fragt bei der nächsten Verbindung erneut; diese Verbindung bleibt bestehen, bis Sie sie schließen.
settings-general-password-kept-no-cookie = In diesem Browser wird ein Passwort für { $stations } gespeichert, weil kein Anmelde-Cookie ausgegeben wurde. Ältere Stationen machen das so; es ist nicht dasselbe.
settings-general-what-remote-client-browser-page = Was ein entfernter Client — die Browserseite oder ein anderer mit --connect gestarteter sdroxide — vorweisen muss, bevor diese Station ihn bedienen lässt. Gilt im Servermodus (--server); die nächste Anmeldung übernimmt eine Änderung, ohne Neustart.
settings-general-username = Benutzername
settings-general-password = Passwort
settings-general-clients-must-give-password-leaving = Clients müssen das Passwort angeben. Ein leerer Benutzername ist in Ordnung.
settings-general-clients-must-sign = Clients müssen sich anmelden.
settings-general-empty-anyone-who-can-reach = ⚠ Leer: Jeder, der den Port des Servers erreichen kann, kann dieses Funkgerät auf Ihrem Rufzeichen bedienen. Setzen Sie ein Passwort, bevor Sie den Port weiterleiten.
settings-general-stored-clear-config-toml-like = Unverschlüsselt in config.toml gespeichert, wie die anderen Passwörter, die sdroxide verwahrt.
settings-general-receive-audio-gain = Verstärkung des Empfangsaudios
settings-general-t-0-db = 0 dB
settings-general-extra-gain-everything-radio-sends =
    Zusätzliche Verstärkung für alles, was dieses Funkgerät an die Lautsprecher schickt, zusätzlich zum Lautstärkeregler. Auf 0 dB lassen, sofern das Funkgerät bei voller Lautstärke nicht leise ist: Das obere Ende des Lautstärkereglers ist das Audio, wie es ankommt, also kann er ein Funkgerät leiser, aber nie lauter machen, und manche USB-Soundausgänge von Transceivern liegen deutlich unter Vollaussteuerung.

    In Schritten von 6 dB erhöhen. Zu viel übersteuert — das Audio wird bei Vollaussteuerung begrenzt statt umzuspringen, daher klingt Übertreiben eher scharf als laut. Aufnahmen werden vor dieser Verstärkung gemacht und sind nicht betroffen.
settings-general-reload-band-plan = BANDPLAN NEU LADEN
settings-general-re-read-bandplan-json-machine = bandplan.json auf dem Rechner, an dem das Funkgerät angeschlossen ist, erneut einlesen und anwenden — Bandgrenzen, Teilabschnitte und Skimmer-Fenster — ohne Neustart.
settings-general-running-station-s-own-band = Läuft mit dem eigenen Bandplan der Station.
settings-general-running-built-iaru-tables = Läuft mit den eingebauten IARU-Tabellen.
settings-general-settings-file = Einstellungsdatei
settings-general-settings-are-machine-radio-attached = Die Einstellungen liegen auf dem Rechner, an dem das Funkgerät angeschlossen ist. Dort exportieren.
settings-general-export = EXPORTIEREN…
settings-general-write-every-setting-station-radios = Jede Einstellung dieser Station — die Funkgeräte, die Betriebsarten, die Server, die Speicher, der Bandplan — in eine Datei schreiben, die Sie auf eine andere Installation mitnehmen können. Ihr Logbuch und jedes gespeicherte Server-Passwort bleiben hier.
settings-general-import = IMPORTIEREN…
settings-general-replace-station-s-settings-ones = Die Einstellungen dieser Station durch die in einer Datei ersetzen, die aus einer anderen Installation exportiert wurde. Danach sdroxide neu starten.
settings-general-import-overwrites-what-here-file = Ein Import überschreibt, was hier steht, Datei für Datei, und wirkt beim nächsten Start von sdroxide — die bereits im Speicher liegenden Einstellungen würden sonst direkt darüber zurückgeschrieben. Was die Datei nicht erwähnt, bleibt unverändert, daher entfernt ein Bündel von einer Ein-Funkgerät-Station hier kein zweites Funkgerät. Ihr Logbuch ist nie in der Datei: Wenn Sie es ebenfalls umziehen wollen, exportieren Sie es als ADIF aus dem Fenster LOG.
settings-general-sdroxide-settings = sdroxide-Einstellungen
settings-general-swr-guard = SWR-Wächter
settings-general-stop-transmitting-high-swr = Bei hohem SWR das Senden beenden
settings-general-trip = Auslösen bei
settings-general-stops-transmission-when-radio-reports =
    Beendet die Aussendung, sobald das Funkgerät ein SWR bei diesem Wert oder darüber meldet, und sperrt das Senden, bis Sie es bestätigen. Fängt eine getrennte Antenne, eine defekte Speiseleitung oder einen auf den falschen Anschluss gestellten Schalter ab.

    Beim Abstimmen wird anders verfahren, denn eine Fehlanpassung zu speisen ist gerade der Zweck: Ein Antennentuner bekommt den doppelten Grenzwert und rund fünf Sekunden, bevor der Wächter überhaupt greift. Ein manueller Tuner, der länger braucht, verlangt, den Wächter für die Sitzung abzuschalten.

    Braucht ein Funkgerät, das SWR über CAT meldet. Ignoriert die ersten zwei Zehntelsekunden jeder Aussendung und wartet nicht auf hohe Leistung.
settings-general-per-mode-settings = Einstellungen je Betriebsart
settings-general-reset-every-mode = JEDE BETRIEBSART ZURÜCKSETZEN
settings-general-forget-every-mode-s-remembered = Vergessen Sie das gemerkte AGC, die Rauschsperre, die Rauschunterdrückung, den Kerbfilter und die Stereoschalter jeder Betriebsart und stellen Sie die eigenen Standardwerte jeder Betriebsart wieder her.
settings-general-selecting-mode-lays-mode-s = Die Wahl einer Betriebsart legt deren eigene Startwerte für AGC, Rauschsperre, Rauschunterdrückung, den Kerbfilter und die Stereoschalter am Empfänger fest; eine Änderung merkt sich das nur für diese Betriebsart. Der kreisförmige Pfeil-Chip am Ende der Zeile Filter/Rauschen des Empfängers listet auf, was geändert wurde, und stellt die aktuelle Betriebsart wieder her.
settings-general-your-audio-speakers-microphone = Ihr Audio (Lautsprecher / Mikrofon)
settings-general-output = Ausgang
settings-general-channels = Kanäle
settings-general-which-speaker-channel-s-received = Welche Lautsprecherkanäle das Empfangsaudio erreicht. Links oder rechts legt alles, zu Mono gemischt, nur auf diese Seite.
settings-general-input = Eingang

## Settings window, network services (src/app/settings/net.rs)

settings-net-operator = Funkamateur
settings-net-callsign-grid-are-set-general = Rufzeichen und Locator werden im Reiter Allgemein eingestellt.
settings-net-set-your-callsign-grid-general = ⚠ Stellen Sie Ihr Rufzeichen und Ihren Locator im Reiter Allgemein ein.
settings-net-freedv-reporter = FreeDV Reporter
settings-net-enable = Aktivieren
settings-net-connects-whenever-enabled-your-station = Verbindet sich, sobald aktiviert. Ihre Station wird anderen nur angezeigt, solange das Funkgerät im RADE-Modus ist — in jeder anderen Betriebsart bleiben Sie verbunden, aber verborgen.
settings-net-station = Station
settings-net-message = Nachricht
settings-net-receive-only-i-cannot-transmit = Nur Empfang (ich kann nicht senden)
settings-net-server = Server
settings-net-host = Host
settings-net-port = Port
settings-net-tls-wss = TLS (wss://)
settings-net-not-yet-implemented-freedv-gui = Noch nicht umgesetzt — die FreeDV-GUI nutzt ebenfalls einfaches ws://.
settings-net-reporting = Meldung
settings-net-report-stations-i-decode = Stationen melden, die ich dekodiere
settings-net-sends-rx-report-each-callsign = Sendet einen rx_report für jedes Rufzeichen, das aus einem RADE-End-of-Over-Rahmen gewonnen wurde.
settings-net-show-other-reporter-stations-spots = Andere Melderstationen als Spots anzeigen
settings-net-adds-them-panadapter-overlay-world = Fügt sie der Überlagerung im Panorama, der Weltkarte und dem Fenster SPOTS unter dem Filter FREEDV hinzu.
settings-net-set-your-callsign-grid-general-2 = ⚠ Stellen Sie Ihr Rufzeichen und Ihren Locator im Reiter Allgemein ein. Ohne beides ist die Verbindung nur zum Ansehen: Sie sehen andere Stationen, erscheinen selbst aber nicht.
settings-net-apply = {" "}ÜBERNEHMEN{" "}
settings-net-persist-re-connect = Speichern und (erneut) verbinden
settings-net-longwave-shortwave-stations-labelled-waterfall = Die Lang- und Kurzwellenstationen, die auf dem Wasserfall beschriftet werden. Der Kurzwellenplan wird für die laufende Rundfunksaison von eibispace.de heruntergeladen und neu geladen, wenn die Saison wechselt.
settings-net-season = Saison
settings-net-downloading = lädt herunter…
settings-net-using-built-copy = die eingebaute Kopie wird verwendet
settings-net-your-own-stations-corrections-go = Ihre eigenen Stationen und Korrekturen kommen in diese Datei. sdroxide schreibt sie nie, und ein Eintrag hier ersetzt einen geplanten mit demselben Namen und derselben Frequenz.
settings-net-reload = Neu laden
settings-net-re-read-your-station-file = Ihre Stationsdatei nach dem Bearbeiten erneut einlesen
settings-net-download-schedule-now = Plan jetzt herunterladen
settings-net-fetch-season-s-schedule-again = Den Plan dieser Saison erneut abrufen und die zwischengespeicherte Kopie ersetzen
settings-net-broadcast-stations-labelled-waterfall-come = Die Rundfunksender, die auf dem Wasserfall beschriftet werden, stammen aus der Liste, die in diesen Build eingebaut ist. Zum Bearbeiten braucht man die Desktop-App.
settings-net-ask-service-whether-these-credentials = Beim Dienst nachfragen, ob diese Zugangsdaten funktionieren. Wendet vorher die Einstellungen oben an und protokolliert nichts: Es liest nur.
settings-net-checking = prüft…

## Settings window: messages with values filled in.

settings-amp-connected-to = Verbunden mit dem { $model }
settings-controls-last-midi = Letzte Meldung: { $message }  Wert { $value }
settings-general-band-plan-file = Jede Bandgrenze und jeder Teilabschnitt stammt aus { $path }, beim ersten Mal aus den eingebauten IARU-Tabellen geschrieben und danach von Ihnen zu bearbeiten — engen Sie ein Band auf Ihre Lizenz ein, und sdroxide verweigert das Senden außerhalb davon. Frequenzen sind in MHz; löschen Sie die Datei für eine frische Kopie der Standardwerte. Dies ist die regionale Zuteilung, nicht Ihre Lizenz: Ihre eigenen Bedingungen können enger sein, und nationale Pläne unterscheiden sich innerhalb einer Region.
settings-general-exported = { $note } exportiert — wählen Sie, wo Sie es speichern wollen.
settings-general-export-failed = Export fehlgeschlagen: { $error }
settings-general-import-failed = Import fehlgeschlagen: { $error }
settings-general-imported = { $summary } — sdroxide neu starten, um sie zu nutzen.
settings-general-import-skipped = { $path } übersprungen: { $why }
settings-general-swr-tripped = ⚠ Ausgelöst bei { $swr }:1 — das Senden ist gesperrt.
settings-net-call-grid-set-on-general = { $call } / { $grid } — im Reiter Allgemein eingestellt
settings-net-reporting-as = Meldet als { $call } / { $grid } — SDRoxide { $version }
settings-net-schedule-download-failed = ⚠ Download fehlgeschlagen: { $error }. Der eingebaute Plan wird verwendet.
settings-net-test-service = { $service } testen

## Settings window (src/app/settings/mod.rs), continued
settings-transmit-off-while-converter-set = Senden ist aus, solange ein Konverter eingestellt ist — oben angeben, was in der Sendeleitung liegt, um es wieder einzuschalten.
settings-transmit-goes-through-same-converter = Senden läuft über denselben Konverter, daher arbeitet das Funkgerät in beide Richtungen mit demselben Versatz von der Skala.
settings-transmit-not-converted-radio-transmits = Senden wird nicht umgesetzt: Das Funkgerät sendet auf der Frequenz, die die Skala zeigt, während der Empfang über den Konverter läuft.
settings-transmit-takes-own-offset-separate = Senden hat seinen eigenen Versatz, getrennt vom Empfangsversatz.
settings-no-enabled-subscriptions-update = Keine aktivierten Abonnements zum Aktualisieren.
settings-add-radio-here-station-you = Ein Funkgerät hinzufügen — hier oder auf einer Station, mit der Sie verbunden sind

## Settings window, General, continued

settings-general-every-band-edge-sub-segment = Jede Bandgrenze und jeder Teilabschnitt stammt aus bandplan.json auf dem Rechner, an dem das Funkgerät angeschlossen ist. Dies ist die regionale Zuteilung, nicht Ihre Lizenz: Ihre eigenen Bedingungen können enger sein, und nationale Pläne unterscheiden sich innerhalb einer Region.

## Settings window, picture upload (src/app/settings/picup.rs)

settings-picup-received-pictures-ftp = Empfangene Bilder → FTP
settings-picup-uploads-every-received-sstv-picture = Lädt jedes empfangene SSTV-Bild (und, falls angekreuzt, jede Wetterkarte) beim Speichern hoch, im Aufbau, wie QSSTV es macht, damit für QSSTV geschriebene Webseiten unverändert funktionieren.
settings-picup-upload-received-pictures = Empfangene Bilder hochladen
settings-picup-server = Server
settings-picup-port = Port
settings-picup-security = Sicherheit
settings-picup-user = Benutzer
settings-picup-password = Passwort
settings-picup-certificate = Zertifikat
settings-picup-trusted-when-computer-s-operating = Vertraut, wenn das Betriebssystem dieses Computers ihm vertraut, wie es ein Browser hier täte. Unten kann eine private CA hinzugefügt werden (PEM).
settings-picup-extra-ca = Zusätzliche CA
settings-picup-only-certificate-fingerprint-accepted-way = Nur das Zertifikat mit diesem Fingerabdruck wird akzeptiert — so vertraut man einem selbstsignierten Server. TEST zeigt den des Servers.
settings-picup-any-certificate-accepted-link-encrypted = ⚠ Jedes Zertifikat wird akzeptiert. Die Verbindung ist verschlüsselt, doch jeder auf dem Weg kann sich als Server ausgeben und das Passwort mitlesen.
settings-picup-plain-ftp-sends-password-pictures = ⚠ Einfaches FTP sendet das Passwort und die Bilder unverschlüsselt.
settings-picup-sstv = SSTV
settings-picup-sstv-dir = SSTV-Verzeichnis
settings-picup-weather-fax = Wetterfax
settings-picup-wefax-dir = WEFAX-Verzeichnis
settings-picup-empty-login-directory = Leer ist das Anmeldeverzeichnis.
settings-picup-file-names = Dateinamen
settings-picup-format = Format
settings-picup-add-extension = Erweiterung anhängen
settings-picup-image1-png-rather-than-qsstv = image1.png statt QSSTVs blankem image1
settings-picup-caption-bar = Beschriftungsleiste
settings-picup-time-frequency-mode-sender-s = Zeit, Frequenz, Betriebsart und die FSK-ID des Absenders oben, wie QSSTV sie zeichnet. Nur die hochgeladene Kopie wird markiert.
settings-picup-fit-960-768 = Auf 960 × 768 anpassen
settings-picup-qsstv-s-heartbeat-tiny-picture = Der Herzschlag von QSSTV: ein winziges Bild, dessen Höhe die Minute ist, wird neben jedem hochgeladen und beim Schließen des Programms zwei Pixel breit gemacht.
settings-picup-nat-workaround = NAT-Umgehung
settings-picup-ignore-private-address-server-s = Eine private Adresse in der Antwort des Servers im passiven Modus ignorieren und seine eigene verwenden — für einen Server hinter NAT, der seine öffentliche Adresse nicht kennt.
settings-picup-apply = {" "}ÜBERNEHMEN{" "}
settings-picup-test = TEST
settings-picup-connect-settings-above-applied-not = Mit den Einstellungen oben verbinden — übernommen oder nicht —, anmelden und in jedem Upload-Verzeichnis eine kleine Datei schreiben und wieder löschen.
settings-picup-checking = prüft…
settings-picup-trust-certificate = Dieses Zertifikat vertrauen
settings-picup-pin-switch-pinned-fingerprint-one = Festpinnen: auf „Festgepinnter Fingerabdruck“ mit diesem umschalten. Vorher am Server prüfen — das ist, was der Server angibt, kein Beweis, wer er ist. Dann ÜBERNEHMEN.
settings-picup-last-upload = Letzter Upload

## Settings window, Profiles (src/app/settings/profiles.rs)

settings-profiles-profiles-station-s-saved-setups = Profile: die gespeicherten Einstellungen der Station
settings-profiles-profile-captures-whole-working-setup = Ein Profil erfasst den ganzen Arbeitsstand — Skalen und VFOs, Betriebsart und Filter, die Verstärkungen, Aussteuerung und Antennen, Ihr Rufzeichen und Ihre Nachrichtenvorlagen sowie die Register je Band — und stellt ihn mit einem Klick wieder her. Die Hardware (Backend, Audiogeräte, Konverter) bleibt bewusst unberührt: Ein Profil ist die Art, wie Sie die Station bedienen, nicht das Funkgerät, auf dem sie läuft.
settings-profiles-save-current-setup = Den aktuellen Stand speichern
settings-profiles-profile-name = Profilname
settings-profiles-save = Speichern
settings-profiles-applies-whatever-sdroxide-attached-dials = Wirkt auf alles, womit sdroxide verbunden ist: Skalen und VFOs, Betriebsart und Filter, Pegel, Verstärkungen, Aussteuerung, Antennen, die digitale Identität und die Bandstapel.
settings-profiles-no-profiles-saved-yet-name = Noch keine Profile gespeichert — oben einen Namen vergeben und Speichern drücken.
settings-profiles-saved-profiles = Gespeicherte Profile
settings-profiles-apply = Übernehmen

## Settings window, T/R switch (src/app/settings/relay.rs)

settings-relay-transmit-receive-switch = Sende-/Empfangsumschalter
settings-relay-waiting-station-s-t-r = Warte auf die Konfiguration des Sende-/Empfangsumschalters der Station…
settings-relay-closes-contact-while-station-transmits = Schließt einen Kontakt, während diese Station sendet — um den Antenneneingang des SDR abzutrennen und zu erden und um einen Verstärker oder ein externes Sende-/Empfangsrelais in der Folge damit zu tasten. Funktioniert mit den günstigen USB-Relaiskarten (LCUS, KMtronic, Numato), mit einer seriellen RTS-/DTR-Leitung an jeder Schnittstelle, die einen PTT-Kontakt will, und mit allem anderen über einen Befehls-Hook.
settings-relay-hardware = Hardware
settings-relay-board = Karte
settings-relay-lcus-four-byte-a0-command = LCUS ist die Karte mit vierbytigen A0-Befehlen, die unter einem Dutzend Namen verkauft wird; KMtronic und Numato lassen sich ebenfalls abfragen, wie sie eingestellt sind
settings-relay-port = Port
settings-relay-choose = — wählen —
settings-relay-device = Gerät
settings-relay-gpio-pin = GPIO-Pin
settings-relay-pin-3-every-homebrew-plan = Pin 3 auf jedem Selbstbau-Plan und jeder kommerziellen Karte — er ist der am Ende des Gehäuses, an dem sich ein Draht anbringen lässt. Klickt das Relais nicht, ist die Karte ein Klon, der diese Pins gar nicht umsetzt.
settings-relay-gpio-chip = GPIO-Chip
settings-relay-linux-only-gpiodetect-lists-chips = Nur Linux. `gpiodetect` listet die Chips auf dem Rechner auf.
settings-relay-transmit = Beim Senden
settings-relay-receive = Beim Empfangen
settings-relay-nothing-switched = Nichts wird geschaltet.
settings-relay-starting-program-costs-unpredictable-amount = Das Starten eines Programms kostet eine nicht vorhersehbare Zeit, und davon wird nichts in den Vorlauf eingerechnet — deshalb hier einen größeren Abstand lassen als für eine Relaiskarte. Die Zahlen der Kontakttabelle bedeuten für diese Verbindung nichts: Ihre Befehle werden einmal beim Niederdrücken und einmal beim Loslassen ausgeführt.
settings-relay-contacts = Kontakte
settings-relay-contact-1-rts-contact-2 = Kontakt 1 ist RTS und Kontakt 2 ist DTR.
settings-relay-no = Nr.
settings-relay-gpio = GPIO
settings-relay-name = Name
settings-relay-job = Aufgabe
settings-relay-tx-closes = Senden schließt
settings-relay-lead = Vorlauf
settings-relay-hold = Haltezeit
settings-relay-line-s-offset-chip-bcm = Der Versatz der Leitung an diesem Chip — die BCM-Nummer an einer Raspberry-Pi-Leiste, die nicht die physische Pin-Nummer ist
settings-relay-sdr-antenna = SDR-Antenne
settings-relay-decides-default-timings-what-log = Legt die Standardzeiten fest und wie das Protokoll es nennt — und, bei einem Bandentschlüssler, dass der Kontakt der Bandtabelle unten folgt
settings-relay-transmitting-energises-coil-off-transmitting =
    Ein: Beim Senden wird die Spule erregt. Aus: Beim Senden wird sie freigegeben.

    Das ist eher eine Verdrahtungsentscheidung als eine Einstellung — siehe die Anmerkung unten.
settings-relay-how-long-before-rf-contact = Wie lange vor HF dieser Kontakt von seinem Empfangswort des Bandes auf sein Sendewort wechselt. Eine Neuabstimmung bewegt ihn sofort; dies gilt fürs Niederdrücken, wenn ein Filter nur für Senden eingeschaltet sein muss, bevor die Leistung anliegt.
settings-relay-closed-long-before-rf-small = Geschlossen so lange vor HF. Ein kleines Koax-Relais schaltet in 5 bis 15 ms. Senden wartet auf den längsten dieser Werte, daher ist ein großer Wert eine hörbare Lücke im Empfangsaudio; alles über 250 ms wird ignoriert.
settings-relay-how-long-after-rf-stops = Wie lange nach dem Ende von HF dieser Kontakt von seinem Sendewort zurück auf sein Empfangswort geht.
settings-relay-opened-long-after-rf-stops = Geöffnet so lange nach dem Ende von HF. Absichtlich länger als der Vorlauf: Die Antenne einen Moment zu spät zurückzuschalten kostet nichts, sie zu früh zurückzuschalten kostet eine Frontstufe. Null ist eine gültige Antwort und die richtige für die Tastleitung eines Verstärkers.
settings-relay-test = {" "}TEST{" "}
settings-relay-not-while-station-air = Nicht, solange die Station auf Sendung ist
settings-relay-close-contact-half-second-so = Diesen Kontakt eine halbe Sekunde schließen, damit man das Relais hört und die Verdrahtung bei kaltem Sender prüfen kann
settings-relay-remove = Entfernen
settings-relay-add-contact = + Kontakt hinzufügen
settings-relay-if-will-not-answer = Wenn sie nicht antwortet
settings-relay-switch-exists-protect-receiver-not = Ein Schalter, der einen Empfänger schützen soll, ist nicht viel wert, wenn er ihn stillschweigend nicht mehr schützt — deshalb ist Verweigern der Standard. Wählen Sie die andere Option dort, wo ein gelöstes USB-Kabel Ihren Contest beenden würde, das schlimmere Ergebnis wäre.
settings-relay-order-timings-longest-lead-closes = Die Reihenfolge ergibt sich aus den Zeiten: Der längste Vorlauf schließt zuerst, und die längste Haltezeit öffnet zuletzt. Geben Sie dem Antennenrelais also den längeren Vorlauf und die längere Haltezeit und dem Verstärker jeweils den kürzeren — dann wird die Antenne immer umgeschaltet, bevor der Verstärker getastet wird, und der Verstärker wird immer entsperrt, bevor die Antenne zurückkommt.
settings-relay-transmit-sense-input = Eingang für die Sendeerkennung
settings-relay-when-transceiver-keys-itself-own = Wenn ein Transceiver sich selbst tastet — mit seiner eigenen Mikrofontaste, einem Fußschalter, VOX oder Keyer — erfährt sdroxide das, indem es über CAT fragt, und die Antwort kommt einige hundert Millisekunden nach Beginn der Aussendung. Führt man die SEND-Leitung des Geräts über einen Optokoppler auf einen Handshake-Eingang an demselben Port, wird sie stattdessen in wenigen Millisekunden erkannt. Das Messgerät und die Sendeverriegelung werden dadurch schneller.
settings-relay-line = Leitung
settings-relay-transmitting = Senden ist
settings-relay-high-line = eine High-Leitung
settings-relay-most-opto-isolated-interfaces-pull = Die meisten optoisolierten Schnittstellen ziehen die Leitung herunter, wenn das Gerät tastet; deshalb ist dies meist aus
settings-relay-belongs-radio = Gehört zu Funkgerät
settings-relay-which-radio-tab-sensed-transceiver = Zu welchem Funkgerät-Reiter der erkannte Transceiver gehört. Bei einer Station mit einem Funkgerät ist das 1, und nichts hängt davon ab.
settings-relay-needs-serial-port-handshake-lines = Braucht einen seriellen Port — die Handshake-Leitungen liegen darauf.
settings-relay-wiring-only-fail-safe-survives = Die Verdrahtung ist die einzige Ausfallsicherung, die dieses Programm übersteht.
settings-relay-choose-tx-closes-so-de = „Senden schließt“ wählen, damit der *stromlose* Kontakt der gewünschte Zustand ist, wenn die Karte abgezogen ist, der Computer aus ist oder sdroxide nicht läuft. Verdrahtet so, dass stromlos Masse bedeutet, lässt ein totes Relais den SDR taub — lästig, aber sicher. Andersherum verdrahtet, bedeutet ein totes Relais plus eine Aussendung eine tote Frontstufe. Mit einem echten Verstärker dahinter die erste Option nehmen.
settings-relay-cannot-protect-first-instant-over = Das kann den ersten Augenblick einer Aussendung, die Sie am Funkgerät beginnen, nicht schützen. Tastet man das Gerät von sdroxide aus, eilen die Kontakte der HF immer voraus; verdrahtet man den Sense-Eingang oben, folgen sie innerhalb von Millisekunden. Für eine Frontstufe, die man wirklich schützen will, zusätzlich einen HF-gesteuerten Hardware-Sende-/Empfangsschalter verwenden — kein Programm auf einem Computer kann die ganze Antwort sein.
settings-relay-contacts-transmit = Kontakte beim Senden
settings-relay-not-open = Nicht geöffnet.
settings-relay-status-unknown-press-apply = Status unbekannt — ÜBERNEHMEN drücken.
settings-relay-station-air-apply-will-take = Die Station ist auf Sendung — ÜBERNEHMEN wirkt nach dieser Aussendung.
settings-relay-band-decoder-outputs = Ausgänge des Bandentschlüsslers
settings-relay-one-control-word-per-band = Ein Steuerwort je Band: Bit 0 ist Kontakt 1, Bit 1 Kontakt 2 usw., entsprechend der oben genannten Nummerierung. RX folgt dem Band der Empfangsskala; TX dem Band des Funkgeräts, das getastet hat, und wird auf dem Vorlauf jedes Kontakts vor der HF eingeschaltet und nach seiner Haltezeit wieder ausgeschaltet — gleicher Wert für ein Filter, dem egal ist, in welche Richtung die HF geht, und verschiedene Werte für eine Empfangsvorverstärker-Überbrückung oder eine Sendetiefpassbank. Nicht belegte Bänder auf 0 lassen, dann wird nichts angesteuert. Wirkt nach Übernehmen / erneut verbinden.
settings-relay-band = Band
settings-relay-rx = RX
settings-relay-tx = TX
settings-relay-outputs-asserted = Ausgänge gesetzt
settings-relay-other = Andere
settings-relay-everywhere-outside-amateur-bands-short = Überall außerhalb der Amateurfunkbänder — Kurzwellenempfang und alles, worauf die Skala eines Transverters landet und das kein Band abdeckt.
settings-relay-apply = {" "}ÜBERNEHMEN{" "}
settings-relay-persist-re-open-switch = Speichern und den Schalter (erneut) öffnen

## Settings window, Remote (src/app/settings/remote.rs)

settings-remote-connect-server = Mit einem Server verbinden
settings-remote-drive-sdroxide-running-somewhere-else = Ein sdroxide steuern, das anderswo läuft — der Shack-Rechner, ein entfernter Standort — über das Netzwerk. Dort mit --server starten und seine Adresse hier eingeben. Funkgerät, Decoder und Logbuch bleiben auf diesem Rechner; dieser Bildschirm bekommt den Wasserfall und das Audio, und benutzt werden seine Lautsprecher und sein Mikrofon.
settings-remote-address = Adresse
settings-remote-host-name-ip = Hostname oder IP
settings-remote-machine-running-sdroxide-server-host = Der Rechner, auf dem sdroxide --server läuft: ein Hostname, eine IPv4- oder IPv6-Adresse oder eine vollständige ws://- (bzw. wss://-) URL, wenn der Server hinter einem Reverse-Proxy steht — eine URL wird genau so verwendet, wie eingegeben, und ignoriert Port und Schalter unten.
settings-remote-port = Port
settings-remote-port-server-listens-server-port = Der Port, auf den dieser Server lauscht — server_port in seiner config.toml. 4950, sofern er nicht mit --port angegeben wurde.
settings-remote-secure = Sicher
settings-remote-wss-tls = wss:// (TLS)
settings-remote-tick-where-server-sits-behind = Hier ankreuzen, wenn der Server hinter etwas steht, das HTTPS für ihn abschließt — ein Reverse-Proxy auf Port 443, was der übliche Weg ist, eine Station über das offene Internet zu erreichen. Der eigene Server von sdroxide spricht einfaches ws:// und will das nicht.
settings-remote-connect = {" "}VERBINDEN{" "}
settings-remote-open-station-radio-tab-own = Diese Station als eigenen Funkgerät-Reiter öffnen
settings-remote-client-cannot-hold-second-connection = Dieser Client kann keine zweite Verbindung halten.
settings-remote-station-appears-tab-beside-radios =
    Die Station erscheint als Reiter neben den hier bereits geöffneten Funkgeräten, und die Reiterleiste oben im Fenster schaltet zwischen ihnen um (⊞ stellt zwei nebeneinander). Schließen Sie sie über die Liste oben im Reiter Funkgerät — das legt auf; auf dem Server wird nichts verändert.

    Fragt der Server nach Benutzername und Passwort, erscheint seine Anmeldeseite im neuen Reiter. Eine einfache ws://-Verbindung trägt das alles unverschlüsselt; über das offene Internet sollte man sie deshalb entweder durch ein VPN oder einen SSH-Tunnel leiten oder HTTPS vor den Server schalten und oben Sicher ankreuzen. Ein Proxy mit selbstsigniertem Zertifikat ist kein Problem: Der Reiter des Funkgeräts zeigt dessen Fingerabdruck und fragt, bevor er ihm vertraut.

## Settings window, Servers (src/app/settings/servers.rs)

settings-servers-wsjt-x-udp-broadcast = WSJT-X-UDP-Broadcast
settings-servers-waiting-station-s-wsjt-x = Warte auf die WSJT-X-Broadcast-Konfiguration der Station…
settings-servers-sends-decodes-station-status-logged = Sendet Dekodierungen, den Stationsstatus und protokollierte QSOs so, wie WSJT-X es tut, damit GridTracker, JTAlert, N1MM+ und Log4OM mit sdroxide unverändert funktionieren. Nur Ausgabe — auf diesem Socket kann nichts das Funkgerät beeinflussen.
settings-servers-enable = Aktivieren
settings-servers-send = Senden an
settings-servers-t-127-0-0-1-reaches = 127.0.0.1 erreicht Clients auf diesem Rechner; eine LAN-Adresse oder eine Multicast-Gruppe (224.0.0.1) erreicht andere
settings-servers-port = Port
settings-servers-t-2237-port-every-client-defaults = 2237 ist der Port, auf den jeder Client voreingestellt ist
settings-servers-identify = Sich ausweisen als
settings-servers-name-clients-see-some-loggers = Der Name, den die Clients sehen. Manche Logger akzeptieren nur Datenverkehr, der sich als WSJT-X ausweist.
settings-servers-n1mm-contactinfo-broadcast = N1MM+-Contactinfo-Broadcast
settings-servers-same-news-n1mm-s-words = Dieselbe Meldung in N1MMs Worten: ein XML-Datagramm pro protokolliertem Kontakt, für die Logger, die auf N1MM statt auf WSJT-X hören — die Brücke der World Radio League nimmt beides. Nur Kontakte; in diesem Protokoll gibt es keinen Dekodierstrom. Eigener Schalter und eigener Port, weil eine Station beides wollen kann.
settings-servers-t-127-0-0-1-reaches-2 = 127.0.0.1 erreicht Clients auf diesem Rechner. N1MMs eigene Empfehlung für ein Contest-Netz ist die Broadcast-Adresse dieses Subnetzes — 192.168.1.255 für ein 192.168.1.n-Netz —, die jede Position auf einmal erreicht.
settings-servers-t-12060-port-n1mm-s-documentation = 12060 ist der Port, den die Dokumentation von N1MM empfiehlt
settings-servers-station-name = Stationsname
settings-servers-what-n1mm-calls-stationname-name = Was N1MM StationName nennt: der Name des Computers, der das Paket gesendet hat. Logger zeigen ihn an, um einen Bedienplatz von einem anderen zu unterscheiden.
settings-servers-decoded-cw-over-udp = Dekodiertes CW über UDP
settings-servers-cw-panel-s-copy-plain = Die Mitschrift des CW-Panels als Klartext, ein UDP-Datagramm pro Stück, sobald es feststeht — für einen Stationscontroller, das CW-Fenster eines Loggers oder ein Skript. Kein Protokoll: Ein Empfänger hängt einfach an, was ankommt (probieren Sie nc -ul 9999). Nur festgelegter Text wird gesendet, nie der Schwanz, den der Decoder noch überarbeitet. Jedes Funkgerät hat seinen eigenen Schalter.
settings-servers-t-127-0-0-1-reaches-3 = 127.0.0.1 erreicht Programme auf diesem Rechner; eine LAN-Adresse oder die Broadcast-Adresse dieses Subnetzes erreicht andere.
settings-servers-t-9999-default = Standardmäßig 9999
settings-servers-apply = {" "}ÜBERNEHMEN{" "}
settings-servers-persist-re-open-broadcast-sockets = Speichern und die Broadcast-Sockets (erneut) öffnen
settings-servers-hamlib-rigctld-server = Hamlib-rigctld-Server
settings-servers-waiting-station-s-rigctld-configuration = Warte auf die rigctld-Konfiguration der Station…
settings-servers-listen = Lauschen auf
settings-servers-t-127-0-0-1-machine = 127.0.0.1 (dieser Rechner)
settings-servers-t-0-0-0-0-whole = 0.0.0.0 (ganzes Netzwerk)
settings-servers-t-4532-port-every-rigctld-client = 4532 ist der Port, den jeder rigctld-Client voraussetzt
settings-servers-rig-name = Gerätename
settings-servers-reported-clients = an Clients gemeldet
settings-servers-max-clients = Max. Clients
settings-servers-allow-clients-transmit = Clients das Senden erlauben
settings-servers-off-refuses-every-key-request = Aus verweigert jede Tastanforderung und hört auf, einen Sendebereich anzukündigen, daher tastet Hamlib selbst nicht.
settings-servers-rigctld-has-no-authentication-0 = ⚠ rigctld hat keine Authentifizierung. Auf 0.0.0.0 kann jeder, der diesen Port erreicht, Ihren Sender abstimmen und tasten.
settings-servers-t-1-client = 1 Client
settings-servers-not-running = Läuft nicht.
settings-servers-status-unknown-press-apply = Status unbekannt — ÜBERNEHMEN drücken.
settings-servers-persist-re-bind-server = Speichern und den Server (neu) binden
settings-servers-lets-any-hamlib-capable-program = Erlaubt jedem Hamlib-fähigen Programm, dieses Funkgerät zu steuern: Frequenz, Betriebsart, PTT, Split, RIT und Leistung. In WSJT-X, fldigi oder CQRLOG das Gerät „Hamlib NET rigctl“ (Modell 2) wählen und auf diese Adresse zeigen lassen; in GPredict und N1MM Host und Port direkt eingeben. Anders als der TCI-Server überträgt er nur Steuerung — kein Audio, kein IQ.
settings-servers-antenna-rotator = Antennenrotor
settings-servers-waiting-station-s-rotator-configuration = Warte auf die Rotor-Konfiguration der Station…
settings-servers-points-motorized-antenna-satellite-you = Richtet eine motorisierte Antenne auf den Satelliten aus, auf den Sie aufgeschaltet sind, über einen Hamlib-rotctld-Dienst (z. B. „rotctld -m 603 -r /dev/ttyUSB0“ für einen GS-232B-Rotor oder „rotctld -m 1“, um es ohne Hardware auszuprobieren) — ein Dienst deckt jeden Rotor ab, den Hamlib ansteuert — oder über PSTRotator, mit aktiviertem UDP Control in dessen Setup.
settings-servers-program = Programm
settings-servers-host = Host
settings-servers-t-4533-rotctld-s-default-pstrotator = 4533 ist der Standard von rotctld. Der UDP-Steuerport von PSTRotator ist standardmäßig 12000, und es beantwortet Positionsabfragen auf dem nächsthöheren Port, 12001 — der auf diesem Rechner frei sein muss.
settings-servers-min-elevation = Min. Elevation
settings-servers-below-rotator-parks-instead-tracking = Darunter parkt der Rotor, statt zu folgen — auf Ihren örtlichen Horizont oder Ihre Dachlinie einstellen
settings-servers-azimuth-offset = Azimutversatz
settings-servers-added-every-command-rotator-whose = Wird jedem Befehl hinzugefügt, für einen Rotor, dessen Norden versetzt ist
settings-servers-min-movement = Min. Bewegung
settings-servers-steps-smaller-than-are-not = Schritte, die kleiner sind, werden nicht gesendet — das Verfolgen von Zehntelgrad verschleißt Motoren für nichts
settings-servers-park-position = Parkposition
settings-servers-drive-position-when-idle = im Leerlauf in eine Position fahren
settings-servers-off-leaves-antenna-wherever-last = Aus lässt die Antenne dort stehen, wo der letzte Durchgang endete
settings-servers-not-connected = Nicht verbunden.
settings-servers-persist-re-connect-daemon = Speichern und mit dem Dienst (erneut) verbinden
settings-servers-waiting-station-s-tci-server = Warte auf die TCI-Server-Konfiguration der Station…
settings-servers-built-tci-server = Eingebauter TCI-Server
settings-servers-device-name = Gerätename
settings-servers-off-leaves-control-receive-streams = Aus lässt Steuerung und Empfangsströme laufen, aber jede Tastanforderung wird abgelehnt.
settings-servers-tci-has-no-authentication-0 = ⚠ TCI hat keine Authentifizierung. Auf 0.0.0.0 kann jeder, der diesen Port erreicht, Ihren Sender abstimmen und tasten.
settings-servers-tci-client = TCI-Client
settings-servers-when-you-tune-send-radio = Beim Abstimmen den VFO dieses Funkgeräts an einen anderen TCI-Server senden, damit dieses Gerät mitläuft. Nur Steuerung — kein Audio, kein IQ, und es tastet den Sender nicht.
settings-servers-server = Server
settings-servers-same-address-server-above-client = Dies ist dieselbe Adresse wie der Server oben. Der Client verbindet sich nur, wenn dieser Server nicht der lauschende ist — ausgeschaltet lassen oder auf einem anderen Port, wenn 590Ctrl diesen Port benutzt.
settings-servers-tci-client-connected = ● TCI-Client verbunden
settings-servers-tci-client-connecting = TCI-Client verbindet…
settings-servers-tci-client-press-apply = TCI-Client — ÜBERNEHMEN drücken.
settings-servers-persist-re-bind-server-connect = Speichern und den Server (neu) binden und den TCI-Client verbinden
settings-servers-server-lets-tci-capable-programs = Der Server erlaubt TCI-fähigen Programmen, dieses Funkgerät zu steuern: Frequenz- und Betriebsartensteuerung, Breitband-IQ, Empfangsaudio und Sendeaudio. In WSJT-X den Gerätetyp SunSDR (TCI) wählen, auf diese Adresse zeigen lassen und PTT auf TCI setzen. TCI-Client ist die andere Richtung: Die Skala von sdroxide wird als VFO-Befehl an 590Ctrl (oder einen anderen TCI-Server) an der oben genannten Adresse gesendet. Es gibt kein Audio weiter.

## Settings window, TLE (src/app/settings/tle.rs)

settings-tle-satellites-element-sets-frequencies = Satelliten: Bahnelemente und Frequenzen
settings-tle-waiting-station-s-satellite-configuration = Warte auf die Satelliten-Konfiguration der Station…
settings-tle-tracker-already-fetches-celestrak-s = Der Tracker holt die Amateurfunkgruppe von CelesTrak bereits selbst. Das hier ist für alles andere: die NOAA-Wetter-Satelliten, einen Cubesat, der zu neu für die Gruppe ist, oder einen aktuelleren Bahnelementsatz als den, der angekommen ist. Die Listen werden dort abgerufen — und gehalten —, wo die Funkmaschine läuft, daher ist es dieselbe Satellitenmenge, wo auch immer die App geöffnet ist.
settings-tle-subscriptions = Abonnements
settings-tle-listings-fetched-kept-current-same = Listen werden im selben Sechs-Stunden-Takt wie der Amateurfunksatz abgerufen und aktuell gehalten. Sie werden aktualisiert, während das Sonnenfenster offen ist, und durch JETZT AKTUALISIEREN hier.
settings-tle-fetch-track-listing = Diese Liste abrufen und verfolgen
settings-tle-remove-subscription = Dieses Abonnement entfernen
settings-tle-orbits = Umlaufbahnen
settings-tle-which-satellites-listing-get-orbit = Welche Satelliten in dieser Liste einen Bahnring und eine Beschriftung erhalten. Eine ganze Gruppe will „kuratiert“: neunzig Ringe auf einmal sind unlesbar, und keiner lässt neunzig anonyme Punkte zurück.
settings-tle-nothing-listing-sdroxide-s-curated = Nichts in dieser Liste steht auf der kuratierten Liste von sdroxide — diese Liste umfasst zehn Amateurfunksatelliten, daher verhielte sich das genau wie „keine“.
settings-tle-all-satellites = alle Satelliten
settings-tle-catalogue-numbers-keep-comma-separated = Katalognummern, die behalten werden, durch Kommas getrennt. Leer verfolgt alles, was die Liste enthält.
settings-tle-not-fetched-yet = noch nicht abgerufen
settings-tle-subscription = + Abonnement
settings-tle-update-now = {" "}JETZT AKTUALISIEREN{" "}
settings-tle-ask-radio-engine-fetch-every = Die Funkmaschine bitten, jetzt jedes aktivierte Abonnement abzurufen
settings-tle-celestrak-groups = CelesTrak-Gruppen
settings-tle-pasted-element-sets = Eingefügte Bahnelementsätze
settings-tle-one-off-these-do-not = Für einen einmaligen Fall. Diese aktualisieren sich nicht selbst, und SGP4 hört auf, einen Bahnelementsatz zu berechnen, sobald er zwei Wochen über seiner Epoche liegt — abonnieren Sie stattdessen alles, was Sie behalten wollen.
settings-tle-track-one = Diesen verfolgen
settings-tle-show-two-element-lines = Die zwei Bahnelementzeilen anzeigen
settings-tle-remove = Entfernen
settings-tle-paste-two-three-line-element = Zwei- oder dreizeilige Bahnelementsätze hier einfügen
settings-tle-add-pasted = + Eingefügtes hinzufügen
settings-tle-nothing-paste-box-looked-like = Nichts im Einfügefeld sah wie ein Bahnelementsatz aus.
settings-tle-clear-box = Feld leeren
settings-tle-frequencies = Frequenzen
settings-tle-shown-under-pass-table-solar = Wird unter der Überflugtabelle im Sonnenfenster angezeigt. Ein Eintrag hier ersetzt den eingebauten für diese Katalognummer vollständig, daher von einer Kopie davon ausgehen, sofern Sie nicht den Rest verwerfen wollen.
settings-tle-remove-satellite-s-entry = Den Eintrag dieses Satelliten entfernen
settings-tle-link = VERBINDUNG
settings-tle-downlink = ABWÄRTSSTRECKE
settings-tle-uplink = AUFWÄRTSSTRECKE
settings-tle-mode = BETRIEBSART
settings-tle-note = HINWEIS
settings-tle-fm-repeater = FM-Relais
settings-tle-fm = FM
settings-tle-ctcss-67-0-hz = CTCSS 67,0 Hz
settings-tle-link-2 = + Verbindung
settings-tle-copy-built = Eingebaute kopieren
settings-tle-frequency-either-one-number-145 = Eine Frequenz ist entweder eine Zahl (145.800) oder ein Transponder-Durchlassbereich, als 145.950-145.970 geschrieben. Für ein Baken-Signal eine Richtung leer lassen.
settings-tle-norad = NORAD
settings-tle-satellite = + Satellit
settings-tle-frequency-entry-needs-satellite-s = Ein Frequenzeintrag braucht die NORAD-Katalognummer des Satelliten.

## Settings window, Wavelog (src/app/settings/wavelog.rs)

settings-wavelog-address = Adresse
settings-wavelog-api-key = API-Schlüssel
settings-wavelog-your-wavelog-s-front-page = Die Startseiten-Adresse Ihres Wavelog — https://log.example.org oder http://192.168.1.10/wavelog — und ein Schlüssel von dessen API-Seite. Zum Hochladen braucht man einen Schlüssel mit Lese- und Schreibrechten.
settings-wavelog-plain-http-address-internet-api = Einfaches http:// zu einer Adresse im Internet: Der API-Schlüssel wird unverschlüsselt übertragen. Im eigenen Netz in Ordnung; über das Internet https:// verwenden.
settings-wavelog-trusted-cert = Vertrautes Zertifikat
settings-wavelog-forget-certificate = Dieses Zertifikat vergessen
settings-wavelog-go-back-ordinary-checking-against = Zurück zur üblichen Prüfung gegen die öffentlichen Zertifizierungsstellen — für einen Server, der jetzt ein richtiges Zertifikat hat.
settings-wavelog-server-presented-certificate-sha-256 = Der Server hat dieses Zertifikat vorgelegt (SHA-256):
settings-wavelog-compare-your-server-s-e = Vergleichen Sie es mit dem Ihres Servers — z. B. `openssl x509 -noout -fingerprint -sha256 -in cert.pem` — und vertrauen Sie ihm nur bei Übereinstimmung. Von da an wird genau dieses Zertifikat akzeptiert und jedes andere abgelehnt.
settings-wavelog-trust-certificate = Dieses Zertifikat vertrauen
settings-wavelog-station-location = Standort der Station
settings-wavelog-none = — keiner —
settings-wavelog-press-test-above-list-station = Oben Testen drücken, um die in Ihrem Wavelog definierten Stationsstandorte aufzulisten.
settings-wavelog-default = Standard
settings-wavelog-where-qsos-go-when-operating = Wohin QSOs gehen, wenn das Betriebsprofil keinen Standort nennt.
settings-wavelog-profile = Dieses Profil
settings-wavelog-use-default = Den Standard verwenden
settings-wavelog-not-set = nicht gesetzt
settings-wavelog-no-station-location-chosen-uploads = Kein Stationsstandort gewählt — Uploads schlagen fehl, bis einer gewählt ist.
settings-wavelog-qsos-not-wavelog = QSOs nicht in Wavelog
settings-wavelog-every-qso-logbook-wavelog = Jedes QSO im Logbuch ist in Wavelog.
settings-wavelog-everything-logbook-wavelog-has-not = Alles im Logbuch, was Wavelog nicht bestätigt hat — einschließlich Kontakte von vor dem automatischen Upload. Solche, die es schon hat, kommen als Duplikate zurück und werden einfach als gesendet markiert.
settings-wavelog-set-address-key-station-location = Zuerst Adresse, Schlüssel und einen Stationsstandort setzen.
settings-wavelog-yes-send-them = Ja, senden
settings-wavelog-cancel = Abbrechen
settings-wavelog-download-into-logbook = In dieses Logbuch herunterladen
settings-wavelog-downloading-runs-where-radio-attached = Das Herunterladen läuft dort, wo das Funkgerät angeschlossen ist, in das Logbuch dieses Rechners — dort einrichten.
settings-wavelog-download-qsos-start-every = QSOs beim Start und alle herunterladen
settings-wavelog-downloaded-qsos-join-logbook-so = Heruntergeladene QSOs kommen ins Logbuch, damit FT8 und die Spot-Listen wissen, was Sie aus anderen Programmen und von anderen Geräten gearbeitet haben. Jeder Download holt nur, was neu ist.
settings-wavelog-press-test-above-choose-which = Oben Testen drücken, um zu wählen, welche Stationsstandorte herunterzuladen sind.
settings-wavelog-sync-now = Jetzt abgleichen
settings-wavelog-fetch-what-wavelog-has-added = Holt, was Wavelog seit dem letzten Download hinzugefügt hat. Wendet vorher die Einstellungen oben an.
settings-wavelog-full-resync = Vollständiger Abgleich…
settings-wavelog-download-ticked-locations-whole-compare = Die angekreuzten Standorte vollständig herunterladen und mit dem Logbuch vergleichen: In Wavelog bearbeitete oder gelöschte QSOs oder solche von einem nicht mehr angekreuzten Standort werden wieder angeglichen. Sie sehen die Änderungen, bevor etwas geschieht.
settings-wavelog-full-resync-changes-make = Vollständiger Abgleich — vorzunehmende Änderungen
settings-wavelog-qsos-logged-here-imported-file = QSOs, die hier protokolliert oder aus einer Datei importiert wurden, werden nie geändert oder entfernt — nur solche, die aus Wavelog heruntergeladen wurden.
settings-wavelog-apply = Übernehmen
settings-wavelog-discard = Verwerfen
settings-wavelog-live-radio-status = Live-Funkgerätstatus
settings-wavelog-send-frequency-mode-wavelog-they = Frequenz und Betriebsart an Wavelog senden, sobald sie sich ändern
settings-wavelog-radio-name = Funkgerätename
settings-wavelog-pick-radio-wavelog-s-live = Dieses Funkgerät in Wavelogs Live-Logging wählen, dann folgt dessen Formular der Skala. Braucht einen Schlüssel mit Lese- und Schreibrechten. Nur vom Hauptfunkgerät der Station.

## Settings window, picture upload: messages with values

settings-picup-server-fingerprint = SHA-256 des Serverzertifikats: { $fingerprint }

## Settings window, Profiles: messages with values

settings-profiles-apply-hint = Die Station wieder auf „{ $name }“ zurücksetzen: Skalen, VFOs, Betriebsart, Filter, Verstärkungen, Aussteuerung, Antennen, Identität und Bandstapel.
settings-profiles-delete = „{ $name }“ löschen

## Settings window, T/R switch: messages with values

settings-relay-not-usable = Noch nicht nutzbar: { $why }
settings-relay-receiving = empfängt
settings-relay-rx-tx-words = { $rx } / TX { $tx }

## Settings window, Servers: messages with values

settings-servers-n-clients = { $n ->
        [one] { $n } Client
       *[other] { $n } Clients
    }
settings-servers-listening = ● Lauscht auf { $addr } — { $clients }
settings-servers-failed = Fehlgeschlagen: { $error }
settings-servers-rotator-at = ● Verbunden — Antenne bei { $az }° Az, { $el }° El
settings-servers-tci-client-error = TCI-Client: { $error }

## Settings window, TLE: messages with values

settings-tle-subscription-age = { $count } Satelliten · { $age } alt
settings-tle-subscribed = Abonniert: { $name }. JETZT AKTUALISIEREN drücken, um es abzurufen.
settings-tle-norad-too-stale = NORAD { $norad } · { $age } alt — zu alt zum Berechnen
settings-tle-norad-age = NORAD { $norad } · { $age } alt
settings-tle-norad-n = NORAD { $norad }
settings-tle-added = { $n ->
        [one] Ein Bahnelementsatz hinzugefügt.
       *[other] { $n } Bahnelementsätze hinzugefügt.
    }
settings-tle-added-refreshed = { $added } hinzugefügt und { $r ->
        [one] ein Bahnelementsatz aktualisiert.
       *[other] { $r } Bahnelementsätze aktualisiert.
    }
settings-tle-links = { $n } { $n ->
        [one] Verbindung
       *[other] Verbindungen
    }
settings-tle-replace-links = Diese Verbindungen durch die eingebauten für { $name } ersetzen

## Settings window, Wavelog: messages with values

settings-wavelog-call-mismatch = Standort „{ $location }“ ist { $station_call }, Sie protokollieren aber als { $call } — Wavelog wird diese QSOs ablehnen.
settings-wavelog-grid-mismatch = Standort „{ $location }“ liegt bei { $station_grid }, Ihr Locator ist aber { $grid } — Wavelog wird diese QSOs ablehnen.
settings-wavelog-location-n = Standort Nr. { $id }
settings-wavelog-saved-with-callsign = Mit dem Rufzeichen ({ $call }) in Einstellungen → Profile gespeichert, damit ein portables Profil auf einen portablen Standort loggen kann.
settings-wavelog-owed = { $n } { $n ->
        [one] QSO
       *[other] QSOs
    } seit dem Einschalten des automatischen Uploads protokolliert und noch zu senden — alle 5 Minuten erneut versucht.
settings-wavelog-send-unsent = { $n } { $n ->
        [one] QSO
       *[other] QSOs
    } noch nicht in Wavelog senden…
settings-wavelog-send-confirm = { $n } QSOs an { $location } senden? Die Einstellungen werden vorher übernommen.
settings-wavelog-plan-add = • { $n } QSOs aus Wavelog, die ins Logbuch hinzugefügt werden
settings-wavelog-plan-update = • { $n } heruntergeladene QSOs, die in Wavelog geändert wurden und aktualisiert werden
settings-wavelog-plan-remove = • { $n } heruntergeladene QSOs, die nicht mehr in Wavelog sind (oder von einem nicht angekreuzten Standort), zum Entfernen
settings-wavelog-plan-mark = • { $n } eigene QSOs, die als in Wavelog vorhanden markiert werden

## Settings window, Radio tab (src/app/settings/radio.rs)

settings-radio-antenna = Antenne
settings-radio-which-socket-back-radio-using =
    Welche Buchse auf der Rückseite das Funkgerät benutzt — dessen eigener ANT-Befehl, dieselbe Einstellung wie die Taste ANT an der Frontplatte.

    Wirkt sofort, und es ist die Einstellung des Funkgeräts und keine hier gehaltene Kopie: Die Buchse, auf der das Funkgerät liegt, wird beim Öffnen der Sitzung zurückgelesen.

    Die Wahl wird je Band gemerkt. Auf 2 m auf die Antenne und auf 40 auf den Draht umschalten, und jede kommt beim nächsten Wechsel in das Band zurück — und beim nächsten Start von sdroxide. Ein hier gespeicherter Speicherkanal führt seine Buchse mit.
settings-radio-receive-antenna = Empfangsantenne
settings-radio-radio-s-separate-receiving-antenna =
    Die eigene Empfangsantenne des Funkgeräts — RX ANT auf der Rückseite — wird in den Empfangsweg geschaltet oder daraus entfernt. Eigene Einstellung, dieselbe wie die Taste RX ANT am Funkgerät.

    Die Antenne an der Hauptbuchse bleibt in beiden Fällen beim Senden: Dies ist ein zusätzlicher Eingang zum Empfangen, eine Schleife oder eine Beverage, keine Wahl der Buchse.

    Anders als die Antenne oben wird dies *nicht* hier gemerkt. Das Funkgerät hält es selbst je Band, daher liest sdroxide es nach jedem Bandwechsel zurück und zeigt, was das Funkgerät meldet; nur das Ankreuzen dieses Feldes verändert es.
settings-radio-receive-path = Im Empfangsweg
settings-radio-radio-power = Funkgerät-Strom
settings-radio-switch-radio-itself-off-back =
    Das Funkgerät selbst aus- und wieder einschalten, über die Steuerverbindung. Das ist das einzige echte Ein/Aus im Programm — nicht der eigene Schalter LINK von sdroxide, der das Ende von sdroxide schließt und das Funkgerät laufen lässt.

    Damit das Wieder-Einschalten irgendetwas erreicht, muss das Steuerende des Funkgeräts im ausgeschalteten Zustand wach bleiben. Über das Netzwerk leistet das Network Control. Über ein serielles Kabel ist es der CI-V-Port des Funkgeräts, der bei einem Gerät, das vorne ausgeschaltet und nicht abgezogen ist, aus dem Netz versorgt bleibt; sdroxide sendet vor dem Einschalten den Weckvorgang, den die eigene Dokumentation von Icom verlangt.

    Das Ausschalten beendet das Audio und die Messgeräte — es gibt kein Funkgerät mehr dahinter —, und sdroxide hält die Steuerverbindung offen, damit das Wieder-Einschalten ein Ziel hat.
settings-radio-on = Ein
settings-radio-off = Aus
settings-radio-waiting-configuration-machine-radio-attached = Warte auf die Konfiguration des Rechners, an dem das Funkgerät angeschlossen ist.
settings-radio-sound-format = Tonformat
settings-radio-invert-spectrum = Spektrum umkehren
settings-radio-swap-i-q = I/Q tauschen
settings-radio-mirror-panadapter-about-tuned-frequency =
    Das Panorama um die abgestimmte Frequenz spiegeln, für ein Funkgerät, das I und Q auf seiner Soundkarte vertauscht führt. Das verrät sich durch einen Wasserfall voller überzeugender Signale, die alle auf der falschen Seite der Skala liegen, wobei SSB auf dem gegenüberliegenden Seitenband herauskommt — das Vertauschen der beiden Kabel an der Soundkarte behebt es ebenso.

    Nur Empfang: Beim Senden gibt das Funkgerät ein einziges echtes Audiosignal aus, das kein Seitenband hat, das man umkehren könnte.
settings-radio-i-q-sample-rate = I/Q-Abtastrate
settings-radio-how-fast-radio-s-i =
    Wie schnell die I/Q-Soundkarte des Funkgeräts läuft — und damit wie viel Band das Panorama zeigt, denn ein Quadraturstrom umfasst seine ganze Abtastrate: 48 kHz ergibt ±24 kHz beiderseits der Skala, 192 kHz ergibt ±96.

    Die Karte hat das letzte Wort. Eine, die die hier gewählte Rate nicht kann, wird auf die nächsthöhere Rate geöffnet, und das Panorama ist entsprechend schmaler — das Protokoll meldet das beim Start, wenn es passiert.

    Eine schnellere Karte bedeutet auch mehr Arbeit: Der Rechner hat anteilig weniger Zeit, sie zu leeren, und eine, die nicht mithält, verwirft Abtastwerte. Das zeigt sich, wenn das Audio zerfällt, während der Wasserfall perfekt aussieht, und es wird ebenfalls protokolliert. Sehen Sie das, gehen Sie einen Schritt zurück.
settings-radio-t-48-khz-24-khz = 48 kHz (±24 kHz)
settings-radio-t-96-khz-48-khz = 96 kHz (±48 kHz)
settings-radio-t-192-khz-96-khz = 192 kHz (±96 kHz)
settings-radio-t-384-khz-192-khz = 384 kHz (±192 kHz)
settings-radio-i-q-centre-offset = I/Q-Mittenversatz
settings-radio-how-far-radio-s-i =
    Wie weit der I/Q-Ausgang des Funkgeräts über der eigenen Skala zentriert ist, für ein Gerät, dessen Empfangs-ZF von null weg verschoben wurde — bei einem Elecraft, wo MENU:RX SHFT auf 8.0 statt NOR gestellt ist, was die Skala vom DC-Spike des Mischers wegnimmt (und verhindert, dass ein starker nahe gelegener SSB-/AM-Sender als AM demoduliert wird).

    Kein Konverter: Das Funkgerät zeigt und sendet weiterhin auf der echten Frequenz, und hier wird nie etwas an es gesendet. Dies sagt nur, wo die Abtastwerte auf der Soundkarte liegen, und der Strom wird beim Eintreffen auf die Skala zurückverschoben.

    Auf 0 lassen, sofern Sie einen solchen Menüpunkt nicht eingeschaltet haben. Liegen Signale doppelt so weit vom Versatz entfernt, ist das Vorzeichen umgekehrt.
settings-radio-iq-correction = IQ-Korrektur
settings-radio-cancel-mirror-image-every-signal =
    Das Spiegelbild jedes Signals und den DC-Spike in der Mitte des Wasserfalls aufheben.

    Der I/Q-Ausgang eines Funkgeräts besteht aus zwei analogen Pfaden, die nie ganz gleich verstärken und nie genau 90° auseinanderliegen. Das hinterlässt eine Kopie jedes Signals, gespiegelt um die abgestimmte Frequenz, meist 30–40 dB leiser — stark genug, um wie eine Station auszusehen, die gar nicht da ist, und als solche dekodiert zu werden. Funkgeräte mit Abgleichreglern an der Frontplatte stellen genau dies ein; hier wird es aus dem empfangenen Rauschen gemessen und braucht keine Einstellung.

    Eingeschaltet lassen. Ausschalten, wenn Sie AM genau auf dem Träger hören — der Träger ist Gleichstrom und fällt damit zusammen mit dem Spike — oder um zu prüfen, ob ein Signal echt ist, indem Sie beobachten, ob es das überlebt.
settings-radio-cancel-mirror-images = Spiegelbilder aufheben
settings-radio-dc-notch = DC-Kerbe
settings-radio-widen-hole-taken-out-middle =
    Das Loch in der Mitte des Frequenzbereichs verbreitern, für ein Funkgerät, dessen Mittenspike breiter ist als der Versatz darunter.

    0 lässt den gewöhnlichen Sperrbereich, der einige zehn Hertz breit ist und den Versatz entfernt, ohne sonst etwas zu berühren. Hochdrehen, und der untere Rand des Bereichs fällt mit weg: ein Hochpass erster Ordnung, 3 dB unter dem hier eingestellten Wert und darunter weiter abfallend.

    Er ist auf die Mitte des I/Q des Funkgeräts zentriert, also auf die Skala, außer der obige Versatz hat sie verschoben — daher geht alles, was dort abgestimmt ist, mit weg. Ein CW-Ton bei 600 Hz liegt innerhalb einer Einstellung von 600 Hz.
settings-radio-panadapter-bw = Panorama-Bandbreite
settings-radio-cat-family = CAT-Familie
settings-radio-rigctld-address = rigctld-Adresse
settings-radio-host-port-running-hamlib-rigctld =
    Host:Port eines laufenden Hamlib-rigctld — 127.0.0.1:4532 ist dessen eigener Standard, auf diesem Rechner.

    Starten Sie einen zum Beispiel mit `rigctld -m 2028 -r /dev/ttyUSB0 -s 38400`, wobei -m die Hamlib-Modellnummer Ihres Funkgeräts ist (`rigctl -l` listet sie auf).

    Dies ist der Sammelweg: Er erreicht Frequenz, Betriebsart, PTT, Leistung, S-Meter und SWR und sonst nichts. Wo eine der eigenen Familien oben zu Ihrem Funkgerät passt, leistet sie mehr — Tasten aus dem eigenen Textpuffer des Geräts, das Empfangsfilter und Messskalen je Modell.
settings-radio-flrig-address = flrig-Adresse
settings-radio-host-port-running-flrig-127 =
    Host:Port eines laufenden flrig — 127.0.0.1:12345 ist dessen eigener Standard, auf diesem Rechner. flrig spricht XML-RPC, solange es läuft; der Port steht unter seiner Config → Setup → Server.

    Wie die Hamlib-Option steuert dies einen Dienst statt des Funkgeräts, aber über den eigenen Treiber von flrig je Modell — bei einer Reihe von Geräten ist dessen Leistungs- und Filterbehandlung die treuere der beiden. Es erreicht Frequenz, Betriebsart, PTT, Sendeleistung (in ganzen Watt), die Empfangsbandbreite, das S-Meter, SWR und Ausgangsleistung. CW wird über den eigenen cwio-Port von flrig getastet, der in flrig selbst eingerichtet werden muss.
settings-radio-serial-port = Serieller Port
settings-radio-select = — wählen —
settings-radio-baud = Baud
settings-radio-data-bits = Datenbits
settings-radio-parity = Parität
settings-radio-stop-bits = Stopbits
settings-radio-force-rts = RTS erzwingen
settings-radio-force-dtr = DTR erzwingen
settings-radio-ptt-method = PTT-Methode
settings-radio-how-transmit-keyed = Wie das Senden getastet wird.
settings-radio-how-transmit-keyed-network-link = Wie das Senden getastet wird. Eine Netzwerkverbindung hat keine Steuerleitungen, daher tasten DTR und RTS hier nichts — CAT verwenden, das den Dienst bittet, das Funkgerät zu tasten, oder VOX.
settings-radio-mode-control = Betriebsartensteuerung
settings-radio-digimode-mode = Digimode-Betriebsart
settings-radio-what-put-radio-mode-sdroxide =
    Auf welche Betriebsart das Funkgerät für eine Betriebsart gestellt wird, die sdroxide über seine Soundkarte moduliert. „DIGI“ wählt die Stellung DATA/PKT des Geräts, die das Sendeaudio vom USB- oder ACC-Eingang nimmt, mit der Sprachverarbeitung des Mikrofonwegs außen vor; „USB“ lässt es auf dem einfachen Seitenband, für ein Funkgerät, das SSB schon von seinem Dateneingang moduliert, oder eines ohne DATA-Stellung.

    Es betrifft alle digitalen Betriebsarten — FT8, FT4, PSK, RTTY, SSTV und die übrigen — und CW, das als „Soundkarte“ gesendet wird, und überschreibt dafür die Betriebsartensteuerung. SSTV ist der Fall, dessen Seitenband dem Band folgt, dort bedeutet es die DATA-Stellung auf diesem Seitenband: USB-D über 40 m, LSB-D bei und unter 40 m. SSTV-FM gehört nicht dazu — ein FM-Träger hat kein Seitenband, das man wählen könnte.
settings-radio-cw-keying = CW-Tastung
settings-radio-how-cw-panel-s-keyer =
    Wie der Keyer des CW-Panels sendet. „Geräte-Keyer“ stellt das Funkgerät auf CW und gibt ihm den zu sendenden Text an seinen eigenen Keyer. Er nutzt die Keyer-Geschwindigkeit des Geräts (aus dem WPM des Panels eingestellt), und bei Yaesu sendet er über Keyer-Speicher 1 und überschreibt, was darin stand.

    Dafür braucht es Break-in am Funkgerät, das sdroxide bei jeder Nachricht bei Icom, Yaesu und Kenwood selbst einschaltet — ein Gerät mit ausgeschaltetem Break-in nimmt die Nachricht an und sendet nichts. Es bleibt danach eingeschaltet, und ein volles Break-in wird nie auf Halb zurückgenommen.

    „Soundkarte“ sendet stattdessen den getasteten Mithörton als Audio (MCW), ein Ton auf Skala + Tonhöhe — und weil ein Gerät im CW-Modus seinen Soundkarten-Eingang völlig ignorieren würde, folgt die Wahl von CW dann der Einstellung Digimode-Betriebsart (USB, DATA oder Radio-gesteuert), statt das Gerät auf CW umzuschalten. Dort „Radio-gesteuert“ wählen für Geräte, deren Datenstellung sich nicht über CAT befehlen lässt (das U-D eines Xiegu), und die Stellung selbst einstellen, wie bei FT8.
settings-radio-poll-rate = Abfragerate
settings-radio-how-often-radio-asked-what =
    Wie oft das Funkgerät gefragt wird, was es tut — seine Skala, seine Betriebsart und seine Messgeräte. Das ist die Hälfte der Steuerverbindung, die vom Funkgerät zurück zu sdroxide läuft: Dreht man am VFO-Knopf des Geräts oder ändert man seine Betriebsart an der Frontplatte, folgen Anzeige, Band und Panorama innerhalb einer Abfrage.

    Das ist außerdem der gesamte Steuerverkehr, den dieses Ende erzeugt, und bei einem modernen Icom ist dieser Verkehr nicht kostenlos: Der CI-V-Port und die Soundkarte sitzen hinter einem USB-Hub im Inneren des Funkgeräts, daher ist jeder hier angeforderte Rahmen Buszeit, die dem Audio fehlt — als Aussetzer zu hören, die wie ein DSP-Fehler aussehen und keiner sind. Der Standard von 2 Hz liegt eine halbe Sekunde hinter dem Knopf und ist ruhig genug, um dem Audio nicht im Weg zu sein. Erhöhen, wo der Steuerport ein eigenes Gerät ist (ein separater USB-Seriell-Adapter, ein netzwerkbasiertes rigctld); senken, wenn das Audio des Funkgeräts zerfällt.

    Ein Icom mit eingeschaltetem CI-V-Transceive im eigenen Menü braucht dafür für die Skala nichts: Er meldet den Knopf in dem Moment, in dem er sich bewegt, und sdroxide stellt die Skalenabfrage ein, sobald es das sieht — und nimmt sie wieder auf, falls das Funkgerät sich je bewegt, ohne es zu sagen.

    Die Rate gilt der Skala. Die Betriebsart wird nur bei jeder vierten Abfrage mitgenommen, da sie eine Einstellung ist, die sich ein paar Mal am Abend ändert statt einer, die einem Knopf folgt.
settings-radio-send-command = Sendebefehl
settings-radio-which-transceiver-generation-keys-rig =
    Welche Transceiver-Generation das Gerät tastet, für die PTT-Methode „CAT“. Die beiden sind sich uneins darüber, was der TX-Parameter bedeutet, und auf der Leitung ist nichts, was sie unterscheidet, daher das wählen, das zu Ihrem Funkgerät passt.

    „TS-2000-Stil (TX;)“ — TS-480, TS-570, TS-870, TS-2000: das gewöhnliche Senden, auf dem Hauptband.

    „TS-590-Stil (TX1;)“ — TS-590S/SG, TS-890, TS-990: DATA SEND, das mit dem ACC2-/USB-Audioeingang aktiv tastet. Bei diesen Geräten wählt das einfache Senden stattdessen das Mikrofon und stummt das Audio, das sdroxide sendet.

    Falsch eingestellt sendet ein TS-590 Stille — ein TS-2000 aber auf dem Unterband, das ein ganz anderes Band ist.
settings-radio-radio = Funkgerät
settings-radio-k3-k3s-kx3-kx2-k4 = K3 · K3S · KX3 · KX2 · K4
settings-radio-one-profile-covers-family-k3 =
    Ein Profil deckt die Familie ab: der K3-Befehlssatz, den KX3, KX2 und K4 alle beantworten. Hier gibt es nichts zu wählen — wie viele Watt der Regler Aussteuerung umfasst (12 bei einem nackten KX2 oder KX3, 110 mit einem KPA3 oder einem KXPA100), wird beim Öffnen des Ports aus der Abfrage der Optionsmodule des Geräts gelesen.

    Beachten Sie die Baudraten oben: K3, K3S, KX3 und KX2 gehen nicht schneller als 38400, und ein Gerät, das unter der hier gewählten Rate eingestellt ist, antwortet überhaupt nicht.
settings-radio-transmit-input = Sendeeingang
settings-radio-where-radio-takes-transmit-audio =
    Woher das Funkgerät sein Sendeaudio nimmt — der Befehl TI des Geräts, das ist Menü 32 „TX IN“ an der Frontplatte. Wird beim Öffnen des Ports gesetzt.

    „USB-Audio“ ist es, was diese Schnittstelle zum Arbeiten bringt: Der FDM-DUO sendet, was sdroxide in seine USB-Soundkarte gibt. Ein Funkgerät auf „Mikrofon“ sendet stattdessen den Raum, und auf dem Bildschirm steht nichts davon.

    „Auto“ lässt das Gerät wählen — das Mikrofon bei einem PTT-Druck am Mikrofon, den USB-Port bei CAT oder RTS-Tastung.

    „Wie am Funkgerät eingestellt lassen“ sendet gar kein TI, für eine Bedienperson, die dies an der Frontplatte einstellt.
settings-radio-which-two-sockets-back-receiver =
    Welche der beiden Buchsen auf der Rückseite der Empfänger hört — der Befehl AN des Geräts, das ist Menü 31 „ANTENNAS“ an der Frontplatte und die Anzeige „ANT 1 2“ auf seinem Display.

    „RTX“ ist eine Antenne, die beide Aufgaben übernimmt, an der M-Buchse, die auch das Senden führt. „Nur RX“ verlegt den Empfang auf die zweite Buchse und lässt das Senden auf RTX — eine Empfangsantenne, eine Schleife oder eine Beverage, mit der Richtantenne weiter am Sender.

    Wirkt sofort und wird beim Öffnen des Steuerports vom Funkgerät zurückgelesen: Das ist die eigene Einstellung des Geräts und keine hier gehaltene Kopie.
settings-radio-one-profile-covers-both-there =
    Ein Profil deckt beide ab. Hier gibt es nichts zu wählen — das Funkgerät nennt sich beim Öffnen des Ports selbst.

    Das ist nur die CAT-Hälfte: Sie steuert Skala, Betriebsart, PTT, das S-Meter, das SWR und die Sendeleistung über den CAT-USB-Port des Geräts und nimmt Audio von seinem USB-Audio-Port. Der dritte USB-Port des FDM-DUO führt Breitband-I/Q, das diese Schnittstelle nicht liest — dafür die Schnittstelle ELAD oben wählen.

    Die Baudrate beachten: Menü 70 „CAT BAUD“ am Funkgerät muss übereinstimmen, und es wird auf 38400 ausgeliefert.

    CW wird über die eigene Taste oder das Paddle des Funkgeräts getastet. Der FDM-DUO hat keinen Befehl, der Text annimmt, daher kann das CW-Panel ihn nicht über CAT tasten.
settings-radio-one-profile-covers-range-qrp =
    Ein Profil deckt die Reihe ab: der eigene Befehlssatz von QRP Labs, eine Teilmenge des Kenwood TS-480 mit einer ganzen Menge dazu. Hier gibt es nichts zu wählen — das Funkgerät nennt sich und seine Firmware-Version beim Öffnen des Ports, und die Version entscheidet, ob die Abfrage des SWR-Schutzes überhaupt gestellt wird.

    Nicht das Kenwood-Profil: Beim QMX ist der PC-Befehl das Leistungs-MESSGERÄT statt der Leistungssteuerung, daher würde ein als Kenwood gesteuertes Funkgerät seine Messwerte so schreiben, als wären es Einstellungen, und MD8 — eine Kenwood-Betriebsart — ist hier SWR Tune.

    Die Sendeleistung wird am Funkgerät eingestellt. Dafür gibt es keinen CAT-Befehl, daher erreicht der Regler Aussteuerung nur den Pegel des Audios, das in die Soundkarte geht.

    Das Empfangsfilter gehört ebenfalls dem Funkgerät: Es meldet die Breite, die seine Betriebsart ergibt (3,2 kHz in Digi, 300 Hz in CW), und bietet nichts, womit man sie ändern könnte.
settings-radio-baud-rate-ignored-port-usb = Die Baudrate wird ignoriert — der Port ist USB
settings-radio-qmx-serves-own-virtual-com =
    Ein QMX stellt seine eigenen virtuellen COM-Ports über USB bereit, daher hat die oben eingestellte Rate keine Wirkung auf beiden Seiten. Es bietet bis zu drei davon an; antwortet das Funkgerät nicht, liegt der wahrscheinlichste Grund bei dem falschen.

    Eines zählt beim Port tatsächlich: Ein an ihn gesendetes Wagenrücklauf-Zeichen schaltet das Funkgerät für den Rest der Sitzung in den Terminalmodus. sdroxide sendet nie eines — ein Terminalprogramm, das auf demselben Port offen bleibt, aber schon, und CAT hört in dem Moment auf zu funktionieren, in dem es das tut.
settings-radio-i-q-mode-switched-radio = I/Q-Modus wird am Funkgerät eingeschaltet
settings-radio-radio-s-sound-card-carries =
    Die Soundkarte des Funkgeräts führt entweder demoduliertes Audio oder das rohe I/Q, das sein ADC sieht, und sdroxide setzt, was das Tonformat oben verlangt, beim Öffnen des Ports (der Befehl Q9 des Geräts, der Menüpunkt „IQ mode“).

    Der Mittenversatz und die Abtastrate oben sind auf das gesetzt, was ein QMX braucht: Sein Empfänger ist ein Superhet mit einer ZF von 12 kHz, daher liegt der Synthesizer 12 kHz unter der Skala, und alles auf der Karte liegt 12 kHz über der Mitte des Bereichs. Die Karte läuft mit 48 kHz, was das Panorama 48 kHz breit macht.

    Im CW-Modus verschiebt das Gerät diese ZF um weitere ~700 Hz, was ein einziger Wert nicht nachverfolgen kann — von Hand hinzufügen, wenn man das Panorama mit dem Gerät in CW betreibt. In Digi, wo ein als I/Q-Frontend benutztes Funkgerät normalerweise steht, gibt es nichts hinzuzufügen.

    QRP Labs weisen darauf hin, dass der I/Q-Modus nicht für WSJT-X und andere Programme geeignet ist, die demoduliertes Audio erwarten; hier geschieht die Demodulation auf dieser Seite, daher geht es.
settings-radio-rs-hfiq-5-w-hf = RS-HFIQ (5-W-KW-Transceiver)
settings-radio-hobbypcb-s-rs-hfiq-there =
    Der RS-HFIQ von HobbyPCB. Hier gibt es nichts zu wählen: Das Profil ist der ganze Befehlssatz des Funkgeräts, und die Firmware-Version wird beim Öffnen des Ports protokolliert.

    Die Steuerverbindung trägt zwei Dinge und nicht mehr — wohin der Oszillator gelegt wird und ob gesendet wird. Es gibt keinen Befehl für Betriebsart, Leistung, Filter, Rauschsperre oder Messwert, weil das Gerät keinen davon hat. Alles andere ist Sache von sdroxide: Was von der Soundkarte kommt, ist komplexes Basisband, das auf der Skala zentriert ist, daher finden Betriebsart, Filter und Modulation alle auf dieser Seite statt.

    Der Frequenzbefehl umfasst 3–30 MHz. Verlangt man etwas außerhalb, lehnt das Gerät es ab und bleibt, wo es ist; das Protokoll meldet das.

    CW wird als Audio über die Sendekette getastet — dieses Gerät hat keinen Keyer, der Text annimmt. (Es hat einen internen CW-Generator; sdroxide nutzt ihn nie, auf Rat der Firmware selbst.)
settings-radio-sound-format-baud-ptt-are = Tonformat, Baudrate und PTT sind die des Funkgeräts, keine Einstellungen
settings-radio-all-three-have-been-filled =
    Alle drei sind eingetragen, und keiner davon ist eine Vorliebe:

    • Tonformat ist I/Q (Stereo) — die Karte trägt komplexes Basisband und sonst nichts.
    • 57600 Baud, 8N1 — die einzige Rate der Firmware, ohne Menü zum Ändern. Jede andere ist eine stumme Verbindung, daher wird sie beim Öffnen des Ports festgelegt.
    • PTT über CAT — der Befehl *X ist der einzige Sendeschalter, den die Schnittstelle hat.

    Der Mittenversatz ist auf null gesetzt: Der Synthesizer läuft mit dem Vierfachen der Skala in einen Quadraturdetektor, daher ist die Mitte des Bereichs die Skala. Setzen Sie die Abtastrate oben auf das, womit Ihre Soundkarte tatsächlich läuft; das macht das Panorama so breit, wie es ist.
settings-radio-dl2man-pe1nnz-s-pocket-qrp =
    Der Taschen-QRP-Transceiver von DL2MAN/PE1NNZ und die offene uSDX-Firmware, aus der er hervorging. Er emuliert einen Kenwood TS-480, doch die Teilmenge ist dünn: Skala, Betriebsart, PTT und das Löschen von RIT/XIT sind alles, was er beantwortet. Kein S-Meter, kein SWR, keine Leistungssteuerung, kein VFO B, kein Split und kein CAT-Keyer, daher bewirken der Regler Aussteuerung und die Messgeräte über CAT nichts — der Audiopegel ist die einzige Sendesteuerung, die es gibt.

    CW wird als Audio (MCW) über das Seitenband getastet oder mit der Taste oder dem Paddle am Gerät; es gibt keinen Befehl, der Text annimmt.
settings-radio-audio = Audio
settings-radio-tr-usdx-has-no-sound =
    Ein (tr)uSDX hat keine eigene Soundkarte und zwei Wege, gehört zu werden, und welcher richtig ist, ist eine Tatsache über Ihre Shack-Einrichtung statt über das Funkgerät.

    „Ein Kabel“ nimmt das Audio innerhalb der CAT-Serienverbindung selbst — den eigenen 8-Bit-Strom des Funkgeräts, über dasselbe USB-Kabel wie die Steuerung, ohne etwas anderes einzustecken. Der Haken liegt bei der Firmware: Sie kann keinen CAT-Befehl annehmen, solange ihr Strom läuft (ein in den Strom gesendeter Befehl hält ihn nicht an, sondern bricht ihn ab), daher fragt dieser Modus nie ab. Abstimmung und Betriebsartwechsel werden gesendet, sobald Sie sie vornehmen, und jeder hält den Strom kurz an und startet ihn wieder; erwarten Sie also bei jedem Wechsel eine kurze Lücke. Beim bloßen Zuhören wird nichts gesendet. Die eigene Skala und Betriebsart des Funkgeräts werden ebenfalls nicht verfolgt — dafür gibt es kein Lesen —, daher von hier aus steuern.

    „USB-Soundkarte“ nimmt das Audio von einer externen Soundkarte, die an die Lautsprecher-/Mikrofonbuchse 3,5 mm des Funkgeräts angeschlossen ist, und verhält sich wie jedes andere CAT-Funkgerät: Die Skalenabfrage läuft, daher werden der eigene Knopf und die Betriebsart des Funkgeräts verfolgt. Die Karte unten unter Funkgerät-Audio wählen.

    Beide Modi öffnen den Port mit gehaltenem DTR-High: Das DTR des seriellen Adapters ist mit dem Reset des Funkgeräts verdrahtet, daher startet das Öffnen des Ports es neu, und die erste Sekunde oder zwei sind still, während es hochfährt. DTR wird nie zum Tasten benutzt.
settings-radio-radio-model = Funkgerätemodell
settings-radio-which-icom-two-things-ci =
    Welcher Icom, für die zwei Dinge, die CI-V nicht bei allen gleich macht.

    Die Transceiver-Adresse unten wird daraus gefüllt: Jedes Modell hat eine andere, und ein an die falsche Adresse gesendeter Rahmen wird einfach ignoriert — ein Funkgerät, das nichts antwortet, ohne irgendeinen Fehler, der sagt, warum.

    Es entscheidet außerdem, ob sdroxide den DATA-Modus wählen kann. Bei CI-V sind USB und USB-DATA dasselbe Betriebsart-Byte, und ein eigener Befehl unterscheidet sie; ohne ihn geht eine digitale Betriebsart über den Mikrofoneingang hinaus, mit der Sprachverarbeitung und dem SSB-Filter des Geräts im Weg.

    „Andere“ überlässt Ihnen die Adresse und sendet gar keinen DATA-Modus-Befehl.
settings-radio-radio-id-hex = Funkgerät-ID (hex)
settings-radio-show-radio-s-spectrum-scope = Das Spektrum-Oszilloskop des Funkgeräts anzeigen
settings-radio-scope-span = Scope-Bereich
settings-radio-how-wide-sweep-radio-keeps = Wie breit es abgetastet wird. Das Funkgerät behält, was zuletzt auf seinem eigenen Bildschirm gewählt wurde — oft ein paar kHz. Ein Bereich, der hier gesetzt wird, schaltet das Scope außerdem in den Mittenmodus, sodass es der Skala folgt. Es ändert auch die eigene Anzeige des Funkgeräts; „Wie am Funkgerät eingestellt“ lässt sie unberührt.
settings-radio-press-apply-reconnect-switch-without = „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-radio-audio-sound-card = Funkgerät-Audio (Soundkarte)
settings-radio-waiting-sound-cards-machine-radio = Warte auf die Soundkarten des Rechners, an dem das Funkgerät angesteckt ist.
settings-radio-radio-rx = Vom Funkgerät (RX)
settings-radio-radio-tx = Zum Funkgerät (TX)
settings-radio-apply-reconnect = Übernehmen / erneut verbinden
settings-radio-reopen-cat-rig-these-sound = Das CAT-Funkgerät mit diesen Soundkarten erneut öffnen — ohne Neustart
settings-radio-reconnects-radio-without-restarting = Verbindet das Funkgerät ohne Neustart erneut.
settings-radio-receive-radio-pc = Empfang (Funkgerät → PC)
settings-radio-sound-card-radio-s-own = Die Soundkarte, über die das eigene Audio des Funkgeräts hereinkommt — seine Kopfhörerbuchse in den Line- oder Mikrofon-Eingang des Computers. Alles auf dem Panorama und in den Decodern kommt hier an.
settings-radio-transmit-pc-radio-mic = Senden (PC → Funkgerät-Mikrofon)
settings-radio-sound-card-carries-audio-radio = Die Soundkarte, die das Audio trägt, das das Funkgerät aussenden muss — der Ausgang des Computers in die Mikrofonbuchse des Funkgeräts. Das Funkgerät tastet sich selbst per VOX, sobald hier Audio ankommt; dieses Gerät ist daher still, außer wenn über das Funkgerät gesendet wird.
settings-radio-radio-has-no-control-cable = Dieses Funkgerät hat kein Steuerkabel — es tastet sich per VOX selbst, daher gibt es keine PTT zu befehlen. Senden ist Audio in das Mikrofon des Funkgeräts.
settings-radio-reopen-radio-these-sound-cards = Das Funkgerät mit diesen Soundkarten erneut öffnen — ohne Neustart
settings-radio-devices = Geräte
settings-radio-discover = Suchen
settings-radio-no-devices-press-discover = keine Geräte — Suchen drücken
settings-radio-manual-ip = IP manuell
settings-radio-optional-e-g-192-168 = optional, z. B. 192.168.1.50
settings-radio-sample-rate = Abtastrate
settings-radio-receiver-ddc = Empfänger (DDC)
settings-radio-protocol-2-board-carries-several = Eine Protokoll-2-Karte trägt mehrere unabhängig abstimmbare Empfänger (DDCs) über eine Verbindung — dieses Funkgerät auf DDC1 und ein anderes Funkgerät, dieselbe Adresse, auf DDC2 betreiben. Der Sender gehört dem DDC1-Funkgerät. Protokoll-1-Karten haben nur DDC1.
settings-radio-lna-gain = LNA-Verstärkung
settings-radio-front-end-gain-hermes-lite = Frontstufen-Verstärkung eines Hermes-Lite 2. Wirkt sofort — ohne erneutes Verbinden — und wird als der Pegel gemerkt, mit dem das Funkgerät startet. Zu hoch übersteuert den ADC und das ganze Band wirkt verzerrt; zu niedrig, und der Empfänger wird taub.
settings-radio-overload-protection = Überlastschutz
settings-radio-back-lna-gain-off-itself =
    Die LNA-Verstärkung selbstständig zurücknehmen, solange die Karte meldet, dass ihr ADC übersteuert, und sie wieder hochfahren, sobald das aufhört.

    Ein Hermes-Lite 2 tastet das ganze Band von 0–38 MHz auf einen einzigen 12-Bit-Wandler ab, ohne Mischer und ohne Vorselektor davor, daher kann ein Rundfunksender eine Bandbreite entfernt ihn in die Übersteuerung treiben, während das Band, das Sie betrachten, nichts Auffälliges zeigt — der Rauschboden steigt, alles mischt sich, und die Decoder hören auf. Die Karte weiß es, und dies handelt danach.

    Standardmäßig aus: Es bewegt einen Regler, den Sie eingestellt haben. Beim Senden geschieht nichts.
settings-radio-wind-gain-back-when-adc = Die Verstärkung zurücknehmen, wenn der ADC übersteuert
settings-radio-applies-apply-reconnect-main-window = Wirkt nach Übernehmen / erneut verbinden. Die Verstärkungsleiste des Hauptfensters zeigt die Verstärkung, die die Karte tatsächlich fährt, daher folgt sie der Schleife, und Sie können beobachten, was sie tut; der Regler oben ist der Pegel, mit dem das Funkgerät startet, und die Schleife schreibt ihn nicht um.
settings-radio-step = {"  "}Schritt
settings-radio-how-far-gain-moves-each = Wie weit sich die Verstärkung jedes Mal bewegt, in dB. Eins ist der Schritt, den das eigene Verstärkungsregister der Karte hat.
settings-radio-attack-decay = {"  "}Angriff / Abklingen
settings-radio-how-often-gain-may-come =
    Wie oft die Verstärkung sinken darf, solange der Wandler übersteuert, und wie oft sie wieder steigen darf, sobald das aufhört.

    Die beiden liegen absichtlich hundertmal auseinander. Sofort zurückweichen: Jede Millisekunde Übersteuerung ist ein Empfänger voller Intermodulation. Langsam zurückkehren: Was es verursacht hat — ein Nachbar, der tastet, ein Rundfunksender, der beim Einbruch der Dunkelheit hochfährt —, ist meist nicht verschwunden, und eine Schleife, die sich so schnell erholte, wie sie zurückwich, würde den Abend über die Schwelle hin- und herschwingen. 100 ms und 10 s pro Dezibel haben sowohl PowerSDRs Auto S-Att als auch N1GPs HermesIntf verwendet.
settings-radio-range = {"  "}Bereich
settings-radio-lowest-highest-gain-loop-may = Die niedrigste und höchste Verstärkung, die die Schleife nutzen darf, in dB. Die Obergrenze verhindert, dass sie entscheidet, wie empfindlich Ihr Empfänger sein soll; die Untergrenze ist der Punkt, an dem Sie festlegen, dass die Übersteuerung unterhalb einer Schwelle das Problem eines anderen ist und die Antwort ein Filter ist, nicht noch zwanzig Dezibel.
settings-radio-filter-board = Filterkarte
settings-radio-accessory-board-hermes-lite-2 = Zusatzkarte am J16-Header des Hermes-Lite 2 (oder den Open-Collector-Ausgängen jeder anderen openHPSDR-Karte — Protokoll 2 eingeschlossen). „N2ADR“ wählt je Band ein Relais; „Alex / Hermes-Bandcode“ legt die Bandnummer auf die Ausgänge 1–4, was die Alex-Karte eines ANAN, ein Zeus SDR, ein HiQSDR und Quisk erwarten. „Eigene“ öffnet unten eine Tabelle, in der Sie die sieben Ausgänge selbst angeben, Band für Band, beim Empfang und beim Senden — für einen Antennenumschalter, einen Bandentschlüssler eines Verstärkers oder alles andere, worauf keine der Voreinstellungen passt; jede der Voreinstellungen kann als Ausgangspunkt hineinkopiert werden. Auf „Keine“ lassen, sofern nicht wirklich etwas eingebaut ist: Diese sieben Pins sind allgemeine Open-Collector-Ausgänge, und Bediener verdrahten sie auch mit der PTT eines Verstärkers, Antennenrelais und Transverter-Umschaltung. Eine Ansteuerung aus den Banddaten würde alles in Betrieb setzen, was angeschlossen ist. Wirkt nach Übernehmen / erneut verbinden.
settings-radio-io-board-rx-input = IO-Karte-RX-Eingang
settings-radio-where-n2adr-hl2ioboard-takes-receive = Woher eine N2ADR-HL2IOBoard ihr Empfangssignal nimmt. Die Karte selbst wird automatisch gefunden und braucht keine Einstellung; diese existiert nur für Bediener, die deren eigene SMA-Buchsen verdrahtet haben. Ansonsten auf „Eigener Eingang des Funkgeräts“ lassen — die J9 der IO-Karte zu wählen, wenn nichts daran angeschlossen ist, lässt den Empfänger taub. Wirkt nach Übernehmen / erneut verbinden. Auf eine der J9-Wahlen gestellt, erscheinen die Eingänge außerdem bei der Steuerung ANT und werden je Band gemerkt.
settings-radio-power-amplifier = Leistungsverstärker
settings-radio-use-hermes-lite-2-s = Die eingebaute PA des Hermes-Lite 2 benutzen
settings-radio-default-what-you-want-unless = Standardmäßig an, und das ist, was Sie wollen, sofern ein externer Verstärker vom Niederleistungs-HF-Ausgang RF1 der Karte angesteuert wird. Ist es aus, tastet das Funkgerät trotzdem — das Sende-/Empfangsrelais schaltet und jede Zusatzkarte folgt —, aber die Antennenbuchse liefert überhaupt keine Leistung, und das Relais wird bewusst im Empfang gehalten. Bei anderen Karten als einem Hermes-Lite ohne Wirkung.
settings-radio-mirror-board-s-spectrum-about = Das Spektrum der Karte um die abgestimmte Frequenz spiegeln, beim Senden wie beim Empfang. Standardmäßig an: Ein Hermes-Lite 2 braucht das. Nur ausschalten, wenn Signale auf der falschen Seite der Skala erscheinen und nichts dekodiert wird — das verrät sich durch einen Wasserfall voller überzeugender Spuren, während SSB auf dem falschen Seitenband landet und FT8 gar keine Dekodierungen liefert.
settings-radio-frequency-correction = Frequenzkorrektur
settings-radio-crystal-tcxo-error-ppm-applied = Fehler von Quarz/TCXO in ppm, auf Empfang und Senden angewandt. Wirkt sofort.
settings-radio-ppm = {" "}ppm
settings-radio-transmit-buffer = Sendepuffer
settings-radio-how-far-ahead-real-time = Wie weit das Sendeaudio der Echtzeit voraus über das Netzwerk an die Karte gegeben wird, bevor sdroxide langsamer wird, um dem zu entsprechen. Die Karte selbst hält keinen solchen Puffer — dies erweitert nur den eigenen Spielraum von sdroxide. Erhöhen bei WLAN oder einem VPN, wo der niedrige Standard (richtig für eine direkte Kabelverbindung) nicht genug Reserve gegen Jitter bietet und das gesendete Audio oder die PTT stottert; höher kostet Sende-Latenz. Wirkt bei ÜBERNEHMEN, was sich wieder mit der Karte verbindet.
settings-radio-puresignal = PureSignal
settings-radio-adaptive-predistortion-linearise-transmitter-sample = Adaptive Vorverzerrung: den Sender anhand einer Probe dessen linearisieren, was er tatsächlich abgestrahlt hat, für rund zwanzig Dezibel weniger Intermodulation bei gleicher Leistung. Der Empfänger ist der Rückkopplungspfad — die Karte empfängt während einer Aussendung weiter —, daher müssen ein Richtkoppler und ein Dämpfungsglied eine Probe des Verstärkerausgangs in einen Eingang legen, den der Sende-/Empfangsschalter beim Senden nicht wegnimmt. Bei einem Hermes-Lite 2 ist das die PureSignal-Buchse der IO-Karte, mit dem Empfangseingang oben entsprechend eingestellt; bei einer Karte ohne sie der RX-Port eines ANAN oder ein externes Relais, das den Koppler für die Länge der Aussendung einschaltet. Gelesen wird dieser Empfänger — eine Karte mit einem zweiten ADC kann RX2 nicht als Rückkopplungsabgriff nutzen, wie ihre eigene Firmware es auch leitet. Ist nichts angekoppelt, rastet die Schleife nie ein, und der Sender bleibt genau so, wie er ohne sie gewesen wäre. Wirkt nach Übernehmen / erneut verbinden, und nur auf dem Funkgerät, das den Sender besitzt (DDC1).
settings-radio-correct-transmitter-receiver-s-own = Den Sender anhand der eigenen Rückkopplung des Empfängers korrigieren
settings-radio-table-steps = {"  "}Tabellenschritte
settings-radio-how-finely-amplifier-s-curve = Wie fein die Kurve des Verstärkers modelliert wird. Mehr Schritte folgen einer schärferen Knickstelle und brauchen länger zum Füllen; 32 ist ein vernünftiger Anfang.
settings-radio-adaptation = {"  "}Anpassung
settings-radio-how-fast-correction-follows-what = Wie schnell die Korrektur dem folgt, was der Koppler meldet. Langsam ist richtig — es mittelt die Kurve eines Verstärkers, die sich nicht bewegt, aus einem Rückkopplungspfad, der Rauschen enthält.
settings-radio-hold = Halten
settings-radio-stop-adapting-keep-correction-where = Anpassung beenden und die Korrektur dort lassen, wo sie ist — zum Messen und für eine Bedienperson, die mit dem zufrieden ist, was sie gelernt hat.
settings-radio-manual-ip-overrides-discovery-press = Eine manuelle IP überschreibt die Suche. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-open-collector-outputs-band = Open-Collector-Ausgänge nach Band
settings-radio-one-control-word-per-band = Ein Steuerwort je Band, so wie die Dokumentation Ihrer Hardware es angibt: Bit 0 ist Ausgang 1, Bit 6 ist Ausgang 7. RX ist gesetzt, während auf diesem Band empfangen wird, und TX, während der Sender getastet ist — gleicher Wert für ein Filter, verschiedene Werte für alles, was nur auf einer Seite des Umschaltens gehört (die Tastleitung eines Verstärkers, die Überbrückung eines Empfangsvorverstärkers). Nicht belegte Bänder auf 00 lassen, dann wird nichts angesteuert. Wirkt nach Übernehmen / erneut verbinden.
settings-radio-alex-band-code = ALEX-BANDCODE
settings-radio-replace-table-what-preset-would = Ersetzt die Tabelle durch das, was diese Voreinstellung auf jedem Band senden würde, als Ausgangspunkt zum Bearbeiten. Sonst ändert sich nichts in der Konfiguration.
settings-radio-clear = LÖSCHEN
settings-radio-every-output-off-every-band = Jeder Ausgang auf jedem Band aus, was „Keine“ bewirkt.
settings-radio-band = Band
settings-radio-rx = RX
settings-radio-tx = TX
settings-radio-outputs-asserted = Ausgänge gesetzt
settings-radio-other = Andere
settings-radio-everywhere-outside-amateur-bands-short = Überall außerhalb der Amateurfunkbänder — Kurzwellenempfang und alles, worauf die Skala eines Transverters landet und das kein Band abdeckt.
settings-radio-dongle = Stick
settings-radio-rescan = Erneut suchen
settings-radio-re-list-usb-bus-no = Den USB-Bus erneut auflisten. Es wird kein Gerät geöffnet, daher ist es sicher, während des Empfangs zu drücken.
settings-radio-no-dongles-press-rescan = keine Sticks — Erneut suchen drücken
settings-radio-first-one-found = — erster gefundener —
settings-radio-rtl2832u-s-resampler-reaches-225 = Der Resampler des RTL2832U reicht bis 225–300 kHz und 900 kHz–3,2 MHz, dazwischen nichts. Wirkt nach Übernehmen.
settings-radio-often-drops-samples = {"  "}(verwirft oft Abtastwerte)
settings-radio-agc = AGC
settings-radio-manual-setting-measurement-weak-signal = Manuell ist die Einstellung für Messung und digitale Schwachsignal-Betriebsarten. Der Tuner und der Demodulator haben unabhängige automatische Schleifen.
settings-radio-tuner-gain = Tuner-Verstärkung
settings-radio-applies-immediately-no-reconnect-tuner = Wirkt sofort — ohne erneutes Verbinden. Der Tuner hat 29 diskrete Stufen, daher rastet der Wert auf die nächste ein, die er erzeugen kann. Wird ignoriert, solange die Tuner-AGC läuft.
settings-radio-crystal-error-parts-per-million = Quarzfehler in Teilen pro Million. Mit RUST_LOG=sdroxide_rtlsdr=debug starten, dann gibt das Protokoll nach etwa 20 Sekunden den gemessenen Takt-Fehler aus — das ist die Zahl, die man eingibt. Wirkt sofort.
settings-radio-hf-reception = KW-Empfang
settings-radio-tuner-itself-starts-24-mhz =
    Der Tuner selbst beginnt bei 24 MHz. Ein RTL-SDR Blog V4 setzt darunter in der Hardware aufwärts um; andere Sticks erreichen KW nur, indem sie den ADC direkt abtasten, über den KW-Anschluss des V3. Automatisch gibt alles unter 24 MHz an den ADC und alles darüber an den Tuner.

    Die Direktabtastung deckt jedes KW-Band ab, einschließlich 17 m und 15 m — sie kommen in der zweiten Nyquist-Zone des ADC an, richtig herum. Am Eingang des ADC wird jedoch nichts gefiltert, daher kommt alles mit, was bei 28,8 MHz minus der Skala liegt: 10,7 MHz unter 17 m, 7,726 MHz unter 15 m.

    Ein Moduswechsel unterbricht den Strom kurz.
settings-radio-removes-dongle-s-own-dc =
    Entfernt den eigenen DC-Spike des Sticks aus der Mitte des Bereichs und das Spiegelbild, das jedes Signal daran gespiegelt zurücklässt, indem das Ungleichgewicht in den Abtastwerten selbst gemessen wird — ohne Kalibrierung, und sofort wirksam. Der Tuner hat keinen Versatz-Abstimmmodus, daher ist dies der einzige Weg, die Mitte freizubekommen.

    Ein AM-Träger, der genau auf der Skala abgestimmt ist, liegt ebenfalls bei DC, daher fällt er mit dem Spike weg: einen Kilohertz daneben abstimmen oder dies ausschalten.
settings-radio-remove-centre-spike-mirror-image = Mittenspike und Spiegelbild entfernen
settings-radio-bias-tee = Bias-Tee
settings-radio-feed-4-5-v-dc = ca. 4,5 V Gleichspannung über das Koaxkabel speisen
settings-radio-bias-tee-never-connect-transceiver = Bias-Tee ist EIN. Niemals einen Transceiver, eine geerdete Antenne oder einen anderswo gespeisten Vorverstärker anschließen, solange dies aktiv ist — die Gleichspannung läuft direkt die Speiseleitung hinunter.
settings-radio-receive-only-dongle-sample-rate = Nur Empfang. Stick und Abtastrate wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern.
settings-radio-server-address = Serveradresse
settings-radio-where-rtl-tcp-listening-address =
    Wo rtl_tcp lauscht: eine Adresse oder eine Adresse und ein Port. Der Port ist standardmäßig 1234, dem eigenen Standard von rtl_tcp.

    Auf der Gegenseite starten Sie es als `rtl_tcp -a 0.0.0.0` — gebunden an 127.0.0.1, wie es ohne -a geschieht, nimmt es nur Verbindungen von demselben Rechner an.

    Wirkt nach Übernehmen.
settings-radio-host-host-port-e-g = Host oder Host:Port, z. B. raspberrypi.local:1234
settings-radio-requested-server-same-resampler-limits =
    Beim Server angefragt, mit denselben Resampler-Grenzen wie ein lokaler Stick — es ist derselbe Chip auf der Gegenseite.

    Die Zahl neben jeder Rate ist, was sie auf der Verbindung kostet: Die Abtastwerte werden unkomprimiert gesendet, und eine Rate, die das Netzwerk nicht tragen kann, lässt rtl_tcp die Verbindung abbrechen, statt sie zu verschlechtern. Wirkt nach Übernehmen.
settings-radio-wired-link = {"  "}(Kabelverbindung)
settings-radio-runs-server-s-dongle-manual = Läuft auf dem Stick des Servers. Manuell ist die Einstellung für Messung und digitale Schwachsignal-Betriebsarten.
settings-radio-applies-immediately-no-reconnect-sent = Wirkt sofort — ohne erneutes Verbinden. Wird in Zehnteln eines dB gesendet und vom Server auf eine Stufe seines Tuners eingerastet; das Protokoll hat keine Antworten, daher lässt sich nicht lesen, worauf er sich eingestellt hat, und dieser Regler zeigt weiter, was verlangt wurde. Wird ignoriert, solange die Tuner-AGC läuft.
settings-radio-crystal-error-server-s-dongle =
    Quarzfehler des *Stick des Servers*, in Teilen pro Million — eine Eigenschaft dieser Hardware, daher wird er hier und nicht an den Sticks dieses Rechners eingestellt. Wirkt sofort.

    Der gemessene Takt-Fehler, den die USB-Schnittstelle ausgibt, ist hier nicht verfügbar: Über ein Netzwerk misst das die Pufferung, nicht den Quarz, und es ist um Tausende ppm falsch. Den Stick einmal über USB kalibrieren und die Zahl übernehmen, oder eine Rundfunkstation bekannter Frequenz abstimmen und nachregeln, bis sie auf der Skala sitzt.
settings-radio-tuner-starts-24-mhz-below =
    Der Tuner beginnt bei 24 MHz; darunter braucht die Gegenseite Hilfe. Ein Blog V4 setzt von selbst aufwärts um, und Automatisch lässt es in Ruhe — das ist das Einzige, was es tun kann, da das Protokoll nur den Tuner-Chip meldet und nichts sonst, und ein V4 ist über die Leitung nicht von einem gewöhnlichen R828D zu unterscheiden.

    Bei einem V3 oder einem anderen Stick schaltet Automatisch den Server unterhalb der eigenen Untergrenze des Tuners von 24 MHz auf Direktabtastung — das deckt jedes KW-Band ab, mit 17 m und 15 m, die in der zweiten Nyquist-Zone des ADC ankommen, und 28,8 MHz minus der Skala darüber gefaltet. Direktabtastung ausdrücklich wählen für einen gewöhnlichen R828D, der auf KW nichts hört. Ein Umschalten unterbricht den Strom kurz.
settings-radio-removes-dongle-s-dc-spike =
    Entfernt hier den DC-Spike des Sticks und das Spiegelbild, aus den Abtastwerten, sobald sie eintreffen — es sind Artefakte der Hardware, daher wandern sie mit allem anderen über das Netzwerk. Wirkt sofort.

    Ein AM-Träger, der genau auf der Skala abgestimmt ist, liegt ebenfalls bei DC, daher fällt er mit dem Spike weg: einen Kilohertz daneben abstimmen oder dies ausschalten.
settings-radio-powers-preamp-far-end-s = Speist einen Vorverstärker vom Stick der Gegenseite. Ältere Server implementieren den Befehl nicht und ignorieren ihn stillschweigend — das Protokoll kennt keine Art, Nein zu sagen —, daher ist ein Bias-Tee, der nicht einschaltet, nicht unbedingt Schuld dieser Seite.
settings-radio-feed-4-5-v-dc-2 = ca. 4,5 V Gleichspannung über das entfernte Koaxkabel speisen
settings-radio-bias-tee-hardware-somewhere-else = Bias-Tee ist EIN, auf einer Hardware, die anderswo steht. Was am anderen Ende dieser Speiseleitung hängt — ein Transceiver, eine geerdete Antenne, ein bereits gespeister Vorverstärker —, steht nicht vor Ihnen, um es zu prüfen.
settings-radio-sdrplay-server-rsp-tcp = SDRplay-Server (rsp_tcp)
settings-radio-sdrplay-server-greets-exactly-like = Ein SDRplay-Server meldet sich genau wie ein Stick, daher werden diese immer angezeigt statt nur, wenn einer erkannt wird. Gegenüber einem gewöhnlichen rtl_tcp-Server werden sie einfach ignoriert — das Protokoll hat keine Antworten und verwirft Befehle, die es nicht kennt. Der Reiter Gerät benennt den Server als rsp_tcp, sobald er sich selbst ausgewiesen hat.
settings-radio-input = Eingang A
settings-radio-input-b = Eingang B
settings-radio-hi-z = Hi-Z
settings-radio-lna-state = LNA-Stufe
settings-radio-step-index-not-db-figure = Ein Stufenindex, keine dB-Angabe: Was jede Stufe wert ist, hängt vom RSP-Modell und vom Band ab. 0 ist die meiste Verstärkung.
settings-radio-if-gain-reduction = ZF-Verstärkungsreduktion
settings-radio-reduction-so-more-less-signal = Eine Reduktion, daher bedeutet mehr weniger Signal. Wird nur befolgt, wenn die AGC des RSP aus ist.
settings-radio-enable = Aktivieren
settings-radio-dbfs = {" "}dBfs
settings-radio-level-rsp-s-agc-aims = Der Pegel, den die AGC des RSP zu halten anstrebt.
settings-radio-notches = Kerben
settings-radio-am = AM
settings-radio-fm = FM
settings-radio-dab = DAB
settings-radio-rf = HF
settings-radio-reference-out = Referenzausgang
settings-radio-t-24-mhz-clock-out = 24-MHz-Takt aus
settings-radio-rsp2-rspduo-only = Nur RSP2 und RSPduo.
settings-radio-receive-only-address-sample-rate = Nur Empfang. Adresse und Abtastrate wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern. Eine abgebrochene Verbindung wird von selbst wiederholt, daher kommt ein neu gestarteter Server ohne Eingriff hier zurück.
settings-radio-where-spyserver-listening-address-address =
    Wo spyserver lauscht: eine Adresse oder eine Adresse und ein Port. Der Port ist standardmäßig 5555, dem eigenen Standard von spyserver.

    Auf der Gegenseite prüfen, ob seine Konfigurationsdatei eine Adresse bindet, die andere Rechner erreichen können — gebunden an 127.0.0.1 nimmt es nur Verbindungen von demselben Rechner an.

    Wirkt nach Übernehmen.
settings-radio-host-host-port-e-g-2 = Host oder Host:Port, z. B. raspberrypi.local:5555
settings-radio-i-q-bandwidth = I/Q-Bandbreite
settings-radio-how-wide-slice-band-arrives =
    Wie breit ein Ausschnitt des Bandes als I/Q ankommt — und damit wie viel der Verbindung dies beansprucht. Das ist das *demodulierte* Fenster: Es folgt der Skala, und alles, was breiter ist, ist die FFT des Servers im Streifen über dem Panorama.

    Automatisch zielt auf etwa 96 kHz, was alle Betriebsarten hier trägt, einschließlich breitem FM, und noch in eine Mobilfunk-Uplink passt. Wirkt nach Übernehmen.
settings-radio-which-stage-server-s-own = Welche Stufe der eigenen Raten-Leiter des Servers angefragt wird. Jeder Empfänger hat eine andere Leiter — es ist seine Höchstrate, stufenweise halbiert —, daher wird dies als Stufe statt als Hertz-Wert gespeichert, und dieselbe Einstellung bedeutet auf einen anderen Server gerichtet noch etwas Vernünftiges.

    Automatisch zielt auf etwa 1 Msps. „Verbindung testen“ drücken, um zu sehen, was die Stufen auf diesem Server ergeben. Wirkt nach Übernehmen.
settings-radio-automatic = Automatisch
settings-radio-sample-format = Abtastformat
settings-radio-bits-per-component-wire-so =
    Bits je Komponente auf der Leitung, und damit was es auf der Verbindung kostet: 16 Bit ist das Doppelte von 8 Bit, und 32-Bit-Gleitkomma das Vierfache, für nicht mehr Information, als der ADC des Empfängers hatte.

    8 Bit ist es, was einen entfernten Empfänger über eine heimische Leitung zum Laufen bringt, und es ist für fast alles richtig. Ein Server darf dies übersteuern — manche sind so konfiguriert, dass sie auf einem Format bestehen — und meldet das im Protokoll, wenn er es tut. Wirkt nach Übernehmen.
settings-radio-gain = Verstärkung
settings-radio-server-s-gain-stage-index =
    Die Verstärkungsstufe des Servers, als Index — nicht als Anzahl Dezibel. Was jeder Index bedeutet, gehört dem Empfänger auf der Gegenseite und ändert sich mit dem Band, und das Protokoll sagt es nie, daher kann hier nichts davon in dB umrechnen, ohne eine Zahl zu erfinden.

    Der tatsächliche Bereich gehört dem Server und ist erst nach dem Verbinden bekannt; ein Index darüber wird begrenzt. Wirkt sofort — ohne erneutes Verbinden.
settings-radio-index = Index{" "}
settings-radio-digital-gain = Digitale Verstärkung
settings-radio-how-far-server-scales-samples =
    Wie weit der Server seine Abtastwerte hochskaliert, bevor er sie für die Leitung quantisiert. Automatisch beobachtet, was tatsächlich ankommt, und hält den Spitzenwert bei der Hälfte der Vollaussteuerung — das ist das Einzige, was sowohl auf einem toten als auch auf einem belegten Band richtig sein kann: Die beiden liegen dreißig Dezibel auseinander, und keine feste Zahl passt zu beiden.

    Es zählt nur bei 8 Bit, wo ein Signal weit unter Vollaussteuerung seine niederen Bits an den Quantisierer verliert und eines darüber flach abgeschnitten wird. Breitere Formate haben Platz und bekommen die Zahl, die jeder andere Client errechnet, aus dem Verstärkungsindex und der Dezimierungsstufe. Wirkt sofort.
settings-radio-full-band-strip = Streifen für das ganze Band
settings-radio-server-s-own-fft-whole =
    Die eigene FFT des Servers über das ganze Band, im Streifen über dem Panorama gezeichnet. In dieser Oberfläche ist es die einzige Bandansicht, die es gibt — das Panorama selbst ist nur so breit wie das empfangene I/Q.

    Ausschalten lässt einen Empfänger ohne Möglichkeit, etwas zu sehen, auf das er nicht schon abgestimmt ist. Wirkt sofort, mit einer kurzen Pause im Audio, während der Server wechselt, was er sendet.
settings-radio-ask-server-low-rate-fft =
    Den Server bitten, zusätzlich zum I/Q eine FFT des ganzen Bandes mit niedriger Rate zu senden, und sie im Streifen über dem Panorama zu zeichnen.

    Sie kostet fast nichts — ein paar Kilobyte je Rahmen, ein Dutzend Mal oder so pro Sekunde —, und sie zeigt den ganzen Empfänger statt des Ausschnitts, der demoduliert wird. Nur auf einer Verbindung ausschalten, auf der jedes Byte zählt. Wirkt sofort, mit einer kurzen Pause im Audio.
settings-radio-show = Anzeigen
settings-radio-whole-band = ganzes Band
settings-radio-how-much-receiver-strip-covers = Wie viel des Empfängers der Streifen abdeckt. Das ganze Band ist die breiteste Ansicht, die es gibt; verengt man sie, kommen dieselben Bins auf weniger Spektrum, was feinere Details auf einem kleineren Abschnitt ergibt.
settings-radio-strip-db-window = dB-Fenster des Streifens
settings-radio-server-quantises-fft-into-window =
    Der Server quantisiert seine FFT vor dem Senden in dieses Fenster, ein Byte je Bin — daher bestimmen diese Werte, wie fein der Streifen gemessen wird, nicht nur, wie er gezeichnet wird. Der Boden und die Decke, mit denen der Streifen *angezeigt* wird, setzt die eigene Automatik-Pegelung der Maschine, und das ist eine andere Sache.

    Der Standard von 150 dB ist der ganze Bereich des Protokolls und braucht keine Aufmerksamkeit. Ihn um den Rauschboden einzuengen erkauft Auflösung auf einem Empfänger, dessen Signale alle in einem kleinen Teil der Skala liegen. Wirkt sofort.
settings-radio-top = oben{" "}
settings-radio-range-2 = Bereich{" "}
settings-radio-i-q-correction = I/Q-Korrektur
settings-radio-remove-dc-spike-mirror-image = DC-Spike und Spiegelbild im DSP entfernen, auf dieser Seite. Ob der Empfänger auf der Gegenseite es braucht, hängt davon ab, was er ist — ein Airspy HF+ braucht es nicht, ein RTL-SDR schon —, und das Protokoll sagt nicht, mit welchem er spricht. Wirkt sofort.
settings-radio-enabled = Aktiviert
settings-radio-test-connection = Verbindung testen
settings-radio-connect-read-what-server-says = Verbinden, lesen, was der Server über seinen Empfänger sagt, und wieder trennen — ohne einen Strom zu starten, daher ist es sicher gegen einen Server zu drücken, den jemand anderes benutzt.
settings-radio-receive-only-panadapter-narrow-i = Nur Empfang. Das Panorama ist das schmale I/Q-Fenster, das der Skala folgt; die Bandansicht ist die FFT des Servers im Streifen darüber. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-receive-only-wideband-i-q = Nur Empfang. Breitband-I/Q, wie bei jedem lokalen SDR. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-receiver-address = Empfängeradresse
settings-radio-where-receiver-serves-web-page =
    Wo der Empfänger seine Webseite bereitstellt: eine Adresse oder eine Adresse und ein Port. Der Port ist standardmäßig 8073, dem eigenen Port eines KiwiSDR.

    Beachten Sie, dass ein Empfänger, der über den Proxy des Projekts erreicht wird (etwas.proxy.kiwisdr.com — fast die Hälfte der öffentlichen), stattdessen auf Port 80 antwortet, daher ausdrücklich angeben. Einen Empfänger aus „Öffentliche SDRs“ zu wählen, füllt das richtig aus.

    Wirkt nach Übernehmen.
settings-radio-host-host-port-e-g-3 = Host oder Host:Port, z. B. kiwi.example.org:8073
settings-radio-password = Passwort
settings-radio-receiver-s-user-password-where =
    Das *Benutzer*-Passwort des Empfängers, sofern sein Betreiber eines gesetzt hat. Bei fast jedem öffentlichen Empfänger leer. Nicht das Admin-Passwort.

    Wirkt nach Übernehmen.
settings-radio-announce = Anmelden als
settings-radio-name-connection-shows-up-under =
    Der Name, unter dem diese Verbindung erscheint und den der Eigentümer des Empfängers und alle anderen Zuhörer sehen können.

    Leer gelassen ist es Ihr Stationsrufzeichen, was die Konvention des Netzes ist — manche Betreiber blockieren Clients, die sich nicht ausweisen. Wirkt nach Übernehmen.
settings-radio-blank-station-callsign = leer = Stationsrufzeichen
settings-radio-band-view = Bandansicht
settings-radio-ask-receiver-own-waterfall-well =
    Den Empfänger bitten, zusätzlich zu seinem I/Q auch seinen eigenen Wasserfall zu senden, und ihn im Streifen über dem Panorama zu zeichnen.

    Das lohnt sich: Das I/Q ist nur etwa 12 kHz breit, daher gäbe es ohne dies nichts, woran man abstimmen könnte. Es ist eine zweite Verbindung mit etwa 20 kB/s bei voller Geschwindigkeit, gegenüber 44 kB/s für das I/Q.

    Wirkt nach Übernehmen.
settings-radio-show-receiver-s-0-30 = den Wasserfall 0–30 MHz des Empfängers anzeigen
settings-radio-band-view-speed = Geschwindigkeit der Bandansicht
settings-radio-how-often-receiver-sends-waterfall =
    Wie oft der Empfänger eine Wasserfallzeile sendet, 1 (langsamste) bis 4. Die einzige Einstellung hier, die während des Betriebs ändert, was die Verbindung kostet — daher auf einer nach Volumen abgerechneten Verbindung herunterdrehen.

    Wirkt sofort.
settings-radio-band-view-span = Bereich der Bandansicht
settings-radio-how-much-band-receiver-s =
    Wie viel Band der Wasserfall des Empfängers abdeckt. Ganz links am Regler sein volles 0–30 MHz, mit jedem Schritt nach rechts halbiert.

    Der Wasserfall ist immer 1024 Bins breit, egal wie breit das Fenster ist, daher ist dies wirklich ein Auflösungsregler: Das ganze Band ist 29 kHz je Bin, was eine Bandkarte statt eines Bildes von etwas ist, während einige hundert Kilohertz einzelne Stationen in einem Rundfunkband zeigen. Herangezoomt folgt das Fenster Ihrer Skala.

    Das ganze Band ist der Standard und macht den Streifen zu etwas, woran man abstimmen kann; das Panorama darunter ist die etwa 12 kHz des I/Q. Wirkt sofort.
settings-radio-receiver-agc = AGC des Empfängers
settings-radio-receiver-s-agc-which-far =
    Die AGC des *Empfängers*, auf der anderen Seite der Verbindung und vor dem I/Q — sie wirkt also, bevor irgendetwas, das sdroxide tut.

    Standardmäßig an, anders als bei jedem lokalen SDR hier, weil die gemessene Alternative schlechter war: Bei einem Live-Empfänger war die manuelle Verstärkung über ihren Bereich nicht monoton, und ihr oberes Ende übersteuerte das I/Q bei Vollaussteuerung, während die AGC bei etwa -24 dBFS mit 7 dB Reserve hielt.

    Es bedeutet allerdings, dass die Abtastamplitude kein Signalpegel ist, weshalb das S-Meter stattdessen aus dem eigenen Wert des Empfängers gelesen wird. Wirkt sofort.
settings-radio-let-receiver-ride-own-gain = den Empfänger seine eigene Verstärkung fahren lassen
settings-radio-manual-gain = Manuelle Verstärkung
settings-radio-fixed-gain-when-receiver-s = Feste Verstärkung, wenn die AGC des Empfängers aus ist, auf seiner eigenen Skala 0–90. Keine Dezibel von irgendetwas Angegebenem — das Protokoll nennt sie manGain und sagt nicht mehr, und sie wurde als nicht monoton gemessen, daher als Regler behandeln, an dem man eine gute Stelle findet, nicht als Kalibrierung. Wirkt sofort.
settings-radio-read-receiver-s-status-page = Die Statusseite des Empfängers lesen: was er ist, was er abdeckt, wie viele Kanäle frei sind und ob sein Betreiber Verbindungen von anderen Apps als einem Browser erlaubt. Belegt keinen seiner Kanäle.
settings-radio-host-port-e-g-127 = Host:Port, z. B. 127.0.0.1:50001
settings-radio-iq-sample-rate = IQ-Abtastrate
settings-radio-receiver = Empfänger
settings-radio-rig-two-receivers-sunsdr2dx-can = Ein Gerät mit zwei Empfängern (SunSDR2DX) kann über eine Verbindung zwei Funkgerät-Reiter bedienen — dieses Funkgerät auf RX1 und ein anderes auf RX2. Der Sender gehört dem RX1-Funkgerät.
settings-radio-stream-delay = Stromverzögerung
settings-radio-how-long-rig-s-iq = Wie lange das IQ des Geräts braucht, um auf einer neuen Mitte anzukommen, nachdem sdroxide es verschoben hat. Es ist die eigene DSP-Kette des Geräts, nicht das Netzwerk — ein Server auf diesem Rechner hat auch eine —, und während das Panorama ganz herausgezoomt gezogen wird, verschiebt ein Fehler in dieser Höhe die neuesten Wasserfallzeilen seitlich zur Historie, um den Fehler mal der Geschwindigkeit des Ziehens. Zu hoch verschiebt sie genau so weit wie zu niedrig, nur in die andere Richtung. Messen mit `cargo run --release -p sdroxide-tci --example retune_latency`; der Standard ist eine SunSDR2DX auf ExpertSDR3 bei 192 kHz.
settings-radio-wideband-iq-receive-audio-transmit = Breitband-IQ-Empfang, Audio-Senden. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-radio-address = Funkgerät-Adresse
settings-radio-address-shown-radio-under-set = Die Adresse, die am Funkgerät unter SET > Network angezeigt wird. Network Control muss dort eingeschaltet sein, und am Funkgerät müssen ein Netzwerk-Benutzername und ein Passwort gesetzt sein.
settings-radio-host-ip-e-g-192 = Host oder IP, z. B. 192.168.1.50
settings-radio-control-port = Steuerport
settings-radio-t-50001-unless-has-been-changed = 50001, sofern es am Funkgerät nicht geändert wurde.
settings-radio-network-user = Netzwerk-Benutzer
settings-radio-stored-clear-radio-json-protocol = Unverschlüsselt in radio.json gespeichert. Das Protokoll verschleiert es auf der Leitung umkehrbar, daher würde hier nichts es zu einem Geheimnis machen.
settings-radio-receive = Empfang von
settings-radio-af-radio-demodulates-sdroxide-shows = AF: Das Funkgerät demoduliert, und sdroxide zeigt das Audioband. 12-kHz-ZF: Das Funkgerät sendet stattdessen seine DRM-Zwischenfrequenz, und sdroxide demoduliert, was seine eigenen Filter, Rauschunterdrückung und Decoder über etwa ±12 kHz zur Wirkung bringt. In beiden Fällen ist der breite Wasserfall das eigene Scope des Funkgeräts — kein Icom gibt I/Q aus. Eine ZF-Vorgabe nutzt vorübergehend AF für breitere Betriebsarten wie WFM und stellt dann für kompatible Betriebsarten die ZF wieder her. Eine AF-Vorgabe bleibt AF. HD Radio ist auf dieser Verbindung nicht verfügbar.
settings-radio-audio-sample-rate = Audio-Abtastrate
settings-radio-t-12-khz-if-needs-48000 = Eine 12-kHz-ZF braucht den 48000-Hz-Strom — bei dieser Rate wird stattdessen das demodulierte Audio des Funkgeräts genutzt.
settings-radio-if-spectrum = ZF-Spektrum
settings-radio-which-way-round-radio-s = Wie herum die 12-kHz-ZF des Funkgeräts läuft. „Automatisch“ nutzt, was das Modell bekanntermaßen tut — gespiegelt bei einem IC-7760, normal bei jedem anderen Icom. Von Hand setzen, wenn SSB auf dem falschen Seitenband herauskommt: Das verrät sich dadurch, dass man USB wählen muss, wo das Band LSB fährt, während die eigene Betriebsartenanzeige des Funkgeräts durchgehend mit sdroxide übereinstimmt.
settings-radio-normal = Normal
settings-radio-mirrored = Gespiegelt
settings-radio-displayed-bandwidth = Angezeigte Bandbreite
settings-radio-width-audio-band-panadapter-cat = Breite des Panoramas im Audioband, wie bei einem CAT-Gerät. Wird in den digitalen Betriebsarten und dort genutzt, wo das Scope des Funkgeräts aus ist; sonst ist das Panorama das Scope, und der Bereich des Scope legt seine Breite fest.
settings-radio-how-cw-panel-s-keyer-2 = Wie der Keyer des CW-Panels sendet. „Geräte-Keyer“ stellt das Funkgerät auf CW und gibt ihm den zu sendenden Text an seinen eigenen Keyer (CI-V 0x17). „Soundkarte“ sendet den getasteten Mithörton stattdessen über das LAN als Audio (MCW), ein Ton auf Skala + Tonhöhe — und weil ein Gerät im CW-Modus seinen Modulationseingang ignorieren würde, bleibt das Funkgerät bei der Wahl von CW im einfachen USB, derselben Betriebsart, auf der die digitalen Betriebsarten laufen.
settings-radio-how-much-audio-radio-holds = Wie viel Audio das Funkgerät hält, bevor es moduliert. Mehr übersteht ein schlechteres Netzwerk, auf Kosten der Sende-Latenz.
settings-radio-streams-radio-s-own-475 = Streamt den eigenen 475-Bin-Durchlauf des Funkgeräts. Auf AF ist es das Panorama — das Audio, das das Funkgerät sendet, ist, was durch seinen Filter kam, kein Bild des Bandes —, und auf der 12-kHz-ZF ist es der Wasserfall über das ganze Band darüber. In beiden Fällen ist es das Bild des Funkgeräts, nicht die DSP von sdroxide: Es gibt kein I/Q, aus dem man eines errechnen könnte.
settings-radio-how-wide-sweep-only-wide = Wie breit es abgetastet wird. Dies ist die einzige breite Ansicht, die ein Icom hat, und das Funkgerät behält, was zuletzt auf seinem eigenen Bildschirm gewählt wurde — oft ein paar kHz, weshalb der Streifen nicht breiter als das Panorama darunter erscheinen kann. Ein Bereich, der hier gesetzt wird, schaltet das Scope außerdem in den Mittenmodus, sodass es der Skala folgt. Es ändert auch die eigene Anzeige des Funkgeräts; „Wie am Funkgerät eingestellt“ lässt sie unberührt.
settings-radio-switch-modulation-input-lan = Modulationseingang auf LAN umschalten
settings-radio-transmit-audio-only-heard-when = Das Sendeaudio ist nur zu hören, wenn der MOD-Eingang des Funkgeräts auf LAN steht. sdroxide kann das bei einem Modell schreiben, dessen Menünummerierung es kennt; bei jedem anderen sagt es das und lässt das Menü in Ruhe. Es ist eine Leihgabe: Was das Funkgerät dort vorher hatte, wird zuerst gelesen und beim Ende der Sitzung zurückgeschrieben, daher hört ein Gerät, das danach allein benutzt wird, weiterhin sein eigenes Mikrofon.
settings-radio-copy-diagnostic-report = Diagnosebericht kopieren
settings-radio-copies-radio-s-last-session = Kopiert die letzte Sitzung dieses Funkgeräts — seinen Verbindungsaufbau und seine CI-V-Aufzeichnung — für einen Fehlerbericht in die Zwischenablage. Die dieses Funkgeräts, nicht die der Station: Bei zwei Icoms im LAN antwortet jeder Reiter zu seiner eigenen Adresse.
settings-radio-control-audio-radio-s-own = Steuerung, Audio und das eigene Scope des Funkgeräts über eine Netzwerkverbindung — kein serielles Kabel und keine Soundkarte. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-testing = Prüfe…
settings-radio-was-only-check-press-apply = Das war nur eine Prüfung — „Übernehmen / erneut verbinden“ unten drücken, um dieses Funkgerät zu benutzen.
settings-radio-radios = Funkgeräte
settings-radio-asks-network-iio-devices-also = Fragt das Netzwerk nach IIO-Geräten und versucht außerdem 192.168.2.1 direkt — ein Pluto am Ende eines USB-Kabels ist per Multicast oft nicht erreichbar, obwohl die Adresse funktioniert.
settings-radio-no-radios-press-discover = keine Funkgeräte — Suchen drücken
settings-radio-address = Adresse
settings-radio-overrides-selection-above-usb-cable = Überschreibt die Auswahl oben. Das USB-Kabel stellt den Pluto als Netzwerkadapter dar, nicht als seriellen Port, daher ist dies eine IP-Adresse, auch wenn das Funkgerät auf Ihrem Schreibtisch steht.
settings-radio-pluto-revision-c-pluto-unlocked = Ein Pluto+ oder ein Pluto der Revision C, auf 2R2T freigeschaltet, kann zwei Funkgerät-Reiter von einem Gerät bedienen — dieses Funkgerät auf RX1 und ein anderes auf RX2, jeweils an seiner eigenen Antenne. Die beiden Kanäle teilen sich einen Oszillator, daher verschiebt die Neuabstimmung eines Funkgeräts beide; was RX2 bringt, ist eine zweite Antenne auf demselben Spektrum (Diversität), nicht ein zweites Band. Der Sender gehört dem RX1-Funkgerät. Ein serienmäßiger Pluto 1R1T verweigert RX2 beim Verbinden.
settings-radio-width-spectrum-sdroxide-receives-ad9361 = Breite des Spektrums, das sdroxide empfängt. Der AD9361 erreicht 61,44 Msps; die USB-Netzwerkverbindung nicht, worauf diese Liste abgestimmt ist. Die niedrigsten Raten brauchen eine Filterkonfiguration im AD9361, die sdroxide nicht lädt — ein serienmäßiger Pluto läuft sie stattdessen mit etwa 2,084 Msps und sagt das beim Verbinden. Wirkt nach Übernehmen.
settings-radio-more-than-usb-2-will = {"  "}(mehr, als USB 2 trägt)
settings-radio-stock-pluto-runs-2-084 = {"  "}(ein serienmäßiger Pluto läuft mit 2,084)
settings-radio-analog-filter = Analogfilter
settings-radio-ad9361-s-baseband-filter-leave = Das Basisbandfilter des AD9361. Auf 0 lassen für Automatik, die es auf neun Zehntel der Abtastrate öffnet — absichtlich breit, weil der Empfänger seinen Oszillator um ein Viertel einer Spanne von der Skala wegsetzt, um Signale vom DC-Spike freizuhalten, und ein enges Filter genau den Teil abschneiden würde, in den er sie verschoben hat. Wirkt nach Übernehmen.
settings-radio-ad9361-has-four-modes-not = Der AD9361 hat vier Modi, keinen Ein-/Aus-Schalter. Langsamer Angriff passt zu SSB und CW; schneller Angriff zu stoßweisen Signalen; manuell ist die Einstellung für Messung und digitale Schwachsignal-Betriebsarten. Wirkt sofort.
settings-radio-rx-gain = RX-Verstärkung
settings-radio-applies-immediately-no-reconnect-ignored = Wirkt sofort — ohne erneutes Verbinden. Wird ignoriert, sofern die AGC nicht auf manuell steht; das ist das eigene Verhalten des AD9361, nicht das von sdroxide.
settings-radio-tx-gain = Sendeverstärkung
settings-radio-negative-because-ad9361-expresses-transmit = Negativ, weil der AD9361 den Sendepegel als Dämpfung ausdrückt: 0 dB ist volle Leistung. Wirkt sofort. Der Sender wird auf seinen leisesten Wert gestellt, bevor dieser Wert beim Verbinden angewendet wird, damit nichts, was das vorherige Programm hinterlassen hat, aktiv ist.
settings-radio-reference-error-parts-per-million = Referenzfehler in Teilen pro Million. Mit RUST_LOG=sdroxide_pluto=debug starten, dann gibt das Protokoll nach etwa 20 Sekunden den gemessenen Takt-Fehler aus — das ist die Zahl, die man eingibt. Wird von sdroxide angewendet, nicht in den eigenen dauerhaften Trimm des Funkgeräts geschrieben. Wirkt sofort.
settings-radio-full-duplex = Vollduplex
settings-radio-keep-receiving-while-transmitting = Während des Sendens weiter empfangen
settings-radio-leave-off-pluto-reached-over =
    Dies bei einem Pluto aus lassen, der über sein USB-Kabel erreicht wird. Diese Verbindung kann nicht gleichzeitig in beide Richtungen eine Megasample pro Sekunde tragen, und eine Aussendung, die den Sendepuffer aushungert, geht zerhackt hinaus — daher hält der Empfang standardmäßig für die Dauer einer Aussendung an.

    Einschalten für eine Karte an echtem Ethernet (ein LibreSDR, ein Pluto an einem Gigabit-Adapter), wo Platz für beides ist: Dann hören Sie den Empfänger während Ihrer eigenen Aussendung, so wie eine QO-100-Station ihren eigenen Downlink mithört.

    Eine Aussendung braucht das Doppelte der Abtastrate an Verbindung — 2,5 Msps sind 10 MB/s in jede Richtung —, daher die Rate senken, wenn das Protokoll meldet, dass die Verbindung sie nicht trägt. Das Panorama zeigt während einer Aussendung weiter das gesendete Signal; es ist das Audio, das weiterläuft.

    Wirkt nach Übernehmen.
settings-radio-duplex = Duplex
settings-radio-whether-ad9361-runs-both-directions =
    Ob der AD9361 in beide Richtungen gleichzeitig läuft (FDD, so startet ein Pluto und so hat sdroxide ihn immer gelassen) oder nacheinander (TDD).

    Dies auf FDD lassen, sofern Sie nicht die PTT-Pins unten wollen — TDD ist das, womit diese tasten, und es schließt Vollduplex oben aus. Wirkt nach Übernehmen.
settings-radio-ptt-pins = PTT-Pins
settings-radio-pluto-s-four-gpo-test =
    Die vier GPO-Testpunkte des Pluto können einen externen Leistungsverstärker, LNA oder Sende-/Empfangsschalter allein tasten. Wählen Sie ein Paar, und ein Pin ist die ganze Zeit high, während das Gerät empfängt, der andere die ganze Zeit, während es sendet — ohne Host-Software im Spiel und ohne serielle PTT-Leitung zu verdrahten.

    Dies setzt das Funkgerät auf TDD, was auch immer die Zeile Duplex sagt, weil die Pins den Enable-Leitungen des AD9361 folgen, und in FDD sind beide die ganze Sitzung gesetzt. Vollduplex entfällt damit, daher beide auf den Standardwerten lassen, wenn Sie Satelliten arbeiten und Ihren eigenen Downlink hören müssen.

    Analog Devices' eigene Anmerkung legt einen externen LNA an GPO0/GPO1, daher GPO2/GPO3 nutzen, wenn Ihre Karte so verdrahtet ist. Die Pins liegen bei etwa 1,3 V mit wenigen Milliampere: Mit ihnen einen Transistor oder einen Optokoppler ansteuern, niemals eine Relaisspule.

    Wirkt nach Übernehmen.
settings-radio-buffer-size = Puffergröße
settings-radio-how-much-device-holds-before =
    Wie viel das Gerät vor jeder Übertragung hält, in komplexen Abtastwerten. Der Standard von 32768 sind etwa 16 ms bei 2 Msps: lang genug, dass der Umlauf je Übertragung nicht der Engpass ist, kurz genug, dass eine Neuabstimmung nicht sichtbar verzögert ist.

    Halbieren, wenn das Protokoll meldet, dass der Empfangssocket ersetzt wird. Eine grenzwertige Verbindung — ein Pluto, der über sein USB-Kabel mit hoher Abtastrate erreicht wird, ist der übliche Fall — stockt mitten in einer Übertragung, und eine kleinere Übertragung wird seltener mitten im Flug erwischt und ist danach schneller wiederhergestellt. Erhöhen, um Neuabstimmungs-Latenz gegen weniger Umläufe zu tauschen.

    Wirkt nach Übernehmen.
settings-radio-samples = {" "}Abtastwerte
settings-radio-rx-tx-port = RX-/TX-Port
settings-radio-ad9361-s-rf-port-select = Der rf_port_select des AD9361. Ein serienmäßiger Pluto verdrahtet je eines, daher diese leer lassen, sofern Sie keine Karte haben, die das nicht tut. Wirkt nach Übernehmen.
settings-radio-opens-radio-reads-what-says = Öffnet das Funkgerät, liest, was es über sich selbst sagt, und meldet den Abstimmbereich, den genau diese Karte hat. Startet keinen Strom.
settings-radio-copies-last-session-s-protocol = Kopiert die Protokollaufzeichnung der letzten Sitzung für einen Fehlerbericht in die Zwischenablage.
settings-radio-wideband-iq-receive-transmit-half = Breitband-IQ-Empfang und -Senden. Halbduplex — der Empfang hält für die Dauer einer Aussendung an, weil die USB-Netzwerkverbindung nicht beides gleichzeitig trägt. Kein SoapySDR und keine libiio nötig.
settings-radio-not-yet-verified-against-real = Noch nicht an echter Hardware geprüft. Wenn es sich falsch verhält, senden Sie bitte den Diagnosebericht — er enthält jeden Befehl, der mit dem Funkgerät ausgetauscht wurde, und die ersten Bytes des Abtaststroms.
settings-radio-flexradio-announces-itself-local-network = Ein FlexRadio meldet sich etwa einmal pro Sekunde im lokalen Netz. Ein Funkgerät, das über einen Router oder ein VPN erreicht wird, sendet Ihnen nie etwas — geben Sie dann unten seine Adresse ein.
settings-radio-another-gui-client-has-radio = Ein anderer GUI-Client hat dieses Funkgerät, und multiFLEX ist ausgeschaltet. Trennen Sie diesen Client oder schalten Sie multiFLEX am Funkgerät ein.
settings-radio-overrides-selection-above-use-radio = Überschreibt die Auswahl oben. Dies nutzen für ein Funkgerät in einem anderen Subnetz, hinter einem VPN oder auf einem nicht standardmäßigen Port.
settings-radio-width-spectrum-sdroxide-receives-192 = Breite des Spektrums, das sdroxide empfängt. 192 kHz ist das Maximum des Funkgeräts für einen DAX-IQ-Strom und damit der breiteste Bereich, den diese Schnittstelle zeigen kann.
settings-radio-t-192-khz-widest-dax-iq = 192 kHz ist der breiteste DAX-IQ-Strom, den ein FLEX sendet, und das Funkgerät bindet den Bereich des Panoramas daran — das ist also der ganze Bereich, und keine Einstellung hier macht ihn breiter.
settings-radio-dax-iq-channel = DAX-IQ-Kanal
settings-radio-radio-has-four-change-only = Das Funkgerät hat vier. Nur ändern, wenn schon etwas anderes im Netz Kanal 1 belegt — das Funkgerät lehnt einen Kanal doppelt ab.
settings-radio-station-name = Stationsname
settings-radio-shown-against-session-radio-s = Wird in der Client-Liste des Funkgeräts neben dieser Sitzung angezeigt. Das Funkgerät merkt sich einen Client auch darüber, daher behandelt es sdroxide bei einer Umbenennung als neuen.
settings-radio-gui-client-id = GUI-Client-ID
settings-radio-identity-radio-restores-session-s = Die Identität, auf die das Funkgerät die Slices und Panoramen dieser Sitzung wiederherstellt. Leer abgeleitet vom Stationsnamen — stabil, aber auf jedem sdroxide gleich, der den Standardnamen behalten hat, und ein Funkgerät klärt ein Duplikat, indem es den früheren Client hinauswirft. sdroxide erkennt das auf der Leitung und fällt auf eine Ein-Sitzungs-Identität zurück, was die Wiederherstellung kostet; eine eigene UUID hier eintragen, um sie zu behalten.
settings-radio-optional-e-g-uuid-your = optional, z. B. eine eigene UUID
settings-radio-mirror-radio-s-i-q =
    Das I/Q des Funkgeräts um die Mitte des Panoramas spiegeln. Standardmäßig aus, so wurde ein FLEX-6600 geprüft.

    Probieren, wenn Empfangsaudio auf USB *und* auf LSB unverständlich ist und nichts dekodiert, während der Wasserfall völlig überzeugend aussieht — so sieht ein gespiegelter Strom aus, und es ist der eine Fehler ohne anderes Symptom (Ticket #368). Ist es das nicht, macht das Einschalten es offensichtlich statt subtil. Wirkt nach Übernehmen / erneut verbinden.
settings-radio-network-mtu = Netzwerk-MTU
settings-radio-largest-datagram-radio-may-send = Das größte Datagramm, das das Funkgerät senden darf. 1450 ist das, was SmartSDR selbst verlangt. Auf einem Pfad mit kleinerer MTU senken — einem VPN oder einem Tunnel —, wo die Fragmente verworfen werden und überhaupt kein Spektrum ankommt.
settings-radio-b = {" "}B
settings-radio-checks-radio-answers-without-registering = Prüft, ob das Funkgerät antwortet, ohne sich als GUI-Client anzumelden — so stört es keine bereits laufende SmartSDR-Sitzung.
settings-radio-wideband-iq-receive-over-dax = Breitband-IQ-Empfang über DAX, Audio-Senden, das das Funkgerät moduliert. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-not-yet-verified-against-real-2 = Noch nicht an echter Hardware geprüft. Wenn es sich falsch verhält, senden Sie bitte den Diagnosebericht — er enthält jede Protokollzeile, die mit dem Funkgerät ausgetauscht wurde.
settings-radio-devices-soapysdr-can-see = Geräte, die SoapySDR sehen kann
settings-radio-waiting-machine-radio-attached-where = Warte auf den Rechner, an dem das Funkgerät angeschlossen ist, wo die Module installiert sind. Welches Gerät geöffnet wird, steht in `device_args` in der config.toml dieses Rechners oder in seinem --device.
settings-radio-ask-every-installed-soapysdr-module = Jedes installierte SoapySDR-Modul bitten, zu suchen. Es wird nichts geöffnet, daher ist es während des Empfangs sicher — es kann aber einen Moment dauern.
settings-radio-not-enumerated-yet-press-rescan = Noch nicht aufgezählt — Erneut suchen drücken.
settings-radio-no-soapysdr-devices-found-check = Keine SoapySDR-Geräte gefunden. Prüfen Sie, ob das Modul für Ihr Funkgerät installiert ist und ob Sie auf das Gerät zugreifen dürfen.
settings-radio-not-radio = kein Funkgerät
settings-radio-sound-card-listed-above-if = Eine Soundkarte ist oben aufgeführt, als wäre sie ein SDR. Sie akzeptiert jede Frequenz und ignoriert sie, daher zeigt das Öffnen den Eingang der Soundkarte statt des Bandes. sdroxide wählt diese nicht automatisch — aber eine device_args-Zeile, die eine nennt, wird trotzdem befolgt.
settings-radio-which-one-opens-set-device = Welches geöffnet wird, legt --device oder device_args in config.toml fest; ohne beides gewinnt das erste aufgelistete Funkgerät.
settings-radio-no-device = kein Gerät
settings-radio-rig-has-no-software-adjustable = Dieses Funkgerät hat keine per Software einstellbaren Verstärkungen.
settings-radio-rx-gains = RX-Verstärkungen
settings-radio-tx-gains = TX-Verstärkungen
settings-radio-antennas = Antennen
settings-radio-remembered-next-start = Für den nächsten Start gemerkt.
settings-radio-these-controls-appear-once-soapysdr = Diese Regler erscheinen, sobald ein SoapySDR-Gerät geöffnet ist — es sind die eigenen des Geräts, und man muss es fragen.
settings-radio-stream = Strom
settings-radio-follow-app-setting = Der App-Einstellung folgen
settings-radio-msps = oder in Msps
settings-radio-baseband-filter = Basisbandfilter
settings-radio-let-driver-choose = Den Treiber wählen lassen
settings-radio-changing-either-reopens-radio = Eine Änderung von beiden öffnet das Funkgerät erneut.
settings-radio-driver-publishes-no-settings-own = Dieser Treiber veröffentlicht keine eigenen Einstellungen.
settings-radio-applied-once-remembered-next-start = Sofort angewendet und für den nächsten Start gemerkt.
settings-radio-no-rx-888-found-press = Kein RX-888 gefunden — Erneut suchen drücken
settings-radio-adc-clock = ADC-Takt
settings-radio-t-129-6-msps-needs-superspeed = 129,6 Msps braucht eine SuperSpeed-Verbindung und einen schnellen Rechner; 64,8 ist der sichere Standard. Eine Änderung öffnet den Empfänger automatisch neu, was einen Moment dauert, aber keinen Neustart braucht.
settings-radio-panadapter-width = Breite des Panoramas
settings-radio-hf-only = {"  "}· nur KW
settings-radio-usable-both-front-ends = An beiden Frontstufen nutzbar.
settings-radio-below-vhf-crossover-ordinary-width = Unterhalb der UKW-Schwelle ist dies eine gewöhnliche Breite. Darüber kann die 8-MHz-ZF des Tuners in einem so breiten Fenster nicht zentriert werden, daher sitzt das Live-Spektrum seitlich am Panorama, mit leerem Spektrum daneben.
settings-radio-dial-above-crossover-now-so = Die Skala liegt jetzt über der Schwelle, daher zeigt diese Breite die 8-MHz-ZF des Tuners außermittig mit totem Spektrum daneben. Die Breite verengen, um das Panorama zu füllen.
settings-radio-vga-gain = VGA-Verstärkung
settings-radio-ad8370-variable-gain-amplifier-ahead = AD8370-Verstärker mit variabler Verstärkung vor dem ADC.
settings-radio-attenuator = Dämpfungsglied
settings-radio-pe4304-step-attenuator-0-5 = PE4304-Stufendämpfungsglied in 0,5-dB-Schritten.
settings-radio-adc-range = ADC-Bereich
settings-radio-wide-2-25-vp-p = Breit (2,25 Vs-s)
settings-radio-selects-adc-s-wider-input = Wählt den breiteren Eingangsbereich des ADC: mehr Reserve für starke Rundfunksignale, weniger Zählwerte für schwache. Aus wählt den empfindlicheren Bereich von 1,5 Vs-s.
settings-radio-dither = Dither
settings-radio-adds-small-dither-signal-ahead = Fügt vor dem ADC ein kleines Dither-Signal hinzu: kostet etwas Rauschboden, bringt aber einen störfreien Dynamikbereich.
settings-radio-randomiser = Randomisierer
settings-radio-adc-scrambles-output-so-digital = Der ADC verwürfelt seinen Ausgang, damit der digitale Bus nicht in die Frontstufe abstrahlt; der Treiber entwürfelt ihn. Dies eingeschaltet lassen, sofern Sie nicht Fehler suchen. Wirkt beim erneuten Verbinden.
settings-radio-dc-hf-antenna-port = Gleichspannung am KW-Antennenanschluss
settings-radio-powers-active-antenna-preamp-down = Speist eine aktive Antenne oder einen Vorverstärker über das Koaxkabel.
settings-radio-vhf-tuner-gain = UKW-Tuner-Verstärkung
settings-radio-r828d-rf-gain-used-above = HF-Verstärkung des R828D, genutzt oberhalb der automatischen KW/UKW-Schwelle. 29 diskrete Stufen; die nächste wird genutzt.
settings-radio-auto = Automatisch
settings-radio-let-tuner-run-own-lna = Den Tuner seine eigenen LNA- und Mischerschleifen laufen lassen.
settings-radio-bias-tee-vhf = Bias-Tee (UKW)
settings-radio-dc-vhf-antenna-port = Gleichspannung am UKW-Antennenanschluss
settings-radio-clock-trim = Takttrimm
settings-radio-corrects-reference-oscillator-applied-when = Korrigiert den Referenzoszillator. Wird angewendet, wenn Sie den Wert loslassen — ein Neuöffnen bei jedem Pixel eines Ziehens würde den Empfänger hunderte Male neu starten.
settings-radio-receive-only-below-adc-s = Nur Empfang. Unterhalb der Nyquist-Grenze des ADC wird die Antenne direkt abgetastet, und die Neuabstimmung ist sofort, weil es keinen Hardware-Abwärtsmischer gibt — der volle ADC-Strom wird auf dem Rechner ins Basisband umgesetzt. Darüber schaltet der Empfänger automatisch auf seinen R828D-Tuner und die UKW-SMA um, daher müssen beide Antennen angeschlossen sein. UKW braucht einen ADC-Takt von 32,4 Msps oder mehr, damit die ZF des Tuners passt, und bei Takten unter 48 Msps gibt es eine Lücke zwischen den beiden Bereichen, die nichts erreicht. Jede Einstellung hier wirkt sofort — es gibt keine Taste ÜBERNEHMEN.
settings-radio-device = Gerät
settings-radio-no-devices-press-rescan = keine Geräte — Erneut suchen drücken
settings-radio-six-rates-are-six-different =
    Die sechs Raten sind sechs verschiedene FPGA-Abbilder, kein Register — deshalb wählt nichts im Protokoll von ELAD zwischen ihnen, und was diese Einstellung bedeutet, hängt vom Modell ab.

    FDM-S1 / FDM-S2: ein Befehl. Ihr FPGA wird bei jedem Einschalten vom Computer geladen, und sdroxide führt den eigenen elad-firmware-Lader von ELAD aus, um das Abbild dieser Rate hineinzugeben. Ohne diesen Lader installiert sendet ein Sampler gar keine Abtastwerte, und sdroxide sagt das auf dem Bildschirm.

    FDM-DUO: kein Befehl. Das Funkgerät startet sein eigenes FPGA und landet bei der Rate, mit der es eingeschaltet wurde oder die FDM-SW2 zuletzt hinterlassen hat (192 kHz bei einem frischen), und dies sagt nur, wie der Strom GELESEN wird. Falsch eingestellt, ist das Panorama einfach die falsche Breite, mit jeder Frequenz darin passend skaliert.

    Die echte Rate des Stroms wird ein paar Sekunden nach dem Start gemessen, und eine Abweichung wird auf dem Bildschirm gemeldet. Wirkt nach Übernehmen.
settings-radio-fdm-s-fpga-image-loaded = FDM-S: das geladene FPGA-Abbild — FDM-DUO: wie der Strom gelesen wird
settings-radio-receiver-s-input-pad-one = Die Eingangsdämpfung des Empfängers — eine Stufe, hinein oder hinaus. Derselbe Regler wie der Schieberegler Verstärkung im Hauptfenster.
settings-radio-pre-selection-filters = Vorselektionsfilter
settings-radio-low-pass-bank-front-adc = Die Tiefpassbank vor dem ADC. Sie zu überbrücken gibt die breiteste Ansicht und das schlechteste Verhalten nahe starker Signale außerhalb des Bandes — ein Rundfunksender ein paar Meilen entfernt wirft Spiegelbilder über das Band. Eingeschaltet lassen, sofern Sie nicht bewusst außerhalb des gefilterten Bereichs hören.
settings-radio-filters-circuit = Filter im Einsatz
settings-radio-rig-control-fdm-duo-only = Gerätesteuerung — nur FDM-DUO
settings-radio-transceiver-s-cat-usb-port = Der CAT-USB-Port des Transceivers, der ein eigenes Kabel neben dem Empfangskabel hat. Den Port bei einem FDM-S1 oder FDM-S2 ungesetzt lassen, die keinen haben — und bei einem FDM-DUO wäre es besser, nur über dessen Empfangskabel zu steuern, wo es noch abgestimmt und getastet werden kann, aber kein Messgerät abgelesen werden kann.
settings-radio-port-set-radio-s-display = Ist der Port gesetzt, sind die Anzeige des Funkgeräts und diese Skala in beide Richtungen dieselbe Zahl: Das Empfangsfenster des FDM-DUO ist um seinen VFO herum gebaut, daher bewegt Abstimmen hier die Frontplatte, und Drehen des Frontplattenknopfs bewegt die Anzeige hier. Das Panorama zentriert sich beim Gehen auf die Skala neu. Ohne Port — oder einen, auf dem das Funkgerät nicht antwortet — gibt es keinen VFO zu steuern, und das Abstimmen bleibt im Fenster, das das Funkgerät bereits sendet.
settings-radio-none-usb-control-only = — keiner (nur USB-Steuerung) —
settings-radio-must-match-menu-70-cat = Muss zu Menü 70 „CAT BAUD“ am Funkgerät passen, das mit 38400 ausgeliefert wird. Das Funkgerät hat diese vier Raten und keine anderen, und ein auf einer anderen geöffneter Port ist in beide Richtungen stumm — das Funkgerät ignoriert die Skala und lehnt das Tasten ab, ohne dass es sagt, warum.
settings-radio-how-transmit-keyed-cat-sends = Wie das Senden getastet wird. „CAT“ sendet den eigenen TX-Befehl des Funkgeräts und braucht am Gerät keine Einrichtung; RTS braucht Menü 54 „PTT“ auf PTT+RTS.
settings-radio-where-radio-takes-transmit-audio-2 =
    Woher das Funkgerät sein Sendeaudio nimmt — der Befehl TI des Geräts, das ist Menü 32 „TX IN“ an der Frontplatte. Wird beim Öffnen des Ports gesetzt.

    „USB-Audio“ ist es, was das Senden hier zum Laufen bringt: Der FDM-DUO sendet, was sdroxide in seine USB-Soundkarte gibt. Ein Funkgerät auf „Mikrofon“ sendet stattdessen den Raum, und auf dem Bildschirm steht nichts davon.
settings-radio-which-two-sockets-back-receiver-2 =
    Welche der beiden Buchsen auf der Rückseite der Empfänger hört — der Befehl AN des Geräts, das ist Menü 31 „ANTENNAS“ an der Frontplatte und die Anzeige „ANT 1 2“ auf seinem Display.

    „RTX“ ist eine Antenne, die beide Aufgaben übernimmt, an der M-Buchse, die auch das Senden führt. „Nur RX“ verlegt den Empfang auf die zweite Buchse und lässt das Senden auf RTX — eine Empfangsantenne, eine Schleife oder eine Beverage, mit der Richtantenne weiter am Sender.

    Es verlegt diesen ganzen Empfänger: das Panorama, die Demodulatoren und das eigene Audio des Geräts kommen alle von der hier gewählten Buchse.

    Wirkt sofort und wird beim Öffnen des Steuerports vom Funkgerät zurückgelesen — dies ist die eigene Einstellung des Geräts, keine hier gehaltene Kopie.
settings-radio-how-often-radio-asked-what-2 = Wie oft das Funkgerät gefragt wird, was es tut — seine Skala, seine Betriebsart und seine Messgeräte. Dreht man am VFO-Knopf des Geräts, folgt die Anzeige innerhalb einer Abfrage. Es ist außerdem der gesamte Steuerverkehr, den dieses Ende erzeugt, daher senken, wenn das Audio des Funkgeräts zerfällt.
settings-radio-transmit-audio = Sendeaudio
settings-radio-system-default = — Systemstandard —
settings-radio-fdm-duo-s-own-usb = Der eigene USB-Audio-Port des FDM-DUO. Auf dem Systemstandard ist es fast nie das Funkgerät — sondern die Lautsprecher des Rechners.
settings-radio-press-rescan-list-sound-cards = Erneut suchen drücken, um die Soundkarten aufzulisten
settings-radio-copies-last-session-s-trace = Kopiert die Aufzeichnung der letzten Sitzung für einen Fehlerbericht in die Zwischenablage: jeder Befehl, der mit dem Gerät ausgetauscht wurde, und die ersten Bytes des Abtaststroms.
settings-radio-only-receiving-fdm-duo-has = Bisher wurde nur der Empfang an einem FDM-DUO an echter Hardware ausgeführt. Wenn sich etwas falsch verhält, hängen Sie bitte den Diagnosebericht an einen Fehlerbericht an.
settings-radio-cw-keyed-radio-s-own = CW wird über die eigene Taste oder das Paddle des Funkgeräts getastet: Der FDM-DUO hat keinen Befehl, der Text annimmt, daher kann das CW-Panel ihn nicht tasten.
settings-radio-press-apply-reopen-radio = „Übernehmen“ drücken, um das Funkgerät erneut zu öffnen.
settings-radio-no-receivers-press-rescan = keine Empfänger — Erneut suchen drücken
settings-radio-which-rates-receiver-has-depends = Welche Raten ein Empfänger hat, hängt von Modell und Firmware ab. Wirkt nach Übernehmen.
settings-radio-every-rate-any-hf-model = jede Rate, die ein beliebiges HF+-Modell anbietet — eines anschließen, um dessen eigene zu sehen
settings-radio-receiver-s-own-gain-control = Die eigene Verstärkungsregelung des Empfängers. Eingeschaltet lassen für allgemeines Hören; ausschalten, um das Dämpfungsglied für Messungen von Hand einzustellen.
settings-radio-agc-threshold = AGC-Schwelle
settings-radio-high = Hoch
settings-radio-high-trades-little-sensitivity-headroom = Hoch tauscht etwas Empfindlichkeit gegen Reserve gegenüber starken Nachbarn — die richtige Einstellung auf einem belegten Band nachts.
settings-radio-front-end-attenuation-gain-0 = Frontstufen-Dämpfung als Verstärkung — 0 dB ist keine. Wird nur befolgt, wenn die AGC aus ist.
settings-radio-preamp = Vorverstärker
settings-radio-hf-low-noise-amplifier-buys = Der KW-rauscharme Verstärker. Er erkauft Empfindlichkeit um den Preis von Intermodulation, daher ist er standardmäßig aus — was an einer echten Antenne meist richtig ist.
settings-radio-lna = LNA ein
settings-radio-frequency-calibration = Frequenzkalibrierung
settings-radio-parts-per-billion-receiver-s = Teile pro Milliarde — die eigene Einheit dieses Empfängers, tausendmal feiner als die ppm, die ein RTL-SDR nutzt. Nichts hier wird je in den Flash des Empfängers geschrieben: Dies überschreibt den gespeicherten Wert nur für die Sitzung.
settings-radio-use-receiver-s-stored-value = Den gespeicherten Wert des Empfängers verwenden
settings-radio-ppb = {" "}ppb
settings-radio-feed-dc-up-coax = Gleichspannung über das Koaxkabel speisen
settings-radio-not-every-hf-has-one = Nicht jeder HF+ hat eine; bei einem Empfänger ohne tut dies nichts.
settings-radio-host-dsp = Host-DSP
settings-radio-image-balancer-zero-if-offset = Der Spiegelbild-Abgleich, der Null-ZF-Versatz und der Feinabstimmoszillator. Nur ausschalten, um die rohe Ausgabe der Hardware zu sehen — ohne sie erscheint das Spiegelbild auf den Null-ZF-Raten, und die Skala ist nur auf das nächste Kilohertz genau.
settings-radio-correct-image-fine-tune = Spiegelbild korrigieren und feinabstimmen
settings-radio-copies-last-session-s-trace-2 = Kopiert die Aufzeichnung der letzten Sitzung für einen Fehlerbericht in die Zwischenablage: jeden Befehl, der mit dem Empfänger ausgetauscht wurde, und die ersten Bytes des Abtaststroms.
settings-radio-receive-only-no-soapysdr-no = Nur Empfang. Kein SoapySDR und keine libairspyhf nötig. Unterhalb der Untergrenze des Synthesizers übernimmt der Host-Oszillator die Abstimmung, so erreicht dieser Empfänger VLF. Empfänger und Abtastrate wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern.
settings-radio-not-yet-verified-against-real-3 = Noch nicht an echter Hardware geprüft. Wenn es sich falsch verhält, senden Sie bitte den Diagnosebericht — er enthält jeden Befehl, der mit dem Empfänger ausgetauscht wurde, und die ersten Bytes des Abtaststroms, dekodiert als I/Q-Paare.
settings-radio-no-airspy-r2-mini-found = Kein Airspy R2 oder Mini gefunden — Erneut suchen drücken
settings-radio-read-receiver = von diesem Empfänger lesen
settings-radio-r2-mini-offer-different-rates = ein R2 und ein Mini bieten unterschiedliche Raten an und lassen sich erst unterscheiden, wenn einer geöffnet ist
settings-radio-rate-you-get-receiver-s = Dies ist die Rate, die Sie bekommen. Der ADC des Empfängers läuft mit dem Doppelten — er digitalisiert ein reelles Signal, und sdroxide macht daraus auf dem Rechner komplexes Basisband.
settings-radio-gain-curve = Verstärkungskurve
settings-radio-step-along-curve-above-not = Ein Schritt entlang der Kurve oben, keine dB-Angabe — LNA, Mischer und VGA des Tuners bewegen sich zusammen, und was jeder Schritt wert ist, hängt von der Kurve und dem Band ab. 0 ist das leise Ende.
settings-radio-tuner-agc = AGC des Tuners
settings-radio-lna-2 = LNA
settings-radio-hands-lna-tuner-s-own = Übergibt den LNA der eigenen Schleife des Tuners und übersteuert die Kurve.
settings-radio-mixer = Mischer
settings-radio-same-mixer-stage = Dasselbe für die Mischerstufe.
settings-radio-loop-running-gain-slider-no = Während eine Schleife läuft, setzt der Verstärkungsregler nicht mehr die Stufe, die sie besitzt — die Schleife überschreibt sie.
settings-radio-dc-antenna-port = Gleichspannung am Antennenanschluss
settings-radio-t-12-bit-packing = 12-Bit-Packung
settings-radio-third-less-usb-traffic-leave = Ein Drittel weniger USB-Verkehr. Eingeschaltet lassen: Dies ist ein USB-2.0-Gerät, und die höchste Rate ist 30 MB/s gepackt gegenüber 40 ungepackt. Wirkt beim erneuten Verbinden.
settings-radio-dc-removal = DC-Entfernung
settings-radio-remove-adc-s-offset = Den Versatz des ADC entfernen
settings-radio-turn-off-see-raw-hardware = Ausschalten, um die rohe Ausgabe der Hardware zu sehen. Es lohnt sich zu wissen, wohin der Störpeak fällt: Der Versatz landet am Rand des Bereichs, nicht in seiner Mitte, weil das Signal auf dem Weg um ein Viertel der Abtastrate verschoben wird.
settings-radio-every-command-exchanged-receiver-sample = Jeder Befehl, der mit dem Empfänger ausgetauscht wurde, die Rechnung mit der Abtastrate und die ersten Abtastwerte, dekodiert als I/Q.
settings-radio-receive-only-24-1800-mhz = Nur Empfang, 24–1800 MHz. Kein SoapySDR und kein libairspy nötig. Empfänger, Abtastrate und Packung wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern.
settings-radio-not-yet-verified-against-real-4 = Noch nicht an echter Hardware geprüft. Wenn es sich falsch verhält, senden Sie bitte den Diagnosebericht — er enthält jeden Befehl, der mit dem Empfänger ausgetauscht wurde, die Rechnung mit der Rate und die ersten Abtastwerte, dekodiert als I/Q-Paare.
settings-radio-no-hydrasdr-rfone-found-press = Kein HydraSDR RFOne gefunden — Erneut suchen drücken
settings-radio-these-are-rates-receiver-turned = Dies sind die Raten, die dieser Empfänger tatsächlich hat
settings-radio-receiver-only-reports-three-these = Der Empfänger meldet nur drei davon; die übrigen stehen in der alternativen Tabelle der Firmware, und ein älterer Build hat sie vielleicht nicht
settings-radio-rf-input = HF-Eingang
settings-radio-antenna-sma-only-socket-bias = Die Antennen-SMA — die einzige Buchse mit dem Bias-Tee dahinter.
settings-radio-cable-socket-no-bias-tee = Eine Kabelbuchse. Kein Bias-Tee hier; die Hardware legt ihn nur auf ANT.
settings-radio-powers-active-antenna-preamp-down-2 = Speist eine aktive Antenne oder einen Vorverstärker über das Koaxkabel. Nur die ANT-Buchse hat das — die beiden Kabelanschlüsse sind reine Eingänge.
settings-radio-third-less-usb-traffic-leave-2 = Ein Drittel weniger USB-Verkehr. Eingeschaltet lassen: Dies ist ein USB-2.0-Gerät, und die höchste Rate ist 36 MB/s gepackt gegenüber 48 ungepackt. Wirkt beim erneuten Verbinden.
settings-radio-one-these-boards-1d50-60a1 = Eine dieser Karten hängt an 1d50:60a1, der USB-ID, die die Prototypen von HydraSDR mit dem Airspy R2 und Mini teilen. sdroxide prüft die Firmware nach dem Öffnen und sagt es, falls die falsche Schnittstelle gewählt wurde, in beiden Richtungen.
settings-radio-receive-only-24-1800-mhz-2 = Nur Empfang, 24–1800 MHz. Kein SoapySDR und keine libhydrasdr nötig. Empfänger, Abtastrate und Packung wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern.
settings-radio-re-list-fobos-sdrs-no = Fobos-SDRs erneut auflisten. Es wird kein Gerät geöffnet, daher ist es während des Empfangs sicher.
settings-radio-no-fobos-sdr-found-press = Kein Fobos SDR gefunden — Erneut suchen drücken
settings-radio-input-2 = Eingang
settings-radio-through-tuner-lna-vga-mixer = Über den Tuner: LNA, VGA und der eigene Frequenzbereich des Mischers.
settings-radio-direct-sampling-hf1-antenna-input = Direkte Abtastung am Antenneneingang HF1 — kein Tuner, keine LNA-/VGA-Verstärkung, vollständig in Software abgestimmt.
settings-radio-direct-sampling-hf2-antenna-input = Direkte Abtastung am Antenneneingang HF2, wie HF1.
settings-radio-both-real-adc-channels-once = Beide echten ADC-Kanäle gleichzeitig, kombiniert durch den Diversitätsfilter unten — eine Störquelle ausgelöscht oder zwei schwindende Wege kombiniert.
settings-radio-direct-sampling-tuner-so-lna = Direkte Abtastung — der Tuner (und damit die LNA-/VGA-Verstärkung) ist nicht im Weg. Die Rate unten ist ein Ziel für den Software-Abwärtsmischer, keine Hardware-Einstellung, daher kann die erreichte Rate davon abweichen.
settings-radio-receiver-s-own-reported-rates = die eigenen gemeldeten Raten dieses Empfängers
settings-radio-measured-one-real-unit-connect = an einem echten Gerät gemessen — diesen Empfänger anschließen und Erneut suchen drücken, um seine eigene Liste zu sehen
settings-radio-clock-source = Taktquelle
settings-radio-external-reference = Externe Referenz
settings-radio-diversity-taps = Diversitäts-Abgriffe
settings-radio-taps = {" "}Abgriffe
settings-radio-one-tap-gain-phase-null = Ein Abgriff ist eine Verstärkung und eine Phase — eine Auslöschung auf einer Frequenz, die zu beiden Seiten schlechter wird, mehr kann ein analoger Phasenschieber nicht. Jeder weitere Abgriff bringt eine Abtastperiode des Wegunterschieds zwischen HF1 und HF2, den das Filter ausgleichen kann, und das macht aus dieser Kerbe ein Band, das über die ganze Breite ruhig ist.
settings-radio-receive-only-rf-port-tuner = Nur Empfang. HF-Port (Tuner), HF1/HF2 (direkte Abtastung, in Software dekodiert) und HF1+HF2 (beide gleichzeitig, durch den Diversitätsfilter kombiniert) funktionieren alle. Empfänger, Eingang und Abtastrate wirken nach Übernehmen; Verstärkung, Taktquelle und Abgriffe wirken, sobald Sie sie ändern — Modus, Anpassungsrate und Halten des Diversitätsfilters liegen in der eigenen Box DIV des Hauptfensters, sobald dieses Funkgerät gewählt ist.
settings-radio-no-hackrf-found-press-rescan = Kein HackRF gefunden — Erneut suchen drücken
settings-radio-front-end-amplifier-8-db = Frontstufen-Verstärker in 8-dB-Schritten. Dies ist die Stufe, die die Empfindlichkeit ändert — und die bei einer echten Antenne zuerst übersteuert.
settings-radio-baseband-amplifier-after-mixer-2 = Basisbandverstärker nach dem Mischer in 2-dB-Schritten. Ihn für ein schwaches Signal hochdrehen, bevor man zum LNA greift.
settings-radio-rf-amp = HF-Verstärker
settings-radio-one-switch-shared-transmit-setting = Ein Schalter, geteilt mit der Sendeeinstellung unten — das Funkgerät wendet an, was zur Richtung gehört, in die es gerade wechselt. Aus ist an einer echten Antenne meist richtig.
settings-radio-hackrf-pro-chooses-itself-three = Ein HackRF Pro wählt dies selbst — drei Viertel der Abtastrate, im FPGA gefiltert — und ignoriert alles, was der Host verlangt.
settings-radio-leave-automatic-choosing-one-too = Auf Automatisch lassen. Wählt man eine zu enge, werden nicht nur die Bandränder weicher — es wird der LO-Versatz zurückgenommen, der den DC-Spike von Ihrem Signal fernhält.
settings-radio-about-3-v-50-ma = Etwa 3 V bei 50 mA über das Koaxkabel, für eine aktive Antenne oder einen Vorverstärker. Nur ein HackRF One oder Pro; der Jawbreaker und das rad1o haben keine solche Schaltung.
settings-radio-remove-dc-mirror-image = DC und das Spiegelbild entfernen
settings-radio-zero-if-radio-so-own = Dies ist ein Null-ZF-Funkgerät, daher liegt sein eigenes LO-Übersprechen in der Bandmitte, und der Quadraturfehler des Mischers legt ein Spiegelbild darüber. Ausschalten zeigt die rohe Ausgabe der Hardware, und so erkennt man schnell, ob ein Treiberproblem oder ein DSP-Problem vorliegt.
settings-radio-corrects-reference-oscillator = Korrigiert den Referenzoszillator.
settings-radio-transmit = Senden
settings-radio-transmitter = Sender
settings-radio-off-default-while-off-backend = Standardmäßig aus. Solange dies aus ist, veröffentlicht das Backend überhaupt keinen Sendekanal, daher kann nichts das Funkgerät tasten. Wirkt beim erneuten Verbinden.
settings-radio-tx-vga-gain = TX-VGA-Verstärkung
settings-radio-transmit-driver-amplifier-drive-applied = Der Treiberverstärker des Senders. Die Aussteuerung wird digital vor dieser Stufe angewendet, daher die Aussteuerung hoch lassen und den Ausgangspegel hier setzen — die Aussteuerung herunterzudrehen fährt den DAC stattdessen mit einem Bruchteil der Vollaussteuerung und erhöht die Intermodulation.
settings-radio-same-switch-receive-setting-above = Derselbe Schalter wie die Empfangseinstellung oben, angewendet, wenn das Funkgerät die Richtung wechselt.
settings-radio-transmit-armed-into-dummy-load = Senden ist scharfgeschaltet. Erst in eine Kunstlast, bis Sie es gemessen haben: Die Oberwellen eines HackRF sind stark genug, dass man für das jeweilige Band ein externes Tiefpassfilter braucht, und er ist halbduplex, daher hält der Empfang für die Dauer jeder Aussendung an.
settings-radio-every-command-exchanged-radio-session = Jeder Befehl, der in dieser Sitzung mit dem Funkgerät ausgetauscht wurde, in Reihenfolge, einschließlich der Abfolge um jedes Niederdrücken herum.
settings-radio-dc-7-25-ghz-half = DC – 7,25 GHz, halbduplex. Kein SoapySDR, kein libusb und kein libhackrf nötig. Funkgerät und Abtastrate wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern. Die Bandränder sind die der Firmware: Ein HackRF ist für 1 MHz – 6 GHz (100 kHz bei einem Pro) spezifiziert und außerhalb davon stark gedämpft, stimmt dort aber dennoch ab.
settings-radio-ask-sdrplay-api-service-device = Den SDRplay-API-Dienst nach seiner Geräteliste fragen. Es wird nichts geöffnet, daher ist es während des Empfangs sicher.
settings-radio-no-rsps-press-rescan-needs = keine RSPs — Erneut suchen drücken (braucht den SDRplay-API-Dienst)
settings-radio-both-tuners-running-api-fixes = Laufen beide Tuner, fixiert die API den ADC auf 6 MHz und gibt 2 Msps aus einer niedrigen ZF zurück — 2 Msps sind daher der breiteste Bereich, und die schmaleren sind davon dezimiert. Wirkt nach Übernehmen.
settings-radio-rates-below-2-msps-run = Raten unter 2 Msps laufen mit dem ADC bei 2 Msps und dezimieren im Dienst. Wirkt nach Übernehmen.
settings-radio-reduced-adc-resolution = {"  "}(verringerte ADC-Auflösung)
settings-radio-if-bandwidth = ZF-Bandbreite
settings-radio-tuner-s-analog-filter-auto = Das Analogfilter des Tuners. Automatisch wählt das breiteste, das zur Abtastrate passt. Wirkt nach Übernehmen.
settings-radio-rsp-s-own-if-gain = Die eigene ZF-Verstärkungsschleife des RSP, vom API-Dienst betrieben. Aus gibt den ZF-Verstärkungsregler an Sie zurück — die Einstellung für Messung und digitale Schwachsignal-Betriebsarten.
settings-radio-agc-set-point = AGC-Sollwert
settings-radio-signal-level-loop-holds-adc = Der Signalpegel, auf dem die Schleife den ADC hält. Niedriger lässt mehr Reserve für Signale neben dem Kanal. Wie niedrig er gehen darf, hängt von der Abtastrate ab — über 8,064 Msps hat der Wandler weniger Reserve zu geben, und über 9,216 Msps noch weniger.
settings-radio-rsp-s-native-gain-unit = Die eigene Verstärkungseinheit des RSP: 20 dB sind maximale Verstärkung (0 mit dem erweiterten Bereich unten), 59 dB minimal. Wirkt sofort. Wird ignoriert, solange die AGC läuft — dann gehört dieser Wert der Schleife, und das S-Meter zeigt, worauf sie sich eingependelt hat.
settings-radio-extended-if-range = Erweiterter ZF-Bereich
settings-radio-lets-if-gain-reduction-go = Lässt die ZF-Verstärkungsreduktion unter 20 dB gehen, bis auf 0 — die letzten 20 dB Verstärkung, die der Empfänger hat. Aus ist der eigene Standard der API und der richtige für gewöhnliches Hören, weil das untere Ende des Bereichs der Ort ist, an dem ein RSP am leichtesten übersteuert. Es lohnt sich für schwache Signale, wenn der LNA schon auf 0 steht. Es lässt außerdem den AGC-Sollwert bis 0 dBFS steigen. Wirkt sofort.
settings-radio-allow-if-gain-reduction-below = ZF-Verstärkungsreduktion unter 20 dB erlauben
settings-radio-front-end-attenuation-steps-0 = Frontstufen-Dämpfung in Schritten: 0 ist maximale Verstärkung, jeder Schritt nach oben schaltet mehr Dämpfung ein. Manche Bänder haben weniger Schritte — der Treiber begrenzt und behält Ihre Wahl für die Rückkehr zur Abstimmung, und der Regler hier endet bei dem, was das Band, auf dem das Funkgerät liegt, zulässt. Eine höhere Wahl, die auf einem anderen Band getroffen wurde, bleibt erhalten und wird daneben weiter angezeigt. Wirkt sofort.
settings-radio-t-0-max-gain = 0 = maximale Verstärkung
settings-radio-reference-error-parts-per-million-2 = Referenzfehler in Teilen pro Million, vom Gerät selbst angewendet. Wirkt sofort.
settings-radio-radio-s-tuner = Der Tuner dieses Funkgeräts
settings-radio-tuner = Tuner
settings-radio-which-rspduo-s-two-tuners = Welchen der beiden Tuner des RSPduo dieses Funkgerät abhört. Der andere gehört dem Funkgerät, das dafür eingerichtet ist — geben Sie diesem denselben Empfänger und den anderen Tuner. Wirkt nach Übernehmen.
settings-radio-which-rspduo-s-two-tuners-2 = Welcher der beiden Tuner des RSPduo die Antenne führt, die Sie abhören. Der andere führt die zweite Antenne. Wirkt nach Übernehmen.
settings-radio-which-rspduo-s-two-tuners-3 = Welchen der beiden Tuner des RSPduo betrieben werden soll (jeweils einer). Wirkt nach Übernehmen.
settings-radio-applies-immediately = Wirkt sofort.
settings-radio-fm-broadcast-notch = UKW-Rundfunk-Kerbe
settings-radio-hardware-notch-over-88-108 = Hardware-Kerbe über dem UKW-Rundfunkband 88–108 MHz, wenn ein lokaler Sender alles andere übersteuert. Wirkt sofort.
settings-radio-dab-notch = DAB-Kerbe
settings-radio-hardware-notch-over-165-230 = Hardware-Kerbe über dem DAB-Band 165–230 MHz. Wirkt sofort.
settings-radio-hdr-mode = HDR-Modus
settings-radio-enable-below-2-mhz = Unter 2 MHz aktivieren
settings-radio-rspdx-s-high-dynamic-range =
    Der Pfad mit hohem Dynamikbereich des RSPdx für LF/MF. Wirkt sofort.

    Er funktioniert noch nicht: Auf dem einen RSPdx, an dem er getestet wurde, lässt das Einschalten den Empfänger verstummen — bei jeder Mitte, jedem Filter, Port, jeder Verstärkung und Rate, die versucht wurde. Was der Modus braucht, steht nicht in der dokumentierten API, und es herauszufinden ist die nächste Aufgabe von jemandem. Lassen Sie dies aus, sofern Sie nicht derjenige sind, der danach sucht.

    Bekannt ist: Es ist kein Modus, der der Skala folgt. Der Pfad hat ein festes Analogfilter, das nur bei den unten unter dem Filter aufgeführten Mitten gebaut ist, daher könnte Abstimmen woanders nicht funktionieren, selbst wenn der Rest liefe.
settings-radio-hdr-filter = HDR-Filter
settings-radio-feed-4-7-v-dc = ca. 4,7 V Gleichspannung über das Koaxkabel speisen
settings-radio-second-tuner = Zweiter Tuner
settings-radio-rspduo-two-whole-tuners-one = Ein RSPduo sind zwei vollständige Tuner auf einer Karte, getaktet von einer Referenz. Betreibt man beide, hören sie ihre Spannen im selben Augenblick vom selben Takt — was zwei Antennen an ihnen kohärent macht, mit einer relativen Phase, die durch die Speiseleitungen bestimmt wird statt durch Zufall, und das Paar lässt sich kombinieren. Oder man lässt sie getrennt: Die Tuner stimmen getrennt ab, daher kann der andere ein zweites Funkgerät auf seinem eigenen Band sein.
settings-radio-run-both-tuners = Beide Tuner betreiben
settings-radio-puts-rspduo-api-s-dual = Versetzt das RSPduo in den Dual-Tuner-Modus der API, in dem der ADC auf 6 MHz festgelegt ist und die breiteste Spanne 2 Msps beträgt. Wirkt nach Übernehmen, weil der Modus beim Öffnen des Geräts gewählt wird.
settings-radio-used = Verwendet für
settings-radio-what-other-tuner-takes-effect = Wofür der andere Tuner ist. Wirkt nach Übernehmen — wie das Betreiben beider überhaupt, wird dies gewählt, wenn die Karte geöffnet wird.
settings-radio-add-second-radio-settings-radio = Ein zweites Funkgerät hinzufügen (Einstellungen → Funkgerät → +), ihm denselben Empfänger und den anderen Tuner des RSPduo geben und es ebenfalls beide Tuner betreiben lassen — welches Funkgerät die Karte öffnet, setzt sie in den Dual-Tuner-Modus, daher müssen beide darauf eingestellt sein. Die beiden stimmen dann unabhängig ab: KW in einem Reiter und UKW im anderen, von einer Karte. Sie teilen sich einen ADC-Takt, daher laufen beide mit der Abtastrate, die das Funkgerät, das die Karte öffnete, angefordert hat, und keines kann senden.
settings-radio-what-do = Was damit geschieht
settings-radio-lna-state-2 = Seine LNA-Stufe
settings-radio-second-tuner-s-own-front = Die eigene Frontstufen-Dämpfung des zweiten Tuners. So einstellen, dass beide Antennen etwa denselben Rauschboden zeigen: Darauf ruht alles andere, weil das Kombinieren die beiden Zweige nach ihrem Rauschen gewichtet, und eine zweite Frontstufe, die in die Übersteuerung getrieben wird, gibt dem Filter eine verzerrte Kopie der Störung — die sich nicht von einer unverzerrten abziehen lässt. Wirkt sofort.
settings-radio-if-gain-reduction-2 = Seine ZF-Verstärkungsreduktion
settings-radio-second-tuner-s-if-gain = Die ZF-Verstärkung des zweiten Tuners in der eigenen Einheit des RSP: 20 dB sind maximale Verstärkung. Wird ignoriert, solange die AGC läuft — und eine gleichmäßige Verstärkung ist, was das Filter will, daher lohnt es sich, die AGC für eine Auslöschung auszuschalten, die man halten will.
settings-radio-filter-length = Filterlänge
settings-radio-one-tap-gain-phase-null-2 = Ein Abgriff ist eine Verstärkung und eine Phase — eine Auslöschung auf einer Frequenz, die zu beiden Seiten schlechter wird, mehr kann ein analoger Phasenschieber nicht. Jeder weitere Abgriff bringt eine Abtastperiode des Wegunterschieds zwischen den beiden Antennen, den das Filter ausgleichen kann, und das macht aus dieser Kerbe ein Band, das über die ganze Breite ruhig ist.
settings-radio-adaptation-2 = Anpassung
settings-radio-slow-steady-left-converging-inside = Links langsam und gleichmäßig, es konvergiert innerhalb eines Bruchteils einer Sekunde und jagt rechts sichtbar. Zum Finden der Auslöschung schnell beginnen, dann halten.
settings-radio-stop-filter-moving-reach-moment = Das Filter am Bewegen hindern. Danach greifen, sobald eine Auslöschung erscheint: Ein Filter, das weiter anpasst, richtet sich auf das aus, was am lautesten wird, und auf einem ruhigen Band ist das der Sender, den Sie hören.
settings-radio-restart = Neustart
settings-radio-zero-filter-find-null-again = Das Filter auf null setzen und die Auslöschung erneut finden.
settings-radio-nothing-here-can-tell-wanted = Hier kann niemand ein gewünschtes Signal von einem unerwünschten unterscheiden — das Filter weiß nur, was die beiden Antennen gemeinsam haben. Bei Auslöschen will die zweite Antenne die Störquelle hören und möglichst wenig vom Band, sonst löscht sie gewissenhaft auch den Sender aus. Bei Kombinieren wollen beide denselben Sender hören. Wie tief die Auslöschung ist, wird alle paar Sekunden ins Protokoll geschrieben.
settings-radio-not-yet-verified-against-rspduo = Noch nicht an einem RSPduo geprüft: Der Dual-Tuner-Betrieb ist hier aus der API von SDRplay geschrieben, nicht an der Hardware gemessen.
settings-radio-no-receiver-listed-check-against = Kein Empfänger ist aufgelistet, mit dem man das prüfen könnte — Erneut suchen drücken oder im Protokoll nachsehen, was tatsächlich geöffnet wurde.
settings-radio-receive-only-1-khz-2 = Nur Empfang, 1 kHz–2 GHz. Braucht den Herstellerdienst SDRplay API (sdrplay.com/api) — die RSPs nach dem ursprünglichen RSP1 haben kein offenes Protokoll. Gerät, Abtastrate, Bandbreite und RSPduo-Tuner wirken nach Übernehmen; alles andere wirkt, sobald Sie es ändern.
settings-radio-board = Karte
settings-radio-first-one-found-2 = Der erste gefundene
settings-radio-receive-chain = Empfangskette
settings-radio-both-chains-tune-together-they = Beide Ketten stimmen zusammen ab — sie teilen sich einen Synthesizer —, sind aber getrennte Frontstufen an getrennten Buchsen. Wählen Sie die, an der Ihre Antenne hängt; das ist die Einstellung, zu der man greift, wenn eine Kette die HF-Anpassungsänderung erhalten hat und die andere original ist.
settings-radio-receive-gain = Empfangsverstärkung
settings-radio-one-combined-figure-which-limesuite = Eine kombinierte Angabe, die LimeSuite selbst auf LNA, TIA und PGA verteilt. Sie nimmt ganze Dezibel; alles Feinere wird abgeschnitten.
settings-radio-receive-port = Empfangsport
settings-radio-automatic-follows-frequency-lnal-low = Automatisch folgt der Frequenz: LNAL niedrig, LNAH hoch — außer ein LimeRFE ist unten angeschlossen, das ist ein Kabel in eine Buchse, dann ist es bei jeder Frequenz LNAW. Die Buchse nennen, an der Ihre verdrahtet ist, falls es nicht diese ist. Zu welcher Kette die Buchse gehört, bestimmt die Auswahl oben.
settings-radio-t-0-follows-sample-rate-worth = 0 folgt der Abtastrate. Dort lassen lohnt sich: Ein Filter, das schmaler als ein Viertel der Spanne ist, nimmt stillschweigend den Null-ZF-LO-Versatz zurück, und das LO-Übersprechen liegt dann wieder auf dem Signal, das Sie hören.
settings-radio-following-rate = folgt der Rate
settings-radio-corrections = Korrekturen
settings-radio-host-iq-dc-correction = Host-IQ-/DC-Korrektur
settings-radio-adaptive-image-dc-removal-side = Adaptive Entfernung von Spiegelbild und DC auf dieser Seite, zusätzlich zur eigenen Kalibrierung des Chips. Ausschalten ist der Ein-Klick-Weg, ein Treiberproblem von einem DSP-Problem zu unterscheiden.
settings-radio-calibrate-automatically = Automatisch kalibrieren
settings-radio-runs-chip-s-own-dc = Führt die eigene DC-Offset- und Spiegelbild-Kalibrierung des Chips aus, wenn das Funkgerät geöffnet wird, und erneut, sobald sich die Skala auf einem neuen Band oder einer anderen Buchse gesetzt hat. Diese Werte sind bei einer Frequenz gemessen und anderswo falsch, was ein Träger in der Mitte der Spanne meist ist. Kostet jedes Mal etwa eine Sekunde.
settings-radio-calibrate-now = Jetzt kalibrieren
settings-radio-stalls-receiver-better-part-second = Hält den Empfänger für den größeren Teil einer Sekunde an.
settings-radio-second-aerial = Zweite Antenne
settings-radio-socket = Ihre Buchse
settings-radio-same-first = Wie die erste
settings-radio-gain-2 = Ihre Verstärkung
settings-radio-set-low-coupled-sample-your = Dies NIEDRIG einstellen. Die eingekoppelte Probe Ihres eigenen Senders ist ein starkes Signal, und eine Rückkopplungskette, die in die Kompression getrieben wird, misst die Kurve des Verstärkers falsch — sie lehrt die Korrektur die eigene Verzerrung. Unten beginnen und das Dämpfungsglied des Kopplers nutzen.
settings-radio-set-so-both-aerials-show = So einstellen, dass beide Antennen etwa denselben Rauschboden zeigen. Darauf ruht die ganze Sache: Das Kombinieren gewichtet die beiden Zweige nach ihrem Rauschen, und eine zweite Kette, die in die Kompression getrieben wird, gibt dem Filter eine verzerrte Kopie der Störung — die sich nicht von einer unverzerrten abziehen lässt.
settings-radio-table-steps-2 = Tabellenschritte
settings-radio-how-finely-correction-follows-amplifier = Wie fein die Korrektur der Kurve des Verstärkers folgt. Mehr Schritte verfolgen eine schärfere Knickstelle, aber jeder muss aus den Abtastwerten gelernt werden, die in ihn fielen — und die Spitze eines Sprach-Amplituden-Histogramms ist dünn. Zweiunddreißig reicht für die glatte Kurve, die ein KW-Verstärker tatsächlich hat. Eine Änderung startet die Korrektur neu.
settings-radio-how-hard-each-block-feedback = Wie stark jeder Block der Rückkopplung die Korrektur bewegt. Die Kurve eines Verstärkers ändert sich nicht, daher gibt es keinen Grund zur Eile — die Mitte mittelt mehrere Aussendungen Rauschen heraus.
settings-radio-keep-correction-curve-learned-clean = Die Korrektur so lassen, wie sie ist. Eine Kurve, die bei einer sauberen Aussendung gelernt wurde, lohnt sich zu halten, und der Verstärker wird sich bis zur nächsten nicht geändert haben.
settings-radio-forget-correction-learn-again = Die Korrektur vergessen und neu lernen.
settings-radio-directional-coupler-amplifier-s-output = Ein Richtkoppler am Ausgang des Verstärkers geht in diese Kette, und der Sender vergleicht, was zurückkam, mit dem, was er senden wollte — dann sendet er das Gegenteil der Differenz, damit das, was den Verstärker verlässt, gerade ist. Zwanzig Dezibel weniger Intermodulation in den QSOs anderer, bei gleicher Leistung des Verstärkers. Die Korrektur bleibt bei Eins, bis die Rückkopplung mit der Aussendung übereinstimmt, daher kostet ein nicht angeschlossener Koppler nichts; und sie kann den Konverter nie über die Vollaussteuerung hinaus fordern, daher kann eine Rückkopplung mit Unsinn nichts übersteuern. Wie sie vorankommt, wird beim Senden ins Protokoll geschrieben.
settings-radio-transmit-not-armed-so-there = Senden ist nicht scharfgeschaltet, daher gibt es hier nichts zu korrigieren.
settings-radio-off-interface-publishes-no-transmit = Bei Aus veröffentlicht die Schnittstelle überhaupt keinen Sendekanal, daher kann nichts das Funkgerät tasten.
settings-radio-transmit-gain = Sendeverstärkung
settings-radio-transmit-port = Sendeport
settings-radio-board-s-two-transmit-sockets = Die beiden Sendebuchsen der Karte sind zwei verschiedene Anpassnetzwerke, nicht zwei Anschlüsse an dasselbe: BAND1 (TX_1) führt 30 MHz bis 1,9 GHz, und BAND2 (TX_2) ist der 13-cm-Port. Ein Amateurfunkband unterhalb von 23 cm über BAND2 zu tasten schickt die Aussendung in ein Anpassnetzwerk, das fast nichts davon durchlässt — jede Anzeige stimmt, die Aussteuerung ist, was Sie eingestellt haben, und das Leistungsmessgerät bleibt bei null. Auf Automatisch lassen, dann folgt es der Skala.
settings-radio-limesdr-transmits-about-100-khz = Ein LimeSDR sendet von etwa 100 kHz bis 3,8 GHz ohne eigene Filterung. Einen Tiefpass, einen passenden LimeRFE-Kanal oder eine Kunstlast verwenden.
settings-radio-limerfe-front-end = LimeRFE-Frontstufe
settings-radio-connected = Verbunden über
settings-radio-limerfe-s-own-micro-usb = Der eigene Micro-USB-Port des LimeRFE, nicht der des Funkgeräts. 9600 Baud, durch seine Firmware festgelegt.
settings-radio-receive-connector = Empfangsanschluss
settings-radio-transmit-connector = Sendeanschluss
settings-radio-follow-dial = Der Skala folgen
settings-radio-switch-filters-match-operating-frequency = Die Filter auf die Betriebsfrequenz umschalten, bevor HF erscheint. Abstimmen innerhalb eines Bandes sendet nichts über die Steuerverbindung.
settings-radio-relays = Relais
settings-radio-receive-attenuator = Empfangs-Dämpfungsglied
settings-radio-notch-filter = Kerbfilter
settings-radio-fan = Lüfter
settings-radio-worth-having-any-sustained-transmitting = Lohnt sich für jedes längere Senden eingeschaltet.
settings-radio-automatic-board-receives-switched-transmit = Auf Automatisch empfängt die Karte und wird beim Niederdrücken auf Senden und beim Loslassen zurückgeschaltet — bei beiden Verkabelungen. Ihre Amateurkanäle haben ein Filter mit einem Sende-/Empfangsschalter auf jeder Seite, daher würde eine Karte, die beides gleichzeitig verlangt, diesen Schalter auf den Sender legen und nichts mehr hören. Der 30-MHz-Kanal ist nur über J5 erreichbar, einen Anschluss für beide Richtungen; darüber hält das Senden von J4 den Empfangsweg von dem Anschluss fern, den der Verstärker ansteuert.
settings-radio-limesdr-support-has-not-been = Die Unterstützung des LimeSDR ist nicht an der Hardware geprüft. Dies ist die Aufzeichnung der letzten Sitzung, für einen Problembericht.
settings-radio-gains-filters-corrections-antenna-sockets = Verstärkungen, Filter, Korrekturen, die Antennenbuchsen und jeder LimeRFE-Regler — seine Anschlüsse, Band, Relais, Dämpfungsglied, Kerbe und Lüfter — wirken sofort. Die Karte, die Empfangskette, die Abtastrate, das Scharfschalten des Sendens und die Verbindung des LimeRFE wirken nach Übernehmen.

## Settings window, Radio tab: messages with values

settings-radio-fill-from = VON { $name } ÜBERNEHMEN
settings-radio-decimation-auto = Automatisch (nächstliegend { $khz } kHz)
settings-radio-decimation-stage-server = Stufe { $stage } — die Rate des Servers ÷ { $divisor }
settings-radio-decimation-stage = Stufe { $stage } — Rate ÷ { $divisor }
settings-radio-kiwi-receive-only = Nur Empfang — dies ist die Antenne eines anderen. Das Panorama ist die etwa 12 kHz, die der Empfänger als I/Q sendet und die der Skala folgen; die Bandansicht darüber ist sein eigener Wasserfall, und Abstimmen über ihn stimmt den Empfänger neu ab. Etwa { $rate } kB/s, solange verbunden. „Übernehmen / erneut verbinden“ drücken, um ohne Neustart umzuschalten.
settings-radio-connected-detail = Verbunden: { $detail }
settings-radio-failed = Fehlgeschlagen: { $error }
settings-radio-pluto-buffer = ≈ { $ms } ms, { $kib } KiB je Übertragung
settings-radio-driver-settings = Einstellungen von { $driver }
settings-radio-rx888-coverage = 0–{ $mhz } MHz Abdeckung, { $mbytes } MB/s über USB
settings-radio-rx888-digitised = von den { $mhz } MHz digitalisiert
settings-radio-elad-16-bit = { $khz } kHz (16-Bit-Abtastwerte)
settings-radio-elad-pad = { $db } dB Dämpfung ein
settings-radio-hackrf-rx-preamp = { $db } dB Vorverstärker beim Empfang
settings-radio-hackrf-tx-preamp = { $db } dB Vorverstärker beim Senden
settings-radio-device-not-found = { $device } (nicht gefunden)
settings-radio-lime-chain = Kette { $n } (RX{ $n }_*)
settings-radio-lime-chain-tx = Kette { $n } (RX{ $n }_* / TX{ $n }_*)
settings-radio-lime-second-chain = Die andere Empfangskette der Karte, an den Buchsen { $sockets }. Sie teilt sich den Synthesizer, daher hört sie dieselbe Spanne im selben Augenblick wie die erste — was es ihr erlaubt, eine zweite Antenne zu führen oder eine Probe Ihres eigenen Senders.
settings-radio-tx-blocked = Senden ist blockiert: { $reason }
settings-radio-rfe-rx-fallback = Empfang auf { $port }; diese fallen auf den ungefilterten Breitbandweg zurück: { $bands }.
settings-radio-rfe-tx-fallback = Senden auf { $port }; diese fallen auf den Breitbandweg zurück — kein Bandverstärker und keine Filterung: { $bands }.

## Top bar and control strip (src/app/top_bar.rs)

topbar-how-far-one-press-moves = Wie weit ein Druck die Skala bewegt. Antippen, um den nächsten Schritt zu nehmen: 10 Hz, 100 Hz, 500 Hz, 1, 2,5, 5, 9, 10 und 25 kHz. Die Zeile in den Einstellungen › Oberfläche ausschalten.
topbar-volume-gain-agc-squelch-noise = Lautstärke, Verstärkung, AGC, Rauschsperre, die Rauschregler und die RDS-Anzeige auf UKW-Rundfunk
topbar-receiver = EMPFÄNGER
topbar-vfo-b-split-rit-xit = VFO A/B, Split und die RIT-/XIT-Versätze
topbar-tuning = ABSTIMMUNG
topbar-diversity-filter-which-way-combines = Der Diversitätsfilter: in welcher Weise er die beiden Antennen kombiniert, wie schnell er sich anpasst und ihn an seiner Stelle halten
topbar-diversity = DIVERSITÄT
topbar-second-receiver-s-frequency-mode = Die Frequenz, Betriebsart, das Filter und der Pegel des zweiten Empfängers
topbar-sub-receiver = SUBEMPFÄNGER
topbar-transceiver-s-own-aerial-socket = Die eigene Antennenbuchse des Transceivers, sein Antennentuner und SWR, und sein Netzschalter
topbar-radio = FUNKGERÄT
topbar-tune-voice-keyer-drive-mic = Tune, der Sprachkeyer und die Pegel für Aussteuerung und Mikrofon
topbar-transmit = SENDEN
topbar-tune = {" "}TUNE{" "}
topbar-panadapter-two-layers-their-speeds = Das Panorama — seine beiden Ebenen, ihre Geschwindigkeiten und Detailtiefe, Spitzenwert-Halten — plus Wasserfall-Kontrast, FFT-Größe und die Skimmer
topbar-display = ANZEIGE
topbar-logbook-spots-awards-memories-scanner = Logbuch, Spots, Auszeichnungen, Speicher, der Scanner, Einstellungen und das Handbuch
topbar-system = SYSTEM
topbar-linked = Verbunden
topbar-not-linked = Nicht verbunden
topbar-bar-trace = Balken / Spur
topbar-needle-bar-trace = Zeiger / Balken / Spur
topbar-swap-vfos = VFOs tauschen
topbar-copy-b = A nach B kopieren
topbar-second-receiver-right-ear-tunes = Zweiter Empfänger, im rechten Ohr. Er stimmt unabhängig von A/B ab — seine Regler erscheinen im Modul SUB und sein Durchlassbereich im Wasserfall.
topbar-band-plan = , aus dem Bandplan
topbar-repeater-shift-following-band-plan = Relais-Versatz: folgt dem Bandplan, der für diese Frequenz keinen Versatz hat — Simplex
topbar-repeater-shift-transmit-above-below = Relais-Versatz: über oder unter der Skala senden, um mit einem Relais zu arbeiten. Jetzt Simplex
topbar-repeater-shift = RELAIS-VERSATZ
topbar-auto = AUTO
topbar-take-shift-band-plan-dial = Den Versatz aus dem Bandplan übernehmen, während sich die Skala bewegt. Er gilt nur innerhalb eines Relais-Ausgangsteilbands — überall sonst lässt er das Funkgerät auf Simplex, daher bleiben die Anrufkanäle Simplex.
topbar-offset = Versatz
topbar-repeater-tone-ctcss-dcs-goes = Relaiston: der CTCSS/DCS, der unter der Stimme gesendet wird, der 1750-Hz-Burst und die Empfangs-Tonsperre
topbar-transmit-tone = SENDETON
topbar-set-here-sent-nfm-sub = Hier eingestellt, auf NFM gesendet — die Untertonsignalisierung gehört zu einem FM-Kanal
topbar-normal = NORMAL
topbar-invert = INVERTIERT
topbar-bit-order-dcs-encoded-here = Die Bitfolge, mit der DCS hier kodiert wird, ist dem Standard entnommen und nie gegen ein Relais geprüft worden — öffnet es nicht, CTCSS probieren
topbar-t-1750-hz-burst = 1750-HZ-BURST
topbar-every-over = JEDE AUSSENDUNG
topbar-open-burst-every-time-transmitter = Mit dem Burst öffnen, jedes Mal wenn der Sender tastet
topbar-send = SENDEN
topbar-send-burst-now-keying-transmitter = Den Burst jetzt senden — tastet den Sender für seine Länge, falls er nicht schon getastet ist
topbar-nfm-only-burst-fm-repeater = Nur NFM: Der Burst ist der Türöffner eines FM-Relais
topbar-receive-tone-squelch = EMPFANGS-TONSPERRE
topbar-require-receive-what-station-transmits = Beim Empfang verlangen, was diese Station sendet
topbar-hide-docked-band-selector-click = Den angedockten Bandwähler ausblenden — erneut klicken, um ihn zurückzuholen
topbar-show-docked-band-selector = Den angedockten Bandwähler anzeigen
topbar-dock = ANDOCKEN
topbar-keep-band-mode-selector-open = Den Band- und Betriebsartwähler neben dem Wasserfall geöffnet halten, statt dieses Popup jedes Mal zu schließen
topbar-band-mode = BAND & BETRIEBSART
topbar-hide-band-chip-brings-back = Ausblenden — der Band-Chip holt ihn zurück
topbar-undock = ABDOCKEN
topbar-return-selector-top-bar-popup = Den Wähler zurück in das Popup der oberen Leiste legen
topbar-dec-off = DEC aus
topbar-vol = Lautst.
topbar-counted-steps-not-decibels-what =
    {""}

    In Schritten gezählt, nicht in Dezibel: Was ein Schritt wert ist, ist die eigene Sache des Empfängers, und bei einem SDRplay hängt es außerdem vom Band ab. Richtig ist weiterhin mehr Verstärkung.
topbar-gain = Verstärkung
topbar-plutosdr-s-own-agc-setting =
    {""}

    Die eigene AGC des PlutoSDR stellt diese Verstärkung ein, daher bewirkt das Verschieben des Reglers nichts. Einstellungen → Funkgerät → AGC auf Manuell setzen, um sie hier zu steuern.
topbar-agc-hang-time-click-cycle = AGC-Haltezeit — klicken zum Wechseln: Aus / Langsam / Mittel / Schnell
topbar-radio-s-own-agc-click = Die eigene AGC des Funkgeräts — klicken zum Wechseln: Langsam / Mittel / Schnell. Von hier aus lässt sie sich nicht ausschalten.
topbar-man = Man
topbar-manual-audio-gain-used-while = Manuelle Audioverstärkung, genutzt, solange die AGC aus ist. Vorbelegt mit dem Pegel, den die AGC hielt, als sie ausgeschaltet wurde.
topbar-radio-s-own-squelch-sent = Die eigene Rauschsperre des Funkgeräts, über die Steuerverbindung gesendet. Dies ist das Gate, durch das das Audio tatsächlich geht — die Software-Sperre würde nur weiter auf dem schließen, was das Gerät schon durchgelassen hat.
topbar-max-gain = max. Verstärkung
topbar-manual-gain = manuelle Verstärkung
topbar-noise-reduction = Rauschunterdrückung
topbar-impulse-noise-blanker = Impulsrauschbegrenzer
topbar-auto-notch-cancel-constant-tone = Auto-Kerbe: gleichbleibende Tonanteile auslöschen (Überlagerungen)
topbar-noise-reduction-off-click-switch = Rauschunterdrückung aus — klicken, um sie einzuschalten, oder unten in den Zeilen eine Engine wählen
topbar-binaural-audio-not-while-sub = Binaurales Audio — nicht, solange der Subempfänger das rechte Ohr hat
topbar-binaural-audio-passband-spread-across = Binaurales Audio: Der Durchlassbereich ist auf beide Ohren verteilt, daher kommen Signale auf verschiedenen Tonhöhen aus verschiedenen Richtungen, und das, das Sie abstimmen, wandert mit. Klicken, um zu Mono zurückzukehren
topbar-binaural-cw-spread-passband-across = Binaurales CW: den Durchlassbereich auf beide Ohren verteilen, damit Signale auf verschiedenen Tonhöhen aus verschiedenen Richtungen kommen — ein Pile-up wird zu mehreren Orten statt eines überfüllten Tons, und das Abstimmen einer Station lässt sie mitwandern. Am besten mit Kopfhörern
topbar-binaural-audio-spread-passband-across = Binaurales Audio: den Durchlassbereich auf beide Ohren verteilen. Bei Sprache verteilt sich das Rauschen über das ganze Bild, während die Station in der Mitte bleibt, was sich eine Stunde lang angenehmer hören lässt — auf Kosten der Stimme selbst, die auseinandergezogen wird. Am besten mit Kopfhörern
topbar-mute = STUMM
topbar-record-audio-raw-i-q = Das Audio, das rohe I/Q oder beides aufnehmen
topbar-wfm-stereo-forced-off-click = WFM-Stereo erzwungen aus — klicken für automatisches Stereo
topbar-wfm-stereo-pilot-locked-click = WFM-Stereo: Pilotton rastet ein. Klicken, um auf Mono zu erzwingen
topbar-wfm-stereo-automatic-no-pilot = WFM-Stereo: automatisch, kein Pilotton bei dieser Station
topbar-rds-data-arriving-click-see = RDS-Daten kommen an — klicken, um sie zu sehen
topbar-rds-no-data-station-click = RDS — keine Daten bei dieser Station. Trotzdem klicken für die Diagnose des Decoders
topbar-drm-decoding-click-details = DRM wird dekodiert — klicken für Einzelheiten
topbar-drm-click-decoder-s-state = DRM — klicken für den Zustand des Decoders
topbar-hd-radio-decoding-click-details = HD Radio wird dekodiert — klicken für Einzelheiten
topbar-hd-radio-click-decoder-s = HD Radio — klicken für den Zustand des Decoders
topbar-presets = VOREINSTELLUNGEN
topbar-preset-right-click = Rechtsklick, um diese Voreinstellung zu ändern
topbar-preset-edit = VOREINSTELLUNG BEARBEITEN
topbar-preset-label = Beschriftung
topbar-preset-label-hint = Was auf dem Chip steht. Bis zu 8 Zeichen.
topbar-preset-lo = Unten
topbar-preset-hi = Oben
topbar-preset-use-current = Aktuelle übernehmen
topbar-preset-use-current-hint = Die Kanten des Filters übernehmen, den der Empfänger gerade hat
topbar-preset-save = Speichern
topbar-preset-reset = Zurücksetzen
topbar-preset-reset-hint = Die eingebaute Voreinstellung dieser Betriebsart wiederherstellen
topbar-preset-cancel = Abbrechen
topbar-width-per-sideband = BREITE JE SEITENBAND
topbar-width = BREITE
topbar-passband-width-hertz-type-drag = Durchlassbereich in Hertz — eintippen oder ziehen. In das Feld klicken, um einen genauen Wert einzugeben.
topbar-edges = GRENZEN
topbar-low-edge-hz-carrier = Untere Grenze, in Hz vom Träger
topbar-high-edge-hz-carrier = Obere Grenze, in Hz vom Träger
topbar-noise-reduction-voice-off-click = Rauschunterdrückung (Sprache), aus — klicken, um eine Engine zu wählen
topbar-record-audio = AUDIO AUFNEHMEN
topbar-mp3 = ● MP3
topbar-what-you-hear-mp3-receiver = Was Sie hören, als MP3: der Empfänger auf einem Kanal und Ihr eigenes Senden auf dem anderen. Das ist die Aufnahme eines QSO.
topbar-mono = MONO
topbar-mixed-one-channel-click-two = Auf einen Kanal gemischt — klicken für zwei
topbar-two-channels-rx-left-tx = Zwei Kanäle: RX links, TX rechts — klicken für einen gemischten Kanal
topbar-quick-clip = SCHNELLCLIP
topbar-stop-after = STOPP NACH
topbar-no-stop = kein Stopp
topbar-auto-record = AUTOMATISCH AUFNEHMEN
topbar-set-squelch-auto-record-needs = Eine Rauschsperre setzen — die automatische Aufnahme braucht eine, um Stille zu erkennen
topbar-auto-record-needs-sdr-front = Die automatische Aufnahme braucht eine SDR-Frontstufe mit softwareseitiger Rauschsperre
topbar-stop-following-squelch = Nicht mehr der Rauschsperre folgen
topbar-waiting-signal = warte auf ein Signal
topbar-record-spectrum = SPEKTRUM AUFNEHMEN
topbar-i-q-wav = ● I/Q-WAV
topbar-i-q-wav-2 = I/Q-WAV
topbar-raw-spectrum-receiver-delivering-stereo = Das rohe Spektrum, das der Empfänger liefert, als Stereo-32-Bit-Float-WAV (RF64 über 4 GB), das SDR#, SDRuno, HDSDR und SDRangel öffnen — und das sdroxide selbst mit --file wiedergibt. Groß: 8 Byte je Abtastwert.
topbar-radio-hands-over-demodulated-audio = Dieses Funkgerät gibt demoduliertes Audio aus, daher gibt es kein I/Q zum Aufnehmen.
topbar-what-capture-costs-sample-rate = Was eine Aufnahme bei dieser Abtastrate kostet
topbar-engine = ENGINE
topbar-no-noise-reduction-decoders-never = Keine Rauschunterdrückung — die Decoder haben sie ohnehin nie gesehen
topbar-strength = STÄRKE
topbar-nr-filters = FILTER
topbar-sk-filter-hint = SK-Nachfilter verfeinert neuronale oder andere Rauschentfernung: bestraft Restrauschen
    (Filter resoniert, dadurch Echo) · Filter von HDZ
topbar-sk-filter-tau-hint = Zeitkonstante tau: lang = Filterkurve, kurz (0,1 s) = auch Silbentor
topbar-sk-filter-open = auf
topbar-sk-filter-close = zu
topbar-sk-filter-open-hint = Öffnen: wie schnell ein Bin aufgeht, wenn dort gesprochen wird
topbar-sk-filter-close-hint = Schließen: wie schnell es danach zugeht; kurz = weniger Nachhall
topbar-sk-filter-width = Breite
topbar-sk-filter-width-hint = Glättung der Maske, ±Bins à 47 Hz; breiter = weniger Klingeln, gröber
topbar-sk-filter-after-nr = nach NR
topbar-sk-filter-after-nr-hint = SK hinter RNN/DFNR statt vor der AGC (Variante B)
topbar-sk-filter-caption = SK-NACHFILTER
topbar-sk-filter-reset-hint = SK auf Grundwerte: τ 0,1 s, auf 0,01 s, zu 1 s, Breite ±1, vor NR
topbar-sk-filter-before-nr = vor NR
topbar-sk-preset-hint = Klick lädt, Rechtsklick speichert die aktuellen Werte
    { $values }
topbar-hum-filter-hint = Hochpass 300 Hz: Netzbrumm 50/60 Hz und Oberwellen
topbar-tone = TON
topbar-ctcss-dcs-no-sub-audible = CTCSS / DCS — kein Unterton auf diesem Signal
topbar-tone-squelch = TONSPERRE
topbar-carrier-squelch-open-any-signal = Trägersperre: öffnet bei jedem Signal
topbar-require-tone-currently-being-received = Den Ton verlangen, der gerade empfangen wird
topbar-ctcss = CTCSS
topbar-any-dcs = JEDER DCS
topbar-open-any-dcs-coded-signal = Öffnet bei jedem DCS-kodierten Signal. Welchen der 104 Codes es trägt, lässt sich hier nicht zuverlässig lesen, daher gibt es nichts Feineres zu wählen
topbar-which-socket-back-radio-receiving =
    Welche Buchse auf der Rückseite das Funkgerät zum Empfangen nutzt — der eigene Befehl ANT, dieselbe Einstellung wie die Taste ANT an der Frontplatte. Anklicken, um zur nächsten zu wechseln.

    Die Wahl wird je Band gemerkt und beim nächsten Überschreiten der Bandgrenze wiederhergestellt.
topbar-radio-s-separate-receiving-antenna =
    Die eigene Empfangsantenne des Funkgeräts, in den Empfangsweg geschaltet oder daraus entfernt — die eigene Einstellung RX ANT. Die Antenne an der Hauptbuchse bleibt in beiden Fällen beim Senden.

    Das Funkgerät merkt sich dies selbst je Band, daher liest sdroxide es nach jedem Bandwechsel zurück, statt zurückzusetzen, was es zuletzt gesehen hat: Nur ein Klick hier bewegt es.
topbar-radio-s-own-power-switch =
    Der eigene Netzschalter des Funkgeräts über die Steuerverbindung und der einzige echte Ein-/Aus-Schalter im Programm — nicht der Schalter LINK von sdroxide, der das sdroxide-Ende schließt und das Funkgerät laufen lässt.

    Damit EIN irgendetwas erreicht, muss das Steuerende des Funkgeräts im ausgeschalteten Zustand wach bleiben: Network Control über das LAN oder ein CI-V-Port, der bei einem vorn ausgeschalteten Gerät weiter aus dem Netz gespeist wird.
topbar-switch-radio = Das Funkgerät einschalten
topbar-switch-radio-off-audio-meters = Das Funkgerät ausschalten. Audio und Messgeräte stoppen damit.
topbar-what-second-aerial-click-swap =
    Wofür die zweite Antenne da ist — klicken, um zu tauschen.

    AUSLÖSCHEN zieht sie von der ersten ab, mit der Verstärkung, Phase und Verzögerung, die eine lokale Störquelle auf beiden Antennen gleich ausrichten: die DSP-Form eines störauslöschenden Phasenschiebers. KOMBINIEREN addiert die beiden in der Phase, die verstärkt, gewichtet danach, wie gut jede hört — Diversitätsempfang, der Schwunglöcher füllt.
topbar-hold = HALTEN
topbar-stop-filter-moving-reach-moment = Das Filter am Bewegen hindern. Danach greifen, sobald eine Auslöschung erscheint: Ein Filter, das weiter anpasst, richtet sich auf das aus, was am lautesten wird, und auf einem ruhigen Band ist das der Sender, den Sie hören.
topbar-restart = NEUSTART
topbar-zero-filter-find-null-again = Das Filter auf null setzen und die Auslöschung erneut finden.
topbar-adapt = Anpassen
topbar-how-fast-filter-chases-slow = Wie schnell das Filter nachführt: links langsam und gleichmäßig, es konvergiert innerhalb eines Bruchteils einer Sekunde und jagt rechts sichtbar. Zum Finden der Auslöschung schnell beginnen, dann HALTEN.
topbar-where-sub-receiver-listens-shift = Worauf der Subempfänger hört. Umschalt-Klick auf den Wasserfall oder Ziehen innerhalb des Durchlassbereichs des Subs, um ihn zu verschieben.
topbar-dial = ←SKALA
topbar-move-sub-receiver-main-dial = Den Subempfänger auf die Hauptskala legen
topbar-dial-2 = SKALA←
topbar-move-main-dial-sub-receiver = Die Hauptskala auf den Subempfänger legen
topbar-filter = Filter
topbar-sub-receiver-passband-edges-hz = Grenzen des Durchlassbereichs des Subempfängers in Hz
topbar-sub-receiver-level-plays-right = Pegel des Subempfängers (er spielt im rechten Ohr)
topbar-ptt = {" "}PTT{" "}
topbar-voice-keyer-record-transmit-stored = Sprachkeyer: gespeicherte Meldungen aufnehmen und senden
topbar-drive = Aussteuerung
topbar-tune-2 = Tune
topbar-tx-audio-processing-microphone-click = TX-Audio verarbeitet das Mikrofon — klicken, um es zu öffnen
topbar-click-open-tx-audio-gate = Klicken, um TX-Audio zu öffnen: Gate, Equalizer, Kompressor und Begrenzer für das Mikrofon
topbar-mic = Mic
topbar-cessb = CESSB
topbar-microphone-gain = Mikrofonverstärkung
topbar-deviation-how-far-mode-s =
    Hub: wie weit der Burst dieser Betriebsart ein Funkgerät auslenkt, das selbst moduliert. Ein FM-Sender macht den Audiopegel zu einem Frequenzhub und hat keine ALC, die ihn abfängt; daher übersteuert Vollaussteuerung auf einen Dateneingang, der für Sprache eingestellt ist, den Hub — was für einen Hörer völlig normal klingt und für niemanden dekodiert.

    Je Betriebsart gespeichert, daher landet ein für 1200-Baud-Paket eingestellter Hub nie bei FT8.
topbar-transmit-audio-how-hard-mode =
    Sendeaudio: wie stark diese Betriebsart den Modulator eines Funkgeräts aussteuert, das modulliert, was wir ihm schicken — ein CAT-Gerät über seine Soundkarte, ein FLEX, ein Icom über seinen Netzwerkport. Zurückdrehen, bis die ALC des Geräts kaum noch ausschlägt, und die Leistung am Funkgerät einstellen; eine ALC, die bei einer konstant-hüllkurvigen Digitalbetriebsart schwankt, ist, was zu Splatter führt.

    Aussteuerung ist nicht dieser Regler: Bei diesen Geräten erreicht die Aussteuerung das Leistungsregister des Geräts und berührt nie dessen Audio.

    Je Betriebsart gespeichert — FT8, RTTY, PSK und MCW behalten jeweils ihren eigenen.
topbar-tx-audio = TX-Audio
topbar-cw-psk-rtty-skimmers-decode = CW-/PSK-/RTTY-Skimmer — dekodieren Signale über das Band und markieren sie im Wasserfall
topbar-skimmers = SKIMMER
topbar-run-skimmer = Diesen Skimmer laufen lassen
topbar-minimum-snr-decoded-signal-needs = Mindest-SNR, die ein dekodiertes Signal braucht, um gemeldet zu werden
topbar-needs-wideband-iq-source = braucht eine Breitband-IQ-Quelle
topbar-cw-decoder = CW-DECODER
topbar-how-many-signals-model-reads = Wie viele Signale das Modell gleichzeitig liest. Die übrigen behalten ihre Markierung, tragen aber keinen Text.
topbar-spots = SPOTS
topbar-save-skimmer-s-spot-list = Die Spot-Liste des Skimmers in eine Datei speichern
topbar-show-hide-full-band-waterfall = Den Wasserfallstreifen für das ganze Band über dem Panorama ein-/ausblenden — alles, was dieser Empfänger auf einmal sieht
topbar-spectrum-waterfall-either-layer-both = Spektrum und Wasserfall — eine der beiden Ebenen, beide oder keine. Leuchtet, während beide angezeigt werden.
topbar-spectrum = SPEKTRUM
topbar-show-spectrum = SPEKTRUM ANZEIGEN
topbar-draw-spectrum-line-across-top = Die Spektrumlinie oben über das Panorama zeichnen. Ausgeschaltet nimmt der Wasserfall die ganze Höhe ein.
topbar-peak-hold = SPITZENWERT HALTEN
topbar-snr-guide = S/N
topbar-snr-guide-tip = S/N je Frequenz über Rauschboden, gleitend gemittelt (τ 30 s auf, 6 s ab)
    Volle Höhe = S/N TOP
topbar-caption-snr-top = S/N TOP
topbar-snr-top-tip = S/N bei voller Höhe der S/N-Linie, dB über Rauschboden
topbar-caption-snr-min = S/N MIN
topbar-snr-min-tip = Mindest-S/N für Labels und Kandidaten, dB über Rauschboden
spots-snr-only-tip = Im Blick nur Spots mit S/N ≥ S/N MIN (±2 kHz); braucht S/N und IM BLICK
topbar-snr-label-filter = LABELS NACH S/N
topbar-snr-label-filter-tip = Spot-Labels nur bei S/N ≥ S/N MIN (±2 kHz), alle 2 s, mit Einblenden; braucht S/N
topbar-snr-label-peak = MIT SPITZEN
topbar-snr-label-peak-tip = Labels von AM- und CW-Sendern nur an einer Spitze 3 dB über der Umgebung (0,6–2 kHz): keine auf Störern oder Seitenbändern
topbar-snr-label-flanks = IGNORIERE FLANKEN
topbar-snr-label-flanks-tip = Keine Labels ±10 kHz um einen Sender 15 dB über Rauschboden; sein eigenes Label bleibt
topbar-trace-highest-level-each-column = Den höchsten Pegel, den jede Spalte erreicht hat, über der Live-Linie nachzeichnen, der dann wieder abklingt — was das Band getan hat, während Sie woanders hingesehen haben
topbar-draw-spectrum-receding-surface-instead = Das Spektrum als zurückweichende Fläche statt als flache Linie zeichnen: das neueste Spektrum vorn, die davor fließen von Ihnen weg. Die letzten zwei Sekunden des Bandes als Landschaft — ein Träger, der kommt und geht, ist ein Grat statt einer zuckenden Linie. Das Raster und der Spitzenwert-Halt gehören zur flachen Linie und werden darauf nicht gezeichnet.
topbar-how-quickly-spectrum-line-follows = Wie schnell die Spektrumlinie dem Band folgt. Langsamer mittelt mehr Bilder ineinander: eine ruhigere Linie und ein schwacher Träger, der still genug steht, um ihn abzulesen. Der Wasserfall wird davon nicht berührt — diese Zeilen bekommen ohnehin jedes Bild.
topbar-waterfall = WASSERFALL
topbar-show-waterfall = WASSERFALL ANZEIGEN
topbar-draw-scrolling-waterfall-below-spectrum = Den scrollenden Wasserfall unter dem Spektrum zeichnen. Ausgeschaltet nimmt die Spektrumlinie die ganze Höhe ein.
topbar-how-fast-waterfall-scrolls-lines = Wie schnell der Wasserfall scrollt, in Zeilen pro Sekunde: Langsam 5, Mittel 28, Schnell 56, Schneller 112, Am schnellsten 224. Die Engine taktet diese selbst, daher sind die beiden schnellsten echte Detailtiefe und nicht dieselbe Zeile zweimal gezeichnet — soweit der Empfänger sie liefern kann: Eine Zeile kann nie mehr als eine Transformation zeigen, und eine schmale Frontstufe liefert nur ein paar Dutzend pro Sekunde. Sie kosten Verlauf, da der Wasserfall eine feste Zahl Zeilen hält — 73 Sekunden bei Mittel, 9 bei Am schnellsten.
topbar-decode-labels = DEKODIERUNGS-BESCHRIFTUNGEN
topbar-mark-every-decoded-station-waterfall = Jede dekodierte Station im Wasserfall mit ihrem Rufzeichen markieren, bei der Frequenz, auf der sie gehört wurde — FT8, FT4 und die anderen zeitgeschlitzten Betriebsarten. Ein guter Anfang setzt dreißig davon zweimal pro Minute über die Spanne, über die Spuren, die Sie lesen; ausschalten, und der Wasserfall ist nur das Band. Die Dekodierliste daneben hat noch jedes Rufzeichen.
topbar-switch-3d-set = 3D einschalten, um dies einzustellen
topbar-how-3d-spectrum-drawn-only = Wie das 3D-Spektrum gezeichnet wird. Nur die Form unterscheidet sich — beide Darstellungen zeigen dieselben Spektren, und beide verdecken, was dahinter liegt, daher steht ein starkes Signal vorn über dem Band, das es überdeckt.
topbar-lines = LINIEN
topbar-one-trace-per-remembered-spectrum = Eine Spur je gemerktem Spektrum, in der eigenen Farbe der flachen Linie, und jede verdeckt die dahinter. Die Form ohne die Pegel, und die Darstellung, die man wählt, wenn es um die Form geht — sie kostet etwas mehr als die volle, statt weniger, weil sie dieselbe Fläche zeichnet und dann jeden Kamm obendrauf nachzieht.
topbar-solid = VOLL
topbar-filled-surface-coloured-waterfall-s = Eine gefüllte Fläche in der Palette des Wasserfalls eingefärbt, sodass der Pegel in der Farbe ebenso steckt wie in der Höhe, und die beiden Hälften des Panoramas sich einig sind, wie ein starkes Signal aussieht. Die Palette ändern unter Einstellungen › Anzeige.
topbar-how-fast-3d-spectrum-flows = Wie schnell das 3D-Spektrum von Ihnen weg fließt, in Zeilen pro Sekunde: Langsam 6, Mittel 12, Schnell 24, Schneller 48. Die Fläche ist eine feste Zahl Zeilen tief, daher bestimmt dies auch, wie viel Zeit sie hält. Langsamer ist ein längeres Gedächtnis und eine Fläche, die kriecht; schneller ist ein kürzeres, das sich bewegt. Der Wasserfall behält seine eigene Scrollrate, unten.
topbar-how-many-columns-panadapter-waterfall = Wie viele Spalten das Panorama und sein Wasserfall gezeichnet werden. Auto liest den Renderer dieses Rechners und die Größe des Panoramas und wählt das Meiste, was er tragen kann — ein 4K-Bildschirm will 4096. Jede Spalte ist ein Byte in jedem Bild, daher zahlt ein Client, der mit einer Station aus der Ferne verbunden ist, für die Detailtiefe auf seiner Leitung: 4096 Spalten bei 60 fps sind etwa ein Viertel Megabyte pro Sekunde, das Doppelte der Standardbreite.
topbar-read-machine-screen-take-most = Diesen Rechner und diesen Bildschirm lesen, das Meiste nehmen, was sie tragen können, und ihnen folgen, wenn sie sich ändern
topbar-width-force-right-now-whatever = Die Breite, die jetzt gilt, was auch immer die Zeile oben verlangt
topbar-keep-floor-ceiling-set-best = Den Boden/die Decke für den besten Wasserfall-Kontrast eingestellt halten — wird bei einem Bandwechsel, nach einem Schwenken oder Zoomen und wenn die Pegel wandern sanft zurückgesetzt. Einschalten, um sofort anzupassen; ausschalten, um die Pegel dort zu lassen, wo Sie sie gesetzt haben.
topbar-keep-tuned-frequency-middle-panadapter = Die abgestimmte Frequenz in der Mitte des Panoramas halten: Das Fenster gleitet unter der Skala, statt dass das Bild um eine ganze Spanne springt, wenn Sie am Rand abstimmen. Einschalten, um sofort zu zentrieren; ausschalten, um frei zu schwenken und zu zoomen.
topbar-spectrum-floor-ceiling-fft-size = Boden/Decke des Spektrums und FFT-Größe
topbar-level-drawn-bottom-spectrum-darkest = Der Pegel, der unten im Spektrum gezeichnet wird und als dunkelste Wasserfall-Farbe dient. Hochdrehen, bis der Rauschboden gerade dunkler wird. Ihn zu setzen schaltet ANPASSEN aus.
topbar-ceil = Decke{" "}
topbar-level-drawn-top-bring-down = Der Pegel, der oben gezeichnet wird. Herunterdrehen, bis das stärkste Signal, das Sie interessiert, volle Farbe erreicht. Ihn zu setzen schaltet ANPASSEN aus.
topbar-fft-size = FFT-GRÖSSE
topbar-flip = UMDREHEN
topbar-scroll-waterfall-upwards-newest-row = Den Wasserfall nach oben scrollen — neueste Zeile unten (V)
topbar-logbook-all-qsos-digital-manual = Logbuch — alle QSOs (digital + manuell)
topbar-live-spots-dx-cluster-pota = Live-Spots — DX-Cluster, POTA, SOTA, PSK Reporter
topbar-award-tracking-dxcc-was-waz = Auszeichnungen — DXCC / WAS / WAZ / Locatoren
topbar-band-conditions-published-forecast-beside = Bandbedingungen — die veröffentlichte Vorhersage neben dem, was tatsächlich auf jedem Band gehört wurde
topbar-satellite-qo-100-beacon-calibration = Satellit — die QO-100-Baken-Kalibrierung läuft
topbar-satellite-doppler-tracking-qo-100 = Satellit — Doppler-Verfolgung und die QO-100-Baken-Kalibrierung
topbar-ism-band-devices-decoding-now = Geräte im ISM-Band — dekodieren jetzt, ob dieses Fenster offen ist oder nicht. Mit DEKODIEREN im Fenster ausschalten.
topbar-ism-band-devices-weather-sensors = Geräte im ISM-Band — Wettersensoren, Messgeräte und Hausautomation, die um Sie herum gehört werden
topbar-public-sdrs-internet-browse-kiwisdr = Öffentliche SDRs im Internet — die KiwiSDR- und SpyServer-Verzeichnisse durchsuchen und einen als Funkgerät öffnen
topbar-winlink-radio-email = Winlink-Funk-E-Mail
topbar-memory-channels = Speicherkanäle
topbar-scanner-stopped-signal = Scanner — bei einem Signal angehalten
topbar-scanner-running = Scanner — läuft
topbar-scan-memory-channels-frequency-range = Speicherkanäle oder einen Frequenzbereich absuchen
topbar-hfdl-ground-network-decode-log = HFDL-Bodennetz — das Dekodierprotokoll und die Flugzeugkarte unterhalb des Wasserfalls. Den Dekodierer mit HÖREN im Panel einschalten.
topbar-hfdl-ground-network-decoding-now = HFDL-Bodennetz — dekodiert jetzt. Das Panel öffnen.
topbar-hfdl-ground-network-aircraft-shortwave = HFDL-Bodennetz — die Kurzwellen-Datenverbindung zu Flugzeugen, jeweils ein Hörkanal
topbar-grid-tracker-worked-maidenhead-squares = Locator-Tracker — gearbeitete Maidenhead-Quadrate auf einer Karte, mit dem, was gehört wird
topbar-amplifier-set-up-hardrock-under = Verstärker — einen Hardrock unter Einstellungen → Verstärker einrichten
topbar-settings-device-gains-antennas-audio = Einstellungen — Gerätverstärkungen, Antennen, Audiogeräte
topbar-user-manual-f1 = Benutzerhandbuch (F1)
topbar-range-was-typed-settings-radio = Dieser Bereich wurde in Einstellungen ▸ Funkgerät ▸ RX-Bereich eingetragen, nicht vom Funkgerät gemeldet. Erweitern Sie ihn oder leeren Sie das Feld, um zu verwenden, was das Gerät über sich selbst sagt.
topbar-what-device-reports-about-itself = Das ist, was das Gerät über sich selbst meldet. Wenn das Funkgerät dieses Band tatsächlich abdeckt, dessen echten Bereich unter Einstellungen ▸ Funkgerät ▸ RX-Bereich angeben.
topbar-band = BAND
topbar-recent = ZULETZT
topbar-recent-empty =
    Füllt sich nach 3 s auf einem Platz
topbar-mode = BETRIEBSART
topbar-digital = DIGITAL

## Top bar: the System box's window chips.

topbar-chip-log = LOG
topbar-chip-spots = SPOTS
topbar-chip-awards = AUSZEICHNUNGEN
topbar-chip-bands = BÄNDER
topbar-chip-sat = SAT
topbar-chip-ism = ISM
topbar-chip-public-sdr = ÖFFENTL. SDR
topbar-chip-mail = MAIL
topbar-chip-mem = SPEICHER
topbar-chip-scan = SCAN
topbar-chip-hfdl = HFDL
topbar-chip-grid = LOCATOR
topbar-chip-amp = VERST.
topbar-chip-settings = ⚙ EINSTELLUNGEN
topbar-chip-help = ? HILFE

## Top bar: the mode-defaults chip.

topbar-changed-agc = AGC
topbar-changed-squelch = Rauschsperre
topbar-changed-auto-notch = Auto-Kerbe
topbar-changed-stereo = Stereo
topbar-changed-binaural = binaural
# $mode is a mode name (USB, CW…); $changed lists what differs from its defaults.
topbar-mode-defaults =
    Zurück zu den Standardwerten von { $mode }. Geändert gegenüber diesen: { $changed }.

    Klicken, um sie zurückzusetzen. Die eigenen Werte von { $mode } kehren zurück, und was Sie hier eingestellt haben, wird vergessen.

## Settings window, Radio tab: longer messages with values

settings-radio-no-such-baud = das Funkgerät hat keine { $baud }-Baud-Einstellung — stattdessen wird { $default } verwendet
settings-radio-icom-scope-hint =
    Streamt den eigenen Scope-Durchlauf des Funkgeräts über die CI-V-Verbindung und zeichnet ihn als Panorama — das einzige Bild des Bandes, das ein Funkgerät mit demodulierten Audio geben kann, da das Audio, das es sendet, bereits durch seinen Filter gekommen ist.

    Das braucht ein entsprechend eingerichtetes Funkgerät: CI-V-USB-Baudrate auf 115200 und CI-V-USB-Port auf „Unlink from {"["}REMOTE]“ (beides unter SET > Connectors > CI-V), und die Baudrate oben passend gesetzt — unterhalb von { $min_baud } passen die Durchläufe nicht durch die Verbindung, und das Scope bleibt aus.

    Die Durchläufe teilen sich den internen USB-Bus des Funkgeräts mit seiner eigenen Soundkarte. Wenn empfangenes Audio bei eingeschaltetem Scope auszusetzen beginnt, ist dieses Feld das Erste, was man ausschalten sollte.
settings-radio-rx-tx-words = { $rx } / TX { $tx }
settings-radio-soapy-native = Die Hardware oben wird von sdroxide direkt unterstützt: { $names }. Diese Schnittstelle oben zu wählen gibt ihre eigenen Einstellungen — Verstärkungsstufen, Filter und Kerben, die SoapySDR nicht ausdrücken kann — und braucht kein SoapySDR-Modul.
settings-radio-rx888-width-hint = Wie viel des digitalisierten Spektrums das Panorama auf einmal zeigt. Die ganze DSP-Kette läuft mit dieser Breite, daher kostet eine breitere Einstellung anteilig mehr CPU — 1/2 ist das ganze Band im Wasserfall, und eine ernsthafte Menge Rechenarbeit. Oberhalb der UKW-Schwelle ({ $crossover } MHz) schaut das Panorama nicht auf die Antenne, sondern auf die ZF des Tuners, die 8 MHz breit ist und bei { $if_centre } MHz liegt — daher lässt sich alles, was breiter als { $widest } ist, nicht darauf zentrieren, und die zusätzliche Breite ist totes Spektrum neben dem Signal statt mehr davon.
settings-radio-sdrplay-serial-missing = Die Seriennummer { $serial } ist nicht unter den Empfängern, die der SDRplay-Dienst meldet. Einen der aufgelisteten Empfänger wählen oder diesen neu einstecken und Erneut suchen drücken.
settings-radio-hdr-filter-hint =
    Das Analogfilter vor dem HDR-Pfad — eine andere Regelung als die eigene Bandbreite des Empfängers. Jede Einstellung ist auf eine feste Menge von Mitten gebaut und kann anderswo nichts bewirken; dieses hier ist auf { $centres } gebaut. Wirkt sofort.

    Der Pfad selbst funktioniert noch nicht — siehe den Schalter oben —, daher wählt dies ein Filter für eine Betriebsart, die gerade still ist.
settings-radio-rspduo-only = Nur ein RSPduo hat einen zweiten Tuner, und der hier gewählte Empfänger ist ein { $model }, daher bewirkt dies nichts.
settings-radio-lime-tx-gain-low = Die Sendeverstärkung steht auf { $db } dB, am unteren Ende ihres Bereichs — ein paar Mikrowatt aus der Karte, was auf einem Leistungsmessgerät als nichts erscheint, was auch immer dahinter liegt. Das ist der Standard, damit ein scharfgeschalteter Sender Sie nicht überraschen kann; vor dem Tasten hochdrehen.

## Settings window, General: SWR

settings-general-swr-tune-limit = (Abstimmen: { $limit }:1)

## Top bar and control strip: messages with values

# $reading is the meter's own hover text; $faces lists the faces a click cycles through.
topbar-smeter-hover =
    { $reading }

    Klicken, um die Messgerät-Anzeige zu wechseln: { $faces }
topbar-step-down = Runter { $step }
topbar-step-up = Hoch { $step }
# $auto is topbar-band-plan or empty.
topbar-repeater-shift-hover = Relais-Versatz { $shift } — empfängt auf { $rx }, sendet auf { $tx } MHz{ $auto }
topbar-tone-and-burst = Sendet { $tone } unter der Stimme und einen { $ms} ms langen 1750-Hz-Burst zu Beginn jeder Aussendung
topbar-tone-under-voice = Sendet { $tone } unter der Stimme
topbar-burst-every-over = Ein { $ms } ms langer 1750-Hz-Burst zu Beginn jeder Aussendung
topbar-match-tx = PASSEND ZUM SENDEN ({ $tone })
topbar-decimation-on = Dezimierung der Frontstufe: Dem Empfänger wird die mittlere 1/{ $n } der { $msps } Msps gegeben, die dieses Funkgerät streamt — { $span } — und der Rest wird verworfen, bevor etwas davon den Empfänger erreicht.
topbar-decimation-off = Dezimierung der Frontstufe, aus: Der Empfänger sieht die ganzen { $span }, die dieses Funkgerät streamt.
topbar-decimation-hint =
    { $what }

    Eine schmalere Spanne bedeutet feinere Auflösung im Wasserfall, einen ruhigeren Rauschboden (3 dB je Halbierung) und weniger CPU — auf Kosten des Bandes zu beiden Seiten. Die Skala stimmt weiterhin überall ab; das Funkgerät verschiebt seinen LO, um zu folgen.

    Klicken zum Wechseln: aus / 2 / 4 … { $max }.
topbar-rx-gain-hint = Verstärkung der Frontstufe beim Empfang ({ $stage }). Zu viel übersteuert den ADC des Empfängers und verschmiert Störsignale über das Band; zu wenig, und er wird taub.
topbar-rx-gain-stages =
    {""}

    Dieses Gerät hat { $n } Verstärkungsstufen beim Empfang — die übrigen stehen unter Einstellungen → Gerät.
topbar-passband-now =
    {""}

    Der Durchlassbereich liegt jetzt bei { $dbfs } dBFS.
# $level is topbar-passband-now or empty.
topbar-squelch-hint =
    Das Audio unterhalb dieser Leistung im Empfangs-Durchlassbereich sperren, in dBFS. Links ist offen.{ $level } Auf einen Wert über dem Rauschen und unter dem Signal setzen.

    Ein Strom, der mit der eigenen AGC des Funkgeräts ankommt — etwa die 12-kHz-ZF eines Icom —, liegt auf dieser Skala weit höher als die rohe Basisbandfrequenz eines SDR, und deshalb erreicht der Balken die Vollaussteuerung.
topbar-nr-cycle-hint =
    Rauschunterdrückung: { $strength } — { $engine }

    Klicken, um die Stärke zu wechseln: Niedrig / Mittel / Hoch / Aus. Die Engine steht in den Zeilen unten.
topbar-recording-both = Nehme { $audio } und { $iq } auf
topbar-recording-audio = Nehme Audio auf nach { $audio }
topbar-recording-iq = Nehme I/Q auf nach { $iq }
topbar-auto-record-armed = Automatische Aufnahme scharf: eine Datei je Aussendung, geschlossen nach { $secs } s Stille
topbar-rds-hover = RDS: { $ps }. Klicken für die Daten der Station
topbar-drm-hover = DRM: { $service }. Klicken für die Einzelheiten der Sendung
topbar-drm-no-codec = DRM: { $summary } — eingerastet, aber sein { $codec }-Audio kann hier nicht dekodiert werden. Klicken, um zu sehen, was fehlt
topbar-drm-state = DRM: { $summary } — klicken für den Zustand des Decoders
topbar-hd-hover = HD Radio: { $station }. Klicken für die Einzelheiten der Sendung
topbar-hd-state = HD Radio: { $summary } — klicken für den Zustand des Decoders
topbar-binaural-preset = { $preset } in jedem Ohr — { $lo } … { $hi } Hz, beide Seitenbänder
topbar-nr-hover =
    Rauschunterdrückung: { $strength } — { $engine }

    Klicken, um Engine oder Stärke zu ändern
topbar-clipping = Ausschnitt { $span } — erneut drücken, um von vorn zu beginnen
topbar-record-clip = { $span } aufnehmen und stoppen — ein Ausschnitt, der an einen Empfangsbericht angehängt wird
topbar-starting-clip = starte einen { $span }-Ausschnitt…
topbar-stop-recording-after = Die MP3-Aufnahme nach { $minutes } Minuten stoppen
topbar-stops-in = stoppt in { $time }
topbar-auto-record-hint = Jede Aussendung in eine eigene Datei aufnehmen, geschlossen nach { $secs } Sekunden Stille
topbar-recording-silence = nimmt auf · { $secs } s Stille (schließt um { $hold })
topbar-tone-open = Empfange { $tone }, das ist die Tonsperre — Audio offen
topbar-tone-closed = Empfange { $heard }, aber die Tonsperre verlangt { $wanted } — Audio bleibt geschlossen
topbar-tone-heard = Empfange CTCSS/DCS { $tone }
topbar-tone-nothing = Tonsperre { $tone }: nichts, was dazu passt, wird empfangen
topbar-tone-use = { $tone } NUTZEN
topbar-sockets = Buchsen: { $sockets }
topbar-voice-keyer-tx = Sendet { $slot } — klicken, um den Sprachkeyer zu öffnen
topbar-drive-uncal-gen = Diese Frequenz liegt in keinem Amateurfunkband und ist nicht kalibriert, daher erreicht die Einstellung den Sender unverändert.
topbar-drive-uncal-band = { $band } ist nicht kalibriert, daher erreicht die Einstellung den Sender unverändert.
# $trim is signed (+1.5); $diff is its size; $more is "less" or "more".
topbar-drive-cal-gen = Diese Frequenz liegt in keinem Amateurfunkband und ist um { $trim } dB kalibriert, daher bringt dieselbe Einstellung hier { $diff } dB { $more ->
        [less] weniger
       *[more] mehr
    } in die Luft als auf einem unkalibrierten Band.
topbar-drive-cal-band = { $band } ist um { $trim } dB kalibriert, daher bringt dieselbe Einstellung hier { $diff } dB { $more ->
        [less] weniger
       *[more] mehr
    } in die Luft als auf einem unkalibrierten Band.
topbar-drive-hover =
    Wie stark der Sender ausgesteuert wird. Bei einem Funkgerät, das sdroxide selbst moduliert, skaliert es die modulierten Abtastwerte; bei einem Gerät mit eigener Leistungsregelung ist es der Anteil der Nennleistung, den das Gerät verlangt bekommt.

    { $trim }

    Einstellungen → Funkgerät → Sendeaussteuerung nach Band.
topbar-cessb-hover =
    Controlled-Envelope-SSB: mehr mittlere Leistung bei gleicher Spitze, ohne Splatter — drei oder vier Dezibel scheinbarer Lautstärke am anderen Ende. Wie viele Dezibel die Stimme in den Prozessor gesteuert wird; 0 ist aus, 6 ist ein vernünftiger erster Versuch. Es berührt die Aussteuerung nicht.

    Jetzt: { $now }
topbar-waterfall-rate = { $rows } Zeilen pro Sekunde — { $seconds } Sekunden Band
topbar-detail-auto-columns = { $n } Spalten, was Auto hier gewählt hätte
topbar-n-columns = { $n } Spalten
topbar-fft-bins-whole = { $n } Bins über den ganzen Strom — eines je Spalte oder feiner
topbar-fft-bins-pooled = { $n } Bins in die { $cols } Spalten des Panoramas zusammengefasst: schärfere Signale, nicht mehr davon
topbar-sat-locked = Satellit — eingerastet auf { $name }
topbar-amp-in-line = { $model } — in der Leitung (PTT)
topbar-amp-bypassed = { $model } — überbrückt (AUS)
topbar-amp-not-answering = { $model } — antwortet nicht
topbar-filter-edges = { $lo } … { $hi } Hz vom Träger
topbar-filter-isb = Empfangsfilter: { $width } Hz in jedem Ohr ({ $edges }). Die beiden Seitenbänder von ISB sind getrennte Sendungen, daher ist die Breite die Breite eines davon.
topbar-filter-width = Empfangsfilter: { $width } Hz breit ({ $edges }).
topbar-filter-hint =
    { $what }

    Klicken für die Breite, die beiden Grenzen und die Voreinstellungen dieser Betriebsart als Zahlen — die Griffe des Panoramas setzen einen Durchlassbereich nach Augenmaß, hier wird ein genauer Wert eingegeben.
topbar-band-outside-range =
    { $band } liegt außerhalb dessen, was dieses Funkgerät empfängt ({ $ranges } MHz).

    { $why }
# $time is "day" or "night".
topbar-band-forecast = { $band }: { $verdict } ({ $time ->
        [day] tagsüber
       *[night] nachts
    }) — von HAMQSL.com aus den Sonnenindizes vorhergesagt, keine eigene Messung Ihres Weges.

## Top bar: short words and captions

topbar-squelch-open = offen
topbar-off = aus
topbar-recording = nimmt auf
topbar-sql = Sperre
topbar-stations = Stationen
topbar-caption-reaction = Reaktion
topbar-caption-scroll = Scroll
topbar-caption-flow = Fluss
topbar-caption-detail = Detail
topbar-floor = Boden

## Digital-mode panel tabs (src/app/panels/mod.rs, pane_label).

pane-aircraft = FLUGZEUGE
pane-chart = DIAGRAMM
pane-chat = CHAT
pane-decodes = DEKODIERUNGEN
pane-files = DATEIEN
pane-heard = GEHÖRT
pane-image = BILD
pane-map = KARTE
pane-messages = NACHRICHTEN
pane-monitor = MONITOR
pane-panel = PANEL
pane-qso = QSO
pane-reading = LESEN
pane-receive = EMPFANG
pane-saved = GESPEICHERT
pane-send = SENDEN
pane-spots = SPOTS
pane-stations = STATIONEN
pane-status = STATUS
pane-terminal = TERMINAL
pane-text = TEXT
pane-traffic = VERKEHR
pane-vessels = SCHIFFE
pane-wfall = WASSERFALL

## Digital-mode panels, shared (src/app/panels/mod.rs)

panels-save = {" "}SPEICHERN{" "}
panels-nothing-decoded-save = Nichts dekodiert, was sich speichern ließe
panels-clear-rx = {" "}EMPFANG LEEREN{" "}
panels-empty-receive-window-nothing-air = Das Empfangsfenster leeren. Nichts, was auf Sendung ist, hört auf.
panels-nothing-received-clear = Nichts empfangen, was sich leeren ließe
panels-save-what-panel-has-decoded = Speichert, was dieses Panel dekodiert hat, in eine Datei
panels-prop = AUSBR.
panels-shade-map-where-signals-are = Die Karte nach dem schattieren, wo Signale tatsächlich durchkommen, auf jedem Band. Jeder Empfang wird am Mittelpunkt seines Weges eingetragen — dem Stück Ionosphäre, das ihn gebeugt hat — nicht bei der fernen Station, daher ist dies eine Karte des Himmels und nicht, wo Funkamateure leben. Gebaut aus Wegen, an denen diese Station teilgenommen hat, plus — wenn das Reverse Beacon Network unter Einstellungen → Spots eingeschaltet ist — dem, was die Skimmer der Welt hören, was die Bänder abdeckt, auf denen dieses Funkgerät nicht ist.
panels-night = NACHT
panels-shade-where-sun-down-twilight = Die Stellen schattieren, wo die Sonne untergegangen ist, und die Dämmerung dazwischen, damit die graue Linie auf der Karte erscheint. Tiefe Bänder werden auf der Nachtseite lang und hohe Bänder kurz, und die Terminatorlinie selbst ist, wo der DX liegt.
panels-all-bands = ALLE BÄNDER
panels-every-band-once-one-hue = Jedes Band auf einmal, je ein Farbton — die Gesamtlage.
panels-one-band = EIN BAND
panels-one-band-cold-hot-blue = Ein Band, kalt bis heiß: blau, grün, gelb, rot.
panels-frequency-you-are-actually-working = Die Frequenz, mit der Sie tatsächlich arbeiten: die Skala plus der Tonversatz daneben. Das kommt ins Logbuch — die Skala allein liegt so weit zu tief.
panels-send-return = BEI RÜCKKEHR SENDEN
panels-freq = ⇵ FREQ
panels-nothing-agreed-mode-save-your = Für diese Betriebsart ist nichts vereinbart — speichern Sie unten Ihre eigene.
panels-forget-saved-frequency = Diese gespeicherte Frequenz vergessen
panels-dial-already-one-your-saved = Diese Skala ist schon eine Ihrer gespeicherten Frequenzen.
panels-dial-already-agreed-list-mode = Diese Skala steht schon in der vereinbarten Liste für diese Betriebsart.
panels-decode-squelch-raise-stop-decoding = Dekodier-Rauschsperre — anheben, um das Dekodieren von Rauschen zu stoppen
panels-message-repeats-long-transmit-held = Die Meldung wiederholt sich, solange Senden gehalten wird — ein Meteor fängt den Teil ein, der gerade vorbeikommt.
panels-w1abc-w9xyz-fn42 = W1ABC W9XYZ FN42
panels-tx = {"  "}TX EIN{"  "}
panels-tx-2 = {"   "}TX{"   "}
panels-call-cq = {" "}CQ RUFEN{" "}
panels-nocall = NOCALL

## TX Audio window (src/app/tx_audio.rs)

txaudio-heard-too-little-speech-talk = Zu wenig Sprache gehört — normal über die ganze Zählzeit sprechen und es erneut versuchen.
txaudio-tx-audio = TX-Audio
txaudio-mode-not-voice-mode-so = Diese Betriebsart ist keine Sprachbetriebsart, daher geht nichts, was sie sendet, durch die Kette. Hier vorgenommene Einstellungen wirken in SSB, AM und FM.
txaudio-bypass = BYPASS
txaudio-switch-every-stage-out-once = Alle Stufen auf einmal aus, einschließlich des Entrauschers, damit das Mikrofon unverarbeitet hinausgeht. Erneut klicken, um genau die Stufen wiederherzustellen, die eingeschaltet waren — der schnellste A/B-Vergleich dessen, was die Kette tut.
txaudio-denoise = ENTRAUSCHEN
txaudio-rnnoise-microphone-ahead-every-other = RNNoise auf dem Mikrofon, vor allen anderen Stufen, damit Raumgeräusche entfernt sind, bevor das Gate oder der Kompressor sie hochziehen können. Fügt etwa 10 ms Verzögerung hinzu.
txaudio-digital-voice = DIGITALE SPRACHE
txaudio-also-run-freedv-rade-digital = FreeDV-/RADE-Digitalsprache ebenfalls durch die Kette führen. Standardmäßig aus: Ein Vocoder ist auf natürlicher Sprache trainiert und dekodiert aus stark verarbeiteter Stimme womöglich schlechter, daher vor dem Verlassen darauf auf dem Band testen. TCI-Audio und die digitalen Betriebsarten gehen nie durch die Kette.
txaudio-test = Test
txaudio-rec = {" "}⏺ AUFN.{" "}
txaudio-play = {" "}▶ ABSPIELEN{" "}
txaudio-play-test-phrase-through-chain = Die Testphrase durch die Kette in der jetzigen Einstellung über Ihre Lautsprecher abspielen. Während sie läuft nachstellen und die Änderung hören. Empfangsaudio wird während des Abspielens ersetzt.
txaudio-recording = nimmt auf…
txaudio-no-phrase-yet = noch keine Phrase
txaudio-live = Live
txaudio-live-2 = {" "}🎧 LIVE{" "}
txaudio-hear-your-microphone-through-chain = Ihr Mikrofon durch die Kette hören, während Sie sprechen, ohne den Sender zu tasten. Kopfhörer benutzen: Über Lautsprecher rückkoppelt es. Sie hören sich selbst einige zehn Millisekunden verzögert, was das Sprechen mühsam macht — Pegel an den Messgeräten einstellen und den Klang mit AUFN. / ABSPIELEN beurteilen. Empfangsaudio wird ersetzt, solange es läuft; PTT drücken stoppt es.
txaudio-meters = {" "}MESSGERÄTE{" "}
txaudio-run-your-microphone-through-chain = Ihr Mikrofon nur für die Messgeräte durch die Kette laufen lassen: nichts auf den Lautsprechern, daher keine Verzögerung, gegen die man sprechen müsste, und nichts, was rückkoppelt. So stellt man Mikrofon- und Eingangsverstärkung, die Gate-Schwelle und wie stark der Kompressor arbeitet ein. Empfangsaudio läuft weiter; PTT drücken stoppt es.
txaudio-monitoring-live-use-headphones-speakers = Live-Mithören — Kopfhörer benutzen, sonst rückkoppeln die Lautsprecher ins Mikrofon.
txaudio-voice-modes-only-tci-clients = Nur Sprachbetriebsarten. TCI-Clients und die digitalen Betriebsarten umgehen die Kette; ist sie in Gebrauch, den eigenen TX-EQ und Prozessor des Funkgeräts flach lassen.
txaudio-auto-set-stopped-microphone-was = Automatisch einstellen gestoppt: Das Mikrofon wurde übernommen.
txaudio-microphone-may-clipping-before-gets = {" "}Das Mikrofon übersteuert möglicherweise, bevor es hier ankommt — seine eigene Verstärkung herunterdrehen, wenn es scharf klingt.
txaudio-level = Pegel
txaudio-mic-control-same-one-transmit = Der Regler Mic — derselbe wie auf der Sendeleiste, von allen Profilen geteilt. 0 dB in seiner Mitte, +6 dB am oberen Ende.
txaudio-mic = Mic
txaudio-gain-top-mic-microphone-stored = Verstärkung obendrauf auf Mic für dieses Mikrofon, mit dem Profil gespeichert. Die Schwellen der Voreinstellungen gehen von einer Stimme aus, die auf der Anzeige IN bei etwa -10 dBFS spitzt; Automatisch einstellen bringt sie dorthin. Bleibt unter BYPASS im Eingriff und gilt auch für digitale Sprache: Es ist Pegel, keine Verarbeitung.
txaudio-input = Eingang
txaudio-auto-set = Automatisch einstellen
txaudio-auto-set-cancelled = Automatisch einstellen abgebrochen.
txaudio-talk-now-you-would-air = Jetzt sprechen, wie Sie es auf dem Band täten…
txaudio-db = -∞ dB
txaudio-chain = Kette
txaudio-order-audio-meets-them = In der Reihenfolge, in der das Audio auf sie trifft
txaudio-switch-stage = Diese Stufe einschalten
txaudio-earlier-chain = Früher in der Kette
txaudio-later-chain = Später in der Kette
txaudio-output-limiter = Der Ausgangsbegrenzer
txaudio-output = Ausgang
txaudio-trim-limiter-always-last = Trimm und Begrenzer — immer zuletzt
txaudio-profiles = Profile…
txaudio-gate-expander = Gate / Expander
txaudio-pushes-room-noise-fan-rumble = Drückt Raumgeräusche, Lüfterbrummen und Rauschen in den Pausen nach unten. Die Schwelle knapp über Ihren Rauschboden und unter Ihre leisesten Silben setzen.
txaudio-gate = GATE
txaudio-hard-gate-10-1-down = Hartes Gate: 10:1, hinunter bis -40 dB
txaudio-expander = EXPANDER
txaudio-gentle-expander-2-1-never = Sanfter Expander: 2:1, nie mehr als -15 dB — der Raum wird nie tot
txaudio-reset = ZURÜCKSETZEN
txaudio-back-defaults = Zurück zu den Standardwerten
txaudio-threshold = Schwelle
txaudio-below-gate-starts-close = Darunter beginnt das Gate zu schließen.
txaudio-return = Rückkehr
txaudio-hysteresis-once-open-closes-only = Hysterese: Ist es einmal offen, schließt es nur so weit unter der Schwelle, damit eine Stimme, die daran schwebt, das Gate nicht flattern lässt.
txaudio-hold = Halten
txaudio-how-long-stays-open-after = Wie lange es nach dem Absinken der Stimme offen bleibt, damit Wortenden nicht abgeschnitten werden.
txaudio-release = Ausklingen
txaudio-how-fast-closes-once-hold = Wie schnell es schließt, sobald die Haltezeit vorbei ist. Verlängern, wenn das Gate pumpt.
txaudio-floor = Untergrenze
txaudio-most-ever-turns-pauses-down = Das Höchste, um was es die Pausen je absenkt.
txaudio-ratio = Verhältnis
txaudio-how-steeply-closes-below-threshold = Wie steil es unter der Schwelle schließt.
txaudio-look-ahead = Vorausschau
txaudio-opens-much-before-word-starts = Öffnet so viel, bevor ein Wort beginnt, damit sein erster Konsonant nicht abgeschnitten wird. Fügt allem Gesendeten dieselbe Verzögerung hinzu.
txaudio-high-pass-low-shelf-two = Hochpass, Tiefenshelf, zwei Glocken, Höhenshelf und Tiefpass. Einen Punkt auf der Kurve ziehen oder unten einstellen. Den Tiefpass auf die Breite Ihres Sendefilters setzen.
txaudio-flat = FLACH
txaudio-every-band-back-0-db = Jedes Band zurück auf 0 dB, beide Ränder aus
txaudio-freq = Frequenz
txaudio-gain = Verstärkung
txaudio-q-slope = Q / Steilheit
txaudio-high-pass = Hochpass
txaudio-removes-handling-noise-breath-pops = Entfernt Griffgeräusche, Atemgeräusche und Brummen unterhalb der Eckfrequenz.
txaudio-low-shelf = Tiefenshelf
txaudio-body-mud-cut-little-here = Körper oder Matsch: hier etwas absenken für eine Stimme, die kistenartig klingt.
txaudio-peak-1 = Glocke 1
txaudio-bump-dip-voice-band-often = Eine Überhöhung oder Senke im Stimmband — oft eine Absenkung um 300–500 Hz.
txaudio-peak-2 = Glocke 2
txaudio-presence-lift-around-2-3 = Präsenz: eine Anhebung um 2–3 kHz ist, was bei SSB durchdringt.
txaudio-high-shelf = Höhenshelf
txaudio-brightness = Helligkeit.
txaudio-low-pass = Tiefpass
txaudio-stops-energy-above-transmit-filter = Verhindert, dass Energie oberhalb des Sendefilters im Modulator verbraucht wird.
txaudio-t-24-db-oct = 24 dB/Okt
txaudio-t-12-db-oct = 12 dB/Okt
txaudio-click-steeper-gentler-slope = Klicken für eine steilere oder sanftere Flanke
txaudio-de-esser = De-Esser
txaudio-turns-down-only-sibilant-band = Senkt nur das Zischband, wenn „s“ und „sch“ die Schwelle überschreiten, damit der Rest der Stimme seinen Pegel behält. Die Frequenz findet man, indem man sie durchfährt, während man „Sechs sechzig sieben“ spricht, und dort parkt, wo das Zischen am lautesten ist.
txaudio-frequency = Frequenz
txaudio-centre-sibilant-band = Mitte des Zischbandes.
txaudio-width-band-higher-narrower = Breite des Bandes — höher ist schmaler.
txaudio-band-level-where-reduction-starts = Bandpegel, bei dem die Absenkung beginnt. Zu niedrig, und sie arbeitet die ganze Zeit und dämpft die Stimme.
txaudio-amount = Betrag
txaudio-most-band-ever-turned-down = Das Höchste, um was das Band je abgesenkt wird. Etwa -6 dB anfangen.
txaudio-attack = Angriff
txaudio-how-fast-reacts = Wie schnell es reagiert.
txaudio-how-fast-lets-go = Wie schnell es loslässt.
txaudio-compressor = Kompressor
txaudio-brings-quiet-syllables-up-holds = Hebt leise Silben an und hält laute zurück. Auf 3–6 dB Absenkung an Spitzen zielen; mehr als 10 dB ist offensichtliche Kompression, eher für Contests geeignet.
txaudio-level-above-which-starts-compress = Pegel, oberhalb dessen es zu komprimieren beginnt.
txaudio-how-hard-compresses-above-threshold = Wie stark es oberhalb der Schwelle komprimiert.
txaudio-how-fast-reacts-peak-10 = Wie schnell es auf eine Spitze reagiert. 10–25 ms halten Konsonanten knackig.
txaudio-how-fast-recovers = Wie schnell es sich erholt.
txaudio-knee = Knie
txaudio-soft-knee-width-0-hard = Breite des weichen Knies; 0 ist ein hartes Knie.
txaudio-makeup = Make-up
txaudio-gain-after-compression-bring-level = Verstärkung nach der Kompression, um den Pegel wieder anzuheben.
txaudio-drive = Aussteuerung
txaudio-gain-ahead-compressor-so-more = Verstärkung vor dem Kompressor, damit mehr der Stimme die Schwelle überschreitet — hebt die mittlere Leistung. Mit dem Ausgangsbegrenzer paaren.
txaudio-phase = Phase
txaudio-all-pass-phase-rotator-ahead = Allpass-Phasendreher vor dem Detektor. Er gleicht die einseitigen Spitzen aus, die eine Stimme hat, sodass derselbe Spitzenpegel mehr mittlere Leistung trägt. 4 ist eine übliche Rundfunkeinstellung; 0 ist aus.
txaudio-last-stage-trim-look-ahead = Die letzte Stufe: ein Trimm und ein vorausschauender Begrenzer, der jede Spitze unter der Decke hält, ohne den Splatter, den hartes Clipping erzeugt. Standardmäßig aus, damit das Einschalten der Kette eine bereits eingestellte Aussteuerung nicht verändert.
txaudio-limiter = Begrenzer
txaudio-on = Ein
txaudio-ceiling = Decke
txaudio-no-peak-leaves-chain-above = Keine Spitze verlässt die Kette oberhalb davon.
txaudio-trim = Trimm
txaudio-output-gain-ahead-limiter = Ausgangsverstärkung, vor dem Begrenzer.
txaudio-off = {"  "}aus
txaudio-meters-move-while-you-transmit = Die Messgeräte bewegen sich, während Sie eine Sprachaussendung senden, die Testphrase abspielen oder LIVE bzw. MESSGERÄTE laufen lassen.
txaudio-profiles-2 = Profile
txaudio-profile-whole-chain-every-stage = Ein Profil ist die ganze Kette: die Einstellungen jeder Stufe, welche eingeschaltet sind, ihre Reihenfolge und die Eingangsverstärkung für Ihr Mikrofon. Das Laden eines Profils ersetzt die Kette; der Regler Mic und die Aussteuerung bleiben, wie sie sind.
txaudio-name = Name
txaudio-save = Speichern
txaudio-store-chain-now-under-name = Die Kette, wie sie jetzt ist, unter diesem Namen speichern und ein Profil gleichen Namens ersetzen
txaudio-import = Importieren…
txaudio-read-profile-json-file = Ein Profil aus einer .json-Datei lesen
txaudio-tx-audio-profile = TX-Audio-Profil
txaudio-no-profiles-yet-save-stores = Noch keine Profile. Speichern legt die Kette ab, die Sie jetzt eingerichtet haben.
txaudio-load = Laden
txaudio-export = Exportieren…
txaudio-delete = Löschen
txaudio-starting-points = Ausgangspunkte
txaudio-set-your-level-auto-set = Den Pegel zuerst mit Automatisch einstellen setzen — die Schwellen der Voreinstellungen gehen davon aus —, dann von hier aus feinabstimmen, statt so zu senden, wie geladen: eine Phrase aufnehmen und nach jeder Änderung anhören. Eine Voreinstellung behält Ihre Eingangsverstärkung.
txaudio-profile-loaded = Profil geladen.

## Solar-system 3D view overlay (src/solar3d/overlay.rs)

solar-prop = AUSBR.
solar-where-signals-are-actually-getting =
    Wo Signale tatsächlich durchkommen, Band für Band. Jeder Empfang, an dem diese Station teilnimmt — WSPR, FT8/FT4, JS8 und das Logbuch — wird am Mittelpunkt seines Weges eingetragen, dem Stück Ionosphäre, das ihn gebeugt hat, und nicht bei der fernen Station. Das ist also eine Karte des Himmels, nicht, wo Funkamateure leben.

    Sie kann nur Wege zeigen, bei denen diese Station ein Ende war. Ozeane leuchten, weil die Mittelpunkte langer Wege dort liegen; die Antarktis bleibt dunkel, weil von dort niemand sendet. Standardmäßig aus — sie malt den ganzen Planeten.
solar-orbits = BAHNEN
solar-orbital-paths = Bahnen
solar-clouds = WOLKEN
solar-cloud-cover-noaa-s-global = Wolkendecke aus dem globalen geostationären Mosaik der NOAA, gezeichnet als Tiefe der Luft statt als Bild, das auf die Oberfläche geklebt ist: Infrarot sagt, wie kalt jede Wolkenspitze ist, und daraus folgt, wie hoch sie steht. Gewitter blitzen in den Türmen auf, die sie bauen.
solar-planets = PLANETEN
solar-other-seven-planets-eighteen-their = Die übrigen sieben Planeten, achtzehn ihrer Monde und die Ringe von Saturn und Uranus. Die Planetenpositionen stammen aus den keplerschen Elementen des JPL; die Monde sind Kreisbahnen, die an JPL Horizons angepasst wurden.
solar-coronal-mass-ejection-trajectory-cones = Flugbahnkegel koronaler Masseauswürfe
solar-sun-obs = SONNE BEOB.
solar-solar-observations-sun-s-disk = Sonnenbeobachtungen auf der Sonnenscheibe: aktive Sonnenfleckengebiete und woher die Flares kamen
solar-labels = BESCHRIFTUNGEN
solar-body-region-labels = Beschriftungen für Körper und Regionen
solar-small-bodies = KLEINKÖRPER
solar-names-asteroids-comets-they-are = Namen auf den Asteroiden und Kometen. Sie werden so oder so gezeichnet — dies bestimmt nur, ob dreißig-fünf Bezeichnungen mit ihnen gezeichnet werden. Wie auch immer es eingestellt ist, bleibt der Körper, auf den die Kamera zeigt, und alles, was das Suchfeld getroffen hat, benannt.
solar-decoded-ft8-ft4-stations-path = Dekodierte FT8/FT4-Stationen und der Weg zur gearbeiteten Station
solar-sats = SATS
solar-amateur-radio-satellites-their-orbits = Amateurfunksatelliten, ihre Bahnen und Elevation von Ihrem QTH
solar-aurora = AURORA
solar-auroral-oval-noaa-s-ovation = Das Polarlichtoval aus dem OVATION-Modell der NOAA, als Emissionsschalen in ihren echten Höhen gezeichnet, mit dem äquatorseitigen Rand der 10-%-Kontur auf der Oberfläche markiert
solar-awards = AUSZEICHNUNGEN
solar-award-coverage-every-dxcc-entity = Abdeckung der Auszeichnungen: jede DXCC-Einheit, gemalt nach dem, was Ihrem Logbuch noch fehlt. Orange brennt, wo Sie die Einheit nie gearbeitet haben, Bernstein, wo Sie sie gearbeitet haben, aber unbestätigt, und gedämpftes Grün, wo eine QSL zurückkam. Folgt dem Bandfilter im Fenster AUSZEICHNUNGEN.
solar-view = ANSICHT
solar-which-body-camera-orbits-animated = Um welchen Körper die Kamera kreist, und die animierte Tour
solar-layers = EBENEN
solar-what-drawn-orbits-clouds-cmes = Was gezeichnet wird: Bahnen, Wolken, KMA, Beschriftungen, QSOs, Polarlicht
solar-which-sdo-channel-wraps-sun = Welcher SDO-Kanal die Sonne umhüllt und wie alt er ist
solar-weather = WETTER
solar-aurora-propagation-numbers-which-bands = Das Polarlicht, die Ausbreitungszahlen und welche Bänder offen sind
solar-space-weather = WELTRAUMWETTER
solar-nothing-measured-yet = noch nichts gemessen
solar-scale = MASSSTAB
solar-body-moon-orbit-exaggeration = Überhöhung von Körper und Mondbahn
solar-time = ZEIT
solar-scrub-whole-scene-forward-back = Die ganze Szene vor und zurück durchfahren
solar-activity = AKTIVITÄT
solar-replay-last-hour-decodes-globe = Die letzte Stunde der Dekodierungen auf dem Globus wiedergeben
solar-what-propagation-heat-map-showing = Was die Ausbreitungs-Wärmekarte zeigt
solar-propagation = AUSBREITUNG
solar-auto = ▶ AUTO
solar-fly-spline-through-set-framed = Eine Spline durch eine Reihe gerahmter Blickpunkte fliegen. Jede Mauseingabe bricht sie ab.
solar-moons-ride-circular-orbits-fitted = Die Monde fahren auf Kreisbahnen, die an JPL Horizons angepasst sind: innerhalb ein bis zwei Grad davon, wo sie wirklich sind, und bis zu sechs für Miranda, dessen Bahnebene zu schnell schwenkt, als dass ein Kreis folgen könnte.
solar-fetch-everything-again-now = Jetzt alles erneut abrufen
solar-all-sats = ALLE SATS
solar-show-every-satellite-element-set = Jeden Satelliten im Bahnelementsatz zeigen, nicht nur die beliebten. Die Bahnringe bleiben bei den wenigen kuratierten — neunzig auf einmal sind unlesbar.
solar-starting = startet…
solar-body = Körper{" "}
solar-moon-orbit = Mondbahn{" "}
solar-stretch-earth-moon-distance-so = Die Entfernung Erde→Mond strecken, damit das Paar getrennt sichtbar ist
solar-sun = Sonne{" "}
solar-sun-radius-exaggeration-leave-1 = Überhöhung des Sonnenradius. Auf 1× lassen, um die Geometrie der KMA lesbar zu halten — eine aufgeblähte Sonne verschluckt die Basis jedes Kegels.
solar-show = ZEIGEN
solar-paint-globe-what-getting-through = Den Globus danach malen, was durchkommt
solar-paints-globe-what-getting-through = Malt den Globus danach, was auf jedem Band durchkommt. Jeder Empfang wird am Mittelpunkt seines Weges eingetragen — wo die Ionosphäre die Arbeit getan hat — statt bei der fernen Station. Gebaut aus dem, was diese Station hört, plus den Skimmern der Welt, wenn das Reverse Beacon Network eingeschaltet ist.
solar-display = ANZEIGE
solar-all-bands = ALLE BÄNDER
solar-every-band-once-one-hue = Jedes Band auf einmal, je ein Farbton, mit Mischung dort, wo sich zwei überlappen. Die Ansicht für „wie sind die Bedingungen“ statt „ist zwanzig offen“.
solar-one-band = EIN BAND
solar-one-band-cold-hot-blue = Ein Band, kalt bis heiß: blau, grün, gelb, rot.
solar-band = BAND
solar-nothing-heard-yet = noch nichts gehört
solar-bands = BÄNDER
solar-sources = QUELLEN
solar-contacts-logbook-path-was-open = Kontakte im Logbuch. Ein Weg, der offen war, ohne einen Signalbericht, der den Namen verdient — ein RST ist kein SNR, daher zählen diese zur Auslastung einer Zelle und nie zu ihrer Reserve.
solar-stations-reported-hearing-one-downloaded = Stationen, die gemeldet haben, diese zu hören, heruntergeladen von WSPRnet. Das einzige Feedback, das eine sendende Bake je bekommt.
solar-reverse-beacon-network-skimmers-what = Reverse-Beacon-Network-Skimmer: was alle anderen hören, auf jedem Band, einschließlich der Bänder, auf denen dieses Funkgerät nicht ist. Grober als der Rest — eine RBN-Zeile trägt keine Locatoren, daher werden beide Enden in die Mitte ihres Landes gesetzt, was für ein kleines Land nahe liegt und für ein großes weit daneben. Unter Einstellungen → Spots einschalten.
solar-decodes-station-made-each-measured = Dekodierungen, die diese Station gemacht hat. Jede wird gegen die Untergrenze ihrer eigenen Betriebsart gemessen, daher sind ein WSPR-Bericht und ein FT4-Bericht vergleichbar.
solar-memory = SPEICHER
solar-half-life = Halbwertszeit{" "}
solar-how-long-reception-takes-count = Wie lange ein Empfang braucht, um nur halb so viel zu zählen. Das eigene Gedächtnis der Ionosphäre ist kurz — eine Öffnung, zwei Stunden alt, sollte nicht mit einer streiten, die zwei Minuten alt ist.
solar-live = LIVE
solar-follow-band-happens = Dem Band folgen, während es passiert
solar-replay = ⏸ WIEDERGABE
solar-replay-2 = ▶ WIEDERGABE
solar-replay-last-hour-decodes-over = Die letzte Stunde der Dekodierungen wiedergeben, immer wieder
solar-min-ago = {" "}Min. her
solar-where-last-hour-globe-showing = Wo in der letzten Stunde der Globus gerade zeigt
solar-trail = Spur{" "}
solar-how-long-decode-s-arc = Wie lange der Bogen einer Dekodierung auf dem Globus hinter dem Kopf bleibt
solar-speed = Tempo{" "}
solar-how-much-faster-than-real = Wie viel schneller als Echtzeit die Wiedergabe läuft
solar-qso-layer-off = QSO-Ebene aus
solar-no-decodes-yet = noch keine Dekodierungen
solar-qth-not-set-enter-your = QTH nicht gesetzt — Ihren Locator in den Einstellungen eingeben
solar-mode = BETRIEBSART
solar-path = WEG
solar-sent = GESENDET
solar-rcvd = EMPF.
solar-grid = LOCATOR
solar-dxcc = DXCC
solar-geostationary-always-above-your-horizon = Geostationär — immer über Ihrem Horizont.
solar-no-passes-next-48-hours = Keine Überflüge in den nächsten 48 Stunden von Ihrem QTH.
solar-start = START
solar-max-el = MAX. EL
solar-aos-los-are-azimuths-horizon = AOS/LOS sind Azimute am Horizont. Die Zeiten sind UTC.
solar-locked-unlock = {" "}● EINGERASTET — LÖSEN{" "}
solar-release-satellite-lock = Die Satellitenverfolgung lösen
solar-lock = {" "}EINRASTEN{" "}
solar-track-satellite-main-window-doppler = Diesen Satelliten im Hauptfenster verfolgen: Doppler-korrigierter Empfang und Senden, auf den Transponder abgebildeter Uplink, Rotorsteuerung
solar-no-frequencies-file-one-add = Keine Frequenzen für diesen hinterlegt — unter Einstellungen ▸ TLE hinzufügen.
solar-frequencies-yours = FREQUENZEN · IHRE
solar-frequencies = FREQUENZEN
solar-link = VERBINDUNG
solar-downlink-mhz = ABWÄRTSSTRECKE (MHz)
solar-uplink-mhz = AUFWÄRTSSTRECKE (MHz)
solar-doppler-shifts-these-few-khz = Der Doppler verschiebt diese über einen LEO-Überflug um einige kHz.
solar-sim = -- SIM
solar-satellite-planet-comet = Satellit, Planet oder Komet
solar-planet-asteroid-comet = Planet, Asteroid oder Komet
solar-clear-search = Die Suche leeren
solar-no-match = kein Treffer
solar-no-sounder = kein Sounder
solar-heard = GEHÖRT ≥
solar-kp = Kp / A
solar-x-ray = Röntgen
solar-observed-above-sounder-better-than = beobachtet oberhalb des Sounders — besser als modelliert
solar-power-n-s = Leistung N/S
solar-edge-n-s = Rand N/S
solar-kp-peak-24-h = Kp-Spitze 24 h
solar-observed-forecast = beobachtet | Vorhersage
solar-bands-open = BÄNDER OFFEN
solar-dxcc-coverage = DXCC-ABDECKUNG
solar-ir-vis = IR+VIS
solar-ir-only = nur IR
solar-no-deep-convection = keine tiefe Konvektion
solar-t-1-storm = 1 Sturm
solar-glancing = {" "}· streifend
solar-direction-estimated = {" "}· Richtung geschätzt

## Digital-mode setup dialogs (src/app/panels/setup.rs)

setup-aprs-setup = APRS-Einrichtung
setup-packet-setup = Paket-Einrichtung
setup-ft8-ft4-ft2-setup = FT8 / FT4 / FT2 Einrichtung
setup-my-callsign = Mein Rufzeichen
setup-my-grid = Mein Locator
setup-station-call = Stationsrufzeichen
setup-ssid-oe3jjs-10-packet-station = Mit einer SSID — OE3JJS-10. Eine Paketstation hat üblicherweise eine andere SSID als der Funkamateur, und nichts wird gesendet, bis dies gesetzt ist.
setup-speed = Geschwindigkeit
setup-t-9600-needs-radio-s-data = 9600 braucht den Datenport des Funkgeräts. Ein Weg über Mikrofon und Lautsprecher zerstört es an beiden Enden — das liegt am Funkgerät, nicht an sdroxide.
setup-tx-delay = Sendeverzögerung
setup-flags-sent-before-frame-so = Flags, die vor einem Rahmen gesendet werden, damit die Gegenseite uns hören und ihren Takt einrasten kann. Bei einem CAT-Gerät braucht allein sdroxide 165–240 ms, bis es auf Sendung geht, und die eigene Sendebereitschaftszeit des Geräts kommt noch dazu.
setup-packet-length = Paketlänge
setup-bytes = {" "}Bytes
setup-most-single-frame-carries-shorter = Das Meiste, das ein einzelner Rahmen trägt. Kürzere Rahmen überstehen einen grenzwertigen Weg — nur der getroffene Rahmen wird erneut gesendet — und kosten mehr Overhead. 128 auf KW, 256, wo der Weg stabil ist.
setup-frames-sent-before-waiting-acknowledgement = Rahmen, die gesendet werden, bevor auf eine Bestätigung gewartet wird. Mehr füllt einen guten Weg; auf einem schlechten ist es mehr zum Erneut-Senden.
setup-extended = Erweitert
setup-ask-mod-128 = Mod-128 anfordern
setup-extended-sequence-numbers-window-bigger = Erweiterte Folgenummern, für ein Fenster größer als sieben. Viele Knoten lehnen die Anfrage mit einer DM ab, was genau wie eine Station aussieht, die nicht mit Ihnen sprechen will — daher ausgeschaltet lassen, sofern Sie nicht wissen, dass die Gegenseite es verlangt.
setup-answer-calls = Anrufe beantworten
setup-accept-connections = Verbindungen annehmen
setup-off-winlink-client-which-dials = Aus für einen Winlink-Client, der selbst anwählt und keinen Grund hat zu antworten. Ein, um als Gleichrangiger erreichbar zu sein: Ein Anruf erscheint im Terminalbereich, und Sie sprechen mit dem, der ihn gemacht hat. Eine Station, deren Verbindung schon von einer Winlink-Sitzung belegt ist, lehnt Anrufe ab, bis diese beendet ist.
setup-connect-text = Begrüßungstext
setup-what-caller-greeted = womit ein Anrufer begrüßt wird
setup-sent-station-connects-us-once = Wird an eine Station gesendet, die uns anruft, sobald die Verbindung steht. Leer sendet nichts — was für den Anrufer kaputt aussieht.
setup-default-via = Standardweg über
setup-oe3xlr-1-oe3xms-1 = OE3XLR-1,OE3XMS-1
setup-digipeater-path-terminal-starts-path = Der Digipeater-Pfad, mit dem das Terminal beginnt. Der Pfad zu Ihrem lokalen Knoten ist jedes Mal derselbe, und ihn neu einzutippen ist der Weg, auf dem ein Sprung vergessen wird.
setup-beacon = Bake
setup-t-0-disables-timer = 0 schaltet den Zeitgeber aus
setup-beacon-text = Baketext
setup-kiss-server = KISS-Server
setup-serve = Bereitstellen
setup-offer-modem-kiss-tnc-socket = Dieses Modem als KISS-TNC auf einem Socket anbieten, damit Pat, ein APRS-Client oder der Linux-AX.25-Stack das Funkgerät nutzen können.
setup-aprs-call = APRS-Rufzeichen
setup-callsign-station-beacons-under-ssid = Das Rufzeichen, unter dem diese Station sendet, mit seiner SSID — OE3JJS-9 für ein Auto, -10 für ein I-Gate, -5 für ein Telefon. Leer lassen, um das Rufzeichen oben zu verwenden.
setup-no-callsign-station-will-not = Kein Rufzeichen: Diese Station sendet überhaupt nichts — keine Bake, keine Nachricht, keine Bestätigung.
setup-symbol = Symbol
setup-two-characters-protocol-carries-them = Die zwei Zeichen, so wie das Protokoll sie führt: eine Tabelle (`/` oder `\`) und ein Symbolcode. Eine Ziffer oder ein Buchstabe an der ersten Stelle ist eine Überlagerung, die über das Symbol der Ersatztabelle gezeichnet wird.
setup-path = Pfad
setup-wide1-1-wide2-1 = WIDE1-1,WIDE2-1
setup-how-far-you-ask-repeated = Wie weit Sie wiederholt werden wollen. Das ist die folgenreichste Einstellung auf dem Kanal: Jeder Sprung vervielfacht die Aussendungen, die das ganze Netz auf einer gemeinsamen Frequenz macht.
setup-position = Position
setup-my-grid-2 = Aus meinem Locator
setup-beacon-centre-locator-above-reported = Die Mitte des Locators oben als Bake senden, mit der Unschärfe, die ein Locator tatsächlich hat — einer mit sechs Zeichen ist ein paar Kilometer breit, und das so zu sagen ist ehrlich. Abwählen, um stattdessen genaue Koordinaten zu geben.
setup-latitude = Breitengrad
setup-longitude = Längengrad
setup-locator = aus dem Locator
setup-not-coordinate = keine Koordinate
setup-comment = Kommentar
setup-sent-every-beacon = wird mit jeder Bake gesendet
setup-t-0-default-never-beacons-timer = 0 — der Standard — sendet nie per Zeitgeber eine Bake. Dreißig Minuten sind die Konvention für eine feste Station; eine bewegte sendet öfter, aber jede Bake ist Sendezeit auf dem Kanal eines anderen.
setup-compressed = Komprimiert
setup-compressed-position-format-third-air = Das komprimierte Positionsformat: ein Drittel der Sendezeit und genauer. Jeder Empfänger seit den 1990ern liest es.
setup-first-goes-out-one-interval = Die erste geht eine Periode ab jetzt hinaus, nicht sofort.
setup-messages = Nachrichten
setup-acknowledge = Bestätigen
setup-answer-messages-addressed-you-acknowledgement = Nachrichten beantworten, die an Sie gerichtet sind. Eine Bestätigung ist eine Aussendung, die diese Station ungefragt macht, daher sollte eine reine Empfangseinrichtung sie ausschalten — und die Bake hört dann auch auf zu behaupten, erreichbar zu sein.
setup-flags-sent-ahead-every-frame = Flags, die vor jedem Rahmen gesendet werden, damit das Modem am anderen Ende den Träger hört und seinen Takt einrastet, bevor die Daten beginnen. Sie muss alles überdauern, was zwischen Drücken von Senden und Abstrahlen liegt: Allein sdroxide verbraucht davon 165–240 ms bei einem CAT-Gerät, und ein Funkgerät, das Audio über ein Netzwerk nimmt, puffert obendrauf mehr. Zu wenig, und das Gegenüber rastet nie ein — die Aussendung ist da, und nichts dekodiert sie.
setup-shared-packet-mode = mit dem Paketmodus geteilt
setup-keep-stations = Stationen behalten
setup-how-long-station-stays-map = Wie lange eine Station auf der Karte bleibt, nachdem sie zuletzt gehört wurde. Außerdem ist dies, woran das Verblassen der Karte gemessen wird, daher zeigt ein kurzes Fenster nur, was live ist.
setup-auto-reply = Automatische Antwort
setup-answer-snr-grid-status = SNR? / GRID? / STATUS? beantworten
setup-answer-direct-question-addressed-you = Eine direkte Frage beantworten, die an Sie oder an @ALLCALL gerichtet ist, mit der Antwort statt einer Bestätigung. Beantwortet nie den Verkehr einer anderen Station und nie sich selbst.
setup-heartbeat = Herzschlag
setup-off = Aus
setup-turbo-does-not-beacon-local = Turbo sendet keine Bake — es ist die Geschwindigkeit für lokal und UKW.
setup-sends-your-callsign-grid-so = Sendet Ihr Rufzeichen und Ihren Locator, damit andere wissen, dass Sie empfangbar sind. Die erste geht eine Periode ab jetzt hinaus, nicht sofort.
setup-beacon-frequency = Bakenfrequenz
setup-t-500-1000-hz = 500–1000 Hz
setup-move-each-beacon-free-slot = Jede Bake auf einen freien Platz im Herzschlag-Teilband verlegen, wie JS8Call es tut: Dort suchen Stationen nach Baken, und es hält einen unbeaufsichtigten Sender vom QSO eines anderen fern. Der Platz wird gewählt, wenn die Bake tatsächlich hinausgeht, frei von allem, was dekodiert wird.
setup-working-freq = Arbeitsfrequenz
setup-beacon-where-you-are-working = Stattdessen dort senden, wo Sie arbeiten. Gegen die Bandkonvention, aber es hält alles, was Sie senden, an einem Ort.
setup-heartbeat-reply = Herzschlag-Antwort
setup-answer-heartbeats-signal-report = Herzschläge mit einem Signalbericht beantworten
setup-tell-station-beaconed-how-well = Einer Station, die eine Bake gesendet hat, sagen, wie gut Sie sie aufgenommen haben. Standardmäßig aus: Ein belegtes Band trägt in jedem Zeitschlitz einen Herzschlag, und auf alle zu antworten würde genau das Band überfluten, das Herzschläge ruhig halten sollen. Begrenzt auf eine Antwort je Station alle 15 Minuten, und nie, während noch eine Nachricht hereinkommt oder Sie etwas zum Senden in der Warteschlange haben.
setup-status-message = Statusmeldung
setup-tx-period = Sendeperiode
setup-even = Gerade
setup-odd = Ungerade
setup-auto-sequence = Automatische Abfolge
setup-decode-depth = Dekodiertiefe
setup-fast = Schnell
setup-normal = Normal
setup-deep = Tief
setup-how-hard-ft8-decoder-works =
    Wie stark der FT8-Decoder bei schwachen Signalen arbeitet.

    Schnell — ein Durchlauf, keine Signalsubtraktion: am schnellsten, wenigste Dekodierungen.
    Normal — gleichmäßige Mehrdurchlauf-Subtraktion: etwas schneller als Tief, etwas weniger gründlich.
    Tief — der Mehrdurchlauf mit Kontrollpunkten: die meisten Dekodierungen, und etwa 1,2 s in einem belegten Zeitschlitz.
setup-auto-tx-frequency = Automatische Sendefrequenz
setup-choose-transmit-frequency-automatically-quietest = Die Sendefrequenz automatisch wählen: die ruhigste Stelle in der Periode, in der Sie senden, statt die Frequenz der Station, die Sie beantworten. Aus hält sie NICHT fest — dann wird auf der Frequenz der gerufenen Station geantwortet. Zum Festhalten Sendefrequenz halten verwenden.
setup-hold-tx-frequency = Sendefrequenz halten
setup-pin-transmit-tone-where-nothing = Den Sendeton dort festhalten, wo er ist. Nichts verschiebt ihn: nicht das Antworten auf eine Station, nicht die Anrufwarteschlange, nicht CQ rufen, nicht ein Klick auf eine Dekodierung oder den Wasserfall. Überschreibt die automatische Sendefrequenz. Ein Bandwechsel ist die einzige Ausnahme: Der Versatz, den Sie zuletzt auf dem neuen Band gesetzt haben, kommt damit zurück. Für den Fall, dass Ihre Lizenz schmaler ist als der Bandplan — auf einer britischen 60-m-Skala von 5357 kHz endet die Zuteilung bei 5358,0, daher muss der Ton unter 1000 Hz bleiben.
setup-tx-watchdog = Sende-Watchdog
setup-stop-transmitting-after-long-no = Nach dieser Zeit ohne Antwort und ohne Aktion von Ihnen das Senden beenden. 0 schaltet es aus.
setup-give-up-after = Aufgeben nach
setup-calls = {" "}Rufen
setup-unanswered-calls-one-station-before = Unbeantwortete Rufe an eine Station, bevor man weitergeht. CQ rufen ist ausgenommen. 0 schaltet es aus.
setup-contest = Contest
setup-everyday-exchange-grid-report-rr73 = Der Alltagsaustausch: Locator, Bericht, RR73.
setup-european-vhf-contests-send-signal = Europäische UKW-Contests: einen Signalbericht, eine laufende Nummer und Ihren Locator mit sechs Zeichen senden, im Nachrichtenaufbau, den WSJT-X EU VHF Contest nennt. CQ geht als CQ TEST hinaus.
setup-serial = Laufende Nummer
setup-serial-number-next-exchange-will = Die laufende Nummer, die der nächste Austausch trägt. Sie zählt von selbst weiter, während jeder Kontakt protokolliert wird, und wird zwischen Sitzungen gemerkt — von Hand setzen nur, um einen Contest zu beginnen oder dort weiterzumachen, wo ein Papierlogbuch stand.
setup-needs-6-character-grid = ⚠ braucht einen Locator mit 6 Zeichen
setup-eu-vhf-contest-messages-carry = EU-VHF-Contest-Nachrichten tragen einen Locator mit sechs Zeichen (JN88DD, nicht JN88). Solange Mein Locator keinen solchen hat, wird stattdessen der gewöhnliche Austausch gesendet.
setup-dxpedition = DXpedition
setup-ordinary-ft8-operation = Gewöhnlicher FT8-Betrieb.
setup-calling-dxpedition-running-fox-mode = Eine DXpedition rufen, die den Fox-Modus fährt: von oberhalb 1000 Hz rufen, auf den Fox hinunterwechseln, wenn er antwortet, und auf dessen RR73 loggen, ohne 73 zu senden.
setup-run-pile-up-several-signals = Das Pile-up fahren: mehrere Signale gleichzeitig, eine Warteschlange von Anrufern, die stärksten und seltensten zuerst. CQ RUFEN startet es, QSO STOPPEN hält es an.
setup-fox-signals = Fox-Signale
setup-once = {" "}auf einmal
setup-simultaneous-transmissions-spaced-60-hz = Gleichzeitige Aussendungen im Abstand von 60 Hz. Sie teilen sich die Leistung des Senders, daher ist jedes Signal schwächer, je mehr es sind.
setup-tx-audio = TX-Audio
setup-how-loud-mode-s-burst =
    Wie laut der Burst dieser Betriebsart an ein Funkgerät übergeben wird, das ihn selbst moduliert — und bei FM ist das der Hub, weil ein FM-Sender den Audiopegel in einen Frequenzhub umsetzt und keine ALC hat, die ihn abfängt. 1200-Baud-Paket will etwa 3 kHz, wo Sprache 5 will; Vollaussteuerung auf einen Dateneingang, der für Sprache eingestellt ist, übersteuert daher den Hub — was für einen Hörer völlig normal klingt und für niemanden dekodiert. Herunterdrehen, bis andere Stationen Sie melden, oder den Pegel am Funkgerät einstellen.

    Dieser Pegel gehört dieser Betriebsart allein. Eine Betriebsart, die Sie nie eingestellt haben, startet beim FM-Wert, und die Seitenbandbetriebsarten starten bei ihren eigenen — daher landet ein für 1200 Baud eingestellter Hub nie bei FT8.
setup-how-loud-mode-s-over =
    Wie laut die Aussendung dieser Betriebsart an ein Funkgerät übergeben wird, das sie selbst moduliert — ein CAT-Gerät über seine Soundkarte, ein FLEX, ein Icom über seinen Netzwerkport. Auf Seitenband ist dies die Aussteuerung in den Modulator: Herunterdrehen, bis die ALC des Geräts kaum noch ausschlägt, und die Leistung am Funkgerät einstellen, weil eine ALC, die bei einer Betriebsart mit konstanter Hüllkurve schwankt, zu Splatter führt. Die Aussteuerung erreicht hier das Leistungsregister des Geräts, nicht sein Audio, daher ist dies der Pegel.

    Dieser Pegel gehört dieser Betriebsart allein: FT8, RTTY, PSK und MCW behalten jeweils ihren eigenen, weil sie einen Modulator nicht gleich belasten. Dieselbe Leiste steht auf der Sendeleiste, wo Sie sie während des Sendens erreichen.
setup-deviation-fm = Hub bei FM
setup-drive-ssb = Aussteuerung bei SSB
setup-grid = Locator
setup-report = Bericht
setup-r-report = R+Bericht

## AMP window (src/app/amp.rs)

amp-t-10-w-scale-rescaled = , Skala 10 W, neu skaliert
amp-t-10-w-scale = , Skala 10 W
amp-t-2-w-scale-rescaled = , Skala 2 W, neu skaliert
amp-t-2-w-scale = , Skala 2 W
amp-t-100-w-scale-rescaled = , Skala 100 W, neu skaliert
amp-t-100-w-scale = , Skala 100 W
amp-t-20-w-scale-rescaled = , Skala 20 W, neu skaliert
amp-t-20-w-scale = , Skala 20 W
amp-rescaled = , neu skaliert
amp-and = {" "}UND{" "}
amp-not-while-transmitting-switching-relay = Nicht während des Sendens: Ein Relais unter HF umzuschalten kann es beschädigen. Zuerst die Taste loslassen.
amp-no-amplifier-set-up-choose = Kein Verstärker eingerichtet. Seinen Port unter Einstellungen → Verstärker wählen.
amp-settings = EINSTELLUNGEN
amp-dock = ANDOCKEN
amp-put-amp-window-back-inside = Das Fenster AMP zurück in das Hauptfenster setzen
amp-pop-out = HERAUSLÖSEN
amp-give-amp-window-window-own = Dem Fenster AMP ein eigenes Fenster geben, das auf einen anderen Bildschirm verschoben werden kann
amp-rig = , vom Funkgerät
amp-rig-flagged-reading = , das Funkgerät hat seinen Messwert markiert
amp-drv = , von DRV
amp-rig-s-swr = , vom SWR des Funkgeräts
amp-ptt-keys-amplifier-radio-off = PTT tastet den Verstärker mit dem Funkgerät; AUS lässt ihn überbrückt. Klicken zum Umschalten.
amp-switch-antenna-jack = Antennenbuchse umschalten
amp-band-down = Band abwärts
amp-band-up = Band aufwärts
amp-tuner = TUNER
amp-tuned = Abgestimmt
amp-tune = ABSTIMMEN
amp-start-tune-then-key-steady = Einen Abstimmvorgang starten — dann einen stetigen Träger tasten (CW, FM oder RTTY). Erneut drücken, um zu stoppen.
amp-put-tuner-line-bypass = Den Tuner in die Leitung legen oder ihn überbrücken
amp-tuning = Stimme ab…
amp-bottom = UNTEN
amp-pick-any-reading-either-needle = Für jeden Zeiger einen beliebigen Messwert wählen. Der obere Zeiger ist weiß, der untere blau.
amp-forward-power-measured-radio-amplifier = Vorwärtsleistung, vom Funkgerät gemessen: Der Verstärker meldet bei AUS keine
amp-forward-power-taken-drv-amplifier = Vorwärtsleistung, von DRV genommen: Der Verstärker meldet bei AUS keine, und im Überbrückungsbetrieb geht die Leistung des Funkgeräts direkt durch
amp-forward-power = Vorwärtsleistung
amp-reflected-power-measured-radio-amplifier = Rückwärtsleistung, vom Funkgerät gemessen: Der Verstärker meldet bei AUS keine
amp-reflected-power-worked-out-fwd = Rückwärtsleistung, aus VOR und dem SWR errechnet: Der Verstärker meldet bei AUS keine
amp-reflected-power = Rückwärtsleistung
amp-standing-wave-ratio-red-past = Stehwellenverhältnis: rot bei oder über der Abstimmgrenze (Einstellungen → Allgemein). Die Antenne prüfen
amp-standing-wave-ratio-measured-radio = Stehwellenverhältnis, vom Funkgerät gemessen: Der Verstärker meldet bei AUS keines
amp-standing-wave-ratio = Stehwellenverhältnis
amp-drive-radio = Aussteuerung vom Funkgerät
amp-drain-supply = Drain-Versorgung
amp-drain-current = Drain-Strom
amp-temp = TEMP
amp-heat-spreader-temperature-yellow-49 = Temperatur des Kühlkörpers: gelb ab 49 °C (120 °F), rot über 65 °C (149 °F), außer Betrieb über 70 °C (158 °F). Klicken für °C/°F.
amp-c = °C
amp-f = °F
amp-performance = LEISTUNG
amp-numbers-hold-peaks-bars-are = Zahlen halten Spitzen · Balken sind live
amp-input-power = EINGANGSLEISTUNG
amp-supply-volts-drain-amps = Versorgungsspannung × Drain-Strom
amp-efficiency = WIRKUNGSGRAD
amp-output-power-input-power = Ausgangsleistung ÷ Eingangsleistung
amp-gain = VERSTÄRKUNG
amp-output-power-over-drive-decibels = Ausgangsleistung über der Aussteuerung, in Dezibel
amp-off-line-over-temperature = AUSSER BETRIEB · übertemperatur
amp-connected-transmitting = VERBUNDEN · SENDET
amp-connected = VERBUNDEN
amp-connecting = Verbinde…
amp-atu-bypassed = ATU ÜBERBRÜCKT
amp-no-inductance-needed = keine Induktivität nötig
amp-no-capacitance-needed = keine Kapazität nötig
amp-c-out-antenna-side = C OUT · Antennenseite
amp-c-transmitter-side = C IN · Senderseite

## SSTV panel (src/app/panels/sstv.rs)

sstv-chunks-received-lit-still-missing = Empfangene Teile (leuchtend) und noch fehlende (dunkel)
sstv-sstv = SSTV
sstv-auto = Auto
sstv-signal = Signal
sstv-station-sending-mode-sdroxide-does = Eine Station sendet in einer Betriebsart, die sdroxide nicht hat. Signal und Abstimmung sind in Ordnung — es gibt schlicht keinen Decoder dafür. Auf Ihrer Seite ist nichts zu beheben.
sstv-listening = höre…
sstv-restart-rx = Empfang neu starten
sstv-abandon-picture-being-received-listen = Das gerade empfangene Bild aufgeben und auf den nächsten Kopf warten. Für eine Aussendung, die in der falschen Betriebsart zu dekodieren begann — der Empfänger ist sonst daran gebunden, bis diese Betriebsart abläuft.
sstv-callsign-last-station-sent-fsk = Das Rufzeichen, das die letzte Station nach ihrem Bild als FSK-ID gesendet hat.
sstv-tx-slant = Senden-Schräglage
sstv-transmit-clock-trim-ppm-remove = Trimm des Sendetakts (ppm), um die Schräglage beim Decoder am anderen Ende zu entfernen
sstv-reset-0-ppm = Auf 0 ppm zurücksetzen
sstv-fsk-id = FSK-ID
sstv-send-your-callsign-tones-after = Ihr Rufzeichen nach jedem Bild in Tönen senden — die Kennung, die SSTV-Relais und andere Programme lesen. Das kostet etwa 2,5 Sekunden und sendet gar nichts, bis Sie ein Rufzeichen gesetzt haben.
sstv-tx-lead = Sende-Vorlauf
sstv-silence-sent-after-keying-before = Stille, die nach dem Tasten und vor dem Vorspann des Bildes und dem VIS-Code gesendet wird. Ein Decoder, der einen Teil dieses Kopfes verpasst, zeigt gar kein Bild, daher deckt dies die Lücke zwischen der PTT-Anforderung an ein Gerät und dem tatsächlichen Senden ab. 0 für ein SDR, das sofort tastet.
sstv-live = LIVE
sstv-waiting-picture = warte auf ein Bild…
sstv-waiting-signal = warte auf ein Signal…
sstv-no-low-audio = kein / zu leises Audio
sstv-received = EMPFANGEN
sstv-reading-radio-s-pictures = Die Bilder des Funkgeräts werden gelesen…
sstv-decoded-pictures-collect-here = Dekodierte Bilder sammeln sich hier.
sstv-click-enlarge-right-click-delete = Klicken zum Vergrößern · Rechtsklick zum Löschen
sstv-delete-picture = Dieses Bild löschen
sstv-loading = lädt…
sstv-transmit = SENDEN
sstv-click-edit-slot-s-message = Klicken, um die Nachricht dieses Platzes zu bearbeiten · Doppelklick, um ein Bild zu laden
sstv-change-image = Bild ändern…
sstv-load-image = Bild laden…
sstv-clear = Leeren
sstv-empty-slot-s-picture-message = Das Bild dieses Platzes leeren (die Nachricht bleibt)
sstv-banner = Banner…
sstv-what-printed-across-top-every = Was über den oberen Rand jedes Bildes gedruckt wird, das diese Station sendet
sstv-fetching-slot-s-picture = Das Bild dieses Platzes wird geholt…
sstv-load-image-into-slot = Ein Bild in diesen Platz laden →
sstv-drawn-slot-s-image = Auf das Bild dieses Platzes gezeichnet
sstv-tx = {"   "}TX{"   "}
sstv-abort-tx = {" "}SENDEN ABBRECHEN{" "}
sstv-delete-picture-store = Dieses Bild aus dem Speicher löschen
sstv-received-image = Empfangenes Bild
sstv-no-longer-store = nicht mehr im Speicher
sstv-loading-full-size = lädt in voller Größe…
sstv-save-picture = Bild speichern…
sstv-save-copy-computer = Eine Kopie auf diesem Computer speichern
sstv-delete-sure = Löschen — sicher?
sstv-click-again-delete-good = Erneut klicken, um es endgültig zu löschen
sstv-delete = Löschen…
sstv-image-banner = Bildbanner
sstv-drawn-into-every-picture-station = Wird in jedes Bild gezeichnet, das diese Station sendet, über der eigenen Nachricht des Platzes.
sstv-draw-banner = Das Banner zeichnen
sstv-substituted-when-picture-composed = Wird ersetzt, wenn das Bild zusammengesetzt wird:
sstv-top-left = Oben links
sstv-top-right = Oben rechts
sstv-colours = Farben
sstv-strip-top-edge-fades-black = Der Streifen, an seiner Oberkante — er verblasst zu Schwarz oder zur Verlaufsfarbe darunter
sstv-both-texts = Beide Texte
sstv-gradient = Verlauf
sstv-fade = verblassen zu
sstv-fade-strip-top-colour-second = Den Streifen von seiner oberen Farbe zu einer zweiten verblassen lassen, statt zu Schwarz.
sstv-text-outline = Textumriss
sstv-draw-outline-around-banner-text = Einen Umriss um den Bannertext zeichnen, damit er auf einem belebten oder hellen Streifen lesbar bleibt.
sstv-text-gradient = Textverlauf
sstv-fade-banner-text-colour-second = Den Bannertext von seiner Farbe zu einer zweiten über die Breite des Bildes verblassen lassen.
sstv-rainbow-text = Regenbogentext
sstv-all-text-overriding-colours-above = aller Text, die Farben oben überschreibend
sstv-draw-every-text-picture-banner = Jeden Text im Bild — Banner und Nachricht — als waagerechten Regenbogen zeichnen und die Farben oben ignorieren.
sstv-message = Nachricht
sstv-slot-message-s-text-colour = Die Textfarbe der Platznachricht
sstv-outline = Umriss in
sstv-height = Höhe
sstv-how-tall-strip-pixels-transmitted = Wie hoch der Streifen ist, in Pixeln des gesendeten Bildes. Der Text richtet sich danach — ein SSTV-Rahmen ist nur 320 Pixel breit, daher macht ein höheres Banner den Text am anderen Ende lesbar.
sstv-reset = Zurücksetzen
sstv-back-callsign-left-version-right = Zurück zum Rufzeichen links und zur Version rechts
sstv-waiting-station-s-digital-mode = Warte, bis die Einstellungen der digitalen Betriebsart der Station geladen sind…
sstv-rifp = RIFP
sstv-size = Größe
sstv-calling-frequency-draft-names = Die Anrufsfrequenz, die der Entwurf nennt
sstv-encode = Kodierung
sstv-how-picture-encoded-into-object = Wie das Bild in das Objekt kodiert wird, das RIFP trägt. Auto versucht jede und sendet die kleinste.
sstv-try-every-encoding-send-smallest = Jede Kodierung versuchen, die kleinste senden (nie verlustbehaftet)
sstv-gray = Grau
sstv-grayscale-depth-rifp-s-raster = Graustufentiefe. Das Raster von RIFP ist per Definition Graustufen — Farbe hat in seinem Manifest keinen Platz.
sstv-dither = Dither
sstv-diffuse-quantisation-error-worth-below = Quantisierungsfehler streuen — lohnt sich unterhalb von 8 Bit
sstv-caption = Bildunterschrift
sstv-sent-manifest-content-hint-shown = Im Manifest als Inhaltshinweis gesendet und von Empfängern unter dem Bild angezeigt. Er reist als Text — er wird nicht ins Bild gezeichnet.
sstv-what-picture = Was dieses Bild ist
sstv-repeat-data = Datenwiederholung
sstv-send-every-data-frame-many = Jeden Datenrahmen so oft senden. RIFP ist einseitig, ohne Reparaturanfragen, daher ist dies die einzige Wiederherstellung, die ein Empfänger bekommt.
sstv-chunk = Teil
sstv-payload-octets-per-data-frame = Nutzdaten-Oktette je Datenrahmen (das Profil empfiehlt 192)
sstv-valid-frames-frames-failed-crc = Gültige Rahmen, Rahmen, die die CRC nicht bestanden haben, und vollständig geprüfte Bilder
sstv-no-transfer-progress = keine Übertragung läuft
sstv-forget-incomplete-transfer = Diese unvollständige Übertragung vergessen

## Memories and voice keyer windows (src/app/windows.rs)

windows-recall = Abrufen
windows-delete = Löschen
windows-edit-name-frequency-mode-antenna = Name, Frequenz, Betriebsart, Antenne und Relais-Einstellung bearbeiten
windows-r = {" "}R
windows-repeater-shift-tone-1750-hz = Der Relais-Versatz, der Ton und der 1750-Hz-Burst, die mit diesem Kanal gespeichert sind
windows-abandon-edit-escape = Die Bearbeitung verwerfen (Escape)
windows-save = SPEICHERN
windows-keep-changes-enter = Die Änderungen behalten (Enter)
windows-recall-channel-without-moving-antenna = Diesen Kanal abrufen, ohne die Antenne zu bewegen
windows-antenna-switch-when-channel-recalled = Die Antenne, auf die beim Abrufen dieses Kanals umgeschaltet wird. — lässt, worauf das Funkgerät bereits steht.
windows-shift = Versatz
windows-tone = Ton
windows-invert = INVERTIERT
windows-normal = NORMAL
windows-dcs-polarity = DCS-Polarität
windows-open-every-over-tone-burst = Jede Aussendung mit dem Tonburst öffnen, solange dieser Kanal abgerufen ist
windows-memories = Speicher
windows-memory-name = Speichername
windows-store = Speichern
windows-folder-name = Ordnername
windows-new-folder = Neuer Ordner
windows-import = IMPORTIEREN
windows-read-channel-list-chirp-csv =
    Eine Kanalliste aus einer CHIRP-CSV-Datei (.csv) lesen — ein Export eines Relaisverzeichnisses, eine Kanaltabelle für Seefunk oder PMR oder alles andere, was CHIRP schreiben kann.

    Jeder Kanal bringt seine Frequenz, Betriebsart, seinen Relais-Versatz und seinen CTCSS- oder DCS-Ton mit. Kanäle, die schon auf dieser Liste stehen — gleiche Frequenz, gleiche Betriebsart —, werden übersprungen, daher fügt ein erneutes Importieren eines aktualisierten Verzeichnisses hinzu, was neu ist, statt zu verdoppeln, was es nicht ist.
windows-chirp-csv = CHIRP-CSV
windows-export = EXPORTIEREN
windows-write-list-out-chirp-csv = Diese Liste als CHIRP-CSV-Datei schreiben, zum Laden in ein Handfunkgerät oder als Sicherung.
windows-clear = LEEREN
windows-clear-sure = SICHER?
windows-clear-hint = Alle Speicherkanäle löschen. Einmal klicken, dann binnen fünf Sekunden noch einmal (der Knopf zeigt SICHER?); Ordner bleiben.
windows-no-memories-yet = noch keine Speicher
windows-drag-memory-onto-folder-file = einen Speicher auf einen Ordner ziehen, um ihn einzuordnen; darunter, um ihn herauszunehmen
windows-drop-here-unfile = hier ablegen, um ihn herauszunehmen
windows-sort = Sortieren
windows-order-they-were-stored = In der Reihenfolge, in der sie gespeichert wurden
windows-name-ignoring-case = Nach Name, ohne Beachtung der Groß-/Kleinschreibung
windows-frequency = Nach Frequenz
windows-band-then-frequency-inside-each = Nach Band, dann nach Frequenz innerhalb jedes Bandes
windows-descending-click-ascending = Absteigend — klicken für aufsteigend
windows-ascending-click-descending = Aufsteigend — klicken für absteigend
windows-delete-folder-memories-move-top = Ordner löschen — seine Speicher rücken in die oberste Ebene
windows-rename-folder = Ordner umbenennen
windows-empty-drop-memories-here = leer — Speicher hier ablegen
windows-voice-keyer = Sprachkeyer
windows-rec-records-your-microphone-play = AUFN. nimmt von Ihrem Mikrofon auf, ABSPIELEN lässt Sie hören, was Sie aufgenommen haben, SENDEN bringt es auf Sendung — ebenso eine Taste des Ziffernblocks, ein MIDI-Pad oder send_voice_mem von rigctld.
windows-stop-store = Stoppen und speichern
windows-stop = STOPP
windows-play = ABSPIELEN
windows-stop-listening = Nicht mehr zuhören
windows-nothing-recorded-slot = In diesem Platz ist nichts aufgenommen
windows-not-while-transmitting = Nicht während des Sendens
windows-listen-message-nothing-transmitted = Diese Nachricht anhören — nichts wird gesendet
windows-stop-transmitting = Senden beenden
windows-tune-active-switch-off-first = ABSTIMMEN ist aktiv — zuerst ausschalten
windows-transmit-message = Diese Nachricht senden
windows-erase-recording = Diese Aufnahme löschen
windows-rade-message-encoded-digital-voice = RADE: Die Nachricht wird vom Codec für digitale Sprache kodiert, genau wie eine Live-Aussendung.

## TX Audio window: messages with values

txaudio-rec-hint = Eine Testphrase vom Mikrofon aufnehmen — bis zu { $max } s, ohne den Sender zu tasten. Erneut klicken, um zu stoppen. Sie wird unbearbeitet gehalten, damit ABSPIELEN dieselbe Phrase durch jede Änderung schicken kann, die Sie vornehmen.
txaudio-phrase-length = Phrase von { $secs } s
# $clip is txaudio-microphone-may-clipping-before-gets or empty.
txaudio-auto-set-done = Eingangsverstärkung auf { $gain } dB gesetzt (die Sprachspitzen lagen bei { $loud } dBFS).{ $clip }
txaudio-listening = Höre zu… { $secs } s
txaudio-auto-set-hint = Normal sprechen, { $secs } s lang — so wie auf dem Band —, dann wird die Eingangsverstärkung so gesetzt, dass Ihre Stimme etwa bei { $target } dBFS spitzt. Erneut klicken, um abzubrechen.
txaudio-eq-point = { $hz } Hz, { $db } dB — ziehen zum Verschieben
txaudio-not-a-profile = Kein TX-Audio-Profil: { $error }
txaudio-imported = „{ $name }“ importiert.
txaudio-saved = „{ $name }“ gespeichert.
txaudio-deleted = „{ $name }“ gelöscht.
txaudio-loaded-preset = Der Ausgangspunkt „{ $name }“ wurde geladen.

## Solar-system 3D view: messages with values

solar-age-offline = { $age } · offline
solar-radius-exaggeration = Überhöhung des Erd-/Mondradius (maximal { $max }× bei dieser Mondbahn-Skala)
solar-brightest-cell = hellste Zelle ≈ { $peak } Wege · { $bands } { $bands ->
        [one] Band
       *[other] Bänder
    }
solar-hits-in-hour = { $hits } in der Stunde
solar-passes-from = { $name } · ÜBERFLÜGE VON { $grid }
solar-elements-age = Elemente { $age } alt · SGP4
solar-point-at = Ausrichten auf { $az }° ({ $compass }), Elevation { $el }°.
solar-published-as = veröffentlicht als { $name }
solar-fly-there = { $name } — ↵ um dorthin zu fliegen
solar-sats-tracked = { $n } von { $total } verfolgt
solar-bodies-found = { $n } von { $total } Körpern
solar-sats-and-bodies = { $n } von { $total } verfolgt · { $bodies } von { $bodies_total } Körpern
solar-confidence-on-band = { $confidence } auf { $band }
solar-kp-now = { $kp } jetzt
solar-kp-in = { $kp } in { $hours } h
solar-valid-age = gültig { $valid } · { $age } alt
solar-full-bar = voller Balken = { $share } der Welt
solar-own-paths-memory = wohin die eigenen Wege dieser Station gingen · { $minutes } min Gedächtnis
solar-n-storms = { $n } Stürme
solar-clouds-legend = WOLKEN { $time } · { $age } alt · { $channels } · { $storms } · Blitz simuliert
solar-cme-eta = Ankunft { $eta } (+{ $hours } h)
solar-cme-arrived = Ankunft war { $eta } (vor { $hours } h)
# $glancing and $estimated are solar-glancing / solar-direction-estimated or empty.
solar-cme-earth-directed = AUF DIE ERDE GERICHTETE KMA { $time } · { $speed } km/s · { $when }{ $glancing }{ $estimated }

## Digital-mode setup dialogs: messages with values

setup-mode-setup = Einrichtung { $mode }
setup-transmits-as = Sendet als { $call }
setup-decimal-degrees = Dezimalgrad, negativ für { $hemisphere }. Einfach direkt aus einer Karte einfügen.

## AMP window: messages with values

amp-unreliable-at-swr = { $meters } BEI DIESEM SWR UNZUVERLÄSSIG
amp-lower-drive-first = Zuerst die Aussteuerung senken: { $drive } W würden den Verstärker im Betrieb übersteuern (seine Grenze liegt bei { $limit } W).
amp-live-value = live { $value } { $unit }
amp-tripped = AUSGELÖST{ $why }

## Digital-mode panels, shared: messages with values

panels-top-paths = oben ≈ { $n } Wege
panels-no-agreed-frequency = Für { $mode } ist keine Frequenz vereinbart — öffnen, um die aktuelle zu speichern
panels-mode-frequencies-hint = Die { $n } Frequenzen für { $mode } — eine zu wählen stimmt die Skala ab, und Sie können Ihre eigene speichern
panels-mode-frequencies = { $mode } · Frequenzen
panels-global-convention = Eine weltweite Konvention, die der Bandplan der IARU-Region { $region } für schmalbandige Daten nicht vorsieht — vor dem Senden hier den eigenen Bandplan prüfen.
panels-outside-data-segment = Bernstein: außerhalb des Datensegments der Region { $region }.
panels-save-dial = ＋ { $mhz } MHz speichern
panels-save-dial-hint = Diese Skala unter { $mode } merken und ab jetzt hier anbieten. Auf der Station gespeichert, daher hat jeder Bildschirm, der an dieses Funkgerät angeschlossen ist, sie.
panels-tr-period = { $period }-Sekunden-Sende-/Empfangsperiode
# $mark is panels-burst-mark while transmitting, otherwise empty.
panels-this-turn = Dieser Durchgang: { $into } s von { $slot }. Dekodierungen kommen am Ende an, und dann darf die nächste Aussendung beginnen.{ $mark }
panels-burst-mark = {" "}Die Markierung zeigt, wo der Burst endet, nach { $secs } s.
panels-tr-period-burst = { $slot }-Sekunden-Sende-/Empfangsperiode, { $burst } s Burst

## Memories and voice keyer windows: messages with values

windows-n-channels = { $n } { $n ->
        [one] Kanal
       *[other] Kanäle
    }
windows-slot-n = Platz { $n }
windows-record-from-mic = Vom Mikrofon aufnehmen (bis zu { $max } s)

## SSTV panel: messages with values

sstv-picture-too-big = Dieses Bild ist { $size } MB groß — die Grenze liegt bei { $limit } MB.
sstv-auto-mode = Auto ({ $mode })
sstv-not-decoded = { $what } — nicht dekodiert
sstv-last = zuletzt: { $mode }
# $place is sstv-store-on-radio or sstv-store-in.
sstv-saved-where = Dekodierte Bilder werden { $place } gespeichert und sammeln sich hier.
sstv-older-in-store = { $n } ältere im Speicher
sstv-older-load-more = { $n } ältere — mehr laden
sstv-delete-picture-from = Dieses Bild löschen { $place }
sstv-rifp-profile = { $profile } · belegt etwa { $khz } kHz · Skala ist die Kanalmitte
sstv-rifp-too-wide = ⚠ { $profile } belegt etwa { $khz } kHz — zu breit für ein Schmalband-Segment
sstv-rifp-no-frequency = RIFP legt keine Frequenz fest, und sdroxide sendet es, wo immer Sie abstimmen. Ein Kanal von { $khz } kHz passt nur dort, wo Breitband- oder FM-Betrieb erlaubt ist — { $segments } — und nicht in einem Schmalband-Segment, am wenigsten auf KW. Selbst dort können die Bedingungen Ihrer eigenen Lizenz enger sein. Sie sind der Betreiber; prüfen Sie Ihre eigenen Regeln.
sstv-rifp-tx-frame = ● TX-Rahmen { $frame }/{ $frames } · { $secs } s übrig
sstv-rifp-sent-as = gesendet als { $encoding } · { $octets } Oktette
sstv-rifp-rx-stats = Rahmen { $frames } · fehlerhaft { $bad } · Bilder { $pictures }
sstv-rifp-session-idle = Sitzung { $session } · untätig { $secs } s
sstv-rifp-session-waiting = Sitzung { $session } · Teile gehalten, warte noch auf das Manifest · untätig { $secs } s
sstv-store-on-radio = auf dem Funkgerät, in { $dir }
sstv-store-in = in { $dir }

## decodes (src/app/panels/decodes.rs)

decodes-map-window = Weltkarte (FT8/FT4)
decodes-map-pop-out-tip = Die Weltkarte: im eigenen Fenster öffnen (beliebig groß, auch in der kompakten Anordnung) und Städte, Wer spricht mit wem und Rufzeichen an den Punkten schalten.
decodes-map-cities-tip = Städte auf dieser Karte ein oder aus. Die anderen Karten folgen Einstellungen → Oberfläche.
decodes-map-links-tip = Wer mit wem spricht: eine gepunktete Linie zwischen den beiden Stationen jedes Austauschs der letzten zehn Minuten, wenn beide einen Locator gesendet haben. Verblasst mit dem Alter.
decodes-map-labels-tip = Rufzeichen neben den Stationspunkten, sie verblassen mit.
decodes-map-pick-gone = Keine Dekodierung dieser Station mehr in der Liste; auf der Karte steht sie noch von vorhin.
decodes-map-menu = KARTE ▼
decodes-map-open-window = Im Fenster öffnen
decodes-map-item-cities = Städte
decodes-map-item-links = Wer spricht mit wem
decodes-map-item-labels = Rufzeichen an den Punkten
decodes-map-pick-new-entity = neues Land
decodes-map-pick-new-grid = neues Feld
decodes-map-pick-new-call = neues Rufzeichen
decodes-map-pick-worked = schon gearbeitet
decodes-decodes = DEKODIERUNGEN
decodes-adif = ADIF
decodes-sort = Sortieren
decodes-single-list = Eine Liste
decodes-every-decode-one-list-newest = Jede Dekodierung in einer Liste, der neueste Durchgang zuerst, statt gerade/ungerade Durchgangsblöcke. Die Sortierchips ordnen dann die ganze Liste statt jedes Durchgangs, und jede Zeile trägt ihre Zeitschlitzzeit — Cyan für einen geraden Schlitz, Gold für einen ungeraden.
decodes-cq-only = Nur CQ
decodes-only-stations-calling-cq-only = Nur Stationen, die CQ rufen — und nur die Rufe, auf die Sie antworten dürfen: Ein gerichtetes CQ (DX, EU, JA, POTA, TEST …) wird aufgelistet, wenn es Sie nennt, und ausgeblendet, wenn es jemand anderen nennt.
decodes-new-only = Nur neue
decodes-only-stations-would-new-entity = Nur Stationen, die neu wären: Gebiet, Band-Platz, Locator oder Rufzeichen
decodes-auto-tx-frq = Auto-Sendefrequenz
decodes-overridden-hold-tx-lift-hold = Von Sendefrequenz halten überschrieben. Das Halten aufheben, um zu wählen, in welche Richtung die Sendefrequenz wandert.
decodes-pick-our-transmit-frequency-automatically = Unsere Sendefrequenz automatisch wählen: die ruhigste Stelle in der Periode, in der wir senden, statt die Frequenz desjenigen, den wir beantworten — die senden in der anderen Periode, daher sagt deren Frequenz nichts darüber, wer da ist, wenn wir tasten. Aus hält die Frequenz NICHT fest: Dann wird auf der Frequenz der gerufenen Station geantwortet. Zum Festhalten Sendefrequenz halten verwenden.
decodes-hold-tx = Sendefrequenz halten
decodes-pin-transmit-tone-where-nothing =
    Den Sendeton dort festhalten, wo er ist. Nichts verschiebt ihn: nicht das Antworten auf eine Station, nicht die Anrufwarteschlange, nicht CQ rufen, nicht ein Klick auf eine Dekodierung oder auf den Wasserfall. Zum Verschieben ausschalten und dann wieder einschalten.

    Ein Bandwechsel ist die eine Ausnahme, und er ist Ihre eigene Handlung: Der Versatz, den Sie zuletzt auf dem neuen Band gesetzt haben, kommt damit zurück, daher wird ein Wert für 60 m nicht auf 20 m übertragen und der von 20 m nicht auf 60 m.

    Für den Fall, dass die Lizenz schmaler ist als der Bandplan — auf einer britischen 60-m-Skala von 5357 kHz endet die Zuteilung bei 5358,0, daher muss der Ton unter 1000 Hz bleiben, und jede der beiden automatischen Verschiebungen wandert zwischen einer Aussendung und der nächsten aus dem Band.
decodes-type-transmit-offset-hz-then = Den Sendeversatz in Hz eintippen, dann Enter drücken. Wo Ihre Lizenz schmaler ist als der Bandplan, liegt der ganze Bereich unterhalb der Grenze zur Auswahl bei Ihnen, daher wird hier nichts vorgeschlagen.
decodes-clear-rx = {" "}EMPFANG LEEREN{" "}
decodes-empty-decode-list-nothing-air = Die Dekodierliste leeren. Nichts, was auf Sendung ist, hört auf.
decodes-even = GERADE
decodes-text = TEXT
decodes-dxcc = DXCC
decodes-band = BAND
decodes-grid = LOCATOR
decodes-dupe = DUPLIKAT
decodes-reply = ANTWORT
decodes-queued-click-remove = In der Warteschlange — klicken zum Entfernen
decodes-work-station-after-current-one = Diese Station nach der aktuellen arbeiten
decodes-qsos-worked-since-sdroxide-was = QSOs, seit sdroxide gestartet wurde
decodes-setup = ⚙ EINRICHTUNG
decodes-qso-complete = ✓ QSO ABGESCHLOSSEN
decodes-contact-complete-log-held-here = Der Kontakt ist abgeschlossen und im Logbuch. Er wird hier ein paar Minuten gehalten, damit die letzte Nachricht erneut gesendet werden kann, falls die andere Station ihre wiederholt — mehr ist nicht ausstehend.
decodes-tx = ● TX
decodes-dxpedition-mode-transmit-frequency-held = DXpedition-Modus. Die Sendefrequenz wird aus der Fox-Hälfte des Durchlassbereichs (unter 1000 Hz) herausgehalten, bis der Fox antwortet.
decodes-watchdog = WATCHDOG
decodes-transmitting-stopped-no-reply-no = Senden gestoppt: keine Antwort und keine Aktion während der Watchdog-Zeit. CQ rufen oder eine Nachricht wählen, um fortzufahren.
decodes-well-inside-tolerance = Gut innerhalb der Toleranz.
decodes-no-active-qso-pick-decode = kein aktives QSO — eine Dekodierung zum Antworten wählen oder CQ rufen
decodes-ft8-engine-idle = FT8-Engine untätig
decodes-being-worked = wird gearbeitet
decodes-clear = LEEREN
decodes-no-messages = — keine Nachrichten —
decodes-call-cq = {"  "}CQ RUFEN{"  "}
decodes-stop-qso = {" "}QSO STOPPEN{" "}
decodes-stop-tx = {" "}SENDEN STOPPEN{" "}
decodes-free-text-13-chars = Freitext (13 Zeichen)
decodes-send = SENDEN

## sat (src/app/sat.rs)

sat-pass-profile = ÜBERFLUGPROFIL
sat-satellite = SATELLIT
sat-satellites = SATELLITEN
sat-passes = ÜBERFLÜGE
sat-set-your-grid-passes-settings = Den Locator für Überflüge setzen (Einstellungen ▸ Allgemein).
sat-next = NÄCHSTER
sat-peak = SPITZE ≥
sat-all-satellites = alle Satelliten
sat-off-only-satellites-frequencies-file = Aus: nur Satelliten mit hinterlegten Frequenzen — die, mit denen man arbeiten kann — und die, die Sie selbst eingefügt haben. Ein: jeder Satellit der Liste, was mit einem Abonnement Tausende sein können und eine Weile zum Durchsuchen braucht.
sat-no-passes-window-lower-peak = Keine Überflüge in diesem Fenster. Die Spitze senken oder das Fenster erweitern.
sat-length = DAUER
sat-peak-2 = SPITZE
sat-from-to = VON → BIS
sat-up-now = ● jetzt oben
sat-unlock = ENTRASTEN
sat-release-lock-dial-stays-where = Die Verfolgung lösen; die Skala bleibt, wo sie ist
sat-azimuth = AZIMUT
sat-elevation = ELEVATION
sat-range = ENTFERNUNG
sat-rate = RATE
sat-downlink = ABWÄRTSSTRECKE
sat-doppler-rx = DOPPLER RX
sat-uplink = AUFWÄRTSSTRECKE
sat-doppler-tx = DOPPLER TX
sat-no-pass-inside-next-48 = Kein Überflug innerhalb der nächsten 48 Stunden von Ihrem QTH.
sat-elements-stale-corrections-suspended-until = ⚠ Elemente veraltet — Korrekturen ausgesetzt, bis ein TLE aktualisiert wird
sat-doppler-correction-needs-iq-front = Die Doppler-Korrektur braucht eine IQ-Frontstufe — ein CAT-Gerät verfolgt nur
sat-rotator-not-connected = Rotor: nicht verbunden
sat-doppler = DOPPLER
sat-apply-computed-correction-rx-tx = Die berechnete Korrektur auf Empfang und Senden anwenden
sat-rotator = ROTOR
sat-steer-antenna-through-rotctld-client = Die Antenne über den rotctld-Client steuern (Einstellungen ▸ Server)
sat-name-norad = Name oder NORAD
sat-set-your-grid-passes-settings-2 = den Locator für Überflüge setzen (Einstellungen ▸ Allgemein)
sat-norad = NORAD
sat-pass = ÜBERFLUG
sat-always-visible = immer sichtbar
sat-not-here = nicht von hier
sat-no-elements = keine Elemente
sat-pick-satellite-see-links-lock = Einen Satelliten wählen, um seine Verbindungen zu sehen und sich einzurasten.
sat-no-frequencies-file-one-add = Keine Frequenzen für diesen hinterlegt — unter Einstellungen ▸ TLE hinzufügen.
sat-link = VERBINDUNG
sat-downlink-mhz = ABWÄRTSSTRECKE (MHz)
sat-uplink-mhz = AUFWÄRTSSTRECKE (MHz)
sat-mode = BETRIEBSART
sat-tune = ABSTIMMEN
sat-set-dial-mode-link-no = Skala und Betriebsart auf diese Verbindung setzen — keine Verfolgung, keine Korrektur
sat-locked = {" "}EINGERASTET{" "}
sat-lock = {" "}EINRASTEN{" "}
sat-track-satellite-doppler-correction-rx = Diesen Satelliten verfolgen: Doppler-Korrektur auf Empfang und Senden, auf den Transponder abgebildeter Uplink, und die 3D-Ansicht folgt ihm

## wefax (src/app/panels/wefax.rs)

wefax-wefax = WEFAX
wefax-stop = {" "}■ STOPP{" "}
wefax-end-chart-now-save-what = Die Karte jetzt beenden und speichern, was angekommen ist
wefax-start = {" "}● START{" "}
wefax-start-chart-now-without-waiting = Eine Karte jetzt starten, ohne auf einen Startton zu warten
wefax-phasing = Phasierung…
wefax-subcarrier-offset-1900-hz-tune = Unterträgerversatz von 1900 Hz. Auf ungefähr null abstimmen: Der Faxträger liegt bei 1500 Hz für Schwarz und 2300 Hz für Weiß, und ein Empfänger, der einige hundert Hertz daneben liegt, schneidet das Bild zu vollem Schwarz oder vollem Weiß ab.
wefax-auto-start = AUTOMATISCH STARTEN
wefax-begin-chart-when-300-hz = Eine Karte beginnen, wenn der 300-Hz-Ton (IOC 576) oder der 675-Hz-Startton zu hören ist — und die laufende beenden und die neue phasieren, sobald die nächste Aussendung ankommt, ob oder ob nicht vorher ein Stopp-Ton kam.
wefax-auto-stop = AUTOMATISCH STOPPEN
wefax-end-450-hz-stop-tone = Sie am 450-Hz-Stopp-Ton beenden. Ausschalten, um durch eine Station hindurch weiter aufzunehmen, die mehrere Karten hintereinander sendet.
wefax-phase = PHASE
wefax-slant = SCHRÄGLAGE
wefax-sample-clock-trim-if-chart = Trimm des Abtasttakts. Neigt sich die Karte nach links, erhöhen; nach rechts, verringern. Eine Soundkarte, die hundert ppm daneben liegt, wandert bei einer Viertelstunden-Karte um die meiste Zeile seitlich.
wefax-view = ANSICHT
wefax-scale-chart-panel-width = Die Karte auf die Breite des Panels skalieren
wefax-whole = GANZ
wefax-shrink-chart-until-all-view = Die Karte so verkleinern, dass alles auf einmal zu sehen ist
wefax-half-size = Halbe Größe
wefax-one-screen-pixel-per-fax = Ein Bildschirmpixel je Faxpixel — scrollen für Details
wefax-twice-size-scroll-move-around = Doppelte Größe; scrollen, um sich auf der Karte zu bewegen
wefax-height = HÖHE
wefax-stretch-picture-vertically-chart-came = Das Bild vertikal strecken. Eine Karte, die gestaucht oder gedehnt herauskam, wird meist mit der falschen Zeilenrate dekodiert — das macht sie lesbar, und die LPM-Chips beheben es richtig. Doppelklicken, um einen Wert einzutippen.
wefax-reset = ZURÜCKSETZEN
wefax-back-picture-s-own-proportions = Zurück zu den eigenen Proportionen des Bildes
wefax-follow = FOLGEN
wefax-keep-newest-lines-view-scrolling = Die neuesten Zeilen im Blick behalten. Nach oben scrollen schaltet dies aus, damit Sie lesen können, was schon angekommen ist; zurück nach unten scrollen schaltet es wieder ein.
wefax-tune-fax-schedule-usb-wait = Einen Fax-Fahrplan in USB abstimmen und auf einen Startton warten, oder START drücken, um mitten in einer Karte zu beginnen.
wefax-saved = GESPEICHERT
wefax-path = PFAD
wefax-reading-radio-s-charts = Die Karten des Funkgeräts werden gelesen…
wefax-finished-charts-collect-here = Fertige Karten sammeln sich hier.
wefax-delete-chart = Diese Karte löschen
wefax-loading = lädt…
wefax-stations = STATIONEN
wefax-broadcast-radiofax-schedules-picking-one = Rundfunk-Radiofax-Fahrpläne — eine zu wählen stimmt die Skala ab
wefax-frequencies-are-published-carrier-dial = Die Frequenzen sind die veröffentlichten Träger; die Skala liegt 1,9 kHz darunter, was für Sie erledigt wird. Fahrpläne ändern sich und Stationen schalten ab — das hier als Ort nehmen, wo man zu suchen beginnt, nicht als Zeitplan.
wefax-delete-chart-store = Diese Karte aus dem Speicher löschen
wefax-newer = ◀ NEUER
wefax-chart-received-after-one = Die Karte, die nach dieser empfangen wurde
wefax-older = ÄLTER ▶
wefax-chart-received-before-one = Die Karte, die vor dieser empfangen wurde
wefax-no-longer-store = nicht mehr im Speicher
wefax-loading-full-size = lädt in voller Größe…
wefax-save = SPEICHERN
wefax-save-copy-chart-computer = Eine Kopie dieser Karte auf diesem Computer speichern
wefax-sure = SICHER?
wefax-click-again-delete-chart-good = Erneut klicken, um diese Karte endgültig zu löschen
wefax-delete = LÖSCHEN
wefax-file-chart-was-saved = Die Datei, unter der diese Karte gespeichert wurde

## aprs (src/app/panels/aprs.rs)

aprs-starting-aprs-modem = APRS-Modem wird gestartet…
aprs-aprs = APRS
aprs-no-callsign = kein Rufzeichen
aprs-nothing-will-transmitted-no-beacon = Es wird nichts gesendet — keine Bake, keine Nachricht, nicht einmal eine Bestätigung — bis diese Station ein Rufzeichen hat. Eines unter Einstellungen → Allgemein setzen, oder ein APRS-eigenes mit seiner SSID unter EINRICHTUNG.
aprs-no-position = keine Position
aprs-beacon-needs-somewhere-report-fill = Eine Bake braucht einen Ort, an den sie meldet. Den Locator unter Einstellungen → Allgemein eintragen oder unter EINRICHTUNG Koordinaten angeben.
aprs-setup = EINRICHTUNG
aprs-callsign-symbol-digipeater-path-position = Rufzeichen, Symbol, Digipeater-Pfad, Position und Bake
aprs-how-often-beacon-your-position = Wie oft Ihre Position unbeaufsichtigt als Bake gesendet wird. `aus` — der Standard — sendet nie eine Bake: Die Wahl einer Betriebsart darf keine Station auf Sendung bringen. Dreißig Minuten sind die Konvention für eine feste Station, öfter für eine bewegte, und jede Bake ist Sendezeit auf dem Kanal eines anderen.
aprs-beacon = {" "}BAKE{" "}
aprs-station-has-no-callsign-see = Diese Station hat kein Rufzeichen — siehe die Warnung links.
aprs-station-has-no-position-report = Diese Station hat keine Position zu melden — siehe die Warnung links.
aprs-send-one-position-report-now = Jetzt eine Positionsmeldung senden, ohne auf das Intervall zu warten. Sie wartet wie alles andere, bis der Kanal frei ist.
aprs-stations = STATIONEN
aprs-nothing-heard-yet = noch nichts gehört
aprs-heard-direct = direkt gehört
aprs-two-characters-station-sent-say = Die zwei Zeichen, die die Station sendete, um zu sagen, was sie ist
aprs-killed = GELÖSCHT
aprs-station-put-object-here-has = Die Station, die dieses Objekt hier eingetragen hat, hat es zurückgezogen
aprs-sender-deliberately-reported-square-rather = Der Absender hat bewusst ein Quadrat statt eines Punktes gemeldet. Die Karte zeichnet das Quadrat.
aprs-digipeaters-last-frame-came-through = Die Digipeater, über die der letzte Rahmen kam. Ein `*` markiert einen, der ihn tatsächlich weitergeleitet hat.
aprs-message = {" "}NACHRICHT{" "}
aprs-address-message-box-station = Das Nachrichtenfeld an diese Station adressieren
aprs-center = {" "}ZENTRIEREN{" "}
aprs-put-station-middle-map-double = Diese Station in die Mitte der Karte setzen. Doppelklick auf die Karte gibt die Ansicht wieder an die automatische Einpassung zurück.
aprs-channel = KANAL
aprs-messages = NACHRICHTEN
aprs-frames-arrived-failed-their-check = Rahmen, die ankamen und ihre Prüfsequenz nicht bestanden — eine Kollision, ein Schwund oder ein Signal, das zu schwach zum Lesen ist. Ein steigender Zähler, während nichts dekodiert wird, heißt, dass das Modem den Kanal *hört*: den Pegelmesser prüfen und die Rauschsperre des Funkgeräts öffnen.
aprs-frames-read-cleanly-off-channel = Rahmen, die sauber vom Kanal gelesen wurden und kein APRS waren — die Paketsitzung einer anderen Station oder ein Format, das dieser Build nicht dekodiert.
aprs-every-frame-channel-arrived-other = Jeder Rahmen auf dem Kanal, so wie er ankam — auch der Verkehr anderer und alles, was der Decoder nicht lesen konnte.
aprs-nothing-channel-yet = noch nichts auf dem Kanal
aprs-no-messages = Keine Nachrichten.
aprs-waiting-channel = warte auf den Kanal
aprs-sent-retried-still-waiting-acknowledged = gesendet und wiederholt, wartet noch auf Bestätigung
aprs-air-waiting-acknowledged = auf Sendung, wartet auf Bestätigung
aprs-acknowledged-far-end = von der Gegenseite bestätigt
aprs-refused-far-end-not-retried = von der Gegenseite abgelehnt — nicht wiederholt
aprs-no-ack = keine Bestätigung
aprs-no-answer-after-every-retry = keine Antwort nach allen Wiederholungen
aprs-send = {" "}SENDEN{" "}
aprs-station-has-no-callsign-so = Diese Station hat kein Rufzeichen, daher kann nichts gesendet werden. Eines unter Einstellungen → Allgemein setzen, oder ein APRS-eigenes mit seiner SSID unter EINRICHTUNG.
aprs-needs-station-address-something-say = Braucht eine Station als Adresse und etwas zu sagen.
aprs-send-keep-retrying-until-far = Senden, und so lange wiederholen, bis die Gegenseite bestätigt. Nachrichten sind das Einzige auf diesem Kanal, das beantwortet wird.
aprs-busy = BESETZT
aprs-stations-heard-still-inside-window = Stationen, die gehört wurden und noch im Fenster liegen, das unter APRS-Einrichtung gesetzt ist, und ob gerade eine andere Station auf dem Kanal ist. Solange er besetzt ist, tastet nichts.
aprs-tx = ● TX
aprs-over-progress-frames-waiting-channel = Die laufende Aussendung, die Rahmen, die darauf warten, dass der Kanal frei wird, oder die Zeit bis zur nächsten geplanten Bake.
aprs-audio-reaching-modem-flat-means = Audio, das das Modem erreicht. Flach heißt, dass gar nichts ankommt — prüfen, ob der Datenausgang des Funkgeräts der ist, den sdroxide abhört. Er leuchtet grün, während das Modem einen Träger hört.

## vdl2 (src/app/panels/vdl2.rs)

vdl2-starting-vdl2-decoder = VDL2-Decoder wird gestartet…
vdl2-aircraft-datalink = Luftfahrt-Datenverbindung
vdl2-tune-middle-vdl2-group-decoder = Auf die Mitte der VDL2-Gruppe abstimmen. Das eigene Fenster des Decoders gleitet von dort, um so viele der vierzehn Kanäle aufzunehmen, wie der Empfänger erreicht.
vdl2-setup = EINRICHTUNG
vdl2-which-channels-listen-how-hard = Welche Kanäle abgehört werden, wie stark ein Burst sein muss und wie viel Protokoll behalten wird
vdl2-save-vdl2-message-log-file = Das VDL2-Nachrichtenprotokoll in eine Datei speichern
vdl2-vdl2-channel = VDL2-Kanal
vdl2-messages = NACHRICHTEN
vdl2-address-flight-registration-label = Adresse, Flug, Kennung oder Label
vdl2-held = GEHALTEN
vdl2-hold = HALTEN
vdl2-hold-message-log-where-so = Das Nachrichtenprotokoll dort halten, wo es ist, damit eine Nachricht gelesen werden kann, ohne dass die nächste Aussendung sie den Bildschirm hinaufschiebt. Der Decoder läuft weiter und die Zähler bewegen sich weiter; loslassen zeigt alles, was in der Zwischenzeit ankam.
vdl2-nothing-decoded-yet = noch nichts dekodiert
vdl2-nothing-matches-filter = nichts passt zum Filter
vdl2-stations = STATIONEN
vdl2-nobody-heard-yet = noch niemand gehört
vdl2-vdl2-setup = VDL2-Einrichtung
vdl2-channels = Kanäle
vdl2-every-25-khz-slot-136 = Jeder 25-kHz-Platz von 136,650 bis 136,975 MHz. Je ein Abwärtsmischer, alle im selben Empfängerfenster. Einen davon auszuschalten spart etwas Prozessorzeit; die anderen werden dadurch nicht empfindlicher.
vdl2-burst-threshold = Burst-Schwelle
vdl2-above-each-channel-s-own = über dem eigenen Rauschboden jedes Kanals
vdl2-lower-catches-weaker-transmissions-costs = Niedriger fängt schwächere Aussendungen ein und kostet Prozessorzeit beim Rauschen; höher verpasst sie. Der Boden wird je Kanal gelernt, und eine Änderung hier wirft ihn nicht weg.
vdl2-keep-log = Im Protokoll behalten
vdl2-messages-2 = {" "}Nachrichten
vdl2-track-most = Höchstens verfolgen
vdl2-stations-2 = {" "}Stationen
vdl2-longest-silent-go-first = die längst stillen gehen zuerst
vdl2-forget-station-after = Eine Station vergessen nach
vdl2-nothing-heard = ohne dass von ihr etwas gehört wurde
vdl2-show-unread-payloads = Ungelesene Nutzdaten anzeigen
vdl2-hex = als Hex
vdl2-frames-carrying-x-25-clnp = Rahmen mit X.25, CLNP oder den Datenverbindungs-Anwendungen darüber. SDRoxide benennt sie und zeigt die Bytes, statt sie zu lesen, und sie zu verbergen würde verbergen, wie viel des Verkehrs das ist.
vdl2-registration = Kennung
vdl2-flight = Flug
vdl2-label = Label
vdl2-block = Block
vdl2-sequence = Folge
vdl2-mode = Betriebsart
vdl2-more-blocks-follow = weitere Blöcke folgen
vdl2-check = Prüfung
vdl2-message-crc-good = Nachrichten-CRC in Ordnung
vdl2-message-crc-not-checked = Nachrichten-CRC nicht geprüft
vdl2-parity = Parität
vdl2-link-control-no-payload = Verbindungssteuerung, keine Nutzdaten
vdl2-name = NAME
vdl2-addr = ADR
vdl2-type = TYP
vdl2-msgs = NACHR.

## cw (src/app/panels/cw.rs)

cw-tone-being-copied-tone-transmitted = Der Ton, der mitgeschrieben wird, und der gesendete Ton — bei CW ist es dieselbe Frequenz. Auf den Wasserfall klicken, um ihn auf ein Signal zu legen.
cw-down-10-hz = 10 Hz abwärts
cw-up-10-hz = 10 Hz aufwärts
cw-qrg-main-readout-tuning-line = QRG: die Hauptanzeige und die Abstimmlinie stehen auf der bearbeiteten Frequenz. Klicken für die Skala, mit einer Mithörtonhöhe darunter — was die meisten Funkgeräte zeigen.
cw-put-main-readout-tuning-line =
    Die Hauptanzeige und die Abstimmlinie auf das Signal legen statt auf die Skala, damit die angezeigte Frequenz die ist, die beide Bediener nennen würden, und die Abstimmlinie in der Mitte des Durchlassbereichs sitzt. Das Abstimmen bleibt unverändert; nur die Zahlen wandern.

    Ein Klick auf ein Signal setzt es auf den Cursor nur so genau, wie es der Klickschritt erlaubt — Bedienung → Rundung beim Abstimmen per Klick, standardmäßig 10 Hz. Ein grober Schritt lässt das Signal um bis zu die Hälfte davon neben dem Ton, und die Anzeige sagt das.
cw-sending-speed-read-off-signal = Sendegeschwindigkeit, aus dem Signal abgelesen
cw-signal-noise-500-hz-same = Signal zu Rauschen in 500 Hz — dieselbe Zahl, die ein Bericht nennt
cw-how-far-off-cursor-signal = Wie weit das Signal tatsächlich vom Cursor entfernt ist. Der Decoder folgt ihm; der Durchlassbereich nicht, daher den Cursor nachschieben, wenn es wächst.
cw-listening = — höre zu —
cw-tx = ● TX
cw-trainer = TRAINER
cw-learn-morse-read-characters-words = Morse lernen: Zeichen und Wörter lesen, sie in Ihrem eigenen Tempo abspielen und eine Koch-Übung, die jeweils ein Zeichen hinzufügt. Sie spielt hier über die Lautsprecher dieses Computers ab — nichts tastet das Funkgerät.
cw-nothing-copied-yet = — noch nichts mitgeschrieben —
cw-key-send-characters-you-send = Tasten zum Senden — die Zeichen, die Sie senden, erscheinen hier.
cw-type-line-return-sends = Eine Zeile tippen, Enter sendet sie…
cw-type-here-send = Hier tippen zum Senden…
cw-key = {" "}TASTE ●{" "}
cw-key-2 = {" "}TASTE{" "}
cw-hold-key-bound-cw-straight = Die Taste, die auf CW-Handtaste gelegt ist — standardmäßig Leertaste, und jede Taste Ihrer Wahl unter Einstellungen → Bedienung — als Handtaste halten: unten, solange sie gehalten wird, oben beim Loslassen, statt Text zu tippen. Das Sendefeld ist gesperrt, solange es an ist, und der ganze Keyer wird der Taste übergeben: Was in der Warteschlange stand, wird verworfen.
cw-radio-sends-own-keyer-text =
    Dieses Funkgerät sendet aus seinem eigenen Keyer: Der Text geht über den Steuerport, und das Gerät taktet die Elemente, daher liegt zwischen der Handtaste und der Luft nichts, was eine Hand steuern könnte.

    Zum Handtasten CW-Tastung auf „Soundkarte (MCW)“ in Einstellungen → Funkgerät setzen. Das Gerät wird dann auf einem Seitenband gehalten, und der eigene Mithörton des Keyers wird als Audio gesendet — das ist der Weg, den die Handtaste steuert.
cw-watchdog = WATCHDOG
cw-straight-key-was-held-down = Die Handtaste war zu lange gedrückt — ein verlorenes Loslassen statt einer Hand —, daher wurde der Träger abgeschaltet und das Senden ausgeschaltet. Die Taste erneut drücken, um weiterzumachen.
cw-tx-2 = {"  "}TX EIN{"  "}
cw-tx-3 = {"   "}TX{"   "}
cw-send-what-box-now-without = Jetzt senden, was im Feld steht, ohne auf Enter zu warten
cw-hold-key-down-between-characters = Die Taste zwischen den Zeichen gedrückt halten, damit nichts Getipptes wartet
cw-call-cq = {" "}CQ RUFEN{" "}
cw-nocall = NOCALL
cw-clear = {" "}LEEREN{" "}
cw-stop-sending-drop-whatever-has = Senden stoppen und verwerfen, was nicht hinausgegangen ist. Bei eingeschalteter Handtaste geht die Rückmeldung zu dem, was Sie getastet haben, mit.
cw-sidetone = MITHÖRTON
cw-play-keyed-tone-through-computer = Den getasteten Ton zusätzlich über die Lautsprecher dieses Computers abspielen, damit Sie hören, was Sie senden. Bei `Soundkarte (MCW)` geht der Ton nur zum Funkgerät und sonst nirgends, daher sendet man ohne dies in Stille. Aus, wo der eigene Monitor des Funkgeräts oder eine Kopie vom Band die Aufgabe schon erledigt.
cw-hold-what-typed-until-return = Das Getippte bis Enter festhalten und dann die Zeile in einem Stück senden, statt jedes Zeichen beim Tippen zu tasten. Lohnt sich bei einem Transceiver, der aus Text selbst tastet, wo jede Übergabe an seinen Keyer ein weiterer Sende-/Empfangszyklus ist.
cw-your-own-message-buttons-contest = Ihre eigenen Nachrichtentasten — ein Contest-Austausch, eine Antwort mit Name und QTH, TNX 73 GL. Jede sendet ihren ganzen Text auf einmal, und F2–F10 drücken die ersten neun. Sie reisen mit der Konfiguration der Station, daher hat ein entfernter Client sie auch.
cw-cw-messages = CW-NACHRICHTEN
cw-clear-rx = {" "}EMPFANG LEEREN{" "}
cw-empty-receive-window-straight-key = Das Empfangsfenster und die Rückmeldung der Handtaste leeren. Nichts, was auf Sendung ist, hört auf.
cw-lock = SPERREN
cw-decode-speed-set-here-instead = Mit der hier eingestellten Geschwindigkeit dekodieren statt sie aus dem Signal abzulesen. Hilft bei einem Signal, das für die Geschwindigkeitssuche zu schwach ist, um sich festzulegen.
cw-farnsworth-send-characters-full-speed = Farnsworth: die Zeichen mit voller Geschwindigkeit senden und nur die Pausen dazwischen dehnen, damit sie im richtigen Rhythmus gehört werden, aber langsam genug ankommen, um sie mitzuschreiben.
cw-off-normal-spacing = Aus — normaler Abstand
cw-keying-speed = Tastgeschwindigkeit
cw-idle-0 = LEERLAUF 0
cw-how-long-transmit-held-after = Wie lange das Senden nach dem letzten Zeichen oder nach dem Loslassen der Handtaste gehalten wird, bevor der Träger abfällt. Länger überbrückt die Pausen eines langsamen Tippers; kürzer kommt schneller von der Frequenz; Aus lässt ihn fallen, sobald alles in der Warteschlange hinausgegangen ist.
cw-off-drop-once = Aus — sofort abfallen lassen

## textmodem (src/app/panels/text_modem.rs)

textmodem-tune-down-10-hz = 10 Hz abwärts abstimmen
textmodem-tune-up-10-hz = 10 Hz aufwärts abstimmen
textmodem-tx = ● TX
textmodem-listening = — höre zu —
textmodem-type-line-return-sends = Eine Zeile tippen, Enter sendet sie…
textmodem-type-here-transmit = Hier tippen zum Senden…
textmodem-tx-2 = {"  "}TX EIN{"  "}
textmodem-tx-3 = {"   "}TX{"   "}
textmodem-call-cq = {" "}CQ RUFEN{" "}
textmodem-nocall = NOCALL
textmodem-clear = {" "}LEEREN{" "}
textmodem-hold-what-typed-until-return = Das Getippte bis Enter festhalten und dann die Zeile in einem Stück senden, statt jedes Zeichen beim Tippen auf den Streifen zu malen. Eine Korrektur, die live getippt wird, steht schon auf dem Papier; eine vor Enter gemachte nie.
textmodem-your-own-message-buttons-working = Ihre eigenen Nachrichtentasten — Betriebsbedingungen, das Wetter, eine Standardantwort. Jede sendet ihren ganzen Text auf einmal, und F2–F10 drücken die ersten neun. Sie reisen mit der Konfiguration der Station, daher hat ein entfernter Client sie auch.
textmodem-messages = NACHRICHTEN
textmodem-hell = HELL
textmodem-contrast = Kontrast
textmodem-harder-softer-dots-redraws-whole = Härtere oder weichere Punkte — zeichnet den ganzen Streifen neu
textmodem-width = Breite
textmodem-t-2row = {" "}2ZEILIG{" "}
textmodem-draw-every-column-twice-stacked = Jede Spalte zweimal übereinander zeichnen. Hell hat keine vertikale Synchronisation, daher bleibt so eine vollständige Kopie des Textes lesbar, egal in welcher Phase.
textmodem-rev = {" "}INV{" "}
textmodem-reverse-video-light-dots-dark = Invertierte Darstellung — helle Punkte auf dunklem Papier
textmodem-clear-rx = {" "}EMPFANG LEEREN{" "}
textmodem-hold-channel-idle-sends-blank = Den Kanal halten: Leerlauf sendet leeres Papier, damit der Streifen weiter scrollt
textmodem-hold-what-typed-until-return-2 = Das Getippte bis Enter festhalten und dann die Zeile in einem Stück senden, statt jedes Zeichen beim Tippen auf den Streifen zu malen. Eine Korrektur, die live getippt wird, steht schon auf dem Papier; eine vor Enter gemachte nie.
textmodem-shift = Versatz
textmodem-shift-5-hz = Versatz −5 Hz
textmodem-shift-5-hz-2 = Versatz +5 Hz
textmodem-baud = Baud
textmodem-reverse-swap-mark-space-needed = Invertieren: Mark und Space tauschen. Nötig, wenn das Signal auf dem anderen Seitenband empfangen wird als dem, auf dem es gesendet wurde, was die Töne umkehrt.
textmodem-track-tuning-error-automatically-instead = Den Abstimmfehler automatisch nachführen, statt am Cursor festgenagelt zu bleiben.
textmodem-tones = Töne
textmodem-mode = Betriebsart
textmodem-off-keyed = ein/aus getastet

## ais (src/app/panels/ais.rs)

ais-starting-ais-decoder = AIS-Decoder wird gestartet…
ais-tune-between-two-ais-channels = Zwischen den beiden AIS-Kanälen abstimmen — 161,975 (AIS 1, Seefunk 87B) und 162,025 (AIS 2, 88B). Die Skala liegt in der Mitte, weil dort nichts gesendet wird, daher landet der DC-Spike eines Null-ZF-Empfängers auf keinem der beiden.
ais-setup = EINRICHTUNG
ais-channels-timeouts-trail-length-how = Kanäle, Zeitüberschreitungen, Spurlänge und wie weit die Vektoren vorausreichen
ais-vessels = SCHIFFE
ais-nothing-heard-yet = noch nichts gehört
ais-virtual-nothing-there = virtuell — dort ist nichts
ais-position-stale = Position veraltet
ais-search-rescue-transmitter-man-overboard = ein Such-und-Rettungs-Sender, eine Mann-über-Bord-Bake oder ein AIS-EPIRB
ais-call-sign = Rufzeichen
ais-station-time = Stationszeit
ais-dgnss = {" "}(DGNSS)
ais-first-heard = zuerst gehört
ais-center = ZENTRIEREN
ais-put-vessel-middle-chart = Dieses Schiff in die Mitte der Karte setzen
ais-last-message-station-nmea-sentences = Die letzte Meldung dieser Station, als die NMEA-Sätze, die ein Empfänger auf die Leitung gäbe. In jeden anderen AIS-Decoder einfügen, um zu prüfen, was sdroxide daraus gemacht hat.
ais-ais-setup = AIS-Einrichtung
ais-channels = Kanäle
ais-drop-chart-after = Von der Karte nehmen nach
ais-without-position-report = ohne Positionsmeldung
ais-five-minutes-default-not-ads = Standardmäßig fünf Minuten, nicht die zehn Sekunden von ADS-B: Ein vor Anker liegendes Schiff meldet alle drei Minuten, und ein kürzeres Fenster würde den größten Teil eines Hafens zwischen zwei völlig guten Meldungen leer lassen. Danach verschwindet das Schiff von der Karte und seine Zeile wird grau — nicht verblasst, denn ein blasses Symbol an einer veralteten Position ist immer noch eine Aussage darüber, wo ein Schiff ist, in derselben Farbe wie die echten.
ais-drop-list-after = Von der Liste nehmen nach
ais-nothing-heard-all = ohne dass überhaupt etwas gehört wurde
ais-message-carrying-ship-s-name = Die Meldung mit dem Namen eines Schiffes kommt alle sechs Minuten, daher wirft ein kurzes Listenfenster Schiffe genau dann weg, bevor sie sagen, wie sie heißen.
ais-trail-length = Spurlänge
ais-history-behind-each-target = Verlauf hinter jedem Ziel
ais-minutes-rather-than-points-because = In Minuten statt in Punkten, weil die AIS-Meldeintervalle zwei Größenordnungen umfassen: Eine feste Anzahl wäre achtzig Sekunden einer Fähre und zwei Stunden eines vor Anker liegenden Tankers, gleich gezeichnet. Null schaltet die Spuren aus.
ais-speed-vector = Geschwindigkeitsvektor
ais-how-far-ahead-vector-reaches = wie weit der Vektor vorausreicht
ais-slot-threshold = Platz-Schwelle
ais-above-channel-s-learned-noise = über dem gelernten Rauschboden des Kanals
ais-track-most = Höchstens verfolgen
ais-vessels-2 = {" "}Schiffe
ais-ais-receive-only-here-always = AIS ist hier nur Empfang und wird es immer bleiben: Es ist ein Dienst für die Sicherheit des Lebens, und falschen Schiffsverkehr darauf zu senden ist nichts, was eine Lizenz abdeckt.
ais-fill-my-grid-digimode-setup = Mein Locator in der Einrichtung der digitalen Betriebsarten eintragen, damit Entfernungen und Peilungen stimmen und die Karte sich um Ihren Standort rahmt statt um das, was am weitesten entfernt ist.
ais-name = NAME
ais-mmsi = MMSI
ais-type = TYP

## qo100 (src/app/qo100.rs)

qo100-receiver-cannot-reach-10489-750 = Dieser Empfänger kann 10489,750 MHz nicht von selbst erreichen — zuerst einen LNB-/Konverter-Versatz einrichten (Einstellungen ▸ Funkgerät)
qo100-run-fast-spectral-tracker-every = Den schnellen Spektral-Tracker laufen lassen: Jede Sekunde im Parkfenster nach den beiden symmetrischen Keulen der Bake suchen und melden, wo der Träger sitzt
qo100-telemetry = TELEMETRIE
qo100-also-run-ao-40-uncoded = Außerdem den unkodierten AO-40-Rahmendecoder laufen lassen — Sync-Wort, CRC und den Telemetrietext — mit einer Schritt-für-Schritt-Anzeige (Träger → Sync → CRC), wie weit jeder Überflug kommt
qo100-auto = AUTO
qo100-auto-correct-let-tracker-adjust = Automatische Korrektur: den Tracker den Konverter-/LNB-Versatz selbst nachstellen lassen — eine langsame geschlossene Schleife, die bei einer sauberen und stetigen Schätzung die Bake zurück auf 10489,750 MHz schiebt und dort hält, während der LNB driftet. Öffnet die Frontstufe jedes Mal neu, wenn sie handelt.
qo100-narrower-search-smaller-slice = Schmaler — einen kleineren Ausschnitt absuchen
qo100-wider-when-beacon-isn-t = Breiter — für den Fall, dass die Bake bei der aktuellen Breite nicht gefunden wird
qo100-clear-mark = Markierung löschen
qo100-drop-hand-placed-beacon-mark = Die von Hand gesetzte Bakenmarkierung entfernen und zur eigenen Anzeige des Dekoders zurückkehren
qo100-lo = unten −
qo100-lo-2 = unten +
qo100-hi = oben −
qo100-hi-2 = oben +
qo100-radio-s-own-tuning-range = Der eigene Abstimmbereich dieses Funkgeräts reicht nicht bis 10489,750 MHz — dafür braucht es einen LNB-/Konverter-Versatz (Einstellungen ▸ Funkgerät ▸ Konverter), bevor dieses Fenster etwas tun kann.
qo100-open-settings-radio = Einstellungen ▸ Funkgerät öffnen
qo100-tune-10489-750-mhz = Auf 10489,750 MHz abstimmen
qo100-tip-if-you-can-see = Tipp: Wenn Sie die Bake im Streifen sehen, aber sie nicht einrastet, doppelklicken — das markiert sie als 10489,750 MHz, und ÜBERNEHMEN kann darauf korrigieren
qo100-tracker = TRACKER
qo100-no-twin-lobe-shape-parking = keine Doppelkeulen-Form im Parkfenster
qo100-shape = FORM
qo100-drift = DRIFT
qo100-need-longer-run-estimates = — (braucht eine längere Reihe von Schätzungen)
qo100-cycles = ZYKLEN
qo100-receiver = EMPFÄNGER
qo100-target = ZIEL
qo100-measured = GEMESSEN
qo100-clicked = {"  "}(angeklickt)
qo100-tracker-2 = {"  "}(Tracker)
qo100-not-locked-yet = noch nicht eingerastet
qo100-converter-offset = KONVERTER-VERSATZ
qo100-decode = DEKODIEREN
qo100-decode-filling-first-frame-buffer = DEKODIEREN — der erste Rahmenpuffer wird gefüllt…
qo100-decode-off-turn-telemetry-chip = DEKODIEREN — aus. Den Chip TELEMETRIE (obere Reihe) einschalten für die Anzeige Träger → Sync → CRC.
qo100-apply-correction = {" "}KORREKTUR ANWENDEN{" "}
qo100-write-converter-lnb-offset-puts = Den Konverter-/LNB-Versatz schreiben, der die Bake dorthin legt, wohin Sie geklickt haben, auf 10489,750 MHz, und den Empfänger neu öffnen — dieselbe kurze Unterbrechung wie Einstellungen ▸ Funkgerät ▸ Übernehmen
qo100-write-converter-lnb-offset-puts-2 = Den Konverter-/LNB-Versatz schreiben, der die Schätzung des Trackers auf 10489,750 MHz legt, und den Empfänger neu öffnen — dieselbe kurze Unterbrechung wie Einstellungen ▸ Funkgerät ▸ Übernehmen
qo100-waiting-second-crc-valid-frame = Warte auf einen zweiten rahmen mit gültiger CRC, bevor angeboten wird, dies zu schreiben — eine einzelne Einrastung könnte ein Zufallstreffer sein
qo100-write-corrected-converter-lnb-offset = Den korrigierten Konverter-/LNB-Versatz schreiben und den Empfänger neu öffnen — eine kurze Unterbrechung, dieselbe, die Einstellungen ▸ Funkgerät ▸ Übernehmen macht
qo100-decoder-runs-receiving-station-readout = Der Dekoder läuft auf der empfangenden Station — seine Anzeige wird nicht an entfernte Clients gesendet
qo100-starting = wird gestartet…
qo100-searching-first-window-fills-after = suche — das erste Fenster füllt sich nach etwa 24 s und wiederholt sich dann

## js8 (src/app/panels/js8.rs)

js8-multi = MULTI
js8-decode-every-js8-speed-not = Jede JS8-Geschwindigkeit dekodieren, nicht nur die, mit der Sie senden. Die vier Geschwindigkeiten teilen sich das Teilband und sind vier verschiedene Wellenformen, daher ist ohne dies eine Station auf einer anderen Geschwindigkeit unsichtbar — und ein Austausch zwischen zwei Geschwindigkeiten ist gar nicht möglich. Kostet etwa das Vierfache der Empfangs-CPU.
js8-hb-auto = HB AUTO
js8-turbo-does-not-beacon-local = Turbo sendet keine Bake — es ist die Geschwindigkeit für lokal und UKW
js8-beacon-your-callsign-grid-every = Ihr Rufzeichen und Ihren Locator alle 15 Minuten als Bake senden
js8-until-next-heartbeat = Bis zum nächsten Herzschlag
js8-setup = ⚙ EINRICHTUNG
js8-tx = ● TX
js8-heard = GEHÖRT
js8-nothing-heard-yet = — noch nichts gehört —
js8-dxcc = DXCC
js8-band = BAND
js8-grid = LOCATOR
js8-dupe = DUPLIKAT
js8-reply = ANTWORT
js8-no-messages = — keine Nachrichten —
js8-cq = {" "}CQ{" "}
js8-hb = {" "}HB{" "}
js8-clear = {" "}AN LEEREN{" "}
js8-forget-selected-station-composer-goes = Die gewählte Station vergessen — der Verfasser geht zurück zu @ALLCALL
js8-already-addressing-allcall = Adressiert bereits @ALLCALL
js8-stop = {" "}STOPP{" "}
js8-send = {" "}SENDEN{" "}
js8-message = Nachricht…
## logbook (src/app/logbook.rs)

palette-hint = Gehe zu… einem Modus, einem Band, einem Fenster, einer Einstellung, einem Speicher oder einer Frequenz (14.074)
palette-nothing = nichts mit diesem Namen
palette-group-window = Fenster
palette-group-settings = Einstellungen
palette-group-memory = Speicher
palette-group-tune = Abstimmen
palette-tune-to = Auf { $mhz } MHz abstimmen
palette-win-bands = Bandbedingungen
palette-win-awards = Auszeichnungen
palette-win-windows = Fensterliste
logbook-logbook = LOGBUCH
logbook-new-entry = + NEUER EINTRAG
logbook-adif = ADIF
logbook-import = IMPORTIEREN
logbook-import-qsos-adif-adi-adif = QSOs aus einer ADIF-Datei (.adi oder .adif) importieren
logbook-new-qso = NEUES QSO
logbook-edit-qso = QSO BEARBEITEN
logbook-call = Rufzeichen
logbook-lookup = ABFRAGEN
logbook-look-up-name-qth-grid = Name / QTH / Locator abfragen
logbook-grid = Locator
logbook-freq-mhz = Frequenz MHz
logbook-mode = Betriebsart
logbook-rst-sent = RST gesendet
logbook-rst-rcvd = RST empfangen
logbook-name = Name
logbook-state = Bundesland
logbook-country = Land
logbook-date-utc = Datum UTC
logbook-time = Zeit
logbook-pwr-w = Leist. W
logbook-contest = Contest
logbook-s-sent = Nr. gesendet
logbook-s-rcvd = Nr. empfangen
logbook-comment = Kommentar
logbook-save = {" "}SPEICHERN{" "}
logbook-cancel = ABBRECHEN
logbook-lotw = LoTW ✓
logbook-eqsl = eQSL ✓
logbook-card = QSL-Karte ✓
logbook-lotw-2 = LoTW ↑
logbook-eqsl-2 = eQSL ↑
logbook-qrz = QRZ ↑
logbook-hamqth = HamQTH ↑
logbook-club-log = Club Log ↑
logbook-wavelog = Wavelog ↑
logbook-delete-entry = Diesen Eintrag löschen
logbook-edit = BEARBEITEN
logbook-upload-qso-configured-logs = Dieses QSO in die eingerichteten Logbücher hochladen
logbook-no-qsos-yet-run-ft8 = noch keine QSOs — FT8/FT4 betreiben oder einen manuellen Eintrag hinzufügen

## scanner (src/app/scanner.rs)

scanner-scanner = Scanner
scanner-work-through-stored-memory-channels = Die gespeicherten Speicherkanäle durchgehen
scanner-work-through-slice-band-channel = Einen Ausschnitt eines Bandes auf einem Kanalraster durchgehen
scanner-stop = STOPP
scanner-start = START
scanner-from = Von
scanner-step = Schritt
scanner-mode = Betriebsart
scanner-range-empty-high-edge-has = Dieser Bereich ist leer — die obere Grenze muss mindestens einen Kanal über der unteren liegen.
scanner-skip-while-holding-pass-over = ÜBERSPRINGEN, solange er diesen Kanal ab jetzt überspringen soll
scanner-skipping = Wird übersprungen
scanner-clear = LEEREN
scanner-stop-passing-over-any-them = Keinen davon mehr überspringen
scanner-stop-passing-over-channel = Diesen Kanal nicht mehr überspringen
scanner-no-memory-channels-stored-yet = Noch keine Speicherkanäle gespeichert — im Fenster MEM einige speichern.
scanner-skip-channel-pass-over = Einen Kanal ÜBERSPRINGEN, um ihn auszulassen
scanner-skip = ÜBERSPRINGEN
scanner-look-all-channels-fall-inside = Alle Kanäle suchen, die in ein Empfängerfenster fallen, auf derselben Transformation, aus der das Panorama gebaut ist, und nur auf die abstimmen, auf denen etwas liegt. Eine Liste auf einem Band kostet dann eine Abstimmung pro Umlauf, egal wie lang sie ist, statt einer Einschwingzeit je Kanal. Der Suchlauf hört bei jedem Kandidaten trotzdem hinein, bevor er stoppt, daher ändert sich, was ihn stoppt, nicht — aber die Abtastung misst über die FFT statt über das Filter des Empfängers, daher die Schwelle prüfen, falls er bei nichts zu stoppen beginnt.
scanner-radio-hands-over-demodulated-audio = Dieses Funkgerät gibt demoduliertes Audio aus und hat kein eigenes Spektrum, das es durchsuchen könnte, daher besucht sein Speicher-Suchlauf in jedem Fall jeden Kanal.
scanner-fast = SCHNELL
scanner-read-list-off-spectrum-instead = die Liste aus dem Spektrum ablesen, statt jeden Kanal zu besuchen
scanner-folders = Ordner
scanner-scan-every-folder-every-folder = Jeden Ordner absuchen, und jeden Ordner, der ab jetzt angelegt wird. Ordner auswählen, um nur diese abzusuchen.
scanner-unfiled = Ohne Ordner
scanner-nothing-filed-under-folders-you = Unter den gewählten Ordnern ist nichts abgelegt.
scanner-stops = Hält an bei
scanner-use-receiver-s-own-squelch = Die eigene Rauschsperre des Empfängers nutzen, damit der Suchlauf genau dort stoppt, wo sich das Audio öffnen würde
scanner-squelch-off-scan-will-stop = Rauschsperre ist aus — der Suchlauf hält bei der ersten Sache, die er ansieht
scanner-channel-power-signal-has-reach = Kanalleistung, die ein Signal erreichen muss, um den Suchlauf zu stoppen
scanner-listens = Hört hinein für
scanner-how-long-stay-candidate-before = Wie lange bei einem Kandidaten geblieben wird, bevor geurteilt wird. Unter etwa einer Zehntelsekunde hat sich die Pegelanzeige nicht eingependelt, und schwache Signale werden übersehen
scanner-resumes = Setzt fort
scanner-carry-once-signal-drops = Weitergehen, sobald das Signal abfällt
scanner-carry-after-fixed-time-regardless = Nach einer festen Zeit weitergehen, unabhängig davon
scanner-stay-until-you-press-next = Bleiben, bis Sie NÄCHSTER drücken
scanner-how-long-stay = Wie lange bleiben
scanner-how-long-wait-after-signal = Wie lange nach dem Abfallen des Signals gewartet wird
scanner-move-don-t-stop-channel = Weitergehen und auf diesem Kanal nicht wieder anhalten
scanner-next = NÄCHSTER
scanner-move-now = Jetzt weitergehen

## adsb (src/app/panels/adsb.rs)

adsb-starting-ads-b-decoder = ADS-B-Decoder wird gestartet…
adsb-ads-b = ADS-B
adsb-t-1090-mhz-mode-s = 1090 MHz Mode S
adsb-tune-receiver-1090-000-mhz = Den Empfänger auf 1090,000 MHz abstimmen, den weltweiten ADS-B-Downlink. Es gibt nur einen Kanal.
adsb-setup = EINRICHTUNG
adsb-timeouts-trail-length-how-far = Zeitüberschreitungen, Spurlänge und wie weit die Geschwindigkeitsvektoren reichen
adsb-aircraft = FLUGZEUGE
adsb-nothing-heard-yet = noch nichts gehört
adsb-ground = am Boden
adsb-position-stale = Position veraltet
adsb-gnss-altitude = GNSS-Höhe
adsb-ground-speed = Bodengeschwindigkeit
adsb-vertical-rate = Vertikalgeschwindigkeit
adsb-first-heard = zuerst gehört
adsb-center = ZENTRIEREN
adsb-put-aircraft-middle-map = Dieses Flugzeug in die Mitte der Karte setzen
adsb-last-frame-accepted-aircraft = Der letzte Rahmen, der von diesem Flugzeug angenommen wurde
adsb-ads-b-setup = ADS-B-Einrichtung
adsb-drop-map-after = Von der Karte nehmen nach
adsb-without-position-report = ohne Positionsmeldung
adsb-past-aircraft-comes-off-map = Danach verschwindet das Flugzeug von der Karte und seine Zeile wird grau. Es wird nicht verblasst: Ein blasses Quadrat an einer veralteten Position ist immer noch eine Aussage darüber, wo ein Flugzeug ist, in derselben Farbe wie die echten.
adsb-drop-list-after = Von der Liste nehmen nach
adsb-nothing-heard-all = ohne dass überhaupt etwas gehört wurde
adsb-trail-length = Spurlänge
adsb-points = {" "}Punkte
adsb-history-dots-behind-each-target = Verlaufspunkte hinter jedem Ziel
adsb-speed-vector = Geschwindigkeitsvektor
adsb-how-far-ahead-leader-line = wie weit die Führungslinie reicht
adsb-one-minute-usual-radar-convention = Eine Minute ist die übliche Radarkonvention: Die Linie ist genau so lang wie die Strecke, die das Flugzeug in dieser Zeit zurücklegt, daher sind zwei gleich lange Führungen bei jedem Zoom zwei gleiche Geschwindigkeiten. Null schaltet sie aus.
adsb-track-most = Höchstens verfolgen
adsb-aircraft-2 = {" "}Flugzeuge
adsb-aircraft-ground-are-placed-against = Flugzeuge am Boden werden gegen die eigene Position der Station eingeordnet — ein Oberflächen-Squitter hat keine eindeutige Dekodierung aus sich selbst —, daher Mein Locator in der Einrichtung der digitalen Betriebsarten eintragen, wenn der Flughafen in der Nähe nichts zeigt.
adsb-call = RUFZEICHEN
adsb-icao = ICAO
adsb-v-s = V/S

## drm (src/app/drm.rs)

drm-waiting-receiver = warte auf den Empfänger…
drm-not-drm-set-mode-drm = Nicht in DRM. Die Betriebsart auf DRM setzen, bei einer digitalen Kurzwellen-Sendung — die Skala liegt auf der Kanalmitte, nicht auf einem Seitenband.
drm-no-drm-signal-locked-decoder = Kein DRM-Signal eingerastet. Der Decoder braucht ein paar Sekunden auf einem sauberen Träger: DRM verschachtelt über 400 ms oder 2 s, bevor etwas davon gelesen werden kann.
drm-station-s-audio-codec-not = Der Audio-Codec dieser Station wird hier nicht dekodiert. xHE-AAC braucht libfdk-aac auf dem System — es lässt sich nicht einbauen, weil seine Lizenz und die dieses Programms nicht zusammenpassen. Installieren (Debian/Ubuntu: libfdk-aac2, Arch: libfdk-aac, macOS: brew install fdk-aac, Windows: libfdk-aac-2.dll neben sdroxide.exe) und neu starten. Alles andere auf diesem Bildschirm wird normal dekodiert.
drm-quality = QUALITÄT
drm-last-minute = letzte Minute
drm-waiting-locked-signal = warte auf ein eingerastetes Signal
drm-audio-frames-decoding-but-station = Audio-Rahmen werden dekodiert — aber der Codec dieser Station kann hier nicht dekodiert werden
drm-audio-frames-decoding = Audio-Rahmen werden dekodiert
drm-samples-reaching-decoder = Abtastwerte erreichen den Decoder
drm-time = ZEIT
drm-symbol-timing-recovered = Symboltakt wiederhergestellt
drm-frame = RAHMEN
drm-transmission-frames-found = Übertragungsrahmen gefunden
drm-fast-access-channel-what-transmission = Fast-Access-Kanal — was die Übertragung ist
drm-service-description-channel-what-services = Service-Beschreibungskanal — was die Dienste sind
drm-audio = AUDIO
drm-constellation = KONSTELLATION
drm-waiting-decoded-symbols = warte auf dekodierte Symbole
drm-sampled-across-frame = über den Rahmen abgetastet
drm-whole-frame = ganzer Rahmen
drm-mode = MODUS
drm-robustness-mode-channel-width-ground = Robustheitsmodus und Kanalbreite. A ist für einen Bodenwellenweg und trägt am meisten; D ist für einen stark gestreuten Raumwellenweg und trägt am wenigsten.
drm-interleave = VERSCHACHTELUNG
drm-how-far-transmission-spreads-each = Wie weit die Übertragung jeden Rahmen zeitlich verteilt. Lang übersteht tiefere Schwunglöcher und braucht entsprechend länger zum Einrasten.
drm-protection = SCHUTZ
drm-offset = VERSATZ
drm-residual-sample-clock-error-against = Restfehler des Abtasttakts gegenüber dem Sender. Groß und stetig heißt, dass die Referenz des Empfängers daneben liegt, nicht die Sendung.
drm-doppler = DOPPLER
drm-delay = VERZÖGERUNG
drm-doppler-delay-spread-path-how = Doppler und Verzögerungsspreizung des Weges — wie schnell er sich bewegt und wie weit seine Echos auseinander ankommen.
drm-service = DIENST
drm-decode-service-multiplex = Diesen Dienst des Multiplex dekodieren

## atchat (src/app/panels/atchat.rs)

atchat-starting-atchat-net-station = Die AtCHAT-NET-Station wird gestartet…
atchat-atchat = ATCHAT
atchat-net-multi-station-keyboard-file = NET — Tastatur und Datei für mehrere Stationen
atchat-net = {" "}IM NET{" "}
atchat-net-click-leave-state-kept = Im Net — klicken zum Verlassen (Zustand bleibt erhalten; erneut klicken, um wieder beizutreten)
atchat-rejoin = {" "}WIEDER BEITRETEN{" "}
atchat-off-net-click-rejoin = Nicht im Net — klicken, um wieder beizutreten
atchat-off-net = nicht im Net
atchat-carrier = ● TRÄGER
atchat-another-station-transmitting-channel-busy = Eine andere Station sendet — der Kanal ist belegt.
atchat-tx = ● TX
atchat-setup = ⚙ EINRICHTUNG
atchat-station-callsign-virtual-channel-address = Rufzeichen der Station und die Adresse des virtuellen Kanals
atchat-virtual-channel = VIRTUELLER KANAL
atchat-work-net-over-tcp-endpoint = Das Net über einen TCP-Endpunkt statt über das Funkgerät betreiben — ein mit channel_server kompatibler Loopback für die Entwicklung ohne Funkgerät. Aus setzt die Station auf Sendung.
atchat-roster = TEILNEHMER
atchat-nobody-heard-yet = noch niemand gehört
atchat-chat = {" "}CHAT{" "}
atchat-common-channel-lines-here-go = Der gemeinsame Kanal — Zeilen hier gehen an alle (ALLE)
atchat-log = {" "}PROTOKOLL{" "}
atchat-station-s-own-air-activity = Die eigene Sendeaktivität der Station — Master-Wahl, Alterung der Teilnehmerliste, ARQ-Wiederholungen
atchat-no-activity-logged-yet = Noch keine Aktivität protokolliert.
atchat-no-messages-yet = Noch keine Nachrichten.
atchat-message-everyone-all = Nachricht an alle (ALLE)
atchat-send = {" "}SENDEN{" "}
atchat-transfers = ÜBERTRAGUNGEN
atchat-all = → ALLE
atchat-sent-file-follows-active-chat = Eine gesendete Datei folgt dem aktiven Chat-Reiter
atchat-send-file = {" "}DATEI SENDEN{" "}
atchat-send-file-image-over-air = Eine Datei oder ein Bild über Funk senden, Block-CRC-ARQ
atchat-nothing-flight = nichts unterwegs
atchat-images = BILDER
atchat-no-images-received = Keine Bilder empfangen.

## rds (src/app/rds.rs)

rds-station = STATION
rds-diagnostics = DIAGNOSE
rds-which-programme-type-table-read = Gegen welche Programmtyp-Tabelle diese Station gelesen wird. RDS wird überall außerhalb Nordamerikas genutzt; RBDS ergänzt das Rufzeichen, das der Identitätscode einer US-Station buchstabiert. Auto folgt dem erweiterten Ländercode, den die Station sendet, und bis einer ankommt, buchstabiert es kein Rufzeichen — die Identitätsbereiche, die es lesen würde, werden mit den RDS-Codes von neun Ländern geteilt, daher RBDS wählen, um eine Station zu benennen, die keinen Ländercode sendet.
rds-waiting-receiver = warte auf den Empfänger…
rds-no-rds-station-not-every = Kein RDS auf dieser Station. Nicht jeder Sender trägt es, und ein schwacher kann es tragen, ohne dass es ankommt.
rds-rds-only-carried-wfm-broadcast = RDS wird nur auf WFM-Rundfunksendern übertragen.
rds-holding = HÄLT
rds-block-sync-has-dropped-last = Die Blocksynchronisation ist abgefallen — dies ist die letzte gelesene Station.
rds-identity = IDENTITÄT
rds-programme = PROGRAMM
rds-described = BESCHRIEBEN ALS
rds-content = INHALT
rds-music = Musik
rds-speech = Sprache
rds-traffic = VERKEHR
rds-announcement-air-now = Durchsage läuft jetzt
rds-carried-station = von dieser Station übertragen
rds-not-carried = nicht übertragen
rds-radio-text = RADIOTEXT
rds-station-clock = STATIONSUHR
rds-country-code = LÄNDERCODE
rds-also = AUCH AUF
rds-also-tagged = AUCH MARKIERT
rds-sync = SYNC
rds-no-sync = KEIN SYNC
rds-blocks-failed-their-check-plus = Blöcke, die ihre Prüfung nicht bestanden, plus die, die der Fehlerkorrektor reparieren musste. Korrekturen zählen absichtlich als Fehler: Eine steigende Zahl ist das erste Zeichen dafür, dass eine Station bald nicht mehr dekodiert werden kann.
rds-front-end-overloading = FRONTSTUFE ÜBERLASTET
rds-converter-clipping-which-puts-distortion = Der Konverter übersteuert, was Verzerrung über das ganze Multiplex legt — auch auf den Datenunterträger bei 57 kHz. Die HF-Verstärkung herunterdrehen oder ein Dämpfungsglied einschleifen: RDS verschlechtert sich etwa ein Dezibel, bevor etwas hörbar wird, daher kann eine Station, die perfekt klingt, immer noch zu stark sein, als dass ihre Daten überleben.
rds-group-types = GRUPPENTYPEN
rds-recent-groups-newest-last = LETZTE GRUPPEN — neueste zuletzt
rds-nothing-decoded-yet = noch nichts dekodiert

## winlink (src/app/winlink.rs)

winlink-mail = POST
winlink-abort = ABBRECHEN
winlink-stop-session-progress = Die laufende Sitzung stoppen
winlink-connect = VERBINDEN
winlink-compose = VERFASSEN
winlink-connecting = verbinde…
winlink-not-connected-yet = noch nicht verbunden
winlink-loading = lädt…
winlink-date = DATUM
winlink-from = VON
winlink-subject = BETREFF
winlink-no-subject = (kein Betreff)
winlink-delete-message = Diese Nachricht löschen
winlink-arch = ARCHIV
winlink-move-archive = Ins Archiv verschieben
winlink-select-message = eine Nachricht wählen
winlink-reply = ANTWORTEN
winlink-delete = LÖSCHEN
winlink-to = An
winlink-cc = Kopie
winlink-subject-2 = Betreff
winlink-callsign-smtp-someone-example-org = Ein Rufzeichen oder SMTP:jemand@example.org für Internet-Post. Mehrere durch Kommas trennen.
winlink-file-outbox = IM AUSGANG ABLEGEN
winlink-discard = VERWERFEN
winlink-filed-messages-go-out-next = abgelegte Nachrichten gehen bei der nächsten Sitzung hinaus
winlink-mail-log = POST-PROTOKOLL
winlink-session-running = Sitzung läuft…
winlink-no-session-yet = noch keine Sitzung

## wspr (src/app/panels/wspr.rs)

wspr-receptions = EMPFÄNGE
wspr-save-wspr-reception-list-csv = Die WSPR-Empfangsliste als CSV speichern
wspr-wspr = WSPR
wspr-weak-signal-propagation-beacon = Schwachsignal-Ausbreitungsbake
wspr-time-left-two-minute-slot = Verbleibende Zeit in diesem Zwei-Minuten-Zeitschlitz. Am Ende geht die Aufnahme an den Decoder, und die nächste Entscheidung zum Senden fällt.
wspr-tx = ● TX
wspr-decoding = dekodiere…
wspr-band = Band
wspr-last-slot = Letzter Zeitschlitz
wspr-nothing-heard = nichts gehört
wspr-receive-only = nur Empfang
wspr-next-slot = Nächster Zeitschlitz
wspr-waiting-radio-take-change = warte, bis das Funkgerät die Änderung übernimmt…
wspr-hop-next = Als Nächstes wechseln
wspr-transmit = SENDEN
wspr-receive-only-2 = Nur Empfang.
wspr-power = LEISTUNG
wspr-what-you-actually-radiate = was Sie tatsächlich abstrahlen
wspr-goes-out-message-everyone-who = Das geht in der Nachricht hinaus, und jeder, der Sie hört, beurteilt den Weg danach — ein zu optimistischer Wert hier macht deren Messungen ebenso falsch wie Ihre.
wspr-roam = WANDERN
wspr-pick-different-offset-inside-200 = Für jede Aussendung einen anderen Versatz innerhalb des 200-Hz-Fensters wählen. Das ist die Konvention: Zweihundert Hertz, die alle teilen, funktionieren nur, wenn niemand sich in die Mitte setzt. Aus hält den Cursor fest.
wspr-band-hop = BANDWECHSEL
wspr-move-dial-band-band-between = Die Skala zwischen den Zeitschlitzen von Band zu Band bewegen, damit ein Empfänger das ganze Spektrum statt eines Ausschnitts davon abtastet. Wenn Sie den VFO selbst drehen, pausiert es — zum Fortsetzen zweimal drücken.
wspr-upload = HOCHLADEN
wspr-send-what-station-decodes-wsprnet = Senden, was diese Station dekodiert, an wsprnet.org. Zu melden, was Sie hören, macht einen WSPR-Empfänger zum Teil des Netzes; es sendet nichts auf die Frequenz.
wspr-who-heard-me = WER HAT MICH GEHÖRT
wspr-ask-wsprnet-org-every-few = Alle paar Minuten bei wsprnet.org nachfragen, welche Stationen diese dekodiert haben. WSPR kennt keinerlei Empfangsbestätigung, daher ist dies der einzige Weg, auf dem eine Bake etwas über ihre Reichweite erfährt.
wspr-bands = BÄNDER
wspr-none-selected-hopping-has-nowhere = keines gewählt — der Wechsel hat kein Ziel
wspr-set-your-callsign-grid-general = Rufzeichen und Locator im Reiter Allgemein der Einstellungen setzen, bevor Sie senden.

## packet (src/app/panels/packet.rs)

packet-monitor = MONITOR
packet-busy = BESETZT
packet-frames-arrived-but-failed-their = Rahmen, die ankamen, aber ihre Prüfsequenz nicht bestanden — eine Kollision, ein Schwund oder ein Signal, das zu schwach zum Lesen ist.
packet-setup = ⚙ EINRICHTUNG
packet-station-callsign-speed-tx-delay = Rufzeichen der Station, Geschwindigkeit, TX-Verzögerung, der Digipeater-Pfad, die Bake und der KISS-Server
packet-starting-packet-modem = Paket-Modem wird gestartet…
packet-nothing-heard-yet = noch nichts gehört
packet-terminal = TERMINAL
packet-connected = {" "}VERBUNDEN{" "}
packet-calling = {" "}RUFT{" "}
packet-idle = {" "}LEERLAUF{" "}
packet-extended-sequence-numbers-window-up = Erweiterte Folgenummern — ein Fenster von bis zu 127 Rahmen.
packet-frames-sent-not-yet-acknowledged = Rahmen, die gesendet, aber von der Gegenseite noch nicht bestätigt sind.
packet-retries-against-link-s-limit = Wiederholungen gegen die Grenze dieser Verbindung. Ein Zähler, der steigt, während die unbestätigten Rahmen stehen bleiben, sieht von dieser Seite so aus, wenn ein Weg schwindet.
packet-mail-window-using-link = Das Fenster POST benutzt die Verbindung.
packet-one-radio-one-channel-one = Ein Funkgerät, ein Kanal, eine Verbindung. Eine Winlink-Sitzung und dieses Terminal sind zwei Arten, sie zu nutzen, und wer zuerst fragt, bekommt sie.
packet-digipeaters-nearest-first-separated-commas = Digipeater, der nächste zuerst, durch Kommas getrennt — OE3XLR-1,OE3XMS-1. Leer lassen für eine Station, die Sie direkt hören, oder um den Standardpfad aus den Paketeinstellungen zu nutzen.
packet-disconnect = {" "}TRENNEN{" "}
packet-hang-up-properly-wait-far = Ordentlich auflegen und auf die Zustimmung der Gegenseite warten. Die Betriebsart stattdessen zu wechseln beendet die Sitzung, ohne es jemandem zu sagen.
packet-connect = {" "}VERBINDEN{" "}
packet-mail-window-has-link-finish = Das Fenster POST hat die Verbindung. Zuerst diese Sitzung beenden oder stoppen.
packet-station-has-no-callsign-yet = Diese Station hat noch kein Rufzeichen — eines unter EINRICHTUNG setzen, oberhalb des Monitorbereichs. Bis es gesetzt ist, wird nichts gesendet.
packet-call-station-connected-mode-node = Diese Station im verbundenen Modus rufen — einen Knoten, eine Mailbox oder einen anderen Bediener.
packet-needs-callsign-call = Braucht ein Rufzeichen zum Anrufen.
packet-clear = {" "}LEEREN{" "}
packet-empty-transcript-link-untouched-connected = Das Protokoll leeren. Die Verbindung bleibt unberührt — eine verbundene Station bleibt verbunden.
packet-not-connected-type-callsign-above = Nicht verbunden. Oben ein Rufzeichen eintippen und VERBINDEN drücken, um einen Knoten oder eine Mailbox anzurufen.
packet-type-here = hier tippen
packet-not-connected = nicht verbunden
packet-send = {" "}SENDEN{" "}
packet-send-line-node-bbs-reads = Die Zeile senden. Ein Knoten oder eine Mailbox liest eine Zeile nach der anderen.

## fsq (src/app/panels/fsq.rs)

fsq-speed = Geschwindigkeit
fsq-call = Rufzeichen
fsq-callsign-directed-fsqcall-messages-defaults = Rufzeichen für gerichtete (FSQCALL-)Nachrichten; Standard ist Ihr Rufzeichen
fsq-tx = ● TX
fsq-contacts = KONTAKTE
fsq-heard = GEHÖRT
fsq-none = — keine —
fsq-allcall = ALLCALL
fsq-image = BILD
fsq-clear = LEEREN
fsq-forget-pictures-received-so-far = Die bisher empfangenen Bilder vergessen
fsq-send-image = Bild senden…
fsq-right-click-remove = Rechtsklick zum Entfernen
fsq-remove-picture = Dieses Bild entfernen
fsq-listening = — höre zu —
fsq-message = Nachricht…
fsq-send = {" "}SENDEN{" "}
fsq-call-cq = {" "}CQ RUFEN{" "}
fsq-heard-2 = {" "}? gehört{" "}
fsq-clear-2 = {" "}LEEREN{" "}
fsq-fsq-contacts = FSQ-Kontakte
fsq-add = Hinzufügen

## publicsdr (src/app/publicsdr.rs)

publicsdr-not-stated = nicht angegeben
publicsdr-add-as-radio = ALS RADIO HINZUFÜGEN
publicsdr-add-as-radio-tip = Als eigenes Funkgerät hinzufügen, Name aus Verzeichnis
publicsdr-public-sdrs = ÖFFENTLICHE SDRS
publicsdr-available = VERFÜGBAR
publicsdr-hide-receivers-are-full-ones = Empfänger ausblenden, die voll sind, und die, deren Betreiber keine Kanäle für Apps außer einem Browser geöffnet hat
publicsdr-band = IM BAND
publicsdr-only-receivers-cover-current-dial = Nur Empfänger, die die aktuelle Skalenfrequenz abdecken
publicsdr-low-bw = NIEDR. BB
publicsdr-map-cities = STÄDTE
publicsdr-map-cities-tip = Städte auf dieser Karte ein oder aus.
publicsdr-view-list = LISTE
publicsdr-view-map = KARTE
publicsdr-view-map-tip = Empfänger auf Weltkarte; Klick auf Punkt wählt alle darunter
publicsdr-map-pick = Klick auf einen Punkt wählt die Empfänger darunter.
publicsdr-map-ghost = { $name }: gerade nicht im Katalog — zuletzt hier gesehen am { $seen }
publicsdr-map-names = NAMEN
publicsdr-map-names-tip = Namen aller Empfänger; aus: nur gewählte
publicsdr-check-online = ONLINE
publicsdr-check-silent = STUMM
publicsdr-check-waiting = PRÜFE…
publicsdr-check-no-answer = Keine Antwort innerhalb von 20 Sekunden.
publicsdr-take-spyserver-low-bandwidth-shape = Einen SpyServer in seiner Form mit niedriger Bandbreite nehmen: ein schmales I/Q-Fenster, das der Skala folgt, plus die eigene Bandansicht des Servers, statt Megabit an Breitband-I/Q. Ohne Wirkung auf einen KiwiSDR, der nur die eine Form hat.
publicsdr-refresh = ⟳ AKTUALISIEREN
publicsdr-fetch-both-directories-again = Beide Verzeichnisse erneut abrufen
publicsdr-name-place-antenna-band = Name, Ort, Antenne, Band
publicsdr-clear-search = Die Suche leeren
publicsdr-fetching-receiver-lists = Die Empfängerlisten werden abgerufen…
publicsdr-machine-radio-attached-does-not = Der Rechner, an dem dieses Funkgerät angeschlossen ist, beantwortet keine Gerätefragen, daher kann er die Listen ebenfalls nicht abrufen.
publicsdr-no-match = kein Treffer
publicsdr-nothing-matches-try-turning-filter = nichts passt — einen Filterchip wieder einschalten

## hd (src/app/hd.rs)

hd-hd-radio = HD Radio
hd-waiting-receiver = warte auf den Empfänger…
hd-not-hd-radio-set-mode = Nicht in HD Radio. Die Betriebsart auf HD Radio setzen, bei einer UKW-Sendung mit den digitalen Seitenbändern — die Skala liegt auf der Mitte des analogen Trägers, nicht daneben.
hd-no-hd-radio-lock-digital = Kein HD-Radio-Einrasten. Die digitalen Seitenbänder werden etwa 20 dB unter dem analogen Träger gesendet, daher kann eine hörbare Station immer noch zu verrauscht sein, um sie zu dekodieren: Der Decoder braucht bei einer sauberen ein paar Sekunden.
hd-sync = SYNC
hd-ofdm-frame-timing-recovered = OFDM-Rahmentakt wiederhergestellt
hd-audio = AUDIO
hd-audio-frames-decoding = Audio-Rahmen werden dekodiert
hd-quality = QUALITÄT
hd-lower = UNTEN
hd-upper = OBEN
hd-last-minute = letzte Minute
hd-waiting-locked-signal = warte auf ein eingerastetes Signal
hd-mer-l = MER U
hd-modulation-error-ratio-lower-digital = Modulationsfehlerverhältnis des unteren digitalen Seitenbandes.
hd-mer-u = MER O
hd-modulation-error-ratio-upper-digital = Modulationsfehlerverhältnis des oberen digitalen Seitenbandes.
hd-cber = CBER
hd-channel-bit-error-ratio-after = Kanal-Bitfehlerverhältnis nach dem inneren Code. Unter etwa 1e-3 kann der äußere Code es korrigieren; darüber beginnt das Audio zu zerfallen.
hd-offset = VERSATZ
hd-residual-carrier-frequency-offset-large = Restlicher Trägerfrequenzversatz. Groß und stetig heißt, dass die Referenz des Empfängers daneben liegt, nicht die Sendung.
hd-psmi = PSMI
hd-primary-service-mode-indicator-which = Primary Service Mode Indicator: welchen der FM-Hybrid-Dienstmodi die Station sendet.
hd-programme = PROGRAMM
hd-restricted-access = {" "}— eingeschränkter Zugang

## bands (src/app/bands.rs)

bands-bands = BÄNDER
bands-daylight = ☀ TAG
bands-night = ☾ NACHT
bands-no-forecast-yet-fetching-hamqsl = · noch keine Vorhersage — wird von HAMQSL.com abgerufen
bands-forecast-download-off = · Vorhersage-Download ist aus (Einstellungen → Oberfläche)
bands-band = BAND
bands-condx = AUSBR.
bands-paths = WEGE
bands-reach = REICHW.
bands-best = BEST
bands-condx-global-forecast-rest-what = AUSBR. ist eine weltweite Vorhersage; der Rest ist, was tatsächlich gehört wurde. Eine leere Zeile heißt, dass nichts dekodiert wurde — was heißen kann, dass das Band zu war, oder nur, dass niemand darauf war.
bands-ibp-beacons = IBP-BAKEN
bands-meteor-showers = METEORSCHAUER
bands-peak = SPITZE

## spots (src/app/spots.rs)

spots-pota = POTA
spots-sota = SOTA
spots-freedv = FREEDV
spots-spots = SPOTS
spots-longwave-shortwave-broadcast-stations-air = Lang- und Kurzwellen-Rundfunksender, die jetzt auf Sendung sind
spots-view = IM BLICK
spots-only-spots-inside-panadapter-span = Nur Spots innerhalb des Frequenzbereichs des Panoramas
spots-setup = ⚙ EINRICHTUNG
spots-feeds-lookup-upload-settings = Feeds, Abfrage- und Upload-Einstellungen
spots-call-station-site-frequency = Rufzeichen, Station, Standort, Frequenz
spots-clear-search = Die Suche leeren
spots-no-match = kein Treffer
spots-no-spots-enable-feed-setup = keine Spots — einen Feed unter ⚙ EINRICHTUNG aktivieren
spots-nothing-matches-search = nichts passt zur Suche

## ism (src/app/ism.rs)

ism-ism-devices = ISM-GERÄTE
ism-decoding = DEKODIERT
ism-needs-wideband-iq-source-cat = Braucht eine Breitband-IQ-Quelle; ein CAT-Gerät sendet nur Audio
ism-how-far-burst-must-stand = Wie weit ein Burst über dem eigenen Rauschboden des Kanals stehen muss, bevor er dekodiert wird
ism-give-each-band-width-normally = Jedem Band die Breite geben, die es normalerweise braucht — ein Viertel Megahertz für die OOK-Bänder bei 315, 345 und 433 MHz, eine volle für 868 und 915.
ism-several-hundred-more-device-decoders = Mehrere hundert weitere Gerätedecoder aus dem Projekt rtl_433, eingebaut. Liest die OOK-Geräte bei 433 MHz, die die eigenen Decoder von sdroxide gar nicht erreichen. Wo beide ein Gerät lesen können, kennt rtl_433 weit mehr Varianten und übernimmt.
ism-t-1-your-own = 1 eigener
ism-device-decoders-registered-how-many = Registrierte Gerätedecoder, wie viele aus Ihrer rtl433_flex.conf stammen, und wie viele Übertragungen seit dem Start dekodiert wurden.
ism-reload-decoders = DECODER NEU LADEN
ism-re-read-rtl433-flex-conf = rtl433_flex.conf nach dem Bearbeiten erneut einlesen. Die bereits gehörten Geräte bleiben in der Liste.
ism-rtl-433-knows-more-variants = rtl_433 kennt mehr Varianten dieser Geräte, als die eigenen Decoder von sdroxide, daher liest es sie, während hier gehört wird. Ein anderes Band wählen, und die eigenen Decoder von sdroxide übernehmen wieder.
ism-decoder-your-rtl433-flex-conf = Ein Decoder in Ihrer rtl433_flex.conf wurde abgelehnt. Die übrigen wurden geladen. Die Zeile korrigieren, dann DECODER NEU LADEN drücken.
ism-waiting-decoder = warte auf den Decoder…
ism-where-decoder-actually-listening-follows = Wo der Decoder tatsächlich hört. Das folgt der Hardware-Mitte des Empfängers, nicht dem VFO — bei einer breiten Frontstufe können beide weit auseinanderliegen, und das Fenster entscheidet, welche Kanäle erreichbar sind.
ism-transmissions-gate-opened-how-many = Übertragungen, bei denen das Gate öffnete, und wie viele einen gültigen Rahmen ergaben. Viele Bursts und keine Dekodierungen heißt, dass das Band mit Geräten belegt ist, die sdroxide noch nicht lesen kann. Gar keine heißt, dass nichts die Schwelle erreicht — sie senken oder prüfen, ob die Skala auf dem Band steht.
ism-nothing-heard-yet = noch nichts gehört
ism-click-tune-receiver = Klicken, um den Empfänger darauf abzustimmen

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

navtex-messages = NACHRICHTEN
navtex-sync = SYNC
navtex-character-phase-locked-signal-being = Die Zeichenphase ist eingerastet — ein Signal wird gelesen.
navtex-no-character-phase-yet-no = Noch keine Zeichenphase: kein Signal oder nicht dieses.
navtex-characters-whose-first-copy-was = Zeichen, deren erste Kopie beschädigt war und die aus der Wiederholung fünf Zeitschlitze später genommen wurden, und solche, bei denen beide schlecht waren. Ein NAVTEX-Zeichen wird zweimal gesendet; das ist seine ganze Fehlerkorrektur.
navtex-swap-mark-space-tones-signal = Die Mark- und Space-Töne tauschen, für ein Signal, das auf dem anderen Seitenband empfangen wird. Aus ist das obere Seitenband auf der Kanalfrequenz, was jede veröffentlichte Abstimmanleitung für den Dienst angibt.
navtex-starting-navtex-receiver = NAVTEX-Empfänger wird gestartet…
navtex-nothing-yet-station-transmits-ten = noch nichts — eine Station sendet alle vier Stunden zehn Minuten lang
navtex-cut-short = abgebrochen
navtex-message = NACHRICHT
navtex-time-day-stated-message-utc = Die in der Nachricht angegebene Tageszeit (UTC): nur Anzeige
navtex-receiving = EMPFANG
navtex-monitor = MONITOR
navtex-everything-decoded-message-not = alles dekodiert, Nachricht oder nicht

## rfpaint (src/app/panels/rf_paint.rs)

rfpaint-rf-paint = HF-MALEREI
rfpaint-scan-speed = Abtastgeschwindigkeit
rfpaint-how-fast-text-image-scanned = Wie schnell der Text bzw. das Bild auf den Wasserfall abgetastet wird. Niedriger ist langsamer und lesbarer; 100 % = Grundrate, 25 % (Mitte) ist der Standard.
rfpaint-abort = Abbrechen
rfpaint-text-paint = TEXTMALEREI
rfpaint-type-text-paint = Text zum Malen eintippen…
rfpaint-preview-waterfall = WASSERFALL-VORSCHAU
rfpaint-type-text-preview = Text zur Vorschau eintippen
rfpaint-transmit = {"  "}SENDEN{"  "}
rfpaint-image-paint = BILDMALEREI
rfpaint-no-image-loaded = kein Bild geladen
rfpaint-load-image = Bild laden…
rfpaint-load-image-preview = ein Bild zur Vorschau laden

## hfdl (src/app/hfdl.rs)

hfdl-listen = HÖREN
hfdl-switches-engine-s-hfdl-downconverter = Schaltet den HFDL-Abwärtsmischer und den Decoder der Engine ein oder aus. Das Dekodieren kostet eine 24-kHz-Spur und einen Arbeitsthread, ob das Fenster offen ist oder nicht.
hfdl-channel-assigned-frequency-listen-anything = Der Kanal (die zugewiesene Frequenz), auf dem gehört wird. Alles im Band funktioniert — HFDL-Stationen senden zwischen 2,8 und 22 MHz — gegen eine feste 24-kHz-Spur, die hier zentriert ist.
hfdl-save-hfdl-decode-log-file = Das HFDL-Dekodierprotokoll in eine Datei speichern
hfdl-riverhead-northern-atlantic-north-america = Riverhead (nördlicher Atlantik / Nordamerika) — der Standard und der Kanal, auf dem der Decoder geprüft wurde.
hfdl-hfdl-assigned-frequency = Eine zugewiesene HFDL-Frequenz.
hfdl-one-your-saved-channels-click = Einer Ihrer gespeicherten Kanäle — klicken zum Abstimmen, rechtsklicken zum Entfernen.
hfdl-hfdl-selected-but-decoding-off = HFDL ist gewählt, aber das Dekodieren ist aus — oben HÖREN drücken
hfdl-decodes = DEKODIERUNGEN
hfdl-nothing-decoded-yet-ground-station = Noch nichts dekodiert — das Squitter einer Bodenstation wiederholt sich etwa alle 32 s, sobald eine im Kanal ist.
hfdl-no-decode-matches-filter = Keine Dekodierung passt zum Filter.
hfdl-decoding = DEKODIERT
hfdl-decoding-off = DEKODIEREN AUS
hfdl-dbfs = — dBFS

## rade (src/app/panels/rade.rs)

rade-rade = RADE
rade-freedv-v1-digital-voice = FreeDV V1 digitale Sprache
rade-tx = ● TX
rade-mute-analog = ANALOG STUMM
rade-mute-demodulated-audio-so-only = Das demodulierte Audio stummschalten, sodass nur dekodierte Sprache zu hören ist. Sonst wird das rohe Signal durchgereicht, wann immer das Modem nichts zu spielen hat — dieses Rauschen ist, wie man eine Aussendung findet, bevor sie synchronisiert; daher beim Abstimmen ausgeschaltet lassen.
rade-sync = SYNC
rade-how-far-received-signal-sits = Wie weit das empfangene Signal von dort liegt, wo das Modem es erwartet. Große Werte dekodieren trotzdem — die Akquisitionsschleife folgt ihnen —, aber die Skala so nachzustellen, dass dies nahe null kommt, gibt die beste Reserve.
rade-snr = SNR —
rade-callsign-station-put-end-over =
    Das Rufzeichen, das die Station in ihren End-of-Over-Rahmen gelegt hat — die einzige Stelle in einer RADE-Aussendung, an der eines gesendet wird. Es ist außerdem das, was an FreeDV Reporter gemeldet wird, daher erscheint eine Station, die Sie hören, auf qso.freedv.org als von Ihnen gehört.

    Eine Minute lang gehalten, nachdem das Signal weg ist, und verworfen, wenn Sie eine eigene Aussendung beginnen: Es nennt die Station auf der Frequenz, nicht jede Station, die der Empfänger je gehört hat.
rade-stop-talking = SPRECHEN STOPPEN
rade-talk-ptt = SPRECHEN (PTT)
rade-open-close-rade-over-modem = Eine RADE-Aussendung öffnen oder schließen. Das Modem braucht etwa 120 ms Sprache, bevor der erste Rahmen hinausgeht, und sendet beim Aufhören einen End-of-Over-Rahmen, daher läuft das Senden ein wenig über den Knopf hinaus.
rade-rade-v1-occupies-roughly-1060 = RADE V1 belegt etwa 1060–1880 Hz des Durchlassbereichs — oberes Seitenband, oder unteres auf 160, 80 und 40 m, wo RADE die Sprechübung behält und sdroxide für Sie umschaltet. Das Signal in das schattierte Band auf dem Wasserfall legen; das Modem findet es von dort aus.

## frame (src/app/frame.rs)

frame-acknowledge = Bestätigen
frame-curtain-handle-tip = Ziehen: Panel über Panorama hoch/runter
    Doppelklick: ganz hoch / zurück
frame-dismiss = Schließen
frame-get-sdroxide-com = Auf sdroxide.com herunterladen
frame-reconnecting = Verbinde erneut…
frame-not-connected = Nicht verbunden.
frame-reconnect-now = Jetzt erneut verbinden
frame-transmit-lockout-disabled = ⚠ SENDESPERRE DEAKTIVIERT
frame-engine-was-started-oob-tx = Diese Engine wurde mit --oob-tx gestartet. Die Sperre für Amateurfunkbänder ist aus: Sie tastet den Sender auf jeder Frequenz, die die Hardware unterstützt.
frame-transmitting-outside-your-licence-offence = Außerhalb Ihrer Lizenz zu senden ist in jedem Land, das eine ausstellt, eine Straftat. Nur fortfahren, wenn Sie berechtigt sind, die Frequenzen zu nutzen, auf denen Sie gleich tasten — eine MARS/CAP- oder kommerzielle Lizenz, eine Versuchsgenehmigung oder eine Kunstlast.
frame-i-understand = {"  "}ICH VERSTEHE{"  "}
frame-restart-without-oob-tx-put = Ohne --oob-tx neu starten, um die Sperre wiederherzustellen.

## smeter (src/widgets/smeter.rs)

smeter-alc-how-hard-transmitter-being = ALC — wie stark der Sender ausgesteuert wird, als Prozentsatz dessen, was er höchstens abgeben kann. 100 % ist Vollaussteuerung, nicht volle Leistung in Watt: Was das am Antenneneingang wert ist, hängt vom Funkgerät und von der Aussteuerung ab.
smeter-radio-reports-no-forward-power =
    {""}

    Dieses Funkgerät meldet keine Vorwärtsleistung, daher ist die Angabe neben ALC wieder der ALC-Wert statt Watt.
smeter-no-swr-radio-has-no =
    {""}

    Kein SWR: Dieses Funkgerät hat keine Brücke, um es zu messen, und sdroxide wird keine erfinden — es kann nur zeigen, was das Funkgerät selbst meldet. Ein Gerät mit SWR-Messgerät (über CAT, TCI oder eine LAN-Verbindung) und eine HPSDR-Karte füllen dies beide aus; ein einfacher SDR-Sender kann es nicht.
smeter-no-signal-report-yet-receiver = Noch kein Signalbericht — der Empfänger hat noch keinen Messwert geliefert.
smeter-s9-73-dbm-each-s =
    {""}

    S9 ist −73 dBm, und jede S-Stufe darunter ist 6 dB. Die dBm-Angabe ist nur so ehrlich wie die Kalibrierung der Frontstufe — ein nicht kalibrierter Empfänger meldet dBFS mit einer dBm-Beschriftung.
smeter-ovld-converter-into-rails-so =
    {""}

    OVLD: Der Konverter ist an seinen Grenzen, daher unterschätzt der angezeigte Pegel das Signal. Die Verstärkung der Frontstufe zurücknehmen.
smeter-table-held = , Tabelle gehalten
smeter-ps = PS --

## multi (src/multi.rs)

multi-tx = ● TX
multi-mute-radio-s-audio = Audio dieses Funkgeräts stummschalten
multi-show-radio-here = Dieses Funkgerät hier zeigen
multi-show-on-map = Auf der Karte zeigen
multi-add-radio-computer-settings-radio = Ein Funkgerät auf diesem Computer hinzufügen (zum Hinzufügen an einer Station: Einstellungen → Funkgerät)
multi-add-radio = Ein Funkgerät hinzufügen
multi-take-radio-off-row = Aus Reihe nehmen; läuft weiter
multi-open-pane = Neue Spalte mit verdecktem Funkgerät
multi-pick-radio = Radio ▼
multi-pick-radio-tip = Funkgerät in Reihe holen; Ringe = Verbindung offen
multi-new-radio = Neues Funkgerät…
multi-asked-station-another-radio = Die Station nach einem weiteren Funkgerät gefragt…
multi-client-cannot-open-connection = Dieser Client kann keine Verbindung öffnen.

## net (src/app/net.rs)

net-fail = FEHLER
net-will-retry = {" "}(wird wiederholt)
net-full-resync-downloading = Vollständiger Abgleich: wird heruntergeladen…
net-downloading = Wird heruntergeladen…
net-full-resync-log-already-matches = Vollständiger Abgleich: Das Protokoll stimmt bereits mit Wavelog überein
net-full-resync-review-changes-below = Vollständiger Abgleich: die Änderungen unten prüfen
net-full-resync-discarded-nothing-changed = Vollständiger Abgleich verworfen; nichts geändert
net-adif-import-failed-file-could = ADIF-Import fehlgeschlagen: Die Datei konnte nicht gelesen werden
net-channel-import-failed-file-could = Kanalimport fehlgeschlagen: Die Datei konnte nicht gelesen werden
net-channel-import-nothing-read-chirp = Kanalimport: nichts zu lesen — eine CHIRP-CSV-Datei beginnt mit einer Kopfzeile, die ihre Spalten benennt, und eine davon muss Frequenz sein

## widgets (src/app/panels/widgets.rs)

widgets-free-text = Freitext
widgets-hashed-callsign-not-yet-heard = gehashtes Rufzeichen, noch nicht vollständig gehört
widgets-entity-unknown = Gebiet unbekannt
widgets-band = dieses Band
widgets-new-entity-never-worked-any = Neues Gebiet — nie gearbeitet, auf keinem Band
widgets-new-grid-square = Neues Locator-Quadrat
widgets-not-worked-before = Noch nie gearbeitet
widgets-worked-before-but-not-band = Schon gearbeitet, aber nicht auf diesem Band
widgets-call-queue = In der Anrufwarteschlange
widgets-image = Bild

## pi4 (src/app/panels/pi4.rs)

pi4-receptions = EMPFÄNGE
pi4-save-pi4-reception-list-csv = Die PI4-Empfangsliste als CSV speichern
pi4-next-generation-beacon = Next-Generation-Bake
pi4-time-left-one-minute-beacon = Verbleibende Zeit in diesem Ein-Minuten-Bakenzyklus. Die PI4-Meldung wird gesucht, sobald das Fenster lange genug gefüllt ist, um über sein Ende hinaus zu reichen.
pi4-decoding = dekodiere…
pi4-band = Band
pi4-last-cycle = Letzter Zyklus
pi4-nothing-heard = nichts gehört
pi4-receive-only-tune-so-beacon = Nur Empfang. So abstimmen, dass die CW-Kennung der Bake bei 800 Hz Audio liegt — die eigene Konvention des Netzes —, und die vier PI4-Töne dort liegen, wo dieser Decoder sie standardmäßig sucht.

## awards (src/app/awards.rs)

awards-awards = AUSZEICHNUNGEN
awards-band = Band
awards-all = Alle
awards-dxcc = DXCC
awards-grids = Locatoren
awards-worked-all-states = Worked All States
awards-cq-zones-waz = CQ-Zonen (WAZ)
awards-dxcc-entities = DXCC-Gebiete

## acars (src/app/panels/acars.rs)

acars-acars = ACARS
acars-audio-decoder-s-passband = Audio im Durchlassbereich des Decoders.
acars-blocks-framed-blocks-whose-check = Blöcke, die gerahmt wurden, und Blöcke, deren Prüfsequenz fehlschlug.
acars-starting-acars-receiver = ACARS-Empfänger wird gestartet…
acars-channel = KANAL
acars-tune-dial-here = Die Skala hier abstimmen.
acars-nothing-yet-acars-quiet-between = Noch nichts. ACARS ist zwischen den Flugzeugen still — auf einem Kanal lassen und auf einen Burst warten.
acars-check-failed = Prüfung fehlgeschlagen
acars-block-check-sequence-did-not = Die Blockprüfsequenz stimmte nicht überein — der Text ist möglicherweise falsch.
acars-no-text = (kein Text)

## login (src/login.rs)

login-sdr-oxide = SDR OXIDE
login-username = Benutzername
login-password = Passwort
login-remember-device = Auf diesem Gerät merken
login-signs-without-asking-next-time = Meldet beim nächsten Mal ohne Nachfrage an und lässt den Reiter der 3D-Ansicht ebenfalls ohne Nachfrage hinein. Das Passwort wird unverschlüsselt gespeichert, daher auf einem Rechner, den andere benutzen, ausgeschaltet lassen.
login-keeps-password-device-clear = Behält das Passwort unverschlüsselt auf diesem Gerät.
login-remember-station = Diese Station merken
login-remember-me = ANGEMELDET BLEIBEN
login-station-keeps-sign-in = Die Station behält Ihre Anmeldung, nicht diese Seite — nichts auszufüllen an den anderen Funkgeräten und nichts zu vergessen, wenn Sie gehen.
login-not-this-time = Diesmal nicht: Dieser Browser vergisst sie beim Schließen. Sie werden in der nächsten Sitzung erneut gefragt, und von dieser Anmeldung bleibt nichts auf dem Gerät zurück.
login-checking = {" "}PRÜFE…{" "}
login-sign = {" "}ANMELDEN{" "}
login-wrong-password-shuts-server-s = Ein falsches Passwort schließt die Tür des Servers für einige Sekunden, daher kann das einen Moment dauern.

## help (src/help.rs)

help-find = SUCHEN
help-find-manual = im Handbuch suchen
help-previous-match-shift-enter-shift = Voriger Treffer (Umschalt+Enter, Umschalt+F3)
help-next-match-enter-f3 = Nächster Treffer (Enter, F3)
help-no-matches = keine Treffer
help-clear-search-esc = Die Suche leeren (Esc)
help-sdroxide-manual = SDROXIDE-HANDBUCH
help-contents = INHALT

## aismap (src/ais_map.rs)

aismap-t-1-station-heard-waiting-report = 1 Station gehört — warte, bis sie eine Position meldet
aismap-double-click-reframe = Doppelklick zum Neurahmen
aismap-zoom-out-wheel-pinch-do = Herauszoomen — das Rad oder eine Fingerbewegung tut das ebenfalls
aismap-zoom-about-middle-chart = Um die Mitte der Karte hineinzoomen
aismap-frame-everything-being-tracked-again = Alles, was verfolgt wird, erneut einrahmen — ein Doppelklick tut das ebenfalls

## specview (src/widgets/spectrum_view.rs)

specview-waiting-spectrum = warte auf das Spektrum…
specview-hounds = HUNDE

## spectrum (src/app/spectrum.rs)

spectrum-window-has-no-gpu-renderer = Dieses Fenster hat keinen GPU-Renderer, daher bleibt der Wasserfall bei seiner Standardbreite.
spectrum-machine-drawing-without-gpu-every = Dieser Rechner zeichnet ohne GPU — jede Spalte des Wasserfalls ist Arbeit des Prozessors, und er teilt sie schon mit dem Funkgerät.
spectrum-window-renders-through-opengl-which = Dieses Fenster rendert über OpenGL, dem Kompatibilitätsweg von sdroxide — ein Raspberry Pi, ein älterer Grafikchip oder ein Browser ohne WebGPU. Ein breiterer Wasserfall lohnt dort die Bildrate nicht.
spectrum-wider-than-renderer-can-draw = Breiter, als dieser Renderer zeichnen kann.

## macros (src/app/panels/macros.rs)

macros-nocall = NOCALL
macros-each-button-sends-whole-text = Jede Taste sendet ihren ganzen Text auf einmal. F2–F10 drücken die ersten neun, solange auf dem Bildschirm nichts die Tastatur hat.
macros-remove-button = Diese Taste entfernen

## gridtracker (src/app/grid_tracker.rs)

gridtracker-grid-tracker = LOCATOR-TRACKER
gridtracker-shade-squares-heard-live-decode = Die Quadrate schattieren, die auf der Live-Dekodierliste gehört wurden, nicht nur die im Logbuch — was jetzt auf Sendung ist, in Cyan unter den gearbeiteten Quadraten.
gridtracker-drag-pan-wheel-zoom-amber = Ziehen zum Schwenken, Rad zum Zoomen. Bernstein = gearbeitet, Grün = bestätigt, Cyan = gehört.

## remote (src/remote.rs)

remote-server-busy-another-client-connected = Server belegt — ein anderer Client ist verbunden
remote-transmitting-but-client-s-microphone = Es wird gesendet, aber das Mikrofon dieses Clients liefert gar kein Audio — es wird nichts moduliert. In einem Browser prüfen, ob die Seite das Mikrofon bekommen hat und keine andere Anwendung es belegt.
remote-connection-closed = Verbindung geschlossen

## solarwin (src/solar3d/mod.rs)

solarwin-sdroxide-solar-system = sdroxide — Sonnensystem

## solarbtn (src/app/solar.rs)

solarbtn-solar-system-3d-view-sun = 3D-Ansicht des Sonnensystems — Sonne, Erde, Mond, Sonnenflecken und KMA (eigenes Fenster)
solarbtn-solar-system-3d-view-sun-2 = 3D-Ansicht des Sonnensystems — Sonne, Erde, Mond, Sonnenflecken und KMA (neuer Browser-Reiter)

## widespec (src/widgets/wide_spectrum.rs)

## speech (src/app/speech.rs)

speech-loading-voice = Die Stimme wird geladen…
speech-browser-client-cannot-speak-yet = Der Browser-Client kann noch nicht sprechen

## freqdisplay (src/widgets/freq_display.rs)

freqdisplay-double-click-type-frequency = Doppelklick, um eine Frequenz einzutippen
freqdisplay-tap-type-frequency = Tippen, um eine Frequenz einzutippen

## solarstate (src/solar3d/state.rs)

solarstate-home = START
solarstate-dwarf-planets = ZWERGPLANETEN

## adsbmap (src/adsb_map.rs)

adsbmap-double-click-reframe = Doppelklick zum Neurahmen

## aprsmap (src/aprs_map.rs)

aprsmap-double-click-reframe = Doppelklick zum Neurahmen

## solarnet (src/solar3d/net.rs)

solarnet-connection-closed = Verbindung geschlossen

## alerts (src/app/alerts.rs)

alerts-could-not-start-alert-thread = Der Hinweis-Thread konnte nicht gestartet werden

## hfdlmap (src/hfdl_map.rs)

hfdlmap-no-aircraft-located-yet = noch kein Flugzeug geortet

## worldmap (src/widgets/worldmap.rs)

worldmap-double-click-reframe = DOPPELKLICK ZUM NEURAHMEN

## chrome (src/chrome.rs)

## duopower (src/app/duo_power.rs)

duopower-sets-fdm-duo-s-own = Stellt die eigene Leistungseinstellung des FDM-DUO ein (Menü 34 TX POWER). Aussteuerung und TUNE setzen dann die Leistung innerhalb davon, und SDRoxide sendet die Einstellung bei jeder Verbindung erneut.

## Band conditions window: a peak date.

# $month is 1–12.
bands-peak-date = { $day }. { $month ->
        [1] Jan.
        [2] Feb.
        [3] März
        [4] Apr.
        [5] Mai
        [6] Juni
        [7] Juli
        [8] Aug.
        [9] Sep.
        [10] Okt.
        [11] Nov.
       *[12] Dez.
    }

## download (src/download.rs)

download-browser-could-not-read-file = Der Browser konnte diese Datei nicht lesen

## morse (src/app/morse.rs)

morse-translate = ÜBERSETZEN
morse-practice = ÜBEN
morse-learn = LERNEN
morse-morse = MORSE
morse-text-morse = Text → Morse
morse-morse-text = Morse → Text
morse-per-character-space-between-characters = `.` und `-` je Zeichen, ein Leerzeichen zwischen den Zeichen, ` / ` zwischen Wörtern. Das ITU-Alphabet, Ziffern, Satzzeichen und die gängigen Prosigns.
morse-reference-translator-available-desktop-build = Der Referenz-Übersetzer ist im Desktop-Build verfügbar.
morse-play-what-you-type-your = Abspielen, was Sie tippen, über Ihre eigenen Lautsprecher
morse-play = ABSPIELEN
morse-stop = STOPP
morse-every-character-learned-well-done = Jedes Zeichen gelernt — gut gemacht.
morse-drill-needs-sound-which-desktop = Die Übung braucht Ton, den der Desktop-Build hat.
morse-reveal = AUFDECKEN
morse-reset = ZURÜCKSETZEN
morse-replay = WIEDERHOLEN
morse-what-did-you-hear = Was haben Sie gehört?
morse-check = PRÜFEN
morse-press-new-hear-character = NEU drücken, um ein Zeichen zu hören.
morse-tone = Ton
morse-speed = Geschwindigkeit
morse-spacing = Abstand
morse-characters-go-out-speed-above = Zeichen gehen mit der oben eingestellten Geschwindigkeit hinaus; dies ist die Geschwindigkeit, mit der die *Pausen* gesendet werden, damit ein Anfänger den Buchstaben als Ganzes hört, bevor er ihn so schnell senden kann.
morse-playback-available-desktop-build = Die Wiedergabe ist im Desktop-Build verfügbar.

## ADS-B panel: aircraft detail rows.

adsb-row-altitude = Höhe
adsb-row-track = Kurs
adsb-row-turning = Kurvenflug
adsb-row-squawk = Squawk
adsb-row-range = Entfernung
adsb-row-position = Position
adsb-row-signal = Signal
adsb-row-frames = Rahmen
rade-transmitting = sendet
decodes-odd = UNGERADE
decodes-new = NEU
js8-new = NEU

## AIS panel: vessel detail rows.

ais-row-status = Status
ais-row-speed = Geschwindigkeit
ais-row-course = Kurs
ais-row-heading = Richtung
ais-row-turning = Kurvenflug
ais-row-altitude = Höhe
ais-row-size = Größe
ais-row-draught = Tiefgang
ais-row-destination = Ziel
ais-row-range = Entfernung
ais-row-signal = Signal
ais-row-type = Typ
ais-row-position = Position
ais-row-messages = Meldungen
ism-off = AUS
textmodem-frequency-shifted = frequenzverschoben
winlink-to-heading = AN

## adsbmap: messages with values (src/adsb_map.rs)

adsbmap-ft-min =
    {""}
    { $vr } ft/min

## aismap: messages with values (src/ais_map.rs)

aismap-heard-no-position-newer-than = { $heard } gehört — keine Position, die neuer als { $drop_map_s } s ist
aismap-stations-heard-waiting-position-report = { $heard } Stationen gehört — warte auf eine Positionsmeldung
aismap-km-across = { $lon_span }° · ~{ $km } km breit

## alerts: messages with values (src/app/alerts.rs)

alerts-alarming = alarmiert über { $device }
alerts-alarm-device-unavailable = Alarmgerät nicht verfügbar: { $error }

## awards: messages with values (src/app/awards.rs)

awards-worked = { $w } gearbeitet
awards-confirmed = { $value } bestätigt

## bands: messages with values (src/app/bands.rs)

bands-forecast-hamqsl-com-old = · Vorhersage von HAMQSL.com, { $value } alt
bands-ncdxf-iaru-next-beacon-s = NCDXF/IARU · nächste Bake in { $now } s
bands-hearing-means-path-open-each = { $callsign } bei { $location } — ihn zu hören heißt, dass der Weg auf { $band } offen ist. Jede Bake steigt alle 10 s ein Band höher, daher ändert sich diese Zeile in jedem Zeitschlitz.
bands-radiants-placed = Radianten für { $lat }°, { $lon }° eingetragen
bands-zhr = ZHR { $zhr }
bands-radiant = Radiant { $alt_deg }° { $az_deg }
bands-radiant-down = Radiant unten ({ $alt_deg }°)
bands-km-s-parent-peak-rate =
    { $value } ({ $code }) — { $velocity_kms } km/s, Mutterkörper { $parent }. Spitzenrate ZHR { $zhr } um { $peak }.

    Ein Radiant über dem Horizont heißt, dass die Spuren Sie erreichen können; ein schneller Schauer hinterlässt länger lebende ionisierte Spuren für Meteorstreuung auf 6 m und 2 m sowie die kurzen Öffnungen auf 10 m.

## drm: messages with values (src/app/drm.rs)

drm-not-decoding = { $channel } wird nicht dekodiert
drm-symbols = { $value } Symbole · { $frame }
drm-not-decodable = { $value } — nicht dekodierbar
drm-broadcaster-s-clock-utc = Uhr des Senders { $year }-{ $month }-{ $day } { $hour }:{ $minute } UTC

## duopower: messages with values (src/app/duo_power.rs)

duopower-max-fdm-duo-s-full =
    MAX: die volle Leistung des FDM-DUO, etwa 8 W. Die Einstellung für den HR500 in der Leitung.

    { $base }
duopower-t-5-w-hr500-line-will =
    5 W. Mit dem HR500 in der Leitung erreicht es bei dieser Einstellung auf den meisten Bändern nicht die volle Leistung: MAX nutzen und mit der Aussteuerung herunterregeln.

    { $base }
duopower-w-running-barefoot-qrp =
    { $code } W, für den Betrieb ohne Verstärker oder QRP.

    { $base }

## frame: messages with values (src/app/frame.rs)

frame-re-enables-transmit-after-1 = Aktiviert das Senden nach { $swr }:1 wieder. Zuerst die Antenne prüfen: Besteht der Fehler weiter, stoppt auch die nächste Aussendung.
frame-sdroxide-has-been-released = SDRoxide { $version } ist erschienen — dies ist { $sdroxide_version }.
frame-reconnecting-s = Verbinde in { $value } s erneut…

## gridtracker: messages with values (src/app/grid_tracker.rs)

gridtracker-worked = { $worked } gearbeitet
gridtracker-confirmed = { $confirmed } bestätigt
gridtracker-heard = { $heard } gehört

## hd: messages with values (src/app/hd.rs)

hd-decode = Dekodieren { $value } ({ $codec_label })

## hfdl: messages with values (src/app/hfdl.rs)

hfdl-khz-already-has-chip = { $cur_khz } kHz hat schon einen Chip
hfdl-save-current-channel-khz-one = Den aktuellen Kanal ({ $cur_khz } kHz) als einen Ihrer eigenen Chips speichern
hfdl-aircraft = { $aircraft } Flugzeuge
hfdl-bursts = { $bursts } Bursts
hfdl-decodes-2 = { $decodes } Dekodierungen
hfdl-fixed = { $fec } korrigiert

## ism: messages with values (src/app/ism.rs)

ism-not-decoded-yet = { $hint } — noch nicht dekodiert
ism-listen-tunes-radio-there-one = Auf { $value } hören — stimmt das Funkgerät dort ab. Ein Band nach dem anderen: Sie liegen zu weit auseinander, als dass ein Empfänger zwei auf einmal abdecken könnte.
ism-watch-hz-around-band-centre = { $value } Hz um die Bandmitte beobachten, was auch immer das Band verlangt hätte. Schmaler passt zu einem Empfänger, der die eigene Breite des Bandes nicht liefern kann; breiter erreicht Geräte, die weiter von der Mitte entfernt sitzen. Ihr Empfänger muss etwa ein Drittel mehr als das hier liefern.
ism-rtl-433 = rtl_433 — { $why }
ism-rtl-433-khz = rtl_433 { $value } · { $khz } kHz
ism-your-own = { $flex } eigener
ism-decoders-decoded = { $decoders } Decoder ({ $flex }), { $decodes } dekodiert
ism-handled-rtl-433 = von rtl_433 übernommen: { $names }
ism-flex = flex: { $error }
ism-window-mhz-mhz-wide = Fenster { $mhz } MHz, { $e62 } MHz breit
ism-bursts-decoded = { $bursts } Bursts, { $decodes } dekodiert
ism-devices = { $ism_reports } Geräte
ism-id-mhz-db-heard-first =
    { $protocol } Kennung { $device }
    { $e6 } MHz { $snr_db } dB gehört { $value }×
    zuerst gehört vor { $first_at }

## logbook: messages with values (src/app/logbook.rs)

logbook-confirmations-downloaded-newly-confirmed = Bestätigungen: { $recs } heruntergeladen, { $matched } neu bestätigt
logbook-qso = { $qso_log } QSO
logbook-worked-before = ⚠ SCHON GEARBEITET ({ $dupe_band })
logbook-utc-qso = { $oldest }–{ $newest } UTC · { $value } QSO

## app: messages with values (src/app/mod.rs)

app-radio = Funkgerät { $id }

## morse: messages with values (src/app/morse.rs)

morse-could-not-start-morse-audio = Der Morse-Audio-Thread konnte nicht gestartet werden: { $error }
morse-no-audio-output = Keine Audioausgabe: { $error }
morse-about-s = etwa { $farnsworth_wpm } s
morse-characters-play = Zeichen im Spiel: { $set }
morse-score-run = Ergebnis: { $correct } / { $total }. Serie: { $streak } von { $value }.
morse-correct = richtig — { $value }
morse-was = das war { $value }

## net: messages with values (src/app/net.rs)

net-synced-z-new-already-log = Synchronisiert { $when }Z: { $added } neu, { $matched } schon im Logbuch
net-wavelog-2 = Wavelog: { $line }
net-full-resync-stopped-nothing-changed = Vollständiger Abgleich abgebrochen, nichts geändert: { $error }
net-full-resync-applied-added-updated = Vollständiger Abgleich übernommen: { $add } hinzugefügt, { $update } aktualisiert, { $remove } entfernt, { $mark } eigene als in Wavelog markiert
net-wavelog-3 = Wavelog: { $line }
net-adif-import-failed = ADIF-Import fehlgeschlagen: { $error }
net-not-unicode-read = {" "}(nicht Unicode; gelesen als { $enc })
net-received-reports-swl-left-out = , { $n } empfangene Berichte (SWL) ausgelassen
net-adif-import-added-duplicates-skipped = ADIF-Import: { $added } hinzugefügt, { $skipped } Duplikate übersprungen{ $reports }{ $assumed }
net-channel-import-failed = Kanalimport fehlgeschlagen: { $error }
net-line-s-skipped = , { $n } Zeile(n) übersprungen
net-not-unicode-read-2 = {" "}(nicht Unicode; gelesen als { $enc })
net-channel-import-channel-s-read = Kanalimport: { $n } Kanal/Kanäle gelesen{ $skipped }{ $assumed }

## acars: messages with values (src/app/panels/acars.rs)

acars-frames-bad = { $frames } Rahmen · { $bad } fehlerhaft
acars-mode = Modus { $mode }

## adsb: messages with values (src/app/panels/adsb.rs)

adsb-aircraft-3 = { $aircraft } Flugzeuge
adsb-frames = { $frames } Rahmen
adsb-preambles = { $preambles } Präambeln
adsb-bad-crc = { $bad_crc } fehlerhafte CRC
adsb-tune = ABSTIMMEN { $mhz }
adsb-ft-barometric = { $ft } ft barometrisch

## ais: messages with values (src/app/panels/ais.rs)

ais-vessels-3 = { $vessels } Schiffe
ais-messages = { $messages } Meldungen
ais-slots = { $bursts } Zeitschlitze
ais-bad-fcs = { $bad_fcs } fehlerhafte FCS
ais-ais-mhz = AIS { $value } ({ $mhz } MHz) — { $value2 }
ais-ais-mhz-slots-messages-floor = AIS { $value } ({ $mhz } MHz) — { $bursts } Zeitschlitze, { $messages } Meldungen, Untergrenze { $floor_dbfs } dBFS
ais-ais = AIS { $value }
ais-tune = ABSTIMMEN { $mhz }
ais-ships-heard-are-khz-off = Die gehörten Schiffe liegen { $khz } kHz neben der Frequenz ({ $mhz } ppm bei 162 MHz) — die Frequenzkorrektur der Frontstufe einstellen, sonst hören Sendungen ab etwa 5 kHz Versatz auf zu dekodieren
ais-over-ground = { $value }° über Grund
ais-dbfs-db-snr = { $rssi_dbfs } dBFS, { $snr_db } dB SNR
ais-last-type-ais = { $messages } (letzter Typ { $last_type }, AIS { $channel })
ais-ais-2 = AIS { $ch }
ais-mhz-marine-channel-ship-alternates = { $mhz } MHz — Seefunkkanal { $ch }. Ein Schiff wechselt zwischen beiden, daher halbiert das Abschalten eines davon, wie oft jedes Schiff gehört wird.

## aprs: messages with values (src/app/panels/aprs.rs)

aprs-object = Objekt von { $reported_by }
aprs-via = über { $via }
aprs-object-reported = Objekt gemeldet von { $reported_by }
aprs-digit-s-blanked = ± { $ambiguity } Ziffer(n) ausgeblendet
aprs-wind-m-s = Wind { $value }° { $value2 } m/s
aprs-gust = Böe { $value }
aprs-rain-mm-h = Regen { $value } mm/h
aprs-frame-s-last-ago = { $packets } Rahmen, zuletzt vor { $last_heard }
aprs-via-2 = über { $via }
aprs-bad = { $bad_frames } fehlerhaft
aprs-other = { $non_aprs } andere
aprs-sent = gesendet ×{ $tries }
aprs-stn = { $stations } Stn.
aprs-queued = { $tx_queue } in der Warteschlange

## atchat: messages with values (src/app/panels/atchat.rs)

atchat-master = Master { $value }
atchat-last-heard-s-ago-click = { $status } — zuletzt gehört vor { $age_s } s · klicken, um einen Direktnachrichten-Reiter zu öffnen
atchat-close-tab = den Reiter { $peer } schließen
atchat-direct-message = Direktnachricht an { $value }
atchat-cannot-display = { $path } kann nicht angezeigt werden

## cw: messages with values (src/app/panels/cw.rs)

cw-click-decoder-neural-one-copies =
    { $hint }

    Für den Decoder { $neural } klicken. Der neuronale kopiert weiter unten und liest von Hand gegebenes CW, das eine Taktanpassung nicht annimmt; der Taktdecoder ist der einzige, der die Umlaute Ä, Ö, Ü und das É kopiert, weil das Modell dafür keine Ausgabeklasse hat.
cw-idle-s = LEERLAUF { $idle } s

## decodes: messages with values (src/app/panels/decodes.rs)

decodes-save-decodes-list-adif-received = Die { $n } Dekodierungen der Liste als ADIF speichern — empfangene Berichte, keine Kontakte. Dekodierungen ohne Absender werden ausgelassen.
decodes-save-decodes-list-csv-one = Die { $n } Dekodierungen der Liste als CSV speichern, je eine Zeile
decodes-session-qso = Sitzung: { $session } QSO
decodes-save-whole-logbook-qso-adif = Das ganze Logbuch ({ $logged } QSO) als ADIF speichern
decodes-save-whole-logbook-qso-text = Das ganze Logbuch ({ $logged } QSO) als Text speichern
decodes-hz-slots = { $audio_hz } Hz · { $odd } Zeitschlitze
decodes-your-slot-timing-against-stations =
    Ihr Zeitschlitz-Takt im Vergleich zu den Stationen, die Sie hören.
    { $after }

    Das umfasst den ganzen Empfangsweg, daher zählt eine langsame Audio- oder Netzwerkkette genauso wie eine falsche Uhr. Unter 0,5 s ist komfortabel.
decodes-pile-up = PILE-UP { $fox_queue }
decodes-queue = WARTESCHLANGE { $call_queue }
decodes-db-hz-click-remove =
    { $waiting } · { $snr_db } dB · { $audio_hz } Hz{ $unwrap_or_default }
    Klicken zum Entfernen

## js8: messages with values (src/app/panels/js8.rs)

js8-every-speed-being-decoded-marks = Jede Geschwindigkeit wird dekodiert; ▸ markiert { $speed }, die, mit der Sie senden
js8-every-speed-being-decoded-click = Jede Geschwindigkeit wird dekodiert. Klicken, um stattdessen mit { $speed } zu senden
js8-transmit-decode = Mit { $speed } senden und dekodieren
js8-beaconing-every-min-click-stop = Bake alle { $hb_min } Min. — klicken zum Stoppen
js8-last-beacon-went-out-hz = Die letzte Bake ging bei { $hz } Hz hinaus — ein freier Platz im Herzschlag-Teilband { $value }–{ $value2 } Hz, gewählt, damit sie abseits der dekodierten Signale liegt.
js8-frames-s = { $tx_frames_pending }/{ $tx_frames_total } Rahmen · { $left } s
js8-decoded = Dekodiert bei { $speed }
js8-frames = … ({ $frames } Rahmen)
js8-sent = Gesendet bei { $speed }
js8-partial-frames-sent = unvollständig ({ $frames } Rahmen gesendet)
js8-reply-2 = Antwort an { $value }: „{ $value2 }“
js8-address-composer = Den Verfasser an { $value } adressieren

## macros: messages with values (src/app/panels/macros.rs)

macros-f-sends = F{ $i }: sendet „{ $text }“
macros-sends = Sendet „{ $text }“
macros-most = { $value } ist das Höchstmaß

## navtex: messages with values (src/app/panels/navtex.rs)

navtex-repaired-lost = { $repaired } korrigiert · { $lost } verloren
navtex-characters-lost = { $lost } Zeichen verloren

## packet: messages with values (src/app/panels/packet.rs)

packet-bad = { $bad_frames } fehlerhaft
packet-link-layer = Die Verbindungsschicht ist in { $link }
packet-via = {" "}über { $via }
packet-unacked = { $unacked } unbestätigt
packet-retry = Wiederholung { $retries }

## pi4: messages with values (src/app/panels/pi4.rs)

pi4-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] Bake
       *[other] Baken
    }
pi4-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } her

## rade: messages with values (src/app/panels/rade.rs)

rade-snr-db = SNR { $snr_db } dB
rade-offset-hz = Versatz { $freq_offset_hz } Hz
rade-heard = gehört { $call }
rade-samples-dropped-between-receiver-decoder = ⚠ { $dropped } Abtastwerte zwischen Empfänger und Decoder verworfen — dieser Rechner kommt mit der neuronalen Dekodierung nicht mit.

## textmodem: messages with values (src/app/panels/text_modem.rs)

textmodem-char-s = { $chars_per_sec } Zeichen/s
textmodem-char-s-hz-wide = { $value } — { $chars_per_sec } Zeichen/s, { $bandwidth_hz } Hz breit, { $keyed }

## vdl2: messages with values (src/app/panels/vdl2.rs)

vdl2-frames = { $frames } Rahmen
vdl2-bursts = { $bursts } Bursts
vdl2-sync = { $syncs } Sync
vdl2-hdlc-bad = { $hdlc_bad } HDLC fehlerhaft
vdl2-bad-fcs = { $fcs_bad } fehlerhafte FCS
vdl2-mhz-bursts-frames-noise-floor = { $mhz } MHz, { $what } — { $bursts } Bursts, { $frames } Rahmen, Rauschboden { $floor_dbfs } dBFS
vdl2-tune = ABSTIMMEN { $mhz }
vdl2-held-2 = GEHALTEN +{ $since }
vdl2-use-worldwide-where-every-link = { $what } — weltweit im Einsatz, und dort beginnt jede Verbindung. Die eine, die man behält, wenn man nur eine behält.
vdl2-assigned-which-channels-carry-anything = Zugewiesen an { $what }. Welche Kanäle etwas führen, hängt vom Standort ab: Alle eingeschaltet lassen, sofern Sie nichts anderes wissen.
vdl2-mhz-db-snr-dbfs-evm = { $mhz } MHz · { $snr_db } dB SNR · { $rssi_dbfs } dBFS · EVM { $evm_deg }° · { $freq_err_hz } Hz · { $rs_corrected } RS-Symbole korrigiert
vdl2-characters = { $parity_errors } Zeichen
vdl2-parameter-sdroxide-does-not-read = { $unknown } { $unknown ->
        [one] Parameter
       *[other] Parameter
    } liest SDRoxide nicht — oben als Kennung und Bytes gezeigt

## wefax: messages with values (src/app/panels/wefax.rs)

wefax-lines = { $lines } Zeilen
wefax-lines-held = { $live_size } Zeilen gehalten
wefax-pixels-per-line = { $width } Pixel je Zeile
wefax-shift-picture-pixels = Das Bild um { $px } Pixel verschieben
wefax-charts-are-saved-click-copy =
    Karten werden { $place } gespeichert

    Klicken, um den Pfad zu kopieren
wefax-finished-charts-are-saved-collect = Fertige Karten werden { $place } gespeichert und sammeln sich hier.
wefax-pixels-right-click-delete =
    { $when_full }
    { $size } × { $size2 } Pixel
    { $value }

    Rechtsklick zum Löschen
wefax-older-store = { $older } ältere im Speicher
wefax-older-load-more = { $older } ältere — mehr laden
wefax-published-carrier-khz-dial-khz = Veröffentlichter Träger { $f } kHz → Skala { $value } kHz USB
wefax-delete-chart-2 = Diese Karte löschen { $dir }

## widgets: messages with values (src/app/panels/widgets.rs)

widgets-cq-zone-itu-zone = { $continent } · CQ-Zone { $cq_zone } · ITU-Zone { $itu_zone }
widgets-new-entity = Neues Gebiet auf { $band_label }
widgets-worked-before = Schon gearbeitet auf { $band_label }
widgets-calling-cq-includes-you = CQ-Ruf an { $target } — das schließt Sie ein
widgets-calling-cq-not-aimed-you = CQ-Ruf an { $target } — nicht an Sie gerichtet

## wspr: messages with values (src/app/panels/wspr.rs)

wspr-rx-heard-us = { $heard_us } rx · { $heard_us2 } hörten uns
wspr-beacon = { $last_slot_spots } { $last_slot_spots ->
        [one] Bake
       *[other] Baken
    }
wspr-beaconing-duty = sendet Bake — { $wspr_power_dbm }, { $wspr_tx_percent } % Tastverhältnis
wspr-listening-duty = hört zu ({ $wspr_tx_percent } % Tastverhältnis)
wspr-beacon-one-two-minute-slot = Bake in einem Zwei-Minuten-Zeitschlitz aus jedem { $pct } — { $pct2 } % davon, gleichmäßig verteilt — mit { $wspr_power_dbm }. Auf welchem Zeitschlitz der Zyklus beginnt, ergibt sich aus Ihrem Rufzeichen, daher starten zwei Stationen mit diesem Programm nicht gleichzeitig.
wspr-transmitting-set-general-tab-settings = Sendet als { $call } in { $sent } — eingestellt im Reiter Allgemein der Einstellungen.
wspr-utc-ago = { $t }:{ $t2 } UTC — { $slot_utc } her

## publicsdr: messages with values (src/app/publicsdr.rs)

publicsdr-antenna =
    { $address }
    { $device }
    Antenne: { $antenna }
    { $e3 }
publicsdr-receivers-fetched-ago = { $entries } Empfänger · { $per_net } · abgerufen vor { $age }
publicsdr-showing-search-narrow = { $shown } von { $n } angezeigt — suchen, um einzugrenzen
publicsdr-match = { $n } Treffer
publicsdr-map-status = { $shown } auf der Karte · { $unplaced } ohne Position
publicsdr-opening-another-radio = { $entry } wird als weiteres Funkgerät geöffnet…

## qo100: messages with values (src/app/qo100.rs)

qo100-khz-3 = +{ $park_lo_hz } … +{ $park_hi_hz } kHz
qo100-before-switching-tune-so-beacon = Vor dem Einschalten so abstimmen, dass die beiden Keulen der Bake im Streifen zwischen +{ $park_lo_hz } und +{ $park_hi_hz } kHz liegen (der schattierte Bereich).
qo100-receiver-currently-listening-around-mhz = Der Empfänger hört gerade um { $mhz } MHz — weit weg von der Bake bei 10489,750 MHz, daher gibt es hier nichts zu zeichnen. Der Decoder sucht trotzdem weiter.
qo100-receiver-currently-covers-only-khz = Der Empfänger deckt hier nur ±{ $khz } kHz ab — der Rest des Streifens bleibt leer
qo100-null-db-sym-snr-db = Auslöschung { $est_null_depth_db } dB sym { $est_symmetry } SNR { $est_snr_db } dB
qo100-hz-s-hz-s-de = { $est_drift_hz_s } Hz/s { $est_drift_accel_hz_s2 } Hz/s² (vor dem Dekodieren zurückgedreht)
qo100-hz-s-de-rotated-before = { $est_drift_hz_s } Hz/s (vor dem Dekodieren zurückgedreht)
qo100-found-empty = { $est_updates } gefunden, { $est_misses } leer
qo100-last-s-ago = {"   "}zuletzt { $auto_last_hz } (vor { $auto_last_unix } s)
qo100-over-correction = { $auto_total_hz } über { $auto_applies } { $auto_applies ->
        [one] Korrektur
       *[other] Korrekturen
    }{ $last }
qo100-sync-32-err = Sync { $sync_bit_errors } / 32 Fehler ×{ $sync_matches }
qo100-frame-buffer = Rahmenpuffer { $frame_fill } %
qo100-blocks-tried-decoded = { $blocks_tried } Blöcke versucht, { $blocks_locked } dekodiert
qo100-last-applied-s-ago-hz = zuletzt angewendet vor { $ago } s: { $old } → { $new } Hz ({ $old2 } Hz)
qo100-locked-block-tried-locked = eingerastet — { $blocks_tried } { $blocks_tried ->
        [one] Block versucht
       *[other] Blöcke versucht
    }, { $blocks_locked } eingerastet
qo100-searching-block-tried-none-locked = suche — { $blocks_tried } { $blocks_tried ->
        [one] Block versucht
       *[other] Blöcke versucht
    }, bisher keiner eingerastet

## rds: messages with values (src/app/rds.rs)

rds-auto = Auto ({ $standard })
rds-type = Typ { $class }
rds-groups = · { $groups } Gruppen
rds-blocks-error = · { $ber } % der Blöcke fehlerhaft
rds-clean-corrected-lost = { $blocks_ok } sauber · { $blocks_corrected } korrigiert · { $blocks_bad } verloren

## sat: messages with values (src/app/sat.rs)

sat-utc-local = { $unix } UTC ({ $offset } Ortszeit)
sat-aos = AOS { $rise_unix } { $rise_unix2 }
sat-los = LOS { $set_unix } { $set_unix2 }
sat-above-horizon = { $rise_unix } über dem Horizont
sat-min = in { $value } Min.
sat-h-min = in { $value } h { $value2 } Min.
sat-searching = suche… { $next } von { $entries }
sat-locked-2 = ● EINGERASTET — { $t }
sat-pass-until-max = Überflug bis { $set_unix } · max. { $max_el }°
sat-next-pass-rises-max = Nächster Überflug { $rise_unix } { $rise_unix2 } ({ $now }) · steigt bei { $rise_az }° { $rise_az2 } · max. { $max_el }°
sat-rigctld-client-connected-satellite-software = ⚠ { $clients } rigctld-{ $clients ->
        [one] Client
       *[other] Clients
    } verbunden — Satelliten-Software, die die Skala dort steuert, würde den Doppler zweimal korrigieren
sat-rotator-antenna-az-el = Rotor: Antenne bei { $az }° Az { $el }° El
sat-rotator-2 = Rotor: { $error }
sat-up-now-2 = ● jetzt oben, { $el }°
sat-inv = { $mode } · inv
sat-norad-2 = NORAD { $norad_id }

## scanner: messages with values (src/app/scanner.rs)

scanner-holding-mhz = hält { $here } MHz
scanner-scanning-mhz = sucht · { $here } MHz

## spectrum: messages with values (src/app/spectrum.rs)

spectrum-renderer-will-not-hold-texture = Dieser Renderer kann keine Textur halten, die breiter als { $max_texture_dim } Pixel ist.
spectrum-mem = Speicher: { $f } / { $value }
spectrum-mem-2 = Speicher: { $value }

## speech: messages with values (src/app/speech.rs)

speech-voice-2 = Stimme: { $voice }
speech-speech-unavailable = Sprachausgabe ist nicht verfügbar: { $error }

## spots: messages with values (src/app/spots.rs)

spots-transmissions = { $stations } Aussendungen

## winlink: messages with values (src/app/winlink.rs)

winlink-last-session-failed = letzte Sitzung fehlgeschlagen — { $err }
winlink-last-session-received-sent = letzte Sitzung: { $last_received } empfangen, { $last_sent } gesendet
winlink-empty = { $to_lowercase } ist leer
winlink-showing = zeige { $entries } von { $total }
winlink-fetching = hole { $mid }…
winlink-from-2 = von { $msg } { $date }
winlink-attachment-bytes = Anhang: { $att } ({ $data } Bytes)

## aprsmap: messages with values (src/aprs_map.rs)

aprsmap-object =
    {""}
    Objekt von { $reported_by }
aprsmap-heard-ago =
    {""}
    gehört vor { $age }

## download: messages with values (src/download.rs)

download-reading = Lese { $path }: { $error }

## help: messages with values (src/help.rs)

help-missing-image = {"["}Bild fehlt: { $path }]

## hfdlmap: messages with values (src/hfdl_map.rs)

hfdlmap-mhz =
    {""}
    { $e3 } MHz
hfdlmap-fix =
    {""}
    { $fixes } Fix{ $es }
hfdlmap-heard-ago =
    {""}
    gehört vor { $last_at }

## multi: messages with values (src/multi.rs)

multi-radio = Funkgerät { $n }
multi-could-not-add-radio = Ein Funkgerät konnte nicht hinzugefügt werden: { $error }
multi-was-closed-station = { $value } wurde an der Station geschlossen.
multi-could-not-open = { $peer } konnte nicht geöffnet werden: { $error }
multi-has-tab-own-now = { $value } hat jetzt einen eigenen Reiter.

## remote: messages with values (src/remote.rs)
remote-protocol-error = Protokollfehler: { $error }

## solarnet: messages with values (src/solar3d/net.rs)

solarnet-bad-message = ungültige Nachricht: { $error }
solarnet-solar-protocol-mismatch-server-client = Sonnenprotokoll stimmt nicht überein: Server { $proto }, Client { $value }

## smeter: messages with values (src/widgets/smeter.rs)

smeter-forward-power-w-measured-radio =
    {""}

    Vorwärtsleistung — { $w } W, vom eigenen Sensor des Funkgeräts gemessen.
smeter-po-radio-s-own-power =
    {""}

    PO — das eigene Leistungsmessgerät des Funkgeräts, bei { $clamp } % seines Vollausschlags. Eine Zeigerstellung, wie das Funkgerät sie meldet, keine kalibrierte Wattzahl; zu beobachten ist, ob sie fällt, während die Aussteuerung gleich bleibt.
smeter-swr-1-match-antenna-presenting =
    {""}

    SWR — { $swr }:1, die Anpassung, die die Antenne bietet, wie das Funkgerät sie misst. Unter 2:1 ist unauffällig; über 3:1 wird die Skala rot.
smeter-received-signal-strength-s9-db = Empfangene Signalstärke: S9 + { $over } dB ({ $s_dbm } dBm im Durchlassbereich).
smeter-received-signal-strength-s-dbm = Empfangene Signalstärke: S{ $value } ({ $s_dbm } dBm im Durchlassbereich).
smeter-radio-reports-c =
    {""}

    Das Funkgerät meldet { $t } °C.
smeter-ps-puresignal-correcting-db-compression =
    {""}

    PS — PureSignal korrigiert { $correction_db } dB Kompression{ $held } (die Rückkopplung stimmte mit der Aussendung bei { $score } überein).
smeter-ps-puresignal-switched-but-has =
    {""}

    PS −− — PureSignal ist eingeschaltet, hat aber die Aussendung im Rückkopplungspfad nicht gefunden (bester Treffer { $score }), daher geht der Sender unkorrigiert hinaus. Prüfen, ob eine Probe des Verstärkerausgangs wirklich an einen Empfangseingang gelangt, den der Sende-/Empfangsschalter beim Senden nicht wegnimmt, und dass sein Dämpfungsglied sie nicht verschluckt. Die Schleife hört nur auf diesem Empfänger — ein Koppler an einem zweiten Empfänger (RX2, ADC2) wird nie als Rückkopplung gelesen.

## specview: messages with values (src/widgets/spectrum_view.rs)

specview-rit-hz = RIT { $hz } Hz
specview-xit-hz = XIT { $hz } Hz

## decodes: nested messages

decodes-you-transmit-s-everyone-else = Sie senden { $off } s { $after } alle anderen — der übliche Grund, warum Rufe unbeantwortet bleiben. Die Uhr des Computers synchronisieren.

## publicsdr: nested messages

publicsdr-noise-floor-score = Bewertung des Rauschbodens { $snr }
publicsdr-up-khz-i-q = bis zu { $khz } kHz I/Q

## txaudio: more messages

txaudio-gr-gate-de-ess-comp = GR Gate { $gate_gr_db } De-Esser { $deess_gr_db } Komp. { $comp_gr_db } Begr. { $limiter_gr_db }

## amp: more messages

amp-h-step = ~{ $l_step } µH · Schritt { $l_step2 }
amp-pf-step = ~{ $c_step } pF · Schritt { $c_step2 }

## sstv: more messages

sstv-bit-octets-chunks-first-pass = { $from } · { $filename } · { $width }×{ $height } { $bits_per_pixel }-Bit · { $media_type } / { $content_encoding } · { $encoded_size } Oktette in { $chunk_count } Teilen ({ $chunks_first_pass } im ersten Durchgang) · Sitzung { $session }
sstv-baud-cpfsk-hz-khz-occupied = { $value } — { $symbol_rate } Baud CPFSK, ±{ $deviation_hz } Hz, { $bandwidth_hz } kHz belegte Bandbreite

## Clock, and a few short labels with values.

time-utc-and-local = { $utc } UTC · { $local } Ortszeit
sat-in-seconds = in { $s } s
winlink-to-list = an { $to }
winlink-cc-list = Kopie an { $cc }
spots-n-of-total = { $n } von { $total }
wefax-n-of-total = { $n } von { $total }
adsb-range-bearing = { $km } km bei { $bearing }°
ais-range-bearing = { $km } km bei { $bearing }°

## More values from sdroxide-types, as pickers name them (src/i18n/labels.rs).
rtl-sdr-hf-mode-off = Aus (nur Tuner, ab 24 MHz)
rtl-sdr-hf-mode-auto = Automatisch
rtl-sdr-hf-mode-direct-q = Direkte Abtastung (Q-Zweig)
rtl-sdr-agc-manual = Manuell (keine AGC)
rtl-sdr-agc-tuner = Tuner-AGC
rtl-sdr-agc-rtl = RTL-Digital-AGC
rtl-sdr-agc-both = Tuner- + RTL-AGC
mode-control-cat = CAT
mode-control-radio = Radio-gesteuert
line-state-none = Keine
line-state-high = Hoch
line-state-low = Niedrig
elad-tx-input-usb-audio = USB-Audio
elad-tx-input-mic = Mikrofon
elad-tx-input-auto = Automatisch
elad-tx-input-leave = Wie am Funkgerät eingestellt lassen
tr-usdx-audio-one-cable = Ein Kabel (Audio im CAT-Strom)
tr-usdx-audio-sound-card = USB-Soundkarte (3,5-mm-Buchse)
spy-server-format-uint8 = 8 Bit (geringste Bandbreite)
spy-server-format-int16 = 16 Bit
spy-server-format-float32 = 32-Bit-Float
sound-format-iq = IQ (Stereo)
sound-format-demod-audio = Demodulierte Audio
sdr-play-duo-tuner-tuner1 = Tuner 1 (50 Ohm / Hi-Z)
sdr-play-duo-tuner-tuner2 = Tuner 2 (50 Ohm)
sdr-play-duo-role-diversity = Eine zweite Antenne (Diversität / Störunterdrückung)
sdr-play-duo-role-second-radio = Ein zweites Funkgerät, auf eigener Frequenz
sdr-play-agc-off = Aus (manuelle ZF-Verstärkung)
sdr-play-agc-hz5 = 5 Hz
sdr-play-agc-hz50 = 50 Hz
sdr-play-agc-hz100 = 100 Hz
rotator-protocol-rotctld = Hamlib rotctld (TCP)
rotator-protocol-pst-rotator = PSTRotator (UDP)
pluto-ptt-off = Aus
pluto-ptt-gpo01 = GPO0 = RX, GPO1 = TX
pluto-ptt-gpo23 = GPO2 = RX, GPO3 = TX
pluto-duplex-fdd = FDD (beides gleichzeitig)
pluto-duplex-tdd = TDD (nacheinander)
pluto-agc-manual = Manuell
pluto-agc-slow-attack = Langsamer Angriff
pluto-agc-fast-attack = Schneller Angriff
pluto-agc-hybrid = Hybrid
parity-none = Keine
parity-even = Gerade
parity-odd = Ungerade
panadapter-tap-antenna = Antenne (mit dem Funkgerät geteilt)
panadapter-tap-if-output = Die ZF-Ausgabe des Funkgeräts
panadapter-audio-attached = Der angeschlossene Empfänger
panadapter-audio-transceiver = Der Transceiver
kenwood-send-ts2000 = TS-2000-Stil (TX;)
kenwood-send-ts590 = TS-590-Stil (TX1;)
digi-mode-usb = USB
digi-mode-data = DIGI
digi-mode-radio = Radio-gesteuert
cw-keying-cat = Geräte-Keyer (CAT)
cw-keying-audio = Soundkarte (MCW)
audio-route-both = Beide Kanäle
audio-route-left = Nur links
audio-route-right = Nur rechts
amp-model-hr500 = Hardrock-500
amp-model-hr50 = Hardrock-50 (ungetestet)
alert-sound-ding = Ding
alert-sound-two-tone = Zweiton
alert-sound-triplet = Triole
alert-sound-warble = Warbeln
alert-sound-digital = Digital
alert-reply-tone = Ton
alert-reply-voice = Stimme
alert-reply-both = Ton + Stimme
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
icom-model-other = Andere (Adresse von Hand eintragen)
backend-auto = Automatisch erkennen (SoapySDR / CAT)
backend-soapy = SoapySDR
backend-cat = CAT / Audio
backend-hpsdr = HPSDR (Netzwerk)
backend-tci = TCI (Netzwerk)
backend-icom-net = Icom LAN (Netzwerk)
backend-smart-sdr = SmartSDR / FlexRadio (Netzwerk)
backend-pluto = PlutoSDR (Netzwerk)
backend-rtl-sdr = RTL-SDR (USB)
backend-rtl-tcp = RTL-SDR über rtl_tcp (Netzwerk)
backend-spy-server = SpyServer (Netzwerk)
backend-spy-server-vfo = SpyServer VFO+FFT, geringe Bandbreite (Netzwerk)
backend-kiwi-sdr = KiwiSDR / Web-888 (Netzwerk)
backend-rx888 = RX-888 (USB)
backend-airspy-hf = Airspy HF+ (USB)
backend-airspy = Airspy R2 / Mini (USB)
backend-hack-rf = HackRF One / Pro (USB)
backend-sdr-play = SDRplay RSP (USB)
backend-elad = ELAD FDM-DUO / FDM-S (USB)
backend-lime = LimeSDR + LimeRFE (LimeSuite)
backend-hydra-sdr = HydraSDR RFOne (USB)
backend-fobos = RigExpert Fobos SDR (USB)
backend-usb-audio = USB-Audio-Funkgerät (Soundkarte)
backend-none = Nicht eingerichtet
cat-family-xiegu = Xiegu
cat-family-icom = Icom
cat-family-yaesu = Yaesu
cat-family-kenwood = Kenwood
cat-family-elecraft = Elecraft
cat-family-elad = ELAD
cat-family-qrp-labs = QRP Labs
cat-family-rs-hfiq = RS-HFIQ
cat-family-tr-usdx = (tr)uSDX
cat-family-rigctld = Hamlib rigctld (Netzwerk)
cat-family-flrig = flrig (Netzwerk)

## Values from sdroxide-types drawn on chips, menus and lists (src/i18n/labels.rs).
agc-mode-off = Aus
agc-mode-slow = Langsam
agc-mode-med = Mittel
agc-mode-fast = Schnell
airspy-gain-linearity = Linearität (beste Behandlung starker Signale)
airspy-gain-sensitivity = Empfindlichkeit (beste bei schwachen Signalen)
ais-kind-class-a = Klasse A
ais-kind-class-b = Klasse B
ais-kind-base-station = Basisstation
ais-kind-sar-aircraft = SAR-Flugzeug
ais-kind-aid-to-navigation = Navigationshilfe
ais-kind-sart = SART / MOB / EPIRB
ais-kind-craft = zugehöriges Fahrzeug
alert-event-called = Eine Station ruft mich
alert-event-cq = Ein CQ, auf das ich antworten könnte
alert-event-new-dxcc = Neues DXCC-Gebiet
alert-event-new-dxcc-band = Neues Gebiet auf diesem Band
alert-event-new-grid = Neues Locator-Quadrat
audio-routing-both = Links + rechts
audio-routing-left = Nur links
audio-routing-right = Nur rechts
button-mode-momentary = Halten
button-mode-toggle = Umschalten
contest-mode-none = Keiner
contest-mode-eu-vhf = EU VHF Contest
converter-tx-off = Aus beim Umsetzen
converter-tx-transverter = Über denselben Konverter
converter-tx-own = Eigener Versatz
cw-engine-neural = NEURONAL
cw-engine-timing = TAKT
decode-sort-none = Keine
decode-sort-signal = SNR
decode-sort-distance = Entf.
decode-sort-country = Land
diversity-mode-cancel = Auslöschen — eine Störquelle nullen
diversity-mode-combine = Kombinieren — Diversitätsempfang
dxped-mode-normal = Normal
dxped-mode-hound = Hound
dxped-mode-fox = Fox
fail-safe-refuse-tx = Senden verweigern
fail-safe-warn-only = Trotzdem senden, warnen
ftp-security-plain = FTP (keine Verschlüsselung)
ftp-security-explicit-tls = FTPS explizit (AUTH TLS)
ftp-security-implicit-tls = FTPS implizit
hpsdr-filter-board-none = Keine — Ausgänge bleiben aus
hpsdr-filter-board-n2adr = N2ADR-Filterkarte
hpsdr-filter-board-alex = Alex / Hermes-Bandcode (Zeus, HiQSDR, Quisk)
hpsdr-filter-board-custom = Eigene — meine eigene Tabelle unten
hpsdr-io-rx-input-radio = Eigener Eingang des Funkgeräts
hpsdr-io-rx-input-io-board = IO-Karte J9
hpsdr-io-rx-input-io-board-pure-signal = IO-Karte J9, PureSignal beim Senden
hydra-sdr-gain-linearity = Linearität (beste Behandlung starker Signale)
hydra-sdr-gain-sensitivity = Empfindlichkeit (beste bei schwachen Signalen)
icom-rx-source-af = AF — das Funkgerät demoduliert
icom-rx-source-if12k = 12-kHz-ZF — sdroxide demoduliert
icom-scope-span-radio = Wie am Funkgerät eingestellt
icom-scope-span-khz2-5 = 5 kHz (±2,5 k)
icom-scope-span-khz5 = 10 kHz (±5 k)
icom-scope-span-khz10 = 20 kHz (±10 k)
icom-scope-span-khz25 = 50 kHz (±25 k)
icom-scope-span-khz50 = 100 kHz (±50 k)
icom-scope-span-khz100 = 200 kHz (±100 k)
icom-scope-span-khz250 = 500 kHz (±250 k)
icom-scope-span-khz500 = 1 MHz (±500 k)
ism-family-weather = WETTER
ism-family-meter = MESSGERÄTE
ism-family-home-auto = HAUS
ism-family-lo-ra = LORA
ism-family-unidentified = UNBEKANNT
ism-protocol-la-crosse-it = LaCrosse IT+
ism-protocol-fine-offset = Fine Offset
ism-protocol-bresser = Bresser
ism-protocol-z-wave = Z-Wave
ism-protocol-homematic = Homematic
ism-protocol-rtl433 = rtl_433
ism-protocol-unidentified = nicht identifiziert
js8-speed-normal = NORMAL
js8-speed-fast = SCHNELL
js8-speed-turbo = TURBO
js8-speed-slow = LANGSAM
lime-aux-role-off = Nicht benutzt
lime-aux-role-diversity = Eine zweite Antenne (Diversität / Störunterdrückung)
lime-aux-role-pure-signal = Sende-Rückkopplung (PureSignal-Vorverzerrung)
lookup-provider-none = Aus
lookup-provider-qrz = QRZ.com
lookup-provider-ham-qth = HamQTH
mail-folder-inbox = EINGANG
mail-folder-outbox = AUSGANG
mail-folder-sent = GESENDET
mail-folder-archive = ARCHIV
memory-sort-name = Name
memory-sort-freq = Frequenz
memory-sort-band = Band
memory-sort-stored = Gespeichert
mouse-button-middle = Mitte
mouse-button-extra1 = Zurück (Zusatz 1)
mouse-button-extra2 = Vor (Zusatz 2)
nr-level-off = Aus
nr-level-low = Niedrig
nr-level-medium = Mittel
nr-level-high = Hoch
nr-level-rnn-low = RNN niedrig
nr-level-rnn-med = RNN mittel
nr-level-rnn-high = RNN hoch
nr-level-spec-low = SPEC niedrig
nr-level-spec-med = SPEC mittel
nr-level-spec-high = SPEC hoch
nr-level-nr2-low = NR2 niedrig
nr-level-nr2-med = NR2 mittel
nr-level-nr2-high = NR2 hoch
nr-level-df-low = DFNR niedrig
nr-level-df-med = DFNR mittel
nr-level-df-high = DFNR hoch
nr-strength-low = Niedrig
nr-strength-med = Mittel
nr-strength-high = Hoch
orbit-rings-none = keine
orbit-rings-curated = ausgewählte
orbit-rings-all = alle
qso-step-idle = Leerlauf
qso-step-wait-cq = Warte auf CQ
qso-step-calling-cq = CQ-Ruf
qso-step-tx-grid = Senden: Locator
qso-step-tx-report = Senden: Bericht
qso-step-tx-r-report = Senden: R+Bericht
qso-step-tx-rr73 = Senden: RR73
qso-step-tx73 = Senden: 73
qso-step-confirming = Bestätigung
rds-standard-auto = Auto
rds-standard-rds = RDS
rds-standard-rbds = RBDS
region-r1 = Region 1 — Europa, Afrika, Naher Osten, Nordasien
region-r2 = Region 2 — Amerika
region-r3 = Region 3 — Süd- und Ostasien, Australasien, Pazifik
relative-mode-absolute = Absolut (Schieberegler)
relative-mode-sign-magnitude = Relativ: Vorzeichen/Betrag
relative-mode-twos-complement = Relativ: Zweierkomplement
relative-mode-binary-offset = Relativ: auf 64 zentriert
relay-link-off = Keine
relay-link-serial = USB-Relaiskarte (seriell)
relay-link-serial-lines = Serielle RTS-/DTR-Leitung
relay-link-hid = USB-Relaiskarte (HID)
relay-link-cm108 = CM108/CM119-GPIO der Soundkarte
relay-link-gpio = Linux-GPIO-Leitung
relay-link-command = Externer Befehl
relay-role-unused = Nicht benutzt
relay-role-sdr-antenna = Die SDR-Antenne erden
relay-role-amplifier = Einen Verstärker / ein Sende-/Empfangsrelais tasten
relay-role-aux = Hilfsfunktion
relay-role-band-decoder = Bandentschlüssler (Filter / Transverter)
rfe-channel-wb1000 = Breitband 1 – 1000 MHz
rfe-channel-wb4000 = Breitband 1 – 4 GHz
rfe-channel-ham0030 = KW, bis 30 MHz
rfe-channel-ham0070 = 6 m / 4 m (50 – 70 MHz)
rfe-channel-ham0145 = 2 m (140 – 150 MHz)
rfe-channel-ham0220 = 1,25 m (220 – 225 MHz)
rfe-channel-ham0435 = 70 cm (400 – 450 MHz)
rfe-channel-ham0920 = 33 cm (902 – 928 MHz)
rfe-channel-ham1280 = 23 cm (1220 – 1420 MHz)
rfe-channel-ham2400 = 13 cm (2,3 – 2,5 GHz)
rfe-channel-ham3500 = 9 cm (3,3 – 3,7 GHz)
rfe-channel-cell-band01 = Mobilfunkband 1
rfe-channel-cell-band02 = Mobilfunkband 2
rfe-channel-cell-band03 = Mobilfunkband 3
rfe-channel-cell-band07 = Mobilfunkband 7
rfe-channel-cell-band38 = Mobilfunkband 38
rfe-channel-not-selected = Keiner gewählt
rfe-link-off = Nicht verbunden
rfe-link-serial = Über das eigene USB-Kabel (seriell)
rfe-link-board = Über das LimeSDR (GPIO / I²C)
rfe-mode-control-auto = Automatisch (folgt der Verkabelung)
rfe-mode-control-rx = Immer empfangen
rfe-mode-control-tx = Immer senden
rfe-mode-control-tx-rx = Immer beides (nur getrennte Anschlüsse)
rifp-encoding-auto = Auto
rifp-encoding-png = PNG
rifp-encoding-jpeg = JPEG
rifp-encoding-raw = Roh
rifp-encoding-rle8 = RLE8
rifp-encoding-zlib = Zlib
rifp-encoding-group4 = G4
rifp-encoding-group3 = G3
scan-kind-memories = SPEICHER
scan-kind-range = BEREICH
scan-resume-carrier = TRÄGER
scan-resume-timed = ZEITGESTEUERT
scan-resume-manual = MANUELL
sense-line-off = Nicht verdrahtet
sense-line-cts = CTS
sense-line-dsr = DSR
sense-line-dcd = DCD (Trägererkennung)
shift-simplex = SIMPLEX
shift-minus = −
shift-plus = +
smeter-trace-right-click = Rechtsklick: Richtung und Tempo des Verlaufs
smeter-trace-caption-flow = Flow
smeter-trace-flow-right = RECHTS
smeter-trace-flow-left = LINKS
smeter-trace-flow-hint = Laufrichtung: LINKS setzt den neuesten Wert an den rechten Rand, RECHTS an den linken
smeter-trace-range-hint = Unten und oben des Verlaufs in S-Stufen: Griff ziehen, z. B. S3–S9 oder S6–S9, um ein Band über die ganze Höhe zu spreizen. Nur Empfang
smeter-trace-caption-speed = Tempo
smeter-trace-speed-hint = Stufen des Wasserfalls, in seinen Verhältnissen: LANGSAM zeigt 168 s, MITTEL 30 s, SCHNELL 15 s, SCHNELLER 7,5 s, AM SCHNELLSTEN 3,75 s
smeter-snr-right = Mitte { $mid } dB · { $khz } kHz · { $frames } Bilder
smeter-snr-off = S/N ist aus
smeter-snr-hover = S/N-Anzeige: Leistung im Durchlass über dem Rauschboden des S/N-Guides, in dB ((S+N)/N wie die S/N-Linie); der Balken ist das Mittel über die Breite des Cursors, die Marke die stärkste Zelle in seiner Mitte (±10 % der Breite). Die anderen Formen zeigen absolute dBm.
speed-slow = Langsam
speed-medium = Mittel
speed-fast = Schnell
speed-faster = Schneller
speed-fastest = Am schnellsten
tls-verify-system = Zertifikatsspeicher des Systems
tls-verify-pinned = Festgepinnter Fingerabdruck
tls-verify-off = Nicht prüfen
tone-mode-off = AUS
tone-mode-ctcss = CTCSS
tone-mode-dcs = DCS
tx-voice-stage-gate = Gate
tx-voice-stage-eq = EQ
tx-voice-stage-de-ess = De-Esser
tx-voice-stage-comp = Kompressor
upload-naming-sequence = image1 … imageN (rotierend)
upload-naming-original = Originaler Dateiname
wheel-action-zoom = Zoomen
wheel-action-tune = Abstimmen
wheel-action-none = Nichts

## jtty (src/app/panels/jtty.rs)

jtty-starting-receiver = JTTY-Empfänger wird gestartet…
jtty-jtty = JTTY
jtty-receive-audio-level = Empfangs-Audiopegel
jtty-n-rx = { $n } empf.
jtty-listening-transmissions-start-any-time = Empfang läuft. JTTY-Aussendungen beginnen jederzeit — eine Nachricht erscheint hier kurz nachdem sie gehört wurde.
jtty-tx = TX
jtty-sending = {" "}SENDET{" "}
jtty-tx-2 = {"   "}TX{"   "}
jtty-call-cq = {" "}CQ RUFEN{" "}
jtty-message-sent-once-press-tx = Die Nachricht wird einmal gesendet, wenn Sie TX drücken — JTTY ist asynchron, es gibt also keine Periode abzuwarten und nichts wiederholt sich.

## frame, settings/remote, js8, top_bar: certificates, sent JS8, FLEX tuner

frame-certificate-not-signed-authority = Das Zertifikat des Servers ist von keiner Stelle signiert, der dieser Client vertraut — meist ist es selbstsigniert. Es hat dieses Zertifikat vorgelegt (SHA-256):
frame-compare-certificate-trust-only-if-match = Vergleichen Sie es mit dem Ihres Servers — z. B. openssl x509 -noout -fingerprint -sha256 -in cert.pem — und vertrauen Sie ihm nur, wenn beide übereinstimmen. Danach wird genau dieses Zertifikat von diesem Server akzeptiert und jedes andere abgelehnt.
frame-trust-certificate-connect = Diesem Zertifikat vertrauen und verbinden
settings-remote-trusted-certificates = Vertrauenswürdige Zertifikate
settings-remote-forget = Vergessen
settings-remote-forget-hint = Zurück zur normalen Prüfung gegen die öffentlichen Zertifizierungsstellen. Ein Server, der noch dieses Zertifikat nutzt, fragt erneut nach Vertrauen.
js8-transmitted-by-this-station = Von dieser Station gesendet
topbar-swr-match-antenna-presents =
    Die Anpassung der Antenne, wie das Funkgerät sie misst — live, solange das Funkgerät sendet, egal wer getastet hat: ein ATU-Abstimmlauf, TUNE, PTT oder das Funkgerät selbst. Abgeblendet ist es der letzte Messwert, aus dem letzten Durchgang.

    Grün bis { $good }:1; gelb bis zur Grenze des SWR-Schutzes von { $limit }:1; rot darüber. Mit einem externen Tuner den internen überbrücken und auf den kürzesten Balken abstimmen.
topbar-atu-idle = Seit dem Start des Funkgeräts keine Abstimmung.
topbar-atu-tuning = Stimmt ab — das Funkgerät sendet einen Träger in den Tuner.
topbar-atu-tuned-from-memory = Angepasst, aus dem Speicher des Tuners für diese Frequenz.
topbar-atu-tuned = Angepasst.
topbar-atu-bypass = Überbrückt.
topbar-atu-failed = Die letzte Abstimmung fand keine Anpassung.
topbar-atu-aborted = Die letzte Abstimmung wurde abgebrochen.
topbar-atu-built-in-tuner =
    Der eingebaute Antennentuner des Funkgeräts.

    { $state }
topbar-atu-start-tune = Eine Abstimmung starten. Das Funkgerät tastet sich selbst mit einem Träger, solange die Anpassung dauert, mit seiner Abstimmleistung (dem Tune-Pegel) — die Antenne muss also angeschlossen sein. Leuchtet, solange die gefundene Anpassung eingeschleift ist.
topbar-atu-bypass-hint = Den Tuner überbrücken. Leuchtet, solange er überbrückt ist.

# Undocked windows (from sdroxide-brown)
frame-detached-title = { $flavor } — { $name }
frame-dock-band-menu-hint = DOCK — die Band- und Modusauswahl ins Hauptfenster zurückholen, genauso wie dieses Fenster zu schließen, ohne das Schließfeld suchen zu müssen.
frame-dock-panel-hint = DOCK — dieses Panel wieder unter den Wasserfall setzen
frame-dock-spectrum-hint = DOCK — dieses Spektrum ins Hauptfenster zurückholen, genauso wie dieses Fenster zu schließen, ohne das Schließfeld suchen zu müssen.
frame-every-module-own-window = jedes Modul hat sein eigenes Fenster
frame-module-aux-panadapter = Aux-Panadapter
frame-module-band-mode = Band & Modus
frame-module-controls = Bedienelemente
frame-module-panadapter = Panadapter
frame-module-panel = Panel
multi-dock-all-windows = ALLE FENSTER ANDOCKEN
multi-module-panel = Bedienfeld
multi-undock-all-windows = ALLE FENSTER ABDOCKEN
settings-ui-dock-all-hint = Holt jedes abgedockte Fenster zurück in dieses.
settings-ui-docked = In diesem Fenster angedockt
settings-ui-module-aux = AUX SP (zweites Spektrum)
settings-ui-module-band-menu = Band- und Betriebsartenwahl
settings-ui-module-controls = Bedienelemente
settings-ui-module-panadapter = Panadapter
settings-ui-undock-all-hint = Setzt jedes Modul in ein eigenes Fenster – das Hauptfenster behält das Spektrum und die Decoder. Ein Modul ohne Inhalt, etwa das Bedienpanel in einer Sprachbetriebsart, öffnet kein Fenster.
settings-ui-undock-hint =
    Abgedockt wandert dieses Modul in ein eigenes Fenster, und das Hauptfenster gibt den Platz an das Übrige zurück – legen Sie den Wasserfall auf einen zweiten Monitor oder die Bedienelemente auf einen dritten. Schließen des Fensters dockt es wieder an.
    
    Unter Wayland kommt die Größe zurück, aber der Compositor entscheidet, wohin ein Fenster kommt: Machen Sie es schwebend und heften Sie es mit einer Fensterregel für die app-id „{ $app_id }“ an einen Monitor.
settings-ui-undocked = Abgedockt — eigenes Fenster
settings-ui-window-label = Fenster { $module }
frame-window-chip-hint = WINDOW — dieses Fenster in ein eigenes Betriebssystem-Fenster verschieben, etwa auf einen anderen Monitor
frame-dock-panel-pane-hint = DOCK — dieses Panel in den Bereich unter dem Wasserfall zurückholen
frame-panel-window-title = { $mode } — Panel
frame-roll-up-hint = Einrollen — nur die Titelleiste behalten und dem Wasserfall den Platz zurückgeben
frame-unroll-hint = Ausrollen — das Panel wieder anzeigen
frame-window-panel-hint = WINDOW — diesem Panel ein eigenes Fenster geben, das auf einen anderen Monitor gelegt werden kann
topbar-dock-all = ALLE ANDOCKEN
topbar-dock-all-hint = Alle Modulfenster zurück in dieses Fenster holen
topbar-module-windows-title = Modulfenster
topbar-undock-all = ALLE ABDOCKEN
topbar-undock-all-hint = Jedes Modul in ein eigenes Fenster setzen — dieses Fenster bleibt als Spektrum und Decoder
topbar-windows-chip-hint =
    ▣  Links: alle Modulfenster zurückholen
    Rechts: die Liste der Modulfenster
topbar-wname-aux = SP2 (AUX) — ein zweites Spektrum
topbar-wname-band-menu = Band & Modus
topbar-wname-controls = RX-Bedienung
topbar-wname-panadapter = SP1 — das Spektrum
topbar-wname-panel = Bedienpanel
topbar-wstate-float = { $name } aus dem Bereich lösen und über diesem Fenster schweben lassen
topbar-wstate-floating-panel-only = Schwebend ist ein Zustand nur des Bedienpanels — ein Spektrum oder eine Konsole hat in diesem Fenster nur einen Platz
topbar-wstate-here-now = { $name } ist jetzt hier
topbar-wstate-no-panel = Kein Bedienpanel in { $mode } — das gilt in einem Modus, der eines hat
topbar-wstate-own-window = { $name } ein eigenes Fenster geben, auf einem beliebigen Monitor
topbar-wstate-put-back = { $name } wieder in dieses Fenster setzen
frame-workspace-missing = Kein Arbeitsbereich { $n } gespeichert — speichern Sie einen unter Einstellungen › Oberfläche › Arbeitsbereiche, dann ruft diese Taste ihn auf.
settings-ui-workspaces = Arbeitsbereiche
settings-ui-workspaces-note = Eine benannte Fensteranordnung – welche Module in eigenen Fenstern sind und wo diese Fenster liegen. Speichern Sie die aktuelle oder holen Sie eine gespeicherte zurück. Bis zu zehn.
settings-ui-workspaces-none = Noch keine Arbeitsbereiche gespeichert.
settings-ui-workspace-restore-home = Diese Anordnung wiederherstellen – alle Module im Hauptfenster
settings-ui-workspace-restore-out = Diese Anordnung wiederherstellen – { $n } Modul(e) in eigenen Fenstern
settings-ui-workspace-forget = Diesen Arbeitsbereich vergessen
settings-ui-workspace-name = diese Anordnung benennen
settings-ui-workspace-full = Zehn Arbeitsbereiche sind das Limit von SDRuno; löschen Sie zuerst einen
settings-ui-workspace-save = AKTUELLE SPEICHERN
settings-ui-workspace-save-hint = Die aktuelle Anordnung unter diesem Namen speichern
settings-ui-workspace-name-first = Geben Sie zuerst einen Namen ein
topbar-band-window2-hint = WINDOW — die Auswahl in einem eigenen Fenster öffnen, so breit Sie möchten, auf einem beliebigen Monitor
tnc-link = VERBINDUNG
tnc-connected = VERBUNDEN
tnc-calling = RUFT
tnc-idle = HÖRT
tnc-tx = TX
tnc-busy = BELEGT
tnc-busy-hint = Jemand anderes ist auf dem Kanal. Ein Anruf jetzt würde ihn überdecken.
tnc-setup = ⚙ SETUP
tnc-peer = Gegenstelle
tnc-width = Breite
tnc-speed = Tempo
tnc-snr = S/N
tnc-bytes = Bytes
tnc-queued = Wartend
tnc-host = Host
tnc-ports = Ports
tnc-ports-off = aus
tnc-nobody = niemand
tnc-holder-panel = dieses Feld
tnc-holder-winlink = das MAIL-Fenster
tnc-holder-tcp = ein Programm unter { $addr }
tnc-tune = TUNE
tnc-abort = ABBRUCH
tnc-abort-hint = Die Verbindung sofort trennen, egal wer sie führt. Die Gegenstelle läuft in eine Zeitüberschreitung.
tnc-terminal = TERMINAL
tnc-callsign = Rufzeichen
tnc-connect = VERBINDEN
tnc-disconnect = TRENNEN
tnc-disconnect-hint = Sauber auflegen: Wartendes geht zuerst hinaus, dann stimmt die Gegenstelle zu.
tnc-held-by = Das Modem wird von { $holder } benutzt.
tnc-no-callsign = Zuerst ein Rufzeichen setzen (⚙ SETUP)
tnc-clear = LEEREN
tnc-clear-hint = Das Protokoll leeren. Die Verbindung bleibt.
tnc-nothing-yet = Noch nichts. Oben ein Rufzeichen eingeben und VERBINDEN, oder in ⚙ SETUP „Hören“ einschalten, um Anrufe anzunehmen.
tnc-type-here = hier tippen, Enter sendet
tnc-not-connected = nicht verbunden
tnc-send = SENDEN
setup-tnc-bandwidth = Bandbreite
setup-tnc-listen = Anrufe annehmen
setup-tnc-listen-check = Im Feld hören
setup-tnc-listen-hint = Eingehende Anrufe im Terminal dieses Felds annehmen. Wirkungslos, solange die TCP-Host-Ports an sind, denn ein dort angeschlossenes Programm will die Anrufe.
setup-tnc-host = TCP-Host
playback-radio-n = { " " }· Radio { $n }
playback-no-stamp = kein Zeitstempel im Namen — wird in der aktuellen Betriebsart abgespielt
playback-title = WIEDERGABE — aufgenommenes Audio
playback-find-hint = Betriebsart, MHz, Datum, Name…
playback-refresh-hint = Aufnahmeordner erneut einlesen
playback-asking = Aufnahmen werden bei der Station angefragt…
playback-folder = Ordner: { $dir }
playback-none = Noch keine Aufnahmen. REC → MP3 drücken, während etwas auf Sendung ist; die Datei erscheint hier, benannt nach Uhrzeit, Frequenz und Betriebsart.
playback-on = ▶ AKTIV
playback-play = ▶ ABSPIELEN
playback-iq-hint = Eine I/Q-Aufnahme, kein Audio. Die Wiedergabe nimmt das vom Rekorder geschriebene Audio; eine I/Q-Aufnahme wird mit --file abgespielt.
playback-not-while-tx = Nicht während des Sendens
playback-play-hint = Diese Aufnahme von Anfang an in die Decoder einspielen
playback-busy-playing = Wird abgespielt — vor dem Löschen stoppen
playback-busy-recording = Wird noch aufgenommen — vor dem Löschen REC beenden
playback-delete-ask = LÖSCHEN?
playback-delete-hint-armed = Erneut drücken, um diese Datei endgültig zu löschen. Das lässt sich nicht rückgängig machen.
playback-delete-hint = Diese Aufnahme aus dem Ordner löschen (fragt vorher noch einmal nach)
playback-strip-label = ▶ WIEDERGABE
playback-not-live = nicht live — das Funkgerät pausiert
playback-carry-on = Wiedergabe fortsetzen
playback-pause = ⏸ PAUSE
playback-pause-hint = Pause — die Decoder hören Stille, bis Sie fortsetzen
playback-again = ↺ NOCHMAL
playback-again-hint = Zurück zum Anfang — ein neuer Versuch für ein Bild oder eine Dekodierung
playback-loop = SCHLEIFE
playback-loop-hint = Am Dateiende wieder von vorn beginnen
playback-stop = ■ STOPP
playback-stop-hint = Wiedergabe beenden und das Funkgerät in den vorherigen Zustand zurückversetzen
playback-files = DATEIEN
playback-end-of-file = Dateiende
playback-paused = pausiert
playback-recorded-on-mode = Aufgenommen auf { $mhz } MHz in { $mode } — Frequenz und Betriebsart wurden aus dem Dateinamen übernommen.
playback-recorded-on = Aufgenommen auf { $mhz } MHz — Frequenz und Betriebsart wurden aus dem Dateinamen übernommen.
playback-no-dial-mode = Der Name enthält weder Frequenz noch Betriebsart, daher wird in der aktuellen Betriebsart abgespielt — den Decoder im Betriebsartenmenü wählen.
playback-format-stereo = { $rate } Hz, Stereo — der linke (Empfangs-)Kanal wird abgespielt.
playback-format-mono = { $rate } Hz, Mono — der einzige Kanal wird abgespielt.
playback-stopped = Gestoppt: { $fault }
playback-intro = Eine Aufnahme in die Decoder einspielen — SSTV, CW, RTTY, PSK, FT8 und jede andere Betriebsart, die aus Audio dekodiert. Das Funkgerät pausiert während der Wiedergabe, Frequenz und Betriebsart wechseln zu denen der Aufnahme, und STOPP stellt das Funkgerät wieder her. Die Betriebsart während der Wiedergabe wechseln, um einen anderen Decoder auszuprobieren.
topbar-stereo-yields-hint = WFM-Stereo ist aus, solange Rauschminderung oder Auto-Notch laufen — sie würden die beiden Kanäle verschmieren. Schalten Sie sie ab, um Stereo zu hören
topbar-stereo-playback-hint = Stereo: Die Aufnahme wird wie aufgenommen auf beiden Ohren wiedergegeben. Klicken Sie, um Mono zu erzwingen
topbar-stereo-mono-recording-hint = Diese Aufnahme ist mono — es gibt keinen zweiten Kanal zum Abspielen
topbar-caption-play-back = Wiedergabe
topbar-recordings-chip = ▶ AUFNAHMEN…
topbar-recordings-hint = Eine Aufnahme in die Decoder zurückspielen — ein verpasstes SSTV-Bild, ein schwaches RTTY- oder CW-Signal, ein FT8-Slot — mit Wasserfall und Dekodierprotokoll, als wäre es auf Sendung. Das Funkgerät tritt währenddessen zurück.
playback-find = Suchen
playback-refresh = AKTUALISIEREN
wxsat-title = WETTERSATELLITEN
sat-wxsat-hint = WETTERSATELLITEN — das Bild, das ein NOAA- (APT) oder Meteor-Satellit (LRPT) auf 137 MHz sendet, beim Empfang gezeichnet, in einem abtrennbaren Fenster
sat-wxsat = WETTERSATELLITEN
wxsat-decode = Dekodieren
wxsat-decode-hint = Lässt den Dekoder auf dem gewählten Kanal laufen. Er arbeitet aus dem Abtastfenster und lässt die Betriebsart des Funkgeräts in Ruhe, also muss der Kanal im Fenster liegen.
wxsat-channel = Satellit
wxsat-tune = Hier abstimmen
wxsat-tune-hint = Schaltet den Dekoder ein und stellt die Frequenz auf diesen Kanal, damit das Signal im Abtastfenster liegt.
wxsat-clear = Löschen
wxsat-clear-hint = Beginnt ein leeres Bild. Was bisher empfangen wurde, ist weg, wenn es nicht gespeichert wurde.
wxsat-save = Als PNG speichern
wxsat-save-hint = Speichert das Bild so, wie es gezeichnet ist (gedreht, wenn Drehen an ist), als graues PNG; bei LRPT den gezeigten Kanal.
wxsat-flip = Drehen
wxsat-flip-hint = Dreht das Bild um: Ein Satellit auf dem Weg nach Süden wird kopfstehend gezeichnet.
wxsat-state-off = Dekoder aus
wxsat-level = Pegel { $db } dBFS
wxsat-locked = Zeilensync eingerastet
wxsat-searching = Suche nach dem Zeilensync
wxsat-lines = { $n } Zeilen
wxsat-outside-window = Der Kanal liegt außerhalb des Abtastfensters oder zu nah an dessen Rand für das ganze Signal. Nutzen Sie Hier abstimmen oder ein breiteres Fenster.
wxsat-waiting = Noch kein Bild. Schalten Sie den Dekoder ein, wenn ein Satellit über dem Horizont steht; ein Überflug dauert etwa zehn Minuten.
wxsat-signal = Signal
wxsat-kind-apt = APT
wxsat-kind-lrpt = LRPT
wxsat-image-channel = Kanal { $n }
wxsat-rows = { $n } Reihen
wxsat-frame-locked = Rahmensync eingerastet
wxsat-frames = { $n } gute Rahmen
wxsat-waiting-lrpt = Noch kein Bild. Meteor LRPT ist digital: Das Bild beginnt einige Sekunden nach dem Empfang des Satelliten, Reihe für Reihe, und jeder Kanal ist ein eigenes Bild.
wxsat-searching-frame = Suche nach dem Rahmensync
sondemap-no-sonde-yet = Noch keine Radiosonde empfangen
sonde-decoding = DEKODIERUNG
sonde-decoding-off = DEKODER AUS
sonde-count = Sonden: { $n }
sonde-frames = Frames: { $n }
sonde-channel = Kanal
sonde-channel-hover = Die Sondenfrequenz, auf der gehört wird (400–406 MHz). Das Radio wird ebenfalls dorthin abgestimmt, damit der Kanal im Fenster liegt.
sonde-col-serial = Sonde
sonde-col-alt = Höhe
sonde-col-climb = Steigen
sonde-col-speed = Geschwindigkeit
sonde-col-battery = Batterie
sonde-col-heard = Gehört
sonde-scan = Suche
sonde-scan-hover = Sucht im ganzen Abtastfenster nach Sonden und dekodiert bis zu vier gleichzeitig, statt auf einem Kanal zu hören. Das Fenster muss sie abdecken: das Kanalfeld zentriert das Radio.
sonde-online = Online (SondeHub)
sonde-online-hover = Zeigt auch Sonden, die andere Stationen gehört haben, aus der öffentlichen Liste von SondeHub. Nur lesend: Ihr Locator auf 0,1° gerundet und ein Radius von 400 km werden gesendet; nichts wird hochgeladen. Aus, bis Sie es einschalten.
sonde-online-count = { $n } online
sonde-online-no-grid = Tragen Sie Ihren Locator in der Digimode-Einrichtung ein, um Online-Sonden zu sehen.
sonde-online-failed = SondeHub hat nicht geantwortet.
sonde-export = Flug exportieren
sonde-export-kml-hover = Den Flug dieser Sonde als KML-Spur speichern (Google Earth): Zeit, Länge, Breite, Höhe in Metern
sonde-export-csv-hover = Den Flug dieser Sonde als CSV-Tabelle für eine Tabellenkalkulation speichern: Zeit, Breite, Länge, Höhe in Metern
sonde-export-none = Für diese Sonde ist noch keine Position gespeichert.
sonde-col-temp = Temp.
sonde-col-humidity = Feuchte
sonde-col-pressure = Druck
sonde-online-why-signin = Die Station verlangt zuerst eine Anmeldung.
sonde-online-why-busy = Die Station fragt SondeHub bereits so oft ab, wie sie darf. Versuchen Sie es in einer Minute erneut.
sonde-online-why-upstream = Die Station konnte SondeHub nicht erreichen.
sonde-online-why-unreadable = Die Antwort war keine Sondenliste.
sonde-online-why-no-station = Keine Station zum Fragen, und diese Seite erreicht SondeHub nicht selbst.
sonde-online-why-http = Antwort: HTTP { $n }.
sonde-card-source = Quelle
sonde-card-source-radio = Dieses Funkgerät
sonde-card-source-hub = SondeHub (von anderer Station gehört)
sonde-card-model = Modell
sonde-card-maker = Hersteller
sonde-card-variant = Variante
sonde-card-position = Position
sonde-card-from-you = Von Ihnen
sonde-card-heading = Kurs
sonde-card-satellites = Satelliten
sonde-card-snr = Signal-Rausch-Abstand
sonde-card-station = Gehört von
sonde-card-station-position = Position der Station
sonde-card-antenna = Antenne
sonde-card-software = Software
sonde-card-frames = Empfangene Frames
sonde-card-open = Auf { $site } öffnen
sonde-card-close = Schließen
sonde-card-source-both = Funk + SondeHub (beide empfangen sie)
sonde-card-stale = Status
sonde-card-stale-value = Nicht mehr in der Liste: letzte bekannte Position
sonde-card-detach = Lösen
sonde-card-attach = Anheften
sonde-card-listen-hover = Klicken, um das Funkgerät auf diesen Kanal abzustimmen: Die Frequenz springt dorthin und der Decoder hört nur diesen Kanal.
sonde-card-decoding = Von diesem Funkgerät dekodiert
sonde-card-waiting = Hört auf diesem Kanal, noch nichts dekodiert
sonde-card-click-to-listen = Frequenz anklicken, um zuzuhören
sonde-type-all = Alle Typen
sonde-type-hover = Nach einem Sondentyp suchen: Die Auswahl startet den Suchlauf, der das Fenster über das Band bewegt, bis dieser Typ gehört wird. Nur dieser Typ wird aufgelistet und gezeichnet.
sonde-export-saved = { $name } gespeichert.

## Keyboard modes: MT63 (src/app/panels/text_modem.rs)
textmodem-mt63-long-hint = MT63-{ $label }, { $bw } Hz breit — die lange Verschachtelung verteilt einen Burst auf { $chars } Zeichen
textmodem-mt63-short-hint = MT63-{ $label }, { $bw } Hz breit, kurze Verschachtelung — die kurze Verschachtelung verteilt einen Burst auf { $chars } Zeichen
textmodem-mt63-not-offered = 1K und 500L noch nicht angeboten
textmodem-mt63-not-offered-hint = MT63-1K und die lange 500-Hz-Verschachtelung werden mit dem Batch-Decoder gelesen, aber noch nicht mit dem Live-Empfänger: Die Synchronisation sucht Symbolphase und Polarität, aber nicht die Bandmitte, und eine Suche, die das tut, tauschte nachweislich eine Variante gegen eine andere ein.

## Keyboard modes: MFSK (src/app/panels/text_modem.rs)
textmodem-mfsk-hint = { $label } — { $baud } Baud, { $tones } Töne, { $bw } Hz breit{ $caveat }
textmodem-mfsk-reverse-hint = Kehrt die Tonreihenfolge um. Eine Station, die auf dem entgegengesetzten Seitenband empfangen wird, kommt an ihrer Mitte gespiegelt an und wird als Rauschen gelesen, bis dies eingeschaltet ist.

## Keyboard modes: DominoEX (src/app/panels/text_modem.rs)
textmodem-domino-hint = { $name } — { $baud } Baud, 18 Töne, inkrementelle Frequenztastung ohne Vorwärtsfehlerkorrektur
textmodem-ifk-reverse-hint = Kehrt die Tonschritte um. IFK+ trägt seine Daten im Schritt zwischen aufeinanderfolgenden Tönen, eine auf dem entgegengesetzten Seitenband empfangene Station hat daher lauter negative Schritte und wird als Rauschen gelesen, bis dies eingeschaltet ist.

## Keyboard modes: PSK families (src/app/panels/text_modem.rs)
textmodem-psk-family = Familie
textmodem-psk-family-plain-hint = Die eigentliche PSK31-Familie: ein differenzieller Träger, keine Vorwärtsfehlerkorrektur
textmodem-psk-family-bpsk63f-hint = Der robuste Code mit 62,5 Baud, ohne Interleaver — die einzige gecodete Betriebsart, die fldigi ohne einen sendet
textmodem-psk-family-robust-hint = Ein Träger mit dem robusten Code und einem 2x2xN-Interleaver
textmodem-psk-family-multi-hint = Mehrere Träger, der Varicode von PSK31 und **kein** Code — breiter und schneller, und es braucht ein sauberes Signal
textmodem-psk-family-robust-multi-hint = Mehrere Träger **und** der robuste Code: die breiteste und robusteste der PSK-Betriebsarten
textmodem-psk-baud-fast-hint = { $baud } Baud — wird erst nach etwa einer Sekunde gelesen: Bei dieser Rate beträgt die Einrastzeit des Empfängers ein Drittel einer Nachricht
textmodem-psk-baud-hint = { $baud } Baud
textmodem-psk-bpsk-hint = Ein Bit pro Symbol, differenziell detektiert
textmodem-psk-coded = gecodet
textmodem-psk-no-code = ohne Code
textmodem-psk-coded-hint =
    { $baud } Baud, { $n } { $n ->
            [one] Träger
           *[other] Träger
        }, { $code }, { $bw } Hz breit
